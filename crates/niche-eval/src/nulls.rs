//! Null-model significance and weighted sampling.
//!
//! Null models (Raes & ter Steege 2007, Ecography 30:727–736; Bohl et al. 2019, J.
//! Biogeogr. 46:1101–1111; ENMeval 2.0 `ENMnulls`, Kass et al. 2021): models are refitted with the
//! same settings on presence sets drawn at random from the study area (optionally with the same
//! sampling bias as the real records), and the observed metric is compared with the null
//! distribution.

use crate::rng::Rng;

/// Observed metric against its null distribution.
#[derive(Debug, Clone, PartialEq)]
pub struct NullSummary {
    /// Mean of the finite null values.
    pub mean: f64,
    /// Sample standard deviation (n − 1) of the finite null values, as R's `sd()` in ENMeval.
    pub sd: f64,
    /// (observed − mean) / sd. Its sign is not flipped for lower-is-better metrics: for those a
    /// good model has a negative z.
    pub z: f64,
    /// Empirical one-sided p-value (1 + #nulls as good as or better than observed) / (n + 1)
    /// (North, Curtis & Sham 2002); never 0, so it cannot claim more than the number of nulls
    /// supports.
    pub p_value: f64,
    /// Number of finite null values used.
    pub n: usize,
}

/// Summarizes `observed` against `nulls`. `higher_is_better` is true for AUC/Boyce and false
/// for omission rates. Non-finite nulls are ignored. With no finite null, mean/sd/z are NaN and
/// p = 1; with one, sd and z are NaN. A non-finite `observed` gives NaN z and p = 1.
pub fn null_summary(observed: f64, nulls: &[f64], higher_is_better: bool) -> NullSummary {
    let v: Vec<f64> = nulls.iter().copied().filter(|x| x.is_finite()).collect();
    let n = v.len();
    let mean = if n > 0 {
        v.iter().sum::<f64>() / n as f64
    } else {
        f64::NAN
    };
    let sd = if n > 1 {
        (v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1) as f64).sqrt()
    } else {
        f64::NAN
    };
    let z = if sd > 0.0 {
        (observed - mean) / sd
    } else {
        f64::NAN
    };
    let as_good = if observed.is_finite() {
        v.iter()
            .filter(|&&x| {
                if higher_is_better {
                    x >= observed
                } else {
                    x <= observed
                }
            })
            .count()
    } else {
        n
    };
    NullSummary {
        mean,
        sd,
        z,
        p_value: (1 + as_good) as f64 / (n + 1) as f64,
        n,
    }
}

/// Draws a null presence set of `n` distinct elements of `eligible` (e.g. indices of land cells
/// or background points), without replacement. With `weights` (`weights[i]` belongs to
/// `eligible[i]`; missing entries count as 0) the draw is weighted, e.g. by a target-group
/// sampling-effort surface so that null records share the bias of real ones (Bohl et al. 2019).
/// Returns fewer than `n` elements when not enough are eligible (positive weight). Deterministic
/// for a given seed; the result is in draw order.
pub fn null_presences(
    eligible: &[usize],
    weights: Option<&[f64]>,
    n: usize,
    rng: &mut Rng,
) -> Vec<usize> {
    match weights {
        None => rng
            .sample_indices(eligible.len(), n)
            .into_iter()
            .map(|i| eligible[i])
            .collect(),
        Some(w) => {
            let w: Vec<f64> = (0..eligible.len())
                .map(|i| w.get(i).copied().unwrap_or(0.0))
                .collect();
            sample_weighted_indices(&w, n, rng)
                .into_iter()
                .map(|i| eligible[i])
                .collect()
        }
    }
}

/// Weighted sampling without replacement of `n` indices of `weights` (Efraimidis & Spirakis 2006,
/// Inf. Process. Lett. 97:181–185, algorithm A-Res): each item with weight w > 0 gets the key
/// ln(u)/w with u uniform on (0, 1] (order-equivalent to u^(1/w), without underflow for small w),
/// and the `n` largest keys are returned in decreasing key order — the same distribution as
/// drawing one at a time with probability proportional to weight among the remaining items.
///
/// Zero, negative and non-finite weights are never drawn; if fewer than `n` items have a
/// positive finite weight, all of them are returned. One uniform draw per positive weight, in
/// index order, so the result is deterministic for a given seed. Cost O(N + n log n).
pub fn sample_weighted_indices(weights: &[f64], n: usize, rng: &mut Rng) -> Vec<usize> {
    let mut keys: Vec<(f64, usize)> = weights
        .iter()
        .enumerate()
        .filter(|(_, w)| w.is_finite() && **w > 0.0)
        .map(|(i, &w)| ((1.0 - rng.uniform()).ln() / w, i))
        .collect();
    // Decreasing key, ties by increasing index.
    let cmp = |a: &(f64, usize), b: &(f64, usize)| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1));
    if n == 0 {
        return Vec::new();
    }
    if n < keys.len() {
        keys.select_nth_unstable_by(n - 1, cmp);
        keys.truncate(n);
    }
    keys.sort_by(cmp);
    keys.into_iter().map(|(_, i)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_values() {
        let nulls = [0.5, 0.6, 0.7, 0.8, f64::NAN];
        let s = null_summary(0.75, &nulls, true);
        assert_eq!(s.n, 4);
        assert!((s.mean - 0.65).abs() < 1e-12);
        let sd = (0.05f64 / 3.0).sqrt();
        assert!((s.sd - sd).abs() < 1e-12);
        assert!((s.z - 0.1 / sd).abs() < 1e-9);
        // One null (0.8) at least as good: (1 + 1) / 5.
        assert!((s.p_value - 0.4).abs() < 1e-12);
        // Lower is better (omission): observed 0.55 beaten-or-equalled by 0.5 only.
        let s = null_summary(0.55, &nulls, false);
        assert!((s.p_value - 0.4).abs() < 1e-12);
        // Better than every null: smallest attainable p = 1/(n+1).
        assert!((null_summary(1.0, &nulls, true).p_value - 0.2).abs() < 1e-12);
        let e = null_summary(0.7, &[], true);
        assert_eq!((e.n, e.p_value), (0, 1.0));
        assert!(e.mean.is_nan() && e.z.is_nan());
        assert_eq!(null_summary(f64::NAN, &nulls, true).p_value, 1.0);
    }

    #[test]
    fn weighted_sampling_basic_properties() {
        let w = [0.0, 1.0, -1.0, f64::NAN, 2.0, 3.0];
        let mut r = Rng::new(1);
        let all = sample_weighted_indices(&w, 10, &mut r);
        let mut sorted = all.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![1, 4, 5]);
        assert!(sample_weighted_indices(&w, 0, &mut r).is_empty());
        let a = sample_weighted_indices(&w, 2, &mut Rng::new(7));
        let b = sample_weighted_indices(&w, 2, &mut Rng::new(7));
        assert_eq!(a, b);
        assert_eq!(a.len(), 2);
        assert_ne!(a[0], a[1]);
    }

    #[test]
    fn weighted_sampling_first_draw_proportional_to_weight() {
        // P(first draw = i) = w_i / Σw for A-Res.
        let w = [1.0, 2.0, 7.0];
        let mut r = Rng::new(3);
        let mut first = [0usize; 3];
        let trials = 20_000;
        for _ in 0..trials {
            first[sample_weighted_indices(&w, 2, &mut r)[0]] += 1;
        }
        for (i, &c) in first.iter().enumerate() {
            let p = c as f64 / trials as f64;
            assert!((p - w[i] / 10.0).abs() < 0.015, "{i}: {p}");
        }
    }

    #[test]
    fn weighted_sampling_inclusion_follows_weights() {
        // Draw 100 of 1000; items with weight 10 must be included far more often than weight 1.
        let w: Vec<f64> = (0..1000)
            .map(|i| if i < 100 { 10.0 } else { 1.0 })
            .collect();
        let s = sample_weighted_indices(&w, 100, &mut Rng::new(9));
        let heavy = s.iter().filter(|&&i| i < 100).count();
        // Expected ≈ 100·(1000/1900) ≈ 50 in the first draw; inclusion of heavy ones ≈ 40–60.
        assert!((35..=70).contains(&heavy), "{heavy}");
    }

    #[test]
    fn null_presences_draws_from_eligible() {
        let eligible: Vec<usize> = (100..200).collect();
        let s = null_presences(&eligible, None, 30, &mut Rng::new(1));
        assert_eq!(s.len(), 30);
        assert!(s.iter().all(|i| (100..200).contains(i)));
        let mut d = s.clone();
        d.sort_unstable();
        d.dedup();
        assert_eq!(d.len(), 30);
        // Weighted: only the first five eligible elements have weight.
        let w = [1.0; 5];
        let s = null_presences(&eligible, Some(&w), 30, &mut Rng::new(1));
        let mut s2 = s.clone();
        s2.sort_unstable();
        assert_eq!(s2, vec![100, 101, 102, 103, 104]);
        assert_eq!(
            null_presences(&[], None, 3, &mut Rng::new(1)),
            Vec::<usize>::new()
        );
    }
}
