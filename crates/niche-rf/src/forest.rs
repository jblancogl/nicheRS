//! Down-sampled (balanced) random forest: training, prediction, out-of-bag scores.

use niche_eval::rng::Rng;
use serde::{Deserialize, Serialize};

use crate::tree::{GrowParams, Tree, grow};
use crate::{RfData, RfError};

/// Format version written into serialised forests; bump on any incompatible change.
pub const FORMAT_VERSION: u32 = 1;

/// Training settings. Defaults follow Valavi et al. (2021, 2022): 1000 trees,
/// `mtry = floor(sqrt(p))`, fully grown trees (`nodesize = 1`), each tree on a bootstrap of
/// the presences and as many background points drawn with replacement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RfSettings {
    /// Number of trees. 1000 is the value used by Valavi et al.; the suitability resolution is
    /// `1/ntree`.
    pub ntree: usize,
    /// Variables tried at each split; `None` = `max(1, floor(sqrt(p)))` (Breiman 2001, the
    /// `randomForest` classification default). Values above `p` are capped at `p`.
    pub mtry: Option<usize>,
    /// Nodes holding at most this many samples are not split (`randomForest` `nodesize`;
    /// 1 = fully grown trees, the classification default).
    pub min_node_size: usize,
    /// Optional depth limit (root = depth 0); `None` = unlimited.
    pub max_depth: Option<usize>,
    /// Master seed. Tree `i` uses an RNG stream derived from `(seed, i)` only, so trees can be
    /// fitted in any order or in parallel with identical results. Record it in the manifest.
    pub seed: u64,
}

impl Default for RfSettings {
    fn default() -> Self {
        Self {
            ntree: 1000,
            mtry: None,
            min_node_size: 1,
            max_depth: None,
            seed: 0,
        }
    }
}

impl RfSettings {
    /// The `mtry` actually used for `p` predictors.
    pub fn resolved_mtry(&self, p: usize) -> usize {
        self.mtry
            .unwrap_or_else(|| (p as f64).sqrt().floor() as usize)
            .clamp(1, p.max(1))
    }
}

/// SplitMix64 finaliser (Steele et al. 2014), used to derive independent per-tree seeds.
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// RNG of tree `i`. Seeds are hashed rather than `seed + i` because `Rng::new` advances its
/// SplitMix state by a fixed increment: consecutive raw seeds would share state words.
fn tree_rng(seed: u64, i: usize) -> Rng {
    Rng::new(mix64(
        seed ^ mix64((i as u64).wrapping_add(0x9E37_79B9_7F4A_7C15)),
    ))
}

/// The balanced bootstrap of one tree: `n_p` presence indices drawn with replacement and the
/// same number of background indices drawn with replacement (Valavi et al. 2021, after Chen,
/// Liaw & Breiman 2004 "balanced random forest"; `randomForest(sampsize = c(n_p, n_p),
/// replace = TRUE)`). Drawn first from the tree's RNG, so it can be replayed for OOB scores.
fn draw_sample(rng: &mut Rng, n_p: usize, n_bg: usize) -> (Vec<u32>, Vec<u32>) {
    let pres = (0..n_p).map(|_| rng.below(n_p as u64) as u32).collect();
    let bg = (0..n_p).map(|_| rng.below(n_bg as u64) as u32).collect();
    (pres, bg)
}

/// Validated training problem. Trees are fitted independently with [`Trainer::fit_tree`]
/// (deterministic per tree index), so a caller may parallelise; [`fit`] is the sequential
/// convenience.
#[derive(Debug)]
pub struct Trainer<'a> {
    data: &'a RfData,
    settings: RfSettings,
    params: GrowParams,
}

impl<'a> Trainer<'a> {
    pub fn new(data: &'a RfData, settings: &RfSettings) -> Result<Self, RfError> {
        data.validate()?;
        if settings.ntree == 0 {
            return Err(RfError::Settings("ntree must be at least 1".into()));
        }
        if settings.min_node_size == 0 {
            return Err(RfError::Settings("min_node_size must be at least 1".into()));
        }
        if settings.mtry == Some(0) {
            return Err(RfError::Settings("mtry must be at least 1".into()));
        }
        let p = data.n_vars();
        Ok(Self {
            data,
            settings: settings.clone(),
            params: GrowParams {
                mtry: settings.resolved_mtry(p),
                min_node_size: settings.min_node_size,
                max_depth: settings.max_depth,
            },
        })
    }

    pub fn ntree(&self) -> usize {
        self.settings.ntree
    }

    /// Fit tree `i` (any index; the forest uses `0..ntree`). Pure function of
    /// `(data, settings, i)`.
    pub fn fit_tree(&self, i: usize) -> Tree {
        let d = self.data;
        let mut rng = tree_rng(self.settings.seed, i);
        let (pres, bg) = draw_sample(&mut rng, d.n_presence(), d.n_background());
        let cols: Vec<Vec<f64>> = d
            .presence
            .iter()
            .zip(&d.background)
            .map(|(pc, bc)| {
                pres.iter()
                    .map(|&k| pc[k as usize])
                    .chain(bg.iter().map(|&k| bc[k as usize]))
                    .collect()
            })
            .collect();
        let y: Vec<bool> = (0..pres.len() + bg.len()).map(|k| k < pres.len()).collect();
        grow(&cols, &y, self.params, &mut rng)
    }

    /// Assemble trees fitted with [`Trainer::fit_tree`] for `i = 0..ntree`, in index order.
    pub fn finish(self, trees: Vec<Tree>) -> Result<Forest, RfError> {
        if trees.len() != self.settings.ntree {
            return Err(RfError::Settings(format!(
                "expected {} trees, got {}",
                self.settings.ntree,
                trees.len()
            )));
        }
        let d = self.data;
        let range = |j: usize| {
            d.presence[j]
                .iter()
                .chain(&d.background[j])
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &x| {
                    (lo.min(x), hi.max(x))
                })
        };
        let (training_min, training_max) = (0..d.n_vars()).map(range).unzip();
        let mut settings = self.settings;
        settings.mtry = Some(self.params.mtry);
        Ok(Forest {
            format_version: FORMAT_VERSION,
            variables: d.variables.clone(),
            categorical: d.categorical.clone(),
            settings,
            n_presence: d.n_presence(),
            n_background: d.n_background(),
            training_min,
            training_max,
            trees,
        })
    }
}

/// Fit a down-sampled random forest sequentially. `cancel` is polled between trees (a closure
/// rather than a trait: one implementation does not justify an abstraction, ADR-0201).
pub fn fit(
    data: &RfData,
    settings: &RfSettings,
    cancel: &dyn Fn() -> bool,
) -> Result<Forest, RfError> {
    let trainer = Trainer::new(data, settings)?;
    let mut trees = Vec::with_capacity(trainer.ntree());
    for i in 0..trainer.ntree() {
        if cancel() {
            return Err(RfError::Cancelled);
        }
        trees.push(trainer.fit_tree(i));
    }
    trainer.finish(trees)
}

/// A fitted forest. Serialisable (serde) as flat node arrays; deserialisation re-checks the
/// structure so a corrupted checkpoint cannot make prediction loop or index out of range.
///
/// When stored as JSON, parse with exact float round-tripping (serde_json's
/// `float_roundtrip` feature): thresholds must come back bit for bit for predictions to be
/// identical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ForestRepr")]
pub struct Forest {
    format_version: u32,
    variables: Vec<String>,
    categorical: Vec<bool>,
    /// Settings used, with `mtry` resolved.
    settings: RfSettings,
    n_presence: usize,
    n_background: usize,
    training_min: Vec<f64>,
    training_max: Vec<f64>,
    trees: Vec<Tree>,
}

#[derive(Deserialize)]
struct ForestRepr {
    format_version: u32,
    variables: Vec<String>,
    categorical: Vec<bool>,
    settings: RfSettings,
    n_presence: usize,
    n_background: usize,
    training_min: Vec<f64>,
    training_max: Vec<f64>,
    trees: Vec<Tree>,
}

impl TryFrom<ForestRepr> for Forest {
    type Error = String;
    fn try_from(r: ForestRepr) -> Result<Self, String> {
        if r.format_version != FORMAT_VERSION {
            return Err(format!(
                "unsupported forest format version {}",
                r.format_version
            ));
        }
        let p = r.variables.len();
        if p == 0
            || r.categorical.len() != p
            || r.training_min.len() != p
            || r.training_max.len() != p
            || r.trees.is_empty()
        {
            return Err("forest arrays are empty or inconsistent".into());
        }
        for t in &r.trees {
            t.check(p)?;
        }
        Ok(Forest {
            format_version: r.format_version,
            variables: r.variables,
            categorical: r.categorical,
            settings: r.settings,
            n_presence: r.n_presence,
            n_background: r.n_background,
            training_min: r.training_min,
            training_max: r.training_max,
            trees: r.trees,
        })
    }
}

impl Forest {
    pub fn variables(&self) -> &[String] {
        &self.variables
    }
    pub fn categorical(&self) -> &[bool] {
        &self.categorical
    }
    /// Settings used for training (`mtry` resolved to the value used).
    pub fn settings(&self) -> &RfSettings {
        &self.settings
    }
    pub fn trees(&self) -> &[Tree] {
        &self.trees
    }
    pub fn n_presence(&self) -> usize {
        self.n_presence
    }
    pub fn n_background(&self) -> usize {
        self.n_background
    }
    /// Minimum and maximum of each variable over the training presences and background.
    pub fn training_range(&self, var: usize) -> Option<(f64, f64)> {
        Some((*self.training_min.get(var)?, *self.training_max.get(var)?))
    }

    /// Relative suitability of one row (predictors in training order): the fraction of trees
    /// voting "presence" (Valavi et al. 2021: `predict(type = "prob")` of `randomForest`,
    /// i.e. the proportion of votes). In [0, 1], resolution `1/ntree`.
    ///
    /// This is a **relative suitability index, not a calibrated probability of presence**:
    /// the balanced bootstrap fixes the class ratio at 1:1 regardless of prevalence, and
    /// presence–background data cannot identify prevalence (ADR-0007). Returns `NaN` if `row`
    /// is shorter than the number of variables; non-finite values must be masked by the
    /// caller (a `NaN` predictor always goes right at every split).
    pub fn predict(&self, row: &[f64]) -> f64 {
        if row.len() < self.variables.len() {
            return f64::NAN;
        }
        let votes: f64 = self.trees.iter().map(|t| t.vote(row)).sum();
        votes / self.trees.len() as f64
    }

    /// [`Forest::predict`] for column-major input (`cols[var][row]`, like `RfData`).
    pub fn predict_columns(&self, cols: &[Vec<f64>]) -> Vec<f64> {
        let n = cols.first().map_or(0, Vec::len);
        let mut row = vec![0.0; cols.len()];
        (0..n)
            .map(|i| {
                for (r, c) in row.iter_mut().zip(cols) {
                    *r = c.get(i).copied().unwrap_or(f64::NAN);
                }
                self.predict(&row)
            })
            .collect()
    }

    /// Out-of-bag scores on the **training** data (same `RfData` as passed to [`fit`]): each
    /// point is scored only by the trees whose balanced bootstrap did not contain it (the
    /// bootstraps are replayed from the per-tree seeds). Points that were in-bag for every
    /// tree get `NaN` and are left out of the AUC.
    ///
    /// Caveat: OOB is an optimistic, non-spatial estimate (random rather than spatially
    /// blocked); it is a diagnostic, not a substitute for blocked cross-validation.
    pub fn oob(&self, data: &RfData) -> Result<OobScores, RfError> {
        data.validate()?;
        if data.n_presence() != self.n_presence
            || data.n_background() != self.n_background
            || data.n_vars() != self.variables.len()
        {
            return Err(RfError::Shape);
        }
        let (n_p, n_bg) = (self.n_presence, self.n_background);
        let p = self.variables.len();
        let rows_p = data.presence_rows();
        let rows_b = data.background_rows();
        let mut sum_p = vec![0.0; n_p];
        let mut cnt_p = vec![0u32; n_p];
        let mut sum_b = vec![0.0; n_bg];
        let mut cnt_b = vec![0u32; n_bg];
        let mut in_p = vec![false; n_p];
        let mut in_b = vec![false; n_bg];
        for (i, tree) in self.trees.iter().enumerate() {
            let mut rng = tree_rng(self.settings.seed, i);
            let (pres, bg) = draw_sample(&mut rng, n_p, n_bg);
            in_p.fill(false);
            in_b.fill(false);
            for &k in &pres {
                in_p[k as usize] = true;
            }
            for &k in &bg {
                in_b[k as usize] = true;
            }
            for k in (0..n_p).filter(|&k| !in_p[k]) {
                sum_p[k] += tree.vote(&rows_p[k * p..(k + 1) * p]);
                cnt_p[k] += 1;
            }
            for k in (0..n_bg).filter(|&k| !in_b[k]) {
                sum_b[k] += tree.vote(&rows_b[k * p..(k + 1) * p]);
                cnt_b[k] += 1;
            }
        }
        let avg = |s: Vec<f64>, c: Vec<u32>| -> Vec<f64> {
            s.into_iter()
                .zip(c)
                .map(|(s, c)| if c == 0 { f64::NAN } else { s / f64::from(c) })
                .collect()
        };
        let presence = avg(sum_p, cnt_p);
        let background = avg(sum_b, cnt_b);
        let fp: Vec<f64> = presence.iter().copied().filter(|x| !x.is_nan()).collect();
        let fb: Vec<f64> = background.iter().copied().filter(|x| !x.is_nan()).collect();
        let auc = if fp.is_empty() || fb.is_empty() {
            f64::NAN
        } else {
            niche_eval::auc(&fp, &fb)
        };
        Ok(OobScores {
            presence,
            background,
            auc,
        })
    }
}

/// Out-of-bag relative suitability of the training points and the resulting
/// presence–background AUC.
#[derive(Debug, Clone, PartialEq)]
pub struct OobScores {
    pub presence: Vec<f64>,
    pub background: Vec<f64>,
    pub auc: f64,
}
