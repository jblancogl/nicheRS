//! Sampling effort of a target group on the modelling grid, and the sampling-bias diagnostic
//! (ADR-0013, Science v1 phase 2).
//!
//! The effort surface counts, for each modelling cell, the GBIF records of a target group
//! (a GBIF class, e.g. `Aves`) whose 30-arc-second effort-grid cell centre falls in it. It is
//! used three ways when a study asks for a target-group background:
//! - background cells are drawn among the cells with at least one record
//!   ([`crate::env::sample_background_target_group`]; Phillips et al. 2009);
//! - null models draw their presences with probability proportional to the counts (bias-matched
//!   nulls, Bohl et al. 2019; ADR-0014 hook);
//! - [`diagnostic`] says how concentrated presences are in the most-surveyed cells.

use crate::demo;
use crate::dto::SamplingBias;
use crate::env::EnvStack;

/// Fewest eligible cells (valid, with records) for a target-group background; below it the
/// study falls back to the random background with a warning. Lowered to the requested
/// background size or the number of valid cells when those are smaller.
pub const MIN_TARGET_GROUP_CELLS: usize = 1000;

/// Name of the synthetic demo's target group (its only effort grid).
pub const DEMO_CLASS: &str = "Synthetica (demo observers, SYNTHETIC)";

/// Records of a target group per modelling cell.
#[derive(Debug, Clone)]
pub struct EffortSurface {
    pub class: String,
    /// Per modelling cell: records (≥ 0) in valid cells, 0 elsewhere.
    pub counts: Vec<f64>,
    pub synthetic: bool,
}

impl EffortSurface {
    /// Sum `(lng, lat, records)` points (effort-grid cell centres) into the valid cells of the
    /// modelling grid. Points outside the grid or in cells without predictor data are ignored.
    pub fn from_points(
        stack: &EnvStack,
        class: &str,
        points: impl IntoIterator<Item = (f64, f64, u32)>,
        synthetic: bool,
    ) -> Self {
        let spec = stack.spec;
        let mut counts = vec![0.0; spec.len()];
        for (lng, lat, n) in points {
            if let Some((r, c)) = spec.cell_of(lng, lat) {
                let i = spec.index(r, c);
                if stack.valid[i] {
                    counts[i] += f64::from(n);
                }
            }
        }
        Self {
            class: class.to_string(),
            counts,
            synthetic,
        }
    }

    /// Valid cells with at least one record.
    pub fn eligible_cells(&self, stack: &EnvStack) -> usize {
        self.counts
            .iter()
            .zip(&stack.valid)
            .filter(|(c, v)| **v && **c > 0.0)
            .count()
    }

    /// Raster for `effort.tif`: records per cell, NaN outside the valid cells.
    pub fn raster_data(&self, stack: &EnvStack) -> Vec<f32> {
        self.counts
            .iter()
            .zip(&stack.valid)
            .map(|(c, v)| if *v { *c as f32 } else { f32::NAN })
            .collect()
    }
}

/// Eligible cells needed for a target-group background of `background_points` in a study with
/// `valid_cells` cells.
pub fn min_eligible_cells(background_points: usize, valid_cells: usize) -> usize {
    MIN_TARGET_GROUP_CELLS
        .min(background_points)
        .min(valid_cells)
        .max(1)
}

/// SYNTHETIC effort of the demo: records concentrated around three "towns" and along a
/// "valley road", plus a sparse deterministic scatter, on the demo's 30″ lattice. Not real data;
/// it only lets the demo exercise the target-group path.
pub fn demo_effort(stack: &EnvStack) -> EffortSurface {
    let g = demo::REGION;
    let towns = [(4.85, 45.6, 30.0), (5.7, 45.15, 25.0), (4.8, 44.1, 20.0)];
    let points = (0..g.height).flat_map(move |row| {
        (0..g.width).map(move |col| {
            let (x, y) = g.center(row, col);
            let mut e = 0.0;
            for (tx, ty, peak) in towns {
                let d2 = (x - tx).powi(2) + (y - ty).powi(2);
                e += peak * (-d2 / (2.0 * 0.15f64.powi(2))).exp();
            }
            e += 4.0 * (-(x - 4.8).powi(2) / (2.0 * 0.05f64.powi(2))).exp();
            // Sparse scatter: one record in about 1 cell of 40 (fixed hash of the cell).
            let h = (row as u64)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add((col as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
            if (h >> 33).is_multiple_of(40) {
                e += 1.0;
            }
            (x, y, e.floor() as u32)
        })
    });
    EffortSurface::from_points(
        stack,
        DEMO_CLASS,
        points.filter(|p| p.2 > 0).collect::<Vec<_>>(),
        true,
    )
}

/// How concentrated the presences are in the most-surveyed cells: the share of presence cells
/// inside the cells with most records making up about 10 % of the study area, against the share
/// of the area those cells cover. Descriptive only (no effect on the evidence level).
pub fn diagnostic(stack: &EnvStack, effort: &EffortSurface, presences: &[usize]) -> SamplingBias {
    let valid: Vec<usize> = (0..stack.valid.len()).filter(|&i| stack.valid[i]).collect();
    let n = valid.len().max(1);
    let mut values: Vec<f64> = valid.iter().map(|&i| effort.counts[i]).collect();
    let records: f64 = values.iter().sum();
    let with_effort = values.iter().filter(|v| **v > 0.0).count();
    let max = values.iter().copied().fold(0.0, f64::max);
    values.sort_by(|a, b| b.total_cmp(a));
    // Cut at the cell ranked ⌈10 %⌉; ties at the cut are all included; zero-effort cells never.
    let m = n.div_ceil(10);
    let cut = values.get(m - 1).copied().unwrap_or(0.0);
    let in_top = |c: usize| {
        let e = effort.counts[c];
        e > 0.0 && e >= cut
    };
    let top_cells = valid.iter().filter(|&&c| in_top(c)).count();
    let pres_top = presences.iter().filter(|&&c| in_top(c)).count();
    SamplingBias {
        target_group: effort.class.clone(),
        records: records as u64,
        cells_with_effort: with_effort,
        share_of_area_with_effort: with_effort as f64 / n as f64,
        max_cell_records: max,
        presences_in_top10pct_effort_cells: if presences.is_empty() {
            0.0
        } else {
            pres_top as f64 / presences.len() as f64
        },
        expected: top_cells as f64 / n as f64,
        synthetic: effort.synthetic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use niche_geo::GridSpec;

    fn stack(width: usize, height: usize) -> EnvStack {
        let spec = GridSpec {
            west: 0.0,
            north: 10.0,
            cell: 1.0,
            width,
            height,
        };
        EnvStack {
            spec,
            area_mask: vec![true; spec.len()],
            land_mask: vec![true; spec.len()],
            valid: vec![true; spec.len()],
            layers: vec![],
            row_area_km2: vec![1.0; height],
            provenance: serde_json::Value::Null,
        }
    }

    #[test]
    fn effort_points_are_summed_per_modelling_cell() {
        let mut s = stack(10, 10);
        s.valid[0] = false;
        // Two effort-grid cells in modelling cell (row 1, col 2), one in the invalid cell 0,
        // one outside the grid.
        let e = EffortSurface::from_points(
            &s,
            "Aves",
            [(2.2, 8.7, 3), (2.8, 8.1, 4), (0.5, 9.5, 9), (50.0, 50.0, 1)],
            false,
        );
        assert_eq!(e.counts[s.spec.index(1, 2)], 7.0);
        assert_eq!(e.counts[0], 0.0);
        assert_eq!(e.counts.iter().sum::<f64>(), 7.0);
        assert_eq!(e.eligible_cells(&s), 1);
        let r = e.raster_data(&s);
        assert!(r[0].is_nan());
        assert_eq!(r[s.spec.index(1, 2)], 7.0);
        assert_eq!(r[5], 0.0);
    }

    #[test]
    fn diagnostic_counts_presences_in_the_most_surveyed_tenth() {
        let s = stack(10, 10);
        // Effort: 10 cells with 100 records (the top 10 %), 40 cells with 1, 50 with none.
        let mut counts = vec![0.0; 100];
        for c in counts.iter_mut().take(10) {
            *c = 100.0;
        }
        for c in counts.iter_mut().skip(10).take(40) {
            *c = 1.0;
        }
        let e = EffortSurface {
            class: "Aves".into(),
            counts,
            synthetic: false,
        };
        // 6 of 10 presences in the top cells.
        let pres = [0, 1, 2, 3, 4, 5, 20, 30, 60, 90];
        let d = diagnostic(&s, &e, &pres);
        assert_eq!(d.records, 1040);
        assert_eq!(d.cells_with_effort, 50);
        assert!((d.share_of_area_with_effort - 0.5).abs() < 1e-12);
        assert!((d.expected - 0.10).abs() < 1e-12);
        assert!((d.presences_in_top10pct_effort_cells - 0.6).abs() < 1e-12);
        assert_eq!(d.max_cell_records, 100.0);
    }

    #[test]
    fn ties_at_the_cut_are_included_and_zero_effort_never() {
        let s = stack(10, 10);
        // 30 cells tie with 1 record: all count as "top"; the rest have none.
        let counts: Vec<f64> = (0..100).map(|i| if i < 30 { 1.0 } else { 0.0 }).collect();
        let e = EffortSurface {
            class: "Aves".into(),
            counts,
            synthetic: false,
        };
        let d = diagnostic(&s, &e, &[0, 50]);
        assert!((d.expected - 0.30).abs() < 1e-12);
        assert!((d.presences_in_top10pct_effort_cells - 0.5).abs() < 1e-12);
        // Fewer than 10 % of cells with records: only those.
        let counts: Vec<f64> = (0..100).map(|i| if i < 4 { 2.0 } else { 0.0 }).collect();
        let e = EffortSurface {
            class: "Aves".into(),
            counts,
            synthetic: false,
        };
        assert!((diagnostic(&s, &e, &[0]).expected - 0.04).abs() < 1e-12);
    }

    #[test]
    fn minimum_eligible_cells_follows_the_study_size() {
        assert_eq!(min_eligible_cells(10_000, 1_000_000), 1000);
        assert_eq!(min_eligible_cells(500, 1_000_000), 500);
        assert_eq!(min_eligible_cells(10_000, 300), 300);
    }
}
