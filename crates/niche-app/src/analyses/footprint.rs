//! Project-footprint overlay (ADR-0018 §2): what the models say inside a project's footprint,
//! for a pre-diagnostic.
//!
//! The footprint is intersected exactly with the model cells ([`niche_geo::overlay`]): each
//! cell contributes the ellipsoidal area of its overlap with the footprint, with the cell's
//! values (a footprint smaller than a cell is described by the cells it touches; said in a
//! warning). Suitable area per threshold rule, shares, area-weighted mean suitability and
//! uncertainty, and the extrapolated / outside-AOA shares are all over the part of the footprint
//! with predictions.

use niche_geo::mask::Rings;

use super::{SpeciesLayers, cell_areas, rules_in_order};
use crate::AppError;
use crate::dto::{
    FootprintResult, FootprintSpecies, FootprintThreshold, Severity, SpeciesResult,
    SpeciesRunStatus, Warning,
};

/// Footprint polygons from the request (same rules and vertex limit as study areas).
pub fn polygons(name: &str, geometry: &serde_json::Value) -> Result<Vec<Rings>, AppError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::Invalid(
            "give the footprint a name of 1 to 120 characters".into(),
        ));
    }
    let polys = niche_geo::mask::polygons_from_geojson(geometry)
        .map_err(|e| AppError::Invalid(format!("footprint: {e}")))?;
    let vertices: usize = polys.iter().flatten().map(Vec::len).sum();
    if vertices > 200_000 {
        return Err(AppError::Invalid(
            "the footprint has more than 200 000 vertices; simplify it first".into(),
        ));
    }
    Ok(polys)
}

/// `species`: every species of the study, with its layers when it has a model.
pub fn compute(
    name: &str,
    geometry: &serde_json::Value,
    polys: &[Rings],
    species: &[(SpeciesResult, Option<SpeciesLayers>)],
) -> Result<(FootprintResult, Vec<Warning>), AppError> {
    let Some(first) = species.iter().find_map(|(_, l)| l.as_ref()) else {
        return Err(AppError::Invalid(
            "no species of this study has a model to overlay".into(),
        ));
    };
    let spec = first.suitability.spec;
    let areas = cell_areas(&spec);
    let overlap = niche_geo::overlay::cell_overlap_km2(polys, &spec);
    let area_km2 = niche_geo::overlay::polygons_area_km2(polys);
    let with_pred: Vec<(usize, f64)> = overlap
        .iter()
        .copied()
        .filter(|(i, _)| first.suitability.data[*i].is_finite())
        .collect();
    let in_study: f64 = with_pred.iter().map(|c| c.1).sum();
    if with_pred.is_empty() || in_study <= 0.0 {
        return Err(AppError::Invalid(
            "the footprint does not overlap the study area (no model cell with a prediction under it)".into(),
        ));
    }
    let mean_cell = with_pred.iter().map(|(i, _)| areas[*i]).sum::<f64>() / with_pred.len() as f64;
    let mut warnings = Vec::new();
    if area_km2 < mean_cell {
        warnings.push(
            Warning::new(
                "footprint_smaller_than_cell",
                Severity::Warning,
                format!(
                    "The footprint ({area_km2:.2} km²) is smaller than one model cell ({mean_cell:.2} km²): the values describe the {} cell(s) it touches, not conditions within the footprint itself.",
                    with_pred.len()
                ),
            )
            .with("area_km2", (area_km2 * 100.0).round() / 100.0)
            .with("cell_km2", (mean_cell * 100.0).round() / 100.0)
            .with("cells", with_pred.len()),
        );
    }
    let outside = 1.0 - in_study / area_km2.max(f64::MIN_POSITIVE);
    if outside > 0.001 {
        warnings.push(
            Warning::new(
                "footprint_outside_study_area",
                Severity::Warning,
                format!(
                    "{:.1} % of the footprint lies outside the study area or over cells without predictions (sea, missing predictor data): shares are of the part with predictions.",
                    outside * 100.0
                ),
            )
            .with("percent", (outside * 1000.0).round() / 10.0),
        );
    }

    let mut out = Vec::new();
    let mut any_extrapolated = false;
    for (r, layers) in species {
        let Some(l) = layers
            .as_ref()
            .filter(|_| r.status == SpeciesRunStatus::Succeeded)
        else {
            out.push(FootprintSpecies {
                species_key: r.species_key,
                scientific_name: r.scientific_name.clone(),
                evidence_level: None,
                algorithm: None,
                thresholds: Vec::new(),
                mean_suitability: None,
                max_suitability: None,
                mean_uncertainty: None,
                extrapolated_share: None,
                outside_aoa_share: None,
                note: Some(
                    r.error
                        .clone()
                        .unwrap_or_else(|| "no model for this species".into()),
                ),
            });
            continue;
        };
        if !super::same_grid(&l.suitability.spec, &spec) {
            return Err(AppError::Internal(
                "species rasters of one study on different grids".into(),
            ));
        }
        let s = &l.suitability.data;
        let mut a_pred = 0.0;
        let (mut ws, mut wu, mut au, mut ex, mut out_aoa) = (0.0, 0.0, 0.0, 0.0, 0.0);
        let mut smax = f64::NEG_INFINITY;
        let aoa_t = r.aoa.as_ref().map(|a| a.threshold);
        for &(i, a) in &overlap {
            let v = f64::from(s[i]);
            if !v.is_finite() {
                continue;
            }
            a_pred += a;
            ws += a * v;
            smax = smax.max(v);
            if let Some(u) = l.uncertainty.as_ref().map(|u| f64::from(u.data[i]))
                && u.is_finite()
            {
                wu += a * u;
                au += a;
            }
            if l.mess.as_ref().is_some_and(|m| m.data[i] < 0.0) {
                ex += a;
            }
            if let (Some(t), Some(d)) = (aoa_t, l.aoa.as_ref())
                && f64::from(d.data[i]) > t
            {
                out_aoa += a;
            }
        }
        let thresholds = rules_in_order(r)
            .into_iter()
            .map(|t| {
                // fold from +0.0: an empty f64 `sum()` is -0.0, shown as "-0 ha" (QA 2026-10-20).
                let inside: f64 = overlap
                    .iter()
                    .filter(|(i, _)| f64::from(s[*i]) >= t.threshold)
                    .fold(0.0, |acc, c| acc + c.1);
                let whole: f64 = s
                    .iter()
                    .zip(&areas)
                    .filter(|(v, _)| f64::from(**v) >= t.threshold)
                    .fold(0.0, |acc, (_, a)| acc + a);
                FootprintThreshold {
                    rule: t.rule.clone(),
                    label: t.label.clone(),
                    threshold: t.threshold,
                    suitable_km2: inside,
                    share_of_footprint: if a_pred > 0.0 { inside / a_pred } else { 0.0 },
                    species_suitable_km2: whole,
                    share_of_species_area: if whole > 0.0 { inside / whole } else { 0.0 },
                }
            })
            .collect();
        let extrapolated = if a_pred > 0.0 { ex / a_pred } else { 0.0 };
        any_extrapolated |= extrapolated > 0.0;
        out.push(FootprintSpecies {
            species_key: r.species_key,
            scientific_name: r.scientific_name.clone(),
            evidence_level: Some(r.evidence.level),
            algorithm: Some(r.selected.algorithm().to_string()),
            thresholds,
            mean_suitability: (a_pred > 0.0).then(|| ws / a_pred),
            max_suitability: smax.is_finite().then_some(smax),
            mean_uncertainty: (au > 0.0).then(|| wu / au),
            extrapolated_share: Some(extrapolated),
            outside_aoa_share: (aoa_t.is_some() && l.aoa.is_some() && a_pred > 0.0)
                .then(|| out_aoa / a_pred),
            note: None,
        });
    }
    if any_extrapolated {
        warnings.push(Warning::new(
            "footprint_extrapolation",
            Severity::Warning,
            "For at least one species the model extrapolates (MESS < 0) over part of the footprint: values there are less reliable.",
        ));
    }
    Ok((
        FootprintResult {
            name: name.trim().to_string(),
            geometry: geometry.clone(),
            area_km2,
            in_study_km2: in_study,
            cells: with_pred.len(),
            mean_cell_km2: mean_cell,
            species: out,
        },
        warnings,
    ))
}

/// `footprint_species.csv`: one row per species and threshold rule.
pub fn species_csv(f: &FootprintResult) -> String {
    use super::{csv_field, num, opt_num};
    let mut s = String::from(
        "species_key,scientific_name,evidence_level,rule,threshold,suitable_km2_in_footprint,share_of_footprint,species_suitable_km2_in_study_area,share_of_species_suitable_area,mean_suitability,max_suitability,mean_uncertainty,extrapolated_share,outside_aoa_share,footprint_km2,footprint_with_predictions_km2,note\n",
    );
    for sp in &f.species {
        let level = sp
            .evidence_level
            .map(|l| format!("{l:?}").to_lowercase())
            .unwrap_or_default();
        let common = |s: &mut String| {
            s.push_str(&format!(
                ",{},{},{},{},{},{},{},{}\n",
                opt_num(sp.mean_suitability),
                opt_num(sp.max_suitability),
                opt_num(sp.mean_uncertainty),
                opt_num(sp.extrapolated_share),
                opt_num(sp.outside_aoa_share),
                num(f.area_km2),
                num(f.in_study_km2),
                csv_field(sp.note.as_deref().unwrap_or("")),
            ));
        };
        if sp.thresholds.is_empty() {
            s.push_str(&format!(
                "{},{},{},,,,,,",
                sp.species_key,
                csv_field(&sp.scientific_name),
                level
            ));
            common(&mut s);
        }
        for t in &sp.thresholds {
            s.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}",
                sp.species_key,
                csv_field(&sp.scientific_name),
                level,
                t.rule,
                num(t.threshold),
                num(t.suitable_km2),
                num(t.share_of_footprint),
                num(t.species_suitable_km2),
                num(t.share_of_species_area),
            ));
            common(&mut s);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use niche_geo::area::quad_area_km2;
    use niche_geo::{GridSpec, Raster};

    fn result() -> SpeciesResult {
        let t = |rule: &str, v: f64| serde_json::json!({"rule": rule, "label": rule, "threshold": v, "area_km2": 0.0, "fraction_of_study_area": 0.0});
        serde_json::from_value(serde_json::json!({
            "species_key": 3, "scientific_name": "Toy", "status": "succeeded", "error": null,
            "evidence": {"level": "indicative", "reasons": []},
            "data": {"presences": 30, "background": 100, "records_fetched": 30, "exclusions": [], "source": "t", "query_url": null},
            "variables_used": [], "variables_dropped": [],
            "selected": {"feature_classes": "LQ", "beta_multiplier": 1.0},
            "tuning": [], "validation": {"method": "random", "folds": [], "cbi_mean": null, "or10_mean": 0.1, "ormtp_mean": 0.0, "auc_mean": 0.7},
            "thresholds": [t("p10", 0.5), t("max_sss", 0.6), t("mtp", 0.2)],
            "study_area_km2": 1.0, "extrapolation": {"novel_fraction": 0.0},
            "uncertainty": {"mean_fold_sd": 0.0}, "contributions": [], "bounds": [0.0, 0.0, 1.0, 1.0],
            "files": [], "warnings": []
        }))
        .expect("result")
    }

    #[test]
    fn toy_grid_areas_are_exact() {
        // 2 × 2 cells of 0.5°; suitability 0.9 (NW), 0.3 (NE), 0.55 (SW), NaN (SE).
        let spec = GridSpec {
            west: 0.0,
            north: 46.0,
            cell: 0.5,
            width: 2,
            height: 2,
        };
        let layers = SpeciesLayers {
            result: result(),
            suitability: Raster {
                spec,
                data: vec![0.9, 0.3, 0.55, f32::NAN],
            },
            mess: Some(Raster {
                spec,
                data: vec![1.0, -1.0, 1.0, f32::NAN],
            }),
            uncertainty: None,
            aoa: None,
            effort: None,
        };
        // Western half of the NW cell, the whole NE cell, and a quarter of the SE cell (no data).
        let geometry = serde_json::json!({"type": "MultiPolygon", "coordinates": [
            [[[0.0, 45.5], [0.25, 45.5], [0.25, 46.0], [0.0, 46.0], [0.0, 45.5]]],
            [[[0.5, 45.5], [1.0, 45.5], [1.0, 46.0], [0.5, 46.0], [0.5, 45.5]]],
            [[[0.5, 45.25], [0.75, 45.25], [0.75, 45.5], [0.5, 45.5], [0.5, 45.25]]]
        ]});
        let polys = polygons("Toy", &geometry).expect("polygons");
        let r = result();
        let (f, w) = compute("Toy", &geometry, &polys, &[(r, Some(layers))]).expect("footprint");
        let nw_half = quad_area_km2(45.5, 46.0, 0.25);
        let ne = quad_area_km2(45.5, 46.0, 0.5);
        let se_quarter = quad_area_km2(45.25, 45.5, 0.25);
        let tol = 1e-9;
        assert!((f.area_km2 - (nw_half + ne + se_quarter)).abs() < tol);
        assert!((f.in_study_km2 - (nw_half + ne)).abs() < tol);
        assert_eq!(f.cells, 2);
        assert!(w.iter().any(|w| w.code == "footprint_outside_study_area"));
        let sp = &f.species[0];
        let by = |rule: &str| sp.thresholds.iter().find(|t| t.rule == rule).expect("rule");
        // P10 = 0.5: only the NW cell (0.9) is suitable inside; study-wide NW + SW.
        let p10 = by("p10");
        assert!((p10.suitable_km2 - nw_half).abs() < tol);
        assert!((p10.share_of_footprint - nw_half / (nw_half + ne)).abs() < 1e-12);
        let whole = quad_area_km2(45.5, 46.0, 0.5) + quad_area_km2(45.0, 45.5, 0.5);
        assert!((p10.species_suitable_km2 - whole).abs() < tol);
        // MTP = 0.2: both footprint cells.
        assert!((by("mtp").suitable_km2 - (nw_half + ne)).abs() < tol);
        assert_eq!(sp.thresholds[0].rule, "p10");
        let mean = (nw_half * 0.9 + ne * f64::from(0.3f32)) / (nw_half + ne);
        assert!((sp.mean_suitability.expect("mean") - mean).abs() < 1e-7);
        assert!((sp.extrapolated_share.expect("mess") - ne / (nw_half + ne)).abs() < 1e-12);
        assert!(sp.outside_aoa_share.is_none());
    }
}
