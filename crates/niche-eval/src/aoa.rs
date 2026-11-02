//! Area of applicability (Meyer & Pebesma 2021, Methods Ecol. Evol. 12:1620–1633; R package
//! CAST `trainDI`/`aoa`).
//!
//! The dissimilarity index (DI) of a location is the weighted, standardized Euclidean distance in
//! predictor space to the nearest training point, divided by the mean pairwise distance between
//! training points. The AOA threshold is the upper whisker of the training DI computed under
//! cross-validation (each training point against the training points of the *other* folds),
//! i.e. the largest training DI not above Q3 + 1.5·IQR (`grDevices::boxplot.stats`, Tukey hinges).
//! Locations with DI above the threshold are outside the area of applicability: the model
//! extrapolates there, and CV error estimates do not apply.

use crate::kdtree::KdTree;
use crate::rng::Rng;

/// Training rows above this count are subsampled (seeded) to estimate the mean pairwise distance.
pub const AOA_MEAN_DISTANCE_SAMPLE: usize = 2000;

/// Why an [`Aoa`] could not be built.
#[derive(Debug, Clone, PartialEq)]
pub enum AoaError {
    /// No training row with all values finite.
    NoTrainingRows,
    /// A row's length differs from the first row's.
    RaggedRows {
        row: usize,
        expected: usize,
        found: usize,
    },
    /// `weights` length differs from the number of variables.
    WeightsLength { expected: usize, found: usize },
    /// `folds` length differs from the number of rows.
    FoldsLength { expected: usize, found: usize },
    /// All weighted, standardized training points coincide (mean pairwise distance 0).
    Degenerate,
}

impl std::fmt::Display for AoaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoTrainingRows => write!(f, "no training row with finite values"),
            Self::RaggedRows {
                row,
                expected,
                found,
            } => write!(f, "row {row} has {found} values, expected {expected}"),
            Self::WeightsLength { expected, found } => {
                write!(f, "{found} weights for {expected} variables")
            }
            Self::FoldsLength { expected, found } => {
                write!(f, "{found} fold labels for {expected} rows")
            }
            Self::Degenerate => write!(f, "training points coincide in weighted predictor space"),
        }
    }
}

impl std::error::Error for AoaError {}

/// Fitted area-of-applicability model; see the module docs.
#[derive(Debug, Clone)]
pub struct Aoa {
    center: Vec<f64>,
    /// weight / sd per variable (0 for constant variables or non-positive weights).
    scale: Vec<f64>,
    tree: KdTree,
    mean_distance: f64,
    threshold: f64,
    train_di: Vec<f64>,
}

impl Aoa {
    /// Builds the AOA from training predictor rows (`rows[i][v]`, presences and background),
    /// optional per-variable importance `weights` (e.g. permutation importance; `None` = all 1;
    /// negative or non-finite weights count as 0, as CAST sets negative weights to 0) and the CV
    /// fold of each row (`folds[i]`, any labels).
    ///
    /// - Variables are centred and scaled by the training mean and sample SD (R `scale()`), then
    ///   multiplied by their weight. Constant variables contribute nothing. Rescaling all weights
    ///   by a common factor does not change DI (it cancels in the ratio).
    /// - Rows with any non-finite value are left out of training; their `train_di` is NaN.
    /// - Mean pairwise distance: exact over all pairs up to [`AOA_MEAN_DISTANCE_SAMPLE`] rows,
    ///   otherwise over all pairs of a seeded subsample of that size (`rng`).
    /// - Training DI: nearest neighbour among rows of other folds. If all rows share one fold, the
    ///   nearest other row is used (CAST's behaviour without CV folds).
    ///
    /// Cost: O(n log n) build, k tree builds for the CV distances, O(m²·d) for the mean distance.
    pub fn new(
        rows: &[Vec<f64>],
        weights: Option<&[f64]>,
        folds: &[usize],
        rng: &mut Rng,
    ) -> Result<Self, AoaError> {
        let dims = rows.first().map(Vec::len).ok_or(AoaError::NoTrainingRows)?;
        if let Some((row, r)) = rows.iter().enumerate().find(|(_, r)| r.len() != dims) {
            return Err(AoaError::RaggedRows {
                row,
                expected: dims,
                found: r.len(),
            });
        }
        if folds.len() != rows.len() {
            return Err(AoaError::FoldsLength {
                expected: rows.len(),
                found: folds.len(),
            });
        }
        let w: Vec<f64> = match weights {
            Some(w) if w.len() != dims => {
                return Err(AoaError::WeightsLength {
                    expected: dims,
                    found: w.len(),
                });
            }
            Some(w) => w
                .iter()
                .map(|&x| if x.is_finite() && x > 0.0 { x } else { 0.0 })
                .collect(),
            None => vec![1.0; dims],
        };
        let used: Vec<usize> = (0..rows.len())
            .filter(|&i| rows[i].iter().all(|v| v.is_finite()))
            .collect();
        if used.is_empty() {
            return Err(AoaError::NoTrainingRows);
        }
        let n = used.len() as f64;
        let mut center = vec![0.0; dims];
        for &i in &used {
            for (c, v) in center.iter_mut().zip(&rows[i]) {
                *c += v;
            }
        }
        center.iter_mut().for_each(|c| *c /= n);
        let mut var = vec![0.0; dims];
        for &i in &used {
            for ((s, v), c) in var.iter_mut().zip(&rows[i]).zip(&center) {
                *s += (v - c) * (v - c);
            }
        }
        let scale: Vec<f64> = var
            .iter()
            .zip(&w)
            .map(|(&s, &wt)| {
                let sd = if used.len() > 1 {
                    (s / (n - 1.0)).sqrt()
                } else {
                    0.0
                };
                if sd > 0.0 && wt > 0.0 { wt / sd } else { 0.0 }
            })
            .collect();
        let transform = |r: &[f64]| -> Vec<f64> {
            r.iter()
                .zip(&center)
                .zip(&scale)
                .map(|((v, c), s)| (v - c) * s)
                .collect()
        };
        let train: Vec<Vec<f64>> = used.iter().map(|&i| transform(&rows[i])).collect();

        let sample: Vec<usize> = if train.len() > AOA_MEAN_DISTANCE_SAMPLE {
            rng.sample_indices(train.len(), AOA_MEAN_DISTANCE_SAMPLE)
        } else {
            (0..train.len()).collect()
        };
        let (mut sum, mut pairs) = (0.0, 0usize);
        for (a, &i) in sample.iter().enumerate() {
            for &j in &sample[a + 1..] {
                sum += euclid(&train[i], &train[j]);
                pairs += 1;
            }
        }
        let mean_distance = if pairs > 0 { sum / pairs as f64 } else { 0.0 };
        if mean_distance.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(AoaError::Degenerate);
        }
        let tree = KdTree::new(&train).ok_or(AoaError::NoTrainingRows)?;

        // CV nearest-neighbour distances: one tree per fold over the rows of the other folds.
        let mut fold_ids: Vec<usize> = used.iter().map(|&i| folds[i]).collect();
        fold_ids.sort_unstable();
        fold_ids.dedup();
        let mut cv_dist = vec![f64::NAN; train.len()];
        if fold_ids.len() < 2 {
            for (t, row) in train.iter().enumerate() {
                if let Some((_, d)) = tree.nearest_excluding(row, t) {
                    cv_dist[t] = d;
                }
            }
        } else {
            for &f in &fold_ids {
                let others: Vec<Vec<f64>> = (0..train.len())
                    .filter(|&t| folds[used[t]] != f)
                    .map(|t| train[t].clone())
                    .collect();
                let Some(other_tree) = KdTree::new(&others) else {
                    continue;
                };
                for t in (0..train.len()).filter(|&t| folds[used[t]] == f) {
                    if let Some((_, d)) = other_tree.nearest(&train[t]) {
                        cv_dist[t] = d;
                    }
                }
            }
        }
        let mut train_di = vec![f64::NAN; rows.len()];
        for (t, &i) in used.iter().enumerate() {
            train_di[i] = cv_dist[t] / mean_distance;
        }
        let threshold = upper_whisker(&train_di);
        Ok(Self {
            center,
            scale,
            tree,
            mean_distance,
            threshold,
            train_di,
        })
    }

    /// Dissimilarity index of a predictor row (same variable order as training). NaN when the
    /// row has the wrong length or a non-finite value in a variable with non-zero weight.
    pub fn eval(&self, row: &[f64]) -> f64 {
        if row.len() != self.center.len() {
            return f64::NAN;
        }
        let mut q = Vec::with_capacity(row.len());
        for ((v, c), s) in row.iter().zip(&self.center).zip(&self.scale) {
            if *s == 0.0 {
                q.push(0.0);
            } else if v.is_finite() {
                q.push((v - c) * s);
            } else {
                return f64::NAN;
            }
        }
        match self.tree.nearest(&q) {
            Some((_, d)) => d / self.mean_distance,
            None => f64::NAN,
        }
    }

    /// AOA threshold: locations with `eval(row) > threshold()` are outside the AOA.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Cross-validated training DI, aligned with the input rows (NaN for rows left out).
    pub fn train_di(&self) -> &[f64] {
        &self.train_di
    }

    /// Mean pairwise distance between training points in weighted standardized space.
    pub fn mean_distance(&self) -> f64 {
        self.mean_distance
    }

    /// Number of training rows actually used.
    pub fn n_train(&self) -> usize {
        self.tree.len()
    }
}

fn euclid(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// Upper whisker of `grDevices::boxplot.stats(x, coef = 1.5)`: the largest finite value not above
/// the upper Tukey hinge + 1.5 × (upper hinge − lower hinge). NaN if no finite value.
pub fn upper_whisker(x: &[f64]) -> f64 {
    let mut v: Vec<f64> = x.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let (lo, hi) = tukey_hinges(&v);
    let cap = hi + 1.5 * (hi - lo);
    v.iter()
        .copied()
        .filter(|&x| x <= cap)
        .fold(f64::NEG_INFINITY, f64::max)
}

/// Lower and upper hinges of R's `fivenum` on sorted, non-empty `v`.
fn tukey_hinges(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let n4 = ((n + 3.0) / 2.0).floor() / 2.0;
    // 1-based positions as in R.
    let at = |d: f64| 0.5 * (v[d.floor() as usize - 1] + v[d.ceil() as usize - 1]);
    (at(n4), at(n + 1.0 - n4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisker_matches_boxplot_stats() {
        // boxplot.stats(c(1:10, 100))$stats[5] == 10; fivenum hinges 3.5 and 8.5.
        let mut v: Vec<f64> = (1..=10).map(f64::from).collect();
        v.push(100.0);
        assert_eq!(
            tukey_hinges(&{
                let mut s = v.clone();
                s.sort_by(|a, b| a.total_cmp(b));
                s
            }),
            (3.5, 8.5)
        );
        assert_eq!(upper_whisker(&v), 10.0);
        // fivenum(1:4) hinges 1.5 and 3.5.
        assert_eq!(tukey_hinges(&[1.0, 2.0, 3.0, 4.0]), (1.5, 3.5));
        assert_eq!(upper_whisker(&[5.0]), 5.0);
        assert!(upper_whisker(&[f64::NAN]).is_nan());
    }

    fn grid_rows() -> (Vec<Vec<f64>>, Vec<usize>) {
        let mut rows = Vec::new();
        let mut folds = Vec::new();
        for i in 0..20 {
            for j in 0..20 {
                rows.push(vec![i as f64, j as f64 * 10.0]);
                folds.push((i / 5) % 4);
            }
        }
        (rows, folds)
    }

    #[test]
    fn di_zero_on_training_and_large_far_away() {
        let (rows, folds) = grid_rows();
        let aoa = Aoa::new(&rows, None, &folds, &mut Rng::new(1)).expect("aoa");
        assert_eq!(aoa.n_train(), 400);
        assert_eq!(aoa.eval(&[3.0, 50.0]), 0.0);
        let far = aoa.eval(&[100.0, 50.0]);
        assert!(far > aoa.threshold(), "{far} {}", aoa.threshold());
        assert!(aoa.eval(&[3.5, 55.0]) <= aoa.threshold());
        assert!(aoa.threshold().is_finite() && aoa.threshold() > 0.0);
        // Scale invariance: the second variable is in different units, standardized away.
        let scaled: Vec<Vec<f64>> = rows.iter().map(|r| vec![r[0], r[1] / 10.0]).collect();
        let aoa2 = Aoa::new(&scaled, None, &folds, &mut Rng::new(1)).expect("aoa");
        assert!((aoa2.eval(&[100.0, 5.0]) - far).abs() < 1e-9);
        assert!((aoa2.threshold() - aoa.threshold()).abs() < 1e-12);
    }

    #[test]
    fn di_matches_brute_force_definition() {
        let mut r = Rng::new(5);
        let rows: Vec<Vec<f64>> = (0..300)
            .map(|_| vec![r.uniform(), 3.0 * r.uniform(), r.uniform() * r.uniform()])
            .collect();
        let folds: Vec<usize> = (0..300).map(|i| i % 3).collect();
        let w = [2.0, 1.0, 0.5];
        let aoa = Aoa::new(&rows, Some(&w), &folds, &mut Rng::new(1)).expect("aoa");
        // Manual standardization (sample SD) and weighting.
        let m: Vec<f64> = (0..3)
            .map(|v| rows.iter().map(|x| x[v]).sum::<f64>() / 300.0)
            .collect();
        let sd: Vec<f64> = (0..3)
            .map(|v| (rows.iter().map(|x| (x[v] - m[v]).powi(2)).sum::<f64>() / 299.0).sqrt())
            .collect();
        let t = |x: &[f64]| -> Vec<f64> { (0..3).map(|v| (x[v] - m[v]) / sd[v] * w[v]).collect() };
        let tr: Vec<Vec<f64>> = rows.iter().map(|x| t(x)).collect();
        let mut s = 0.0;
        for i in 0..300 {
            for j in (i + 1)..300 {
                s += euclid(&tr[i], &tr[j]);
            }
        }
        let mean_d = s / (300.0 * 299.0 / 2.0);
        assert!((aoa.mean_distance() - mean_d).abs() < 1e-9);
        let q = [0.5, 4.0, 0.9];
        let tq = t(&q);
        let nn = tr
            .iter()
            .map(|x| euclid(x, &tq))
            .fold(f64::INFINITY, f64::min);
        assert!((aoa.eval(&q) - nn / mean_d).abs() < 1e-12);
        // CV training DI of row 0: nearest among other folds.
        let nn0 = (0..300)
            .filter(|&j| folds[j] != folds[0])
            .map(|j| euclid(&tr[0], &tr[j]))
            .fold(f64::INFINITY, f64::min);
        assert!((aoa.train_di()[0] - nn0 / mean_d).abs() < 1e-12);
    }

    #[test]
    fn handles_nan_rows_zero_weights_and_errors() {
        let (mut rows, folds) = grid_rows();
        rows[0][1] = f64::NAN;
        let aoa = Aoa::new(&rows, Some(&[1.0, 0.0]), &folds, &mut Rng::new(1)).expect("aoa");
        assert!(aoa.train_di()[0].is_nan());
        assert_eq!(aoa.n_train(), 399);
        // Second variable has zero weight: its value (even NaN) is ignored.
        assert_eq!(aoa.eval(&[2.0, f64::NAN]), 0.0);
        assert!(aoa.eval(&[f64::NAN, 1.0]).is_nan());
        assert!(aoa.eval(&[1.0]).is_nan());
        let mut r = Rng::new(1);
        assert_eq!(
            Aoa::new(&rows, Some(&[1.0]), &folds, &mut r).err(),
            Some(AoaError::WeightsLength {
                expected: 2,
                found: 1
            })
        );
        assert!(matches!(
            Aoa::new(&rows, None, &folds[1..], &mut r),
            Err(AoaError::FoldsLength { .. })
        ));
        assert_eq!(
            Aoa::new(&vec![vec![1.0, 2.0]; 5], None, &[0; 5], &mut r).err(),
            Some(AoaError::Degenerate)
        );
        assert_eq!(
            Aoa::new(&[], None, &[], &mut r).err(),
            Some(AoaError::NoTrainingRows)
        );
        // Single fold: leave-one-out nearest neighbour.
        let (rows, _) = grid_rows();
        let one = Aoa::new(&rows, None, &[0; 400], &mut r).expect("aoa");
        assert!(one.train_di().iter().all(|d| *d > 0.0));
    }

    /// Throughput probe (single thread): `cargo test -p niche-eval --release aoa_query_throughput
    /// -- --ignored --nocapture`. Climate-like rows: 19 variables driven by 3 latent factors plus
    /// noise (low intrinsic dimension), and the uniform 19-D worst case.
    #[test]
    #[ignore]
    fn aoa_query_throughput() {
        let mut r = Rng::new(8);
        let latent_row = |r: &mut Rng| -> Vec<f64> {
            let f: Vec<f64> = (0..3).map(|_| r.uniform()).collect();
            (0..19)
                .map(|v| f[v % 3] * (1.0 + v as f64 / 10.0) + 0.02 * r.uniform())
                .collect()
        };
        let rows: Vec<Vec<f64>> = (0..11_000).map(|_| latent_row(&mut r)).collect();
        let folds: Vec<usize> = (0..11_000).map(|i| i % 5).collect();
        let t = std::time::Instant::now();
        let aoa = Aoa::new(&rows, None, &folds, &mut Rng::new(2)).expect("aoa");
        let build = t.elapsed();
        let queries: Vec<Vec<f64>> = (0..200_000).map(|_| latent_row(&mut r)).collect();
        let t = std::time::Instant::now();
        let s: f64 = queries.iter().map(|q| aoa.eval(q)).sum();
        let q = t.elapsed();
        eprintln!(
            "latent-3: build {build:?}, 200k queries {q:?} ({:.2} µs/query, checksum {s:.3})",
            q.as_secs_f64() / 2e5 * 1e6
        );
        let rows: Vec<Vec<f64>> = (0..11_000)
            .map(|_| (0..19).map(|_| r.uniform()).collect())
            .collect();
        let t = std::time::Instant::now();
        let aoa = Aoa::new(&rows, None, &folds, &mut Rng::new(2)).expect("aoa");
        let build = t.elapsed();
        let queries: Vec<Vec<f64>> = (0..20_000)
            .map(|_| (0..19).map(|_| r.uniform()).collect())
            .collect();
        let t = std::time::Instant::now();
        let s: f64 = queries.iter().map(|q| aoa.eval(q)).sum();
        let q = t.elapsed();
        eprintln!(
            "uniform-19: build {build:?}, 20k queries {q:?} ({:.2} µs/query, checksum {s:.3})",
            q.as_secs_f64() / 2e4 * 1e6
        );
    }

    #[test]
    fn large_training_set_is_deterministic_and_fast() {
        let mut r = Rng::new(8);
        let rows: Vec<Vec<f64>> = (0..11_000)
            .map(|_| (0..19).map(|_| r.uniform()).collect())
            .collect();
        let folds: Vec<usize> = (0..11_000).map(|i| i % 5).collect();
        let a = Aoa::new(&rows, None, &folds, &mut Rng::new(2)).expect("aoa");
        let b = Aoa::new(&rows, None, &folds, &mut Rng::new(2)).expect("aoa");
        assert_eq!(a.threshold(), b.threshold());
        assert_eq!(a.mean_distance(), b.mean_distance());
        let q: Vec<f64> = (0..19).map(|_| r.uniform()).collect();
        assert_eq!(a.eval(&q), b.eval(&q));
    }
}
