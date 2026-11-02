//! Model explanations reported by MaxEnt 3.4.4 next to the model itself: permutation
//! importance (`density/PermutationImportance.java`) and marginal response curves
//! (`Runner.createProfiles` / `Runner.responsePlotData`).
//!
//! Both are computed from a trained model and the SWD data it was trained on; neither changes
//! the model. They follow Java's arithmetic and random stream so that they match `maxent.jar`
//! (golden tests in `crates/niche-cli/tests/golden_explain.rs`).

use crate::javarandom::JavaRandom;
use crate::javasort::sort_indices;
use crate::model::{CompiledModel, MaxentModel, OutputFormat};
use crate::train::{SwdData, VarKind};

/// Seed of the generator in `PermutationImportance` (`new Random(11111)`).
const PERMUTATION_SEED: i64 = 11111;
/// Points per continuous response curve (`responsePlotData`: `num = 501`).
const CURVE_POINTS: usize = 501;

/// One marginal response curve: prediction as `variable` varies, every other variable held at
/// its average over the presence samples (the mode for categorical variables).
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseCurve {
    pub variable: String,
    pub categorical: bool,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// Variable indices of `data` in MaxEnt's layer order (Java `String` order, as in `train`).
fn maxent_order(data: &SwdData) -> Vec<usize> {
    let mut order: Vec<usize> = (0..data.vars.len()).collect();
    order.sort_by(|&a, &b| data.vars[a].name.cmp(&data.vars[b].name));
    order
}

fn compile(model: &MaxentModel, data: &SwdData) -> Result<CompiledModel, String> {
    let names: Vec<String> = data.vars.iter().map(|v| v.name.clone()).collect();
    model.compile(&names, true)
}

/// AUC of presences against background, ties counted half (`density.tools.Stats.auc`).
fn auc(pres: &[f64], bg: &[f64]) -> f64 {
    let mut p = pres.to_vec();
    let mut b = bg.to_vec();
    p.sort_by(f64::total_cmp);
    b.sort_by(f64::total_cmp);
    let mut sum: u64 = 0;
    let (mut i, mut j) = (0, 0);
    while i < p.len() {
        while j < b.len() && b[j] < p[i] {
            j += 1;
        }
        let less = j as u64;
        let mut jcnt = 0u64;
        while j < b.len() && b[j] == p[i] {
            j += 1;
            jcnt += 1;
        }
        let mut icnt = 1u64;
        while i + 1 < p.len() && p[i + 1] == p[i] {
            i += 1;
            icnt += 1;
        }
        sum += (2 * less + jcnt) * icnt;
        i += 1;
    }
    sum as f64 / (2.0 * p.len() as f64 * b.len() as f64)
}

/// Permutation importance per variable, in percent (summing to 100 unless no permutation
/// lowers the training AUC, then all 0), in the order of `data.vars`.
///
/// For each variable in turn, its values over the pooled presence + background rows are
/// shuffled, the model is re-evaluated (cloglog, clamped) and the drop in training AUC is
/// recorded; negative drops count as 0 and drops are normalised to percentages.
pub fn permutation_importance(
    model: &MaxentModel,
    data: &SwdData,
) -> Result<Vec<(String, f64)>, String> {
    let cm = compile(model, data)?;
    let nv = data.vars.len();
    let np = data.samples.first().map_or(0, Vec::len);
    let nb = data.background.first().map_or(0, Vec::len);
    if np == 0 || nb == 0 {
        return Err("permutation importance needs presence and background rows".into());
    }
    // Rows: presences first, then background (`allvals` in Java); columns in `data.vars` order.
    let rows: Vec<Vec<f64>> = (0..np)
        .map(|r| data.samples.iter().map(|c| c[r]).collect())
        .chain((0..nb).map(|r| data.background.iter().map(|c| c[r]).collect()))
        .collect();
    let auc_with = |col: Option<(usize, &[usize])>| -> f64 {
        let mut buf = vec![0.0; nv];
        let pred: Vec<f64> = rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                buf.copy_from_slice(row);
                if let Some((v, perm)) = col {
                    buf[v] = rows[perm[i]][v];
                }
                cm.predict(&buf, OutputFormat::Cloglog)
            })
            .collect();
        auc(&pred[..np], &pred[np..])
    };
    let original = auc_with(None);
    let mut rng = JavaRandom::new(PERMUTATION_SEED);
    let mut drops = vec![0.0; nv];
    for v in maxent_order(data) {
        let x: Vec<f64> = (0..np + nb).map(|_| rng.next_double()).collect();
        let perm = sort_indices(&x);
        drops[v] = (original - auc_with(Some((v, &perm)))).max(0.0);
    }
    let total: f64 = maxent_order(data).iter().map(|&v| drops[v]).sum();
    let scale = if total <= 0.0 { 0.0 } else { 100.0 / total };
    Ok(data
        .vars
        .iter()
        .zip(drops)
        .map(|(var, d)| (var.name.clone(), d * scale))
        .collect())
}

/// Marginal response curves for every variable, in MaxEnt's layer order.
///
/// `add_samples_to_background` must be the training setting: the plotted range is that of the
/// background used in training (with the presences when they were added), widened by a tenth
/// of its width on each side.
pub fn response_curves(
    model: &MaxentModel,
    data: &SwdData,
    add_samples_to_background: bool,
    format: OutputFormat,
) -> Result<Vec<ResponseCurve>, String> {
    let cm = compile(model, data)?;
    let np = data.samples.first().map_or(0, Vec::len);
    // Values over the training background (`baseFeaturesWithSamples`).
    let pool = |v: usize| -> Vec<f64> {
        let mut vals = data.background[v].clone();
        if add_samples_to_background {
            vals.extend_from_slice(&data.samples[v]);
        }
        vals
    };
    let categorical = |v: usize| data.vars[v].kind == VarKind::Categorical;
    let categories = |v: usize| -> Vec<f64> {
        let mut c = pool(v);
        c.sort_by(f64::total_cmp);
        c.dedup();
        c
    };
    // `sampleAverages`: mean over presences, or their most frequent category (first in sorted
    // order on ties); 0 without presences.
    let averages: Vec<f64> = (0..data.vars.len())
        .map(|v| {
            if np == 0 {
                return 0.0;
            }
            if categorical(v) {
                let cats = categories(v);
                let mut cnt = vec![0usize; cats.len()];
                for s in &data.samples[v] {
                    if let Ok(k) = cats.binary_search_by(|c| c.total_cmp(s)) {
                        cnt[k] += 1;
                    }
                }
                let (mut max, mut maxj) = (0, 0);
                for (j, &c) in cnt.iter().enumerate() {
                    if c > max {
                        max = c;
                        maxj = j;
                    }
                }
                cats.get(maxj).copied().unwrap_or(0.0)
            } else {
                let mut sum = 0.0;
                for s in &data.samples[v] {
                    sum += s;
                }
                sum / np as f64
            }
        })
        .collect();
    Ok(maxent_order(data)
        .into_iter()
        .map(|v| {
            let x: Vec<f64> = if categorical(v) {
                categories(v)
            } else {
                let vals = pool(v);
                let (mut min, mut max) = (0.0, 0.0);
                for (j, &e) in vals.iter().enumerate() {
                    if j == 0 || e < min {
                        min = e;
                    }
                    if j == 0 || e > max {
                        max = e;
                    }
                }
                let (mut minx, mut maxx) = (min - (max - min) / 10.0, max + (max - min) / 10.0);
                if min == max {
                    minx = min - 0.1;
                    maxx = max + 0.1;
                }
                (0..CURVE_POINTS)
                    .map(|j| minx + j as f64 * (maxx - minx) / (CURVE_POINTS - 1) as f64)
                    .collect()
            };
            let mut row = averages.clone();
            let y = x
                .iter()
                .map(|&xv| {
                    row[v] = xv;
                    cm.predict(&row, format)
                })
                .collect();
            ResponseCurve {
                variable: data.vars[v].name.clone(),
                categorical: categorical(v),
                x,
                y,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auc_counts_ties_half() {
        assert_eq!(auc(&[2.0, 3.0], &[0.0, 1.0]), 1.0);
        assert_eq!(auc(&[1.0], &[1.0]), 0.5);
        assert_eq!(auc(&[1.0, 1.0], &[0.0, 1.0, 2.0]), 0.5);
    }
}
