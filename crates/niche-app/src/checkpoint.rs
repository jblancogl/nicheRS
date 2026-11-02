//! Cross-validation checkpoints: every (settings, fold) model of the tuning grid is saved as soon
//! as it is trained, so a species interrupted by a restart (deployment, crash) resumes with the
//! models already trained instead of starting its grid again.
//!
//! A checkpoint is reused only when it was made from exactly the same inputs: its file records a
//! fingerprint of the occurrence and background cells, their environmental values, the fold
//! assignment, the tuning grid and [`CV_REVISION`]. Every number is stored by its bits, so a
//! resumed species gives bit-identical results to an uninterrupted one.

use std::path::{Path, PathBuf};

use niche_maxent::{FeatureKind, MaxentModel, ModelFeature, VarKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::dto::FoldMetrics;

/// Revision of the fold computation (kernel training, fold metrics). Bump it with any change
/// that alters a fold model or its metrics, so checkpoints made by an older build are not mixed
/// with new results (such changes need an ADR and regenerated golden references anyway).
pub const CV_REVISION: u32 = 1;

/// Directory of a species' checkpoints (outside the species' result directory, so they are never
/// listed in its manifest).
pub fn checkpoint_dir(data_dir: &Path, job: &str, key: u64) -> PathBuf {
    data_dir
        .join("jobs")
        .join(job)
        .join("checkpoints")
        .join(key.to_string())
}

/// Incremental fingerprint of the inputs of a tuning grid.
pub struct Fingerprint(Sha256);

impl Default for Fingerprint {
    fn default() -> Self {
        let mut h = Sha256::new();
        h.update(CV_REVISION.to_le_bytes());
        Self(h)
    }
}

impl Fingerprint {
    pub fn usizes(&mut self, v: &[usize]) {
        self.0.update((v.len() as u64).to_le_bytes());
        for x in v {
            self.0.update((*x as u64).to_le_bytes());
        }
    }
    pub fn f64s(&mut self, v: &[f64]) {
        self.0.update((v.len() as u64).to_le_bytes());
        for x in v {
            self.0.update(x.to_bits().to_le_bytes());
        }
    }
    pub fn str(&mut self, s: &str) {
        self.0.update((s.len() as u64).to_le_bytes());
        self.0.update(s.as_bytes());
    }
    pub fn finish(self) -> String {
        self.0
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

/// One trained fold model and its metrics.
pub struct Saved {
    pub metrics: FoldMetrics,
    pub nonzero: usize,
    pub model: MaxentModel,
}

/// Checkpoints of one species' grid; `None` directory = disabled (CLI batch runs, tests).
pub struct CvCheckpoints {
    dir: Option<PathBuf>,
    context: String,
}

impl CvCheckpoints {
    pub fn new(dir: Option<PathBuf>, context: String) -> Self {
        Self { dir, context }
    }

    fn path(&self, combo: usize, fold: usize) -> Option<PathBuf> {
        self.dir
            .as_ref()
            .map(|d| d.join(format!("cv-{combo}-{fold}.json")))
    }

    /// The saved model of (combo, fold), if it was made from the same inputs.
    pub fn load(&self, combo: usize, fold: usize) -> Option<Saved> {
        let text = std::fs::read_to_string(self.path(combo, fold)?).ok()?;
        let f: File = serde_json::from_str(&text).ok()?;
        (f.context == self.context && f.combo == combo && f.fold == fold).then(|| Saved {
            metrics: f.metrics.into(),
            nonzero: f.nonzero,
            model: f.model.into(),
        })
    }

    /// Save (combo, fold) atomically. A failure only costs the reuse, so it is logged, not raised.
    pub fn save(
        &self,
        combo: usize,
        fold: usize,
        metrics: &FoldMetrics,
        nonzero: usize,
        model: &MaxentModel,
    ) {
        let (Some(dir), Some(path)) = (self.dir.as_ref(), self.path(combo, fold)) else {
            return;
        };
        let file = File {
            context: self.context.clone(),
            combo,
            fold,
            nonzero,
            metrics: metrics.into(),
            model: model.into(),
        };
        let res = std::fs::create_dir_all(dir)
            .map_err(|e| e.to_string())
            .and_then(|()| serde_json::to_vec(&file).map_err(|e| e.to_string()))
            .and_then(|bytes| {
                crate::artifacts::write_atomic(&path, &bytes)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = res {
            tracing::warn!(file = %path.display(), error = %e, "cross-validation checkpoint not saved");
        }
    }
}

impl CvCheckpoints {
    /// Checkpoint directory (`None` when disabled).
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    fn named(&self, name: &str) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(format!("{name}.json")))
    }

    /// Saved metrics of a model that is not kept (null models), if made from the same inputs.
    pub fn load_metrics(&self, name: &str) -> Option<FoldMetrics> {
        let text = std::fs::read_to_string(self.named(name)?).ok()?;
        let f: MetricsFile = serde_json::from_str(&text).ok()?;
        (f.context == self.context && f.name == name).then(|| f.metrics.into())
    }

    /// Save metrics atomically; a failure only costs the reuse, so it is logged, not raised.
    pub fn save_metrics(&self, name: &str, metrics: &FoldMetrics) {
        let (Some(dir), Some(path)) = (self.dir.as_ref(), self.named(name)) else {
            return;
        };
        let file = MetricsFile {
            context: self.context.clone(),
            name: name.to_string(),
            metrics: metrics.into(),
        };
        let res = std::fs::create_dir_all(dir)
            .map_err(|e| e.to_string())
            .and_then(|()| serde_json::to_vec(&file).map_err(|e| e.to_string()))
            .and_then(|bytes| {
                crate::artifacts::write_atomic(&path, &bytes)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = res {
            tracing::warn!(file = %path.display(), error = %e, "null-model checkpoint not saved");
        }
    }
}

#[derive(Serialize, Deserialize)]
struct MetricsFile {
    context: String,
    name: String,
    metrics: Metrics,
}

/// Remove a species' checkpoints once its result is written.
pub fn clear(dir: &Path) {
    if let Err(e) = std::fs::remove_dir_all(dir)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(dir = %dir.display(), error = %e, "removing cross-validation checkpoints");
    }
}

// On-disk form: f64 as raw bits (exact, NaN included).

#[derive(Serialize, Deserialize)]
struct File {
    context: String,
    combo: usize,
    fold: usize,
    nonzero: usize,
    metrics: Metrics,
    model: Model,
}

#[derive(Serialize, Deserialize)]
struct Metrics {
    fold: usize,
    n_test: usize,
    cbi: Option<u64>,
    or10: u64,
    ormtp: u64,
    auc: u64,
}

impl From<&FoldMetrics> for Metrics {
    fn from(m: &FoldMetrics) -> Self {
        Self {
            fold: m.fold,
            n_test: m.n_test,
            cbi: m.cbi.map(f64::to_bits),
            or10: m.or10.to_bits(),
            ormtp: m.ormtp.to_bits(),
            auc: m.auc.to_bits(),
        }
    }
}

impl From<Metrics> for FoldMetrics {
    fn from(m: Metrics) -> Self {
        Self {
            fold: m.fold,
            n_test: m.n_test,
            cbi: m.cbi.map(f64::from_bits),
            or10: f64::from_bits(m.or10),
            ormtp: f64::from_bits(m.ormtp),
            auc: f64::from_bits(m.auc),
        }
    }
}

/// A MaxEnt model with every number stored by its bits (also the projection state, ADR-0016).
#[derive(Serialize, Deserialize)]
pub(crate) struct Model {
    features: Vec<Feature>,
    linear_predictor_normalizer: u64,
    density_normalizer: u64,
    num_background_points: usize,
    entropy: u64,
    /// (name, categorical)
    variables: Vec<(String, bool)>,
}

#[derive(Serialize, Deserialize)]
struct Feature {
    name: String,
    lambda: u64,
    kind: Kind,
}

#[derive(Serialize, Deserialize)]
enum Kind {
    Linear {
        var: String,
        min: u64,
        max: u64,
    },
    Quadratic {
        var: String,
        min: u64,
        max: u64,
    },
    Product {
        a: String,
        b: String,
        min: u64,
        max: u64,
    },
    Hinge {
        var: String,
        reverse: bool,
        min: u64,
        max: u64,
    },
    Threshold {
        var: String,
        threshold: u64,
    },
    Binary {
        var: String,
        value: u64,
        first_category: bool,
    },
}

impl From<&MaxentModel> for Model {
    fn from(m: &MaxentModel) -> Self {
        let b = f64::to_bits;
        Self {
            features: m
                .features
                .iter()
                .map(|f| Feature {
                    name: f.name.clone(),
                    lambda: b(f.lambda),
                    kind: match &f.kind {
                        FeatureKind::Linear { var, min, max } => Kind::Linear {
                            var: var.clone(),
                            min: b(*min),
                            max: b(*max),
                        },
                        FeatureKind::Quadratic { var, min, max } => Kind::Quadratic {
                            var: var.clone(),
                            min: b(*min),
                            max: b(*max),
                        },
                        FeatureKind::Product { a, b: bb, min, max } => Kind::Product {
                            a: a.clone(),
                            b: bb.clone(),
                            min: b(*min),
                            max: b(*max),
                        },
                        FeatureKind::Hinge {
                            var,
                            reverse,
                            min,
                            max,
                        } => Kind::Hinge {
                            var: var.clone(),
                            reverse: *reverse,
                            min: b(*min),
                            max: b(*max),
                        },
                        FeatureKind::Threshold { var, threshold } => Kind::Threshold {
                            var: var.clone(),
                            threshold: b(*threshold),
                        },
                        FeatureKind::Binary {
                            var,
                            value,
                            first_category,
                        } => Kind::Binary {
                            var: var.clone(),
                            value: b(*value),
                            first_category: *first_category,
                        },
                    },
                })
                .collect(),
            linear_predictor_normalizer: b(m.linear_predictor_normalizer),
            density_normalizer: b(m.density_normalizer),
            num_background_points: m.num_background_points,
            entropy: b(m.entropy),
            variables: m
                .variables
                .iter()
                .map(|(n, k)| (n.clone(), *k == VarKind::Categorical))
                .collect(),
        }
    }
}

impl From<Model> for MaxentModel {
    fn from(m: Model) -> Self {
        let f = f64::from_bits;
        Self {
            features: m
                .features
                .into_iter()
                .map(|x| ModelFeature {
                    name: x.name,
                    lambda: f(x.lambda),
                    kind: match x.kind {
                        Kind::Linear { var, min, max } => FeatureKind::Linear {
                            var,
                            min: f(min),
                            max: f(max),
                        },
                        Kind::Quadratic { var, min, max } => FeatureKind::Quadratic {
                            var,
                            min: f(min),
                            max: f(max),
                        },
                        Kind::Product { a, b, min, max } => FeatureKind::Product {
                            a,
                            b,
                            min: f(min),
                            max: f(max),
                        },
                        Kind::Hinge {
                            var,
                            reverse,
                            min,
                            max,
                        } => FeatureKind::Hinge {
                            var,
                            reverse,
                            min: f(min),
                            max: f(max),
                        },
                        Kind::Threshold { var, threshold } => FeatureKind::Threshold {
                            var,
                            threshold: f(threshold),
                        },
                        Kind::Binary {
                            var,
                            value,
                            first_category,
                        } => FeatureKind::Binary {
                            var,
                            value: f(value),
                            first_category,
                        },
                    },
                })
                .collect(),
            linear_predictor_normalizer: f(m.linear_predictor_normalizer),
            density_normalizer: f(m.density_normalizer),
            num_background_points: m.num_background_points,
            entropy: f(m.entropy),
            variables: m
                .variables
                .into_iter()
                .map(|(n, cat)| {
                    (
                        n,
                        if cat {
                            VarKind::Categorical
                        } else {
                            VarKind::Continuous
                        },
                    )
                })
                .collect(),
        }
    }
}
