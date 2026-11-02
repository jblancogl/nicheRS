//! Model explanation: permutation importance and partial dependence.

use niche_eval::rng::Rng;

use crate::{Forest, RfData, RfError};

/// Permutation importance of one variable.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableImportance {
    pub variable: String,
    /// Mean decrease of the presence–background AUC when the variable is permuted
    /// (can be slightly negative for irrelevant variables).
    pub auc_decrease: f64,
    /// Standard deviation of the decrease over the repeats (0 with one repeat).
    pub sd: f64,
    /// Positive mean decreases normalised to sum to 100 (all 0 if none is positive), the
    /// presentation MaxEnt uses for its permutation importance.
    pub percent: f64,
}

/// Permutation importance (Breiman 2001; Fisher, Rudin & Dominici 2019) as the mean decrease
/// in presence–background AUC, in the order of `data.variables`.
///
/// For each variable and repeat, its values are shuffled over the pooled presence +
/// background rows (breaking its link with the label and with the other predictors), the
/// forest is re-evaluated and the AUC drop recorded. AUC rather than accuracy because
/// suitability is used as a ranking (no threshold) and because AUC is the discrimination
/// measure used for MaxEnt's importance, so both engines are read the same way.
///
/// `data` is normally the training data (as for MaxEnt). With fully grown trees the training
/// AUC is near 1 because presences were in most bootstraps; the drop then measures how much
/// the fitted forest *relies* on a variable, not out-of-sample skill. Pass held-out data for
/// the latter. Background rows can be capped at `max_background` (deterministic subsample) to
/// bound cost: one forest evaluation of all rows per variable and repeat. Deterministic given
/// `seed`.
pub fn permutation_importance(
    forest: &Forest,
    data: &RfData,
    repeats: usize,
    max_background: Option<usize>,
    seed: u64,
) -> Result<Vec<VariableImportance>, RfError> {
    data.validate()?;
    let p = forest.variables().len();
    if data.n_vars() != p {
        return Err(RfError::Shape);
    }
    if repeats == 0 {
        return Err(RfError::Settings("repeats must be at least 1".into()));
    }
    let mut rng = Rng::new(seed);
    let n_p = data.n_presence();
    let bg_idx = subsample(&mut rng, data.n_background(), max_background);
    // Row-major pooled matrix: presences first, then the background subsample.
    let mut rows = data.presence_rows();
    let rows_b = data.background_rows();
    for &k in &bg_idx {
        rows.extend_from_slice(&rows_b[k * p..(k + 1) * p]);
    }
    let n = n_p + bg_idx.len();
    let score = |rows: &[f64]| -> f64 {
        let s: Vec<f64> = rows.chunks_exact(p).map(|r| forest.predict(r)).collect();
        niche_eval::auc(&s[..n_p], &s[n_p..])
    };
    let base = score(&rows);
    let mut out = Vec::with_capacity(p);
    let mut perm: Vec<usize> = (0..n).collect();
    let mut work = rows.clone();
    for j in 0..p {
        let mut drops = Vec::with_capacity(repeats);
        for _ in 0..repeats {
            rng.shuffle(&mut perm);
            for (i, &src) in perm.iter().enumerate() {
                work[i * p + j] = rows[src * p + j];
            }
            drops.push(base - score(&work));
        }
        for i in 0..n {
            work[i * p + j] = rows[i * p + j];
        }
        let mean = drops.iter().sum::<f64>() / repeats as f64;
        let sd = if repeats > 1 {
            (drops.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (repeats - 1) as f64).sqrt()
        } else {
            0.0
        };
        out.push(VariableImportance {
            variable: forest.variables()[j].clone(),
            auc_decrease: mean,
            sd,
            percent: 0.0,
        });
    }
    let total: f64 = out.iter().map(|v| v.auc_decrease.max(0.0)).sum();
    if total > 0.0 {
        for v in &mut out {
            v.percent = 100.0 * v.auc_decrease.max(0.0) / total;
        }
    }
    Ok(out)
}

/// A partial dependence curve: mean relative suitability (y) as one variable is set to x.
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseCurve {
    pub variable: String,
    pub categorical: bool,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// Partial dependence curves (Friedman 2001, §8.2) for every variable, in the order of
/// `data.variables`.
///
/// For each grid value `x` of variable `j`, `y(x)` is the mean prediction over a set of rows
/// with variable `j` overwritten by `x` and every other variable left at its observed value,
/// i.e. the marginal effect averaged over the joint distribution of the other predictors
/// (unlike MaxEnt's response curves, which hold the others at their mean). The rows are the
/// pooled presence + background rows of `data` (the training data in Friedman's definition),
/// deterministically subsampled to at most `max_rows` (seeded by `seed`) to bound cost:
/// `p × n_grid × max_rows` forest evaluations. The grid has `n_grid` evenly spaced points
/// from the training minimum to the training maximum of the variable (a single point for a
/// constant variable). Categorical variables use the same grid over their codes (they are
/// ordered codes in this engine); callers may restrict display to observed codes.
pub fn partial_dependence(
    forest: &Forest,
    data: &RfData,
    n_grid: usize,
    max_rows: usize,
    seed: u64,
) -> Result<Vec<ResponseCurve>, RfError> {
    (0..forest.variables().len())
        .map(|j| partial_dependence_of(forest, data, j, n_grid, max_rows, seed))
        .collect()
}

/// [`partial_dependence`] of the single variable `var`: identical to the `var`-th curve of
/// [`partial_dependence`] with the same arguments (same row subsample, drawn from `seed`), so a
/// caller can compute the variables in parallel.
pub fn partial_dependence_of(
    forest: &Forest,
    data: &RfData,
    var: usize,
    n_grid: usize,
    max_rows: usize,
    seed: u64,
) -> Result<ResponseCurve, RfError> {
    data.validate()?;
    let p = forest.variables().len();
    if data.n_vars() != p || var >= p {
        return Err(RfError::Shape);
    }
    if n_grid < 2 || max_rows == 0 {
        return Err(RfError::Settings(
            "partial dependence needs n_grid >= 2 and max_rows >= 1".into(),
        ));
    }
    let mut rng = Rng::new(seed);
    let mut pooled = data.presence_rows();
    pooled.extend(data.background_rows());
    let n_all = data.n_presence() + data.n_background();
    let idx = subsample(&mut rng, n_all, Some(max_rows));
    let mut rows = Vec::with_capacity(idx.len() * p);
    for &k in &idx {
        rows.extend_from_slice(&pooled[k * p..(k + 1) * p]);
    }
    let j = var;
    let (lo, hi) = forest.training_range(j).ok_or(RfError::Shape)?;
    let xs: Vec<f64> = if hi > lo {
        (0..n_grid)
            .map(|g| lo + (hi - lo) * g as f64 / (n_grid - 1) as f64)
            .collect()
    } else {
        vec![lo]
    };
    let ys = xs
        .iter()
        .map(|&x| {
            let mut s = 0.0;
            for r in rows.chunks_exact_mut(p) {
                r[j] = x;
                s += forest.predict(r);
            }
            s / idx.len() as f64
        })
        .collect();
    Ok(ResponseCurve {
        variable: forest.variables()[j].clone(),
        categorical: forest.categorical()[j],
        x: xs,
        y: ys,
    })
}

/// All of `0..n` in order, or a sorted random subset of size `max` if `n > max`.
fn subsample(rng: &mut Rng, n: usize, max: Option<usize>) -> Vec<usize> {
    match max {
        Some(m) if m < n => {
            let mut s = rng.sample_indices(n, m);
            s.sort_unstable();
            s
        }
        _ => (0..n).collect(),
    }
}
