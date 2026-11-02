//! Decision-support analyses of finished studies (ADR-0018): survey priority, project-footprint
//! overlay, multi-species aggregation and comparison of two studies.
//!
//! Every analysis is computed on demand from the study's stored outputs (GeoTIFFs and
//! `result.json`); nothing is refitted and the study's files and manifest are never changed.
//! Each analysis lives in `jobs/{id}/analyses/{aid}/` with its own small manifest that records
//! the checksum of the study manifest and of every input it read
//! (`docs/architecture/execution-manifest.md`). Deleting the study deletes its analyses.
//!
//! This module is pure computation on in-memory rasters; reading and writing files is in
//! `engine/analyses.rs`.

pub mod comparison;
pub mod footprint;
pub mod priority;
pub mod report;
pub mod richness;

use niche_geo::Raster;
use niche_geo::geotiff::GeoTiffInfo;

use crate::AppError;
use crate::dto::{SpeciesResult, ThresholdArea};

/// Format of the manifest addendum of an analysis.
pub const MANIFEST_FORMAT: &str = "nichers-analysis-manifest/1";

/// Largest number of candidate areas listed by the survey priority.
pub const MAX_TOP_N: usize = 100;
pub const DEFAULT_TOP_N: usize = 20;
/// Footprints kept per study (each holds a small set of files).
pub const MAX_FOOTPRINTS: usize = 50;
/// Share of the study-area cells that make up the multi-species hotspots.
pub const HOTSPOT_SHARE: f64 = 0.10;

/// Mandatory disclaimer of every decision-support output (ADR-0018).
pub const DISCLAIMER: &str = "These outputs are derived from habitat suitability models: they show relative suitability of environmental conditions, not the presence or absence of a species. They do not replace a field inventory or an impact assessment; low suitability is not evidence of absence.";

/// One species output of a study, as read from its files.
pub struct SpeciesLayers {
    pub result: SpeciesResult,
    pub suitability: Raster,
    pub mess: Option<Raster>,
    pub uncertainty: Option<Raster>,
    pub aoa: Option<Raster>,
    pub effort: Option<Raster>,
}

/// A GeoTIFF from memory.
pub fn read_tif(bytes: Vec<u8>) -> Result<Raster, AppError> {
    let info = GeoTiffInfo::read(&bytes).map_err(|e| AppError::Storage(e.to_string()))?;
    info.read_window(&bytes, 0, 0, info.height, info.width)
        .map_err(|e| AppError::Storage(e.to_string()))
}

/// Threshold of a rule (`p10`, `mtp`, `max_sss`) in a species result.
pub fn threshold(r: &SpeciesResult, rule: &str) -> Option<f64> {
    r.thresholds
        .iter()
        .find(|t| t.rule == rule)
        .map(|t| t.threshold)
}

/// Threshold rules in report order: P10 (headline, ADR-0009 §5), then the band MTP / max SSS.
pub fn rules_in_order(r: &SpeciesResult) -> Vec<&ThresholdArea> {
    ["p10", "mtp", "max_sss"]
        .iter()
        .filter_map(|rule| r.thresholds.iter().find(|t| t.rule == *rule))
        .collect()
}

/// Exact ellipsoidal area (km²) of every cell of a grid, row-major.
pub fn cell_areas(spec: &niche_geo::GridSpec) -> Vec<f64> {
    let rows = niche_geo::area::row_areas_km2(spec);
    let mut out = Vec::with_capacity(spec.len());
    for a in rows {
        out.extend(std::iter::repeat_n(a, spec.width));
    }
    out
}

/// Two grids are the same lattice (origin and cell size within a millionth of a cell).
pub fn same_grid(a: &niche_geo::GridSpec, b: &niche_geo::GridSpec) -> bool {
    let tol = a.cell * 1e-6;
    a.width == b.width
        && a.height == b.height
        && (a.cell - b.cell).abs() <= tol
        && (a.west - b.west).abs() <= tol
        && (a.north - b.north).abs() <= tol
}

/// Mid-rank percentiles in (0, 1) of `values` (ties share the mean of their ranks):
/// `(rank − 0.5) / n` with 1-based ranks.
pub fn mid_rank_percentiles(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]).then(a.cmp(&b)));
    let mut out = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n && values[order[j]] == values[order[i]] {
            j += 1;
        }
        // 1-based ranks i+1 ..= j share their mean.
        let mean_rank = (i + 1 + j) as f64 / 2.0;
        for &k in &order[i..j] {
            out[k] = (mean_rank - 0.5) / n as f64;
        }
        i = j;
    }
    out
}

/// Median of finite values (`None` when empty).
pub fn median(values: &[f64]) -> Option<f64> {
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let m = v.len() / 2;
    Some(if v.len() % 2 == 1 {
        v[m]
    } else {
        (v[m - 1] + v[m]) / 2.0
    })
}

/// CSV field: quoted when it contains a separator, a quote or a line break.
pub fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Number formatting for CSV (fixed, enough digits for areas and indices).
pub fn num(x: f64) -> String {
    if x.is_finite() {
        let s = format!("{x:.6}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        if s.is_empty() || s == "-0" {
            "0".into()
        } else {
            s.to_string()
        }
    } else {
        String::new()
    }
}

pub fn opt_num(x: Option<f64>) -> String {
    x.map(num).unwrap_or_default()
}

/// Colour of a survey-priority class code (1–4).
pub fn class_colour(v: f32) -> Option<[u8; 4]> {
    let i = (v.round() as i64 - 1).clamp(0, 3) as usize;
    let c = niche_geo::colormap::PRIORITY_CLASSES[i];
    Some([c[0], c[1], c[2], 255])
}

/// Colouring of an analysis layer for map tiles (ranges from the analysis' layer list).
pub type TileStyle = Box<dyn Fn(f32) -> Option<[u8; 4]> + Send + Sync>;

pub fn tile_colour(layer: &str, a: &crate::dto::Analysis) -> Option<TileStyle> {
    let range = a.layers.iter().find(|l| l.id == layer)?;
    Some(match layer {
        "priority" => Box::new(|v: f32| {
            let c = niche_geo::colormap::priority(v);
            Some([c[0], c[1], c[2], 220])
        }),
        "priority_class" => Box::new(|v: f32| class_colour(v).map(|c| [c[0], c[1], c[2], 220])),
        "richness" | "suitability_sum" | "contributing" => {
            let max = range.max.max(1e-6) as f32;
            Box::new(move |v: f32| {
                let c = niche_geo::colormap::viridis(v / max);
                Some([c[0], c[1], c[2], 230])
            })
        }
        "hotspot" => Box::new(|v: f32| {
            (v >= 1.0).then(|| {
                let c = niche_geo::colormap::HOTSPOT;
                [c[0], c[1], c[2], 230]
            })
        }),
        "difference" => {
            let m = range.min.abs().max(range.max.abs()).max(1e-6) as f32;
            Box::new(move |v: f32| {
                let c = niche_geo::colormap::diverging(v / m);
                Some([c[0], c[1], c[2], 230])
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mid_ranks_share_ties() {
        let p = mid_rank_percentiles(&[0.0, 0.0, 1.0, 2.0]);
        assert_eq!(p, vec![0.25, 0.25, 0.625, 0.875]);
        let q = mid_rank_percentiles(&[5.0]);
        assert_eq!(q, vec![0.5]);
    }

    #[test]
    fn medians() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), Some(2.5));
        assert_eq!(median(&[]), None);
    }

    #[test]
    fn csv_quoting() {
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_field("ab"), "ab");
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(2.0), "2");
        assert_eq!(num(-0.0000001), "0");
    }
}
