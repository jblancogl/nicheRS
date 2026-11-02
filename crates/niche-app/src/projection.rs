//! Future-climate projections of a fitted species (ADR-0016, Science v1 phase 5).
//!
//! The final model(s) are applied to CHELSA v2.1 CMIP6 climatologies: every climate predictor
//! (CHELSA BIO1–BIO19; the demo's synthetic temperature and precipitation) is replaced by its
//! future value under one general circulation model (GCM), read on the same native window,
//! aggregated by the same factor; terrain, land-cover and water predictors stay at their present
//! values (ADR-0015). For each scenario (period × SSP) the GCMs are combined into a mean, an SD
//! across GCMs, change classes at the present P10 threshold with the number of GCMs agreeing,
//! areas from exact ellipsoidal cell areas and a future MESS against the training reference.
//!
//! Resume (ADR-0104): once a species is fitted and its present outputs are written, a
//! `projection-state.json` in its checkpoint directory holds what the projections need (final
//! model, training cells, result so far), and every GCM run is saved as it completes
//! (`proj-<period>-<ssp>-<gcm>.bin`). An interrupted species resumes from there without fitting
//! again. `result.json` is still written last, after the projection files and the reports, so it
//! remains the species' completion marker.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use niche_data::{Http, chelsa};
use niche_rf::Forest;
use serde::{Deserialize, Serialize};

use crate::AppError;
use crate::checkpoint::{self, Fingerprint};
use crate::dto::{
    GcmProjection, JobSpec, OccurrencePoint, ProjectionResult, ProjectionSpec, Severity,
    SpeciesResult, Warning,
};
use crate::env::{self, EnvStack};
use crate::fit::{ProjectedRun, Projector};

/// Most scenario × GCM runs per study. Each run reads every climate predictor of the model once
/// (network on first use: one range-read window per layer) and predicts every cell once, so a
/// study costs about `runs` times one present prediction plus the reads (measurements in
/// ADR-0016). 15 covers one period under the three SSPs, or the three periods under one SSP,
/// with all five GCMs.
pub const MAX_RUNS: usize = 15;

/// Revision of the projection computation; part of the state's fingerprint.
pub const REVISION: u32 = 1;

/// Change classes of `future_*_change.tif`.
pub const CLASS_UNSUITABLE: u8 = 1;
pub const CLASS_LOST: u8 = 2;
pub const CLASS_GAINED: u8 = 3;
pub const CLASS_STABLE: u8 = 4;

/// Projection outputs per scenario: suffix, description.
pub const KINDS: [&str; 5] = ["mean", "sd", "change", "agreement", "mess"];

/// A predictor that changes in the future scenarios: CHELSA climate, or the demo's synthetic
/// climate layers. Everything else stays at present values.
pub fn is_climate(id: &str) -> bool {
    chelsa::layer(id).is_some() || crate::demo::is_climate(id)
}

/// Validation of the requested projections (engine `validate`).
pub fn validate(spec: &JobSpec) -> Result<(), AppError> {
    if spec.projections.is_empty() {
        return Ok(());
    }
    let mut runs = 0;
    for (i, p) in spec.projections.iter().enumerate() {
        if !chelsa::FUTURE_PERIODS.contains(&p.period.as_str()) {
            return Err(AppError::Invalid(format!(
                "projection period must be one of {} (got {})",
                chelsa::FUTURE_PERIODS.join(", "),
                p.period
            )));
        }
        if !chelsa::FUTURE_SSPS.contains(&p.ssp.as_str()) {
            return Err(AppError::Invalid(format!(
                "projection scenario must be one of {} (got {})",
                chelsa::FUTURE_SSPS.join(", "),
                p.ssp
            )));
        }
        if p.gcms.is_empty() {
            return Err(AppError::Invalid(format!(
                "projection {} {}: choose at least one climate model (all five are recommended)",
                p.period, p.ssp
            )));
        }
        for (j, g) in p.gcms.iter().enumerate() {
            if !chelsa::FUTURE_GCMS.contains(&g.as_str()) {
                return Err(AppError::Invalid(format!(
                    "climate model must be one of {} (got {g})",
                    chelsa::FUTURE_GCMS.join(", ")
                )));
            }
            if p.gcms[..j].contains(g) {
                return Err(AppError::Invalid(format!(
                    "projection {} {}: climate model {g} is listed twice",
                    p.period, p.ssp
                )));
            }
        }
        if spec.projections[..i]
            .iter()
            .any(|q| q.period == p.period && q.ssp == p.ssp)
        {
            return Err(AppError::Invalid(format!(
                "projection {} {} is listed twice",
                p.period, p.ssp
            )));
        }
        runs += p.gcms.len();
    }
    if runs > MAX_RUNS {
        return Err(AppError::Invalid(format!(
            "at most {MAX_RUNS} scenario × climate-model combinations per study ({runs} requested): each one reads every climate layer of the model again and predicts the whole study area once"
        )));
    }
    if !spec.layers.iter().any(|l| is_climate(l)) {
        return Err(AppError::Invalid(
            "future projections replace climate predictors only; select at least one CHELSA climate variable".into(),
        ));
    }
    Ok(())
}

/// Number of scenario × GCM runs of a specification.
pub fn runs(spec: &JobSpec) -> usize {
    spec.projections.iter().map(|p| p.gcms.len()).sum()
}

/// Base name of a projection output, e.g. `future_2041-2070_ssp370_mean`.
pub fn file_stem(period: &str, ssp: &str, kind: &str) -> String {
    format!("future_{period}_{ssp}_{kind}")
}

/// Parse a projection layer name (`future_<period>_<ssp>_<kind>`) against the catalogue:
/// `(period, ssp, kind)`. Only catalogue values pass, so the name is safe as a file name.
pub fn parse_layer(layer: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let rest = layer.strip_prefix("future_")?;
    let (period, rest) = rest.split_once('_')?;
    let (ssp, kind) = rest.split_once('_')?;
    Some((
        chelsa::FUTURE_PERIODS.iter().find(|p| **p == period)?,
        chelsa::FUTURE_SSPS.iter().find(|s| **s == ssp)?,
        KINDS.iter().find(|k| **k == kind)?,
    ))
}

/// Future layers of one GCM for the model's predictors (`ids`, model order), on the present
/// grid: climate predictors read from CHELSA CMIP6 (synthetic for the demo) on the same native
/// window and aggregated by the same factor as the present stack; others copied from the
/// present stack. Returns the layers and the URLs read.
#[allow(clippy::too_many_arguments)]
pub fn future_layers(
    http: &Http,
    cache: &Path,
    spec: &JobSpec,
    present: &EnvStack,
    ids: &[String],
    period: &str,
    ssp: &str,
    gcm: &str,
) -> Result<(Vec<Vec<f32>>, Vec<String>), AppError> {
    let factor = spec.resolution_factor.clamp(1, 20);
    let static_layer = |id: &str| -> Result<Vec<f32>, AppError> {
        present
            .layers
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.data.clone())
            .ok_or_else(|| AppError::Internal(format!("projection: predictor {id} missing")))
    };
    let check = |id: &str, r: niche_geo::Raster| -> Result<Vec<f32>, AppError> {
        if !env::same_lattice(&r.spec, &present.spec) || r.data.len() != present.spec.len() {
            return Err(AppError::Internal(format!(
                "{id} ({period} {ssp} {gcm}): future grid {:?} differs from the present grid {:?}",
                r.spec, present.spec
            )));
        }
        Ok(r.data)
    };
    let mut out = Vec::with_capacity(ids.len());
    let mut urls = Vec::new();
    if spec.demo {
        let climate: Vec<String> = ids.iter().filter(|i| is_climate(i)).cloned().collect();
        let (_, mut rasters) = crate::demo::future_layers(&climate, period, ssp, gcm)?;
        for id in ids {
            if let Some(k) = rasters.iter().position(|(i, _)| i == id) {
                let (_, r) = rasters.swap_remove(k);
                out.push(check(id, r.aggregate_mean(factor))?);
            } else {
                out.push(static_layer(id)?);
            }
        }
        return Ok((out, urls));
    }
    let mut win = None;
    for id in ids {
        if chelsa::layer(id).is_none() {
            out.push(static_layer(id)?);
            continue;
        }
        let w = match &win {
            Some(w) => w,
            None => win.insert(env::area_window(
                http,
                cache,
                &spec.area,
                spec.resolution_factor,
            )?),
        };
        let r = chelsa::read_window_future(
            http,
            cache,
            id,
            period,
            gcm,
            ssp,
            (w.row, w.col, w.rows, w.cols),
        )?;
        if r.data.len() != w.rows * w.cols {
            return Err(AppError::Internal(format!(
                "{id} ({period} {ssp} {gcm}): window of {} values, expected {}",
                r.data.len(),
                w.rows * w.cols
            )));
        }
        out.push(check(id, r.aggregate_mean(w.factor))?);
        urls.push(chelsa::future_url(id, period, gcm, ssp).unwrap_or_default());
    }
    Ok((out, urls))
}

/// Rasters and summary of one scenario.
pub struct Aggregate {
    /// Full-grid rasters (NaN outside the projected cells).
    pub mean: Vec<f32>,
    pub sd: Vec<f32>,
    pub change: Vec<f32>,
    pub agreement: Vec<f32>,
    pub mess: Vec<f32>,
    pub summary: ProjectionResult,
}

/// Combine the GCM runs of one scenario. `cells` and `present` as in
/// [`crate::fit::PreparedProjection`]; `area(c)` the exact area of cell `c` (km²); `runs` in
/// `p.gcms` order. A cell is projected when every GCM gives it a value.
pub fn aggregate(
    grid_len: usize,
    cells: &[usize],
    area: &dyn Fn(usize) -> f64,
    present: &[f64],
    threshold: f64,
    p: &ProjectionSpec,
    runs: &[ProjectedRun],
) -> Aggregate {
    let g = runs.len();
    let nan = || vec![f32::NAN; grid_len];
    let (mut mean_r, mut sd_r, mut change_r, mut agree_r, mut mess_r) =
        (nan(), nan(), nan(), nan(), nan());
    let mut s = ProjectionResult {
        period: p.period.clone(),
        ssp: p.ssp.clone(),
        gcms: p.gcms.clone(),
        threshold,
        climate_variables: vec![],
        static_variables: vec![],
        cells: 0,
        cells_missing: 0,
        present_suitable_km2: 0.0,
        future_suitable_km2: 0.0,
        stable_km2: 0.0,
        gained_km2: 0.0,
        lost_km2: 0.0,
        unsuitable_km2: 0.0,
        stable_all_gcms_km2: 0.0,
        gained_all_gcms_km2: 0.0,
        lost_all_gcms_km2: 0.0,
        mean_gcm_sd: 0.0,
        novel_fraction: 0.0,
        per_gcm: p
            .gcms
            .iter()
            .map(|gcm| GcmProjection {
                gcm: gcm.clone(),
                future_suitable_km2: 0.0,
                gained_km2: 0.0,
                lost_km2: 0.0,
                novel_fraction: 0.0,
                mean_suitability: 0.0,
            })
            .collect(),
        synthetic: false,
    };
    let mut projected_km2 = 0.0;
    let mut novel_any_km2 = 0.0;
    let mut novel_km2 = vec![0.0; g];
    let mut sd_sum = 0.0;
    let mut vals = vec![0.0; g];
    for (j, &c) in cells.iter().enumerate() {
        if g == 0 || runs.iter().any(|r| !r.suitability[j].is_finite()) {
            s.cells_missing += 1;
            continue;
        }
        s.cells += 1;
        let a = area(c);
        projected_km2 += a;
        for (v, r) in vals.iter_mut().zip(runs) {
            *v = r.suitability[j];
        }
        let mean = vals.iter().sum::<f64>() / g as f64;
        let sd = niche_eval::sd(&vals);
        sd_sum += sd;
        let now = present[j] >= threshold;
        let future = mean >= threshold;
        let class = match (now, future) {
            (false, false) => CLASS_UNSUITABLE,
            (true, false) => CLASS_LOST,
            (false, true) => CLASS_GAINED,
            (true, true) => CLASS_STABLE,
        };
        let agree = vals.iter().filter(|&&v| (v >= threshold) == future).count();
        let min_mess = runs.iter().map(|r| r.mess[j]).fold(f32::INFINITY, f32::min);
        mean_r[c] = mean as f32;
        sd_r[c] = sd as f32;
        change_r[c] = f32::from(class);
        agree_r[c] = agree as f32;
        mess_r[c] = min_mess;
        if now {
            s.present_suitable_km2 += a;
        }
        if future {
            s.future_suitable_km2 += a;
        }
        let all = agree == g;
        match class {
            CLASS_UNSUITABLE => s.unsuitable_km2 += a,
            CLASS_LOST => {
                s.lost_km2 += a;
                if all {
                    s.lost_all_gcms_km2 += a;
                }
            }
            CLASS_GAINED => {
                s.gained_km2 += a;
                if all {
                    s.gained_all_gcms_km2 += a;
                }
            }
            _ => {
                s.stable_km2 += a;
                if all {
                    s.stable_all_gcms_km2 += a;
                }
            }
        }
        if min_mess < 0.0 {
            novel_any_km2 += a;
        }
        for (k, r) in runs.iter().enumerate() {
            let pg = &mut s.per_gcm[k];
            let v = r.suitability[j];
            pg.mean_suitability += v;
            if v >= threshold {
                pg.future_suitable_km2 += a;
                if !now {
                    pg.gained_km2 += a;
                }
            } else if now {
                pg.lost_km2 += a;
            }
            if r.mess[j] < 0.0 {
                novel_km2[k] += a;
            }
        }
    }
    let n = s.cells.max(1) as f64;
    s.mean_gcm_sd = sd_sum / n;
    let share = |x: f64| {
        if projected_km2 > 0.0 {
            x / projected_km2
        } else {
            0.0
        }
    };
    s.novel_fraction = share(novel_any_km2);
    for (pg, nk) in s.per_gcm.iter_mut().zip(&novel_km2) {
        pg.mean_suitability /= n;
        pg.novel_fraction = share(*nk);
    }
    Aggregate {
        mean: mean_r,
        sd: sd_r,
        change: change_r,
        agreement: agree_r,
        mess: mess_r,
        summary: s,
    }
}

// ---- state and per-GCM checkpoints ---------------------------------------------------------

/// A fitted species whose present outputs are written and whose projections remain to do.
pub struct Pending {
    pub result: SpeciesResult,
    /// Present output files written so far (name, sha256).
    pub sums: Vec<(String, String)>,
    /// Occurrence points (map of `report.html`).
    pub points: Vec<OccurrencePoint>,
    pub projector: Projector,
}

#[derive(Serialize, Deserialize)]
struct StateFile {
    revision: u32,
    context: String,
    result: SpeciesResult,
    sums: Vec<(String, String)>,
    points: Vec<OccurrencePoint>,
    algorithm: String,
    vars: Vec<String>,
    maxent: Option<checkpoint::Model>,
    forest: Option<Forest>,
    presence: Vec<usize>,
    background: Vec<usize>,
    p10_bits: u64,
}

/// File name of the projection state in a species' checkpoint directory.
pub const STATE: &str = "projection-state.json";

/// Fingerprint of everything a species' projections depend on: the specification (seed,
/// settings, scenarios), the species and the present grid.
pub fn context(spec_sha256: &str, species_key: u64, stack: &EnvStack) -> String {
    let mut fp = Fingerprint::default();
    fp.usizes(&[REVISION as usize]);
    fp.str(spec_sha256);
    fp.str(&species_key.to_string());
    let s = stack.spec;
    fp.f64s(&[s.west, s.north, s.cell]);
    fp.usizes(&[
        s.width,
        s.height,
        stack.valid.iter().filter(|v| **v).count(),
    ]);
    fp.finish()
}

/// Save the state of a fitted species (atomic). Any older per-GCM checkpoints are removed first:
/// they belong to another state.
pub fn save_state(dir: &Path, context: &str, p: &Pending) -> Result<(), AppError> {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with("proj-") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let file = StateFile {
        revision: REVISION,
        context: context.into(),
        result: p.result.clone(),
        sums: p.sums.clone(),
        points: p.points.clone(),
        algorithm: p.projector.algorithm.clone(),
        vars: p.projector.vars.clone(),
        maxent: p.projector.maxent.as_ref().map(checkpoint::Model::from),
        forest: p.projector.forest.clone(),
        presence: p.projector.presence.clone(),
        background: p.projector.background.clone(),
        p10_bits: p.projector.p10.to_bits(),
    };
    let mut v = serde_json::to_value(&file).map_err(|e| AppError::Internal(e.to_string()))?;
    floats_to_bits(&mut v);
    let bytes = serde_json::to_vec(&v).map_err(|e| AppError::Internal(e.to_string()))?;
    std::fs::create_dir_all(dir).map_err(|e| AppError::Storage(e.to_string()))?;
    crate::artifacts::write_atomic(&dir.join(STATE), &bytes)?;
    Ok(())
}

/// The saved state of a species, if it was made from the same inputs.
pub fn load_state(dir: &Path, context: &str) -> Option<Pending> {
    let text = std::fs::read(dir.join(STATE)).ok()?;
    let mut v: serde_json::Value = serde_json::from_slice(&text).ok()?;
    bits_to_floats(&mut v);
    let f: StateFile = serde_json::from_value(v).ok()?;
    (f.revision == REVISION && f.context == context).then(|| Pending {
        result: f.result,
        sums: f.sums,
        points: f.points,
        projector: Projector {
            algorithm: f.algorithm,
            vars: f.vars,
            maxent: f.maxent.map(Into::into),
            forest: f.forest,
            presence: f.presence,
            background: f.background,
            p10: f64::from_bits(f.p10_bits),
        },
    })
}

/// Marker of an f64 stored by its bits in the state file.
const BITS: &str = "\u{1}f64:";

/// Replace every floating-point number by a string of its bits. The state holds the result,
/// the forest and the training cells, and JSON text does not round-trip every f64 exactly
/// with this build's parser, so the state stores bits and a resumed species gives
/// byte-identical outputs.
fn floats_to_bits(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Number(n) if n.is_f64() => {
            if let Some(x) = n.as_f64() {
                *v = serde_json::Value::String(format!("{BITS}{:016x}", x.to_bits()));
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(floats_to_bits),
        serde_json::Value::Object(o) => o.values_mut().for_each(floats_to_bits),
        _ => {}
    }
}

fn bits_to_floats(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::String(s) if s.starts_with(BITS) => {
            if let Some(n) = u64::from_str_radix(&s[BITS.len()..], 16)
                .ok()
                .and_then(|b| serde_json::Number::from_f64(f64::from_bits(b)))
            {
                *v = serde_json::Value::Number(n);
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(bits_to_floats),
        serde_json::Value::Object(o) => o.values_mut().for_each(bits_to_floats),
        _ => {}
    }
}

fn run_path(dir: &Path, period: &str, ssp: &str, gcm: &str) -> PathBuf {
    dir.join(format!("proj-{period}-{ssp}-{gcm}.bin"))
}

/// Save one GCM run: cell count, suitability (f64 bits) then MESS (f32 bits), little endian.
/// A failure only costs the reuse.
pub fn save_run(dir: &Path, period: &str, ssp: &str, gcm: &str, r: &ProjectedRun) {
    let mut b = Vec::with_capacity(8 + r.suitability.len() * 12);
    b.extend_from_slice(&(r.suitability.len() as u64).to_le_bytes());
    for v in &r.suitability {
        b.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    for v in &r.mess {
        b.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    let path = run_path(dir, period, ssp, gcm);
    if let Err(e) = crate::artifacts::write_atomic(&path, &b) {
        tracing::warn!(file = %path.display(), error = %e, "projection checkpoint not saved");
    }
}

/// A saved GCM run for `cells` cells.
pub fn load_run(
    dir: &Path,
    period: &str,
    ssp: &str,
    gcm: &str,
    cells: usize,
) -> Option<ProjectedRun> {
    let b = std::fs::read(run_path(dir, period, ssp, gcm)).ok()?;
    if b.len() != 8 + cells * 12 || u64::from_le_bytes(b[..8].try_into().ok()?) != cells as u64 {
        return None;
    }
    let s = &b[8..8 + cells * 8];
    let m = &b[8 + cells * 8..];
    Some(ProjectedRun {
        suitability: s
            .chunks_exact(8)
            .map(|c| {
                f64::from_bits(u64::from_le_bytes([
                    c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7],
                ]))
            })
            .collect(),
        mess: m
            .chunks_exact(4)
            .map(|c| f32::from_bits(u32::from_le_bytes([c[0], c[1], c[2], c[3]])))
            .collect(),
    })
}

// ---- warnings, descriptions, reports, manifest ---------------------------------------------

/// Warnings of a species' projections (never hidden, ADR-0016).
pub fn warnings(r: &SpeciesResult) -> Vec<Warning> {
    let mut w = Vec::new();
    let Some(first) = r.projections.first() else {
        return w;
    };
    if !first.static_variables.is_empty() {
        w.push(
            Warning::new(
                "static_predictors_in_projection",
                Severity::Info,
                format!(
                    "In the future projections only climate changes: {} stay at their present values (no land-use or land-cover change is modelled).",
                    first.static_variables.join(", ")
                ),
            )
            .with("variables", first.static_variables.join(", ")),
        );
    }
    if first.climate_variables.is_empty() {
        w.push(Warning::new(
            "projection_without_climate",
            Severity::Warning,
            "No climate predictor was kept in the model (collinearity filter), so the future projections equal the present map.",
        ));
    }
    for p in &r.projections {
        if p.novel_fraction > 0.1 {
            w.push(
                Warning::new(
                    "projection_extrapolation",
                    Severity::Warning,
                    format!(
                        "{} {}: {:.0} % of the projected area has future conditions outside the training range under at least one climate model (MESS < 0); the projection there is an extrapolation.",
                        p.period,
                        scenario_label(&p.ssp),
                        p.novel_fraction * 100.0
                    ),
                )
                .with("period", p.period.clone())
                .with("ssp", p.ssp.clone())
                .with("percent", (p.novel_fraction * 100.0).round()),
            );
        }
        if p.cells_missing > 0 {
            w.push(
                Warning::new(
                    "projection_missing_cells",
                    Severity::Warning,
                    format!(
                        "{} {}: {} cells with present data have no future climate value under at least one model; they are left out of the projection maps and areas.",
                        p.period,
                        scenario_label(&p.ssp),
                        p.cells_missing
                    ),
                )
                .with("period", p.period.clone())
                .with("ssp", p.ssp.clone())
                .with("cells", p.cells_missing),
            );
        }
    }
    w
}

/// Plain name of an SSP.
pub fn scenario_label(ssp: &str) -> &'static str {
    match ssp {
        "ssp126" => "SSP1-2.6 (low emissions)",
        "ssp370" => "SSP3-7.0 (high emissions)",
        "ssp585" => "SSP5-8.5 (very high emissions)",
        _ => "unknown scenario",
    }
}

/// File descriptions of a scenario's outputs.
pub fn description(kind: &str, p: &ProjectionResult) -> String {
    let what = format!(
        "{} {}, {} climate model{}{}",
        p.period,
        scenario_label(&p.ssp),
        p.gcms.len(),
        if p.gcms.len() == 1 { "" } else { "s" },
        if p.synthetic {
            " — SYNTHETIC demonstration scenario, not CMIP6 data"
        } else {
            ""
        }
    );
    match kind {
        "mean" => format!(
            "Future relative habitat suitability ({what}): mean over the climate models of the final model's prediction with future climate (other predictors at present values); NOT a probability of presence"
        ),
        "sd" => format!(
            "Standard deviation of future relative suitability across the climate models ({what})"
        ),
        "change" => format!(
            "Change in suitable conditions at the present P10 threshold ({:.4}) ({what}): 1 unsuitable now and in the future, 2 suitable now but not in the future, 3 newly suitable, 4 suitable now and in the future. No dispersal is modelled: 3 means suitable conditions, not colonisation",
            p.threshold
        ),
        "agreement" => format!(
            "Number of climate models agreeing with the change class of `{}`: whose own future prediction is on the same side of the threshold as the mean ({what})",
            file_stem(&p.period, &p.ssp, "change")
        ),
        _ => format!(
            "Future MESS against the training data, lowest over the climate models ({what}); negative = conditions outside the training range under at least one model (extrapolation)"
        ),
    }
}

/// Method text of the projections (reports and manifest).
pub fn method_text(r: &SpeciesResult) -> String {
    let algorithm = match r.selected.algorithm() {
        "rf" => "the final random forest",
        "ensemble" => {
            "the final MaxEnt + random forest ensemble (percentile ranks taken against the present background predictions, so future values stay on the present scale)"
        }
        _ => "the final MaxEnt model (cloglog)",
    };
    format!(
        "The present model was projected with {algorithm} to future climatologies of CHELSA v2.1 (Karger et al. 2017), downscaled from CMIP6 models bias-adjusted for ISIMIP3b (Lange 2019): every climate predictor was replaced by its future value under each climate model (GCM), read on the same 30″ grid window and aggregated like the present data; terrain, land-cover and water predictors kept their present values. For each period and emissions scenario (SSP; O'Neill et al. 2016) the GCMs were averaged (mean suitability) and their spread mapped (SD across GCMs). Suitable conditions use the present 10th-percentile training-presence threshold for both periods; each cell is classed as suitable in both, newly suitable, no longer suitable or unsuitable in both from the GCM mean, with the number of GCMs agreeing. Areas are sums of exact ellipsoidal cell areas. Future environments outside the training range were flagged with MESS (Elith et al. 2010) against the same training reference as the present map. These describe changes in the area of suitable climatic conditions under each scenario, not range shifts: no dispersal, demography, biotic interaction or land-use change is modelled, and model transfer to new climates is uncertain (Yates et al. 2018); the spread across GCMs is part of the result (Thuiller et al. 2019)."
    )
}

/// Interpretation limits of projections.
pub const LIMITS: [&str; 5] = [
    "No dispersal is modelled: newly suitable areas are conditions the species could tolerate, not places it will reach; no demography, adaptation or biotic interaction either.",
    "Only climate changes in the projections: land cover, land use, terrain and water stay as today.",
    "The model is transferred to climates of another period: relationships fitted on 1981–2010 conditions may not hold, especially where future conditions are outside the training range (hatched; MESS < 0).",
    "Climate models disagree: the mean hides a spread that is mapped (SD across GCMs) and counted (GCMs agreeing with each change class); report the range across GCMs, not the mean alone.",
    "SSPs are scenarios, not forecasts: no likelihood is attached to any of them.",
];

pub const REF_ELITH_2010: &str = "Elith, J., Kearney, M. & Phillips, S. (2010) The art of modelling range-shifting species. Methods in Ecology and Evolution 1, 330–342. https://doi.org/10.1111/j.2041-210X.2010.00036.x";
pub const REF_THUILLER_2019: &str = "Thuiller, W., Guéguen, M., Renaud, J., Karger, D.N. & Zimmermann, N.E. (2019) Uncertainty in ensembles of global biodiversity scenarios. Nature Communications 10, 1446. https://doi.org/10.1038/s41467-019-09519-w";
pub const REF_YATES_2018: &str = "Yates, K.L. et al. (2018) Outstanding challenges in the transferability of ecological models. Trends in Ecology & Evolution 33, 790–802. https://doi.org/10.1016/j.tree.2018.08.001";
pub const REF_ONEILL_2016: &str = "O'Neill, B.C. et al. (2016) The Scenario Model Intercomparison Project (ScenarioMIP) for CMIP6. Geoscientific Model Development 9, 3461–3482. https://doi.org/10.5194/gmd-9-3461-2016";

/// References of the projection section (CHELSA CMIP6 attribution included).
pub fn references(synthetic: bool) -> Vec<String> {
    let mut v = vec![
        REF_ELITH_2010.to_string(),
        REF_THUILLER_2019.to_string(),
        REF_YATES_2018.to_string(),
        REF_ONEILL_2016.to_string(),
    ];
    if !synthetic {
        v.push(format!(
            "Future climate: {}. Licence: {}.",
            chelsa::FUTURE_CITATION,
            chelsa::FUTURE_LICENCE
        ));
    }
    v
}

fn pct(x: f64, of: f64) -> String {
    if of > 0.0 {
        format!("{:.0} %", 100.0 * x / of)
    } else {
        "–".into()
    }
}

/// Spread of a per-GCM value: "min–max".
fn range(p: &ProjectionResult, f: impl Fn(&GcmProjection) -> f64) -> String {
    let (lo, hi) = p
        .per_gcm
        .iter()
        .map(&f)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| {
            (a.min(x), b.max(x))
        });
    if p.per_gcm.is_empty() {
        "–".into()
    } else {
        format!("{lo:.0}–{hi:.0}")
    }
}

/// `## Future climate projections` of `report.md`.
pub fn report_md(r: &SpeciesResult) -> String {
    let mut s = String::new();
    let synthetic = r.projections.iter().any(|p| p.synthetic);
    let _ = writeln!(s, "\n## Future climate projections\n");
    if synthetic {
        let _ = writeln!(
            s,
            "> **SYNTHETIC future scenarios (demo): invented warming of the synthetic layers, not CMIP6 data.**\n"
        );
    }
    let _ = writeln!(s, "{}\n", method_text(r));
    if let Some(p) = r.projections.first() {
        let _ = writeln!(
            s,
            "Climate predictors projected: {}. {}\n",
            if p.climate_variables.is_empty() {
                "none (see warnings)".into()
            } else {
                p.climate_variables.join(", ")
            },
            if p.static_variables.is_empty() {
                String::new()
            } else {
                format!("Kept at present values: {}.", p.static_variables.join(", "))
            }
        );
    }
    let _ = writeln!(
        s,
        "Areas in km² (share of the present suitable area). \"All GCMs\": part of the class on which every climate model agrees. Range: lowest–highest over the individual climate models.\n\n| period | scenario | GCMs | present suitable | future suitable (range) | suitable in both (all GCMs) | newly suitable (all GCMs) | no longer suitable (all GCMs) | mean SD across GCMs | extrapolated (MESS < 0) |\n|---|---|---|---|---|---|---|---|---|---|"
    );
    for p in &r.projections {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {:.0} | {:.0} ({}) | {:.0} ({}; {:.0}) | {:.0} ({}; {:.0}) | {:.0} ({}; {:.0}) | {:.3} | {:.1} % |",
            p.period,
            scenario_label(&p.ssp),
            p.gcms.len(),
            p.present_suitable_km2,
            p.future_suitable_km2,
            range(p, |g| g.future_suitable_km2),
            p.stable_km2,
            pct(p.stable_km2, p.present_suitable_km2),
            p.stable_all_gcms_km2,
            p.gained_km2,
            pct(p.gained_km2, p.present_suitable_km2),
            p.gained_all_gcms_km2,
            p.lost_km2,
            pct(p.lost_km2, p.present_suitable_km2),
            p.lost_all_gcms_km2,
            p.mean_gcm_sd,
            p.novel_fraction * 100.0
        );
    }
    let _ = writeln!(
        s,
        "\nPer climate model (km²; extrapolated share of the projected area):\n\n| period | scenario | GCM | future suitable | newly suitable | no longer suitable | extrapolated |\n|---|---|---|---|---|---|---|"
    );
    for p in &r.projections {
        for g in &p.per_gcm {
            let _ = writeln!(
                s,
                "| {} | {} | {} | {:.0} | {:.0} | {:.0} | {:.1} % |",
                p.period,
                p.ssp,
                g.gcm,
                g.future_suitable_km2,
                g.gained_km2,
                g.lost_km2,
                g.novel_fraction * 100.0
            );
        }
    }
    let _ = writeln!(
        s,
        "\nFiles per scenario: `future_<period>_<ssp>_mean.tif`, `_sd.tif`, `_change.tif`, `_agreement.tif`, `_mess.tif`; all scenarios in `future_areas.csv`.\n\nLimits of the projections:\n"
    );
    for l in LIMITS {
        let _ = writeln!(s, "- {l}");
    }
    s
}

/// References block of `report.md`.
pub fn report_md_references(r: &SpeciesResult) -> String {
    let mut s = String::from("\n## References (future climate)\n\n");
    for x in references(r.projections.iter().any(|p| p.synthetic)) {
        let _ = writeln!(s, "- {x}");
    }
    s
}

/// `future_areas.csv`: one row per scenario (GCM mean) and per climate model.
pub fn areas_csv(r: &SpeciesResult) -> String {
    let mut s = String::from(
        "period,ssp,gcm,threshold,present_suitable_km2,future_suitable_km2,stable_km2,gained_km2,lost_km2,stable_all_gcms_km2,gained_all_gcms_km2,lost_all_gcms_km2,mean_gcm_sd,novel_fraction,cells,cells_missing\n",
    );
    for p in &r.projections {
        let _ = writeln!(
            s,
            "{},{},mean,{},{},{},{},{},{},{},{},{},{},{},{},{}",
            p.period,
            p.ssp,
            p.threshold,
            p.present_suitable_km2,
            p.future_suitable_km2,
            p.stable_km2,
            p.gained_km2,
            p.lost_km2,
            p.stable_all_gcms_km2,
            p.gained_all_gcms_km2,
            p.lost_all_gcms_km2,
            p.mean_gcm_sd,
            p.novel_fraction,
            p.cells,
            p.cells_missing
        );
        for g in &p.per_gcm {
            let _ = writeln!(
                s,
                "{},{},{},{},{},{},,{},{},,,,,{},{},{}",
                p.period,
                p.ssp,
                g.gcm,
                p.threshold,
                p.present_suitable_km2,
                g.future_suitable_km2,
                g.gained_km2,
                g.lost_km2,
                g.novel_fraction,
                p.cells,
                p.cells_missing
            );
        }
    }
    s
}

/// Colours of the change classes (Okabe–Ito, readable with colour-vision deficiencies):
/// unsuitable in both, lost, gained, suitable in both.
pub const CLASS_COLOURS: [[u8; 3]; 4] = [
    [0xE0, 0xE0, 0xE0],
    [0xD5, 0x5E, 0x00],
    [0x00, 0x72, 0xB2],
    [0x00, 0x9E, 0x73],
];

/// Colour of a change class value (`None` for NaN or an unknown value).
pub fn class_colour(v: f32) -> Option<[u8; 3]> {
    if !v.is_finite() || !(1.0..=4.0).contains(&v) {
        return None;
    }
    CLASS_COLOURS.get(v as usize - 1).copied()
}

/// Agreement needed to show a change class at full strength: a majority of the GCMs.
pub fn majority(gcms: usize) -> usize {
    gcms / 2 + 1
}

/// Manifest block of the projections of a study.
pub fn manifest_block(spec: &JobSpec, results: &[SpeciesResult]) -> serde_json::Value {
    let mut urls: Vec<String> = Vec::new();
    for r in results {
        for p in &r.projections {
            for g in &p.gcms {
                for id in &p.climate_variables {
                    if let Some(u) = chelsa::future_url(id, &p.period, g, &p.ssp) {
                        urls.push(u);
                    }
                }
            }
        }
    }
    urls.sort();
    urls.dedup();
    serde_json::json!({
        "scenarios": spec.projections,
        "source": if spec.demo { "SYNTHETIC future layers (demo): invented warming of the synthetic temperature and precipitation layers, not CMIP6 data" } else { chelsa::FUTURE_SOURCE },
        "licence": if spec.demo { "CC0 1.0" } else { chelsa::FUTURE_LICENCE },
        "citation": if spec.demo { "" } else { chelsa::FUTURE_CITATION },
        "files": urls,
        "static_predictors": "terrain, land-cover and water predictors kept at present values (ADR-0015, ADR-0016)",
        "threshold": "present P10 (10th percentile training presence) of each species, applied to the future GCM mean and to each GCM",
        "combination": "unweighted mean and population SD over the GCMs; change class from the mean; agreement = GCMs on the same side of the threshold as the mean",
        "extrapolation": "MESS (Elith et al. 2010) of each GCM's future predictors against the training reference (background + presences); map = lowest over GCMs; share = area with MESS < 0 under at least one GCM",
        "area": "sum of exact WGS84 ellipsoidal cell areas over cells with present and future values under every GCM",
        "ensemble_ranks": "ensemble projections rank each member against its present background predictions",
        "dispersal": "none modelled",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(gcms: &[&str]) -> ProjectionSpec {
        ProjectionSpec {
            period: "2041-2070".into(),
            ssp: "ssp370".into(),
            gcms: gcms.iter().map(|g| (*g).to_string()).collect(),
        }
    }

    /// Five cells of known areas, threshold 0.5, three GCMs: every class, agreement and area is
    /// checked by hand.
    #[test]
    fn change_classes_and_areas_on_a_toy_grid() {
        let cells = [0, 1, 2, 3, 5];
        let areas = [1.0, 2.0, 4.0, 8.0, 0.0, 16.0];
        let present = [0.2, 0.8, 0.6, 0.1, 0.7];
        let run = |s: [f64; 5], m: [f32; 5]| ProjectedRun {
            suitability: s.to_vec(),
            mess: m.to_vec(),
        };
        let runs = [
            run([0.1, 0.9, 0.4, 0.9, f64::NAN], [5.0, 1.0, -2.0, 3.0, 0.0]),
            run([0.3, 0.7, 0.3, 0.9, 0.9], [5.0, 1.0, 4.0, 3.0, 0.0]),
            run([0.2, 0.2, 0.2, 0.0, 0.9], [5.0, 1.0, 4.0, -1.0, 0.0]),
        ];
        let a = aggregate(
            6,
            &cells,
            &|c| areas[c],
            &present,
            0.5,
            &p(&["GFDL-ESM4", "IPSL-CM6A-LR", "MPI-ESM1-2-HR"]),
            &runs,
        );
        let s = &a.summary;
        // Cell 5 has no value under the first GCM: left out.
        assert_eq!((s.cells, s.cells_missing), (4, 1));
        assert!(a.change[5].is_nan() && a.change[4].is_nan());
        // Means: 0.2, 0.6, 0.3, 0.6.
        assert!((a.mean[1] - 0.6).abs() < 1e-6 && (a.mean[3] - 0.6).abs() < 1e-6);
        // Classes: unsuitable, stable, lost, gained.
        assert_eq!(
            [a.change[0], a.change[1], a.change[2], a.change[3]],
            [1.0, 4.0, 2.0, 3.0]
        );
        // Agreement: all 3 below; 2 of 3 above (0.2 below); all 3 below; 2 of 3 above.
        assert_eq!(
            [
                a.agreement[0],
                a.agreement[1],
                a.agreement[2],
                a.agreement[3]
            ],
            [3.0, 2.0, 3.0, 2.0]
        );
        assert_eq!(s.present_suitable_km2, 2.0 + 4.0);
        assert_eq!(s.future_suitable_km2, 2.0 + 8.0);
        assert_eq!(
            (s.unsuitable_km2, s.stable_km2, s.lost_km2, s.gained_km2),
            (1.0, 2.0, 4.0, 8.0)
        );
        assert_eq!(
            (
                s.stable_all_gcms_km2,
                s.lost_all_gcms_km2,
                s.gained_all_gcms_km2
            ),
            (0.0, 4.0, 0.0)
        );
        // MESS: lowest over GCMs; novel cells 2 (GCM 1) and 3 (GCM 3) = 12 of 15 km².
        assert_eq!(a.mess[2], -2.0);
        assert_eq!(a.mess[3], -1.0);
        assert!((s.novel_fraction - 12.0 / 15.0).abs() < 1e-12);
        // Per GCM.
        let g = &s.per_gcm;
        assert_eq!(
            (g[0].future_suitable_km2, g[0].gained_km2, g[0].lost_km2),
            (2.0 + 8.0, 8.0, 4.0)
        );
        assert_eq!(
            (g[2].future_suitable_km2, g[2].gained_km2, g[2].lost_km2),
            (0.0, 0.0, 6.0)
        );
        assert!((g[0].novel_fraction - 4.0 / 15.0).abs() < 1e-12);
        assert!((g[1].mean_suitability - (0.3 + 0.7 + 0.3 + 0.9) / 4.0).abs() < 1e-12);
        // SD across GCMs (population): cell 0 values 0.1, 0.3, 0.2.
        let sd0 = ((0.01f64 + 0.01 + 0.0) / 3.0).sqrt();
        assert!((f64::from(a.sd[0]) - sd0).abs() < 1e-6);
        assert_eq!(majority(5), 3);
        assert_eq!(majority(2), 2);
    }

    #[test]
    fn layer_names_are_parsed_against_the_catalogue() {
        assert_eq!(
            parse_layer("future_2041-2070_ssp370_mean"),
            Some(("2041-2070", "ssp370", "mean"))
        );
        assert_eq!(
            parse_layer(&file_stem("2071-2100", "ssp585", "change")),
            Some(("2071-2100", "ssp585", "change"))
        );
        for bad in [
            "future_2041-2070_ssp245_mean",
            "future_../x_ssp370_mean",
            "future_2041-2070_ssp370_mean.tif",
            "suitability",
        ] {
            assert!(parse_layer(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn runs_round_trip_through_their_checkpoint() {
        let dir = std::env::temp_dir().join(format!("niche-proj-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let r = ProjectedRun {
            suitability: vec![0.1, f64::NAN, 1.0 / 3.0],
            mess: vec![-0.0, f32::NAN, 12.5],
        };
        save_run(&dir, "2011-2040", "ssp126", "MRI-ESM2-0", &r);
        let back = load_run(&dir, "2011-2040", "ssp126", "MRI-ESM2-0", 3).expect("saved run");
        assert_eq!(back.suitability[2].to_bits(), r.suitability[2].to_bits());
        assert!(back.suitability[1].is_nan() && back.mess[1].is_nan());
        assert_eq!(back.mess[0].to_bits(), (-0.0f32).to_bits());
        assert!(load_run(&dir, "2011-2040", "ssp126", "MRI-ESM2-0", 4).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
