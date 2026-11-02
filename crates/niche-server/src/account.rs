//! The signed-in account's properties and GBIF accounts (ADR-0110).

use super::*;

pub(crate) fn account_of(ctx: &Ctx, s: &AppState) -> ApiResult<Principal> {
    accounts_only(s)?;
    ctx.principal
        .clone()
        .ok_or_else(|| ApiError(AppError::Forbidden("sign in first".into())))
}

pub(crate) async fn account_update(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<niche_app::dto::AccountUpdate>,
) -> ApiResult<StatusCode> {
    let p = account_of(&ctx, &s)?;
    let e = s.engine.clone();
    blocking(move || e.store().user_set_name(&p.user_id, &r.name)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn account_password(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    headers: HeaderMap,
    Json(r): Json<niche_app::dto::PasswordChange>,
) -> ApiResult<StatusCode> {
    let p = account_of(&ctx, &s)?;
    let token = session_token(&headers).unwrap_or_default();
    let e = s.engine.clone();
    let user_id = p.user_id.clone();
    blocking(move || {
        e.store()
            .change_password(&user_id, &r.current, &r.new, &token)
    })
    .await?;
    tracing::info!(user_id = %p.user_id, "password changed; other sessions closed");
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn account_gbif(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Response> {
    let p = account_of(&ctx, &s)?;
    let e = s.engine.clone();
    let v = blocking(move || e.gbif_settings(&p)).await?;
    Ok(([(header::CACHE_CONTROL, "no-store")], json(&v)).into_response())
}

/// Store a GBIF account for `owner` (sealed), then answer with the user's settings.
pub(crate) async fn gbif_set(
    s: &AppState,
    p: Principal,
    owner: GbifOwner,
    r: niche_app::dto::GbifCredentialsRequest,
) -> ApiResult<Response> {
    let e = s.engine.clone();
    let tag = owner.tag();
    let v = blocking(move || {
        e.gbif_store(&owner, &r.username, &r.password)?;
        e.gbif_settings(&p)
    })
    .await?;
    tracing::info!(account = %tag, "GBIF account stored");
    Ok(([(header::CACHE_CONTROL, "no-store")], json(&v)).into_response())
}

pub(crate) async fn gbif_clear(s: &AppState, owner: GbifOwner) -> ApiResult<StatusCode> {
    let e = s.engine.clone();
    let tag = owner.tag();
    blocking(move || e.store().gbif_clear(&owner)).await?;
    tracing::info!(account = %tag, "GBIF account removed");
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn gbif_check(s: &AppState, owner: GbifOwner) -> ApiResult<Response> {
    let e = s.engine.clone();
    let v = blocking(move || e.gbif_check(&owner)).await?;
    Ok(([(header::CACHE_CONTROL, "no-store")], json(&v)).into_response())
}

pub(crate) async fn account_gbif_set(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<niche_app::dto::GbifCredentialsRequest>,
) -> ApiResult<Response> {
    let p = account_of(&ctx, &s)?;
    let owner = GbifOwner::User(p.user_id.clone());
    gbif_set(&s, p, owner, r).await
}

pub(crate) async fn account_gbif_clear(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<StatusCode> {
    let p = account_of(&ctx, &s)?;
    gbif_clear(&s, GbifOwner::User(p.user_id)).await
}

pub(crate) async fn account_gbif_check(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Response> {
    let p = account_of(&ctx, &s)?;
    gbif_check(&s, GbifOwner::User(p.user_id)).await
}
