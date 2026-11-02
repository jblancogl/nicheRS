//! HTTP behaviour of hosted mode (ADR-0105): sign-in through invitation links, session cookie,
//! organisation isolation on every job route, quota and deletion over the API, sign-in
//! throttling, and local mode unchanged. Offline (synthetic demo data).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use niche_app::dto::*;
use niche_app::{Config, Engine, Limits, demo};
use niche_server::{ServerOptions, router};
use tower::ServiceExt;

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("niche-api-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn app(dir: &std::path::Path, accounts: bool) -> (Arc<Engine>, Router) {
    let engine = Arc::new(
        Engine::open(Config {
            data_dir: dir.join("data"),
            cache_dir: dir.join("cache"),
            max_records: 5000,
            gbif: None,
            secret_key: Some(
                niche_app::secrets::SecretKey::from_hex(&"ab".repeat(32)).expect("test key"),
            ),
            mirror: None,
            limits: Limits {
                max_species_per_job: 25,
                max_area_cells: None,
            },
            retention_days: 0,
        })
        .expect("engine"),
    );
    let opts = ServerOptions {
        allowed_hosts: vec![],
        ui_dir: None,
        build: "test".into(),
        accounts,
        volume_backup: None,
        feedback_gitlab: None,
    };
    let r = router(engine.clone(), opts, Arc::new(AtomicBool::new(false)));
    (engine, r)
}

fn spec(n: u64) -> JobSpec {
    JobSpec {
        name: "api".into(),
        species: (0..n)
            .map(|i| SpeciesRef {
                key: i,
                scientific_name: format!("{} #{i}", demo::SPECIES_NAME),
                own: None,
            })
            .collect(),
        area: StudyArea {
            name: "Demo".into(),
            geometry: demo::area_geojson(),
            places: vec![],
        },
        layers: demo::LAYERS.iter().map(|l| l.0.to_string()).collect(),
        resolution_factor: 2,
        filters: OccurrenceFilters::default(),
        model: ModelSettings {
            feature_classes: vec!["LQ".into()],
            beta_multipliers: vec![1.0],
            ..ModelSettings::default()
        },
        validation: ValidationSettings::default(),
        seed: 5,
        demo: true,
        projections: vec![],
        own_dataset: None,
    }
}

struct Resp {
    status: StatusCode,
    cookie: Option<String>,
    set_cookie: Option<String>,
    csp: Option<String>,
    disposition: Option<String>,
    cache: Option<String>,
    json: serde_json::Value,
}

async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    body: Option<serde_json::Value>,
) -> Resp {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, "localhost:8080");
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    let req = match body {
        Some(v) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string())),
        None => b.body(Body::empty()),
    }
    .expect("request");
    let res = app.clone().oneshot(req).await.expect("response");
    let status = res.status();
    let header_str = |h| {
        res.headers()
            .get(h)
            .and_then(|v: &header::HeaderValue| v.to_str().ok())
            .map(str::to_string)
    };
    let set_cookie = header_str(header::SET_COOKIE);
    let cache = header_str(header::CACHE_CONTROL);
    let csp = header_str(header::CONTENT_SECURITY_POLICY);
    let disposition = header_str(header::CONTENT_DISPOSITION);
    let cookie = set_cookie
        .as_deref()
        .and_then(|c| c.split(';').next())
        .map(str::to_string);
    let is_sse =
        header_str(header::CONTENT_TYPE).is_some_and(|c| c.starts_with("text/event-stream"));
    let json = if is_sse {
        serde_json::Value::Null
    } else {
        let bytes = res.into_body().collect().await.expect("body").to_bytes();
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    Resp {
        status,
        cookie,
        set_cookie,
        cache,
        csp,
        disposition,
        json,
    }
}

/// Administrators are members of the operator organisation (ADR-0112): designate theirs first.
fn promote(engine: &Engine, email: &str) {
    let st = engine.store();
    let user = st
        .user_list(None)
        .expect("users")
        .into_iter()
        .find(|u| u.email == email)
        .expect("account");
    st.org_set_operator(&user.org_name).expect("operator");
    st.user_set_admin(email, true).expect("promote");
}

async fn sign_up(app: &Router, engine: &Engine, org: &str, email: &str) -> String {
    engine.store().org_create(org, 3).expect("org");
    let token = engine.store().invite(org).expect("invite");
    let r = call(
        app,
        Method::POST,
        "/api/v1/auth/link",
        None,
        Some(serde_json::json!({"token": token})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["kind"], "invite");
    assert_eq!(r.json["org_name"], org);
    let r = call(
        app,
        Method::POST,
        "/api/v1/auth/accept",
        None,
        Some(serde_json::json!({"token": token, "name": "User", "email": email, "password": "a long enough password"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let sc = r.set_cookie.expect("set-cookie");
    assert!(sc.starts_with("__Host-niche_session="), "{sc}");
    for attr in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/"] {
        assert!(sc.contains(attr), "{sc}");
    }
    r.cookie.expect("cookie")
}

/// Job routes declared in the server, read from its source, so a new route cannot escape the
/// isolation test below.
fn declared_job_routes() -> Vec<String> {
    include_str!("../src/lib.rs")
        .lines()
        .filter_map(|l| {
            l.trim()
                .strip_prefix(".route(\"")
                .or_else(|| l.trim().strip_prefix("\""))
        })
        .filter_map(|l| l.split('"').next())
        .filter(|p| p.starts_with("/jobs/{id}"))
        .map(str::to_string)
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn organisations_only_see_their_own_jobs() {
    let dir = tmp("isolation");
    let (engine, app) = app(&dir, true);
    // Without a session nothing is reachable.
    assert_eq!(
        call(&app, Method::GET, "/api/v1/jobs", None, None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, Method::GET, "/api/v1/auth/me", None, None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/v1/jobs",
            Some("__Host-niche_session=forged"),
            None
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );
    let health = call(&app, Method::GET, "/api/v1/health", None, None).await;
    assert_eq!(health.json["data_dir"], serde_json::Value::Null);
    // Public, so the deployment pipeline can wait for no running study.
    assert_eq!(health.json["busy"], false);
    let a = sign_up(&app, &engine, "A", "a@a.fr").await;
    let b = sign_up(&app, &engine, "B", "b@b.fr").await;
    let me = call(&app, Method::GET, "/api/v1/auth/me", Some(&a), None).await;
    assert_eq!(me.json["organisation"], "A");
    assert_eq!(me.json["lang"], serde_json::Value::Null);
    // The account's interface language is stored with the account.
    let lang = |cookie: Option<String>, l: &'static str| {
        let app = app.clone();
        async move {
            call(
                &app,
                Method::POST,
                "/api/v1/auth/lang",
                cookie.as_deref(),
                Some(serde_json::json!({"lang": l})),
            )
            .await
            .status
        }
    };
    assert_eq!(lang(None, "fr").await, StatusCode::UNAUTHORIZED);
    assert_eq!(
        lang(Some(a.clone()), "de").await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(lang(Some(a.clone()), "fr").await, StatusCode::NO_CONTENT);
    let me = call(&app, Method::GET, "/api/v1/auth/me", Some(&a), None).await;
    assert_eq!(me.json["lang"], "fr");
    assert_eq!(me.json["quota"]["total"], 3);
    assert_eq!(me.cache.as_deref(), Some("private, no-store"));

    let r = call(
        &app,
        Method::POST,
        "/api/v1/jobs",
        Some(&a),
        Some(serde_json::to_value(spec(1)).expect("spec")),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json);
    let id = r.json["id"].as_str().expect("id").to_string();
    let e2 = engine.clone();
    let id2 = id.clone();
    tokio::task::spawn_blocking(move || {
        let never = AtomicBool::new(false);
        e2.run_job(&id2, &never, &never);
    })
    .await
    .expect("run");
    let res = call(
        &app,
        Method::GET,
        &format!("/api/v1/jobs/{id}/results"),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let key = res.json["species"][0]["species_key"].as_u64().expect("key");
    let file = res.json["species"][0]["files"][0]["name"]
        .as_str()
        .expect("file")
        .to_string();
    // The HTML report can be viewed in the browser, sandboxed; everything else downloads.
    let report = call(
        &app,
        Method::GET,
        &format!("/api/v1/jobs/{id}/species/{key}/files/report.html?inline=1"),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(report.status, StatusCode::OK);
    assert!(
        report
            .csp
            .as_deref()
            .is_some_and(|c| c.starts_with("sandbox;")),
        "{:?}",
        report.csp
    );
    assert!(
        report
            .disposition
            .as_deref()
            .is_some_and(|d| d.starts_with("inline"))
    );
    let md = call(
        &app,
        Method::GET,
        &format!("/api/v1/jobs/{id}/species/{key}/files/report.md?inline=1"),
        Some(&a),
        None,
    )
    .await;
    assert!(
        md.disposition
            .as_deref()
            .is_some_and(|d| d.starts_with("attachment"))
    );
    assert!(md.csp.is_none());
    // Nothing of an organisation stays in a shared browser's HTTP cache, tiles included.
    let tile = call(
        &app,
        Method::GET,
        &format!("/api/v1/jobs/{id}/species/{key}/tiles/suitability/0/0/0.png"),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(tile.status, StatusCode::OK);
    assert_eq!(tile.cache.as_deref(), Some("private, no-store"));

    // A decision-support analysis of the study (ADR-0018), so that its routes are probed too.
    let priority = serde_json::json!({"kind": "priority", "species_key": key});
    let created = call(
        &app,
        Method::POST,
        &format!("/api/v1/jobs/{id}/analyses"),
        Some(&a),
        Some(priority.clone()),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.json);
    let aid = created.json["id"].as_str().expect("aid").to_string();
    let analysis_probes: Vec<(&str, Method, String, Option<serde_json::Value>)> = vec![
        (
            "/jobs/{id}/analyses",
            Method::POST,
            format!("/api/v1/jobs/{id}/analyses"),
            Some(priority.clone()),
        ),
        (
            "/jobs/{id}/analyses",
            Method::GET,
            format!("/api/v1/jobs/{id}/analyses"),
            None,
        ),
        (
            "/jobs/{id}/analyses/{aid}",
            Method::GET,
            format!("/api/v1/jobs/{id}/analyses/{aid}"),
            None,
        ),
        (
            "/jobs/{id}/analyses/{aid}/files/{name}",
            Method::GET,
            format!("/api/v1/jobs/{id}/analyses/{aid}/files/report.html?inline=1"),
            None,
        ),
        (
            "/jobs/{id}/analyses/{aid}/tiles/{layer}/{z}/{x}/{y}",
            Method::GET,
            format!("/api/v1/jobs/{id}/analyses/{aid}/tiles/priority/0/0/0.png"),
            None,
        ),
    ];
    for (pattern, method, uri, body) in &analysis_probes {
        let r = call(&app, method.clone(), uri, Some(&b), body.clone()).await;
        assert_eq!(
            r.status,
            StatusCode::NOT_FOUND,
            "{pattern} visible to another organisation"
        );
        let r = call(&app, method.clone(), uri, Some(&a), body.clone()).await;
        assert!(r.status.is_success(), "{pattern}: {} {}", r.status, r.json);
        if pattern.ends_with("{name}") {
            assert!(r.csp.as_deref().is_some_and(|c| c.starts_with("sandbox;")));
        }
    }
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/jobs/{id}/analyses/{aid}"),
        Some(&b),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    let probes: Vec<(&str, Method, String)> = vec![
        ("/jobs/{id}", Method::GET, format!("/api/v1/jobs/{id}")),
        (
            "/jobs/{id}/events",
            Method::GET,
            format!("/api/v1/jobs/{id}/events"),
        ),
        (
            "/jobs/{id}/cancel",
            Method::POST,
            format!("/api/v1/jobs/{id}/cancel"),
        ),
        (
            "/jobs/{id}/gbif-wait",
            Method::GET,
            format!("/api/v1/jobs/{id}/gbif-wait"),
        ),
        (
            "/jobs/{id}/skip-download",
            Method::POST,
            format!("/api/v1/jobs/{id}/skip-download"),
        ),
        (
            "/jobs/{id}/results",
            Method::GET,
            format!("/api/v1/jobs/{id}/results"),
        ),
        (
            "/jobs/{id}/manifest",
            Method::GET,
            format!("/api/v1/jobs/{id}/manifest"),
        ),
        (
            "/jobs/{id}/species/{key}/tiles/{layer}/{z}/{x}/{y}",
            Method::GET,
            format!("/api/v1/jobs/{id}/species/{key}/tiles/suitability/0/0/0.png"),
        ),
        (
            "/jobs/{id}/species/{key}/sample",
            Method::GET,
            format!("/api/v1/jobs/{id}/species/{key}/sample?lng=0&lat=0"),
        ),
        (
            "/jobs/{id}/species/{key}/thumbnail.png",
            Method::GET,
            format!("/api/v1/jobs/{id}/species/{key}/thumbnail.png"),
        ),
        (
            "/jobs/{id}/species/{key}/occurrences",
            Method::GET,
            format!("/api/v1/jobs/{id}/species/{key}/occurrences"),
        ),
        (
            "/jobs/{id}/species/{key}/files/{name}",
            Method::GET,
            format!("/api/v1/jobs/{id}/species/{key}/files/{file}"),
        ),
    ];
    let declared = declared_job_routes();
    assert!(declared.len() >= 9, "{declared:?}");
    for d in &declared {
        assert!(
            probes.iter().any(|(p, _, _)| p == d) || analysis_probes.iter().any(|(p, ..)| p == d),
            "route {d} is not covered by the isolation test"
        );
    }
    for (pattern, method, uri) in &probes {
        let r = call(&app, method.clone(), uri, Some(&b), None).await;
        assert_eq!(
            r.status,
            StatusCode::NOT_FOUND,
            "{pattern} visible to another organisation"
        );
        let r = call(&app, method.clone(), uri, Some(&a), None).await;
        if pattern.ends_with("skip-download") {
            // The owner reaches it; the finished study is simply not waiting for GBIF.
            assert_eq!(r.status, StatusCode::CONFLICT, "{pattern}: {}", r.json);
            continue;
        }
        assert!(r.status.is_success(), "{pattern}: {} {}", r.status, r.json);
        if pattern.contains("tiles") {
            assert!(
                r.cache.as_deref().is_some_and(|c| c.starts_with("private")),
                "{:?}",
                r.cache
            );
        }
    }
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/jobs/{id}"),
        Some(&b),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(
        call(&app, Method::GET, "/api/v1/jobs", Some(&b), None)
            .await
            .json,
        serde_json::json!([])
    );
    assert_eq!(
        call(&app, Method::GET, "/api/v1/jobs", Some(&a), None)
            .await
            .json
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    // Quota: A has 3 species/month and 1 is used.
    let r = call(
        &app,
        Method::POST,
        "/api/v1/jobs",
        Some(&a),
        Some(serde_json::to_value(spec(3)).expect("spec")),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.json["title"], "Monthly quota reached");
    let me = call(&app, Method::GET, "/api/v1/auth/me", Some(&a), None).await;
    assert_eq!(
        (
            me.json["quota"]["used"].as_u64(),
            me.json["quota"]["remaining"].as_u64()
        ),
        (Some(1), Some(2))
    );

    // Deletion by the owner.
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/jobs/{id}"),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/v1/jobs/{id}"),
            Some(&a),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    // Logout ends the session.
    let r = call(&app, Method::POST, "/api/v1/auth/logout", Some(&a), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(r.set_cookie.is_some_and(|c| c.contains("Max-Age=0")));
    assert_eq!(
        call(&app, Method::GET, "/api/v1/jobs", Some(&a), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn sign_in_is_throttled_after_repeated_failures() {
    let dir = tmp("throttle");
    let (engine, app) = app(&dir, true);
    sign_up(&app, &engine, "A", "a@a.fr").await;
    let login = |pw: &'static str| {
        let app = app.clone();
        async move {
            call(
                &app,
                Method::POST,
                "/api/v1/auth/login",
                None,
                Some(serde_json::json!({"email": "A@a.fr", "password": pw})),
            )
            .await
        }
    };
    let ok = login("a long enough password").await;
    assert_eq!(ok.status, StatusCode::OK);
    assert!(ok.cookie.is_some());
    for _ in 0..10 {
        assert_eq!(
            login("wrong password value").await.status,
            StatusCode::UNAUTHORIZED
        );
    }
    // Even the right password is refused while blocked.
    assert_eq!(
        login("a long enough password").await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn unauthenticated_auth_requests_stay_cheap() {
    let dir = tmp("auth-bounds");
    let (_engine, app) = app(&dir, true);
    // An unknown invitation or reset token is refused (before any password hash is computed).
    for path in ["/api/v1/auth/accept", "/api/v1/auth/reset"] {
        let r = call(
            &app,
            Method::POST,
            path,
            None,
            Some(serde_json::json!({"token": "junk", "name": "X", "email": "x@x.fr", "password": "a long enough password"})),
        )
        .await;
        assert!(r.status.is_client_error(), "{path}: {}", r.status);
        assert!(r.set_cookie.is_none());
    }
    // Sign-in bodies are small; megabytes are refused without being parsed.
    let r = call(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(serde_json::json!({"email": "a".repeat(20_000), "password": "x"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE);
    // Addresses that cannot be accounts are not tracked, so they never trigger the block.
    let long = format!("{}@x.fr", "a".repeat(300));
    for _ in 0..12 {
        let r = call(
            &app,
            Method::POST,
            "/api/v1/auth/login",
            None,
            Some(serde_json::json!({"email": long, "password": "wrong password value"})),
        )
        .await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn local_mode_needs_no_account() {
    let dir = tmp("local");
    let (_engine, app) = app(&dir, false);
    assert_eq!(
        call(&app, Method::GET, "/api/v1/auth/me", None, None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, Method::GET, "/api/v1/jobs", None, None)
            .await
            .status,
        StatusCode::OK
    );
    let r = call(
        &app,
        Method::POST,
        "/api/v1/jobs",
        None,
        Some(serde_json::to_value(spec(1)).expect("spec")),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json);
    assert!(
        call(&app, Method::GET, "/api/v1/health", None, None)
            .await
            .json["data_dir"]
            .is_string()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn administration_api() {
    let dir = tmp("admin");
    let (engine, app) = app(&dir, true);
    let ns = sign_up(&app, &engine, "Natural Solutions", "admin@ns.fr").await;
    let client = sign_up(&app, &engine, "Client", "c@client.fr").await;
    // Members are refused; the flag is only set from the CLI (store), never over HTTP.
    for path in [
        "/api/v1/admin/orgs",
        "/api/v1/admin/users",
        "/api/v1/admin/usage",
        "/api/v1/admin/activity",
    ] {
        assert_eq!(
            call(&app, Method::GET, path, Some(&ns), None).await.status,
            StatusCode::FORBIDDEN,
            "{path}"
        );
        assert_eq!(
            call(&app, Method::GET, path, None, None).await.status,
            StatusCode::UNAUTHORIZED,
            "{path}"
        );
    }
    promote(&engine, "admin@ns.fr");
    let me = call(&app, Method::GET, "/api/v1/auth/me", Some(&ns), None).await;
    assert_eq!(me.json["is_admin"], true);

    // Organisations: create, invite, change quota, refuse to lock oneself out.
    let r = call(
        &app,
        Method::POST,
        "/api/v1/admin/orgs",
        Some(&ns),
        Some(serde_json::json!({"name": "Bureau Y", "monthly_quota": 7})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json);
    let y = r.json["id"].as_str().expect("id").to_string();
    let r = call(
        &app,
        Method::POST,
        "/api/v1/admin/orgs",
        Some(&ns),
        Some(serde_json::json!({"name": "bureau y", "monthly_quota": 1})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "duplicate name");
    let r = call(
        &app,
        Method::POST,
        &format!("/api/v1/admin/orgs/{y}/invites"),
        Some(&ns),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.cache.as_deref(), Some("private, no-store"));
    let path = r.json["path"].as_str().expect("path").to_string();
    let token = path.strip_prefix("/invite#").expect("invite path");
    let info = call(
        &app,
        Method::POST,
        "/api/v1/auth/link",
        None,
        Some(serde_json::json!({"token": token})),
    )
    .await;
    assert_eq!(info.json["org_name"], "Bureau Y");
    let invites = call(
        &app,
        Method::GET,
        &format!("/api/v1/admin/orgs/{y}/invites"),
        Some(&ns),
        None,
    )
    .await;
    let inv_id = invites.json[0]["id"]
        .as_str()
        .expect("invite id")
        .to_string();
    assert_ne!(inv_id, token, "the token itself is never listed");
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/v1/admin/invites/{inv_id}/revoke"),
            Some(&ns),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/v1/auth/link",
            None,
            Some(serde_json::json!({"token": token}))
        )
        .await
        .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let r = call(
        &app,
        Method::PATCH,
        &format!("/api/v1/admin/orgs/{y}"),
        Some(&ns),
        Some(serde_json::json!({"monthly_quota": 12})),
    )
    .await;
    assert_eq!(r.json["monthly_quota"], 12);
    let r = call(
        &app,
        Method::PATCH,
        "/api/v1/admin/orgs/Natural%20Solutions",
        Some(&ns),
        Some(serde_json::json!({"disabled": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "own organisation");

    // Users: list, refuse self-disable, disable another account, reset link.
    let users = call(&app, Method::GET, "/api/v1/admin/users", Some(&ns), None).await;
    assert_eq!(users.json["users"].as_array().map(Vec::len), Some(2));
    assert_eq!(users.json["total"], 2);
    // Filtered and paged on the server; one account and one organisation by id.
    let r = call(
        &app,
        Method::GET,
        "/api/v1/admin/users?q=CLIENT&limit=1",
        Some(&ns),
        None,
    )
    .await;
    assert_eq!(
        (
            r.json["total"].as_u64(),
            r.json["users"].as_array().map(Vec::len)
        ),
        (Some(1), Some(1))
    );
    let id = r.json["users"][0]["id"].as_str().expect("id").to_string();
    let r = call(
        &app,
        Method::GET,
        &format!("/api/v1/admin/users/{id}"),
        Some(&ns),
        None,
    )
    .await;
    assert_eq!(r.json["email"], "c@client.fr");
    let r = call(
        &app,
        Method::GET,
        "/api/v1/admin/users?limit=1",
        Some(&ns),
        None,
    )
    .await;
    assert_eq!(r.json["next_offset"], 1);
    let r = call(
        &app,
        Method::GET,
        "/api/v1/admin/users/nope",
        Some(&ns),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call(
        &app,
        Method::GET,
        "/api/v1/admin/orgs/Client",
        Some(&ns),
        None,
    )
    .await;
    assert_eq!(
        (r.status, r.json["name"].as_str()),
        (StatusCode::OK, Some("Client"))
    );
    let r = call(
        &app,
        Method::POST,
        "/api/v1/admin/users/disable",
        Some(&ns),
        Some(serde_json::json!({"email": "ADMIN@ns.fr", "disabled": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    let r = call(
        &app,
        Method::POST,
        "/api/v1/admin/users/reset",
        Some(&ns),
        Some(serde_json::json!({"email": "c@client.fr"})),
    )
    .await;
    assert!(
        r.json["path"]
            .as_str()
            .is_some_and(|p| p.starts_with("/reset#"))
    );

    // Read access to a client's run, logged; deleting stays with the owner.
    let job = call(
        &app,
        Method::POST,
        "/api/v1/jobs",
        Some(&client),
        Some(serde_json::to_value(spec(1)).expect("spec")),
    )
    .await;
    let id = job.json["id"].as_str().expect("id").to_string();
    let (e2, id2) = (engine.clone(), id.clone());
    tokio::task::spawn_blocking(move || {
        e2.run_job(&id2, &AtomicBool::new(false), &AtomicBool::new(false))
    })
    .await
    .expect("run");
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/v1/jobs/{id}/results"),
            Some(&ns),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/v1/jobs/{id}"),
            Some(&ns),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, Method::GET, "/api/v1/jobs", Some(&ns), None)
            .await
            .json,
        serde_json::json!([]),
        "workspace lists own runs only"
    );
    assert_eq!(
        call(
            &app,
            Method::DELETE,
            &format!("/api/v1/jobs/{id}"),
            Some(&ns),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    let act = call(&app, Method::GET, "/api/v1/admin/activity", Some(&ns), None).await;
    assert_eq!(act.json[0]["org_name"], "Client");
    assert_eq!(act.json[0]["created_by"], "c@client.fr");

    // Usage: summary per organisation for a month, CSV for invoicing, month validated.
    let month = niche_app::accounts::current_month();
    let us = call(
        &app,
        Method::GET,
        &format!("/api/v1/admin/usage?month={month}"),
        Some(&ns),
        None,
    )
    .await;
    let client_row = us
        .json
        .as_array()
        .expect("rows")
        .iter()
        .find(|r| r["org_name"] == "Client")
        .cloned()
        .expect("client");
    assert_eq!(client_row["species"], 1);
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/v1/admin/usage?month=2026-13",
            Some(&ns),
            None
        )
        .await
        .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let req = Request::builder()
        .uri(format!("/api/v1/admin/usage.csv?month={month}"))
        .header(header::HOST, "localhost")
        .header(header::COOKIE, &ns)
        .body(Body::empty())
        .expect("req");
    let res = app.clone().oneshot(req).await.expect("csv");
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        res.headers()[header::CONTENT_DISPOSITION]
            .to_str()
            .expect("cd")
            .contains(".csv")
    );
    let body = String::from_utf8(
        res.into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes()
            .to_vec(),
    )
    .expect("utf8");
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines[0], "organisation,month,job_id,species_key,counted_at");
    assert!(
        lines[1].starts_with(&format!("Client,{month},{id},")),
        "{body}"
    );

    // Demotion takes effect on the next request.
    engine
        .store()
        .user_set_admin("admin@ns.fr", false)
        .expect("demote");
    assert_eq!(
        call(&app, Method::GET, "/api/v1/admin/orgs", Some(&ns), None)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/v1/jobs/{id}"),
            Some(&ns),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn administration_is_absent_in_local_mode() {
    let dir = tmp("admin-local");
    let (_engine, app) = app(&dir, false);
    assert_eq!(
        call(&app, Method::GET, "/api/v1/admin/orgs", None, None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn administrators_can_erase_an_account() {
    let dir = tmp("erase");
    let (engine, app) = app(&dir, true);
    let ns = sign_up(&app, &engine, "Natural Solutions", "admin@ns.fr").await;
    let client = sign_up(&app, &engine, "Client", "c@client.fr").await;
    let delete = |cookie: String, email: &'static str, confirm: &'static str| {
        let app = app.clone();
        async move {
            call(
                &app,
                Method::POST,
                "/api/v1/admin/users/delete",
                Some(&cookie),
                Some(serde_json::json!({"email": email, "confirm_email": confirm})),
            )
            .await
            .status
        }
    };
    assert_eq!(
        delete(client.clone(), "admin@ns.fr", "admin@ns.fr").await,
        StatusCode::FORBIDDEN,
        "members cannot erase accounts"
    );
    promote(&engine, "admin@ns.fr");
    assert_eq!(
        delete(ns.clone(), "c@client.fr", "other@client.fr").await,
        StatusCode::UNPROCESSABLE_ENTITY,
        "confirmation must repeat the e-mail"
    );
    assert_eq!(
        delete(ns.clone(), "admin@ns.fr", "ADMIN@ns.fr").await,
        StatusCode::CONFLICT,
        "no self-erasure"
    );
    assert_eq!(
        delete(ns.clone(), "c@client.fr", " C@client.fr ").await,
        StatusCode::NO_CONTENT
    );
    // The erased account's session is gone and it no longer appears anywhere.
    assert_eq!(
        call(&app, Method::GET, "/api/v1/auth/me", Some(&client), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let users = call(&app, Method::GET, "/api/v1/admin/users", Some(&ns), None).await;
    assert!(!users.json.to_string().contains("c@client.fr"));
    assert_eq!(
        delete(ns.clone(), "c@client.fr", "c@client.fr").await,
        StatusCode::NOT_FOUND
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn progress_stream_ends_when_the_session_does() {
    let dir = tmp("sse");
    let (engine, app) = app(&dir, true);
    let a = sign_up(&app, &engine, "A", "a@a.fr").await;
    let r = call(
        &app,
        Method::POST,
        "/api/v1/jobs",
        Some(&a),
        Some(serde_json::to_value(spec(1)).expect("spec")),
    )
    .await;
    let id = r.json["id"].as_str().expect("id").to_string();
    // The job is never run here: without the session check the stream would last forever.
    let req = Request::builder()
        .uri(format!("/api/v1/jobs/{id}/events"))
        .header(header::HOST, "localhost:8080")
        .header(header::COOKIE, &a)
        .body(Body::empty())
        .expect("request");
    let res = app.clone().oneshot(req).await.expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let body = tokio::spawn(async move { res.into_body().collect().await.map(|b| b.to_bytes()) });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!body.is_finished(), "stream open while signed in");
    call(&app, Method::POST, "/api/v1/auth/logout", Some(&a), None).await;
    let ended = tokio::time::timeout(std::time::Duration::from_secs(5), body).await;
    assert!(ended.is_ok(), "stream still open after sign-out");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn projects_and_study_updates_stay_within_the_organisation() {
    let dir = tmp("projects");
    let (engine, app) = app(&dir, true);
    let a = sign_up(&app, &engine, "A", "a@a.fr").await;
    let b = sign_up(&app, &engine, "B", "b@b.fr").await;
    let p = call(
        &app,
        Method::POST,
        "/api/v1/projects",
        Some(&a),
        Some(serde_json::json!({"name": "Client X"})),
    )
    .await;
    assert_eq!(p.status, StatusCode::CREATED, "{}", p.json);
    let pid = p.json["id"].as_str().expect("id").to_string();
    // A study launched into the project.
    let j = call(
        &app,
        Method::POST,
        &format!("/api/v1/jobs?project_id={pid}"),
        Some(&a),
        Some(serde_json::to_value(spec(1)).expect("spec")),
    )
    .await;
    assert_eq!(j.status, StatusCode::CREATED, "{}", j.json);
    assert_eq!(j.json["project_id"], pid.as_str());
    let jid = j.json["id"].as_str().expect("id").to_string();
    let list = call(&app, Method::GET, "/api/v1/projects", Some(&a), None).await;
    assert_eq!(list.json[0]["studies"], 1);
    // Organisation B sees nothing of A's project and cannot touch it or A's study.
    let lb = call(&app, Method::GET, "/api/v1/projects", Some(&b), None).await;
    assert_eq!(lb.json, serde_json::json!([]));
    for (method, path, body) in [
        (
            Method::PATCH,
            format!("/api/v1/projects/{pid}"),
            Some(serde_json::json!({"name": "x"})),
        ),
        (Method::DELETE, format!("/api/v1/projects/{pid}"), None),
        (
            Method::PATCH,
            format!("/api/v1/jobs/{jid}"),
            Some(serde_json::json!({"name": "x"})),
        ),
        (
            Method::POST,
            format!("/api/v1/jobs?project_id={pid}"),
            Some(serde_json::to_value(spec(1)).expect("spec")),
        ),
    ] {
        let r = call(&app, method.clone(), &path, Some(&b), body).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{method} {path}");
    }
    // Rename and move out of the project (null = no project).
    let u = call(
        &app,
        Method::PATCH,
        &format!("/api/v1/jobs/{jid}"),
        Some(&a),
        Some(serde_json::json!({"name": "Lacerta · Bretagne", "project_id": null})),
    )
    .await;
    assert_eq!(u.status, StatusCode::OK, "{}", u.json);
    assert_eq!(u.json["name"], "Lacerta · Bretagne");
    assert_eq!(u.json["project_id"], serde_json::Value::Null);
    // Renaming only leaves the project untouched.
    let u = call(
        &app,
        Method::PATCH,
        &format!("/api/v1/jobs/{jid}"),
        Some(&a),
        Some(serde_json::json!({"project_id": pid})),
    )
    .await;
    assert_eq!(u.json["project_id"], pid.as_str());
    let u = call(
        &app,
        Method::PATCH,
        &format!("/api/v1/jobs/{jid}"),
        Some(&a),
        Some(serde_json::json!({"name": "Renamed"})),
    )
    .await;
    assert_eq!(u.json["project_id"], pid.as_str());
    let d = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/projects/{pid}"),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(d.status, StatusCode::NO_CONTENT);
    let after = call(
        &app,
        Method::GET,
        &format!("/api/v1/jobs/{jid}"),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(after.json["project_id"], serde_json::Value::Null);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn places_come_from_the_installed_boundary_pack() {
    use niche_geo::boundaries::{Level, Source, UnitIn, build};
    let dir = tmp("places");
    // A two-level pack installed where the server looks for it.
    let unit = |id: &str, name: &str, level, parent: Option<&str>, x0: f64, s: f64| UnitIn {
        id: id.into(),
        name: name.into(),
        level,
        country: "FR".into(),
        parent: parent.map(String::from),
        population: None,
        aliases: vec![],
        designation: None,
        source: None,
        geometry: serde_json::json!({"type": "Polygon", "coordinates": [[[x0, 44.0], [x0 + s, 44.0], [x0 + s, 44.0 + s], [x0, 44.0 + s], [x0, 44.0]]]}),
    };
    let source = Source {
        country: "FR".into(),
        name: "test".into(),
        licence: "Licence Ouverte 2.0".into(),
        attribution: "© IGN test".into(),
        url: String::new(),
        edition: "t".into(),
        sha256: String::new(),
    };
    let bytes = build(
        vec![
            unit("FR-D26", "Drôme", Level::Department, None, 4.0, 1.0),
            unit(
                "FR-C26362",
                "Valence",
                Level::Municipality,
                Some("FR-D26"),
                4.8,
                0.1,
            ),
        ],
        vec![source],
        "test-1",
        "2026-10-16",
    )
    .expect("pack");
    std::fs::create_dir_all(dir.join("data/boundaries")).expect("dir");
    std::fs::write(dir.join("data/boundaries/test-1.nrsb"), bytes).expect("write");
    let (engine, app) = app(&dir, true);
    assert_eq!(engine.boundary_packs(), vec![("test-1".to_string(), 2)]);
    // Behind the session like every workspace route.
    assert_eq!(
        call(&app, Method::GET, "/api/v1/places?q=drome", None, None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let a = sign_up(&app, &engine, "A", "a@a.fr").await;
    let r = call(&app, Method::GET, "/api/v1/places?q=drome", Some(&a), None).await;
    assert_eq!(r.json[0]["id"], "FR-D26", "{}", r.json);
    assert_eq!(r.json[0]["level"], "department");
    assert_eq!(r.json[0]["attribution"], "© IGN test — Licence Ouverte 2.0");
    let r = call(
        &app,
        Method::GET,
        "/api/v1/places?q=valence",
        Some(&a),
        None,
    )
    .await;
    assert_eq!(r.json[0]["within"], serde_json::json!(["Drôme"]));
    let r = call(
        &app,
        Method::GET,
        "/api/v1/places/at?lng=4.85&lat=44.05",
        Some(&a),
        None,
    )
    .await;
    let ids: Vec<&str> = r
        .json
        .as_array()
        .expect("array")
        .iter()
        .map(|h| h["id"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(ids, ["FR-D26", "FR-C26362"]);
    let g = call(
        &app,
        Method::GET,
        "/api/v1/places/FR-C26362",
        Some(&a),
        None,
    )
    .await;
    assert_eq!(g.json["geometry"]["type"], "MultiPolygon");
    assert_eq!(g.json["place"]["pack"], "test-1");
    assert_eq!(
        call(&app, Method::GET, "/api/v1/places/FR-X", Some(&a), None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A second account in an existing organisation.
async fn join(app: &Router, engine: &Engine, org: &str, email: &str) -> String {
    let token = engine.store().invite(org).expect("invite");
    let r = call(
        app,
        Method::POST,
        "/api/v1/auth/accept",
        None,
        Some(serde_json::json!({"token": token, "name": "Member", "email": email, "password": "a long enough password"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    r.cookie.expect("cookie")
}

#[tokio::test]
async fn managers_handle_their_own_organisation_only() {
    let dir = tmp("managers");
    let (engine, app) = app(&dir, true);
    let boss = sign_up(&app, &engine, "Client A", "boss@a.fr").await;
    let colleague = join(&app, &engine, "Client A", "colleague@a.fr").await;
    let _ns_in_a = join(&app, &engine, "Client A", "support@ns.fr").await;
    promote(&engine, "support@ns.fr");
    let _other = sign_up(&app, &engine, "Client B", "x@b.fr").await;

    // A plain member manages nothing.
    let r = call(&app, Method::GET, "/api/v1/org/members", Some(&boss), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    engine
        .store()
        .user_set_manager("boss@a.fr", true)
        .expect("manager");
    let me = call(&app, Method::GET, "/api/v1/auth/me", Some(&boss), None).await;
    assert_eq!(me.json["is_manager"], true);

    // Members of their organisation only.
    let r = call(&app, Method::GET, "/api/v1/org/members", Some(&boss), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let emails: Vec<&str> = r
        .json
        .as_array()
        .expect("members")
        .iter()
        .filter_map(|u| u["email"].as_str())
        .collect();
    assert_eq!(emails, ["boss@a.fr", "colleague@a.fr", "support@ns.fr"]);

    // Invitations into their organisation, listed and revocable.
    let r = call(&app, Method::POST, "/api/v1/org/invites", Some(&boss), None).await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert!(
        r.json["path"]
            .as_str()
            .is_some_and(|p| p.starts_with("/invite#"))
    );
    let r = call(&app, Method::GET, "/api/v1/org/invites", Some(&boss), None).await;
    let id = r.json[0]["id"].as_str().expect("invite id").to_string();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/v1/org/invites/{id}/revoke"),
        Some(&boss),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    // Another organisation's invitation is not theirs to revoke.
    engine.store().invite("Client B").expect("invite B");
    let b_id = engine.store().open_invites("Client B").expect("B")[0]
        .id
        .clone();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/v1/org/invites/{b_id}/revoke"),
        Some(&boss),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // Reset links and disabling: members of their organisation, never others.
    let reset = |email: &str| serde_json::json!({ "email": email });
    for (email, want) in [
        ("colleague@a.fr", StatusCode::CREATED),
        ("x@b.fr", StatusCode::NOT_FOUND),
        ("boss@a.fr", StatusCode::CONFLICT),
        ("support@ns.fr", StatusCode::FORBIDDEN),
    ] {
        let r = call(
            &app,
            Method::POST,
            "/api/v1/org/members/reset",
            Some(&boss),
            Some(reset(email)),
        )
        .await;
        assert_eq!(r.status, want, "{email}: {}", r.json);
    }
    let r = call(
        &app,
        Method::POST,
        "/api/v1/org/members/disable",
        Some(&boss),
        Some(serde_json::json!({"email": "colleague@a.fr", "disabled": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let r = call(&app, Method::GET, "/api/v1/auth/me", Some(&colleague), None).await;
    assert_eq!(
        r.status,
        StatusCode::UNAUTHORIZED,
        "disabling closes the session"
    );

    // Managers are not administrators.
    for path in [
        "/api/v1/admin/system",
        "/api/v1/admin/metrics",
        "/api/v1/admin/users",
    ] {
        let r = call(&app, Method::GET, path, Some(&boss), None).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{path}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn administrators_see_system_state_and_metrics_and_assign_managers() {
    let dir = tmp("system");
    let (engine, app) = app(&dir, true);
    let ns = sign_up(&app, &engine, "Natural Solutions", "admin@ns.fr").await;
    let _client = sign_up(&app, &engine, "Client", "c@client.fr").await;
    promote(&engine, "admin@ns.fr");
    let r = call(&app, Method::GET, "/api/v1/admin/system", Some(&ns), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["build"], "test");
    assert_eq!(r.json["gbif"]["configured"], false);
    assert_eq!(r.json["db_backups"]["keep"], 7);
    assert!(r.json["jobs"]["total"].is_u64());
    let r = call(
        &app,
        Method::GET,
        "/api/v1/admin/metrics?month=2026-10",
        Some(&ns),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.json.is_array());
    let r = call(
        &app,
        Method::POST,
        "/api/v1/admin/users/manager",
        Some(&ns),
        Some(serde_json::json!({"email": "c@client.fr", "manager": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let users = call(&app, Method::GET, "/api/v1/admin/users", Some(&ns), None).await;
    let c = users.json["users"]
        .as_array()
        .expect("users")
        .iter()
        .find(|u| u["email"] == "c@client.fr")
        .cloned()
        .expect("client");
    assert_eq!(c["is_manager"], true);
    // The database copy is taken once a day.
    assert!(engine.backup_db_if_due().expect("copy").is_some());
    assert!(engine.backup_db_if_due().expect("copy").is_none());
    let r = call(&app, Method::GET, "/api/v1/admin/system", Some(&ns), None).await;
    assert_eq!(r.json["db_backups"]["count"], 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn account_and_organisation_properties() {
    let dir = tmp("properties");
    let (engine, app) = app(&dir, true);
    let boss = sign_up(&app, &engine, "Client P", "boss@p.fr").await;
    let member = join(&app, &engine, "Client P", "member@p.fr").await;
    engine
        .store()
        .user_set_manager("boss@p.fr", true)
        .expect("manager");

    // Name.
    let r = call(
        &app,
        Method::PATCH,
        "/api/v1/account",
        Some(&member),
        Some(serde_json::json!({"name": "Mia Member"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.json);
    let r = call(&app, Method::GET, "/api/v1/auth/me", Some(&member), None).await;
    assert_eq!(r.json["name"], "Mia Member");

    // Own GBIF account: stored, never echoed, no shared account on this server.
    let r = call(
        &app,
        Method::GET,
        "/api/v1/account/gbif",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["effective"], "none");
    assert_eq!(r.json["can_store"], true);
    let r = call(
        &app,
        Method::PUT,
        "/api/v1/account/gbif",
        Some(&member),
        Some(serde_json::json!({"username": "mia-gbif", "password": "very-secret-gbif"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["effective"], "user");
    assert_eq!(r.json["own"]["username"], "mia-gbif");
    assert!(!r.json.to_string().contains("very-secret-gbif"));
    assert!(r.cache.as_deref().is_some_and(|c| c.contains("no-store")));

    // The organisation's account: managers only; members see that it exists, not its name.
    let body = serde_json::json!({"username": "client-p", "password": "org-secret-gbif"});
    let r = call(
        &app,
        Method::PUT,
        "/api/v1/org/gbif",
        Some(&member),
        Some(body.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = call(
        &app,
        Method::PUT,
        "/api/v1/org/gbif",
        Some(&boss),
        Some(body),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["effective"], "organisation");
    let r = call(&app, Method::GET, "/api/v1/org", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["name"], "Client P");
    assert_eq!(r.json["members"], 2);
    assert_eq!(r.json["can_manage"], false);
    assert!(r.json["gbif"].is_object() && r.json["gbif"]["username"].is_null());
    let r = call(&app, Method::GET, "/api/v1/org", Some(&boss), None).await;
    assert_eq!(r.json["gbif"]["username"], "client-p");
    assert!(!r.json.to_string().contains("org-secret-gbif"));
    let r = call(
        &app,
        Method::DELETE,
        "/api/v1/org/gbif",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // Removing one's own account falls back to the organisation's.
    let r = call(
        &app,
        Method::DELETE,
        "/api/v1/account/gbif",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let r = call(
        &app,
        Method::GET,
        "/api/v1/account/gbif",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.json["effective"], "organisation");
    assert!(r.json["own"].is_null());

    // Password change: wrong current refused; the session making the change stays open.
    let r = call(
        &app,
        Method::POST,
        "/api/v1/account/password",
        Some(&member),
        Some(serde_json::json!({"current": "not the password", "new": "a brand new password"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.json);
    let r = call(
        &app,
        Method::POST,
        "/api/v1/account/password",
        Some(&member),
        Some(
            serde_json::json!({"current": "a long enough password", "new": "a brand new password"}),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.json);
    let r = call(&app, Method::GET, "/api/v1/auth/me", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK);

    // Local mode has no accounts.
    let (_, local) = crate::app(&tmp("properties-local"), false);
    let r = call(&local, Method::GET, "/api/v1/account/gbif", None, None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn feedback_reaches_administrators_and_the_organisation_managers() {
    let dir = tmp("feedback");
    let (engine, app) = app(&dir, true);
    let member = sign_up(&app, &engine, "Client A", "member@a.fr").await;
    let boss = join(&app, &engine, "Client A", "boss@a.fr").await;
    engine
        .store()
        .user_set_manager("boss@a.fr", true)
        .expect("manager");
    let other = sign_up(&app, &engine, "Client B", "other@b.fr").await;
    let ns = sign_up(&app, &engine, "NS", "ns@ns.fr").await;
    promote(&engine, "ns@ns.fr");
    let send = |cookie: Option<String>, body: serde_json::Value| {
        let app = app.clone();
        async move {
            call(
                &app,
                Method::POST,
                "/api/v1/feedback",
                cookie.as_deref(),
                Some(body),
            )
            .await
        }
    };
    // Nothing to send, a forbidden type, broken base64: refused.
    for body in [
        serde_json::json!({"message": "  "}),
        serde_json::json!({"attachments": [{"name": "x.svg", "mime": "image/svg+xml", "data": "PHN2Zz4="}]}),
        serde_json::json!({"attachments": [{"name": "x.png", "mime": "image/png", "data": "!!"}]}),
    ] {
        let r = send(Some(member.clone()), body).await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.json);
    }
    assert_eq!(
        send(None, serde_json::json!({"message": "hi"}))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    // A voice recording alone is enough; the server records its own build.
    let r = send(
        Some(member.clone()),
        serde_json::json!({
            "kind": "problem",
            "message": "The map stays grey",
            "page": "/jobs/x/results",
            "context": {"lang": "es", "build": "forged"},
            "attachments": [{"name": "voice.webm", "mime": "audio/webm;codecs=opus", "data": "GkXfow=="}],
        }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json);
    let id = r.json["id"].as_str().expect("id").to_string();
    let r = send(
        Some(other.clone()),
        serde_json::json!({"kind": "idea", "message": "B"}),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);

    // Administrators: everything, newest first, with author and organisation.
    let all = call(&app, Method::GET, "/api/v1/admin/feedback", Some(&ns), None).await;
    assert_eq!(all.status, StatusCode::OK);
    let items = all.json.as_array().expect("list");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["org_name"], "Client B");
    let a = &items[1];
    assert_eq!(a["kind"], "problem");
    assert_eq!(a["author_email"], "member@a.fr");
    assert_eq!(a["context"]["build"], "test");
    assert_eq!(a["status"], "open");
    assert_eq!(a["files"][0]["mime"], "audio/webm");
    assert_eq!(a["files"][0]["size"], 4);
    // Nothing about where feedback is forwarded reaches the API.
    let text = all.json.to_string();
    assert!(
        !text.contains("forward") && !text.to_lowercase().contains("gitlab"),
        "{text}"
    );
    let file_id = a["files"][0]["id"].as_str().expect("file").to_string();
    let file = format!("/api/v1/feedback/{id}/files/{file_id}");

    // Managers: their organisation only; members and other organisations: no list.
    let org = call(&app, Method::GET, "/api/v1/org/feedback", Some(&boss), None).await;
    assert_eq!(org.status, StatusCode::OK);
    assert_eq!(org.json.as_array().expect("list").len(), 1);
    for c in [&member, &other] {
        assert_eq!(
            call(&app, Method::GET, "/api/v1/admin/feedback", Some(c), None)
                .await
                .status,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/v1/org/feedback",
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    // Attachments: author, manager and administrators; sandboxed.
    for c in [&member, &boss, &ns] {
        let r = call(&app, Method::GET, &file, Some(c), None).await;
        assert_eq!(r.status, StatusCode::OK);
        assert!(r.csp.as_deref().is_some_and(|p| p.starts_with("sandbox")));
    }
    assert_eq!(
        call(&app, Method::GET, &file, Some(&other), None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );

    // Closing is for administrators.
    let close = serde_json::json!({"status": "closed"});
    let path = format!("/api/v1/admin/feedback/{id}");
    assert_eq!(
        call(&app, Method::PATCH, &path, Some(&boss), Some(close.clone()))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, Method::PATCH, &path, Some(&ns), Some(close))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let org = call(&app, Method::GET, "/api/v1/org/feedback", Some(&boss), None).await;
    assert_eq!(org.json[0]["status"], "closed");

    // An erased account's feedback stays with the organisation, without author.
    engine.store().user_delete("member@a.fr").expect("erase");
    let org = call(&app, Method::GET, "/api/v1/org/feedback", Some(&boss), None).await;
    assert_eq!(org.json[0]["author"], serde_json::Value::Null);
    assert_eq!(org.json[0]["message"], "The map stays grey");

    assert_eq!(
        call(&app, Method::DELETE, &path, Some(&ns), None)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&app, Method::GET, &file, Some(&ns), None).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn feedback_needs_accounts() {
    let dir = tmp("feedback-local");
    let (_engine, app) = app(&dir, false);
    let r = call(
        &app,
        Method::POST,
        "/api/v1/feedback",
        None,
        Some(serde_json::json!({"message": "hi"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn administrators_grant_the_role_to_the_operator_organisation_with_their_password() {
    let dir = tmp("grant");
    let (engine, app) = app(&dir, true);
    let ns = sign_up(&app, &engine, "Natural Solutions", "admin@ns.fr").await;
    let colleague = join(&app, &engine, "Natural Solutions", "colleague@ns.fr").await;
    let _client = sign_up(&app, &engine, "Client", "c@client.fr").await;
    promote(&engine, "admin@ns.fr");
    let grant = |cookie: String, email: &str, password: &str| {
        let (app, body) = (
            app.clone(),
            serde_json::json!({"email": email, "password": password}),
        );
        async move {
            call(
                &app,
                Method::POST,
                "/api/v1/admin/users/admin",
                Some(&cookie),
                Some(body),
            )
            .await
        }
    };
    let pw = "a long enough password";
    assert_eq!(
        grant(colleague.clone(), "colleague@ns.fr", pw).await.status,
        StatusCode::FORBIDDEN,
        "members cannot grant it"
    );
    // A session alone is not enough: the administrator's own password is checked.
    let r = grant(ns.clone(), "colleague@ns.fr", "a stolen cookie only").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    // Only members of the operator organisation.
    let r = grant(ns.clone(), "c@client.fr", pw).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.json);
    let r = grant(ns.clone(), "colleague@ns.fr", pw).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.json);
    let me = call(&app, Method::GET, "/api/v1/auth/me", Some(&colleague), None).await;
    assert_eq!(me.json["is_admin"], true);
    let users = call(&app, Method::GET, "/api/v1/admin/users", Some(&ns), None).await;
    let c = users.json["users"]
        .as_array()
        .and_then(|a| a.iter().find(|u| u["email"] == "colleague@ns.fr"))
        .expect("listed");
    assert_eq!(c["in_operator_org"], true);
    // The operator organisation cannot be disabled from the web, even by an administrator of
    // another organisation.
    engine.store().org_set_operator("Client").expect("operator");
    let orgs = call(&app, Method::GET, "/api/v1/admin/orgs", Some(&ns), None).await;
    let client = orgs
        .json
        .as_array()
        .and_then(|a| a.iter().find(|o| o["name"] == "Client"))
        .expect("listed");
    assert_eq!(client["is_operator"], true);
    let id = client["id"].as_str().expect("id");
    let r = call(
        &app,
        Method::PATCH,
        &format!("/api/v1/admin/orgs/{id}"),
        Some(&ns),
        Some(serde_json::json!({"disabled": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.json);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn administration_changes_and_support_access_are_in_the_audit_trail() {
    let dir = tmp("audit");
    let (engine, app) = app(&dir, true);
    let ns = sign_up(&app, &engine, "Natural Solutions", "admin@ns.fr").await;
    promote(&engine, "admin@ns.fr");
    let boss = sign_up(&app, &engine, "Client", "boss@client.fr").await;
    let colleague = join(&app, &engine, "Client", "colleague@client.fr").await;
    engine
        .store()
        .user_set_manager("boss@client.fr", true)
        .expect("manager");
    let client = engine.store().org("Client").expect("org");
    // An administrator changes the quota and opens a client's study (once per hour: one row).
    let r = call(
        &app,
        Method::PATCH,
        &format!("/api/v1/admin/orgs/{}", client.id),
        Some(&ns),
        Some(serde_json::json!({"monthly_quota": 9})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let r = call(
        &app,
        Method::POST,
        "/api/v1/jobs",
        Some(&boss),
        Some(serde_json::to_value(spec(1)).expect("spec")),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json);
    let job = r.json["id"].as_str().expect("id").to_string();
    for _ in 0..2 {
        let r = call(
            &app,
            Method::GET,
            &format!("/api/v1/jobs/{job}"),
            Some(&ns),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK);
    }
    // A manager disables a colleague.
    let r = call(
        &app,
        Method::POST,
        "/api/v1/org/members/disable",
        Some(&boss),
        Some(serde_json::json!({"email": "colleague@client.fr", "disabled": true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.json);

    let audit = |q: String, cookie: String| {
        let app = app.clone();
        async move {
            call(
                &app,
                Method::GET,
                &format!("/api/v1/admin/audit{q}"),
                Some(&cookie),
                None,
            )
            .await
        }
    };
    assert_eq!(
        audit(String::new(), boss.clone()).await.status,
        StatusCode::FORBIDDEN
    );
    let r = audit(format!("?org={}", client.id), ns.clone()).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.cache.as_deref(), Some("private, no-store"));
    let entries = r.json["entries"].as_array().expect("entries");
    let actions: Vec<_> = entries
        .iter()
        .map(|e| e["action"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        actions,
        ["account_disabled", "support_access", "org_quota_changed"]
    );
    assert_eq!(entries[0]["actor_name"], "User");
    assert_eq!(entries[0]["user_email"], "colleague@client.fr");
    assert_eq!(entries[1]["job_id"], job.as_str());
    assert_eq!(
        entries[2]["detail"],
        serde_json::json!({"from": 3, "to": 9})
    );
    // Pages.
    let r = audit(format!("?org={}&limit=2", client.id), ns.clone()).await;
    let next = r.json["next"].as_i64().expect("more");
    let r = audit(
        format!("?org={}&limit=2&before={next}", client.id),
        ns.clone(),
    )
    .await;
    assert_eq!(r.json["entries"].as_array().map(Vec::len), Some(1));
    assert!(r.json["next"].is_null());
    let _ = colleague;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn managers_read_their_organisation_history_only() {
    let dir = tmp("org-audit");
    let (engine, app) = app(&dir, true);
    let boss = sign_up(&app, &engine, "Client", "boss@client.fr").await;
    let colleague = join(&app, &engine, "Client", "colleague@client.fr").await;
    let other = sign_up(&app, &engine, "Other", "x@other.fr").await;
    for email in ["boss@client.fr", "x@other.fr"] {
        engine
            .store()
            .user_set_manager(email, true)
            .expect("manager");
    }
    for cookie in [&boss, &other] {
        let r = call(
            &app,
            Method::POST,
            "/api/v1/org/invites",
            Some(cookie),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::CREATED);
    }
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/v1/org/audit",
            Some(&colleague),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN,
        "members do not see it"
    );
    let r = call(&app, Method::GET, "/api/v1/org/audit", Some(&boss), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let entries = r.json["entries"].as_array().expect("entries");
    assert_eq!(entries.len(), 1, "only its own organisation: {}", r.json);
    assert_eq!(entries[0]["action"], "invite_created");
    assert_eq!(entries[0]["org_name"], "Client");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A small SYNTHETIC import in Lambert-93 inside the demo region (ADR-0108), with a witness
/// value that must never reach another organisation.
fn synthetic_import() -> serde_json::Value {
    let pts = [
        (4.0, 44.5, "Présent"),
        (4.5, 45.0, "Non observé"),
        (5.0, 44.2, "Présent"),
    ];
    let (mut x, mut y, mut st) = (Vec::new(), Vec::new(), Vec::new());
    for (lng, lat, s) in pts {
        let (px, py) = niche_geo::crs::Crs::Lambert93.from_wgs84(lng, lat);
        x.push(format!("{px:.3}"));
        y.push(format!("{py:.3}"));
        st.push(s.to_string());
    }
    x.push("654321.987".into());
    y.push("6400000".into());
    st.push("Présent".into());
    serde_json::json!({
        "name": "Synthetic survey", "file_name": "survey.csv", "file_sha256": "b".repeat(64),
        "crs": "EPSG:2154", "synthetic": true, "absent_values": ["Non observé"],
        "columns": {"taxon": vec!["Virtualis demonstrandum"; 4], "x": x, "y": y,
                    "date": vec!["2021"; 4], "status": st},
        "mapping": {"taxon": "nom", "x": "x", "y": "y"}
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn datasets_stay_within_the_organisation() {
    let dir = tmp("datasets");
    let (engine, app) = app(&dir, true);
    let a = sign_up(&app, &engine, "A", "a@a.fr").await;
    let b = sign_up(&app, &engine, "B", "b@b.fr").await;
    // Closed list of coordinate systems.
    let crs = call(
        &app,
        Method::GET,
        "/api/v1/coordinate-systems",
        Some(&a),
        None,
    )
    .await;
    assert!(
        crs.json
            .as_array()
            .is_some_and(|l| l.iter().any(|c| c["code"] == "EPSG:2154"))
    );
    let imp = call(
        &app,
        Method::POST,
        "/api/v1/datasets",
        Some(&a),
        Some(synthetic_import()),
    )
    .await;
    assert_eq!(imp.status, StatusCode::CREATED, "{}", imp.json);
    assert_eq!(imp.json["report"]["rows_imported"], 4);
    assert_eq!(imp.json["report"]["absences"], 1);
    // The answer describes the dataset; it never returns records.
    assert!(!imp.json.to_string().contains("654321"));
    let id = imp.json["id"].as_str().expect("id").to_string();
    let sha = imp.json["records_sha256"]
        .as_str()
        .expect("sha")
        .to_string();
    let la = call(&app, Method::GET, "/api/v1/datasets", Some(&a), None).await;
    assert_eq!(la.json.as_array().map(Vec::len), Some(1));
    let lb = call(&app, Method::GET, "/api/v1/datasets", Some(&b), None).await;
    assert_eq!(lb.json, serde_json::json!([]));
    // B can neither read, delete, preview nor use A's dataset.
    let own = serde_json::json!({"values": ["Virtualis demonstrandum"], "role": "validation", "gbif": true});
    let dref = serde_json::json!({"id": id, "records_sha256": sha});
    let mut job = serde_json::to_value(spec(1)).expect("spec");
    job["species"][0]["own"] = own.clone();
    job["own_dataset"] = dref.clone();
    let preview = serde_json::json!({
        "species_key": 0, "area": spec(1).area, "resolution_factor": 2, "demo": true,
        "own_dataset": dref, "own": own
    });
    for (method, path, body) in [
        (Method::GET, format!("/api/v1/datasets/{id}"), None),
        (Method::DELETE, format!("/api/v1/datasets/{id}"), None),
        (
            Method::POST,
            "/api/v1/occurrences/preview".to_string(),
            Some(preview.clone()),
        ),
        (Method::POST, "/api/v1/jobs".to_string(), Some(job.clone())),
    ] {
        let r = call(&app, method.clone(), &path, Some(&b), body).await;
        assert_eq!(
            r.status,
            StatusCode::NOT_FOUND,
            "{method} {path}: {}",
            r.json
        );
        assert!(!r.json.to_string().contains("654321"));
    }
    // Every dataset route is covered above (a new one must be added to this test).
    let declared: Vec<&str> = include_str!("../src/lib.rs")
        .lines()
        .filter_map(|l| l.trim().strip_prefix(".route(\"/datasets"))
        .filter_map(|l| l.split('"').next())
        .collect();
    assert_eq!(declared, ["", "/{id}"]);
    // A previews and launches its study; the study keeps a copy of the records.
    let p = call(
        &app,
        Method::POST,
        "/api/v1/occurrences/preview",
        Some(&a),
        Some(preview),
    )
    .await;
    assert_eq!(p.status, StatusCode::OK, "{}", p.json);
    assert!(p.json["validation_sites"]["records"] == 4);
    let mut stale = job.clone();
    stale["own_dataset"]["records_sha256"] = serde_json::json!("0".repeat(64));
    let r = call(&app, Method::POST, "/api/v1/jobs", Some(&a), Some(stale)).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.json);
    let j = call(&app, Method::POST, "/api/v1/jobs", Some(&a), Some(job)).await;
    assert_eq!(j.status, StatusCode::CREATED, "{}", j.json);
    let jid = j.json["id"].as_str().expect("id").to_string();
    let copy = dir
        .join("data/jobs")
        .join(&jid)
        .join("input/own-records.json");
    assert!(copy.exists());
    // Deleting the dataset leaves the submitted study's copy; deleting the study removes it.
    let d = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/datasets/{id}"),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(d.status, StatusCode::NO_CONTENT);
    assert!(!dir.join("data/datasets").join(&id).exists());
    assert!(copy.exists());
    call(
        &app,
        Method::POST,
        &format!("/api/v1/jobs/{jid}/cancel"),
        Some(&a),
        None,
    )
    .await;
    let d = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/jobs/{jid}"),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(d.status, StatusCode::NO_CONTENT, "{}", d.json);
    assert!(!copy.exists());
    // An import without a valid row is refused without quoting the file.
    let mut bad = synthetic_import();
    bad["crs"] = serde_json::json!("EPSG:4326");
    let r = call(&app, Method::POST, "/api/v1/datasets", Some(&a), Some(bad)).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.json);
    assert!(!r.json.to_string().contains("654321"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Submit a study over the API and run it to completion.
async fn run_study(app: &Router, engine: &Arc<Engine>, cookie: &str) -> String {
    let r = call(
        app,
        Method::POST,
        "/api/v1/jobs",
        Some(cookie),
        Some(serde_json::to_value(spec(1)).expect("spec")),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json);
    let id = r.json["id"].as_str().expect("id").to_string();
    let (e, id2) = (engine.clone(), id.clone());
    tokio::task::spawn_blocking(move || {
        let never = AtomicBool::new(false);
        e.run_job(&id2, &never, &never);
    })
    .await
    .expect("run");
    id
}

/// Decision-support analyses (ADR-0018): a comparison never reaches another organisation's
/// study, footprints are created and deleted by the owner only, and they go with the study.
#[tokio::test(flavor = "multi_thread")]
async fn analyses_stay_within_the_organisation() {
    let dir = tmp("analyses");
    let (engine, app) = app(&dir, true);
    let a = sign_up(&app, &engine, "A", "a@a.fr").await;
    let b = sign_up(&app, &engine, "B", "b@b.fr").await;
    let a1 = run_study(&app, &engine, &a).await;
    let a2 = run_study(&app, &engine, &a).await;
    let b1 = run_study(&app, &engine, &b).await;
    let compare = |other: &str| serde_json::json!({"kind": "comparison", "other_job_id": other, "species_key": 0});
    let post = |id: &str, cookie: &str, body: serde_json::Value| {
        let (app, uri, cookie) = (
            app.clone(),
            format!("/api/v1/jobs/{id}/analyses"),
            cookie.to_string(),
        );
        async move { call(&app, Method::POST, &uri, Some(&cookie), Some(body)).await }
    };
    // Another organisation's study cannot be compared with, in either direction.
    assert_eq!(
        post(&a1, &a, compare(&b1)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        post(&b1, &b, compare(&a1)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        post(&a1, &b, compare(&a2)).await.status,
        StatusCode::NOT_FOUND
    );
    let r = post(&a1, &a, compare(&a2)).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json);
    assert!(r.json["comparison"]["schoener_d"].as_f64().is_some());
    // The same comparison again: the stored one (200).
    assert_eq!(post(&a1, &a, compare(&a2)).await.status, StatusCode::OK);
    // A footprint: invalid geometry is a 422, a valid one is stored with the study.
    let bad = serde_json::json!({"kind": "footprint", "name": "x", "geometry": {"type": "Point", "coordinates": [4.0, 44.0]}});
    assert_eq!(
        post(&a1, &a, bad).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let fp = serde_json::json!({"kind": "footprint", "name": "Quarry (SYNTHETIC)", "geometry": {"type": "Polygon", "coordinates": [[[4.2, 44.6], [4.3, 44.6], [4.3, 44.7], [4.2, 44.7], [4.2, 44.6]]]}});
    let r = post(&a1, &a, fp).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.json);
    let fid = r.json["id"].as_str().expect("id").to_string();
    assert!(
        r.json["footprint"]["species"][0]["thresholds"][0]["suitable_km2"]
            .as_f64()
            .is_some()
    );
    let uri = format!("/api/v1/jobs/{a1}/analyses/{fid}");
    assert_eq!(
        call(&app, Method::DELETE, &uri, Some(&b), None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let csv = format!("{uri}/files/footprint_species.csv");
    let r = call(&app, Method::GET, &csv, Some(&a), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.disposition.is_some_and(|d| d.starts_with("attachment")));
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("{uri}/files/../../manifest.json"),
            Some(&a),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, Method::DELETE, &uri, Some(&a), None)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&app, Method::GET, &uri, Some(&a), None).await.status,
        StatusCode::NOT_FOUND
    );
    // Deleting the study deletes its analyses.
    let analyses = dir.join("data/jobs").join(&a1).join("analyses");
    assert!(analyses.exists());
    assert_eq!(
        call(
            &app,
            Method::DELETE,
            &format!("/api/v1/jobs/{a1}"),
            Some(&a),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert!(!analyses.exists());
    let _ = std::fs::remove_dir_all(&dir);
}
