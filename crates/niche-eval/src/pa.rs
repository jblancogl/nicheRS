//! Presence–absence metrics for independent validation data (Allouche, Tsoar & Kadmon 2006,
//! J. Appl. Ecol. 43:1223–1232; Leroy et al. 2018, J. Biogeogr. 45:1994–2002).
//!
//! These need *true absences* (surveyed and not found). With background or pseudo-absences they
//! are not valid: specificity, TSS and kappa then measure discrimination from the landscape, not
//! from absences. Leroy et al. 2018 show TSS is still sensitive to prevalence when both errors
//! are not balanced and recommend reporting sensitivity and specificity alongside it (and the
//! Sørensen index for presence-only data). Calibration (Brier, binned calibration) only makes
//! sense for probability-like outputs on PA data: MaxEnt cloglog/logistic outputs are relative
//! suitability, not calibrated probabilities, so a calibration table for them describes a
//! transform, not the probability of presence.

use crate::auc;

/// Counts at a threshold: a site is predicted present when its score is ≥ threshold (consistent
/// with [`crate::omission_rate`], which counts presences strictly below the threshold as missed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfusionMatrix {
    pub true_pos: usize,
    pub false_pos: usize,
    pub false_neg: usize,
    pub true_neg: usize,
}

impl ConfusionMatrix {
    /// Confusion matrix of presence scores `pres` and absence scores `abs` at `threshold`.
    /// Non-finite scores are counted as predicted absent.
    pub fn at_threshold(pres: &[f64], abs: &[f64], threshold: f64) -> Self {
        let tp = pres.iter().filter(|&&x| x >= threshold).count();
        let fp = abs.iter().filter(|&&x| x >= threshold).count();
        Self {
            true_pos: tp,
            false_pos: fp,
            false_neg: pres.len() - tp,
            true_neg: abs.len() - fp,
        }
    }

    pub fn n(&self) -> usize {
        self.true_pos + self.false_pos + self.false_neg + self.true_neg
    }

    /// TP / (TP + FN); NaN without presences.
    pub fn sensitivity(&self) -> f64 {
        ratio(self.true_pos, self.true_pos + self.false_neg)
    }

    /// TN / (TN + FP); NaN without absences.
    pub fn specificity(&self) -> f64 {
        ratio(self.true_neg, self.true_neg + self.false_pos)
    }

    /// True skill statistic = sensitivity + specificity − 1 (Allouche et al. 2006).
    pub fn tss(&self) -> f64 {
        self.sensitivity() + self.specificity() - 1.0
    }

    /// Cohen's kappa (observed vs chance agreement). NaN when chance agreement is 1 or n = 0.
    pub fn kappa(&self) -> f64 {
        let n = self.n() as f64;
        if n == 0.0 {
            return f64::NAN;
        }
        let (a, b, c, d) = (
            self.true_pos as f64,
            self.false_pos as f64,
            self.false_neg as f64,
            self.true_neg as f64,
        );
        let po = (a + d) / n;
        let pe = ((a + b) * (a + c) + (c + d) * (b + d)) / (n * n);
        if pe >= 1.0 {
            f64::NAN
        } else {
            (po - pe) / (1.0 - pe)
        }
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 {
        f64::NAN
    } else {
        a as f64 / b as f64
    }
}

/// Threshold-dependent and threshold-free metrics on presence–absence data.
#[derive(Debug, Clone, PartialEq)]
pub struct PaMetrics {
    pub threshold: f64,
    pub confusion: ConfusionMatrix,
    pub sensitivity: f64,
    pub specificity: f64,
    pub tss: f64,
    pub kappa: f64,
    /// Presence–absence AUC (Mann–Whitney, ties ½); unlike the presence-background AUC it can
    /// reach 1.
    pub auc: f64,
    /// Brier score (mean squared difference between score and 0/1 outcome); only meaningful for
    /// scores on [0, 1] read as probabilities.
    pub brier: f64,
    /// Proportion of presences among the evaluated sites.
    pub prevalence: f64,
}

/// All metrics for presence scores `pres` and absence scores `abs` at `threshold`. Metrics that
/// are undefined for the inputs (e.g. no absences) are NaN.
pub fn pa_metrics(pres: &[f64], abs: &[f64], threshold: f64) -> PaMetrics {
    let cm = ConfusionMatrix::at_threshold(pres, abs, threshold);
    let auc = if pres.is_empty() || abs.is_empty() {
        f64::NAN
    } else {
        auc(pres, abs)
    };
    PaMetrics {
        threshold,
        confusion: cm,
        sensitivity: cm.sensitivity(),
        specificity: cm.specificity(),
        tss: cm.tss(),
        kappa: cm.kappa(),
        auc,
        brier: brier_score(pres, abs),
        prevalence: ratio(pres.len(), pres.len() + abs.len()),
    }
}

/// Brier score: mean of (s − 1)² over presences and s² over absences (Brier 1950). NaN without
/// data.
pub fn brier_score(pres: &[f64], abs: &[f64]) -> f64 {
    let n = pres.len() + abs.len();
    if n == 0 {
        return f64::NAN;
    }
    let s: f64 = pres.iter().map(|p| (p - 1.0) * (p - 1.0)).sum::<f64>()
        + abs.iter().map(|a| a * a).sum::<f64>();
    s / n as f64
}

/// One bin of a calibration (reliability) table.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationBin {
    /// Smallest and largest score in the bin.
    pub lower: f64,
    pub upper: f64,
    pub mean_predicted: f64,
    /// Fraction of presences in the bin.
    pub observed_frequency: f64,
    pub n: usize,
}

/// Equal-count calibration table (reliability diagram data): finite scores of presences and
/// absences are pooled, sorted and cut into `n_bins` bins of ≈ equal size; tied scores are never
/// split between bins (a boundary moves to the end of its tie group), so bins may be unequal or
/// fewer than requested. Only meaningful for PA data and probability-like scores (see module
/// docs). Empty input or `n_bins == 0` gives an empty table.
pub fn calibration_table(pres: &[f64], abs: &[f64], n_bins: usize) -> Vec<CalibrationBin> {
    let mut v: Vec<(f64, bool)> = pres
        .iter()
        .map(|&s| (s, true))
        .chain(abs.iter().map(|&s| (s, false)))
        .filter(|(s, _)| s.is_finite())
        .collect();
    if v.is_empty() || n_bins == 0 {
        return Vec::new();
    }
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let n = v.len();
    let mut out = Vec::new();
    let mut start = 0;
    for b in 1..=n_bins {
        if start >= n {
            break;
        }
        let mut end = if b == n_bins {
            n
        } else {
            (n * b).div_ceil(n_bins).max(start + 1)
        };
        while end < n && v[end].0 == v[end - 1].0 {
            end += 1;
        }
        let bin = &v[start..end];
        let m = bin.len() as f64;
        out.push(CalibrationBin {
            lower: bin[0].0,
            upper: bin[bin.len() - 1].0,
            mean_predicted: bin.iter().map(|x| x.0).sum::<f64>() / m,
            observed_frequency: bin.iter().filter(|x| x.1).count() as f64 / m,
            n: bin.len(),
        });
        start = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confusion_and_scores() {
        let pres = [0.9, 0.8, 0.4, 0.7];
        let abs = [0.1, 0.6, 0.2, 0.3, 0.5, 0.05];
        let m = pa_metrics(&pres, &abs, 0.5);
        assert_eq!(
            m.confusion,
            ConfusionMatrix {
                true_pos: 3,
                false_pos: 2,
                false_neg: 1,
                true_neg: 4
            }
        );
        assert!((m.sensitivity - 0.75).abs() < 1e-12);
        assert!((m.specificity - 4.0 / 6.0).abs() < 1e-12);
        assert!((m.tss - (0.75 + 4.0 / 6.0 - 1.0)).abs() < 1e-12);
        // po = 0.7; pe = (5·4 + 5·6)/100 = 0.5; kappa = 0.4.
        assert!((m.kappa - 0.4).abs() < 1e-12);
        assert!((m.prevalence - 0.4).abs() < 1e-12);
        // AUC: pairs where pres > abs: 0.9,0.8,0.7 beat all 6 (18), 0.4 beats 0.1,0.2,0.3,0.05 (4).
        assert!((m.auc - 22.0 / 24.0).abs() < 1e-12);
        let brier = (0.01 + 0.04 + 0.36 + 0.09 + 0.01 + 0.36 + 0.04 + 0.09 + 0.25 + 0.0025) / 10.0;
        assert!((m.brier - brier).abs() < 1e-12);
    }

    #[test]
    fn perfect_and_degenerate() {
        let m = pa_metrics(&[1.0, 1.0], &[0.0, 0.0], 0.5);
        assert_eq!((m.tss, m.kappa, m.auc, m.brier), (1.0, 1.0, 1.0, 0.0));
        let m = pa_metrics(&[0.7], &[], 0.5);
        assert!(m.specificity.is_nan() && m.auc.is_nan() && m.kappa.is_nan());
        assert!(brier_score(&[], &[]).is_nan());
    }

    #[test]
    fn calibration_bins() {
        // Scores 0.05..0.95, presence when index ≥ 10 (perfectly separable).
        let s: Vec<f64> = (0..20).map(|i| 0.025 + i as f64 * 0.05).collect();
        let t = calibration_table(&s[10..], &s[..10], 4);
        assert_eq!(t.len(), 4);
        assert!(t.iter().all(|b| b.n == 5));
        assert_eq!(t[0].observed_frequency, 0.0);
        assert_eq!(t[3].observed_frequency, 1.0);
        assert!(t.windows(2).all(|w| w[0].upper < w[1].lower));
        // Ties are kept in one bin.
        let t = calibration_table(&[0.5, 0.5, 0.5], &[0.5, 0.1, 0.9], 3);
        // Sorted: 0.1, 0.5 ×4, 0.9 — the first boundary (after 2) moves past the 0.5 ties.
        assert_eq!(t.iter().map(|b| b.n).collect::<Vec<_>>(), vec![5, 1]);
        assert_eq!((t[0].upper, t[1].lower), (0.5, 0.9));
        assert!(calibration_table(&[], &[], 5).is_empty());
        assert!(calibration_table(&[0.1], &[], 0).is_empty());
        assert_eq!(calibration_table(&[0.1, f64::NAN], &[], 5).len(), 1);
    }
}
