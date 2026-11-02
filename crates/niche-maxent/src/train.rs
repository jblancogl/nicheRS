//! MaxEnt training: a faithful port of MaxEnt 3.4.4 `FeaturedSpace`, `Sequential`,
//! `HingeFeatureGenerator`, `ThrFeatureGenerator` and the feature set-up in `Runner`.
//!
//! Faithfulness matters more than elegance here. The optimizer is a greedy coordinate-descent
//! whose path depends on bookkeeping details (which feature expectations are refreshed when,
//! the order of floating-point sums, tie-breaking in sorts). Each function cites the Java
//! method it reproduces; deviations are listed in `docs/architecture/mathematical-specification.md`.

// Index loops deliberately mirror the Java source line by line to keep the port auditable.
#![allow(clippy::needless_range_loop)]

use std::collections::HashSet;

use crate::javafmt::{double_to_string, float_to_string};
use crate::javasort::sort_indices;
use crate::model::{FeatureKind, MaxentModel, ModelFeature};
use crate::settings::MaxentSettings;

const EPS: f64 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Continuous,
    Categorical,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    pub name: String,
    pub kind: VarKind,
}

/// Samples-with-data (SWD) training input: every value is present (no NODATA).
#[derive(Debug, Clone)]
pub struct SwdData {
    pub vars: Vec<Variable>,
    /// `background[var][point]`
    pub background: Vec<Vec<f64>>,
    /// `samples[var][sample]` — presence records.
    pub samples: Vec<Vec<f64>>,
}

#[derive(Debug, thiserror::Error)]
pub enum TrainError {
    #[error("no presence samples")]
    NoSamples,
    #[error("no background points")]
    NoBackground,
    #[error("input arrays have inconsistent dimensions")]
    Shape,
    #[error("no features available: select more feature classes or disable auto features")]
    NoFeatures,
    #[error("numerical failure: {0}")]
    Numerical(String),
    #[error("training cancelled")]
    Cancelled,
}

/// Diagnostics of a training run.
#[derive(Debug, Clone)]
pub struct TrainReport {
    pub iterations: usize,
    pub loss: f64,
    /// Regularized training gain, as reported by MaxEnt.
    pub regularized_gain: f64,
    pub unregularized_gain: f64,
    pub num_background: usize,
    pub num_added_samples: usize,
    /// Percent contribution per variable (same order as the model's variables).
    pub percent_contribution: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Proto {
    Binary {
        var: usize,
        target: f32,
        first: bool,
    },
    Linear {
        var: usize,
    },
    Square {
        var: usize,
    },
    Product {
        a: usize,
        b: usize,
    },
    ThrGen {
        var: usize,
    },
    HingeGen {
        var: usize,
        rev: bool,
    },
}

struct Feat {
    kind: FeatureKind,
    name: String,
    pts: Vec<f64>,
    smp: Vec<f64>,
    lambda: f64,
    sample_exp: f64,
    sample_dev: f64,
    expectation: f64,
    beta: f64,
    active: bool,
    generated: bool,
    binary: bool,
    last_change: i64,
    last_exp_update: i64,
    previous_lambda: f64,
    contribution: f64,
    previous_contribution: f64,
    /// Variables this feature depends on (for contributions).
    vars: Vec<usize>,
}

enum GenKind {
    Hinge,
    Threshold,
}

struct Generator {
    kind: GenKind,
    var: usize,
    rev: bool,
    name: String,
    beta: f64,
    pts: Vec<f64>,
    smp: Vec<f64>,
    sorted: Vec<i64>,
    thr: Vec<f64>,
    val_to_thr: Vec<i64>,
    max_val: f64,
    thr_first: i64,
    thr_last: i64,
    sample_exp: Vec<f64>,
    sample_dev: Vec<f64>,
    feat_exp: Vec<f64>,
    exported: Vec<Option<usize>>,
}

impl Generator {
    fn eval(&self, i: i64) -> f64 {
        if i < 0 {
            self.smp[(-i - 1) as usize]
        } else {
            self.pts[i as usize]
        }
    }

    /// `SortedFeatureGenerator` constructor.
    fn new(
        kind: GenKind,
        var: usize,
        rev: bool,
        name: String,
        beta: f64,
        pts: Vec<f64>,
        smp: Vec<f64>,
    ) -> Self {
        let n = pts.len();
        let m = smp.len();
        let mut g = Generator {
            kind,
            var,
            rev,
            name,
            beta,
            pts,
            smp,
            sorted: Vec::new(),
            thr: Vec::new(),
            val_to_thr: Vec::new(),
            max_val: 0.0,
            thr_first: -1,
            thr_last: -1,
            sample_exp: Vec::new(),
            sample_dev: Vec::new(),
            feat_exp: Vec::new(),
            exported: Vec::new(),
        };
        let mut sorted: Vec<i64> = (0..n as i64).chain((0..m as i64).map(|i| -i - 1)).collect();
        // Java: Arrays.sort(Integer[], comparator) is a stable merge sort, as is `sort_by`.
        sorted.sort_by(|&a, &b| {
            let (fa, fb) = (g.eval(a), g.eval(b));
            if fa < fb {
                std::cmp::Ordering::Less
            } else if fa > fb {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        let mut prec = f64::INFINITY;
        for &s in &sorted {
            let p = precision(g.eval(s));
            if p < prec {
                prec = p;
            }
        }
        let num_vals = sorted.len();
        let min_val = g.eval(sorted[0]);
        g.max_val = g.eval(sorted[num_vals - 1]);
        let mut val_to_thr = vec![-1i64; num_vals];
        let mut thr = Vec::new();
        let mut last_val = min_val;
        let mut first_sample: Option<f64> = None;
        let mut last_sample: Option<f64> = None;
        for (i, &s) in sorted.iter().enumerate() {
            let val = g.eval(s);
            if s < 0 {
                last_sample = Some(val);
                if first_sample.is_none() {
                    first_sample = last_sample;
                }
            }
            if val - last_val > prec {
                thr.push((val + last_val) / 2.0);
                last_val = val;
                val_to_thr[i] = thr.len() as i64 - 1;
            }
        }
        let num_thr = thr.len();
        g.sorted = sorted;
        g.thr = thr;
        g.val_to_thr = val_to_thr;
        g.sample_exp = vec![0.0; num_thr];
        g.sample_dev = vec![0.0; num_thr];
        g.feat_exp = vec![0.0; num_thr];
        g.exported = vec![None; num_thr];
        match g.kind {
            GenKind::Hinge => {
                g.thr_first = 0;
                g.thr_last = num_thr as i64;
            }
            GenKind::Threshold => {
                if m > 0 {
                    let fv = first_sample.unwrap_or(min_val);
                    let mut i_thr = 0usize;
                    while i_thr < num_thr && fv > g.thr[i_thr] {
                        i_thr += 1;
                    }
                    g.thr_first = i_thr as i64;
                    let lv = last_sample.unwrap_or(min_val);
                    let mut i_thr = num_thr;
                    while i_thr > 0 && lv <= g.thr[i_thr - 1] {
                        i_thr -= 1;
                    }
                    g.thr_last = i_thr as i64;
                }
            }
        }
        g
    }

    fn in_range(&self, i_thr: i64) -> bool {
        i_thr >= self.thr_first && i_thr < self.thr_last
    }

    /// `HingeFeatureGenerator.setSampleExpectations` / `ThrFeatureGenerator.setSampleExpectations`.
    /// The bias interval is exactly [1, 1] without a bias file, so it is omitted.
    fn set_sample_expectations(&mut self, min_deviation: f64) {
        let num_samples = self.smp.len();
        let ns = num_samples as f64;
        let thr_max = 1.0; // X.biasInfo.max
        let (mut sum1, mut sum2, mut wsum1, mut wsum2) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for i in (0..self.sorted.len()).rev() {
            let s = self.sorted[i];
            match self.kind {
                GenKind::Hinge => {
                    if s < 0 {
                        let lval = 1.0;
                        let val = self.eval(s) * lval;
                        sum1 += val;
                        sum2 += val * val;
                        wsum1 += lval;
                        wsum2 += lval * lval;
                    }
                }
                GenKind::Threshold => {
                    if s < 0 {
                        let val = 1.0;
                        sum1 += val;
                        sum2 += val * val;
                    }
                }
            }
            let i_thr = self.val_to_thr[i];
            if !self.in_range(i_thr) {
                continue;
            }
            let t = self.thr[i_thr as usize];
            let (avg, std);
            match self.kind {
                GenKind::Hinge => {
                    let mut a = thr_max / 2.0;
                    let mut sd = thr_max / 2.0;
                    if num_samples > 0 {
                        let range = self.max_val - t;
                        let csum1 = (sum1 - t * wsum1) / range;
                        let csum2 = (sum2 - 2.0 * sum1 * t + t * t * wsum2) / range / range;
                        a = csum1 / ns;
                        if num_samples > 1 {
                            if csum2 < ns * a * a {
                                sd = 0.0;
                            } else {
                                sd = ((csum2 - ns * a * a) / (ns - 1.0)).sqrt();
                            }
                            if sd > thr_max / 2.0 {
                                sd = thr_max / 2.0;
                            }
                            if sd < thr_max / ns.sqrt() {
                                sd = thr_max / ns.sqrt();
                            }
                        }
                    }
                    avg = a;
                    std = sd;
                }
                GenKind::Threshold => {
                    if num_samples == 0 {
                        avg = thr_max / 2.0;
                        std = thr_max / 2.0;
                    } else if num_samples == 1 {
                        avg = sum1;
                        std = thr_max / 2.0;
                    } else {
                        let a = sum1 / ns;
                        let mut sd = if sum2 < ns * a * a {
                            0.0
                        } else {
                            ((sum2 - ns * a * a) / (ns - 1.0)).sqrt()
                        };
                        if sd > thr_max / 2.0 {
                            sd = thr_max / 2.0;
                        }
                        avg = a;
                        std = sd;
                    }
                }
            }
            let (low, high) = interval(avg, std, num_samples, 0.0, thr_max, self.beta);
            let k = i_thr as usize;
            self.sample_exp[k] = 0.5 * (low + high);
            self.sample_dev[k] = 0.5 * (high - low);
            if self.sample_dev[k] < min_deviation {
                self.sample_dev[k] = min_deviation;
            }
        }
    }

    /// `updateFeatureExpectations` of both generator kinds.
    fn update_feature_expectations(&mut self, density: &[f64], dn: f64, features: &mut [Feat]) {
        let (mut sum, mut wsum) = (0.0f64, 0.0f64);
        for i in (0..self.sorted.len()).rev() {
            let s = self.sorted[i];
            if s >= 0 {
                let d = density[s as usize];
                match self.kind {
                    GenKind::Hinge => {
                        sum += self.pts[s as usize] * d;
                        wsum += d;
                    }
                    GenKind::Threshold => sum += d,
                }
            }
            let i_thr = self.val_to_thr[i];
            if self.in_range(i_thr) {
                let k = i_thr as usize;
                self.feat_exp[k] = match self.kind {
                    GenKind::Hinge => {
                        (sum - self.thr[k] * wsum) / dn / (self.max_val - self.thr[k])
                    }
                    GenKind::Threshold => sum / dn,
                };
            }
        }
        for k in self.thr_first.max(0) as usize..self.thr_last.max(0) as usize {
            if let Some(fi) = self.exported[k] {
                features[fi].expectation = self.feat_exp[k];
            }
        }
    }
}

/// `FeaturedSpace.Interval(SampleInfo, beta)` divided by the bias interval [1, 1].
fn interval(avg: f64, std: f64, cnt: usize, min: f64, max: f64, beta: f64) -> (f64, f64) {
    if cnt == 0 {
        (min / 1.0, max / 1.0)
    } else {
        let d = beta / (cnt as f64).sqrt() * std;
        ((avg - d) / 1.0, (avg + d) / 1.0)
    }
}

/// `SortedFeatureGenerator.precision`.
fn precision(f: f64) -> f64 {
    const MAX_DIGITS: i32 = 6;
    let mut f = f.abs();
    let first_sig = (f.ln() / 10f64.ln()).floor();
    let mut current_power = 10f64.powf(first_sig);
    let mut last_two: i32 = 0;
    for i in 1..=MAX_DIGITS {
        let current_digit = (f / current_power) as i32;
        last_two = (last_two.wrapping_mul(10).wrapping_add(current_digit)) % 100;
        if i >= 2 && matches!(last_two, 0 | 1 | 99 | 98) {
            return current_power * 50.0;
        }
        f -= f64::from(current_digit) * current_power;
        current_power /= 10.0;
    }
    current_power * 5.0
}

/// `Runner.interpolate`.
fn interpolate(x: &[i64], y: &[f64], xx: i64) -> f64 {
    let mut i = 0;
    while i < x.len() && xx > x[i] {
        i += 1;
    }
    if i == 0 {
        y[0]
    } else if i == x.len() {
        y[x.len() - 1]
    } else {
        y[i - 1] + (y[i] - y[i - 1]) * (xx - x[i - 1]) as f64 / (x[i] - x[i - 1]) as f64
    }
}

fn scaled(raw: &[f64], raw_smp: &[f64]) -> (Vec<f64>, Vec<f64>, f64, f64) {
    let mut min = raw[0];
    let mut max = raw[0];
    for &v in &raw[1..] {
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    let mut scale = max - min;
    if scale == 0.0 {
        scale = 1.0;
    }
    let pts = raw.iter().map(|v| (v - min) / scale).collect();
    let smp = raw_smp.iter().map(|v| (v - min) / scale).collect();
    (pts, smp, min, max)
}

fn new_feat(
    kind: FeatureKind,
    name: String,
    pts: Vec<f64>,
    smp: Vec<f64>,
    beta: f64,
    vars: Vec<usize>,
) -> Feat {
    let binary = matches!(
        kind,
        FeatureKind::Binary { .. } | FeatureKind::Threshold { .. }
    );
    Feat {
        kind,
        name,
        pts,
        smp,
        lambda: 0.0,
        sample_exp: 0.0,
        sample_dev: 0.0,
        expectation: 0.0,
        beta,
        active: true,
        generated: false,
        binary,
        last_change: -1,
        last_exp_update: -1,
        previous_lambda: 0.0,
        contribution: 0.0,
        previous_contribution: 0.0,
        vars,
    }
}

struct Trainer<'a> {
    settings: &'a MaxentSettings,
    n: usize,
    lp: Vec<f64>,
    lpn: f64,
    density: Vec<f64>,
    dn: f64,
    features: Vec<Feat>,
    gens: Vec<Generator>,
    min_deviation: f64,
    // Sequential state
    iteration: i64,
    reg: f64,
    old_loss: f64,
    previous_loss: f64,
    fatal: Option<String>,
}

/// Cooperative cancellation hook, polled once per optimizer iteration.
pub trait CancelCheck {
    fn is_cancelled(&self) -> bool;
}

impl CancelCheck for () {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Train a MaxEnt model reproducing MaxEnt 3.4.4 (`density.MaxEnt`) on SWD input.
pub fn train(
    data: &SwdData,
    settings: &MaxentSettings,
    cancel: &dyn CancelCheck,
) -> Result<(MaxentModel, TrainReport), TrainError> {
    let nv = data.vars.len();
    if data.background.len() != nv || data.samples.len() != nv || nv == 0 {
        return Err(TrainError::Shape);
    }
    let n_bg = data.background[0].len();
    let m = data.samples[0].len();
    if data.background.iter().any(|c| c.len() != n_bg) || data.samples.iter().any(|c| c.len() != m)
    {
        return Err(TrainError::Shape);
    }
    if m == 0 {
        return Err(TrainError::NoSamples);
    }
    if n_bg == 0 {
        return Err(TrainError::NoBackground);
    }

    // MaxEnt orders layers by name (java.lang.String ordering; byte order for ASCII names).
    let mut order: Vec<usize> = (0..nv).collect();
    order.sort_by(|&a, &b| data.vars[a].name.cmp(&data.vars[b].name));
    let vars: Vec<Variable> = order.iter().map(|&i| data.vars[i].clone()).collect();
    let mut pts_raw: Vec<Vec<f64>> = order.iter().map(|&i| data.background[i].clone()).collect();
    let smp_raw: Vec<Vec<f64>> = order.iter().map(|&i| data.samples[i].clone()).collect();

    // Runner.featuresWithSamples: add samples whose environmental vector is not in the
    // background. Java detects this with a random-projection hash; exact vector matching gives
    // the same answer barring hash collisions.
    let mut added = 0usize;
    if settings.add_samples_to_background {
        let key = |cols: &Vec<Vec<f64>>, i: usize| -> Vec<u64> {
            cols.iter().map(|c| c[i].to_bits()).collect()
        };
        let bg: HashSet<Vec<u64>> = (0..n_bg).map(|i| key(&pts_raw, i)).collect();
        let to_add: Vec<usize> = (0..m)
            .filter(|&s| settings.add_all_samples_to_background || !bg.contains(&key(&smp_raw, s)))
            .collect();
        for &s in &to_add {
            for v in 0..nv {
                let val = smp_raw[v][s];
                pts_raw[v].push(val);
            }
        }
        added = to_add.len();
    }
    let n = n_bg + added;

    // Runner.makeFeatures
    let mut protos: Vec<Proto> = Vec::new();
    let mut cont: Vec<usize> = Vec::new();
    for (v, var) in vars.iter().enumerate() {
        match var.kind {
            VarKind::Categorical => {
                let mut vals: Vec<f64> = pts_raw[v].clone();
                vals.sort_by(|a, b| a.total_cmp(b));
                vals.dedup();
                for (k, &val) in vals.iter().enumerate() {
                    protos.push(Proto::Binary {
                        var: v,
                        target: val as f32,
                        first: k == 0,
                    });
                }
            }
            VarKind::Continuous => cont.push(v),
        }
    }
    let fc = settings.features;
    for &v in &cont {
        protos.push(Proto::Linear { var: v });
    }
    if fc.quadratic {
        for &v in &cont {
            protos.push(Proto::Square { var: v });
        }
    }
    if fc.product {
        let is_mask = |v: usize| pts_raw[v].iter().all(|&x| x == pts_raw[v][0]);
        for (i, &a) in cont.iter().enumerate() {
            for &b in &cont[i + 1..] {
                if !is_mask(a) && !is_mask(b) {
                    protos.push(Proto::Product { a, b });
                }
            }
        }
    }
    if fc.threshold {
        for &v in &cont {
            protos.push(Proto::ThrGen { var: v });
        }
    }
    if fc.hinge {
        for &v in &cont {
            protos.push(Proto::HingeGen { var: v, rev: false });
            protos.push(Proto::HingeGen { var: v, rev: true });
        }
    }

    // Runner.autoSetBeta
    let mi = m as i64;
    let auto = settings.auto_features;
    let (thresholds, betas): (&[i64], &[f64]) =
        if fc.product && (!auto || m >= settings.lq2lqpt_threshold) {
            (&[0, 10, 17, 30, 100], &[2.6, 1.6, 0.9, 0.55, 0.05])
        } else if fc.quadratic && (!auto || m >= settings.l2lq_threshold) {
            (&[0, 10, 17, 30, 100], &[1.3, 0.8, 0.5, 0.25, 0.05])
        } else {
            (&[10, 30, 100], &[1.0, 0.2, 0.05])
        };
    let beta_lqp = settings
        .beta_lqp
        .unwrap_or_else(|| interpolate(thresholds, betas, mi));
    let beta_thr = settings
        .beta_threshold
        .unwrap_or_else(|| interpolate(&[0, 100], &[2.0, 1.0], mi));
    let beta_hge = settings.beta_hinge.unwrap_or(0.5);
    let beta_cat = settings
        .beta_categorical
        .unwrap_or_else(|| interpolate(&[0, 10, 17], &[0.65, 0.5, 0.25], mi));
    let bm = settings.beta_multiplier;
    let beta_of = |p: &Proto| match p {
        Proto::Binary { .. } => beta_cat * bm,
        Proto::ThrGen { .. } => beta_thr * bm,
        Proto::HingeGen { .. } => beta_hge * bm,
        _ => beta_lqp * bm,
    };
    // Runner.autoSetActive and the linear-only-for-clamping rule in maxentRun.
    let active_of = |p: &Proto| -> bool {
        match p {
            Proto::ThrGen { .. } | Proto::Product { .. }
                if auto && m < settings.lq2lqpt_threshold =>
            {
                false
            }
            Proto::HingeGen { .. } if auto && m < settings.hinge_threshold => false,
            Proto::Square { .. } if auto && m < settings.l2lq_threshold => false,
            Proto::Linear { .. } if !fc.linear => false,
            _ => true,
        }
    };
    if !protos.iter().any(active_of) {
        return Err(TrainError::NoFeatures);
    }

    let mut features: Vec<Feat> = Vec::new();
    let mut gens: Vec<Generator> = Vec::new();
    for p in &protos {
        let active = active_of(p);
        if !active && !matches!(p, Proto::Linear { .. }) {
            continue;
        }
        let beta = beta_of(p);
        let name_of = |v: usize| vars[v].name.clone();
        match *p {
            Proto::Binary { var, target, first } => {
                let t = f64::from(target);
                let pts = pts_raw[var]
                    .iter()
                    .map(|&x| if x == t { 1.0 } else { 0.0 })
                    .collect();
                let smp = smp_raw[var]
                    .iter()
                    .map(|&x| if x == t { 1.0 } else { 0.0 })
                    .collect();
                let name = format!("({}={})", name_of(var), float_to_string(target));
                let kind = FeatureKind::Binary {
                    var: name_of(var),
                    value: t,
                    first_category: first,
                };
                features.push(new_feat(kind, name, pts, smp, beta, vec![var]));
            }
            Proto::Linear { var } => {
                let (pts, smp, min, max) = scaled(&pts_raw[var], &smp_raw[var]);
                let kind = FeatureKind::Linear {
                    var: name_of(var),
                    min,
                    max,
                };
                let mut f = new_feat(kind, name_of(var), pts, smp, beta, vec![var]);
                f.active = active;
                features.push(f);
            }
            Proto::Square { var } => {
                let rp: Vec<f64> = pts_raw[var].iter().map(|x| x * x).collect();
                let rs: Vec<f64> = smp_raw[var].iter().map(|x| x * x).collect();
                let (pts, smp, min, max) = scaled(&rp, &rs);
                let kind = FeatureKind::Quadratic {
                    var: name_of(var),
                    min,
                    max,
                };
                features.push(new_feat(
                    kind,
                    format!("{}^2", name_of(var)),
                    pts,
                    smp,
                    beta,
                    vec![var],
                ));
            }
            Proto::Product { a, b } => {
                let rp: Vec<f64> = pts_raw[a]
                    .iter()
                    .zip(&pts_raw[b])
                    .map(|(x, y)| x * y)
                    .collect();
                let rs: Vec<f64> = smp_raw[a]
                    .iter()
                    .zip(&smp_raw[b])
                    .map(|(x, y)| x * y)
                    .collect();
                let (pts, smp, min, max) = scaled(&rp, &rs);
                let kind = FeatureKind::Product {
                    a: name_of(a),
                    b: name_of(b),
                    min,
                    max,
                };
                features.push(new_feat(
                    kind,
                    format!("{}*{}", name_of(a), name_of(b)),
                    pts,
                    smp,
                    beta,
                    vec![a, b],
                ));
            }
            Proto::ThrGen { var } => {
                gens.push(Generator::new(
                    GenKind::Threshold,
                    var,
                    false,
                    name_of(var),
                    beta,
                    pts_raw[var].clone(),
                    smp_raw[var].clone(),
                ));
            }
            Proto::HingeGen { var, rev } => {
                let neg = |v: &Vec<f64>| -> Vec<f64> {
                    v.iter().map(|x| if rev { -x } else { *x }).collect()
                };
                let name = if rev {
                    format!("{}__rev", name_of(var))
                } else {
                    name_of(var)
                };
                gens.push(Generator::new(
                    GenKind::Hinge,
                    var,
                    rev,
                    name,
                    beta,
                    neg(&pts_raw[var]),
                    neg(&smp_raw[var]),
                ));
            }
        }
    }

    let mut t = Trainer {
        settings,
        n,
        lp: vec![0.0; n],
        lpn: 0.0,
        density: vec![0.0; n],
        dn: 0.0,
        features,
        gens,
        min_deviation: 0.001 * bm,
        iteration: 0,
        reg: 0.0,
        old_loss: 0.0,
        previous_loss: f64::INFINITY,
        fatal: None,
    };
    t.set_sample_expectations();
    t.set_linear_predictor_normalizer();
    let init: Vec<usize> = (0..t.features.len())
        .filter(|&j| t.features[j].active && !t.features[j].generated)
        .collect();
    t.set_density(&init);
    for f in &mut t.features {
        if f.binary && f.sample_exp == 0.0 {
            f.active = false;
        }
    }
    // Sequential constructor
    for f in &mut t.features {
        f.last_change = -1;
        f.last_exp_update = -1;
        f.previous_lambda = f.lambda;
    }
    t.set_reg();
    let loss = t.run(cancel)?;
    if let Some(msg) = t.fatal.take() {
        return Err(TrainError::Numerical(msg));
    }

    let gain = (t.n as f64).ln() - loss;
    let l1 = t.get_l1_reg();
    let entropy = t.entropy();
    let mut contrib = vec![0.0f64; vars.len()];
    for f in &t.features {
        if f.contribution <= 0.0 {
            continue;
        }
        for &v in &f.vars {
            contrib[v] += f.contribution / f.vars.len() as f64;
        }
    }
    let sum: f64 = contrib.iter().sum();
    let percent_contribution = vars
        .iter()
        .zip(&contrib)
        .map(|(v, c)| {
            (
                v.name.clone(),
                if sum == 0.0 { 0.0 } else { c * 100.0 / sum },
            )
        })
        .collect();

    let model = MaxentModel {
        features: t
            .features
            .iter()
            .map(|f| ModelFeature {
                name: f.name.clone(),
                kind: f.kind.clone(),
                lambda: f.lambda,
            })
            .collect(),
        linear_predictor_normalizer: t.lpn,
        density_normalizer: t.dn,
        num_background_points: t.n,
        entropy,
        variables: vars.iter().map(|v| (v.name.clone(), v.kind)).collect(),
    };
    let report = TrainReport {
        iterations: t.iteration.max(0) as usize,
        loss,
        regularized_gain: gain,
        unregularized_gain: gain + l1,
        num_background: n,
        num_added_samples: added,
        percent_contribution,
    };
    Ok((model, report))
}

impl Trainer<'_> {
    // ---- FeaturedSpace -------------------------------------------------------------------

    fn set_sample_expectations(&mut self) {
        for f in &mut self.features {
            // getDividedSampleInfo(f, const 1)
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for &v in &f.pts {
                if v < min {
                    min = v;
                }
                if v > max {
                    max = v;
                }
            }
            let cnt = f.smp.len();
            let (mut avg, mut std) = (0.0f64, 0.0f64);
            for &v in &f.smp {
                avg += v;
                std += v * v;
            }
            if cnt == 0 {
                avg = (min + max) / 2.0;
                std = 0.5 * (max - min);
            } else if cnt == 1 {
                std = 0.5 * (max - min);
            } else {
                let c = cnt as f64;
                avg /= c;
                std = if std < c * avg * avg {
                    0.0
                } else {
                    ((std - c * avg * avg) / (c - 1.0)).sqrt()
                };
                if std > 0.5 * (max - min) {
                    std = 0.5 * (max - min);
                }
            }
            let (low, high) = interval(avg, std, cnt, min, max, f.beta);
            f.sample_exp = 0.5 * (low + high);
            f.sample_dev = 0.5 * (high - low);
            if f.sample_dev < self.min_deviation {
                if f.binary
                    && !matches!(f.kind, FeatureKind::Threshold { .. })
                    && f.sample_exp == 1.0
                    && cnt > 0
                {
                    f.sample_dev = 1.0 / (2.0 * cnt as f64);
                } else {
                    f.sample_dev = self.min_deviation;
                }
            }
            if cnt == 0 {
                f.active = false;
            }
        }
        let md = self.min_deviation;
        for g in &mut self.gens {
            g.set_sample_expectations(md);
        }
    }

    fn set_linear_predictor_normalizer(&mut self) {
        self.lpn = if self.n == 0 { 0.0 } else { self.lp[0] };
        for i in 1..self.n {
            if self.lp[i] > self.lpn {
                self.lpn = self.lp[i];
            }
        }
    }

    /// `FeaturedSpace.setDensity(Feature[] toUpdate)`.
    fn set_density(&mut self, to_update: &[usize]) {
        let lpn = self.lpn;
        let mut dn = 0.0f64;
        for (d, &l) in self.density.iter_mut().zip(&self.lp) {
            *d = 1.0 * (l - lpn).exp();
            dn += *d;
        }
        self.dn = dn;
        for &j in to_update {
            let f = &mut self.features[j];
            let mut s = 0.0f64;
            for (d, v) in self.density.iter().zip(&f.pts) {
                s += d * v;
            }
            f.expectation = s / dn;
        }
        for g in &mut self.gens {
            g.update_feature_expectations(&self.density, dn, &mut self.features);
        }
    }

    fn get_n1(&self) -> f64 {
        let mut sum = 0.0f64;
        for f in &self.features {
            sum += f.lambda * f.sample_exp;
        }
        -sum + self.lpn
    }

    fn get_l1_reg(&self) -> f64 {
        let mut r = 0.0f64;
        for f in &self.features {
            r += f.lambda.abs() * f.sample_dev;
        }
        r
    }

    /// `FeaturedSpace.increaseLambda(Feature h, double alpha, Feature[] toUpdate)`.
    fn x_increase_lambda_one(&mut self, h: usize, alpha: f64, to_update: &[usize]) {
        if alpha == 0.0 {
            return;
        }
        self.features[h].lambda += alpha;
        let pts = &self.features[h].pts;
        for i in 0..self.n {
            self.lp[i] += alpha * pts[i];
            if i == 0 || self.lp[i] > self.lpn {
                self.lpn = self.lp[i];
            }
        }
        self.set_density(to_update);
    }

    /// `FeaturedSpace.increaseLambda(double[] alpha, Feature[] toUpdate)`.
    fn x_increase_lambda_vec(&mut self, alpha: &[f64], to_update: &[usize]) {
        for (j, &a) in alpha.iter().enumerate() {
            if a == 0.0 {
                continue;
            }
            self.features[j].lambda += a;
            let pts = &self.features[j].pts;
            for (l, v) in self.lp.iter_mut().zip(pts) {
                *l += a * v;
            }
        }
        self.set_linear_predictor_normalizer();
        self.set_density(to_update);
    }

    fn entropy(&self) -> f64 {
        let mut e = 0.0f64;
        for &d in &self.density {
            let d = d / self.dn;
            if d > 0.0 {
                e += -d * d.ln();
            }
        }
        e
    }

    // ---- Sequential ----------------------------------------------------------------------

    fn get_loss(&self) -> f64 {
        self.get_n1() + self.dn.ln() + self.reg
    }

    fn set_reg(&mut self) {
        self.reg = self.get_l1_reg();
    }

    fn good_alpha_raw(&mut self, n1: f64, w1: f64, lambda: f64, beta1: f64) -> f64 {
        let w0 = 1.0 - w1;
        let n0 = 1.0 - n1;
        if w0 < EPS || w1 < EPS {
            return 0.0;
        }
        let mut alpha = 0.0;
        let pos = n1 - beta1 > EPS && {
            alpha = ((n1 - beta1) * w0 / ((n0 + beta1) * w1)).ln();
            alpha + lambda > 0.0
        };
        let neg = !pos && n0 - beta1 > EPS && {
            alpha = ((n1 + beta1) * w0 / ((n0 - beta1) * w1)).ln();
            alpha + lambda < 0.0
        };
        if !pos && !neg {
            alpha = -lambda;
        }
        if alpha.is_nan() || alpha.is_infinite() {
            self.fatal.get_or_insert_with(|| {
                format!("goodAlpha returned {alpha} for pi[f]={n1}+-{beta1}, q[f]={w1}")
            });
            return 0.0;
        }
        alpha
    }

    fn delta_loss_bound_raw(
        &mut self,
        active: bool,
        n1: f64,
        w1: f64,
        lambda: f64,
        beta1: f64,
    ) -> f64 {
        if !active {
            return 0.0;
        }
        if n1 == -1.0 {
            return 0.0;
        }
        let w0 = 1.0 - w1;
        let alpha = self.good_alpha_raw(n1, w1, lambda, beta1);
        if alpha.is_infinite() {
            return 0.0;
        }
        let bound = -n1 * alpha
            + (w0 + w1 * alpha.exp()).ln()
            + beta1 * ((lambda + alpha).abs() - lambda.abs());
        if bound.is_nan() { 0.0 } else { bound }
    }

    fn delta_loss_bound(&mut self, j: usize) -> f64 {
        let f = &self.features[j];
        let (a, n1, w1, l, b) = (
            f.active,
            f.sample_exp,
            f.expectation,
            f.lambda,
            f.sample_dev,
        );
        self.delta_loss_bound_raw(a, n1, w1, l, b)
    }

    fn good_alpha(&mut self, j: usize) -> f64 {
        let f = &self.features[j];
        let (n1, w1, l, b) = (f.sample_exp, f.expectation, f.lambda, f.sample_dev);
        self.good_alpha_raw(n1, w1, l, b)
    }

    fn get_best_feature(&mut self) -> Option<usize> {
        let mut best_lb = 1.0;
        let mut best: Option<usize> = None;
        for j in 0..self.features.len() {
            if self.features[j].active && !self.features[j].generated {
                let lb = self.delta_loss_bound(j);
                if lb < best_lb {
                    best = Some(j);
                    best_lb = lb;
                }
            }
        }
        let mut best_gen: Option<(usize, usize)> = None;
        for g in 0..self.gens.len() {
            let (first, last) = (self.gens[g].thr_first, self.gens[g].thr_last);
            for k in first.max(0)..last.max(0) {
                let k = k as usize;
                let gn = &self.gens[g];
                let lambda = gn.exported[k].map_or(0.0, |fi| self.features[fi].lambda);
                let (n1, w1, b) = (gn.sample_exp[k], gn.feat_exp[k], gn.sample_dev[k]);
                let lb = self.delta_loss_bound_raw(true, n1, w1, lambda, b);
                if lb < best_lb {
                    best_gen = Some((g, k));
                    best_lb = lb;
                }
            }
        }
        if let Some((g, k)) = best_gen {
            best = Some(match self.gens[g].exported[k] {
                Some(fi) => fi,
                None => self.export_feature(g, k),
            });
        }
        best
    }

    fn export_feature(&mut self, g: usize, k: usize) -> usize {
        let gn = &self.gens[g];
        let t = gn.thr[k];
        let var_name = gn.name.trim_end_matches("__rev").to_string();
        let (kind, name, pts, smp) = match gn.kind {
            GenKind::Hinge => {
                let (min, max) = (t, gn.max_val);
                let range = max - min;
                let ev = |d: f64| if d > min { (d - min) / range } else { 0.0 };
                let kind = if gn.rev {
                    // Written to .lambdas as "`var, lambda, -max, -min" of the reversed variable.
                    FeatureKind::Hinge {
                        var: var_name,
                        reverse: true,
                        min: -max,
                        max: -min,
                    }
                } else {
                    FeatureKind::Hinge {
                        var: var_name,
                        reverse: false,
                        min,
                        max,
                    }
                };
                (
                    kind,
                    format!("'{}", gn.name),
                    gn.pts.iter().map(|&d| ev(d)).collect(),
                    gn.smp.iter().map(|&d| ev(d)).collect(),
                )
            }
            GenKind::Threshold => {
                let ev = |d: f64| if d > t { 1.0 } else { 0.0 };
                let name = format!("({}<{})", double_to_string(t), gn.name);
                (
                    FeatureKind::Threshold {
                        var: var_name,
                        threshold: t,
                    },
                    name,
                    gn.pts.iter().map(|&d| ev(d)).collect(),
                    gn.smp.iter().map(|&d| ev(d)).collect(),
                )
            }
        };
        let mut f = new_feat(kind, name, pts, smp, gn.beta, vec![gn.var]);
        f.generated = true;
        f.sample_exp = gn.sample_exp[k];
        f.sample_dev = gn.sample_dev[k];
        f.expectation = gn.feat_exp[k];
        self.features.push(f);
        let idx = self.features.len() - 1;
        self.gens[g].exported[k] = Some(idx);
        idx
    }

    /// `Sequential.newLossChange(h, alpha)`.
    fn new_loss_change(&self, h: usize, alpha: f64) -> f64 {
        let f = &self.features[h];
        let mut z = 0.0f64;
        for (d, v) in self.density.iter().zip(&f.pts) {
            z += d * (alpha * v).exp();
        }
        -alpha * f.sample_exp + z.ln() + ((f.lambda + alpha).abs() - f.lambda.abs()) * f.sample_dev
    }

    fn search_alpha(&self, h: usize, mut alpha: f64) -> f64 {
        let mut current = self.new_loss_change(h, alpha);
        loop {
            let try_loss = self.new_loss_change(h, alpha * 4.0);
            if try_loss >= current {
                break;
            }
            if try_loss.is_nan() || try_loss.is_infinite() {
                break;
            }
            current = try_loss;
            alpha *= 4.0;
        }
        let try_loss = self.new_loss_change(h, alpha * 2.0);
        if try_loss < current && !try_loss.is_nan() && !try_loss.is_infinite() {
            alpha *= 2.0;
        }
        alpha
    }

    fn deriv(&self, j: usize) -> f64 {
        let f = &self.features[j];
        let unreg = f.expectation - f.sample_exp;
        let wb = f.sample_dev;
        if f.lambda > 0.0 {
            unreg + wb
        } else if f.lambda < 0.0 {
            unreg - wb
        } else if unreg + wb > 0.0 {
            unreg + wb
        } else if unreg - wb < 0.0 {
            unreg - wb
        } else {
            0.0
        }
    }

    /// `Sequential.newtonStep(double[] u)`.
    fn newton_step_dir(&mut self, u: &[f64]) -> f64 {
        let h: Vec<usize> = (0..self.features.len()).filter(|&j| u[j] != 0.0).collect();
        let mut sum = vec![0.0f64; h.len()];
        let (mut uthu, mut uty) = (0.0f64, 0.0f64);
        for i in 0..self.n {
            let mut ftu = 0.0f64;
            let d = self.density[i];
            for (k, &j) in h.iter().enumerate() {
                let val = self.features[j].pts[i];
                ftu += u[j] * val;
                sum[k] += d * val;
            }
            uthu += d * ftu * ftu;
            uty += d * ftu;
        }
        for (k, &j) in h.iter().enumerate() {
            self.features[j].expectation = sum[k] / self.dn;
            self.features[j].last_exp_update = self.iteration;
        }
        uty /= self.dn;
        uthu = uthu / self.dn - uty * uty;
        if uthu < EPS * EPS {
            return 0.0;
        }
        let mut xty = 0.0f64;
        for (j, &uj) in u.iter().enumerate() {
            xty += self.deriv(j) * uj;
        }
        let mut step = -xty / uthu;
        for (j, &uj) in u.iter().enumerate() {
            let l = self.features[j].lambda;
            if (step * uj + l) * l < 0.0 {
                step = -l / uj;
            }
        }
        step
    }

    /// `Sequential.newtonStep(Feature h)`.
    fn newton_step(&self, h: usize) -> f64 {
        let f = &self.features[h];
        let uty = f.expectation;
        let mut uthu = 0.0f64;
        for (d, v) in self.density.iter().zip(&f.pts) {
            uthu += d * v * v;
        }
        uthu = uthu / self.dn - uty * uty;
        if uthu < EPS * EPS {
            return 0.0;
        }
        let mut step = -self.deriv(h) / uthu;
        let l = f.lambda;
        if (step + l) * l < 0.0 {
            step = -l;
        }
        step
    }

    fn features_to_update(&mut self) -> Vec<usize> {
        const UPDATE_CYCLE: i64 = 20;
        const RECENT_CHANGE: i64 = 10;
        const TOP_SELECT: usize = 5;
        let nf = self.features.len();
        let mut to_update = Vec::new();
        let mut dlb = vec![0.0f64; nf];
        for j in 0..nf {
            let f = &self.features[j];
            if f.active
                && !f.generated
                && (self.iteration < f.last_change + RECENT_CHANGE
                    || (j as i64 % UPDATE_CYCLE == self.iteration % UPDATE_CYCLE))
            {
                to_update.push(j);
            }
            dlb[j] = self.delta_loss_bound(j);
        }
        let ordered = sort_indices(&dlb);
        for &j in ordered.iter().take(TOP_SELECT) {
            let f = &self.features[j];
            if f.active && !f.generated && !to_update.contains(&j) {
                to_update.push(j);
            }
        }
        to_update
    }

    fn set_feature_expectation(&mut self, h: usize) {
        let f = &self.features[h];
        let mut s = 0.0f64;
        for (d, v) in self.density.iter().zip(&f.pts) {
            s += d * v;
        }
        let e = s / self.dn;
        let it = self.iteration;
        let f = &mut self.features[h];
        f.expectation = e;
        f.last_exp_update = it;
    }

    fn mark_updated(&mut self, to_update: &[usize]) {
        for &j in to_update {
            self.features[j].last_exp_update = self.iteration;
        }
    }

    /// `Sequential.increaseLambda(double[] alpha, Feature[] toUpdate)`.
    fn increase_lambda_vec(&mut self, alpha: &[f64], to_update: &[usize]) -> f64 {
        self.x_increase_lambda_vec(alpha, to_update);
        self.mark_updated(to_update);
        self.set_reg();
        self.get_loss()
    }

    /// `Sequential.increaseLambda(Feature h, double alpha, Feature[] toUpdate)`.
    fn increase_lambda_one(&mut self, h: usize, alpha: f64, to_update: &[usize]) -> f64 {
        let f = &self.features[h];
        self.reg += ((f.lambda + alpha).abs() - f.lambda.abs()) * f.sample_dev;
        self.x_increase_lambda_one(h, alpha, to_update);
        self.mark_updated(to_update);
        self.get_loss()
    }

    fn do_parallel_update(&mut self) -> f64 {
        let nf = self.features.len();
        let mut alpha = vec![0.0f64; nf];
        let mut contribdelta = vec![0.0f64; nf];
        for j in 0..nf {
            let f = &mut self.features[j];
            alpha[j] = if f.binary || f.lambda == 0.0 {
                0.0
            } else {
                f.lambda - f.previous_lambda
            };
            f.previous_lambda = f.lambda;
            contribdelta[j] = f.contribution - f.previous_contribution;
            if contribdelta[j] < 0.0 {
                contribdelta[j] = 0.0;
            }
            f.previous_contribution = f.contribution;
        }
        let step = self.newton_step_dir(&alpha);
        for j in 0..nf {
            let l = self.features[j].lambda;
            alpha[j] *= step;
            if alpha[j] != -l && (alpha[j] + l).abs() < EPS {
                alpha[j] = -l;
            }
        }
        let loss_was = self.get_loss();
        let to_update = self.features_to_update();
        let mut loss_now = self.increase_lambda_vec(&alpha, &to_update);
        if loss_now > loss_was {
            for a in &mut alpha {
                *a = -*a;
            }
            loss_now = self.increase_lambda_vec(&alpha, &to_update);
        } else {
            let delta = loss_was - loss_now;
            let mut contribsum = 0.0f64;
            for j in 0..nf {
                if alpha[j] != 0.0 {
                    contribsum += contribdelta[j];
                }
            }
            for j in 0..nf {
                if alpha[j] != 0.0 && contribsum > 0.0 {
                    self.features[j].contribution += delta * contribdelta[j] / contribsum;
                }
            }
        }
        loss_now
    }

    fn reduce_alpha(&self, alpha: f64) -> f64 {
        if self.iteration < 10 {
            alpha / 50.0
        } else if self.iteration < 20 {
            alpha / 10.0
        } else if self.iteration < 50 {
            alpha / 3.0
        } else {
            alpha
        }
    }

    fn do_sequential_update(&mut self, h: usize) -> f64 {
        let old_loss = self.old_loss;
        let mut new_loss;
        let old_lambda = self.features[h].lambda;
        if !self.features[h].generated && self.features[h].last_exp_update != self.iteration - 1 {
            self.set_feature_expectation(h);
        }
        self.features[h].last_change = self.iteration;
        let to_update = self.features_to_update();
        let dlb = self.delta_loss_bound(h);
        let alpha;
        if self.features[h].binary {
            let a = self.good_alpha(h);
            alpha = self.reduce_alpha(a);
            new_loss = self.increase_lambda_one(h, alpha, &to_update);
        } else {
            let a = self.newton_step(h);
            let a = self.reduce_alpha(a);
            new_loss = self.increase_lambda_one(h, a, &to_update);
            if new_loss - old_loss > dlb {
                self.increase_lambda_one(h, -a, &[h]);
                let ga = self.good_alpha(h);
                let s = self.search_alpha(h, ga);
                let s = self.reduce_alpha(s);
                new_loss = self.increase_lambda_one(h, s, &to_update);
                alpha = s;
            } else {
                alpha = a;
            }
        }
        let f = &mut self.features[h];
        if alpha * old_lambda < 0.0 {
            f.contribution += new_loss - old_loss;
            if f.contribution < 0.0 {
                f.contribution = 0.0;
            }
        } else {
            f.contribution += old_loss - new_loss;
        }
        new_loss
    }

    fn termination_test(&mut self, new_loss: f64) -> bool {
        const CONVERGENCE_TEST_FREQUENCY: i64 = 20;
        if self.iteration == 0 {
            self.previous_loss = new_loss;
            return false;
        }
        if self.iteration % CONVERGENCE_TEST_FREQUENCY != 0 {
            return false;
        }
        if self.previous_loss - new_loss < self.settings.convergence_threshold {
            return true;
        }
        let gain = (self.n as f64).ln() - new_loss;
        if gain > 10000.0 {
            return true;
        }
        self.previous_loss = new_loss;
        false
    }

    fn run(&mut self, cancel: &dyn CancelCheck) -> Result<f64, TrainError> {
        let mut new_loss = self.get_loss();
        let max_it = self.settings.max_iterations as i64;
        let puf = self.settings.parallel_update_frequency as i64;
        self.iteration = 0;
        while self.iteration < max_it {
            if cancel.is_cancelled() {
                return Err(TrainError::Cancelled);
            }
            self.old_loss = new_loss;
            if self.iteration > 0 && puf > 0 && self.iteration % puf == 0 {
                new_loss = self.do_parallel_update();
            } else {
                let Some(f) = self.get_best_feature() else {
                    break;
                };
                new_loss = self.do_sequential_update(f);
            }
            if self.fatal.is_some() {
                break;
            }
            if self.termination_test(new_loss) {
                break;
            }
            self.iteration += 1;
        }
        Ok(self.get_loss())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_matches_runner() {
        let x = [0, 10, 17, 30, 100];
        let y = [2.6, 1.6, 0.9, 0.55, 0.05];
        assert_eq!(interpolate(&x, &y, 0), 2.6);
        assert_eq!(interpolate(&x, &y, 200), 0.05);
        assert!((interpolate(&x, &y, 60) - (0.55 + (0.05 - 0.55) * 30.0 / 70.0)).abs() < 1e-15);
        assert_eq!(interpolate(&[10, 30, 100], &[1.0, 0.2, 0.05], 5), 1.0);
    }

    #[test]
    fn precision_examples() {
        assert_eq!(precision(0.0), 0.0);
        assert!(precision(12.5) > 0.0);
    }
}
