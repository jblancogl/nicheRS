//! Species search, layers, previews, studies and their results, tiles and places; every
//! study route checks access first (`authorize`).

use super::*;

/// Every job-level route calls this first: a job of another organisation is "not found".
pub(crate) async fn authorize(s: &AppState, ctx: &Ctx, id: &str) -> ApiResult<()> {
    authorize_in(s, ctx, id, &ctx.access).await
}

/// Like [`authorize`], within an explicit scope. An administrator opening another organisation's
/// job is logged (ADR-0106: support access to client data is traceable).
pub(crate) async fn authorize_in(
    s: &AppState,
    ctx: &Ctx,
    id: &str,
    scope: &Scope,
) -> ApiResult<()> {
    let e = s.engine.clone();
    let (id2, scope) = (id.to_string(), scope.clone());
    blocking(move || e.job(&id2, &scope).map(|_| ())).await?;
    if let Some(p) = ctx.principal.as_ref().filter(|p| p.is_admin()) {
        let e = s.engine.clone();
        let id2 = id.to_string();
        let owner = blocking(move || e.store().job_org(&id2)).await?.flatten();
        if owner.as_deref() != Some(p.org_id.as_str()) && first_access_this_hour(s, &p.user_id, id)
        {
            let mut ev = AuditEvent::web(p, A::SupportAccess).job(id);
            ev.org_id = owner;
            audit(s, ev).await;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
pub(crate) struct Q {
    q: String,
}

pub(crate) async fn species_search(
    State(s): State<AppState>,
    Query(q): Query<Q>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let hits = blocking(move || e.search_species(&q.q)).await?;
    Ok(Json(serde_json::to_value(hits).unwrap_or_default()))
}

#[derive(Deserialize)]
pub(crate) struct DemoQ {
    #[serde(default)]
    demo: bool,
}

pub(crate) async fn layers(
    State(s): State<AppState>,
    Query(q): Query<DemoQ>,
) -> Json<serde_json::Value> {
    Json(serde_json::to_value(s.engine.layers(q.demo)).unwrap_or_default())
}

pub(crate) async fn check_area(
    State(s): State<AppState>,
    Json(req): Json<AreaCheckRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let c = blocking(move || e.check_area(&req.area)).await?;
    Ok(Json(serde_json::to_value(c).unwrap_or_default()))
}

pub(crate) async fn preview(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(req): Json<DataPreviewRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    // The organisation's own datasets only (ADR-0108).
    let p = blocking(move || e.preview_in(&req, &ctx.scope)).await?;
    Ok(Json(serde_json::to_value(p).unwrap_or_default()))
}

pub(crate) async fn list_jobs(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let jobs = blocking(move || e.store().list(&ctx.scope)).await?;
    Ok(Json(serde_json::to_value(jobs).unwrap_or_default()))
}

#[derive(serde::Deserialize)]
pub(crate) struct CreateJobQuery {
    project_id: Option<String>,
}

pub(crate) async fn create_job(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Query(q): Query<CreateJobQuery>,
    Json(spec): Json<JobSpec>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let e = s.engine.clone();
    let job = blocking(move || {
        e.submit_in(
            spec,
            ctx.principal.as_ref(),
            q.project_id.as_deref().filter(|p| !p.is_empty()),
        )
    })
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(job).unwrap_or_default()),
    ))
}

/// Rename a study or move it between projects. Like deletion, only its own organisation may
/// (administrators' support access is read-only here).
pub(crate) async fn update_job(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
    Json(u): Json<JobUpdate>,
) -> ApiResult<Json<serde_json::Value>> {
    authorize_in(&s, &ctx, &id, &ctx.scope).await?;
    let e = s.engine.clone();
    let job = blocking(move || {
        e.store().job_update(
            &id,
            &ctx.scope,
            u.name.as_deref(),
            u.project_id.as_ref().map(|p| p.as_deref()),
        )?;
        e.job(&id, &ctx.scope)
    })
    .await?;
    Ok(Json(serde_json::to_value(job).unwrap_or_default()))
}

pub(crate) async fn list_projects(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let projects = blocking(move || e.store().project_list(&ctx.scope)).await?;
    Ok(Json(serde_json::to_value(projects).unwrap_or_default()))
}

pub(crate) async fn create_project(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<ProjectRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let e = s.engine.clone();
    let p = blocking(move || {
        e.store()
            .project_create(&ctx.scope, ctx.principal.as_ref(), &r.name)
    })
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(p).unwrap_or_default()),
    ))
}

pub(crate) async fn rename_project(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
    Json(r): Json<ProjectRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let p = blocking(move || e.store().project_rename(&id, &ctx.scope, &r.name)).await?;
    Ok(Json(serde_json::to_value(p).unwrap_or_default()))
}

pub(crate) async fn delete_project(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let e = s.engine.clone();
    blocking(move || e.store().project_delete(&id, &ctx.scope)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn get_job(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let job = blocking(move || e.job(&id, &ctx.access)).await?;
    Ok(Json(serde_json::to_value(job).unwrap_or_default()))
}

pub(crate) async fn delete_job(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    // Deleting stays with the owning organisation, administrators included.
    authorize_in(&s, &ctx, &id, &ctx.scope).await?;
    let e = s.engine.clone();
    blocking(move || e.delete_job(&id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn cancel_job(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let job = blocking(move || e.cancel(&id)).await?;
    Ok(Json(serde_json::to_value(job).unwrap_or_default()))
}

/// The GBIF download a queued study is waiting for (`null` when it is not waiting).
pub(crate) async fn gbif_wait(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    authorize(&s, &ctx, &id).await?;
    Ok(Json(
        serde_json::to_value(s.engine.gbif_wait(&id)).unwrap_or_default(),
    ))
}

/// Continue a study whose GBIF download is unusually slow with the search API instead. Changes
/// how the study gets its data, so only its own organisation may (no support access).
pub(crate) async fn skip_download(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    authorize_in(&s, &ctx, &id, &ctx.scope).await?;
    let e = s.engine.clone();
    blocking(move || e.skip_download(&id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn results(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let r = blocking(move || e.results(&id)).await?;
    Ok(Json(serde_json::to_value(r).unwrap_or_default()))
}

pub(crate) async fn manifest(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let m = blocking(move || e.manifest(&id)).await?;
    Ok((
        [(
            header::CONTENT_DISPOSITION,
            "inline; filename=\"manifest.json\"",
        )],
        Json(m),
    )
        .into_response())
}

/// SSE: replays stored events after `Last-Event-ID`, then polls for new ones; ends when the job is
/// finished or the server shuts down (so graceful shutdown is not blocked).
pub(crate) async fn events(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    authorize(&s, &ctx, &id).await?;
    let last: i64 = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    {
        let e = s.engine.clone();
        let id2 = id.clone();
        blocking(move || {
            e.store()
                .get(&id2)?
                .ok_or_else(|| AppError::NotFound("job".into()))
        })
        .await?;
    }
    // Hosted mode: the stream ends as soon as the session is closed or the account or
    // organisation disabled, not only when the job finishes.
    let session = if s.opts.accounts {
        session_token(&headers)
    } else {
        None
    };
    let stream = futures_util::stream::unfold((last, false, s, id), move |(last, done, s, id)| {
        let session = session.clone();
        async move {
            if done {
                return None;
            }
            loop {
                if s.shutdown.load(Ordering::Relaxed) {
                    return None;
                }
                let e = s.engine.clone();
                let id2 = id.clone();
                let session = session.clone();
                let r = tokio::task::spawn_blocking(move || -> Result<_, AppError> {
                    if let Some(t) = &session
                        && e.store().session(t)?.is_none()
                    {
                        return Ok(None);
                    }
                    let evs = e.store().events_after(&id2, last)?;
                    let job = e.store().get(&id2)?;
                    Ok(Some((evs, job)))
                })
                .await;
                let Ok(Ok(Some((evs, job)))) = r else {
                    return None;
                };
                let terminal = job.as_ref().is_none_or(|j| j.status.is_terminal());
                if let Some(ev) = evs.first() {
                    let mut ev = ev.clone();
                    if let Some(j) = &job {
                        ev.progress = j.progress;
                        ev.stage = j.stage.clone();
                        ev.status = j.status;
                    }
                    let data = serde_json::to_string(&ev).unwrap_or_default();
                    let out = Event::default()
                        .event("progress")
                        .id(ev.seq.to_string())
                        .data(data);
                    let finished = terminal && evs.len() == 1;
                    return Some((
                        Ok::<_, std::convert::Infallible>(out),
                        (ev.seq, finished, s, id),
                    ));
                }
                if terminal {
                    return None;
                }
                // Heartbeat with current progress, so the UI bar moves between events.
                if let Some(j) = job {
                    let hb = serde_json::json!({"seq": last, "job_id": j.id, "status": j.status, "stage": j.stage, "progress": j.progress, "species_key": null, "message": "", "level": "info", "at": ""});
                    tokio::time::sleep(Duration::from_millis(700)).await;
                    return Some((
                        Ok(Event::default().event("status").data(hb.to_string())),
                        (last, false, s, id),
                    ));
                }
            }
        }
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

#[derive(Deserialize)]
pub(crate) struct TileQ {
    threshold: Option<f64>,
}

/// Small suitability map of a species, for the workspace cards (404 until it has results).
pub(crate) async fn thumbnail(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path((id, key)): Path<(String, u64)>,
) -> ApiResult<Response> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let png = blocking(move || e.thumbnail(&id, key)).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            // Local mode: results never change once written. Hosted mode: the guard sets no-store.
            (header::CACHE_CONTROL, "private, max-age=86400"),
        ],
        png,
    )
        .into_response())
}

#[derive(Deserialize)]
pub(crate) struct PlacesQ {
    q: String,
    country: Option<String>,
    limit: Option<usize>,
}

/// Administrative units by name, from the installed boundary packs (no external service).
pub(crate) async fn places_search(
    State(s): State<AppState>,
    Query(p): Query<PlacesQ>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let hits =
        blocking(move || Ok(e.search_places(&p.q, p.country.as_deref(), p.limit.unwrap_or(12))))
            .await?;
    Ok(Json(serde_json::to_value(hits).unwrap_or_default()))
}

#[derive(Deserialize)]
pub(crate) struct PointQ {
    lng: f64,
    lat: f64,
}

pub(crate) async fn places_at(
    State(s): State<AppState>,
    Query(p): Query<PointQ>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let hits = blocking(move || Ok(e.places_at(p.lng, p.lat))).await?;
    Ok(Json(serde_json::to_value(hits).unwrap_or_default()))
}

pub(crate) async fn place_geometry(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let e = s.engine.clone();
    let g = blocking(move || e.place_geometry(&id)).await?;
    Ok(Json(serde_json::to_value(g).unwrap_or_default()))
}

pub(crate) async fn tile(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path((id, key, layer, z, x, y)): Path<(String, u64, String, u8, u32, String)>,
    Query(q): Query<TileQ>,
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
        // Future-climate projections (ADR-0016): `future_<period>_<ssp>_<kind>`.
        if let Some((_, _, kind)) = niche_app::projection::parse_layer(&layer) {
            return Ok(match kind {
                "mean" => {
                    let r = e.raster(&id, key, &layer)?;
                    tiles::render(&r, z, x, y, |v, _, _| {
                        let c = colormap::viridis(v);
                        Some([c[0], c[1], c[2], 255])
                    })
                }
                "sd" => {
                    let r = e.raster(&id, key, &layer)?;
                    tiles::render(&r, z, x, y, |v, _, _| {
                        let c = colormap::greys(v / 0.25);
                        Some([c[0], c[1], c[2], 255])
                    })
                }
                // Change classes; pale where fewer than a majority of the GCMs agree.
                "change" => {
                    let r = e.projection_change_display(&id, key, &layer)?;
                    tiles::render(&r, z, x, y, |v, _, _| {
                        let strong = v >= 10.0;
                        let c =
                            niche_app::projection::class_colour(if strong { v - 10.0 } else { v })?;
                        Some([c[0], c[1], c[2], if strong { 235 } else { 110 }])
                    })
                }
                // Future extrapolation: hatched like the present MESS layer.
                "mess" => {
                    let r = e.raster(&id, key, &layer)?;
                    tiles::render(&r, z, x, y, |v, px, py| {
                        (v < 0.0 && (px + py) % 8 < 3).then_some([
                            colormap::HATCH[0],
                            colormap::HATCH[1],
                            colormap::HATCH[2],
                            230,
                        ])
                    })
                }
                other => return Err(AppError::NotFound(format!("layer kind {other}"))),
            });
        }
        let r = e.raster(&id, key, &layer)?;
        let threshold = q.threshold;
        Ok(match layer.as_str() {
            "suitability" => tiles::render(&r, z, x, y, |v, _, _| {
                let c = colormap::viridis(v);
                Some([c[0], c[1], c[2], 255])
            }),
            "uncertainty" => tiles::render(&r, z, x, y, |v, _, _| {
                let c = colormap::greys(v / 0.25);
                Some([c[0], c[1], c[2], 255])
            }),
            "extrapolation" => tiles::render(&r, z, x, y, |v, px, py| {
                (v < 0.0 && (px + py) % 8 < 3).then_some([
                    colormap::HATCH[0],
                    colormap::HATCH[1],
                    colormap::HATCH[2],
                    230,
                ])
            }),
            // Area of applicability (ADR-0014): hatched where the dissimilarity index exceeds the
            // species' threshold (outside the AOA), diagonals opposite to the MESS hatching so
            // both overlays can be shown together.
            "aoa" => {
                let t = e.aoa_threshold(&id, key)? as f32;
                tiles::render(&r, z, x, y, move |v, px, py| {
                    (v > t && (px % 8 + 8 - py % 8) % 8 < 3).then_some(AOA_HATCH)
                })
            }
            // Sampling effort of the target group (ADR-0013): records per cell on a log scale
            // up to the species' maximum; cells without records transparent.
            "effort" => {
                let lmax = (1.0 + e.effort_max(&id, key)? as f32)
                    .ln()
                    .max(f32::EPSILON);
                tiles::render(&r, z, x, y, move |v, _, _| {
                    (v > 0.0).then(|| {
                        let c = colormap::effort((1.0 + v).ln() / lmax);
                        [c[0], c[1], c[2], 230]
                    })
                })
            }
            "binary" => {
                let t = threshold
                    .ok_or_else(|| AppError::Invalid("binary layer needs ?threshold=".into()))?
                    as f32;
                tiles::render(&r, z, x, y, move |v, _, _| {
                    if v >= t {
                        Some([
                            colormap::SUITABLE[0],
                            colormap::SUITABLE[1],
                            colormap::SUITABLE[2],
                            255,
                        ])
                    } else {
                        Some([200, 200, 200, 90])
                    }
                })
            }
            other => return Err(AppError::NotFound(format!("layer {other}"))),
        })
    })
    .await?;
    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            // Local mode: cached a day. Hosted mode: the guard replaces this with no-store.
            (header::CACHE_CONTROL, "private, max-age=86400"),
        ],
        png,
    )
        .into_response())
}

/// Colour of the area-of-applicability hatching (purple, distinct from the MESS hatching).
const AOA_HATCH: [u8; 4] = [120, 60, 160, 230];

#[derive(Deserialize)]
pub(crate) struct SampleQ {
    lng: f64,
    lat: f64,
}

pub(crate) async fn sample(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path((id, key)): Path<(String, u64)>,
    Query(q): Query<SampleQ>,
) -> ApiResult<Json<serde_json::Value>> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let v = blocking(move || e.sample(&id, key, q.lng, q.lat)).await?;
    Ok(Json(serde_json::to_value(v).unwrap_or_default()))
}

pub(crate) async fn occurrences(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path((id, key)): Path<(String, u64)>,
) -> ApiResult<Json<serde_json::Value>> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let v = blocking(move || e.occurrences(&id, key)).await?;
    Ok(Json(serde_json::to_value(v).unwrap_or_default()))
}

#[derive(serde::Deserialize)]
pub(crate) struct FileQuery {
    /// `1` or `true`.
    inline: Option<String>,
}

/// Downloads by default. `?inline=1` displays an HTML report in the browser, sandboxed: an
/// opaque origin with no script, no network and only embedded images and styles.
pub(crate) async fn file(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path((id, key, name)): Path<(String, u64, String)>,
    Query(q): Query<FileQuery>,
) -> ApiResult<Response> {
    authorize(&s, &ctx, &id).await?;
    let e = s.engine.clone();
    let n2 = name.clone();
    let (path, media) = blocking(move || e.file_path(&id, key, &n2)).await?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| ApiError(AppError::NotFound(name.clone())))?;
    let inline =
        matches!(q.inline.as_deref(), Some("1" | "true")) && media.starts_with("text/html");
    let disposition = if inline {
        format!("inline; filename=\"{key}_{name}\"")
    } else {
        format!("attachment; filename=\"{key}_{name}\"")
    };
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
