//! The organisation's own occurrence datasets (ADR-0108). Every route works in the caller's
//! own organisation (`ctx.scope`): another organisation's dataset is "not found", and the
//! administrators' support access to studies does not extend to datasets. Records are never
//! served: the import answers with counts, studies show the points they used.

use super::*;
use niche_app::dto::{ImportRequest, NameMatchRequest};

pub(crate) async fn list_datasets(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let list = blocking(move || e.store().dataset_list(&ctx.scope)).await?;
    Ok(Json(serde_json::to_value(list).unwrap_or_default()))
}

pub(crate) async fn import_dataset(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(req): Json<ImportRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let e = s.engine.clone();
    let info = blocking(move || e.import_dataset(&req, &ctx.scope, ctx.principal.as_ref())).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(info).unwrap_or_default()),
    ))
}

pub(crate) async fn get_dataset(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let info = blocking(move || e.store().dataset_get(&id, &ctx.scope)).await?;
    Ok(Json(serde_json::to_value(info).unwrap_or_default()))
}

pub(crate) async fn delete_dataset(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let e = s.engine.clone();
    blocking(move || e.delete_dataset(&id, &ctx.scope)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn coordinate_systems() -> Json<serde_json::Value> {
    Json(serde_json::to_value(niche_app::owndata::crs_options()).unwrap_or_default())
}

/// Match the names of a dataset's taxon column to the GBIF backbone: only the names are sent.
pub(crate) async fn match_species(
    State(s): State<AppState>,
    Json(req): Json<NameMatchRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let m = blocking(move || e.match_names(&req.names)).await?;
    Ok(Json(serde_json::to_value(m).unwrap_or_default()))
}
