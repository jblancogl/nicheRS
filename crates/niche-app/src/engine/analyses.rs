//! Decision-support analyses in the engine (ADR-0018): reading a finished study's outputs,
//! writing each analysis into `jobs/{id}/analyses/{aid}/` with its manifest addendum, listing,
//! serving and deleting them. The computations are in [`crate::analyses`].
//!
//! The study directory is never modified outside `analyses/`, so the study's execution manifest
//! and files stay as they were written. Each analysis directory appears atomically (written in a
//! temporary directory, then renamed). Survey priority, aggregation and comparison have
//! deterministic ids (one per species / minimum level / pair of studies, computed once);
//! footprints get a new id each time.

use std::path::PathBuf;
use std::sync::Arc;

use niche_geo::Raster;
use niche_geo::write::write_geotiff_f32;
use serde_json::json;

use super::Engine;
use crate::AppError;
use crate::accounts::Scope;
use crate::analyses::{
    self, DEFAULT_TOP_N, MANIFEST_FORMAT, MAX_FOOTPRINTS, MAX_TOP_N, SpeciesLayers, class_colour,
    report,
};
use crate::artifacts::{sha256, species_dir, write_atomic};
use crate::dto::{
    Analysis, AnalysisLayer, AnalysisRequest, EvidenceLevel, FileInfo, JobSpec, JobStatus,
    Severity, SpeciesResult, SpeciesRunStatus, Warning,
};

const TIF: &str = "image/tiff; application=geotiff";

/// A study file read by an analysis (recorded with its checksum in the manifest addendum).
#[derive(Clone)]
struct Input {
    job_id: String,
    species_key: u64,
    file: String,
    sha256: String,
}

/// A file of an analysis before it is written.
struct OutFile {
    name: String,
    description: String,
    media_type: String,
    bytes: Vec<u8>,
}

impl OutFile {
    fn new(name: &str, description: &str, media_type: &str, bytes: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            media_type: media_type.into(),
            bytes,
        }
    }

    fn tif(name: &str, description: &str, r: &Raster) -> Result<Self, AppError> {
        Ok(Self::new(
            name,
            description,
            TIF,
            write_geotiff_f32(r).map_err(|e| AppError::Internal(e.to_string()))?,
        ))
    }
}

/// Analysis ids are generated here; anything else in a route is refused before touching files.
fn valid_id(aid: &str) -> bool {
    !aid.is_empty()
        && aid.len() <= 160
        && aid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn io(e: std::io::Error) -> AppError {
    AppError::Storage(e.to_string())
}

fn layer_range(id: &str, r: &Raster) -> AnalysisLayer {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in &r.data {
        if v.is_finite() {
            lo = lo.min(f64::from(*v));
            hi = hi.max(f64::from(*v));
        }
    }
    if !lo.is_finite() {
        (lo, hi) = (0.0, 0.0);
    }
    AnalysisLayer {
        id: id.into(),
        min: lo,
        max: hi,
    }
}

fn level_slug(l: EvidenceLevel) -> &'static str {
    match l {
        EvidenceLevel::Insufficient => "insufficient",
        EvidenceLevel::Exploratory => "exploratory",
        EvidenceLevel::Indicative => "indicative",
        EvidenceLevel::Supported => "supported",
        EvidenceLevel::Validated => "validated",
    }
}

impl Engine {
    fn analyses_dir(&self, id: &str) -> PathBuf {
        self.cfg.data_dir.join("jobs").join(id).join("analyses")
    }

    /// The study, which must have finished successfully (its manifest written).
    fn finished_study(&self, id: &str, scope: &Scope) -> Result<(JobSpec, Vec<u8>), AppError> {
        let job = self.job(id, scope)?;
        if job.status != JobStatus::Succeeded {
            return Err(AppError::Conflict(
                "decision-support outputs need a study that finished successfully".into(),
            ));
        }
        let manifest = std::fs::read(self.job_file(id, "manifest.json"))
            .map_err(|_| AppError::Conflict("the study has no execution manifest yet".into()))?;
        Ok((job.spec, manifest))
    }

    /// A species' result and, when it has a model, its layers; every file read is recorded.
    fn species_layers(
        &self,
        id: &str,
        key: u64,
        inputs: &mut Vec<Input>,
    ) -> Result<(SpeciesResult, Option<SpeciesLayers>), AppError> {
        let dir = species_dir(&self.cfg.data_dir, id, key);
        let mut read = |name: &str| -> Option<Vec<u8>> {
            let b = std::fs::read(dir.join(name)).ok()?;
            inputs.push(Input {
                job_id: id.into(),
                species_key: key,
                file: name.into(),
                sha256: sha256(&b),
            });
            Some(b)
        };
        let r: SpeciesResult = serde_json::from_slice(
            &read("result.json")
                .ok_or_else(|| AppError::NotFound(format!("results for species {key}")))?,
        )
        .map_err(|e| AppError::Storage(e.to_string()))?;
        if r.status != SpeciesRunStatus::Succeeded {
            return Ok((r, None));
        }
        let listed = |n: &str| r.files.iter().any(|f| f.name == n);
        let mut tif = |name: &str| -> Result<Option<Raster>, AppError> {
            if !listed(name) {
                return Ok(None);
            }
            read(name).map(analyses::read_tif).transpose()
        };
        let suitability = tif("suitability.tif")?
            .ok_or_else(|| AppError::NotFound(format!("suitability of species {key}")))?;
        let layers = SpeciesLayers {
            suitability,
            mess: tif("mess.tif")?,
            uncertainty: tif("uncertainty.tif")?,
            aoa: tif("aoa.tif")?,
            effort: tif("effort.tif")?,
            result: r.clone(),
        };
        Ok((r, Some(layers)))
    }

    /// Run an analysis of study `id` (on demand). Returns it and whether it was created now
    /// (`false`: the same deterministic analysis already existed and is returned unchanged).
    /// `scope` must be the caller's organisation: analyses are stored with the study, and a
    /// compared study must also be the organisation's.
    pub fn create_analysis(
        &self,
        id: &str,
        req: &AnalysisRequest,
        scope: &Scope,
    ) -> Result<(Analysis, bool), AppError> {
        let (spec, manifest) = self.finished_study(id, scope)?;
        let aid = match req {
            AnalysisRequest::Priority { species_key, .. } => format!("priority-{species_key}"),
            AnalysisRequest::Richness { min_evidence } => format!(
                "richness-{}",
                level_slug(min_evidence.unwrap_or(EvidenceLevel::Indicative))
            ),
            AnalysisRequest::Comparison {
                other_job_id,
                species_key,
            } => {
                if !valid_id(other_job_id) {
                    return Err(AppError::NotFound("study".into()));
                }
                format!("comparison-{other_job_id}-{species_key}")
            }
            AnalysisRequest::Footprint { .. } => {
                format!("footprint-{}", uuid::Uuid::now_v7())
            }
        };
        if let Ok(a) = self.analysis(id, &aid) {
            return Ok((a, false));
        }
        let mut inputs = Vec::new();
        let mut warnings = Vec::new();
        if spec.demo {
            warnings.push(Warning::new(
                "synthetic",
                Severity::Info,
                "Synthetic demonstration data: a virtual species on artificial predictors. Results illustrate the workflow only.",
            ));
        }
        let mut a = Analysis {
            id: aid.clone(),
            kind: String::new(),
            job_id: id.into(),
            created_at: jiff::Timestamp::now().to_string(),
            title: String::new(),
            species_key: None,
            synthetic: spec.demo,
            files: Vec::new(),
            layers: Vec::new(),
            warnings: Vec::new(),
            priority: None,
            footprint: None,
            richness: None,
            comparison: None,
        };
        let mut files: Vec<OutFile> = Vec::new();
        let mut request = serde_json::to_value(req).unwrap_or_default();
        let mut other_study = serde_json::Value::Null;
        let method: serde_json::Value;
        let html: String;
        match req {
            AnalysisRequest::Priority { species_key, top_n } => {
                let top_n = top_n.unwrap_or(DEFAULT_TOP_N);
                if !(1..=MAX_TOP_N).contains(&top_n) {
                    return Err(AppError::Invalid(format!(
                        "top_n must be between 1 and {MAX_TOP_N}"
                    )));
                }
                self.species_in(&spec, *species_key)?;
                let (_, layers) = self.species_layers(id, *species_key, &mut inputs)?;
                let l = layers.ok_or_else(|| {
                    AppError::Invalid("this species has no model to prioritise surveys".into())
                })?;
                let known: Vec<(f64, f64)> = if l.effort.is_some() {
                    Vec::new()
                } else {
                    let path = species_dir(&self.cfg.data_dir, id, *species_key)
                        .join("occurrences.geojson");
                    if let Ok(b) = std::fs::read(&path) {
                        inputs.push(Input {
                            job_id: id.into(),
                            species_key: *species_key,
                            file: "occurrences.geojson".into(),
                            sha256: sha256(&b),
                        });
                    }
                    self.occurrences(id, *species_key)?
                        .into_iter()
                        .filter(|p| {
                            matches!(p.status.as_str(), "retained" | "thinned" | "validation")
                        })
                        .map(|p| (p.lng, p.lat))
                        .collect()
                };
                let out = analyses::priority::compute(&l, &known, top_n)?;
                warnings.extend(out.warnings);
                let p = out.result;
                a.kind = "priority".into();
                a.title = format!("Survey priority — {}", p.scientific_name);
                a.species_key = Some(*species_key);
                a.layers = vec![
                    layer_range("priority", &out.score),
                    layer_range("priority_class", &out.classes),
                ];
                method = json!({
                    "score": if p.method == "effort" { "suitability percentile × (1 − effort percentile)" } else { "suitability percentile × distance-to-known-records percentile" },
                    "percentiles": "mid-rank among the study-area cells (cells with a prediction)",
                    "suitability_split": "species' P10 threshold",
                    "sampling_split": if p.method == "effort" { "effort percentile < 0.5 = little sampled" } else { "distance percentile > 0.5 = little sampled (far from known records)" },
                    "known_records": if p.method == "distance" { json!(["retained", "thinned", "validation"]) } else { serde_json::Value::Null },
                    "candidates": "8-connected groups of class 1 (prospect first), ranked by area-weighted mean priority",
                    "top_n": top_n,
                });
                files.push(OutFile::tif(
                    "priority.tif",
                    "Survey priority score (0–1): suitability percentile × little-sampled percentile within the study area. Areas worth surveying, not where the species is",
                    &out.score,
                )?);
                files.push(OutFile::tif(
                    "priority_class.tif",
                    "Survey priority classes: 1 prospect first (higher suitability, little sampled), 2 known (higher suitability, well sampled), 3 lower suitability and little sampled, 4 lower suitability and well sampled",
                    &out.classes,
                )?);
                files.push(OutFile::new(
                    "candidates.geojson",
                    "Candidate areas worth surveying: centroid points with bounding box, area, priority, suitability and sampling (GeoJSON, WGS84)",
                    "application/geo+json",
                    serde_json::to_vec_pretty(&analyses::priority::candidates_geojson(
                        &p, spec.demo,
                    ))
                    .map_err(|e| AppError::Internal(e.to_string()))?,
                ));
                files.push(OutFile::new(
                    "candidates.csv",
                    "Candidate areas worth surveying (same content as the GeoJSON, one row per area)",
                    "text/csv",
                    analyses::priority::candidates_csv(&p).into_bytes(),
                ));
                a.priority = Some(p);
                a.warnings = warnings.clone();
                let score_style = |v: f32| {
                    let c = niche_geo::colormap::priority(v);
                    Some([c[0], c[1], c[2], 255])
                };
                let class_style = |v: f32| class_colour(v);
                html = report::html(
                    &a,
                    &spec,
                    &[
                        report::Figure {
                            raster: &out.score,
                            style: &score_style,
                            caption: "Survey priority score (0 pale → 1 dark purple).".into(),
                            legend: String::new(),
                        },
                        report::Figure {
                            raster: &out.classes,
                            style: &class_style,
                            caption: "Survey priority classes (colours as in the class table)."
                                .into(),
                            legend: String::new(),
                        },
                    ],
                );
            }
            AnalysisRequest::Footprint { name, geometry } => {
                let polys = analyses::footprint::polygons(name, geometry)?;
                let existing = self.list_analyses(id)?;
                if existing.iter().filter(|x| x.kind == "footprint").count() >= MAX_FOOTPRINTS {
                    return Err(AppError::Conflict(format!(
                        "a study keeps at most {MAX_FOOTPRINTS} footprints; delete one first"
                    )));
                }
                let mut species = Vec::new();
                for sp in &spec.species {
                    species.push(self.species_layers(id, sp.key, &mut inputs)?);
                }
                let (f, w) = analyses::footprint::compute(name, geometry, &polys, &species)?;
                warnings.extend(w);
                a.kind = "footprint".into();
                a.title = format!("Project footprint — {}", f.name);
                if let Some(o) = request.as_object_mut() {
                    o.insert(
                        "geometry".into(),
                        json!({"file": "footprint.geojson", "sha256": sha256(geometry.to_string().as_bytes())}),
                    );
                }
                method = json!({
                    "overlap": "exact ellipsoidal area (WGS84) of the footprint ∩ each model cell, edges straight in longitude/latitude",
                    "thresholds": ["p10 (headline)", "mtp", "max_sss"],
                    "means": "area-weighted over the footprint part with predictions",
                    "extrapolation": "MESS < 0",
                    "aoa": "dissimilarity index above the species' AOA threshold (when computed)",
                });
                files.push(OutFile::new(
                    "footprint.geojson",
                    "The project footprint as submitted (GeoJSON, WGS84)",
                    "application/geo+json",
                    serde_json::to_vec_pretty(&json!({
                        "type": "Feature",
                        "properties": {"name": f.name, "area_km2": f.area_km2},
                        "geometry": geometry,
                    }))
                    .map_err(|e| AppError::Internal(e.to_string()))?,
                ));
                files.push(OutFile::new(
                    "footprint_species.csv",
                    "Suitable conditions inside the footprint per species and threshold rule (km², shares), mean/max suitability, uncertainty, extrapolated and outside-AOA shares. Does not replace a field inventory or an impact assessment",
                    "text/csv",
                    analyses::footprint::species_csv(&f).into_bytes(),
                ));
                // Locator map: the headline species' suitability around the footprint.
                let locator = species
                    .iter()
                    .find_map(|(_, l)| l.as_ref())
                    .and_then(|l| footprint_locator(&l.suitability, &polys));
                a.footprint = Some(f);
                a.warnings = warnings.clone();
                let style = |v: f32| {
                    if v >= 2.0 {
                        let c = niche_geo::colormap::viridis(v - 2.0);
                        let m = niche_geo::colormap::HOTSPOT;
                        let mix = |x: u8, y: u8| ((u16::from(x) + u16::from(y)) / 2) as u8;
                        Some([mix(c[0], m[0]), mix(c[1], m[1]), mix(c[2], m[2]), 255])
                    } else {
                        let c = niche_geo::colormap::viridis(v);
                        Some([c[0], c[1], c[2], 255])
                    }
                };
                let figs: Vec<report::Figure<'_>> = locator
                    .as_ref()
                    .map(|r| report::Figure {
                        raster: r,
                        style: &style,
                        caption: format!(
                            "Relative suitability of {} around the footprint (footprint cells tinted pink).",
                            a.footprint
                                .as_ref()
                                .and_then(|f| f.species.iter().find(|s| s.evidence_level.is_some()))
                                .map_or("the first species", |s| s.scientific_name.as_str())
                        ),
                        legend: String::new(),
                    })
                    .into_iter()
                    .collect();
                html = report::html(&a, &spec, &figs);
            }
            AnalysisRequest::Richness { min_evidence } => {
                let min = min_evidence.unwrap_or(EvidenceLevel::Indicative);
                let mut species = Vec::new();
                for sp in &spec.species {
                    species.push(self.species_layers(id, sp.key, &mut inputs)?);
                }
                let out = analyses::richness::compute(&species, min)?;
                warnings.extend(out.warnings);
                let r = out.result;
                a.kind = "richness".into();
                a.title = format!(
                    "Species with suitable conditions — {} species",
                    r.included.len()
                );
                a.layers = vec![
                    layer_range("richness", &out.richness),
                    layer_range("suitability_sum", &out.sum),
                    layer_range("hotspot", &out.hotspot),
                    layer_range("contributing", &out.contributing),
                ];
                method = json!({
                    "richness": "count of included species with relative suitability ≥ their own P10 threshold",
                    "suitability_sum": "sum of relative suitability of the included species (index, not an expected number of species)",
                    "min_evidence": level_slug(min),
                    "hotspot": "top 10 % of study-area cells by count, ties by summed suitability, then cell order",
                });
                files.push(OutFile::tif(
                    "richness.tif",
                    "Number of species with suitable conditions (each at its own P10 threshold). Stacked models overpredict: not a count of species present",
                    &out.richness,
                )?);
                files.push(OutFile::tif(
                    "suitability_sum.tif",
                    "Sum of the species' relative suitability: an index for ranking cells, not an expected number of species",
                    &out.sum,
                )?);
                files.push(OutFile::tif(
                    "contributing.tif",
                    "Number of species with a prediction in the cell (the species included in the aggregation)",
                    &out.contributing,
                )?);
                files.push(OutFile::tif(
                    "hotspot.tif",
                    "Hotspots: 1 = top 10 % of the study-area cells by number of species with suitable conditions (ties by summed suitability), 0 = other cells",
                    &out.hotspot,
                )?);
                files.push(OutFile::new(
                    "richness_species.csv",
                    "Species included in and left out of the aggregation, with evidence level, P10 threshold and reason",
                    "text/csv",
                    analyses::richness::species_csv(&r).into_bytes(),
                ));
                let max = r.max_species.max(1) as f32;
                a.richness = Some(r);
                a.warnings = warnings.clone();
                let rich_style = move |v: f32| {
                    let c = niche_geo::colormap::viridis(v / max);
                    Some([c[0], c[1], c[2], 255])
                };
                let hot_style = |v: f32| {
                    if v >= 1.0 {
                        let c = niche_geo::colormap::HOTSPOT;
                        Some([c[0], c[1], c[2], 255])
                    } else {
                        Some([225, 225, 225, 255])
                    }
                };
                html = report::html(
                    &a,
                    &spec,
                    &[
                        report::Figure {
                            raster: &out.richness,
                            style: &rich_style,
                            caption: format!(
                                "Number of species with suitable conditions (0 dark → {} yellow).",
                                max as usize
                            ),
                            legend: String::new(),
                        },
                        report::Figure {
                            raster: &out.hotspot,
                            style: &hot_style,
                            caption: "Hotspots (pink): top 10 % of the cells.".into(),
                            legend: String::new(),
                        },
                    ],
                );
            }
            AnalysisRequest::Comparison {
                other_job_id,
                species_key,
            } => {
                if other_job_id == id {
                    return Err(AppError::Invalid(
                        "choose another study to compare with".into(),
                    ));
                }
                // The other study must be the organisation's too (404 otherwise).
                let (other_spec, other_manifest) = self.finished_study(other_job_id, scope)?;
                self.species_in(&spec, *species_key)?;
                self.species_in(&other_spec, *species_key).map_err(|_| {
                    AppError::Invalid(
                        "the other study does not model this species: comparisons are between studies of the same species".into(),
                    )
                })?;
                let (ra, la) = self.species_layers(id, *species_key, &mut inputs)?;
                let (rb, lb) = self.species_layers(other_job_id, *species_key, &mut inputs)?;
                let (Some(la), Some(lb)) = (la, lb) else {
                    return Err(AppError::Invalid(
                        "the species has no model in one of the two studies".into(),
                    ));
                };
                let out = analyses::comparison::compute(
                    &analyses::comparison::Side {
                        job_id: id,
                        spec: &spec,
                        result: &ra,
                        suitability: &la.suitability,
                    },
                    &analyses::comparison::Side {
                        job_id: other_job_id,
                        spec: &other_spec,
                        result: &rb,
                        suitability: &lb.suitability,
                    },
                )?;
                if other_spec.demo && !spec.demo {
                    a.synthetic = true;
                }
                let c = out.result;
                a.kind = "comparison".into();
                a.title = format!(
                    "Comparison — {}: {} vs {}",
                    c.scientific_name,
                    if spec.name.is_empty() {
                        "A"
                    } else {
                        &spec.name
                    },
                    if other_spec.name.is_empty() {
                        "B"
                    } else {
                        &other_spec.name
                    }
                );
                a.species_key = Some(*species_key);
                a.layers = vec![layer_range("difference", &out.difference)];
                other_study = json!({
                    "job_id": other_job_id,
                    "manifest_sha256": sha256(&other_manifest),
                    "spec_sha256": manifest_spec_hash(&other_manifest),
                });
                method = json!({
                    "difference": "B − A on the common grid (refused when grids differ)",
                    "schoener_d": "1 − ½ Σ|pA − pB|, each surface divided by its sum over the common cells",
                    "spearman": "Pearson correlation of mid-rank percentiles over the common cells",
                    "p10_agreement": "areas suitable at each study's own P10 threshold",
                });
                files.push(OutFile::tif(
                    "difference.tif",
                    "Difference of relative suitability, study B − study A, on the common grid (NaN where either has no prediction)",
                    &out.difference,
                )?);
                files.push(OutFile::new(
                    "comparison.csv",
                    "Settings, metrics, evidence and areas of both studies side by side, then Schoener's D, Spearman's rho and P10 agreement",
                    "text/csv",
                    analyses::comparison::csv(&c).into_bytes(),
                ));
                let m = a.layers[0].min.abs().max(a.layers[0].max.abs()).max(1e-6) as f32;
                a.comparison = Some(c);
                a.warnings = warnings.clone();
                let style = move |v: f32| {
                    let c = niche_geo::colormap::diverging(v / m);
                    Some([c[0], c[1], c[2], 255])
                };
                html = report::html(
                    &a,
                    &spec,
                    &[report::Figure {
                        raster: &out.difference,
                        style: &style,
                        caption: format!(
                            "Difference B − A (blue: lower in B, red: higher in B; scale ±{m:.2})."
                        ),
                        legend: String::new(),
                    }],
                );
            }
        }
        a.warnings = warnings;
        files.push(OutFile::new(
            "report.html",
            "Printable page: what this answers and what it does not, method, tables, maps, references and disclaimer (open in a browser; print to PDF)",
            "text/html",
            html.into_bytes(),
        ));
        let study = json!({
            "job_id": id,
            "manifest_sha256": sha256(&manifest),
            "spec_sha256": manifest_spec_hash(&manifest),
        });
        let a = self.write_analysis(a, files, &inputs, study, other_study, request, method)?;
        Ok((a, true))
    }

    fn species_in(&self, spec: &JobSpec, key: u64) -> Result<(), AppError> {
        spec.species
            .iter()
            .any(|s| s.key == key)
            .then_some(())
            .ok_or_else(|| AppError::NotFound(format!("species {key} in this study")))
    }

    #[allow(clippy::too_many_arguments)]
    fn write_analysis(
        &self,
        mut a: Analysis,
        files: Vec<OutFile>,
        inputs: &[Input],
        study: serde_json::Value,
        other_study: serde_json::Value,
        request: serde_json::Value,
        method: serde_json::Value,
    ) -> Result<Analysis, AppError> {
        let root = self.analyses_dir(&a.job_id);
        let tmp = root.join(format!(".tmp-{}", uuid::Uuid::now_v7()));
        let result = (|| -> Result<Analysis, AppError> {
            let mut outputs = serde_json::Map::new();
            for f in &files {
                let h = write_atomic(&tmp.join(&f.name), &f.bytes)?;
                outputs.insert(
                    f.name.clone(),
                    json!({"sha256": h, "bytes": f.bytes.len(), "media_type": f.media_type}),
                );
                a.files.push(FileInfo {
                    name: f.name.clone(),
                    description: f.description.clone(),
                    media_type: f.media_type.clone(),
                });
            }
            let summary =
                serde_json::to_vec_pretty(&a).map_err(|e| AppError::Internal(e.to_string()))?;
            let h = write_atomic(&tmp.join("analysis.json"), &summary)?;
            outputs.insert(
                "analysis.json".into(),
                json!({"sha256": h, "bytes": summary.len(), "media_type": "application/json"}),
            );
            let manifest = json!({
                "format": MANIFEST_FORMAT,
                "analysis_id": a.id,
                "kind": a.kind,
                "created_at": a.created_at,
                "software": {"name": "NicheRS", "version": env!("CARGO_PKG_VERSION")},
                "decision": "ADR-0018 (Proposed)",
                "study": study,
                "other_study": other_study,
                "synthetic_data": a.synthetic,
                "request": request,
                "method": method,
                "inputs": inputs.iter().map(|i| json!({"job_id": i.job_id, "species_key": i.species_key, "file": i.file, "sha256": i.sha256})).collect::<Vec<_>>(),
                "outputs": outputs,
            });
            write_atomic(
                &tmp.join("manifest.json"),
                &serde_json::to_vec_pretty(&manifest)
                    .map_err(|e| AppError::Internal(e.to_string()))?,
            )?;
            Ok(a.clone())
        })();
        let a = match result {
            Ok(a) => a,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp);
                return Err(e);
            }
        };
        let dest = root.join(&a.id);
        if let Err(e) = std::fs::rename(&tmp, &dest) {
            let _ = std::fs::remove_dir_all(&tmp);
            // Computed at the same time by another request: keep the first one.
            return self.analysis(&a.job_id, &a.id).map_err(|_| io(e));
        }
        if let Ok(d) = std::fs::File::open(&root) {
            let _ = d.sync_all();
        }
        // As stored (JSON numbers), so the first answer equals every later read.
        self.analysis(&a.job_id, &a.id)
    }

    fn list_analyses(&self, id: &str) -> Result<Vec<Analysis>, AppError> {
        let root = self.analyses_dir(id);
        let rd = match std::fs::read_dir(&root) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(io(e)),
        };
        let mut out = Vec::new();
        for e in rd.filter_map(Result::ok) {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || !valid_id(&name) {
                continue;
            }
            if let Ok(b) = std::fs::read(e.path().join("analysis.json"))
                && let Ok(a) = serde_json::from_slice::<Analysis>(&b)
            {
                out.push(a);
            }
        }
        out.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
        Ok(out)
    }

    /// Analyses of a study, oldest first. The caller has checked access to the study.
    pub fn analyses(&self, id: &str) -> Result<Vec<Analysis>, AppError> {
        self.list_analyses(id)
    }

    pub fn analysis(&self, id: &str, aid: &str) -> Result<Analysis, AppError> {
        if !valid_id(aid) {
            return Err(AppError::NotFound("analysis".into()));
        }
        let b = std::fs::read(self.analyses_dir(id).join(aid).join("analysis.json"))
            .map_err(|_| AppError::NotFound("analysis".into()))?;
        serde_json::from_slice(&b).map_err(|e| AppError::Storage(e.to_string()))
    }

    /// Delete one analysis (its directory) and its cached rasters.
    pub fn delete_analysis(&self, id: &str, aid: &str) -> Result<(), AppError> {
        self.analysis(id, aid)?;
        let dir = self.analyses_dir(id).join(aid);
        // Rename first so a concurrent reader never sees a half-deleted analysis.
        let gone = self
            .analyses_dir(id)
            .join(format!(".del-{}", uuid::Uuid::now_v7()));
        std::fs::rename(&dir, &gone).map_err(io)?;
        std::fs::remove_dir_all(&gone).map_err(io)?;
        if let Ok(mut r) = self.rasters.lock() {
            r.remove_prefix(&format!("{id}/analyses/{aid}/"));
        }
        Ok(())
    }

    /// Path and media type of an analysis file; only names listed in the analysis (and its
    /// manifest addendum) are served.
    pub fn analysis_file(
        &self,
        id: &str,
        aid: &str,
        name: &str,
    ) -> Result<(PathBuf, String), AppError> {
        let dir = self.analyses_dir(id).join(aid);
        if name == "manifest.json" {
            self.analysis(id, aid)?;
            return Ok((dir.join(name), "application/json".into()));
        }
        let a = self.analysis(id, aid)?;
        let f = a
            .files
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| AppError::NotFound(format!("file {name}")))?;
        Ok((dir.join(&f.name), f.media_type.clone()))
    }

    /// Raster of an analysis layer (cached like species layers) with the analysis.
    pub fn analysis_raster(
        &self,
        id: &str,
        aid: &str,
        layer: &str,
    ) -> Result<(Arc<Raster>, Analysis), AppError> {
        let a = self.analysis(id, aid)?;
        if !a.layers.iter().any(|l| l.id == layer) {
            return Err(AppError::NotFound(format!("layer {layer}")));
        }
        let file = format!("{layer}.tif");
        let cache_key = format!("{id}/analyses/{aid}/{file}");
        if let Some(r) = self
            .rasters
            .lock()
            .map_err(|_| AppError::Internal("lock".into()))?
            .get(&cache_key)
        {
            return Ok((r, a));
        }
        let bytes = std::fs::read(self.analyses_dir(id).join(aid).join(&file))
            .map_err(|_| AppError::NotFound(file.clone()))?;
        let r = Arc::new(analyses::read_tif(bytes)?);
        self.rasters
            .lock()
            .map_err(|_| AppError::Internal("lock".into()))?
            .put(cache_key, r.clone());
        Ok((r, a))
    }
}

fn manifest_spec_hash(manifest: &[u8]) -> serde_json::Value {
    serde_json::from_slice::<serde_json::Value>(manifest)
        .ok()
        .and_then(|m| m.get("spec_sha256").cloned())
        .unwrap_or(serde_json::Value::Null)
}

/// Suitability around a footprint (window of the footprint's cells plus a margin), with the
/// footprint's cells offset by +2 so the report can tint them.
fn footprint_locator(s: &Raster, polys: &[niche_geo::mask::Rings]) -> Option<Raster> {
    let spec = s.spec;
    let ov = niche_geo::overlay::cell_overlap_km2(polys, &spec);
    let (mut r0, mut r1, mut c0, mut c1) = (usize::MAX, 0, usize::MAX, 0);
    for (i, _) in &ov {
        let (r, c) = (i / spec.width, i % spec.width);
        r0 = r0.min(r);
        r1 = r1.max(r);
        c0 = c0.min(c);
        c1 = c1.max(c);
    }
    if ov.is_empty() {
        return None;
    }
    let margin = ((r1 - r0).max(c1 - c0) + 1).max(10);
    let (r0, c0) = (r0.saturating_sub(margin), c0.saturating_sub(margin));
    let r1 = (r1 + margin + 1).min(spec.height);
    let c1 = (c1 + margin + 1).min(spec.width);
    let sub = niche_geo::GridSpec {
        west: spec.west + c0 as f64 * spec.cell,
        north: spec.north - r0 as f64 * spec.cell,
        cell: spec.cell,
        width: c1 - c0,
        height: r1 - r0,
    };
    let mut out = Raster::new(sub, f32::NAN);
    for r in r0..r1 {
        for c in c0..c1 {
            out.data[(r - r0) * sub.width + (c - c0)] = s.data[spec.index(r, c)];
        }
    }
    for (i, _) in &ov {
        let (r, c) = (i / spec.width, i % spec.width);
        let k = (r - r0) * sub.width + (c - c0);
        if out.data[k].is_finite() {
            out.data[k] += 2.0;
        }
    }
    Some(out)
}
