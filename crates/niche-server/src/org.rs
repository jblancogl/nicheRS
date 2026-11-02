//! The user's organisation and its managers (ADR-0109, ADR-0110): members, invitations, GBIF
//! account and feedback.

use super::*;

/// The user's organisation (every member reads it; managers edit its GBIF account).
pub(crate) async fn org_profile(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Response> {
    let p = account_of(&ctx, &s)?;
    let e = s.engine.clone();
    let v = blocking(move || e.org_profile(&p)).await?;
    Ok(([(header::CACHE_CONTROL, "no-store")], json(&v)).into_response())
}

pub(crate) async fn org_gbif_set(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<niche_app::dto::GbifCredentialsRequest>,
) -> ApiResult<Response> {
    let p = manager(&s, &ctx)?;
    let owner = GbifOwner::Org(p.org_id.clone());
    let ev = AuditEvent::web(&p, A::OrgGbifSet).org(&p.org_id);
    let res = gbif_set(&s, p, owner, r).await?;
    audit(&s, ev).await;
    Ok(res)
}

pub(crate) async fn org_gbif_clear(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<StatusCode> {
    let p = manager(&s, &ctx)?;
    let res = gbif_clear(&s, GbifOwner::Org(p.org_id.clone())).await?;
    audit(&s, AuditEvent::web(&p, A::OrgGbifRemoved).org(&p.org_id)).await;
    Ok(res)
}

pub(crate) async fn org_gbif_check(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Response> {
    let p = manager(&s, &ctx)?;
    gbif_check(&s, GbifOwner::Org(p.org_id)).await
}

pub(crate) fn manager(s: &AppState, ctx: &Ctx) -> ApiResult<Principal> {
    accounts_only(s)?;
    match &ctx.principal {
        Some(p) if p.manages_org() => Ok(p.clone()),
        _ => Err(ApiError(AppError::Forbidden(
            "managing members is reserved to the organisation's managers".into(),
        ))),
    }
}

pub(crate) async fn org_members(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Json<serde_json::Value>> {
    let p = manager(&s, &ctx)?;
    let e = s.engine.clone();
    Ok(json(
        &blocking(move || e.store().user_list(Some(&p.org_name))).await?,
    ))
}

pub(crate) async fn org_invites(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Json<serde_json::Value>> {
    let p = manager(&s, &ctx)?;
    let e = s.engine.clone();
    Ok(json(
        &blocking(move || e.store().open_invites(&p.org_name)).await?,
    ))
}

pub(crate) async fn org_create_invite(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Response> {
    let p = manager(&s, &ctx)?;
    let e = s.engine.clone();
    let org = p.org_name.clone();
    let token = blocking(move || e.store().invite(&org)).await?;
    audit(&s, AuditEvent::web(&p, A::InviteCreated).org(&p.org_id)).await;
    Ok((
        StatusCode::CREATED,
        [(header::CACHE_CONTROL, "no-store")],
        json(&LinkCreated {
            path: format!("/invite#{token}"),
            days_valid: niche_app::accounts::INVITE_DAYS,
        }),
    )
        .into_response())
}

pub(crate) async fn org_revoke_invite(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let p = manager(&s, &ctx)?;
    let e = s.engine.clone();
    let org_id = p.org_id.clone();
    blocking(move || e.store().revoke_link_in(&org_id, &id)).await?;
    audit(&s, AuditEvent::web(&p, A::InviteRevoked).org(&p.org_id)).await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn org_reset_member(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<UserRequest>,
) -> ApiResult<Response> {
    let p = manager(&s, &ctx)?;
    let e = s.engine.clone();
    let (email, me) = (r.email.clone(), p.clone());
    let (token, account) = blocking(move || {
        e.store().check_member(&me, &email)?;
        let token = e.store().reset_link(&email)?;
        Ok((token, e.store().account_ref(&email)?))
    })
    .await?;
    audit(
        &s,
        AuditEvent::web(&p, A::ResetLinkCreated).account(&account),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        [(header::CACHE_CONTROL, "no-store")],
        json(&LinkCreated {
            path: format!("/reset#{token}"),
            days_valid: niche_app::accounts::RESET_DAYS,
        }),
    )
        .into_response())
}

pub(crate) async fn org_disable_member(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<UserDisableRequest>,
) -> ApiResult<StatusCode> {
    let p = manager(&s, &ctx)?;
    let e = s.engine.clone();
    let (email, disabled, me) = (r.email.clone(), r.disabled, p.clone());
    let account = blocking(move || {
        e.store().check_member(&me, &email)?;
        e.store().user_set_disabled(&email, disabled)?;
        e.store().account_ref(&email)
    })
    .await?;
    let action = if r.disabled {
        A::AccountDisabled
    } else {
        A::AccountEnabled
    };
    audit(&s, AuditEvent::web(&p, action).account(&account)).await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn org_feedback(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Json<serde_json::Value>> {
    let p = manager(&s, &ctx)?;
    let e = s.engine.clone();
    Ok(json(
        &blocking(move || e.store().feedback_list(Some(&p.org_id))).await?,
    ))
}

#[derive(Deserialize)]
pub(crate) struct OrgAuditQ {
    before: Option<i64>,
    limit: Option<usize>,
}

/// The organisation's history (ADR-0113): what its managers, Natural Solutions and the server
/// changed in it, and when an administrator opened one of its studies (support access).
pub(crate) async fn org_audit(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Query(q): Query<OrgAuditQ>,
) -> ApiResult<Json<serde_json::Value>> {
    let p = manager(&s, &ctx)?;
    let e = s.engine.clone();
    let f = AuditFilter {
        org: Some(p.org_id),
        user: None,
        before: q.before,
        limit: q.limit,
    };
    Ok(json(&blocking(move || e.store().audit_list(&f)).await?))
}
