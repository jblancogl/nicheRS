//! Decision-support analyses of a finished study (ADR-0018): survey priority, project footprint,
//! multi-species aggregation, comparison with another study. Reading follows the study's access
//! rules (`authorize`); creating and deleting are for the owning organisation only, like renaming
//! a study (administrators' support access stays read-only), and a compared study must also be
//! the organisation's.

use niche_app::dto::AnalysisRequest;

use super::*;

pub(crate) async fn list_analyses(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let v = blocking(move || e.analyses(&id)).await?;
    Ok(Json(serde_json::to_value(v).unwrap_or_default()))
}

pub(crate) async fn create_analysis(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
    Json(req): Json<AnalysisRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    authorize_in(&s, &ctx, &id, &ctx.scope).await?;
    let e = s.engine.clone();
    // Reads the study's rasters and writes a few files: blocking pool, not the async runtime.
    let (a, created) = blocking(move || e.create_analysis(&id, &req, &ctx.scope)).await?;
    Ok((
        if created {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(serde_json::to_value(a).unwrap_or_default()),
    ))
}

pub(crate) async fn get_analysis(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path((id, aid)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let a = blocking(move || e.analysis(&id, &aid)).await?;
    Ok(Json(serde_json::to_value(a).unwrap_or_default()))
}

pub(crate) async fn delete_analysis(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path((id, aid)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    authorize_in(&s, &ctx, &id, &ctx.scope).await?;
    let e = s.engine.clone();
    blocking(move || e.delete_analysis(&id, &aid)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub(crate) struct AnalysisFileQuery {
    inline: Option<String>,
}

/// Downloads by default; `?inline=1` shows `report.html` sandboxed, like the species reports.
pub(crate) async fn analysis_file(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path((id, aid, name)): Path<(String, String, String)>,
    Query(q): Query<AnalysisFileQuery>,
) -> ApiResult<Response> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let (aid2, n2) = (aid.clone(), name.clone());
    let (path, media) = blocking(move || e.analysis_file(&id, &aid2, &n2)).await?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| ApiError(AppError::NotFound(name.clone())))?;
    let inline =
        matches!(q.inline.as_deref(), Some("1" | "true")) && media.starts_with("text/html");
    let disposition = format!(
        "{}; filename=\"{aid}_{name}\"",
        if inline { "inline" } else { "attachment" }
    );
    let mut resp = (
        [
            (header::CONTENT_TYPE, media),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        bytes,
    )
        .into_response();
    if inline {
        resp.headers_mut().insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "sandbox; default-src 'none'; img-src data:; style-src 'unsafe-inline'; frame-ancestors 'none'",
            ),
        );
        resp.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        );
    }
    Ok(resp)
}

pub(crate) async fn analysis_tile(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path((id, aid, layer, z, x, y)): Path<(String, String, String, u8, u32, String)>,
) -> ApiResult<Response> {
    authorize(&s, &ctx, &id).await?;
    let y: u32 = y
        .trim_end_matches(".png")
        .parse()
        .map_err(|_| ApiError(AppError::Invalid("bad tile y".into())))?;
    if z > 18 {
        return Err(ApiError(AppError::Invalid("zoom too large".into())));
    }
    let e = s.engine.clone();
    let png = blocking(move || {
        let (r, a) = e.analysis_raster(&id, &aid, &layer)?;
        let style = niche_app::analyses::tile_colour(&layer, &a)
            .ok_or_else(|| AppError::NotFound(format!("layer {layer}")))?;
        Ok(tiles::render(&r, z, x, y, move |v, _, _| style(v)))
    })
    .await?;
    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "private, max-age=86400"),
        ],
        png,
    )
        .into_response())
}
