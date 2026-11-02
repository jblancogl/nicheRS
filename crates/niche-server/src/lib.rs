//! HTTP server: REST API v1, SSE job events, raster tiles and the embedded web UI (ADR-0004).
//!
//! Async handlers never run numerical work: engine calls go through `spawn_blocking` (short
//! I/O-bound calls) and jobs run on the engine's worker thread with the Rayon pool (ADR-0103).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use niche_app::accounts::{Principal, Role, Scope, UserFilter};
use niche_app::audit::{AuditAction as A, AuditEvent, AuditFilter};
use niche_app::dto::{
    AcceptInviteRequest, AreaCheckRequest, CreateOrgRequest, DataPreviewRequest, FeedbackCreated,
    FeedbackRequest, FeedbackStatusRequest, GrantAdminRequest, JobSpec, JobUpdate, LangRequest,
    LinkCreated, LinkRequest, LoginRequest, Me, ProjectRequest, ResetPasswordRequest,
    UpdateOrgRequest, UserDeleteRequest, UserDisableRequest, UserManagerRequest, UserRequest,
};
use niche_app::feedback::{self, GitLabProject};
use niche_app::gbif_accounts::GbifOwner;
use niche_app::{AppError, Engine};
use niche_geo::{colormap, tiles};
use serde::Deserialize;

mod account;
mod admin;
mod analyses;
mod auth;
mod datasets;
mod org;
mod studies;
mod user_feedback;

use account::*;
use admin::*;
use analyses::*;
use auth::*;
use datasets::*;
use org::*;
use studies::*;
use user_feedback::*;

#[derive(Clone)]
pub struct ServerOptions {
    /// Extra host names accepted in the Host header (server mode behind a reverse proxy).
    pub allowed_hosts: Vec<String>,
    /// Serve the UI from this directory instead of the embedded copy (development).
    pub ui_dir: Option<std::path::PathBuf>,
    pub build: String,
    /// Hosted mode: every API route except `/health` and `/auth/*` needs a session (ADR-0105).
    pub accounts: bool,
    /// How the deployment backs up the data volume (shown to administrators).
    pub volume_backup: Option<String>,
    /// Operator setting: GitLab project that receives a copy of every feedback as an issue
    /// (ADR-0110). Never exposed through the API or the interface.
    pub feedback_gitlab: Option<GitLabProject>,
}

#[derive(Clone)]
struct AppState {
    engine: Arc<Engine>,
    opts: Arc<ServerOptions>,
    shutdown: Arc<AtomicBool>,
    /// Failed sign-ins per e-mail: (count, first failure of the window).
    failures: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    /// Last time an (administrator, job) support access was logged: once per hour, not per tile.
    admin_access: Arc<Mutex<HashMap<(String, String), Instant>>>,
}

/// Who is calling: set by the session layer on every protected route.
#[derive(Clone)]
struct Ctx {
    /// Jobs listed and deletable: the user's organisation (everything in local mode).
    scope: Scope,
    /// Jobs that may be opened: also every organisation's for administrators (ADR-0106).
    access: Scope,
    principal: Option<Principal>,
}

/// `__Host-`: the browser refuses it unless Secure, Path=/ and without Domain, so a sibling
/// subdomain cannot plant a session cookie for this host.
const SESSION_COOKIE: &str = "__Host-niche_session";
/// Distinct e-mails with recent failures kept in memory; beyond, new ones are not tracked.
const MAX_TRACKED_FAILURES: usize = 10_000;
const MAX_FAILURES: u32 = 10;
const FAILURE_WINDOW: Duration = Duration::from_secs(15 * 60);

#[derive(rust_embed::Embed)]
#[folder = "../../web/dist"]
struct Ui;

struct ApiError(AppError);

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, title) = match &self.0 {
            AppError::Invalid(_) => (StatusCode::UNPROCESSABLE_ENTITY, "Invalid request"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "Not found"),
            AppError::Data(_) => (StatusCode::BAD_GATEWAY, "Data source unavailable"),
            AppError::Cancelled => (StatusCode::CONFLICT, "Cancelled"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "Conflict"),
            AppError::QuotaExceeded(_) => (StatusCode::CONFLICT, "Monthly quota reached"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "Not allowed"),
            AppError::Model(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Model could not be fitted",
            ),
            AppError::Storage(_) | AppError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal error")
            }
        };
        // Internal errors can carry file paths or SQL text: the client gets a reference to the
        // server log line instead of the message.
        let detail = if status == StatusCode::INTERNAL_SERVER_ERROR {
            let reference = format!(
                "{:x}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_micros())
            );
            tracing::error!(error = %self.0, %reference, "request failed");
            format!("internal error (reference {reference} in the server log)")
        } else {
            self.0.to_string()
        };
        let body = serde_json::json!({"type": "about:blank", "title": title, "status": status.as_u16(), "detail": detail});
        (
            status,
            [(header::CONTENT_TYPE, "application/problem+json")],
            body.to_string(),
        )
            .into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> ApiResult<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ApiError(AppError::Internal(e.to_string())))?
        .map_err(ApiError)
}

pub fn router(engine: Arc<Engine>, opts: ServerOptions, shutdown: Arc<AtomicBool>) -> Router {
    let state = AppState {
        engine,
        opts: Arc::new(opts),
        shutdown,
        failures: Arc::new(Mutex::new(HashMap::new())),
        admin_access: Arc::new(Mutex::new(HashMap::new())),
    };
    let public = Router::new()
        .route("/health", get(health))
        .route("/auth/link", post(auth_link))
        .route("/auth/accept", post(auth_accept))
        .route("/auth/reset", post(auth_reset))
        .route("/auth/login", post(auth_login))
        .route("/auth/logout", post(auth_logout))
        .route("/auth/me", get(auth_me))
        .route("/auth/lang", post(auth_lang))
        // Sign-in forms are a few hundred bytes; no reason to buffer megabytes unauthenticated.
        .layer(DefaultBodyLimit::max(16 << 10));
    let protected = Router::new()
        .route("/species/search", get(species_search))
        .route("/places", get(places_search))
        .route("/places/at", get(places_at))
        .route("/places/{id}", get(place_geometry))
        .route("/layers", get(layers))
        .route("/study-areas/check", post(check_area))
        .route("/occurrences/preview", post(preview))
        .route("/datasets", get(list_datasets).post(import_dataset))
        .route("/datasets/{id}", get(get_dataset).delete(delete_dataset))
        .route("/coordinate-systems", get(coordinate_systems))
        .route("/species/match", post(match_species))
        .route("/jobs", get(list_jobs).post(create_job))
        .route(
            "/jobs/{id}",
            get(get_job).delete(delete_job).patch(update_job),
        )
        .route("/projects", get(list_projects).post(create_project))
        .route(
            "/projects/{id}",
            axum::routing::patch(rename_project).delete(delete_project),
        )
        .route("/jobs/{id}/events", get(events))
        .route("/jobs/{id}/cancel", post(cancel_job))
        .route("/jobs/{id}/gbif-wait", get(gbif_wait))
        .route("/jobs/{id}/skip-download", post(skip_download))
        .route("/jobs/{id}/results", get(results))
        .route("/jobs/{id}/manifest", get(manifest))
        .route(
            "/jobs/{id}/species/{key}/tiles/{layer}/{z}/{x}/{y}",
            get(tile),
        )
        .route("/jobs/{id}/species/{key}/sample", get(sample))
        .route("/jobs/{id}/species/{key}/thumbnail.png", get(thumbnail))
        .route("/jobs/{id}/species/{key}/occurrences", get(occurrences))
        .route("/jobs/{id}/species/{key}/files/{name}", get(file))
        .route(
            "/jobs/{id}/analyses",
            get(list_analyses).post(create_analysis),
        )
        .route(
            "/jobs/{id}/analyses/{aid}",
            get(get_analysis).delete(delete_analysis),
        )
        .route("/jobs/{id}/analyses/{aid}/files/{name}", get(analysis_file))
        .route(
            "/jobs/{id}/analyses/{aid}/tiles/{layer}/{z}/{x}/{y}",
            get(analysis_tile),
        )
        .route("/admin/orgs", get(admin_orgs).post(admin_create_org))
        .route("/admin/orgs/{org}", get(admin_org).patch(admin_update_org))
        .route(
            "/admin/orgs/{org}/invites",
            get(admin_invites).post(admin_create_invite),
        )
        .route("/admin/invites/{id}/revoke", post(admin_revoke_invite))
        .route("/admin/users", get(admin_users))
        .route("/admin/users/{id}", get(admin_user))
        .route("/admin/users/disable", post(admin_disable_user))
        .route("/admin/users/reset", post(admin_reset_user))
        .route("/admin/users/delete", post(admin_delete_user))
        .route("/admin/usage", get(admin_usage))
        .route("/admin/usage.csv", get(admin_usage_csv))
        .route("/admin/activity", get(admin_activity))
        .route("/admin/system", get(admin_system))
        .route("/admin/metrics", get(admin_metrics))
        .route("/admin/users/manager", post(admin_set_manager))
        .route("/admin/users/admin", post(admin_grant_admin))
        .route("/admin/audit", get(admin_audit))
        .route("/account", axum::routing::patch(account_update))
        .route("/account/password", post(account_password))
        .route(
            "/account/gbif",
            get(account_gbif)
                .put(account_gbif_set)
                .delete(account_gbif_clear),
        )
        .route("/account/gbif/check", post(account_gbif_check))
        .route("/org", get(org_profile))
        .route(
            "/org/gbif",
            axum::routing::put(org_gbif_set).delete(org_gbif_clear),
        )
        .route("/org/gbif/check", post(org_gbif_check))
        .route("/org/members", get(org_members))
        .route("/org/members/reset", post(org_reset_member))
        .route("/org/members/disable", post(org_disable_member))
        .route("/org/invites", get(org_invites).post(org_create_invite))
        .route("/org/invites/{id}/revoke", post(org_revoke_invite))
        .route("/org/feedback", get(org_feedback))
        .route("/org/audit", get(org_audit))
        .route(
            "/feedback",
            // Screenshots and voice recordings, base64 in JSON (MAX_FILE_BYTES decoded).
            post(create_feedback).layer(DefaultBodyLimit::max(
                feedback::MAX_FILE_BYTES / 3 * 4 + (256 << 10),
            )),
        )
        .route("/feedback/{id}/files/{file}", get(feedback_file))
        .route("/admin/feedback", get(admin_feedback))
        .route(
            "/admin/feedback/{id}",
            axum::routing::patch(admin_feedback_status).delete(admin_feedback_delete),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), session_layer));
    let api = public
        .merge(protected)
        .layer(DefaultBodyLimit::max(8 << 20));
    Router::new()
        .nest("/api/v1", api)
        .fallback(static_ui)
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}

/// Local-mode protections (docs/research/12-security-and-licensing.md): Host allow-list against
/// DNS rebinding; Origin check and JSON content type for state-changing requests (CSRF).
async fn guard(State(s): State<AppState>, req: Request, next: Next) -> Response {
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let hostname = host.rsplit_once(':').map_or(host, |(h, p)| {
        if p.chars().all(|c| c.is_ascii_digit()) {
            h
        } else {
            host
        }
    });
    let allowed = matches!(hostname, "localhost" | "127.0.0.1" | "[::1]")
        || s.opts.allowed_hosts.iter().any(|a| a == hostname);
    if !allowed {
        return (StatusCode::MISDIRECTED_REQUEST, "Host not allowed").into_response();
    }
    if req.method() != Method::GET && req.method() != Method::HEAD {
        if let Some(origin) = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|o| o.to_str().ok())
        {
            let ok = origin.split("://").nth(1).is_some_and(|o| o == host);
            if !ok {
                return (StatusCode::FORBIDDEN, "Cross-origin request refused").into_response();
            }
        }
        let ct = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|c| c.to_str().ok())
            .unwrap_or("");
        let has_body = req
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|c| c.to_str().ok())
            .is_some_and(|l| l != "0");
        if has_body && !ct.starts_with("application/json") {
            return (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected application/json",
            )
                .into_response();
        }
    }
    let api = req.uri().path().starts_with("/api/");
    let mut resp = next.run(req).await;
    let h = resp.headers_mut();
    // Hosted mode: an organisation's results must not outlive the session in a shared
    // browser's HTTP cache (MapLibre keeps its own in-memory tile cache while the page is open).
    if s.opts.accounts && api {
        h.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        );
    }
    h.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    h.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    resp
}

async fn health(State(s): State<AppState>) -> Json<serde_json::Value> {
    let e = s.engine.clone();
    // Read by the deployment pipeline, which waits for no running study before a restart.
    let busy = blocking(move || e.busy()).await.ok();
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "build": s.opts.build,
        // The data directory is a server detail: only shown to the local single user.
        "data_dir": (!s.opts.accounts).then(|| s.engine.cfg.data_dir.display().to_string()),
        "online": true,
        "accounts": s.opts.accounts,
        // Installed administrative boundary packs (study areas by place name).
        "boundaries": s.engine.boundary_packs().into_iter().map(|(pack, _)| pack).collect::<Vec<_>>(),
        "busy": busy,
        // This process runs the studies (ADR-0115); during a deployment the other one serves too.
        "leader": s.engine.is_leader(),
    }))
}

async fn static_ui(State(s): State<AppState>, uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") {
        return ApiError(AppError::NotFound(format!("/{path}"))).into_response();
    }
    let (bytes, name): (Option<Vec<u8>>, String) = match &s.opts.ui_dir {
        Some(dir) => {
            let candidate = dir.join(path);
            let safe = !path.split('/').any(|seg| seg == "..");
            match (safe, std::fs::read(&candidate)) {
                (true, Ok(b)) if candidate.is_file() => (Some(b), path.to_string()),
                _ => (
                    std::fs::read(dir.join("index.html")).ok(),
                    "index.html".into(),
                ),
            }
        }
        None => match Ui::get(path) {
            Some(f) if !path.is_empty() => (Some(f.data.into_owned()), path.to_string()),
            _ => (
                Ui::get("index.html").map(|f| f.data.into_owned()),
                "index.html".into(),
            ),
        },
    };
    let Some(bytes) = bytes else {
        return (StatusCode::NOT_FOUND, "UI not available").into_response();
    };
    let mime = mime_guess::from_path(&name).first_or_octet_stream();
    // Hashed assets are immutable; index.html must revalidate so new builds are picked up.
    let cache = if name.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let mut resp = Response::new(Body::from(bytes));
    let h = resp.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    if name == "index.html" {
        h.insert(
            "content-security-policy",
            HeaderValue::from_static(
                "default-src 'self'; img-src 'self' data: blob: https:; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' data: https:; connect-src 'self' https:; worker-src 'self' blob:; child-src blob:; frame-ancestors 'none'; base-uri 'none'",
            ),
        );
    }
    resp
}

/// Wait for the leader lease, then run the background work until shutdown; the lease is
/// released when this returns (and by the kernel if the process dies).
fn lead(engine: &Arc<Engine>, stop: &Arc<AtomicBool>) {
    let data_dir = engine.cfg.data_dir.clone();
    let mut waiting_logged = false;
    let lease = loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        match niche_app::lease::Lease::try_acquire(&data_dir) {
            Ok(Some(lease)) => break lease,
            Ok(None) if !waiting_logged => {
                tracing::info!(
                    "serving requests; studies stay with the previous instance until it stops"
                );
                waiting_logged = true;
            }
            Ok(None) => {}
            Err(e) => tracing::error!(error = %e, "leader lease unavailable"),
        }
        std::thread::sleep(Duration::from_millis(500));
    };
    engine.set_leader(true);
    tracing::info!("leader: running studies, data preparation and the GBIF mirror");
    std::thread::scope(|s| {
        let worker = std::thread::Builder::new()
            .name("niche-worker".into())
            .spawn_scoped(s, || engine.worker_loop(stop));
        let preparer = std::thread::Builder::new()
            .name("niche-prepare".into())
            .spawn_scoped(s, || engine.prepare_loop(stop));
        if let Err(e) = worker.and(preparer) {
            tracing::error!(error = %e, "starting background threads");
            stop.store(true, Ordering::Relaxed);
        }
    });
    engine.set_leader(false);
    drop(lease);
}

/// The regional GBIF mirror (ADR-0114) runs only in the leader: I/O only, it may wait hours for
/// GBIF, so it is not joined at shutdown (an interrupted download or build resumes or restarts).
fn spawn_mirror(engine: &Arc<Engine>, stop: &Arc<AtomicBool>) {
    let e = engine.clone();
    let stop = stop.clone();
    let _ = std::thread::Builder::new()
        .name("niche-mirror".into())
        .spawn(move || {
            while !e.is_leader() {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            e.mirror_loop(&stop);
        });
}

/// Run the HTTP server until Ctrl-C / SIGTERM; the worker thread is stopped gracefully and a
/// running job is left resumable.
pub async fn serve(
    engine: Arc<Engine>,
    addr: SocketAddr,
    opts: ServerOptions,
) -> std::io::Result<()> {
    let shutdown = Arc::new(AtomicBool::new(false));
    // Background work runs in the process holding the leader lease (ADR-0115). During a
    // deployment the new process serves requests at once and takes over the work when the old
    // one exits and releases the lease (interrupted studies are then requeued and resume).
    spawn_mirror(&engine, &shutdown);
    let leader = {
        let e = engine.clone();
        let stop = shutdown.clone();
        std::thread::Builder::new()
            .name("niche-leader".into())
            .spawn(move || lead(&e, &stop))?
    };
    if let Some(gl) = opts.feedback_gitlab.clone() {
        tokio::spawn(forward_loop(engine.clone(), gl, shutdown.clone()));
    }
    let app = router(engine, opts, shutdown.clone());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "NicheRS listening — open http://{addr}/");
    let sd = shutdown.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let ctrl_c = async {
                let _ = tokio::signal::ctrl_c().await;
            };
            #[cfg(unix)]
            let term = async {
                if let Ok(mut s) =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                {
                    s.recv().await;
                }
            };
            #[cfg(not(unix))]
            let term = std::future::pending::<()>();
            tokio::select! { () = ctrl_c => {}, () = term => {} }
            tracing::info!(
                "shutdown requested; finishing in-flight requests and checkpointing jobs"
            );
            sd.store(true, Ordering::Relaxed);
        })
        .await?;
    shutdown.store(true, Ordering::Relaxed);
    let _ = tokio::task::spawn_blocking(move || leader.join()).await;
    Ok(())
}
