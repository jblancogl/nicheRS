//! Sessions, sign-in, invitations and password resets (hosted mode, ADR-0105).

use super::*;

pub(crate) fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == SESSION_COOKIE)
        .map(|(_, v)| v.to_string())
        .filter(|v| !v.is_empty())
}

pub(crate) async fn principal_of(
    s: &AppState,
    headers: &HeaderMap,
) -> ApiResult<Option<Principal>> {
    let Some(token) = session_token(headers) else {
        return Ok(None);
    };
    let e = s.engine.clone();
    blocking(move || e.store().session(&token)).await
}

pub(crate) fn unauthorized() -> Response {
    let body = serde_json::json!({"type": "about:blank", "title": "Sign-in required", "status": 401, "detail": "sign in to use NicheRS"});
    (
        StatusCode::UNAUTHORIZED,
        [(header::CONTENT_TYPE, "application/problem+json")],
        body.to_string(),
    )
        .into_response()
}

/// Local mode: everyone sees everything (single user). Hosted mode: a valid session is required
/// and scopes the request to the user's organisation.
pub(crate) async fn session_layer(
    State(s): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let ctx = if s.opts.accounts {
        match principal_of(&s, req.headers()).await {
            Ok(Some(p)) => Ctx {
                scope: p.scope(),
                access: p.access(),
                principal: Some(p),
            },
            Ok(None) => return unauthorized(),
            Err(e) => return e.into_response(),
        }
    } else {
        Ctx {
            scope: Scope::All,
            access: Scope::All,
            principal: None,
        }
    };
    req.extensions_mut().insert(ctx);
    next.run(req).await
}

pub(crate) fn session_cookie(token: &str) -> HeaderValue {
    let max_age = niche_app::accounts::SESSION_DAYS * 86_400;
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={max_age}"
    ))
    .unwrap_or(HeaderValue::from_static(""))
}

pub(crate) fn clear_cookie() -> HeaderValue {
    HeaderValue::from_static(
        "__Host-niche_session=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0",
    )
}

pub(crate) async fn me_of(s: &AppState, p: Principal) -> ApiResult<Me> {
    let e = s.engine.clone();
    let quota = blocking({
        let (org, total) = (p.org_id.clone(), p.monthly_quota);
        move || e.store().quota(&org, total)
    })
    .await?;
    Ok(Me {
        is_admin: p.is_admin(),
        is_manager: p.role == Role::Manager,
        name: p.name,
        email: p.email,
        organisation: p.org_name,
        quota,
        lang: p.lang,
    })
}

/// Response that opens a session: the account summary plus the session cookie.
pub(crate) async fn signed_in(s: &AppState, p: Principal, token: &str) -> ApiResult<Response> {
    let me = me_of(s, p).await?;
    let mut resp = Json(me).into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, session_cookie(token));
    resp.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(resp)
}

pub(crate) fn accounts_only(s: &AppState) -> ApiResult<()> {
    if s.opts.accounts {
        Ok(())
    } else {
        Err(ApiError(AppError::NotFound(
            "accounts are disabled (local mode)".into(),
        )))
    }
}

pub(crate) async fn auth_link(
    State(s): State<AppState>,
    Json(r): Json<LinkRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    accounts_only(&s)?;
    let e = s.engine.clone();
    let info = blocking(move || e.store().link_info(&r.token)).await?;
    Ok(Json(serde_json::to_value(info).unwrap_or_default()))
}

pub(crate) async fn auth_accept(
    State(s): State<AppState>,
    Json(r): Json<AcceptInviteRequest>,
) -> ApiResult<Response> {
    accounts_only(&s)?;
    let e = s.engine.clone();
    let (p, token) = blocking(move || {
        e.store()
            .accept_invite(&r.token, &r.name, &r.email, &r.password)
    })
    .await?;
    tracing::info!(user_id = %p.user_id, org = %p.org_name, "invitation accepted");
    signed_in(&s, p, &token).await
}

pub(crate) async fn auth_reset(
    State(s): State<AppState>,
    Json(r): Json<ResetPasswordRequest>,
) -> ApiResult<Response> {
    accounts_only(&s)?;
    let e = s.engine.clone();
    let (p, token) = blocking(move || e.store().reset_password(&r.token, &r.password)).await?;
    tracing::info!(user_id = %p.user_id, "password reset");
    signed_in(&s, p, &token).await
}

/// True when the e-mail is temporarily blocked after too many failures.
pub(crate) fn throttled(s: &AppState, email: &str) -> bool {
    let Ok(mut m) = s.failures.lock() else {
        return false;
    };
    m.retain(|_, (_, t)| t.elapsed() < FAILURE_WINDOW);
    m.get(email).is_some_and(|(n, _)| *n >= MAX_FAILURES)
}

pub(crate) fn record_failure(s: &AppState, email: &str, failed: bool) {
    if let Ok(mut m) = s.failures.lock() {
        if !failed {
            m.remove(email);
        } else if let Some(e) = m.get_mut(email) {
            e.0 += 1;
        } else if m.len() < MAX_TRACKED_FAILURES {
            m.insert(email.to_string(), (1, Instant::now()));
        }
    }
}

/// Throttling key of a sign-in: the e-mail as stored, or `None` when it cannot be an account
/// (such attempts fail without a hash and are not worth remembering).
pub(crate) fn failure_key(email: &str) -> Option<String> {
    let e = email.trim().to_lowercase();
    (!e.is_empty() && e.len() <= 254 && e.contains('@')).then_some(e)
}

pub(crate) async fn auth_login(
    State(s): State<AppState>,
    Json(r): Json<LoginRequest>,
) -> ApiResult<Response> {
    accounts_only(&s)?;
    let key = failure_key(&r.email);
    if key.as_deref().is_some_and(|k| throttled(&s, k)) {
        let body = serde_json::json!({"type": "about:blank", "title": "Too many attempts", "status": 429, "detail": "too many failed sign-in attempts for this account; try again in 15 minutes or ask Natural Solutions for a password reset link"});
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            [(header::CONTENT_TYPE, "application/problem+json")],
            body.to_string(),
        )
            .into_response());
    }
    let e = s.engine.clone();
    let res = blocking(move || e.store().login(&r.email, &r.password)).await?;
    if let Some(k) = &key {
        record_failure(&s, k, res.is_none());
    }
    match res {
        Some((p, token)) => signed_in(&s, p, &token).await,
        None => {
            tracing::warn!("failed sign-in");
            let body = serde_json::json!({"type": "about:blank", "title": "Sign-in failed", "status": 401, "detail": "e-mail or password incorrect"});
            Ok((
                StatusCode::UNAUTHORIZED,
                [(header::CONTENT_TYPE, "application/problem+json")],
                body.to_string(),
            )
                .into_response())
        }
    }
}

pub(crate) async fn auth_logout(
    State(s): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    accounts_only(&s)?;
    if let Some(token) = session_token(&headers) {
        let e = s.engine.clone();
        blocking(move || e.store().logout(&token)).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, clear_cookie())],
    )
        .into_response())
}

/// 404 in local mode (the UI then shows no account controls); 401 without a valid session.
pub(crate) async fn auth_me(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    accounts_only(&s)?;
    match principal_of(&s, &headers).await? {
        Some(p) => {
            let me = me_of(&s, p).await?;
            Ok(([(header::CACHE_CONTROL, "no-store")], Json(me)).into_response())
        }
        None => Ok(unauthorized()),
    }
}

/// The signed-in account chooses its interface language (401 without a session).
pub(crate) async fn auth_lang(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<LangRequest>,
) -> ApiResult<Response> {
    accounts_only(&s)?;
    let Some(p) = principal_of(&s, &headers).await? else {
        return Ok(unauthorized());
    };
    let e = s.engine.clone();
    blocking(move || e.store().user_set_lang(&p.user_id, &r.lang)).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
