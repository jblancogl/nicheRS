//! Feedback from every page (ADR-0111), forwarded to the operator's issue tracker.

use super::*;

/// The signed-in user (feedback needs an account: hosted mode only).
pub(crate) fn signed(s: &AppState, ctx: &Ctx) -> ApiResult<Principal> {
    accounts_only(s)?;
    ctx.principal
        .clone()
        .ok_or_else(|| ApiError(AppError::Forbidden("sign in to send feedback".into())))
}

/// Forward to the operator's GitLab in the background; a failure is kept on the item and
/// retried by [`forward_loop`].
pub(crate) fn forward_later(s: &AppState, id: String) {
    let Some(gl) = s.opts.feedback_gitlab.clone() else {
        return;
    };
    let e = s.engine.clone();
    tokio::task::spawn_blocking(
        move || match feedback::forward(e.store(), e.http(), &gl, &id) {
            Ok(Some(url)) => tracing::info!(feedback_id = %id, issue = %url, "feedback forwarded"),
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(feedback_id = %id, error = %err, "feedback not forwarded; will retry")
            }
        },
    );
}

/// Every 15 minutes, forward what could not be forwarded (GitLab down, token expired, restart).
pub(crate) async fn forward_loop(
    engine: Arc<Engine>,
    gl: GitLabProject,
    shutdown: Arc<AtomicBool>,
) {
    let mut tick = tokio::time::interval(Duration::from_secs(15 * 60));
    loop {
        tick.tick().await;
        if shutdown.load(Ordering::Relaxed) {
            return;
        }
        let (e, gl) = (engine.clone(), gl.clone());
        match blocking(move || feedback::forward_pending(e.store(), e.http(), &gl)).await {
            Ok(n) if n > 0 => tracing::info!(forwarded = n, "pending feedback forwarded"),
            Ok(_) => {}
            Err(err) => {
                tracing::warn!(error = %err.0, "pending feedback not forwarded; will retry")
            }
        }
    }
}

pub(crate) async fn create_feedback(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(mut r): Json<FeedbackRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let p = signed(&s, &ctx)?;
    // The build is the server's own, whatever the page says.
    if let Some(o) = r.context.as_object_mut() {
        o.insert("build".into(), s.opts.build.clone().into());
    }
    let e = s.engine.clone();
    let me = p.clone();
    let id = blocking(move || {
        e.store().feedback_create(
            &feedback::Author {
                org_id: &me.org_id,
                user_id: &me.user_id,
            },
            &r,
        )
    })
    .await?;
    tracing::info!(user_id = %p.user_id, org_id = %p.org_id, feedback_id = %id, "feedback received");
    forward_later(&s, id.clone());
    Ok((StatusCode::CREATED, json(&FeedbackCreated { id })))
}

pub(crate) async fn feedback_file(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path((id, file)): Path<(String, String)>,
) -> ApiResult<Response> {
    let p = signed(&s, &ctx)?;
    let e = s.engine.clone();
    let f = blocking(move || e.store().feedback_file(&p, &id, &file)).await?;
    let name: String = f
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_graphic() && c != '"' && c != '\\' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok((
        [
            (header::CONTENT_TYPE, f.mime),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{name}\""),
            ),
            (
                header::CONTENT_SECURITY_POLICY,
                "sandbox; default-src 'none'; frame-ancestors 'none'".to_string(),
            ),
        ],
        f.data,
    )
        .into_response())
}
