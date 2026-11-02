//! Survey priority (ADR-0018 §1): where surveys are most worth doing for one species.
//!
//! Within the study area (cells with a prediction):
//!
//! - `s` = mid-rank percentile of the cell's relative suitability;
//! - with the target-group effort surface (ADR-0013), `e` = mid-rank percentile of the cell's
//!   target-group records and `priority = s · (1 − e)`; "little sampled" = `e < 0.5`;
//! - without it, the proxy is the distance to the nearest known record of the species
//!   (retained, thinned and held-out presences): `d` = mid-rank percentile of that distance,
//!   `priority = s · d`; "little sampled" (far from known records) = `d > 0.5`.
//!
//! Classes: suitability at or above the species' P10 threshold ("higher") or below ("lower"),
//! crossed with little / well sampled: 1 prospect first (higher, little sampled), 2 known (higher,
//! well sampled), 3 lower and little sampled, 4 lower and well sampled. Candidate areas are the
//! 8-connected groups of class-1 cells among the 10 % of study-area cells with the highest
//! priority score, ranked by their area-weighted mean priority.
//! Following Guisan et al. (2006) and Le Lay et al. (2010), model-guided sampling targets
//! cells of higher predicted suitability; crossing with effort points to those not yet surveyed.

use niche_geo::{GridSpec, Raster};

use super::{SpeciesLayers, cell_areas, median, mid_rank_percentiles, threshold};
use crate::AppError;
use crate::dto::{PriorityCandidate, PriorityClass, PriorityResult, Severity, Warning};

pub struct PriorityOutput {
    pub result: PriorityResult,
    /// Priority score 0–1, NaN outside the study area.
    pub score: Raster,
    /// Class codes 1–4, NaN outside the study area.
    pub classes: Raster,
    pub warnings: Vec<Warning>,
}

pub const CLASS_IDS: [&str; 4] = ["prospect_first", "known", "low_unsampled", "low_sampled"];
/// Share of the study-area cells, by priority score, among which candidate areas are drawn.
pub const CANDIDATE_SHARE: f64 = 0.10;

/// `known`: (lng, lat) of the species' known records, used when there is no effort surface.
pub fn compute(
    sp: &SpeciesLayers,
    known: &[(f64, f64)],
    top_n: usize,
) -> Result<PriorityOutput, AppError> {
    let r = &sp.result;
    let spec = sp.suitability.spec;
    let suit = &sp.suitability.data;
    let cells: Vec<usize> = (0..spec.len()).filter(|&i| suit[i].is_finite()).collect();
    if cells.is_empty() {
        return Err(AppError::Invalid(
            "this species has no prediction in the study area".into(),
        ));
    }
    let p10 = threshold(r, "p10")
        .ok_or_else(|| AppError::Invalid("this species has no P10 threshold".into()))?;
    let areas = cell_areas(&spec);
    let s_vals: Vec<f64> = cells.iter().map(|&i| f64::from(suit[i])).collect();
    let s_pct = mid_rank_percentiles(&s_vals);

    let effort = sp
        .effort
        .as_ref()
        .filter(|e| super::same_grid(&e.spec, &spec))
        .map(|e| {
            cells
                .iter()
                .map(|&i| {
                    let v = f64::from(e.data[i]);
                    if v.is_finite() { v.max(0.0) } else { 0.0 }
                })
                .collect::<Vec<f64>>()
        })
        .filter(|v| v.iter().any(|x| *x > 0.0));
    let mut warnings = Vec::new();
    // Per cell (in `cells` order): sampling percentile (high = well sampled) and the raw value.
    let (method, sampled_pct, raw, split, known_records) = if let Some(e) = effort {
        let pct = mid_rank_percentiles(&e);
        let split = median(&e).unwrap_or(0.0);
        ("effort", pct, e, split, None)
    } else {
        let mut mask = vec![false; spec.len()];
        let mut n = 0usize;
        for &(x, y) in known {
            if let Some((row, col)) = spec.cell_of(x, y) {
                mask[spec.index(row, col)] = true;
                n += 1;
            }
        }
        if n == 0 {
            return Err(AppError::Invalid(
                "no effort surface and no known record of the species in the study area: survey priority cannot be computed".into(),
            ));
        }
        let dist = niche_geo::distance::distance_to_mask_km(&spec, &mask);
        let d: Vec<f64> = cells.iter().map(|&i| f64::from(dist[i])).collect();
        let pct = mid_rank_percentiles(&d);
        let split = median(&d).unwrap_or(0.0);
        // Far from records = little sampled: the sampling percentile is 1 − distance percentile.
        let sampled: Vec<f64> = pct.iter().map(|p| 1.0 - p).collect();
        warnings.push(Warning::new(
            "priority_distance_proxy",
            Severity::Warning,
            "No sampling-effort surface for this study (target-group background not used): survey priority uses the distance to the nearest known record of the species as a proxy. Distance to the species' own records is not sampling effort: areas far from records may have been surveyed without finding it.",
        ));
        ("distance", sampled, d, split, Some(n))
    };

    let mut score = Raster::new(spec, f32::NAN);
    let mut classes = Raster::new(spec, f32::NAN);
    let mut class_cells = [0usize; 4];
    let mut class_area = [0.0f64; 4];
    let mut study_km2 = 0.0;
    for (k, &i) in cells.iter().enumerate() {
        let little = sampled_pct[k] < 0.5;
        let high = s_vals[k] >= p10;
        score.data[i] = (s_pct[k] * (1.0 - sampled_pct[k])) as f32;
        let code = match (high, little) {
            (true, true) => 1,
            (true, false) => 2,
            (false, true) => 3,
            (false, false) => 4,
        };
        classes.data[i] = code as f32;
        class_cells[code - 1] += 1;
        class_area[code - 1] += areas[i];
        study_km2 += areas[i];
    }
    let class_rows: Vec<PriorityClass> = (0..4)
        .map(|c| PriorityClass {
            code: c as u8 + 1,
            id: CLASS_IDS[c].into(),
            cells: class_cells[c],
            area_km2: class_area[c],
            fraction_of_study_area: if study_km2 > 0.0 {
                class_area[c] / study_km2
            } else {
                0.0
            },
        })
        .collect();

    // Raw sampling value per grid cell (for candidate summaries).
    let mut raw_grid = vec![f64::NAN; spec.len()];
    for (k, &i) in cells.iter().enumerate() {
        raw_grid[i] = raw[k];
    }
    // Candidate cells: "prospect first" cells among the top 10 % of the study area by priority
    // score, so candidate areas stay focused even when the class covers much of the area.
    let mut sorted: Vec<f32> = cells.iter().map(|&i| score.data[i]).collect();
    sorted.sort_by(|a, b| b.total_cmp(a));
    let k = ((cells.len() as f64 * CANDIDATE_SHARE).ceil() as usize).clamp(1, cells.len());
    let cut = sorted[k - 1];
    let mask: Vec<bool> = (0..spec.len())
        .map(|i| classes.data[i] == 1.0 && score.data[i] >= cut)
        .collect();
    let mut groups = groups_of(&mask, &spec);
    let mut cands: Vec<PriorityCandidate> = groups
        .iter_mut()
        .map(|g| {
            summarise(
                g,
                &spec,
                &areas,
                &score,
                suit,
                sp.mess.as_ref(),
                &raw_grid,
                method,
            )
        })
        .collect();
    cands.sort_by(|a, b| {
        b.mean_priority
            .total_cmp(&a.mean_priority)
            .then(b.area_km2.total_cmp(&a.area_km2))
    });
    let total_groups = cands.len();
    cands.truncate(top_n);
    for (k, c) in cands.iter_mut().enumerate() {
        c.rank = k + 1;
    }
    if total_groups == 0 {
        warnings.push(Warning::new(
            "priority_no_candidate",
            Severity::Info,
            "No cell combines suitability at or above the P10 threshold with little sampling among the highest priority scores: no candidate area to list.",
        ));
    }
    let bs = r.background_sampling.as_ref();
    let result = PriorityResult {
        species_key: r.species_key,
        scientific_name: r.scientific_name.clone(),
        evidence_level: r.evidence.level,
        method: method.into(),
        target_group: if method == "effort" {
            r.sampling_bias
                .as_ref()
                .map(|b| b.target_group.clone())
                .or_else(|| bs.and_then(|b| b.target_group.clone()))
        } else {
            None
        },
        effort_synthetic: method == "effort"
            && (r.sampling_bias.as_ref().is_some_and(|b| b.synthetic)
                || bs.is_some_and(|b| b.synthetic)),
        known_records,
        suitability_threshold: p10,
        sampling_split: split,
        study_area_km2: study_km2,
        classes: class_rows,
        candidates: cands,
        candidate_groups: total_groups,
        candidate_min_priority: f64::from(cut),
    };
    Ok(PriorityOutput {
        result,
        score,
        classes,
        warnings,
    })
}

/// 8-connected groups of the cells in `mask`, each as a sorted list of indices.
fn groups_of(mask: &[bool], spec: &GridSpec) -> Vec<Vec<usize>> {
    let mut seen = vec![false; spec.len()];
    let mut out = Vec::new();
    for start in 0..spec.len() {
        if seen[start] || !mask[start] {
            continue;
        }
        let mut group = Vec::new();
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(i) = stack.pop() {
            group.push(i);
            let (row, col) = (i / spec.width, i % spec.width);
            for dr in -1i64..=1 {
                for dc in -1i64..=1 {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let (nr, nc) = (row as i64 + dr, col as i64 + dc);
                    if nr < 0 || nc < 0 || nr >= spec.height as i64 || nc >= spec.width as i64 {
                        continue;
                    }
                    let j = spec.index(nr as usize, nc as usize);
                    if !seen[j] && mask[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        group.sort_unstable();
        out.push(group);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn summarise(
    group: &[usize],
    spec: &GridSpec,
    areas: &[f64],
    score: &Raster,
    suit: &[f32],
    mess: Option<&Raster>,
    raw: &[f64],
    method: &str,
) -> PriorityCandidate {
    let (mut a, mut sx, mut sy, mut sp, mut ss, mut smax, mut sraw, mut extra) =
        (0.0, 0.0, 0.0, 0.0, 0.0, f64::NEG_INFINITY, 0.0, 0.0);
    let (mut w, mut s, mut e, mut n) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for &i in group {
        let (row, col) = (i / spec.width, i % spec.width);
        let (x, y) = spec.center(row, col);
        let ai = areas[i];
        a += ai;
        sx += ai * x;
        sy += ai * y;
        sp += ai * f64::from(score.data[i]);
        let v = f64::from(suit[i]);
        ss += ai * v;
        smax = smax.max(v);
        sraw += ai * raw[i];
        if mess.is_some_and(|m| m.data[i] < 0.0) {
            extra += ai;
        }
        let half = spec.cell / 2.0;
        w = w.min(x - half);
        e = e.max(x + half);
        s = s.min(y - half);
        n = n.max(y + half);
    }
    let mean_raw = if a > 0.0 { sraw / a } else { 0.0 };
    PriorityCandidate {
        rank: 0,
        lng: sx / a,
        lat: sy / a,
        bbox: [w, s, e, n],
        cells: group.len(),
        area_km2: a,
        mean_priority: sp / a,
        mean_suitability: ss / a,
        max_suitability: smax,
        mean_effort: (method == "effort").then_some(mean_raw),
        mean_distance_km: (method == "distance").then_some(mean_raw),
        extrapolated_share: extra / a,
    }
}

/// `candidates.csv`.
pub fn candidates_csv(r: &PriorityResult) -> String {
    use super::{num, opt_num};
    let mut s = String::from(
        "rank,lng,lat,west,south,east,north,cells,area_km2,mean_priority,mean_suitability,max_suitability,mean_effort_records,mean_distance_km,extrapolated_share\n",
    );
    for c in &r.candidates {
        s.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            c.rank,
            num(c.lng),
            num(c.lat),
            num(c.bbox[0]),
            num(c.bbox[1]),
            num(c.bbox[2]),
            num(c.bbox[3]),
            c.cells,
            num(c.area_km2),
            num(c.mean_priority),
            num(c.mean_suitability),
            num(c.max_suitability),
            opt_num(c.mean_effort),
            opt_num(c.mean_distance_km),
            num(c.extrapolated_share),
        ));
    }
    s
}

/// `candidates.geojson`: one point per candidate area (centroid), with its bounding box.
pub fn candidates_geojson(r: &PriorityResult, synthetic: bool) -> serde_json::Value {
    let feats: Vec<serde_json::Value> = r
        .candidates
        .iter()
        .map(|c| {
            serde_json::json!({
                "type": "Feature",
                "bbox": c.bbox,
                "geometry": {"type": "Point", "coordinates": [c.lng, c.lat]},
                "properties": {
                    "rank": c.rank,
                    "cells": c.cells,
                    "area_km2": c.area_km2,
                    "mean_priority": c.mean_priority,
                    "mean_suitability": c.mean_suitability,
                    "max_suitability": c.max_suitability,
                    "mean_effort_records": c.mean_effort,
                    "mean_distance_km": c.mean_distance_km,
                    "extrapolated_share": c.extrapolated_share,
                }
            })
        })
        .collect();
    serde_json::json!({
        "type": "FeatureCollection",
        "name": format!("Areas worth surveying — {}", r.scientific_name),
        "description": format!(
            "Candidate areas worth surveying (higher relative suitability, little {}). Not where the species is. {}{}",
            if r.method == "effort" { "sampling effort" } else { "known records nearby" },
            super::DISCLAIMER,
            if synthetic { " SYNTHETIC DEMONSTRATION DATA." } else { "" }
        ),
        "features": feats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::*;

    fn result(p10: f64) -> SpeciesResult {
        serde_json::from_value(serde_json::json!({
            "species_key": 7, "scientific_name": "Testus", "status": "succeeded", "error": null,
            "evidence": {"level": "indicative", "reasons": []},
            "data": {"presences": 30, "background": 100, "records_fetched": 30, "exclusions": [], "source": "t", "query_url": null},
            "variables_used": [], "variables_dropped": [],
            "selected": {"feature_classes": "LQ", "beta_multiplier": 1.0},
            "tuning": [], "validation": {"method": "random", "folds": [], "cbi_mean": null, "or10_mean": 0.1, "ormtp_mean": 0.0, "auc_mean": 0.7},
            "thresholds": [{"rule": "p10", "label": "P10", "threshold": p10, "area_km2": 0.0, "fraction_of_study_area": 0.0}],
            "study_area_km2": 1.0, "extrapolation": {"novel_fraction": 0.0},
            "uncertainty": {"mean_fold_sd": 0.0}, "contributions": [], "bounds": [0.0, 0.0, 1.0, 1.0],
            "files": [], "warnings": []
        }))
        .expect("result")
    }

    fn grid() -> GridSpec {
        GridSpec {
            west: 0.0,
            north: 1.0,
            cell: 0.25,
            width: 4,
            height: 4,
        }
    }

    #[test]
    fn effort_classes_and_candidates() {
        let spec = grid();
        // Suitability rises eastwards; effort is high in the two northern rows.
        let suit: Vec<f32> = (0..16).map(|i| (i % 4) as f32 / 3.0).collect();
        let effort: Vec<f32> = (0..16).map(|i| if i < 8 { 10.0 } else { 0.0 }).collect();
        let sp = SpeciesLayers {
            result: result(0.5),
            suitability: Raster { spec, data: suit },
            mess: None,
            uncertainty: None,
            aoa: None,
            effort: Some(Raster { spec, data: effort }),
        };
        let out = compute(&sp, &[], 10).expect("priority");
        let r = &out.result;
        assert_eq!(r.method, "effort");
        // Prospect first: the two eastern columns of the two southern rows.
        assert_eq!(r.classes[0].cells, 4);
        assert_eq!(r.classes[1].cells, 4);
        // Candidates: the top 10 % of cells by score (2 of 16), both in the eastern column.
        assert_eq!(r.candidate_groups, 1);
        assert_eq!(r.candidates[0].cells, 2);
        assert!((r.candidates[0].lng - 0.875).abs() < 1e-12);
        let total: f64 = r.classes.iter().map(|c| c.area_km2).sum();
        assert!((total - r.study_area_km2).abs() < 1e-9);
        // Score: highest in the south-east corner.
        let best = out
            .score
            .data
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i);
        assert!(matches!(best, Some(11 | 15)));
    }

    #[test]
    fn distance_proxy_without_effort() {
        let spec = grid();
        let suit: Vec<f32> = vec![0.9; 16];
        let sp = SpeciesLayers {
            result: result(0.5),
            suitability: Raster { spec, data: suit },
            mess: None,
            uncertainty: None,
            aoa: None,
            effort: None,
        };
        // One known record in the north-west cell.
        let out = compute(&sp, &[(0.1, 0.9)], 5).expect("priority");
        let r = &out.result;
        assert_eq!(r.method, "distance");
        assert_eq!(r.known_records, Some(1));
        assert!(
            out.warnings
                .iter()
                .any(|w| w.code == "priority_distance_proxy")
        );
        // The far (south-east) cells are "prospect first", the record's cell is "known".
        assert_eq!(out.classes.data[15], 1.0);
        assert_eq!(out.classes.data[0], 2.0);
        assert!(compute(&sp, &[(9.0, 9.0)], 5).is_err());
    }
}
