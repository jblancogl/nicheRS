//! Trained MaxEnt model: `.lambdas` serialization (MaxEnt 3.4.4 format) and prediction.
//!
//! Prediction reproduces `FeaturedSpace(GridSet, lambdaFile, clamp)` + `Project`:
//! base variables are clamped to the training range of their linear feature, derived features
//! are scaled with their stored (min, max) and clamped to [0, 1].

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::javafmt::double_to_string;
use crate::train::VarKind;

#[derive(Debug, Clone, PartialEq)]
pub enum FeatureKind {
    Linear {
        var: String,
        min: f64,
        max: f64,
    },
    Quadratic {
        var: String,
        min: f64,
        max: f64,
    },
    Product {
        a: String,
        b: String,
        min: f64,
        max: f64,
    },
    /// `reverse` hinges are stored as in `.lambdas`: `min`/`max` are in the original
    /// (non-negated) variable units.
    Hinge {
        var: String,
        reverse: bool,
        min: f64,
        max: f64,
    },
    Threshold {
        var: String,
        threshold: f64,
    },
    Binary {
        var: String,
        value: f64,
        first_category: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelFeature {
    pub name: String,
    pub kind: FeatureKind,
    pub lambda: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaxentModel {
    pub features: Vec<ModelFeature>,
    pub linear_predictor_normalizer: f64,
    pub density_normalizer: f64,
    pub num_background_points: usize,
    pub entropy: f64,
    /// Variables in model order. Empty when parsed from a `.lambdas` file (not recorded there).
    pub variables: Vec<(String, VarKind)>,
}

/// Output scale. Only `Raw` is a normalized density; `Cloglog` and `Logistic` are monotone
/// transforms that MaxEnt documents as approximations of probability of presence under
/// strong assumptions (Phillips et al. 2017). NicheRS labels them "relative suitability".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Raw,
    Cloglog,
    /// Logistic with MaxEnt's default prevalence tau = 0.5.
    Logistic,
}

#[derive(Debug, thiserror::Error)]
pub enum LambdasError {
    #[error("line {line}: {msg}")]
    Parse { line: usize, msg: String },
}

impl MaxentModel {
    /// Serialize to MaxEnt's `.lambdas` text format (`FeaturedSpace.writeWeights`).
    pub fn to_lambdas(&self) -> String {
        let mut out = String::new();
        let nf = self.features.len();
        let mut written = false;
        let d = double_to_string;
        for (i, f) in self.features.iter().enumerate() {
            let keep_anyway = matches!(f.kind, FeatureKind::Linear { .. })
                || matches!(
                    f.kind,
                    FeatureKind::Binary {
                        first_category: true,
                        ..
                    }
                );
            if f.lambda == 0.0 && (written || i < nf - 1) && !keep_anyway {
                continue;
            }
            written = true;
            let _ = match &f.kind {
                FeatureKind::Linear { min, max, .. }
                | FeatureKind::Quadratic { min, max, .. }
                | FeatureKind::Product { min, max, .. } => {
                    writeln!(out, "{}, {}, {}, {}", f.name, d(f.lambda), d(*min), d(*max))
                }
                FeatureKind::Hinge {
                    var,
                    reverse,
                    min,
                    max,
                } => {
                    let prefix = if *reverse { '`' } else { '\'' };
                    writeln!(
                        out,
                        "{prefix}{var}, {}, {}, {}",
                        d(f.lambda),
                        d(*min),
                        d(*max)
                    )
                }
                FeatureKind::Threshold { .. } | FeatureKind::Binary { .. } => {
                    writeln!(out, "{}, {}, 0.0, 1.0", f.name, d(f.lambda))
                }
            };
        }
        let _ = writeln!(
            out,
            "linearPredictorNormalizer, {}",
            d(self.linear_predictor_normalizer)
        );
        let _ = writeln!(out, "densityNormalizer, {}", d(self.density_normalizer));
        let _ = writeln!(out, "numBackgroundPoints, {}", self.num_background_points);
        let _ = writeln!(out, "entropy, {}", d(self.entropy));
        out
    }

    /// Parse a MaxEnt `.lambdas` file (as written by MaxEnt 3.x or [`Self::to_lambdas`]).
    pub fn from_lambdas(text: &str) -> Result<Self, LambdasError> {
        let mut features = Vec::new();
        let (mut lpn, mut dn, mut nbg, mut entropy) = (0.0, 0.0, 0usize, -1.0);
        for (ln, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let err = |msg: &str| LambdasError::Parse {
                line: ln + 1,
                msg: msg.to_string(),
            };
            let parts: Vec<&str> = line.split(',').map(str::trim).collect();
            if parts.len() < 2 {
                return Err(err("expected at least 2 comma-separated fields"));
            }
            let num = |s: &str| {
                s.parse::<f64>()
                    .map_err(|_| err(&format!("invalid number {s:?}")))
            };
            let descr = parts[0];
            let lambda = num(parts[1])?;
            let (min, max) = if parts.len() >= 4 {
                (num(parts[2])?, num(parts[3])?)
            } else {
                (0.0, 1.0)
            };
            let kind = if let Some((a, b)) = descr.split_once('*') {
                FeatureKind::Product {
                    a: a.into(),
                    b: b.into(),
                    min,
                    max,
                }
            } else if let Some(v) = descr.strip_suffix("^2") {
                FeatureKind::Quadratic {
                    var: v.into(),
                    min,
                    max,
                }
            } else if descr.starts_with('(') && descr.contains('=') {
                let inner = &descr[1..descr.len() - 1];
                let (v, val) = inner
                    .split_once('=')
                    .ok_or_else(|| err("bad categorical feature"))?;
                FeatureKind::Binary {
                    var: v.into(),
                    value: f64::from(val.parse::<f32>().map_err(|_| err("bad category"))?),
                    first_category: false,
                }
            } else if descr.starts_with('(') && descr.contains('<') {
                let inner = &descr[1..descr.len() - 1];
                let (t, v) = inner
                    .split_once('<')
                    .ok_or_else(|| err("bad threshold feature"))?;
                FeatureKind::Threshold {
                    var: v.into(),
                    threshold: num(t)?,
                }
            } else if let Some(v) = descr.strip_prefix('\'') {
                FeatureKind::Hinge {
                    var: v.into(),
                    reverse: false,
                    min,
                    max,
                }
            } else if let Some(v) = descr.strip_prefix('`') {
                FeatureKind::Hinge {
                    var: v.into(),
                    reverse: true,
                    min,
                    max,
                }
            } else {
                match descr {
                    "linearPredictorNormalizer" => {
                        lpn = lambda;
                        continue;
                    }
                    "densityNormalizer" => {
                        dn = lambda;
                        continue;
                    }
                    "numBackgroundPoints" => {
                        nbg = lambda as usize;
                        continue;
                    }
                    "entropy" => {
                        entropy = lambda;
                        continue;
                    }
                    v => FeatureKind::Linear {
                        var: v.into(),
                        min,
                        max,
                    },
                }
            };
            features.push(ModelFeature {
                name: descr.to_string(),
                kind,
                lambda,
            });
        }
        Ok(Self {
            features,
            linear_predictor_normalizer: lpn,
            density_normalizer: dn,
            num_background_points: nbg,
            entropy,
            variables: Vec::new(),
        })
    }

    /// Names of the environmental variables the model needs.
    pub fn required_variables(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut push = |v: &String| {
            if !out.contains(v) {
                out.push(v.clone());
            }
        };
        for f in &self.features {
            match &f.kind {
                FeatureKind::Product { a, b, .. } => {
                    push(a);
                    push(b);
                }
                FeatureKind::Linear { var, .. }
                | FeatureKind::Quadratic { var, .. }
                | FeatureKind::Hinge { var, .. }
                | FeatureKind::Threshold { var, .. }
                | FeatureKind::Binary { var, .. } => push(var),
            }
        }
        out
    }

    /// Compile the model for fast repeated prediction against a fixed variable order.
    pub fn compile(&self, var_order: &[String], clamp: bool) -> Result<CompiledModel, String> {
        let idx: HashMap<&str, usize> = var_order
            .iter()
            .enumerate()
            .map(|(i, v)| (v.as_str(), i))
            .collect();
        let find = |v: &str| {
            idx.get(v)
                .copied()
                .ok_or_else(|| format!("missing variable {v}"))
        };
        // Base-variable clamping ranges come from linear features (always written).
        let mut ranges: Vec<Option<(f64, f64)>> = vec![None; var_order.len()];
        if clamp {
            for f in &self.features {
                if let FeatureKind::Linear { var, min, max } = &f.kind {
                    ranges[find(var)?] = Some((*min, *max));
                }
            }
        }
        let mut terms = Vec::new();
        for f in &self.features {
            if f.lambda == 0.0 && !matches!(f.kind, FeatureKind::Linear { .. }) {
                continue;
            }
            let t = match &f.kind {
                FeatureKind::Linear { var, min, max } => Term::Linear {
                    v: find(var)?,
                    min: *min,
                    scale: max - min,
                },
                FeatureKind::Quadratic { var, min, max } => Term::Square {
                    v: find(var)?,
                    min: *min,
                    scale: max - min,
                },
                FeatureKind::Product { a, b, min, max } => Term::Product {
                    a: find(a)?,
                    b: find(b)?,
                    min: *min,
                    scale: max - min,
                },
                FeatureKind::Hinge {
                    var,
                    reverse: false,
                    min,
                    max,
                } => Term::Hinge {
                    v: find(var)?,
                    sign: 1.0,
                    min: *min,
                    range: max - min,
                },
                // Reverse hinge on -x with knot -max and max -min (FeaturedSpace lambdas reader).
                FeatureKind::Hinge {
                    var,
                    reverse: true,
                    min,
                    max,
                } => Term::Hinge {
                    v: find(var)?,
                    sign: -1.0,
                    min: -max,
                    range: -min - -max,
                },
                FeatureKind::Threshold { var, threshold } => Term::Threshold {
                    v: find(var)?,
                    t: *threshold,
                },
                FeatureKind::Binary { var, value, .. } => Term::Binary {
                    v: find(var)?,
                    value: *value,
                },
            };
            terms.push((f.lambda, t));
        }
        Ok(CompiledModel {
            terms,
            ranges,
            clamp,
            lpn: self.linear_predictor_normalizer,
            dn: self.density_normalizer,
            entropy: self.entropy,
        })
    }
}

#[derive(Debug, Clone)]
enum Term {
    Linear {
        v: usize,
        min: f64,
        scale: f64,
    },
    Square {
        v: usize,
        min: f64,
        scale: f64,
    },
    Product {
        a: usize,
        b: usize,
        min: f64,
        scale: f64,
    },
    Hinge {
        v: usize,
        sign: f64,
        min: f64,
        range: f64,
    },
    Threshold {
        v: usize,
        t: f64,
    },
    Binary {
        v: usize,
        value: f64,
    },
}

/// A model bound to a variable order; cheap to evaluate per cell.
#[derive(Debug, Clone)]
pub struct CompiledModel {
    terms: Vec<(f64, Term)>,
    ranges: Vec<Option<(f64, f64)>>,
    clamp: bool,
    lpn: f64,
    dn: f64,
    entropy: f64,
}

impl CompiledModel {
    fn base(&self, x: &[f64], v: usize) -> f64 {
        match self.ranges[v] {
            Some((lo, hi)) => x[v].clamp(lo, hi),
            None => x[v],
        }
    }

    fn unit(&self, val: f64) -> f64 {
        if self.clamp { val.clamp(0.0, 1.0) } else { val }
    }

    /// Linear predictor (sum of lambda_j * f_j(x)).
    pub fn linear_predictor(&self, x: &[f64]) -> f64 {
        let mut lp = 0.0;
        for (lambda, t) in &self.terms {
            let f = match *t {
                Term::Linear { v, min, scale } => self.unit((self.base(x, v) - min) / scale),
                Term::Square { v, min, scale } => {
                    let b = self.base(x, v);
                    self.unit((b * b - min) / scale)
                }
                Term::Product { a, b, min, scale } => {
                    self.unit((self.base(x, a) * self.base(x, b) - min) / scale)
                }
                Term::Hinge {
                    v,
                    sign,
                    min,
                    range,
                } => {
                    let d = sign * x[v];
                    self.unit(if d > min { (d - min) / range } else { 0.0 })
                }
                Term::Threshold { v, t } => {
                    if x[v] > t {
                        1.0
                    } else {
                        0.0
                    }
                }
                Term::Binary { v, value } => {
                    if x[v] == value {
                        1.0
                    } else {
                        0.0
                    }
                }
            };
            lp += lambda * f;
        }
        lp
    }

    /// Prediction as in `Project.pred`: raw = min(1, exp(lp - lpn) / densityNormalizer), then
    /// the output transform; non-finite results become 1 if lp > lpn, else 0.
    pub fn predict(&self, x: &[f64], format: OutputFormat) -> f64 {
        let lp = self.linear_predictor(x);
        let mut pred = ((lp - self.lpn).exp() / self.dn).min(1.0);
        pred = match format {
            OutputFormat::Raw => pred,
            OutputFormat::Cloglog => 1.0 - (-pred * self.entropy.exp()).exp(),
            OutputFormat::Logistic => {
                let v = pred * self.entropy.exp();
                let dp = 0.5;
                dp * v / ((1.0 - dp) + dp * v)
            }
        };
        if !pred.is_finite() || (pred as f32).is_infinite() {
            pred = if lp > self.lpn { 1.0 } else { 0.0 };
        }
        pred
    }
}
