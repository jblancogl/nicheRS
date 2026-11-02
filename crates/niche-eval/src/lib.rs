//! Model evaluation for presence-background SDMs: data partitioning, discrimination and
//! calibration-free metrics, thresholds, environmental novelty (MESS) and collinearity.
//!
//! Science v1 additions (`docs/architecture/eval-methods-v1.md`):
//! - [`spatial`]: great-circle distances, distance thinning of presences (spThin), square
//!   spatial blocks for CV (blockCV) and the spatial autocorrelation range used to size them;
//! - [`aoa`] and [`kdtree`]: area of applicability / dissimilarity index (CAST) on an exact
//!   k-d tree nearest-neighbour search;
//! - [`nulls`]: null-model summaries (ENMeval `ENMnulls`) and weighted sampling without
//!   replacement (Efraimidis–Spirakis);
//! - [`pa`]: presence–absence metrics for independent validation (TSS, kappa, Brier,
//!   calibration table).
//!
//! Pure functions, no I/O, no threads; all randomness comes from the frozen [`rng::Rng`]. Method
//! choices and their evidence are documented in `docs/research/07-scientific-validation.md` and
//! ADR-0009.

pub mod aoa;
pub mod kdtree;
pub mod nulls;
pub mod pa;
pub mod rng;
pub mod spatial;

pub use aoa::{Aoa, AoaError, upper_whisker};
pub use kdtree::KdTree;
pub use nulls::{NullSummary, null_presences, null_summary, sample_weighted_indices};
pub use pa::{
    CalibrationBin, ConfusionMatrix, PaMetrics, brier_score, calibration_table, pa_metrics,
};
pub use spatial::{
    EARTH_RADIUS_KM, RangeEstimate, VariogramFit, VariogramModel, autocorrelation_range_km,
    haversine_km, spatial_grid_blocks, thin_by_distance, thin_by_distance_reps,
};

use rng::Rng;

/// Fold assignment of presences and background points (`0..k`).
#[derive(Debug, Clone, PartialEq)]
pub struct Folds {
    pub k: usize,
    pub presence: Vec<usize>,
    pub background: Vec<usize>,
}

/// Spatial "block" partitioning (Muscarella et al. 2014, ENMeval `get.block`, lat→lon):
/// presences are split at their median latitude, each half at its median longitude, giving
/// four groups of near-equal size; background points are assigned with the same lines.
/// Spatially structured folds reduce the optimism of random CV under spatial autocorrelation
/// (Roberts et al. 2017; Valavi et al. 2019).
pub fn spatial_blocks(pres: &[(f64, f64)], bg: &[(f64, f64)]) -> Folds {
    let lat_split = median(pres.iter().map(|p| p.1).collect());
    let south: Vec<f64> = pres
        .iter()
        .filter(|p| p.1 <= lat_split)
        .map(|p| p.0)
        .collect();
    let north: Vec<f64> = pres
        .iter()
        .filter(|p| p.1 > lat_split)
        .map(|p| p.0)
        .collect();
    let lon_s = if south.is_empty() { 0.0 } else { median(south) };
    let lon_n = if north.is_empty() { 0.0 } else { median(north) };
    let assign = |&(x, y): &(f64, f64)| -> usize {
        if y <= lat_split {
            if x <= lon_s { 0 } else { 1 }
        } else if x <= lon_n {
            2
        } else {
            3
        }
    };
    Folds {
        k: 4,
        presence: pres.iter().map(assign).collect(),
        background: bg.iter().map(assign).collect(),
    }
}

/// Random k-fold for presences; background is shared by all folds (assigned fold `k`, i.e. never
/// held out), as in ENMeval's random k-fold.
pub fn random_kfold(n_pres: usize, n_bg: usize, k: usize, rng: &mut Rng) -> Folds {
    let mut idx: Vec<usize> = (0..n_pres).collect();
    rng.shuffle(&mut idx);
    let mut presence = vec![0; n_pres];
    for (pos, &i) in idx.iter().enumerate() {
        presence[i] = pos % k;
    }
    Folds {
        k,
        presence,
        background: vec![k; n_bg],
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// Threshold below which 10 % of training presences fall ("10th percentile training presence",
/// ENMeval `or.10p` definition: the value at rank ceil(0.9 n) in decreasing order).
pub fn p10_threshold(train_pres: &[f64]) -> f64 {
    let mut v = train_pres.to_vec();
    v.sort_by(|a, b| b.total_cmp(a));
    let n90 = ((v.len() as f64) * 0.9).ceil() as usize;
    v[n90.clamp(1, v.len()) - 1]
}

/// Minimum training presence (MTP) threshold.
pub fn mtp_threshold(train_pres: &[f64]) -> f64 {
    train_pres.iter().copied().fold(f64::INFINITY, f64::min)
}

/// Threshold maximizing sensitivity + specificity using presences vs background
/// (Liu et al. 2013: valid for presence-only data with background as pseudo-absences).
pub fn max_sss_threshold(pres: &[f64], bg: &[f64]) -> f64 {
    let mut cands: Vec<f64> = pres.iter().chain(bg).copied().collect();
    cands.sort_by(|a, b| a.total_cmp(b));
    cands.dedup();
    let mut ps = pres.to_vec();
    ps.sort_by(|a, b| a.total_cmp(b));
    let mut bs = bg.to_vec();
    bs.sort_by(|a, b| a.total_cmp(b));
    let (np, nb) = (ps.len() as f64, bs.len() as f64);
    let mut best = (f64::NEG_INFINITY, cands.first().copied().unwrap_or(0.0));
    for &t in &cands {
        let sens = 1.0 - ps.partition_point(|&x| x < t) as f64 / np;
        let spec = bs.partition_point(|&x| x < t) as f64 / nb;
        if sens + spec > best.0 {
            best = (sens + spec, t);
        }
    }
    best.1
}

/// Fraction of `test` values strictly below `threshold`.
pub fn omission_rate(test: &[f64], threshold: f64) -> f64 {
    if test.is_empty() {
        return f64::NAN;
    }
    test.iter().filter(|&&x| x < threshold).count() as f64 / test.len() as f64
}

/// Presence-background AUC (Mann–Whitney, ties count ½). Its maximum is < 1 because background
/// contains presences (Phillips et al. 2006); report as "presence-background AUC".
pub fn auc(pres: &[f64], bg: &[f64]) -> f64 {
    let mut bs = bg.to_vec();
    bs.sort_by(|a, b| a.total_cmp(b));
    let mut s = 0.0;
    for &p in pres {
        let below = bs.partition_point(|&x| x < p);
        let not_above = bs.partition_point(|&x| x <= p);
        s += below as f64 + 0.5 * (not_above - below) as f64;
    }
    s / (pres.len() as f64 * bs.len() as f64)
}

/// Continuous Boyce index (Hirzel et al. 2006), following `ecospat.boyce` defaults: 101 moving
/// windows of width (max−min)/10 over the range of background predictions, predicted-to-expected
/// ratio F per window, consecutive duplicate F removed, Spearman correlation of F with window
/// centres. Returns `None` when fewer than 3 windows are informative.
pub fn continuous_boyce(pres: &[f64], bg: &[f64]) -> Option<f64> {
    let (lo, hi) = bg
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| {
            (a.min(x), b.max(x))
        });
    if hi.partial_cmp(&lo) != Some(std::cmp::Ordering::Greater) || pres.is_empty() {
        return None;
    }
    let w = (hi - lo) / 10.0;
    let steps = 101;
    let mut ps = pres.to_vec();
    ps.sort_by(|a, b| a.total_cmp(b));
    let mut bs = bg.to_vec();
    bs.sort_by(|a, b| a.total_cmp(b));
    let count =
        |v: &[f64], a: f64, b: f64| v.partition_point(|&x| x <= b) - v.partition_point(|&x| x < a);
    let mut f = Vec::new();
    let mut mids = Vec::new();
    for i in 0..steps {
        let a = lo + (hi - w - lo) * i as f64 / (steps - 1) as f64;
        let b = a + w;
        let e = count(&bs, a, b) as f64 / bs.len() as f64;
        if e == 0.0 {
            continue;
        }
        let p = count(&ps, a, b) as f64 / ps.len() as f64;
        f.push(p / e);
        mids.push((a + b) / 2.0);
    }
    let mut ff = Vec::new();
    let mut mm = Vec::new();
    for i in 0..f.len() {
        if i == 0 || f[i] != f[i - 1] {
            ff.push(f[i]);
            mm.push(mids[i]);
        }
    }
    if ff.len() < 3 {
        return None;
    }
    spearman(&ff, &mm)
}

fn ranks(v: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    let mut r = vec![0.0; v.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for &k in &idx[i..=j] {
            r[k] = avg;
        }
        i = j + 1;
    }
    r
}

pub fn spearman(a: &[f64], b: &[f64]) -> Option<f64> {
    pearson(&ranks(a), &ranks(b))
}

pub fn pearson(a: &[f64], b: &[f64]) -> Option<f64> {
    let n = a.len() as f64;
    if a.len() < 2 || a.len() != b.len() {
        return None;
    }
    let ma = a.iter().sum::<f64>() / n;
    let mb = b.iter().sum::<f64>() / n;
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma) * (x - ma);
        sbb += (y - mb) * (y - mb);
    }
    if saa == 0.0 || sbb == 0.0 {
        return None;
    }
    Some(sab / (saa * sbb).sqrt())
}

/// Greedy collinearity filter: keep variables in priority order, dropping any whose |Pearson r|
/// with an already kept variable exceeds `max_abs_r` (Dormann et al. 2013 recommend 0.7).
/// Returns kept indices and, for dropped ones, (index, conflicting kept index, r).
pub fn collinearity_filter(
    columns: &[Vec<f64>],
    max_abs_r: f64,
) -> (Vec<usize>, Vec<(usize, usize, f64)>) {
    let mut kept: Vec<usize> = Vec::new();
    let mut dropped = Vec::new();
    for i in 0..columns.len() {
        let mut conflict = None;
        for &k in &kept {
            if let Some(r) = pearson(&columns[i], &columns[k])
                && r.abs() > max_abs_r
            {
                conflict = Some((k, r));
                break;
            }
        }
        match conflict {
            Some((k, r)) => dropped.push((i, k, r)),
            None => kept.push(i),
        }
    }
    (kept, dropped)
}

/// Multivariate environmental similarity surface (Elith et al. 2010). `reference[var]` holds the
/// training environment; returns MESS per query row (negative = outside the training range of at
/// least one variable).
pub struct Mess {
    sorted: Vec<Vec<f64>>,
}

impl Mess {
    pub fn new(reference: &[Vec<f64>]) -> Self {
        let sorted = reference
            .iter()
            .map(|c| {
                let mut v = c.clone();
                v.sort_by(|a, b| a.total_cmp(b));
                v
            })
            .collect();
        Self { sorted }
    }

    pub fn eval(&self, x: &[f64]) -> f64 {
        let mut m = f64::INFINITY;
        for (v, &p) in self.sorted.iter().zip(x) {
            let n = v.len() as f64;
            let (min, max) = (v[0], v[v.len() - 1]);
            let range = max - min;
            let f = 100.0 * v.partition_point(|&r| r < p) as f64 / n;
            let s = if range == 0.0 {
                if p == min { 100.0 } else { -100.0 }
            } else if f == 0.0 {
                (p - min) / range * 100.0
            } else if f <= 50.0 {
                2.0 * f
            } else if f < 100.0 {
                2.0 * (100.0 - f)
            } else {
                (max - p) / range * 100.0
            };
            m = m.min(s);
        }
        m
    }
}

/// Standard deviation of a slice (population SD, used for per-cell spread across fold models).
pub fn sd(v: &[f64]) -> f64 {
    let n = v.len() as f64;
    if v.is_empty() {
        return f64::NAN;
    }
    let m = v.iter().sum::<f64>() / n;
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auc_perfect_and_random() {
        assert_eq!(auc(&[2.0, 3.0], &[0.0, 1.0]), 1.0);
        assert_eq!(auc(&[1.0], &[1.0]), 0.5);
    }

    #[test]
    fn p10_matches_enmeval_definition() {
        let v: Vec<f64> = (1..=10).map(f64::from).collect();
        // ceil(0.9*10)=9th largest = 2
        assert_eq!(p10_threshold(&v), 2.0);
        assert_eq!(mtp_threshold(&v), 1.0);
        assert_eq!(omission_rate(&v, 2.0), 0.1);
    }

    #[test]
    fn boyce_positive_for_good_model() {
        let bg: Vec<f64> = (0..1000).map(|i| i as f64 / 1000.0).collect();
        let pres: Vec<f64> = (0..200).map(|i| 0.5 + i as f64 / 400.0).collect();
        let b = continuous_boyce(&pres, &bg).expect("defined");
        assert!(b > 0.8, "{b}");
    }

    #[test]
    fn mess_negative_outside_range() {
        let m = Mess::new(&[vec![0.0, 1.0, 2.0, 3.0]]);
        assert!(m.eval(&[-1.0]) < 0.0);
        assert!(m.eval(&[1.5]) > 0.0);
        assert!(m.eval(&[4.0]) < 0.0);
    }

    #[test]
    fn blocks_balanced() {
        let pres: Vec<(f64, f64)> = (0..100)
            .map(|i| ((i * 37 % 100) as f64, (i * 61 % 100) as f64))
            .collect();
        let f = spatial_blocks(&pres, &[(0.0, 0.0)]);
        let mut counts = [0; 4];
        for &g in &f.presence {
            counts[g] += 1;
        }
        assert!(counts.iter().all(|&c| (20..=30).contains(&c)), "{counts:?}");
    }

    #[test]
    fn collinearity_drops_duplicates() {
        let a: Vec<f64> = (0..50).map(f64::from).collect();
        let b: Vec<f64> = a.iter().map(|x| 2.0 * x + 1.0).collect();
        let c: Vec<f64> = a.iter().map(|x| (x * 1.7).sin()).collect();
        let (kept, dropped) = collinearity_filter(&[a, b, c], 0.7);
        assert_eq!(kept, vec![0, 2]);
        assert_eq!(dropped[0].0, 1);
    }

    #[test]
    fn max_sss_separates() {
        let t = max_sss_threshold(&[0.8, 0.9], &[0.1, 0.2, 0.85]);
        assert!(t > 0.2 && t <= 0.8, "{t}");
    }
}
