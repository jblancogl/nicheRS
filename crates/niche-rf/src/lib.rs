//! # niche-rf — down-sampled random forest for presence–background SDMs
//!
//! The second modelling engine of NicheRS (ADR-0002, ADR-0017): a *down-sampled* (balanced)
//! random forest as evaluated by Valavi et al. (2021, *Ecography* 44:1731–1742, "Modelling
//! species presence-only data with random forests") and Valavi et al. (2022, *Ecological
//! Monographs* 92:e01486), where it was in the top performance group for presence–background
//! (PB) data while the default random forest was in the lowest group.
//!
//! Why down-sampling: PB data are heavily imbalanced (tens of presences, thousands of
//! background points) and the classes overlap (background contains presences). A default
//! forest then predicts "background" almost everywhere. Each tree here is instead fitted to a
//! bootstrap of the presences plus an *equal* number of background points drawn with
//! replacement (Chen, Liaw & Breiman 2004, "balanced random forest";
//! `randomForest(sampsize = c(n_p, n_p), replace = TRUE)` in Valavi et al.'s code).
//!
//! | Choice | Value | Source |
//! |---|---|---|
//! | Trees | 1000 | Valavi et al. 2021 |
//! | Per-tree sample | `n_p` presences + `n_p` background, both with replacement | Valavi et al. 2021 |
//! | Split criterion | Gini, midpoints between distinct sorted values | Breiman 2001, `randomForest` |
//! | `mtry` | `floor(sqrt(p))` | `randomForest` classification default |
//! | Node size | 1 (fully grown) | `randomForest` classification default |
//! | Output | fraction of trees voting presence | `predict(type = "prob")` |
//! | Importance | permutation, mean AUC decrease | Breiman 2001; see [`permutation_importance`] |
//! | Response | partial dependence | Friedman 2001; see [`partial_dependence`] |
//!
//! **Output semantics.** Predictions are a *relative suitability index* in [0, 1]: they rank
//! environments, they are **not calibrated probabilities of presence** (the 1:1 balanced
//! sample fixes the apparent prevalence, and PB data cannot identify prevalence; ADR-0007).
//!
//! **Determinism.** All randomness comes from `niche_eval::rng::Rng`. Tree `i` uses a stream
//! derived only from `(seed, i)`, so [`Trainer::fit_tree`] can be called in any order or from
//! several threads by the caller and the forest is bit-identical to the sequential [`fit`].
//!
//! **Purity.** No I/O, no threads, no async, no third-party numerics; CART is implemented in
//! `tree.rs`. Cancellation is a plain `&dyn Fn() -> bool` polled between trees.
//!
//! **Validation.** There is no reference implementation to compare against bit for bit (the
//! R forest uses R's RNG and random tie-breaking), so correctness is tested by recovery of a
//! known synthetic niche and by invariants (determinism, bounds, serialisation round trip);
//! see `tests/` and ADR-0017.

mod explain;
mod forest;
mod tree;

pub use explain::{
    ResponseCurve, VariableImportance, partial_dependence, partial_dependence_of,
    permutation_importance,
};
pub use forest::{FORMAT_VERSION, Forest, OobScores, RfSettings, Trainer, fit};
pub use tree::Tree;

/// Presence–background training data, column-major like `niche_maxent::SwdData`:
/// `presence[var][i]`, `background[var][i]`. Every value must be finite (mask NODATA before).
#[derive(Debug, Clone, PartialEq)]
pub struct RfData {
    pub variables: Vec<String>,
    /// Per variable: `true` if categorical. Categorical codes are split as ordered numbers
    /// (documented simplification, see `tree.rs`); the flag is kept for reporting.
    pub categorical: Vec<bool>,
    pub presence: Vec<Vec<f64>>,
    pub background: Vec<Vec<f64>>,
}

impl RfData {
    pub fn n_vars(&self) -> usize {
        self.variables.len()
    }
    pub fn n_presence(&self) -> usize {
        self.presence.first().map_or(0, Vec::len)
    }
    pub fn n_background(&self) -> usize {
        self.background.first().map_or(0, Vec::len)
    }

    pub fn validate(&self) -> Result<(), RfError> {
        let p = self.variables.len();
        if p == 0
            || self.categorical.len() != p
            || self.presence.len() != p
            || self.background.len() != p
        {
            return Err(RfError::Shape);
        }
        let (n_p, n_bg) = (self.n_presence(), self.n_background());
        if n_p == 0 {
            return Err(RfError::NoPresence);
        }
        if n_bg == 0 {
            return Err(RfError::NoBackground);
        }
        if n_p > u32::MAX as usize || n_bg > u32::MAX as usize {
            return Err(RfError::Shape);
        }
        for j in 0..p {
            if self.presence[j].len() != n_p || self.background[j].len() != n_bg {
                return Err(RfError::Shape);
            }
            if self.presence[j]
                .iter()
                .chain(&self.background[j])
                .any(|x| !x.is_finite())
            {
                return Err(RfError::NonFinite(self.variables[j].clone()));
            }
        }
        Ok(())
    }

    fn rows(cols: &[Vec<f64>]) -> Vec<f64> {
        let n = cols.first().map_or(0, Vec::len);
        let mut out = Vec::with_capacity(n * cols.len());
        for i in 0..n {
            out.extend(cols.iter().map(|c| c[i]));
        }
        out
    }
    /// Presence rows, row-major (`n_presence × n_vars`).
    pub(crate) fn presence_rows(&self) -> Vec<f64> {
        Self::rows(&self.presence)
    }
    /// Background rows, row-major (`n_background × n_vars`).
    pub(crate) fn background_rows(&self) -> Vec<f64> {
        Self::rows(&self.background)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RfError {
    #[error("no presence samples")]
    NoPresence,
    #[error("no background points")]
    NoBackground,
    #[error("input arrays have inconsistent dimensions")]
    Shape,
    #[error("variable {0} has missing or non-finite values")]
    NonFinite(String),
    #[error("invalid settings: {0}")]
    Settings(String),
    #[error("training cancelled")]
    Cancelled,
}
