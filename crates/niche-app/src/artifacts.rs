//! Per-species output files: atomic writes, checksums, methods report.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use niche_geo::write::write_geotiff_f32;
use sha2::{Digest, Sha256};

use crate::AppError;
use crate::dto::{FileInfo, JobSpec, OccurrencePoint, SpeciesResult};
use crate::env::{self, EnvStack, raster};

pub fn species_dir(data_dir: &Path, job: &str, key: u64) -> PathBuf {
    data_dir.join("jobs").join(job).join(key.to_string())
}

/// Write via temp file in the same directory, fsync, rename (job-lifecycle.md).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<String, AppError> {
    use std::io::Write;
    let dir = path
        .parent()
        .ok_or_else(|| AppError::Internal("no parent dir".into()))?;
    std::fs::create_dir_all(dir).map_err(io)?;
    let tmp = path.with_extension("partial");
    {
        let mut f = std::fs::File::create(&tmp).map_err(io)?;
        f.write_all(bytes).map_err(io)?;
        f.sync_all().map_err(io)?;
    }
    std::fs::rename(&tmp, path).map_err(io)?;
    if let Ok(d) = std::fs::File::open(dir) {
        let _ = d.sync_all();
    }
    Ok(sha256(bytes))
}

pub fn sha256(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn io(e: std::io::Error) -> AppError {
    AppError::Storage(e.to_string())
}

pub struct Outputs<'a> {
    pub stack: &'a EnvStack,
    pub suitability: Vec<f32>,
    pub uncertainty: Vec<f32>,
    pub mess: Vec<f32>,
    /// Dissimilarity index of the area of applicability (only when requested, ADR-0014).
    pub aoa: Option<Vec<f32>>,
    /// Records of the target group per cell (only with a target-group effort surface, ADR-0013).
    pub effort: Option<Vec<f32>>,
    pub lambdas: Option<String>,
    /// Final random forest (`rf`, `ensemble`), ADR-0017.
    pub forest: Option<&'a niche_rf::Forest>,
    pub points: &'a [OccurrencePoint],
    /// Full marginal response curves of the final model (empty when no model was fitted).
    pub curves: &'a [niche_maxent::ResponseCurve],
}

/// `variable,x,y` rows of every curve (MaxEnt's `plots/*.dat` layout, all variables in one file).
pub fn curves_csv(curves: &[niche_maxent::ResponseCurve]) -> String {
    let mut s = String::from("variable,x,y\n");
    for c in curves {
        for (x, y) in c.x.iter().zip(&c.y) {
            let _ = writeln!(s, "{},{x},{y}", c.variable);
        }
    }
    s
}

/// Files written for a species: list for `result.json` and name → sha256 for the manifest.
pub struct FileSink<'a> {
    dir: &'a Path,
    pub files: Vec<FileInfo>,
    pub sums: Vec<(String, String)>,
}

impl<'a> FileSink<'a> {
    pub fn new(dir: &'a Path) -> Self {
        Self {
            dir,
            files: Vec::new(),
            sums: Vec::new(),
        }
    }

    /// Write one file atomically and record it.
    pub fn put(
        &mut self,
        name: &str,
        desc: &str,
        media: &str,
        bytes: &[u8],
    ) -> Result<(), AppError> {
        let h = write_atomic(&self.dir.join(name), bytes)?;
        self.files.push(FileInfo {
            name: name.into(),
            description: desc.into(),
            media_type: media.into(),
        });
        self.sums.push((name.to_string(), h));
        Ok(())
    }

    pub fn finish(self) -> (Vec<FileInfo>, Vec<(String, String)>) {
        (self.files, self.sums)
    }
}

pub const TIFF: &str = "image/tiff; application=geotiff";

/// Write all files; returns (file list, name → sha256).
pub fn write_species(
    dir: &Path,
    out: Outputs<'_>,
    result: &SpeciesResult,
    spec: &JobSpec,
) -> Result<(Vec<FileInfo>, Vec<(String, String)>), AppError> {
    // Before the rasters are moved into their GeoTIFFs.
    let html = crate::report_html::report_html(
        result,
        spec,
        Some(&crate::report_html::Maps {
            stack: out.stack,
            suitability: &out.suitability,
            uncertainty: &out.uncertainty,
            mess: &out.mess,
            aoa: out.aoa.as_deref(),
            effort: out.effort.as_deref(),
            points: out.points,
            future: &[],
        }),
    );
    let mut sink = FileSink::new(dir);
    write_species_files(&mut sink, out, result, spec)?;
    write_reports(&mut sink, result, spec, &html)?;
    Ok(sink.finish())
}

/// `report.md` and `report.html` (the latter already rendered).
pub fn write_reports(
    sink: &mut FileSink<'_>,
    result: &SpeciesResult,
    spec: &JobSpec,
    html: &str,
) -> Result<(), AppError> {
    sink.put(
        "report.md",
        "Methods and results summary (ODMAP-style)",
        "text/markdown",
        methods_report(result, spec).as_bytes(),
    )?;
    sink.put(
        "report.html",
        "Printable report with maps, response curves and tables (open in a browser; print to PDF)",
        "text/html",
        html.as_bytes(),
    )
}

/// Every output of a species except the reports: rasters, models, curves, occurrences.
pub fn write_species_files(
    sink: &mut FileSink<'_>,
    out: Outputs<'_>,
    result: &SpeciesResult,
    spec: &JobSpec,
) -> Result<(), AppError> {
    let tif = TIFF;
    let mut put =
        |name: &str, desc: &str, media: &str, bytes: &[u8]| sink.put(name, desc, media, bytes);
    if result.status == crate::dto::SpeciesRunStatus::Succeeded {
        let w = |d: Vec<f32>| {
            write_geotiff_f32(&raster(out.stack, d)).map_err(|e| AppError::Internal(e.to_string()))
        };
        put(
            "suitability.tif",
            suitability_description(result.selected.algorithm()),
            tif,
            &w(out.suitability)?,
        )?;
        put(
            "uncertainty.tif",
            "Standard deviation of suitability across cross-validation models",
            tif,
            &w(out.uncertainty)?,
        )?;
        put(
            "mess.tif",
            "Multivariate environmental similarity (MESS); negative = extrapolation",
            tif,
            &w(out.mess)?,
        )?;
        if let Some(a) = out.aoa {
            put(
                "aoa.tif",
                "Area of applicability: dissimilarity index (DI) to the training data; cells with DI above the threshold in the report are outside the area of applicability",
                tif,
                &w(a)?,
            )?;
        }
        if let Some(e) = out.effort {
            put(
                "effort.tif",
                "Sampling effort: GBIF records of the target group per cell (regional mirror; synthetic in the demo), NaN outside the study area. Records, not visits or observers",
                tif,
                &w(e)?,
            )?;
        }
        if let Some(l) = &out.lambdas {
            put(
                "model.lambdas",
                if result.selected.algorithm() == "ensemble" {
                    "Final MaxEnt member of the ensemble (MaxEnt 3.4.4 .lambdas format)"
                } else {
                    "Final MaxEnt model (MaxEnt 3.4.4 .lambdas format)"
                },
                "text/plain",
                l.as_bytes(),
            )?;
        }
        if let Some(f) = out.forest {
            let bytes = serde_json::to_vec(f).map_err(|e| AppError::Internal(e.to_string()))?;
            put(
                "model.rf.json",
                "Final down-sampled random forest (niche-rf format version 1, JSON: settings with seed, training ranges, trees as flat node arrays; parse numbers with exact float round-tripping to reproduce predictions)",
                "application/json",
                &bytes,
            )?;
        }
        if !out.curves.is_empty() {
            put(
                "response_curves.csv",
                "Marginal response curves of the final model (cloglog relative suitability; 501 points per variable, other variables at their presence mean)",
                "text/csv",
                curves_csv(out.curves).as_bytes(),
            )?;
        }
        if !result.partial_dependence.is_empty() {
            let mut csv = String::from("variable,x,y\n");
            for c in &result.partial_dependence {
                for (x, y) in c.x.iter().zip(&c.y) {
                    let _ = writeln!(csv, "{},{x},{y}", c.variable);
                }
            }
            put(
                "partial_dependence.csv",
                "Partial dependence of the final random forest (mean fraction of trees voting presence as one variable varies, other variables at their observed values; Friedman 2001)",
                "text/csv",
                csv.as_bytes(),
            )?;
        }
    }
    let occ_desc = if spec.demo {
        OCCURRENCES_DESCRIPTION.to_string()
    } else {
        format!("{OCCURRENCES_DESCRIPTION}. {SINP_NOTE}")
    };
    put(
        "occurrences.geojson",
        &occ_desc,
        "application/geo+json",
        occurrences_geojson(out.points).as_bytes(),
    )?;
    Ok(())
}

/// File description of `suitability.tif` per algorithm (ADR-0007, ADR-0017).
pub fn suitability_description(algorithm: &str) -> &'static str {
    match algorithm {
        "rf" => {
            "Relative habitat suitability index (fraction of the random forest's trees voting presence, 0–1), final model; NOT a probability of presence"
        }
        "ensemble" => {
            "Relative habitat suitability index of the MaxEnt + random forest ensemble (mean percentile rank of the two models among the background points, 0–1), final model; NOT a probability of presence"
        }
        _ => {
            "Relative habitat suitability (cloglog 0–1), final model; NOT a probability of presence"
        }
    }
}

const OCCURRENCES_DESCRIPTION: &str = "Occurrence records: every retained presence, plus excluded records with their exclusion reason (capped; exact counts in the report)";

/// Reminder on sensitive species (the design notes §A5). Real GBIF data only:
/// the synthetic demo has no sensitive species.
pub const SINP_NOTE: &str = "Coordinates as published by GBIF. Some species are sensitive under French SINP rules: check the regional référentiel before reproducing points in a public report.";

/// Background sentence of the reports (ADR-0009 §3, ADR-0013).
pub fn background_text(r: &SpeciesResult) -> String {
    match &r.background_sampling {
        Some(b) if b.method == "target_group" => format!(
            "{} cells drawn at random among the {} land cells of the study area with at least one GBIF record of the target group {}{} (target-group background, Phillips et al. 2009), so that presences and background share the observers' sampling bias.",
            r.data.background,
            b.eligible_cells,
            b.target_group.as_deref().unwrap_or("–"),
            if b.synthetic {
                " (synthetic effort, demo)"
            } else {
                ""
            }
        ),
        Some(b) => format!(
            "{} random cells within the study area (land only). A target-group background was requested but not available ({}); the random background was used.",
            r.data.background,
            b.fallback_reason.as_deref().unwrap_or("unknown reason")
        ),
        None => format!(
            "{} random cells within the study area (land only).",
            r.data.background
        ),
    }
}

/// Sampling-bias sentence of "Interpretation limits": accurate for each run.
pub fn bias_limit_text(r: &SpeciesResult) -> String {
    match &r.background_sampling {
        Some(b) if b.method == "target_group" => format!(
            "Occurrence data from opportunistic sources carry sampling bias (observer effort, accessibility); it was corrected with a target-group background ({}), which removes only the part of the bias the target group shares with the species.",
            b.target_group.as_deref().unwrap_or("–")
        ),
        _ => "Occurrence data from opportunistic sources carry sampling bias (observer effort, accessibility); no bias correction was applied.".to_string(),
    }
}

/// Generalised-records sentence (ADR-0011), real data only.
pub fn generalised_text(r: &SpeciesResult, max_m: f64) -> Option<String> {
    let g = r.data.generalised.as_ref()?;
    Some(if g.flags_available {
        format!(
            "Coordinates generalised by the publisher (sensitive species or observer request; Darwin Core dataGeneralizations / informationWithheld): {} records, {} excluded because their precision is not stated or > {max_m} m, {} retained as presences (ADR-0011; Chapman 2020).",
            g.records, g.excluded, g.retained
        )
    } else {
        "This GBIF download format does not report coordinate generalisation: generalised records with small or missing uncertainty could not be detected (ADR-0011).".to_string()
    })
}

/// Human-readable methods text that can be pasted into a consultancy report.
pub fn methods_report(r: &SpeciesResult, spec: &JobSpec) -> String {
    let mut s = String::new();
    let synthetic = if spec.demo {
        "\n> **SYNTHETIC DEMONSTRATION DATA — not a real species or real environment.**\n"
    } else {
        ""
    };
    let _ = writeln!(
        s,
        "# Habitat suitability model — {}\n{synthetic}",
        r.scientific_name
    );
    let _ = writeln!(
        s,
        "Generated by NicheRS {} on {}.\n",
        env!("CARGO_PKG_VERSION"),
        jiff::Timestamp::now()
    );
    let _ = writeln!(s, "## Evidence level: **{:?}**\n", r.evidence.level);
    for reason in &r.evidence.reasons {
        let _ = writeln!(s, "- {reason}");
    }
    let _ = writeln!(s, "\n## Data\n");
    let _ = writeln!(
        s,
        "- Study area: {} ({:.0} km² of land with predictor data).",
        spec.area.name, r.study_area_km2
    );
    if !spec.area.places.is_empty() {
        let mut credits: Vec<&str> = spec
            .area
            .places
            .iter()
            .map(|p| p.attribution.as_str())
            .collect();
        credits.sort_unstable();
        credits.dedup();
        let _ = writeln!(
            s,
            "  - built from administrative units {} (boundaries: {}).",
            spec.area
                .places
                .iter()
                .map(|p| format!("{} [{}]", p.name, p.id))
                .collect::<Vec<_>>()
                .join(", "),
            credits.join("; ")
        );
    }
    let _ = writeln!(
        s,
        "- Occurrences: {} source; {} records retrieved; {} grid cells retained after cleaning.",
        r.data.source, r.data.records_fetched, r.data.presences
    );
    if let Some(q) = &r.data.query_url {
        let _ = writeln!(s, "- Query: <{q}>");
    }
    let _ = writeln!(
        s,
        "- Filters: records from {} onwards, coordinate uncertainty ≤ {} m, licences {:?}, one record per grid cell.",
        spec.filters.year_min, spec.filters.max_uncertainty_m, spec.filters.licences
    );
    if let Some(km) = spec.filters.thin_km {
        let _ = writeln!(
            s,
            "- Distance thinning: presence cells at least {km} km apart (spThin, Aiello-Lammens et al. 2015), seeded by the study seed."
        );
    }
    for e in &r.data.exclusions {
        let _ = writeln!(s, "  - excluded — {}: {}", e.reason, e.count);
    }
    if let Some(g) = generalised_text(r, spec.filters.max_uncertainty_m) {
        let _ = writeln!(s, "- {g}");
    }
    if !spec.demo {
        let _ = writeln!(s, "- {SINP_NOTE}");
    }
    let _ = writeln!(s, "- Background: {}", background_text(r));
    if let Some(b) = &r.sampling_bias {
        let _ = writeln!(
            s,
            "- Sampling-bias diagnostic: {} records of the target group {}{} in {} cells ({:.1} % of the study area; layer `effort.tif`). {:.0} % of the presence cells lie in the {:.0} % of the study area with most records ({:.0} % expected if presences ignored observer effort). Descriptive only; no effect on the evidence level.",
            b.records,
            b.target_group,
            if b.synthetic { " (synthetic)" } else { "" },
            b.cells_with_effort,
            b.share_of_area_with_effort * 100.0,
            b.presences_in_top10pct_effort_cells * 100.0,
            b.expected * 100.0,
            b.expected * 100.0
        );
    }
    let _ = writeln!(s, "- Predictors used: {}.", r.variables_used.join("; "));
    for d in &r.variables_dropped {
        let _ = writeln!(s, "  - dropped {}: {}", d.variable, d.reason);
    }
    s.push_str(&crate::independent::sources_md(r, spec));
    if env::uses_additional_predictors(&spec.layers) {
        let _ = writeln!(s, "\n## Data sources\n");
        for c in env::sources_used(&spec.layers) {
            let _ = writeln!(
                s,
                "- **{}** ({}) — layers {}. Licence: {} <{}>. Citation: {}.",
                c.source,
                c.vintage,
                c.layers.join(", "),
                c.licence,
                c.licence_url,
                c.citation.trim_end_matches('.')
            );
            if let Some(a) = &c.attribution {
                let _ = writeln!(s, "  - Attribution: {a}");
            }
            for m in &c.methods {
                let _ = writeln!(s, "  - {m}");
            }
        }
        let _ = writeln!(
            s,
            "- Land mask: {}.\n\n{}",
            niche_data::landmask::SOURCE,
            env::predictor_methods(spec.resolution_factor)
        );
    }
    let algorithm = r.selected.algorithm();
    if !r.contributions.is_empty() && algorithm == "maxent" {
        let _ = writeln!(
            s,
            "\n## Predictor contributions\n\nPercent contribution (training gain) and permutation importance (drop in training AUC when the variable is permuted, normalised to 100), as defined by MaxEnt 3.4.4. Correlated variables share credit. Response curves: `response_curves.csv` and `report.html`.\n\n| variable | percent contribution | permutation importance |\n|---|---|---|"
        );
        for c in &r.contributions {
            let _ = writeln!(
                s,
                "| {} | {:.1} | {} |",
                c.variable,
                c.percent,
                c.permutation_importance
                    .map_or("–".into(), |p| format!("{p:.1}"))
            );
        }
    } else if !r.contributions.is_empty() {
        let _ = writeln!(
            s,
            "\n## Predictor importance\n\n{}Correlated variables share credit. {}\n",
            if algorithm == "ensemble" {
                "MaxEnt member: percent contribution (training gain) and permutation importance as defined by MaxEnt 3.4.4. "
            } else {
                ""
            },
            RF_IMPORTANCE_TEXT
        );
        if algorithm == "ensemble" {
            let _ = writeln!(
                s,
                "| variable | MaxEnt percent contribution | MaxEnt permutation importance | RF permutation importance |\n|---|---|---|---|"
            );
            for c in &r.contributions {
                let _ = writeln!(
                    s,
                    "| {} | {:.1} | {} | {} |",
                    c.variable,
                    c.percent,
                    c.permutation_importance
                        .map_or("–".into(), |p| format!("{p:.1}")),
                    c.rf_importance.map_or("–".into(), |p| format!("{p:.1}"))
                );
            }
        } else {
            let _ = writeln!(s, "| variable | RF permutation importance |\n|---|---|");
            for c in &r.contributions {
                let _ = writeln!(s, "| {} | {:.1} |", c.variable, c.percent);
            }
        }
    }
    let _ = writeln!(s, "\n## Model\n");
    let _ = writeln!(s, "{}", model_text(r));
    if let Some(b) = &r.validation.blocks {
        let _ = writeln!(s, "\n## Spatial block design\n");
        let _ = writeln!(
            s,
            "Square spatial blocks of {:.0} km ({}), assigned to folds so that presence counts are balanced (Valavi et al. 2019, blockCV; Roberts et al. 2017). {} folds requested, {} obtained.",
            b.block_km,
            if b.chosen_by == "user" {
                "size given by the user".to_string()
            } else {
                "sized by the spatial autocorrelation of the predictors".to_string()
            },
            b.k_requested,
            b.k_achieved
        );
        if let Some(a) = &b.autocorrelation {
            let _ = writeln!(
                s,
                "\nVariogram ranges of the predictors on {} background points (exponential, spherical and Gaussian models, weighted least squares; lag cutoff {:.0} km). Median {}; block size clamped to [{:.0}, {:.0}] km.\n\n| variable | range (km) | model |\n|---|---|---|",
                a.n_points,
                a.cutoff_km,
                a.median_km
                    .map_or("not estimable".into(), |m| format!("{m:.0} km")),
                a.min_km,
                a.max_km
            );
            for v in &a.variables {
                let _ = writeln!(
                    s,
                    "| {} | {} | {} |",
                    v.variable,
                    v.range_km.map_or("–".into(), |x| format!(
                        "{}{x:.0}",
                        if v.at_upper_bound { "≥ " } else { "" }
                    )),
                    v.model.as_deref().unwrap_or("–")
                );
            }
        }
    }
    let _ = writeln!(s, "\n## Validation (held-out folds)\n");
    let _ = writeln!(
        s,
        "| fold | test presences | Boyce | OR10 | ORmtp | AUC (presence-background) |\n|---|---|---|---|---|---|"
    );
    for f in &r.validation.folds {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {:.2} | {:.2} | {:.2} |",
            f.fold,
            f.n_test,
            f.cbi.map_or("–".into(), |c| format!("{c:.2}")),
            f.or10,
            f.ormtp,
            f.auc
        );
    }
    if !r.validation.members.is_empty() {
        let _ = writeln!(
            s,
            "\nThe metrics above are those of the ensemble itself (fold by fold, the ensemble of the two members' fold models). Members on the same folds:\n\n| member | mean Boyce | mean OR10 | mean AUC |\n|---|---|---|---|"
        );
        for m in &r.validation.members {
            let _ = writeln!(
                s,
                "| {} | {} | {:.2} | {:.2} |",
                member_label(&m.algorithm),
                m.cbi_mean.map_or("–".into(), |c| format!("{c:.2}")),
                m.or10_mean,
                m.auc_mean
            );
        }
    }
    if let Some(nm) = &r.validation.null_model {
        let _ = writeln!(s, "\n## Null models\n");
        let _ = writeln!(
            s,
            "The selected setting was refitted on {} sets of {} random locations ({} sampling over cells with predictor data), each partitioned and evaluated like the real records (Raes & ter Steege 2007; Bohl et al. 2019; ENMeval 2.0, Kass et al. 2021); {} of {} null sets were fitted. p is the empirical one-sided p-value (1 + nulls at least as good) / (n + 1). A small p means the model is better than random-location models; it does not show that the model is ecologically right.\n\n| metric | observed | null mean ± SD | z | p |\n|---|---|---|---|---|",
            nm.requested,
            nm.presences,
            null_sampling_label(&nm.sampling),
            nm.completed,
            nm.requested
        );
        for (label, st) in [
            ("Boyce index (higher is better)", &nm.cbi),
            ("OR10 (lower is better)", &nm.or10),
        ] {
            let f = |x: Option<f64>| x.map_or("–".into(), |v| format!("{v:.3}"));
            let _ = writeln!(
                s,
                "| {label} | {} | {} ± {} | {} | {:.3} |",
                f(st.observed),
                f(st.mean),
                f(st.sd),
                f(st.z),
                st.p_value
            );
        }
    }
    s.push_str(&crate::independent::validation_md(r));
    let _ = writeln!(s, "\n## Suitable-conditions area\n");
    let _ = writeln!(
        s,
        "Areas are sums of exact WGS84 ellipsoidal cell areas above each threshold; they describe the extent of suitable *conditions*, not occupied range.\n"
    );
    for t in &r.thresholds {
        let _ = writeln!(
            s,
            "- {} (≥ {:.3}): {:.0} km² ({:.1} % of study area)",
            t.label,
            t.threshold,
            t.area_km2,
            t.fraction_of_study_area * 100.0
        );
    }
    let _ = writeln!(s, "\n## Uncertainty and extrapolation\n");
    let _ = writeln!(
        s,
        "- Mean SD of suitability across fold models: {:.3}.",
        r.uncertainty.mean_fold_sd
    );
    let _ = writeln!(
        s,
        "- Share of the study area with MESS < 0 (outside training conditions): {:.1} %.",
        r.extrapolation.novel_fraction * 100.0
    );
    if let Some(a) = &r.aoa {
        let _ = writeln!(
            s,
            "- Area of applicability (Meyer & Pebesma 2021): {:.1} % of the study area is outside it (dissimilarity index > {:.3}, the upper whisker of the cross-validated training DI; {} training points; predictors weighted by permutation importance). It complements MESS and does not change the evidence level (ADR-0014). Layer: `aoa.tif`.",
            a.fraction_outside * 100.0,
            a.threshold,
            a.n_train
        );
    }
    if !r.projections.is_empty() {
        s.push_str(&crate::projection::report_md(r));
    }
    let _ = writeln!(s, "\n## Interpretation limits\n");
    let _ = writeln!(
        s,
        "- Values are relative habitat suitability, not probability of presence: presence-background data cannot estimate prevalence."
    );
    if let Some(t) = rf_limit_text(r) {
        let _ = writeln!(s, "- {t}");
    }
    let _ = writeln!(s, "- {}", bias_limit_text(r));
    let _ = writeln!(
        s,
        "- Climate baseline 1981–2010; land cover, soils and dispersal limits are not represented unless supplied as predictors."
    );
    for l in env::predictor_limits(&spec.layers) {
        let _ = writeln!(s, "- {l}");
    }
    for w in &r.warnings {
        let _ = writeln!(s, "- {:?}: {}", w.severity, w.message);
    }
    let refs = references(r, spec);
    if !refs.is_empty() {
        let _ = writeln!(s, "\n## References (bias and cleaning)\n");
        for x in refs {
            let _ = writeln!(s, "- {x}");
        }
    }
    let refs = model_references(r);
    if !refs.is_empty() {
        let _ = writeln!(s, "\n## References (random forest)\n");
        for x in refs {
            let _ = writeln!(s, "- {x}");
        }
    }
    if !r.projections.is_empty() {
        s.push_str(&crate::projection::report_md_references(r));
    }
    s
}

/// Values of a single-band GeoTIFF written by NicheRS (the reports of a species with
/// projections are rendered from its written rasters, so a resumed species gives the same
/// reports).
pub fn read_tif(path: &Path) -> Result<Vec<f32>, AppError> {
    let src = niche_geo::geotiff::FileSource::open(path)
        .map_err(|e| AppError::Storage(format!("{}: {e}", path.display())))?;
    let info = niche_geo::geotiff::GeoTiffInfo::read(&src)
        .map_err(|e| AppError::Storage(format!("{}: {e}", path.display())))?;
    Ok(info
        .read_window(&src, 0, 0, info.height, info.width)
        .map_err(|e| AppError::Storage(format!("{}: {e}", path.display())))?
        .data)
}

/// How the random forest's importance is computed (reports).
pub const RF_IMPORTANCE_TEXT: &str = "Random forest: permutation importance, the mean drop in training presence–background AUC when the variable's values are shuffled (3 repeats, presences and up to 2000 background points), normalised to 100 (Breiman 2001). It measures how much the fitted forest relies on a variable, not its ecological effect. Partial dependence: `partial_dependence.csv` and `report.html`.";

/// Name of an ensemble member.
pub fn member_label(algorithm: &str) -> &str {
    match algorithm {
        "maxent" => "MaxEnt",
        "rf" => "Down-sampled random forest",
        other => other,
    }
}

/// "Model" paragraph of the reports (plain text).
pub fn model_text(r: &SpeciesResult) -> String {
    let sel = &r.selected;
    let cv = format!(
        "{} cross-validation ({} folds)",
        r.validation.method.replace('_', " "),
        r.validation.folds.len()
    );
    let rf = || {
        format!(
            "down-sampled (balanced) random forest (Breiman 2001; Valavi et al. 2021; NicheRS niche-rf): {} classification trees, each grown fully on a bootstrap of the presences and as many background points drawn with replacement, mtry = {} variables tried per split{}, seed {}; output = fraction of trees voting presence",
            sel.rf_trees.unwrap_or(0),
            sel.rf_mtry.unwrap_or(0),
            if r.tuning
                .iter()
                .filter(|t| t.algorithm.as_deref() == Some("rf"))
                .count()
                > 1
            {
                " (selected among the candidate mtry values by the same criteria)"
            } else {
                ""
            },
            sel.rf_seed.unwrap_or(0)
        )
    };
    let n_maxent = r.tuning.iter().filter(|t| t.algorithm.is_none()).count();
    match sel.algorithm() {
        "rf" => format!(
            "Algorithm: {}. Evaluated under {cv} with the same folds, metrics and selection rule as MaxEnt. The output is a relative suitability index, not a probability of presence: the balanced samples fix the apparent prevalence at 1:1.",
            rf()
        ),
        "ensemble" => format!(
            "Algorithm: ensemble of MaxEnt (NicheRS port of MaxEnt 3.4.4; Phillips et al. 2006, 2017; cloglog; feature classes {} and regularization multiplier {} selected among {} candidate settings by lowest mean 10th-percentile omission, then highest Boyce index) and a {}. Each member's prediction is converted to its percentile rank among its own predictions at the background points, and the two ranks are averaged with equal weights (unweighted average of tuned members as in Valavi et al. 2022, with percentile ranks instead of min–max rescaling so that the two scales are comparable). The ensemble is evaluated as a model of its own under {cv}, and thresholds are computed on its output with the same rules. Its output is a relative suitability index (mean background percentile), not a probability of presence.",
            sel.feature_classes,
            sel.beta_multiplier,
            n_maxent.max(1),
            rf()
        ),
        _ => format!(
            "MaxEnt (NicheRS port of MaxEnt 3.4.4; Phillips et al. 2006, 2017), cloglog output. Feature classes {} and regularization multiplier {} selected among {} candidate settings by lowest mean 10th-percentile omission, then highest Boyce index, under {cv}.",
            sel.feature_classes,
            sel.beta_multiplier,
            r.tuning.len().max(1),
        ),
    }
}

/// Interpretation limit of the random forest (absent for MaxEnt).
pub fn rf_limit_text(r: &SpeciesResult) -> Option<&'static str> {
    match r.selected.algorithm() {
        "rf" => Some(
            "The random forest's output is a relative suitability index (fraction of trees voting presence), not a probability of presence; its partial-dependence curves average over the other variables and are not MaxEnt response curves.",
        ),
        "ensemble" => Some(
            "The ensemble's output is a relative suitability index (mean percentile rank of MaxEnt and the random forest among the background points), not a probability of presence; values are ranks within this study area and are not comparable between studies.",
        ),
        _ => None,
    }
}

/// References of the random-forest engine (ADR-0017); empty for MaxEnt.
pub fn model_references(r: &SpeciesResult) -> Vec<&'static str> {
    match r.selected.algorithm() {
        "rf" => vec![
            REF_BREIMAN_2001,
            REF_CHEN_2004,
            REF_VALAVI_2021,
            REF_FRIEDMAN_2001,
        ],
        "ensemble" => vec![
            REF_BREIMAN_2001,
            REF_CHEN_2004,
            REF_VALAVI_2021,
            REF_VALAVI_2022,
            REF_FRIEDMAN_2001,
        ],
        _ => vec![],
    }
}

pub const REF_BREIMAN_2001: &str = "Breiman, L. (2001) Random forests. Machine Learning 45, 5–32. https://doi.org/10.1023/A:1010933404324";
pub const REF_CHEN_2004: &str = "Chen, C., Liaw, A. & Breiman, L. (2004) Using random forest to learn imbalanced data. Technical Report 666, Department of Statistics, University of California, Berkeley.";
pub const REF_VALAVI_2021: &str = "Valavi, R., Elith, J., Lahoz-Monfort, J.J. & Guillera-Arroita, G. (2021) Modelling species presence-only data with random forests. Ecography 44, 1731–1742. https://doi.org/10.1111/ecog.05615";
pub const REF_VALAVI_2022: &str = "Valavi, R., Guillera-Arroita, G., Lahoz-Monfort, J.J. & Elith, J. (2022) Predictive performance of presence-only species distribution models: a benchmark study with reproducible code. Ecological Monographs 92, e01486. https://doi.org/10.1002/ecm.1486";
pub const REF_FRIEDMAN_2001: &str = "Friedman, J.H. (2001) Greedy function approximation: a gradient boosting machine. Annals of Statistics 29, 1189–1232.";

/// How null presences were drawn, in words.
pub fn null_sampling_label(sampling: &str) -> &str {
    match sampling {
        "effort_weighted" => "target-group effort-weighted (bias-matched)",
        other => other,
    }
}

/// References of the bias and cleaning methods used by a run (plain text). Empty for a default
/// run on synthetic data, so default reports are unchanged.
pub fn references(r: &SpeciesResult, spec: &JobSpec) -> Vec<&'static str> {
    let mut v = Vec::new();
    if r.data.generalised.is_some() {
        v.push(REF_CHAPMAN);
    }
    if spec.filters.thin_km.is_some() {
        v.push(REF_SPTHIN);
    }
    if r.background_sampling.is_some() {
        v.extend([REF_PHILLIPS_2009, REF_BARBER_2022, REF_VOLLERING_2019]);
    }
    if r.validation
        .null_model
        .as_ref()
        .is_some_and(|n| n.sampling == "effort_weighted")
    {
        v.push(REF_BOHL_2019);
    }
    v
}

pub const REF_CHAPMAN: &str = "Chapman, A.D. (2020) Current Best Practices for Generalizing Sensitive Species Occurrence Data. GBIF Secretariat, Copenhagen. https://doi.org/10.15468/doc-5jp4-5g10";
pub const REF_SPTHIN: &str = "Aiello-Lammens, M.E. et al. (2015) spThin: an R package for spatial thinning of species occurrence records for use in ecological niche models. Ecography 38, 541–545.";
pub const REF_PHILLIPS_2009: &str = "Phillips, S.J. et al. (2009) Sample selection bias and presence-only distribution models: implications for background and pseudo-absence data. Ecological Applications 19, 181–197.";
pub const REF_BARBER_2022: &str = "Barber, R.A. et al. (2022) Target-group backgrounds prove effective at correcting sampling bias in Maxent models. Diversity and Distributions 28, 128–141.";
pub const REF_VOLLERING_2019: &str = "Vollering, J. et al. (2019) Bunching up the background betters bias in species distribution models. Ecography 42, 1717–1727.";
pub const REF_BOHL_2019: &str = "Bohl, C.L., Kass, J.M. & Anderson, R.P. (2019) A new null model approach to quantify performance and significance for ecological niche models of species distributions. Journal of Biogeography 46, 1101–1111.";

/// Excluded records written for map display and export (exact counts are in the report). Whole
/// national batches can exclude hundreds of thousands of records; drawing them all would make the
/// map unusable and the file huge.
pub const MAX_EXCLUDED_POINTS: usize = 20_000;

/// GeoJSON with all retained presences and an evenly spaced, deterministic subset of excluded
/// records, written without building a JSON tree (bounded memory for millions of records).
pub fn occurrences_geojson(points: &[OccurrencePoint]) -> String {
    let excluded = points.iter().filter(|p| p.status != "retained").count();
    let stride = excluded.div_ceil(MAX_EXCLUDED_POINTS).max(1);
    let mut s = String::from("{\"type\":\"FeatureCollection\",\"features\":[");
    let mut first = true;
    let mut ex_i = 0usize;
    for p in points {
        if p.status != "retained" {
            ex_i += 1;
            if !(ex_i - 1).is_multiple_of(stride) {
                continue;
            }
        }
        if !first {
            s.push(',');
        }
        first = false;
        let year = p.year.map_or_else(|| "null".to_string(), |y| y.to_string());
        let status = serde_json::to_string(&p.status).unwrap_or_else(|_| "\"unknown\"".into());
        // The source only when the organisation's data are used (ADR-0108): GBIF-only outputs
        // are unchanged.
        let source = p.source.as_ref().map_or_else(String::new, |src| {
            format!(
                ",\"source\":{}",
                serde_json::to_string(src).unwrap_or_else(|_| "\"unknown\"".into())
            )
        });
        let _ = write!(
            s,
            "{{\"type\":\"Feature\",\"geometry\":{{\"type\":\"Point\",\"coordinates\":[{},{}]}},\"properties\":{{\"status\":{status},\"year\":{year}{source}}}}}",
            p.lng, p.lat
        );
    }
    let _ = write!(
        s,
        "],\"excluded_total\":{excluded},\"excluded_written\":{}}}",
        excluded.div_ceil(stride)
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geojson_keeps_all_retained_and_caps_excluded() {
        let mut pts: Vec<OccurrencePoint> = (0..50_000)
            .map(|i| OccurrencePoint {
                lng: f64::from(i) * 1e-4,
                lat: 45.0,
                year: Some(2020),
                status: "duplicate_cell".into(),
                source: None,
            })
            .collect();
        pts.extend((0..300).map(|i| OccurrencePoint {
            lng: 1.0 + f64::from(i) * 1e-3,
            lat: 46.0,
            year: None,
            status: "retained".into(),
            source: None,
        }));
        let v: serde_json::Value =
            serde_json::from_str(&occurrences_geojson(&pts)).expect("valid JSON");
        let feats = v["features"].as_array().expect("features");
        let retained = feats
            .iter()
            .filter(|f| f["properties"]["status"] == "retained")
            .count();
        assert_eq!(retained, 300);
        assert!(feats.len() - retained <= MAX_EXCLUDED_POINTS);
        assert_eq!(v["excluded_total"], 50_000);
    }
}
