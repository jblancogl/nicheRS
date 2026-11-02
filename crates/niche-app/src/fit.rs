//! Per-species modelling: variable selection, spatial cross-validated tuning, final model,
//! prediction, uncertainty, extrapolation, thresholds, suitable area and evidence level.
//!
//! Scientific choices and their evidence: ADR-0009 (`docs/architecture/architecture-decisions`).

use std::sync::atomic::{AtomicBool, Ordering};

use niche_eval::rng::Rng;
use niche_eval::spatial::KM_PER_DEGREE;
use niche_eval::{
    Aoa, Folds, Mess, auc, autocorrelation_range_km, collinearity_filter, continuous_boyce,
    max_sss_threshold, mtp_threshold, null_presences, null_summary, omission_rate, p10_threshold,
    random_kfold, sd, spatial_blocks, spatial_grid_blocks,
};
use niche_maxent::{
    CancelCheck, FeatureClasses, MaxentModel, MaxentSettings, OutputFormat, SwdData, VarKind,
    Variable, permutation_importance, response_curves, train,
};
use niche_rf::{Forest, RfData, RfSettings, Trainer};
use rayon::prelude::*;

use crate::AppError;
use crate::checkpoint::{CvCheckpoints, Fingerprint};
use crate::dto::*;
use crate::env::EnvStack;

/// Minimum presences to fit at all; below it the result is "insufficient" (van Proosdij et al.
/// 2016 found ≥14–25 records needed for reliable MaxEnt models).
pub const MIN_PRESENCES: usize = 10;

pub struct Cancel<'a>(pub &'a AtomicBool);

impl CancelCheck for Cancel<'_> {
    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

pub struct FitOutput {
    pub result: SpeciesResult,
    pub suitability: Vec<f32>,
    pub uncertainty: Vec<f32>,
    pub mess: Vec<f32>,
    /// Dissimilarity index of the area of applicability (only when requested).
    pub aoa: Option<Vec<f32>>,
    /// Final MaxEnt model (`maxent`, and the MaxEnt member of an `ensemble`).
    pub model: Option<MaxentModel>,
    /// Final random forest (`rf`, and the RF member of an `ensemble`), ADR-0017.
    pub forest: Option<Forest>,
    /// Full marginal response curves (MaxEnt's 501 points), for `response_curves.csv`.
    pub curves: Vec<niche_maxent::ResponseCurve>,
    /// Cross-validation models taken from checkpoints (resumed species), out of `cv_models`.
    pub cv_reused: usize,
    pub cv_models: usize,
    /// Layer ids of the predictors kept by the collinearity filter, in model column order
    /// (empty when no model was fitted).
    pub variable_ids: Vec<String>,
}

/// The API result keeps every 5th point of MaxEnt's 501 (101 points, both ends included).
const CURVE_STEP: usize = 5;

fn classes(code: &str) -> Result<FeatureClasses, AppError> {
    let mut fc = FeatureClasses {
        linear: false,
        quadratic: false,
        product: false,
        threshold: false,
        hinge: false,
    };
    for ch in code.chars() {
        match ch {
            'L' => fc.linear = true,
            'Q' => fc.quadratic = true,
            'P' => fc.product = true,
            'T' => fc.threshold = true,
            'H' => fc.hinge = true,
            _ => {
                return Err(AppError::Invalid(format!(
                    "unknown feature class '{ch}' in {code}"
                )));
            }
        }
    }
    Ok(fc)
}

struct Data<'a> {
    stack: &'a EnvStack,
    vars: Vec<usize>,
}

impl Data<'_> {
    fn swd(&self, pres: &[usize], bg: &[usize]) -> SwdData {
        let cols = |cells: &[usize]| -> Vec<Vec<f64>> {
            self.vars
                .iter()
                .map(|&v| {
                    cells
                        .iter()
                        .map(|&c| f64::from(self.stack.layers[v].data[c]))
                        .collect()
                })
                .collect()
        };
        SwdData {
            vars: self
                .vars
                .iter()
                .map(|&v| Variable {
                    name: self.stack.layers[v].id.clone(),
                    kind: VarKind::Continuous,
                })
                .collect(),
            background: cols(bg),
            samples: cols(pres),
        }
    }

    fn names(&self) -> Vec<String> {
        self.vars
            .iter()
            .map(|&v| self.stack.layers[v].id.clone())
            .collect()
    }

    fn fill(&self, cell: usize, buf: &mut [f64]) {
        for (b, &v) in buf.iter_mut().zip(&self.vars) {
            *b = f64::from(self.stack.layers[v].data[cell]);
        }
    }

    /// Evaluate `f` on every cell's predictor row with one reusable buffer per worker thread.
    fn map_rows<F: Fn(&[f64]) -> f64 + Sync>(&self, cells: &[usize], f: F) -> Vec<f64> {
        let n = self.vars.len();
        cells
            .par_iter()
            .map_init(
                || vec![0.0; n],
                |buf, &c| {
                    self.fill(c, buf);
                    f(buf)
                },
            )
            .collect()
    }

    fn predict(&self, model: &MaxentModel, cells: &[usize]) -> Result<Vec<f64>, AppError> {
        let cm = model
            .compile(&self.names(), true)
            .map_err(AppError::Internal)?;
        Ok(self.map_rows(cells, |x| cm.predict(x, OutputFormat::Cloglog)))
    }

    /// Presence–background data of the random forest (same columns as [`Data::swd`]).
    fn rf_data(&self, pres: &[usize], bg: &[usize]) -> RfData {
        let swd = self.swd(pres, bg);
        RfData {
            variables: swd.vars.into_iter().map(|v| v.name).collect(),
            categorical: vec![false; self.vars.len()],
            presence: swd.samples,
            background: swd.background,
        }
    }

    /// Relative suitability of `cells` under any fitted model: MaxEnt cloglog, the fraction of
    /// trees voting presence, or the ensemble's mean percentile rank (ADR-0017).
    fn predict_fitted(&self, model: &Fitted, cells: &[usize]) -> Result<Vec<f64>, AppError> {
        match model {
            Fitted::Maxent(m) => self.predict(m, cells),
            Fitted::Rf(f) => Ok(self.map_rows(cells, |x| f.predict(x))),
            Fitted::Ensemble(e) => {
                let a = self.predict(&e.maxent, cells)?;
                let b = self.map_rows(cells, |x| e.rf.predict(x));
                Ok(a.iter().zip(&b).map(|(&a, &b)| e.combine(a, b)).collect())
            }
        }
    }
}

/// Modelling algorithm (ADR-0017). MaxEnt is the default (ADR-0002, ADR-0009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Maxent,
    /// Down-sampled (balanced) random forest (Valavi et al. 2021).
    Rf,
    /// MaxEnt + down-sampled RF: mean of the members' percentile ranks over the background.
    Ensemble,
}

impl Algorithm {
    pub fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "maxent" => Ok(Self::Maxent),
            "rf" => Ok(Self::Rf),
            "ensemble" => Ok(Self::Ensemble),
            other => Err(AppError::Invalid(format!(
                "algorithm must be maxent, rf or ensemble (got {other})"
            ))),
        }
    }
    fn uses_maxent(self) -> bool {
        self != Self::Rf
    }
    fn uses_rf(self) -> bool {
        self != Self::Maxent
    }
    fn label(self) -> &'static str {
        match self {
            Self::Maxent => "maxent",
            Self::Rf => "rf",
            Self::Ensemble => "ensemble",
        }
    }
}

/// One candidate setting of the tuning grid (or the selected setting refitted by null models).
#[derive(Debug, Clone, PartialEq)]
enum Setting {
    Maxent { fc: String, beta: f64 },
    Rf { mtry: usize },
    Ensemble { fc: String, beta: f64, mtry: usize },
}

/// Forest size and seed shared by every forest of a species (fold, final and null models).
#[derive(Debug, Clone, Copy)]
struct RfConfig {
    trees: usize,
    seed: u64,
}

/// A fitted model of any algorithm.
enum Fitted {
    Maxent(MaxentModel),
    Rf(Forest),
    Ensemble(Box<EnsembleModel>),
}

impl Fitted {
    /// Non-zero MaxEnt coefficients (tie-breaker of the selection; 0 for a forest).
    fn nonzero(&self) -> usize {
        match self {
            Fitted::Maxent(m) => m.features.iter().filter(|f| f.lambda != 0.0).count(),
            Fitted::Ensemble(e) => e.maxent.features.iter().filter(|f| f.lambda != 0.0).count(),
            Fitted::Rf(_) => 0,
        }
    }
}

/// MaxEnt + RF ensemble (ADR-0017). Each member's output is replaced by its percentile rank
/// among the member's predictions on the training background (mid-rank for ties), and the two
/// ranks are averaged with equal weights. Ranks make the members comparable whatever their scale
/// (cloglog vs. vote fraction) and are unchanged by any monotone transform of either member; the
/// result is a relative suitability index in [0, 1]: roughly the share of the study area's
/// background that the two members rank lower, on average.
struct EnsembleModel {
    maxent: MaxentModel,
    rf: Forest,
    /// Sorted member predictions on the training background (rank references).
    ref_maxent: Vec<f64>,
    ref_rf: Vec<f64>,
}

impl EnsembleModel {
    fn new(
        data: &Data<'_>,
        maxent: MaxentModel,
        rf: Forest,
        bg: &[usize],
    ) -> Result<Self, AppError> {
        let mut ref_maxent = data.predict(&maxent, bg)?;
        let mut ref_rf = data.map_rows(bg, |x| rf.predict(x));
        ref_maxent.sort_by(f64::total_cmp);
        ref_rf.sort_by(f64::total_cmp);
        Ok(Self {
            maxent,
            rf,
            ref_maxent,
            ref_rf,
        })
    }

    fn combine(&self, maxent: f64, rf: f64) -> f64 {
        0.5 * (percentile_rank(&self.ref_maxent, maxent) + percentile_rank(&self.ref_rf, rf))
    }
}

/// Relative suitability of the training presences used for thresholds (P10, MTP, max SSS). MaxEnt:
/// the model's predictions. Random forest: out-of-bag scores (each presence scored only by the
/// trees whose bootstrap left it out; `randomForest`'s `predict()` without new data), because
/// fully grown trees predict their own training presences at almost 1 and a threshold taken from
/// them would omit nearly every new presence (ADR-0017). Ensemble: MaxEnt prediction and RF
/// out-of-bag score, combined by ranks. Presences in-bag for every tree (NaN) are left out.
fn training_presence_scores(
    data: &Data<'_>,
    model: &Fitted,
    pres: &[usize],
    bg: &[usize],
) -> Result<Vec<f64>, AppError> {
    let oob = |f: &Forest| -> Result<Vec<f64>, AppError> {
        Ok(f.oob(&data.rf_data(pres, bg)).map_err(map_rf)?.presence)
    };
    let scores = match model {
        Fitted::Maxent(m) => return data.predict(m, pres),
        Fitted::Rf(f) => oob(f)?,
        Fitted::Ensemble(e) => {
            let a = data.predict(&e.maxent, pres)?;
            let b = oob(&e.rf)?;
            a.iter().zip(&b).map(|(&a, &b)| e.combine(a, b)).collect()
        }
    };
    Ok(scores.into_iter().filter(|x| !x.is_nan()).collect())
}

/// Candidate `mtry` values for `p` predictors: `floor(sqrt(p))` (the `randomForest` classification
/// default used by Valavi et al. 2021), plus `floor(p / 2)` when tuning is requested and it differs
/// (ADR-0017). Both at least 1.
fn rf_mtry_candidates(p: usize, tune: bool) -> Vec<usize> {
    let a = ((p as f64).sqrt().floor() as usize).max(1);
    let b = (p / 2).max(1);
    if tune && b != a { vec![a, b] } else { vec![a] }
}

/// Mid-rank empirical CDF of `x` in the sorted sample `sorted`: (#below + ½ #equal) / n.
fn percentile_rank(sorted: &[f64], x: f64) -> f64 {
    if x.is_nan() || sorted.is_empty() {
        return f64::NAN;
    }
    let below = sorted.partition_point(|v| *v < x);
    let upto = sorted.partition_point(|v| *v <= x);
    (below as f64 + 0.5 * (upto - below) as f64) / sorted.len() as f64
}

/// Fit a down-sampled random forest, trees in parallel (`Trainer::fit_tree` is a pure function of
/// the data, settings and tree index, so the forest does not depend on thread scheduling).
fn fit_forest(
    data: &Data<'_>,
    pres: &[usize],
    bg: &[usize],
    mtry: usize,
    rf: RfConfig,
    cancel: &AtomicBool,
) -> Result<Forest, AppError> {
    let d = data.rf_data(pres, bg);
    let settings = RfSettings {
        ntree: rf.trees,
        mtry: Some(mtry),
        seed: rf.seed,
        ..RfSettings::default()
    };
    let trainer = Trainer::new(&d, &settings).map_err(map_rf)?;
    let trees = (0..trainer.ntree())
        .into_par_iter()
        .map(|i| {
            if cancel.load(Ordering::Relaxed) {
                Err(AppError::Cancelled)
            } else {
                Ok(trainer.fit_tree(i))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    trainer.finish(trees).map_err(map_rf)
}

fn map_rf(e: niche_rf::RfError) -> AppError {
    match e {
        niche_rf::RfError::Cancelled => AppError::Cancelled,
        other => AppError::Model(format!("random forest: {other}")),
    }
}

fn maxent_settings(fc: &str, beta: f64) -> Result<MaxentSettings, AppError> {
    Ok(MaxentSettings {
        features: classes(fc)?,
        auto_features: false,
        beta_multiplier: beta,
        ..MaxentSettings::default()
    })
}

/// Fit `setting` on the given presences and background.
fn fit_model(
    data: &Data<'_>,
    setting: &Setting,
    pres: &[usize],
    bg: &[usize],
    rf: RfConfig,
    cancel: &AtomicBool,
) -> Result<Fitted, AppError> {
    let maxent = |fc: &str, beta: f64| -> Result<MaxentModel, AppError> {
        let (model, _) = train(
            &data.swd(pres, bg),
            &maxent_settings(fc, beta)?,
            &Cancel(cancel),
        )
        .map_err(map_train)?;
        Ok(model)
    };
    Ok(match setting {
        Setting::Maxent { fc, beta } => Fitted::Maxent(maxent(fc, *beta)?),
        Setting::Rf { mtry } => Fitted::Rf(fit_forest(data, pres, bg, *mtry, rf, cancel)?),
        Setting::Ensemble { fc, beta, mtry } => {
            let m = maxent(fc, *beta)?;
            let f = fit_forest(data, pres, bg, *mtry, rf, cancel)?;
            Fitted::Ensemble(Box::new(EnsembleModel::new(data, m, f, bg)?))
        }
    })
}

/// Training and held-out cells of fold `f`. Background assigned to fold `folds.k` is shared by
/// every fold (random k-fold).
struct Split {
    tr_p: Vec<usize>,
    te_p: Vec<usize>,
    tr_b: Vec<usize>,
    te_b: Vec<usize>,
}

fn split(presence_cells: &[usize], background: &[usize], folds: &Folds, f: usize) -> Split {
    let n = presence_cells.len();
    let k = folds.k;
    let tr_p: Vec<usize> = (0..n)
        .filter(|&i| folds.presence[i] != f)
        .map(|i| presence_cells[i])
        .collect();
    let te_p: Vec<usize> = (0..n)
        .filter(|&i| folds.presence[i] == f)
        .map(|i| presence_cells[i])
        .collect();
    let (tr_b, te_b): (Vec<usize>, Vec<usize>) = if folds.background.contains(&k) {
        (background.to_vec(), background.to_vec())
    } else {
        (
            (0..background.len())
                .filter(|&i| folds.background[i] != f)
                .map(|i| background[i])
                .collect(),
            (0..background.len())
                .filter(|&i| folds.background[i] == f)
                .map(|i| background[i])
                .collect(),
        )
    };
    Split {
        tr_p,
        te_p,
        tr_b,
        te_b,
    }
}

/// Held-out metrics of a fold model: thresholds from its training presences, Boyce, omission and
/// AUC on the fold's test presences and background.
fn evaluate(data: &Data<'_>, model: &Fitted, s: &Split, f: usize) -> Result<FoldMetrics, AppError> {
    let p_tr = training_presence_scores(data, model, &s.tr_p, &s.tr_b)?;
    let p_te = data.predict_fitted(model, &s.te_p)?;
    let p_bg = data.predict_fitted(model, &s.te_b)?;
    let t10 = p10_threshold(&p_tr);
    let tmtp = mtp_threshold(&p_tr);
    Ok(FoldMetrics {
        fold: f + 1,
        n_test: s.te_p.len(),
        cbi: if s.te_p.is_empty() || p_bg.is_empty() {
            None
        } else {
            continuous_boyce(&p_te, &p_bg)
        },
        or10: omission_rate(&p_te, t10),
        ormtp: omission_rate(&p_te, tmtp),
        auc: if s.te_p.is_empty() || p_bg.is_empty() {
            f64::NAN
        } else {
            auc(&p_te, &p_bg)
        },
    })
}

/// Mean of the finite values (NaN if none).
fn mean_finite(v: &[f64]) -> f64 {
    let w: Vec<f64> = v.iter().copied().filter(|x| x.is_finite()).collect();
    if w.is_empty() {
        f64::NAN
    } else {
        w.iter().sum::<f64>() / w.len() as f64
    }
}

/// Validation summary of a selected setting from its fold metrics (in fold order).
fn summarise(method: &str, fold_metrics: Vec<FoldMetrics>) -> ValidationSummary {
    let cbis: Vec<f64> = fold_metrics.iter().filter_map(|m| m.cbi).collect();
    ValidationSummary {
        method: method.to_string(),
        cbi_mean: if cbis.is_empty() {
            None
        } else {
            Some(mean_finite(&cbis))
        },
        or10_mean: mean_finite(&fold_metrics.iter().map(|m| m.or10).collect::<Vec<_>>()),
        ormtp_mean: mean_finite(&fold_metrics.iter().map(|m| m.ormtp).collect::<Vec<_>>()),
        auc_mean: mean_finite(&fold_metrics.iter().map(|m| m.auc).collect::<Vec<_>>()),
        folds: fold_metrics,
        blocks: None,
        null_model: None,
        members: vec![],
    }
}

struct FoldRun {
    /// Index in the tuning grid.
    combo: usize,
    fold: usize,
    metrics: FoldMetrics,
    nonzero: usize,
    /// `None` for a random forest whose metrics came from a checkpoint (refitted if selected).
    model: Option<Fitted>,
}

/// Random-forest explanations of the final model (ADR-0017, Integration): permutation repeats
/// and background rows of the importance, grid points and averaged rows of partial dependence.
const RF_IMPORTANCE_REPEATS: usize = 3;
const RF_IMPORTANCE_MAX_BACKGROUND: usize = 2000;
const RF_PD_GRID: usize = 51;
const RF_PD_ROWS: usize = 300;

#[allow(clippy::too_many_arguments)]
pub fn fit_species(
    stack: &EnvStack,
    species: &SpeciesRef,
    presence_cells: &[usize],
    background: &[usize],
    model_settings: &ModelSettings,
    validation: &ValidationSettings,
    seed: u64,
    null_weights: Option<&[f64]>,
    checkpoint_dir: Option<std::path::PathBuf>,
    cancel: &AtomicBool,
    progress: &(dyn Fn(&str, f64) + Sync),
) -> Result<FitOutput, AppError> {
    let mut warnings = Vec::new();
    let n = presence_cells.len();
    let spec = stack.spec;
    let empty = || vec![f32::NAN; spec.len()];
    let study_km2 = stack.valid_area_km2();
    let bounds = [spec.west, spec.south(), spec.east(), spec.north];
    let base = |level: EvidenceLevel,
                reasons: Vec<String>,
                warnings: Vec<Warning>,
                status: SpeciesRunStatus| SpeciesResult {
        species_key: species.key,
        scientific_name: species.scientific_name.clone(),
        status,
        error: None,
        evidence: Evidence { level, reasons },
        data: DataSummary {
            presences: n,
            background: background.len(),
            records_fetched: 0,
            exclusions: vec![],
            source: String::new(),
            query_url: None,
            generalised: None,
            by_source: Vec::new(),
        },
        variables_used: vec![],
        variables_dropped: vec![],
        selected: SelectedModel {
            feature_classes: String::new(),
            beta_multiplier: 0.0,
            algorithm: None,
            rf_trees: None,
            rf_mtry: None,
            rf_seed: None,
        },
        tuning: vec![],
        validation: ValidationSummary {
            method: validation.method.clone(),
            folds: vec![],
            cbi_mean: None,
            or10_mean: f64::NAN,
            ormtp_mean: f64::NAN,
            auc_mean: f64::NAN,
            blocks: None,
            null_model: None,
            members: vec![],
        },
        thresholds: vec![],
        study_area_km2: study_km2,
        extrapolation: Extrapolation {
            novel_fraction: f64::NAN,
        },
        aoa: None,
        background_sampling: None,
        sampling_bias: None,
        independent_validation: None,
        uncertainty: UncertaintySummary {
            mean_fold_sd: f64::NAN,
        },
        contributions: vec![],
        response_curves: vec![],
        partial_dependence: vec![],
        bounds,
        files: vec![],
        warnings,
        projections: vec![],
    };
    if n < MIN_PRESENCES {
        warnings.push(Warning::new(
            "too_few_presences",
            Severity::Blocking,
            format!("Only {n} usable presence cells; at least {MIN_PRESENCES} are required to fit a model (van Proosdij et al. 2016)."),
        ).with("n", n).with("min", MIN_PRESENCES));
        let r = base(
            EvidenceLevel::Insufficient,
            vec![format!(
                "{n} usable presence cells (< {MIN_PRESENCES}): no model fitted."
            )],
            warnings,
            SpeciesRunStatus::Skipped,
        );
        return Ok(FitOutput {
            result: r,
            suitability: empty(),
            uncertainty: empty(),
            mess: empty(),
            aoa: None,
            model: None,
            forest: None,
            curves: vec![],
            cv_reused: 0,
            cv_models: 0,
            variable_ids: vec![],
        });
    }

    // 1. Collinearity filter on background values, in the user's priority order.
    progress("Checking collinearity between variables", 0.02);
    let cols: Vec<Vec<f64>> = stack
        .layers
        .iter()
        .map(|l| background.iter().map(|&c| f64::from(l.data[c])).collect())
        .collect();
    let (kept, dropped) = collinearity_filter(&cols, model_settings.collinearity_threshold);
    let variables_dropped: Vec<DroppedVariable> = dropped
        .iter()
        .map(|&(i, k, r)| DroppedVariable {
            variable: stack.layers[i].name.clone(),
            reason: format!(
                "|r| = {:.2} with {} (threshold {:.2})",
                r.abs(),
                stack.layers[k].name,
                model_settings.collinearity_threshold
            ),
        })
        .collect();
    let data = Data { stack, vars: kept };

    // 2. Partitions.
    let center = |c: usize| spec.center(c / spec.width, c % spec.width);
    let mut rng = Rng::new(seed ^ species.key.wrapping_mul(0x9E37_79B9));
    let pc: Vec<(f64, f64)> = presence_cells.iter().map(|&c| center(c)).collect();
    let bc: Vec<(f64, f64)> = background.iter().map(|&c| center(c)).collect();
    let mut blocks = None;
    let folds = match validation.method.as_str() {
        "random_kfold" => {
            warnings.push(Warning::new(
                "random_cv",
                Severity::Warning,
                "Random k-fold cross-validation overestimates performance under spatial autocorrelation (Roberts et al. 2017); prefer spatial blocks.",
            ));
            random_kfold(n, background.len(), validation.folds.clamp(2, 10), &mut rng)
        }
        "spatial_grid" => {
            let mut design =
                grid_block_size(validation, &data, background, &bc, &mut rng, &mut warnings);
            let folds =
                spatial_grid_blocks(&pc, &bc, design.block_km, design.k_requested, &mut rng);
            design.k_achieved = folds.k;
            if folds.k < 2 {
                return Err(AppError::Invalid(format!(
                    "spatial grid blocks of {:.0} km put every presence in a single block, so there is nothing to validate against; choose smaller blocks or the default spatial blocks",
                    design.block_km
                )));
            }
            if folds.k < design.k_requested {
                warnings.push(
                    Warning::new(
                        "fewer_folds",
                        Severity::Warning,
                        format!(
                            "Spatial grid blocks gave {} folds instead of the {} requested: too few blocks contain presences. Use smaller blocks or fewer folds.",
                            folds.k, design.k_requested
                        ),
                    )
                    .with("requested", design.k_requested)
                    .with("achieved", folds.k),
                );
            }
            blocks = Some(design);
            folds
        }
        _ => spatial_blocks(&pc, &bc),
    };
    let k = folds.k;
    for f in 0..k {
        let nt = folds.presence.iter().filter(|&&g| g == f).count();
        if nt < 3 {
            warnings.push(
                Warning::new(
                    "small_fold",
                    Severity::Warning,
                    format!(
                        "Fold {} has only {nt} test presences; its metrics are unstable.",
                        f + 1
                    ),
                )
                .with("fold", f + 1)
                .with("n", nt),
            );
        }
    }

    // 3. Tuning grid × folds, in parallel. MaxEnt settings first (indices = MaxEnt combos, as
    //    their checkpoint names), then random-forest settings (ADR-0017).
    let algorithm = Algorithm::parse(&model_settings.algorithm)?;
    let rf = RfConfig {
        trees: model_settings.rf_trees(),
        // Kept below 2^53 so that the value recorded in result.json and the manifest is read back
        // exactly by JSON parsers that use doubles (browsers, Python's float fallback).
        seed: (seed ^ species.key.wrapping_mul(0x9E37_79B9) ^ RF_STREAM) & ((1 << 53) - 1),
    };
    let combos: Vec<(String, f64)> = if !algorithm.uses_maxent() {
        vec![]
    } else if model_settings.tuning == "fixed" {
        vec![(
            model_settings
                .feature_classes
                .first()
                .cloned()
                .unwrap_or_else(|| "LQH".into()),
            *model_settings.beta_multipliers.first().unwrap_or(&1.0),
        )]
    } else {
        model_settings
            .feature_classes
            .iter()
            .flat_map(|fc| {
                model_settings
                    .beta_multipliers
                    .iter()
                    .map(move |b| (fc.clone(), *b))
            })
            .collect()
    };
    if algorithm.uses_maxent() && combos.is_empty() {
        return Err(AppError::Invalid("no model settings to evaluate".into()));
    }
    for (fc, b) in &combos {
        classes(fc)?;
        if b.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(AppError::Invalid(format!(
                "regularization multiplier must be > 0 (got {b})"
            )));
        }
    }
    // mtry: floor(sqrt(p)) (randomForest default, Valavi et al. 2021); optionally also
    // floor(p / 2) (ADR-0017).
    let mtrys: Vec<usize> = if algorithm.uses_rf() {
        rf_mtry_candidates(data.vars.len(), model_settings.rf_tune_mtry)
    } else {
        vec![]
    };
    let n_maxent = combos.len();
    let grid: Vec<Setting> = combos
        .iter()
        .map(|(fc, beta)| Setting::Maxent {
            fc: fc.clone(),
            beta: *beta,
        })
        .chain(mtrys.iter().map(|&mtry| Setting::Rf { mtry }))
        .collect();
    let tasks: Vec<(usize, usize)> = (0..grid.len())
        .flat_map(|c| (0..k).map(move |f| (c, f)))
        .collect();
    // Every input of a fold model, so a checkpoint is reused only for exactly the same grid.
    let checkpoints = {
        let mut fp = Fingerprint::default();
        fp.usizes(presence_cells);
        fp.usizes(background);
        fp.usizes(&data.vars);
        for name in data.names() {
            fp.str(&name);
        }
        let mut buf = vec![0.0; data.vars.len()];
        for &c in presence_cells.iter().chain(background) {
            data.fill(c, &mut buf);
            fp.f64s(&buf);
        }
        fp.usizes(&[k]);
        fp.usizes(&folds.presence);
        fp.usizes(&folds.background);
        for (fc, b) in &combos {
            fp.str(fc);
            fp.f64s(&[*b]);
        }
        // Random-forest grid (absent for MaxEnt, so MaxEnt checkpoints keep their fingerprint).
        if algorithm.uses_rf() {
            fp.str("rf");
            fp.usizes(&[rf.trees, niche_rf::FORMAT_VERSION as usize]);
            fp.usizes(&mtrys);
            fp.str(&rf.seed.to_string());
        }
        let context = fp.finish();
        (CvCheckpoints::new(checkpoint_dir, context.clone()), context)
    };
    let (checkpoints, cv_context) = checkpoints;
    let reused = AtomicUsizeCounter::default();
    let done = AtomicUsizeCounter::default();
    let total = tasks.len();
    let runs: Vec<Result<FoldRun, AppError>> = tasks
        .par_iter()
        .map(|&(ci, f)| {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Cancelled);
            }
            // MaxEnt: model and metrics saved; random forest: metrics only (the forest is a
            // deterministic function of the inputs and seed, refitted if selected; ADR-0017).
            let saved = if ci < n_maxent {
                checkpoints.load(ci, f).map(|s| FoldRun {
                    combo: ci,
                    fold: f,
                    metrics: s.metrics,
                    nonzero: s.nonzero,
                    model: Some(Fitted::Maxent(s.model)),
                })
            } else {
                checkpoints
                    .load_metrics(&format!("cv-rf-{}-{f}", ci - n_maxent))
                    .map(|metrics| FoldRun {
                        combo: ci,
                        fold: f,
                        metrics,
                        nonzero: 0,
                        model: None,
                    })
            };
            if let Some(run) = saved {
                reused.inc();
                let d = done.inc();
                progress(
                    &format!("Cross-validation: {d}/{total} models"),
                    0.05 + 0.6 * d as f64 / total as f64,
                );
                return Ok(run);
            }
            let (metrics, nonzero, model) = fit_fold(
                &data,
                presence_cells,
                background,
                &folds,
                f,
                &grid[ci],
                rf,
                cancel,
            )?;
            match &model {
                Fitted::Maxent(m) => checkpoints.save(ci, f, &metrics, nonzero, m),
                _ => checkpoints.save_metrics(&format!("cv-rf-{}-{f}", ci - n_maxent), &metrics),
            }
            let d = done.inc();
            progress(
                &format!("Cross-validation: {d}/{total} models"),
                0.05 + 0.6 * d as f64 / total as f64,
            );
            Ok(FoldRun {
                combo: ci,
                fold: f,
                metrics,
                nonzero,
                model: Some(model),
            })
        })
        .collect();
    let mut fold_runs = Vec::with_capacity(runs.len());
    for r in runs {
        fold_runs.push(r?);
    }

    // 4. Aggregate and select: lowest mean OR10, then highest mean CBI, then fewest coefficients
    //    (sequential criteria, Radosavljevic & Anderson 2014; Kass et al. 2021). Each member of an
    //    ensemble is selected on its own grid by the same criteria.
    let mean = mean_finite;
    let mut tuning: Vec<TuningRow> = grid
        .iter()
        .enumerate()
        .map(|(ci, setting)| {
            let rs: Vec<&FoldRun> = fold_runs.iter().filter(|r| r.combo == ci).collect();
            let cbis: Vec<f64> = rs.iter().filter_map(|r| r.metrics.cbi).collect();
            let (feature_classes, beta_multiplier, algorithm, mtry) = match setting {
                Setting::Maxent { fc, beta } => (fc.clone(), *beta, None, None),
                Setting::Rf { mtry } => (String::new(), 0.0, Some("rf".into()), Some(*mtry)),
                Setting::Ensemble { .. } => (String::new(), 0.0, None, None),
            };
            TuningRow {
                feature_classes,
                beta_multiplier,
                or10_mean: mean(&rs.iter().map(|r| r.metrics.or10).collect::<Vec<_>>()),
                cbi_mean: if cbis.is_empty() {
                    None
                } else {
                    Some(mean(&cbis))
                },
                auc_mean: mean(&rs.iter().map(|r| r.metrics.auc).collect::<Vec<_>>()),
                n_nonzero: (rs.iter().map(|r| r.nonzero).sum::<usize>() as f64
                    / rs.len().max(1) as f64)
                    .round() as usize,
                selected: false,
                algorithm,
                mtry,
            }
        })
        .collect();
    let key = |t: &TuningRow| (t.or10_mean, -t.cbi_mean.unwrap_or(-2.0), t.n_nonzero as f64);
    let select = |range: std::ops::Range<usize>| -> Option<usize> {
        range.min_by(|&a, &b| {
            let (x, y) = (key(&tuning[a]), key(&tuning[b]));
            x.0.total_cmp(&y.0)
                .then(x.1.total_cmp(&y.1))
                .then(x.2.total_cmp(&y.2))
        })
    };
    let best_m = select(0..n_maxent);
    let best_r = select(n_maxent..grid.len());
    for b in [best_m, best_r].into_iter().flatten() {
        tuning[b].selected = true;
    }
    // Fold models and metrics of the selected setting(s), in fold order.
    let mut take = |ci: usize| -> Vec<FoldRun> {
        let mut rs: Vec<FoldRun> = Vec::with_capacity(k);
        let mut i = 0;
        while i < fold_runs.len() {
            if fold_runs[i].combo == ci {
                rs.push(fold_runs.swap_remove(i));
            } else {
                i += 1;
            }
        }
        rs.sort_by_key(|r| r.fold);
        rs
    };
    let sel_m: Vec<FoldRun> = best_m.map(&mut take).unwrap_or_default();
    let sel_r: Vec<FoldRun> = best_r.map(&mut take).unwrap_or_default();
    drop(fold_runs);
    let (sel_fc, sel_beta) = best_m.map(|b| combos[b].clone()).unwrap_or_default();
    let sel_mtry = best_r.map(|b| mtrys[b - n_maxent]);
    // Selected forests taken from checkpoints (metrics only) are refitted: same inputs and seed
    // give the same forest.
    let mut rf_fold_models: Vec<Forest> = Vec::with_capacity(sel_r.len());
    let mut rf_fold_metrics: Vec<FoldMetrics> = Vec::with_capacity(sel_r.len());
    for r in sel_r {
        let forest = match r.model {
            Some(Fitted::Rf(f)) => f,
            _ => {
                if cancel.load(Ordering::Relaxed) {
                    return Err(AppError::Cancelled);
                }
                let s = split(presence_cells, background, &folds, r.fold);
                fit_forest(&data, &s.tr_p, &s.tr_b, sel_mtry.unwrap_or(1), rf, cancel)?
            }
        };
        rf_fold_models.push(forest);
        rf_fold_metrics.push(r.metrics);
    }
    let mut m_fold_models: Vec<MaxentModel> = Vec::with_capacity(sel_m.len());
    let mut m_fold_metrics: Vec<FoldMetrics> = Vec::with_capacity(sel_m.len());
    for r in sel_m {
        if let Some(Fitted::Maxent(m)) = r.model {
            m_fold_models.push(m);
        }
        m_fold_metrics.push(r.metrics);
    }
    let selected_setting = match (algorithm, sel_mtry) {
        (Algorithm::Maxent, _) => Setting::Maxent {
            fc: sel_fc.clone(),
            beta: sel_beta,
        },
        (Algorithm::Rf, Some(mtry)) => Setting::Rf { mtry },
        (Algorithm::Ensemble, Some(mtry)) => Setting::Ensemble {
            fc: sel_fc.clone(),
            beta: sel_beta,
            mtry,
        },
        _ => {
            return Err(AppError::Internal(
                "no random-forest setting selected".into(),
            ));
        }
    };
    let (fold_models, mut val): (Vec<Fitted>, ValidationSummary) = match algorithm {
        Algorithm::Maxent => (
            m_fold_models.into_iter().map(Fitted::Maxent).collect(),
            summarise(&validation.method, m_fold_metrics),
        ),
        Algorithm::Rf => (
            rf_fold_models.into_iter().map(Fitted::Rf).collect(),
            summarise(&validation.method, rf_fold_metrics),
        ),
        Algorithm::Ensemble => {
            // The ensemble is evaluated as a model of its own: fold by fold, the average of the
            // two members' fold models, on the same held-out data.
            progress(
                "Evaluating the ensemble on the cross-validation folds",
                0.66,
            );
            let mut models = Vec::with_capacity(k);
            let mut metrics = Vec::with_capacity(k);
            for (f, (m, r)) in m_fold_models.into_iter().zip(rf_fold_models).enumerate() {
                let s = split(presence_cells, background, &folds, f);
                let e = Fitted::Ensemble(Box::new(EnsembleModel::new(&data, m, r, &s.tr_b)?));
                metrics.push(evaluate(&data, &e, &s, f)?);
                models.push(e);
            }
            let member = |name: &str, ms: Vec<FoldMetrics>| {
                let v = summarise(&validation.method, ms);
                MemberValidation {
                    algorithm: name.into(),
                    cbi_mean: v.cbi_mean,
                    or10_mean: v.or10_mean,
                    auc_mean: v.auc_mean,
                }
            };
            let mut v = summarise(&validation.method, metrics);
            v.members = vec![
                member("maxent", m_fold_metrics),
                member("rf", rf_fold_metrics),
            ];
            (models, v)
        }
    };
    if fold_models.len() != val.folds.len() {
        return Err(AppError::Internal(
            "cross-validation models missing for the selected setting".into(),
        ));
    }
    val.blocks = blocks;

    // Valid cells: prediction domain, and where null presences are drawn.
    let cells: Vec<usize> = (0..spec.len()).filter(|&i| stack.valid[i]).collect();

    // 4b. Null models of the selected setting (opt-in, ADR-0014).
    if validation.null_models > 0 {
        let t = std::time::Instant::now();
        // Bias-matched nulls (Bohl et al. 2019; ADR-0013): with a target-group effort surface,
        // null presences are drawn with probability proportional to its records per cell.
        let weights: Option<Vec<f64>> = null_weights.map(|w| {
            cells
                .iter()
                .map(|&c| w.get(c).copied().unwrap_or(0.0))
                .collect()
        });
        let null_context = match &weights {
            None => format!("{cv_context}/null"),
            Some(w) => {
                let mut fp = Fingerprint::default();
                fp.f64s(w);
                format!("{cv_context}/null-weighted-{}", fp.finish())
            }
        };
        let summary = null_models(
            &NullRun {
                data: &data,
                eligible: &cells,
                weights: weights.as_deref(),
                presences: n,
                background,
                bc: &bc,
                method: &validation.method,
                k: match (&val.blocks, validation.method.as_str()) {
                    (Some(b), _) => b.k_requested,
                    (None, "random_kfold") => validation.folds.clamp(2, 10),
                    _ => k,
                },
                block_km: val.blocks.as_ref().map(|b| b.block_km),
                setting: &selected_setting,
                rf,
                seed: seed ^ species.key.wrapping_mul(0x9E37_79B9) ^ NULL_STREAM,
                observed: (val.cbi_mean, val.or10_mean),
            },
            validation.null_models,
            &CvCheckpoints::new(
                checkpoints.dir().map(std::path::Path::to_path_buf),
                null_context,
            ),
            cancel,
            progress,
        )?;
        tracing::info!(
            species_key = species.key,
            replicates = summary.requested,
            folds = k,
            seconds = t.elapsed().as_secs_f64(),
            "null models"
        );
        if summary.completed < summary.requested {
            warnings.push(
                Warning::new(
                    "null_models_incomplete",
                    Severity::Warning,
                    format!(
                        "Only {} of {} null models could be fitted; the significance test uses those.",
                        summary.completed, summary.requested
                    ),
                )
                .with("completed", summary.completed)
                .with("requested", summary.requested),
            );
        }
        if summary.completed > 0 && summary.cbi.p_value > NULL_ALPHA {
            warnings.push(
                Warning::new(
                    "not_better_than_null",
                    Severity::Warning,
                    format!(
                        "The model is not better than models fitted to random locations (Boyce index, p = {:.2}); the evidence level is capped at Exploratory.",
                        summary.cbi.p_value
                    ),
                )
                .with("p_value", (summary.cbi.p_value * 1000.0).round() / 1000.0),
            );
        }
        val.null_model = Some(summary);
    }

    // 5. Final model on all data.
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Cancelled);
    }
    progress("Fitting final model on all records", 0.7);
    // MaxEnt (and the ensemble's MaxEnt member): model, percent contribution, permutation
    // importance and marginal response curves, as before ADR-0017.
    let mut maxent_final = None;
    if algorithm.uses_maxent() {
        let settings = maxent_settings(&sel_fc, sel_beta)?;
        let final_swd = data.swd(presence_cells, background);
        let (final_model, report) =
            train(&final_swd, &settings, &Cancel(cancel)).map_err(map_train)?;
        progress("Computing variable importance and response curves", 0.75);
        let permutation =
            permutation_importance(&final_model, &final_swd).map_err(AppError::Internal)?;
        let full_curves = response_curves(
            &final_model,
            &final_swd,
            settings.add_samples_to_background,
            OutputFormat::Cloglog,
        )
        .map_err(AppError::Internal)?;
        maxent_final = Some((final_model, report, permutation, full_curves, final_swd));
    }
    // Random forest (and the ensemble's RF member): forest, permutation importance and partial
    // dependence (ADR-0017).
    let mut rf_final = None;
    if let Some(mtry) = sel_mtry {
        progress("Fitting the random forest on all records", 0.72);
        let forest = fit_forest(&data, presence_cells, background, mtry, rf, cancel)?;
        progress(
            "Computing random-forest variable importance and partial dependence",
            0.76,
        );
        let rf_data = data.rf_data(presence_cells, background);
        let importance = niche_rf::permutation_importance(
            &forest,
            &rf_data,
            RF_IMPORTANCE_REPEATS,
            Some(RF_IMPORTANCE_MAX_BACKGROUND),
            rf.seed ^ IMPORTANCE_STREAM,
        )
        .map_err(map_rf)?;
        let pd = (0..data.vars.len())
            .into_par_iter()
            .map(|j| {
                niche_rf::partial_dependence_of(
                    &forest,
                    &rf_data,
                    j,
                    RF_PD_GRID,
                    RF_PD_ROWS,
                    rf.seed ^ PD_STREAM,
                )
                .map_err(map_rf)
            })
            .collect::<Result<Vec<_>, _>>()?;
        rf_final = Some((forest, importance, pd));
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Cancelled);
    }
    let final_fitted =
        match (algorithm, &maxent_final, &rf_final) {
            (Algorithm::Maxent, Some(m), _) => Fitted::Maxent(m.0.clone()),
            (Algorithm::Rf, _, Some(r)) => Fitted::Rf(r.0.clone()),
            (Algorithm::Ensemble, Some(m), Some(r)) => Fitted::Ensemble(Box::new(
                EnsembleModel::new(&data, m.0.clone(), r.0.clone(), background)?,
            )),
            _ => return Err(AppError::Internal("final model missing".into())),
        };
    // Weights of the area of applicability: permutation importance of the model (ensemble: the
    // mean of the two members' normalised importances).
    let aoa_weights: Vec<(String, f64)> = {
        let ids = data.names();
        let norm = |w: Vec<f64>| -> Option<Vec<f64>> {
            let w: Vec<f64> = w
                .into_iter()
                .map(|x| if x.is_finite() && x > 0.0 { x } else { 0.0 })
                .collect();
            let t: f64 = w.iter().sum();
            (t > 0.0).then(|| w.iter().map(|x| x / t).collect())
        };
        let m = maxent_final.as_ref().map(|m| {
            ids.iter()
                .map(|id| m.2.iter().find(|(v, _)| v == id).map_or(0.0, |(_, w)| *w))
                .collect::<Vec<f64>>()
        });
        let r = rf_final
            .as_ref()
            .map(|r| r.1.iter().map(|v| v.percent).collect::<Vec<f64>>());
        let w = match (m, r) {
            (Some(m), None) => m,
            (None, Some(r)) => r,
            (Some(m), Some(r)) => match (norm(m.clone()), norm(r.clone())) {
                (Some(a), Some(b)) => a.iter().zip(&b).map(|(x, y)| 0.5 * (x + y)).collect(),
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => m,
            },
            (None, None) => vec![0.0; ids.len()],
        };
        ids.into_iter().zip(w).collect()
    };

    // 6. Predictions over all valid cells: final model, fold models (uncertainty), MESS.
    progress("Predicting habitat suitability", 0.8);
    let pred = data.predict_fitted(&final_fitted, &cells)?;
    let mut suitability = empty();
    for (&c, &p) in cells.iter().zip(&pred) {
        suitability[c] = p as f32;
    }
    progress(
        "Estimating uncertainty across cross-validation models",
        0.86,
    );
    let fold_preds: Vec<Vec<f64>> = fold_models
        .iter()
        .map(|m| data.predict_fitted(m, &cells))
        .collect::<Result<_, _>>()?;
    drop(fold_models);
    let mut uncertainty = empty();
    let mut sd_sum = 0.0;
    for (j, &c) in cells.iter().enumerate() {
        let v: Vec<f64> = fold_preds.iter().map(|p| p[j]).collect();
        let s = sd(&v);
        sd_sum += s;
        uncertainty[c] = s as f32;
    }
    progress("Computing environmental novelty (MESS)", 0.92);
    let reference: Vec<Vec<f64>> = data
        .vars
        .iter()
        .map(|&v| {
            background
                .iter()
                .chain(presence_cells)
                .map(|&c| f64::from(stack.layers[v].data[c]))
                .collect()
        })
        .collect();
    let mess_model = Mess::new(&reference);
    let mess_vals: Vec<f64> = data.map_rows(&cells, |x| mess_model.eval(x));
    let mut mess = empty();
    let mut novel = 0usize;
    for (&c, &m) in cells.iter().zip(&mess_vals) {
        mess[c] = m as f32;
        if m < 0.0 {
            novel += 1;
        }
    }
    let novel_fraction = novel as f64 / cells.len().max(1) as f64;

    // 6b. Area of applicability (opt-in, ADR-0014). Diagnostic only: no effect on the evidence
    //     level, where MESS remains the extrapolation criterion (ADR-0009).
    let mut aoa_layer = None;
    let mut aoa_summary = None;
    if validation.aoa {
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Cancelled);
        }
        progress("Computing the area of applicability", 0.95);
        let t = std::time::Instant::now();
        let (summary, di) = area_of_applicability(
            &data,
            presence_cells,
            background,
            &folds,
            &aoa_weights,
            &cells,
            seed ^ species.key.wrapping_mul(0x9E37_79B9) ^ AOA_STREAM,
        )?;
        tracing::info!(
            species_key = species.key,
            cells = cells.len(),
            training_rows = summary.n_train,
            seconds = t.elapsed().as_secs_f64(),
            "area of applicability"
        );
        let mut layer = empty();
        for (&c, &d) in cells.iter().zip(&di) {
            layer[c] = d as f32;
        }
        if summary.fraction_outside > 0.1 {
            warnings.push(Warning::new(
                "aoa_outside",
                Severity::Warning,
                format!("{:.0} % of the study area is outside the area of applicability (dissimilarity to the training data above the threshold); predictions there are less reliable.", summary.fraction_outside * 100.0),
            ).with("percent", (summary.fraction_outside * 100.0).round()));
        }
        aoa_layer = Some(layer);
        aoa_summary = Some(summary);
    }

    // 7. Thresholds and suitable-conditions area (exact ellipsoidal cell areas).
    let p_pres = training_presence_scores(&data, &final_fitted, presence_cells, background)?;
    let p_bg = data.predict_fitted(&final_fitted, background)?;
    let area_at = |t: f64| -> f64 {
        cells
            .iter()
            .zip(&pred)
            .filter(|(_, p)| **p >= t)
            .map(|(&c, _)| stack.cell_area(c))
            .sum()
    };
    let thresholds: Vec<ThresholdArea> = [
        (
            "p10",
            "10th percentile training presence",
            p10_threshold(&p_pres),
        ),
        (
            "max_sss",
            "Maximum sensitivity + specificity",
            max_sss_threshold(&p_pres, &p_bg),
        ),
        ("mtp", "Minimum training presence", mtp_threshold(&p_pres)),
    ]
    .iter()
    .map(|&(rule, label, t)| {
        let a = area_at(t);
        ThresholdArea {
            rule: rule.into(),
            label: label.into(),
            threshold: t,
            area_km2: a,
            fraction_of_study_area: a / study_km2,
        }
    })
    .collect();

    let name_of = |id: &str| {
        stack
            .layers
            .iter()
            .find(|l| l.id == id)
            .map_or_else(|| id.to_string(), |l| l.name.clone())
    };
    let rf_pct = |id: &str| -> Option<f64> {
        rf_final
            .as_ref()
            .and_then(|r| r.1.iter().find(|v| v.variable == id).map(|v| v.percent))
    };
    let mut contributions: Vec<Contribution> = match &maxent_final {
        Some((_, report, permutation, _, _)) => report
            .percent_contribution
            .iter()
            .map(|(v, p)| Contribution {
                variable: name_of(v),
                percent: *p,
                permutation_importance: permutation
                    .iter()
                    .find(|(id, _)| id == v)
                    .map(|(_, pi)| *pi),
                rf_importance: rf_pct(v),
            })
            .collect(),
        // Random forest alone: its permutation importance is the only measure.
        None => rf_final
            .as_ref()
            .map(|r| {
                r.1.iter()
                    .map(|v| Contribution {
                        variable: name_of(&v.variable),
                        percent: v.percent,
                        permutation_importance: None,
                        rf_importance: Some(v.percent),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };
    contributions.sort_by(|a, b| b.percent.total_cmp(&a.percent));
    let curves: Vec<ResponseCurve> = match &maxent_final {
        Some((_, _, _, full_curves, final_swd)) => full_curves
            .iter()
            .map(|c| {
                let v = final_swd
                    .vars
                    .iter()
                    .position(|x| x.name == c.variable)
                    .unwrap_or(0);
                let (lo, hi) = final_swd.background[v]
                    .iter()
                    .chain(&final_swd.samples[v])
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| {
                        (a.min(x), b.max(x))
                    });
                let keep = |i: usize| c.categorical || i.is_multiple_of(CURVE_STEP);
                ResponseCurve {
                    variable: c.variable.clone(),
                    name: name_of(&c.variable),
                    categorical: c.categorical,
                    x: c.x
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| keep(*i))
                        .map(|(_, x)| *x)
                        .collect(),
                    y: c.y
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| keep(*i))
                        .map(|(_, y)| *y)
                        .collect(),
                    training_min: lo,
                    training_max: hi,
                }
            })
            .collect(),
        None => vec![],
    };
    let partial_dependence: Vec<ResponseCurve> = rf_final
        .as_ref()
        .map(|(forest, _, pd)| {
            pd.iter()
                .enumerate()
                .map(|(j, c)| {
                    let (lo, hi) = forest.training_range(j).unwrap_or((f64::NAN, f64::NAN));
                    ResponseCurve {
                        variable: c.variable.clone(),
                        name: name_of(&c.variable),
                        categorical: c.categorical,
                        x: c.x.clone(),
                        y: c.y.clone(),
                        training_min: lo,
                        training_max: hi,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // 8. Evidence level (ADR-0009).
    let evidence = evidence_level(n, &val, novel_fraction, &validation.method);
    if novel_fraction > 0.1 {
        warnings.push(Warning::new(
            "extrapolation",
            Severity::Warning,
            format!("{:.0} % of the study area has environmental conditions outside the training range (MESS < 0); predictions there are extrapolations.", novel_fraction * 100.0),
        ).with("percent", (novel_fraction * 100.0).round()));
    }
    let mut r = base(
        evidence.level,
        evidence.reasons,
        warnings,
        SpeciesRunStatus::Succeeded,
    );
    r.variables_used = data
        .vars
        .iter()
        .map(|&v| stack.layers[v].name.clone())
        .collect();
    r.variables_dropped = variables_dropped;
    r.selected = SelectedModel {
        feature_classes: sel_fc,
        beta_multiplier: sel_beta,
        algorithm: (algorithm != Algorithm::Maxent).then(|| algorithm.label().to_string()),
        rf_trees: sel_mtry.map(|_| rf.trees),
        rf_mtry: sel_mtry,
        rf_seed: sel_mtry.map(|_| rf.seed),
    };
    r.tuning = tuning;
    r.validation = val;
    r.thresholds = thresholds;
    r.extrapolation = Extrapolation { novel_fraction };
    r.aoa = aoa_summary;
    r.uncertainty = UncertaintySummary {
        mean_fold_sd: sd_sum / cells.len().max(1) as f64,
    };
    r.contributions = contributions;
    r.response_curves = curves;
    r.partial_dependence = partial_dependence;
    progress("Model complete", 1.0);
    Ok(FitOutput {
        result: r,
        suitability,
        uncertainty,
        mess,
        aoa: aoa_layer,
        model: maxent_final.as_ref().map(|m| m.0.clone()),
        forest: rf_final.map(|r| r.0),
        curves: maxent_final.map(|m| m.3).unwrap_or_default(),
        cv_reused: reused.0.load(Ordering::Relaxed),
        cv_models: total,
        variable_ids: data.names(),
    })
}

/// What projecting a fitted species to other environments needs (future climate, ADR-0016):
/// the final model(s), the predictors in model column order, the training cells and the
/// present P10 threshold. The MESS reference and the ensemble's rank references are recomputed
/// from the training cells on the present stack, exactly as in [`fit_species`], so a projection
/// resumed from a saved `Projector` gives the same numbers as one made right after the fit.
#[derive(Clone)]
pub struct Projector {
    /// `maxent`, `rf` or `ensemble`.
    pub algorithm: String,
    /// Layer ids in model column order.
    pub vars: Vec<String>,
    pub maxent: Option<MaxentModel>,
    pub forest: Option<Forest>,
    pub presence: Vec<usize>,
    pub background: Vec<usize>,
    /// Present P10 threshold (relative suitability units).
    pub p10: f64,
}

/// A [`Projector`] bound to the present stack: model ready to predict, MESS reference, and the
/// present suitability of every valid cell.
pub struct PreparedProjection<'a> {
    data: Data<'a>,
    model: Fitted,
    mess: Mess,
    /// Valid cells of the present stack (the projection domain), ascending.
    pub cells: Vec<usize>,
    /// Present relative suitability at `cells` (the values of `suitability.tif`, before their
    /// conversion to f32, so that thresholds classify cells exactly as the present areas).
    pub present: Vec<f64>,
    pub threshold: f64,
}

/// One projection of the model: suitability and MESS at the prepared cells (NaN where a future
/// predictor value is missing).
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedRun {
    pub suitability: Vec<f64>,
    pub mess: Vec<f32>,
}

impl Projector {
    pub fn prepare<'a>(&self, stack: &'a EnvStack) -> Result<PreparedProjection<'a>, AppError> {
        let vars = self
            .vars
            .iter()
            .map(|id| {
                stack
                    .layers
                    .iter()
                    .position(|l| &l.id == id)
                    .ok_or_else(|| {
                        AppError::Internal(format!("projection: predictor {id} missing"))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let data = Data { stack, vars };
        let model = match (
            Algorithm::parse(&self.algorithm)?,
            &self.maxent,
            &self.forest,
        ) {
            (Algorithm::Maxent, Some(m), _) => Fitted::Maxent(m.clone()),
            (Algorithm::Rf, _, Some(f)) => Fitted::Rf(f.clone()),
            (Algorithm::Ensemble, Some(m), Some(f)) => Fitted::Ensemble(Box::new(
                EnsembleModel::new(&data, m.clone(), f.clone(), &self.background)?,
            )),
            _ => return Err(AppError::Internal("projection: final model missing".into())),
        };
        let reference: Vec<Vec<f64>> = data
            .vars
            .iter()
            .map(|&v| {
                self.background
                    .iter()
                    .chain(&self.presence)
                    .map(|&c| f64::from(stack.layers[v].data[c]))
                    .collect()
            })
            .collect();
        let cells: Vec<usize> = (0..stack.spec.len()).filter(|&i| stack.valid[i]).collect();
        let present = data.predict_fitted(&model, &cells)?;
        Ok(PreparedProjection {
            data,
            model,
            mess: Mess::new(&reference),
            cells,
            present,
            threshold: self.p10,
        })
    }
}

impl PreparedProjection<'_> {
    /// Predict with `layers` (one per model variable, in model order, on the present grid) in
    /// place of the present predictors. Ensemble ranks keep the present background as their
    /// reference, so future values are on the present scale. Cells where any layer is not
    /// finite are NaN.
    pub fn predict(&self, layers: Vec<Vec<f32>>) -> Result<ProjectedRun, AppError> {
        let base = self.data.stack;
        if layers.len() != self.data.vars.len() || layers.iter().any(|l| l.len() != base.spec.len())
        {
            return Err(AppError::Internal(
                "projection: future layers do not match the model grid".into(),
            ));
        }
        let future = EnvStack {
            spec: base.spec,
            area_mask: vec![],
            land_mask: vec![],
            valid: vec![],
            layers: layers
                .into_iter()
                .zip(&self.data.vars)
                .map(|(data, &v)| crate::env::EnvLayer {
                    id: base.layers[v].id.clone(),
                    name: base.layers[v].name.clone(),
                    data,
                })
                .collect(),
            row_area_km2: vec![],
            provenance: serde_json::Value::Null,
        };
        let fdata = Data {
            stack: &future,
            vars: (0..future.layers.len()).collect(),
        };
        let ok: Vec<usize> = self
            .cells
            .iter()
            .copied()
            .filter(|&c| future.layers.iter().all(|l| l.data[c].is_finite()))
            .collect();
        let pred = fdata.predict_fitted(&self.model, &ok)?;
        let mess = fdata.map_rows(&ok, |x| self.mess.eval(x));
        let mut suitability = vec![f64::NAN; self.cells.len()];
        let mut mess_out = vec![f32::NAN; self.cells.len()];
        // `ok` is a subsequence of `cells` (both ascending).
        let mut j = 0;
        for (k, &c) in ok.iter().enumerate() {
            while self.cells[j] != c {
                j += 1;
            }
            suitability[j] = pred[k];
            mess_out[j] = mess[k] as f32;
        }
        Ok(ProjectedRun {
            suitability,
            mess: mess_out,
        })
    }
}

/// Significance level of the null-model test of the Boyce index (ADR-0014).
pub const NULL_ALPHA: f64 = 0.05;
/// Seed streams of the opt-in evaluation stages, mixed into the species seed so that they never
/// reuse the partition's random numbers.
const NULL_STREAM: u64 = 0x4E55_4C4C_5345_4544; // "NULLSEED"
const AOA_STREAM: u64 = 0x414F_4153_4545_4421; // "AOASEED!"
/// Random-forest seed streams (ADR-0017): forests, permutation importance, partial dependence.
const RF_STREAM: u64 = 0x5246_5345_4544_2121; // "RFSEED!!"
const IMPORTANCE_STREAM: u64 = 0x5045_524D_5345_4544; // "PERMSEED"
const PD_STREAM: u64 = 0x5044_5345_4544_2121; // "PDSEED!!"
/// Smallest automatic grid block, km: below a few cells blocks no longer separate training and
/// test data in space (the block is also never smaller than two grid cells).
const MIN_BLOCK_KM: f64 = 5.0;

/// Block size of `spatial_grid` validation: the user's value, or the median spatial
/// autocorrelation range of the predictors on the background sample (blockCV
/// `cv_spatial_autocor`, Valavi et al. 2019) clamped to [max(5 km, 2 cells), shorter side of the
/// background extent / 3]. The upper bound keeps at least 3 × 3 blocks over the study area, so
/// that k = 5 folds can usually be filled; the lower bound keeps blocks wider than the grid.
fn grid_block_size(
    validation: &ValidationSettings,
    data: &Data<'_>,
    background: &[usize],
    bc: &[(f64, f64)],
    rng: &mut Rng,
    warnings: &mut Vec<Warning>,
) -> BlockDesign {
    let k_requested = validation.folds.clamp(2, 10);
    if let Some(b) = validation.block_km {
        return BlockDesign {
            block_km: b,
            chosen_by: "user".into(),
            k_requested,
            k_achieved: k_requested,
            autocorrelation: None,
        };
    }
    let (mut lon0, mut lon1, mut lat0, mut lat1) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in bc {
        lon0 = lon0.min(x);
        lon1 = lon1.max(x);
        lat0 = lat0.min(y);
        lat1 = lat1.max(y);
    }
    let coslat = (0.5 * (lat0 + lat1)).to_radians().cos().max(0.01);
    let width_km = ((lon1 - lon0) * coslat * KM_PER_DEGREE).max(0.0);
    let height_km = ((lat1 - lat0) * KM_PER_DEGREE).max(0.0);
    let short_km = width_km.min(height_km);
    let cell_km = data.stack.spec.cell * KM_PER_DEGREE;
    let max_km = (short_km / 3.0).max(cell_km);
    let min_km = MIN_BLOCK_KM.max(2.0 * cell_km).min(max_km);
    let values: Vec<Vec<f64>> = data
        .vars
        .iter()
        .map(|&v| {
            background
                .iter()
                .map(|&c| f64::from(data.stack.layers[v].data[c]))
                .collect()
        })
        .collect();
    let est = autocorrelation_range_km(bc, &values, rng);
    let median = est
        .as_ref()
        .map(|e| e.median_km)
        .filter(|m| m.is_finite() && *m > 0.0);
    let block_km = match median {
        Some(m) => {
            let b = m.clamp(min_km, max_km);
            if b != m {
                warnings.push(
                    Warning::new(
                        "block_size_clamped",
                        Severity::Info,
                        format!(
                            "The median autocorrelation range of the predictors ({m:.0} km) was clamped to {b:.0} km to keep enough blocks for cross-validation."
                        ),
                    )
                    .with("median_km", m.round())
                    .with("block_km", b.round()),
                );
            }
            b
        }
        None => {
            let b = (short_km / 5.0).clamp(min_km, max_km);
            warnings.push(
                Warning::new(
                    "block_size_fallback",
                    Severity::Warning,
                    format!(
                        "The spatial autocorrelation range of the predictors could not be estimated; blocks of {b:.0} km (one fifth of the study-area extent) were used."
                    ),
                )
                .with("block_km", b.round()),
            );
            b
        }
    };
    let autocorrelation = AutocorrelationRange {
        median_km: median,
        min_km,
        max_km,
        n_points: est.as_ref().map_or(0, |e| e.n_points),
        cutoff_km: est.as_ref().map_or(f64::NAN, |e| e.cutoff_km),
        variables: data
            .vars
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let fit = est
                    .as_ref()
                    .and_then(|e| e.per_variable.get(i).cloned().flatten());
                VariableRange {
                    variable: data.stack.layers[v].name.clone(),
                    range_km: fit.as_ref().map(|f| f.range_km),
                    model: fit
                        .as_ref()
                        .map(|f| format!("{:?}", f.model).to_lowercase()),
                    at_upper_bound: fit.as_ref().is_some_and(|f| f.at_upper_bound),
                }
            })
            .collect(),
    };
    BlockDesign {
        block_km,
        chosen_by: "autocorrelation".into(),
        k_requested,
        k_achieved: k_requested,
        autocorrelation: Some(autocorrelation),
    }
}

/// Inputs of the null-model test of the selected setting.
struct NullRun<'a> {
    data: &'a Data<'a>,
    /// Cells null presences are drawn from.
    eligible: &'a [usize],
    /// Optional sampling weights aligned with `eligible` (e.g. a target-group effort surface so
    /// that nulls share the records' sampling bias, Bohl et al. 2019). `None` = uniform.
    weights: Option<&'a [f64]>,
    presences: usize,
    background: &'a [usize],
    bc: &'a [(f64, f64)],
    method: &'a str,
    /// Folds requested (random k-fold, spatial grid).
    k: usize,
    /// Block size actually used by `spatial_grid` validation (not re-estimated per null).
    block_km: Option<f64>,
    /// Selected setting, refitted on every null set.
    setting: &'a Setting,
    rf: RfConfig,
    seed: u64,
    /// Observed mean held-out Boyce index and OR10 of the selected setting.
    observed: (Option<f64>, f64),
}

/// Null models (Raes & ter Steege 2007; Bohl et al. 2019; ENMeval 2.0 `ENMnulls`): `reps` sets of
/// `presences` random cells, each partitioned with the study's validation method and fitted fold
/// by fold with the selected setting; their mean held-out Boyce index and OR10 form the null
/// distributions. Replicate `r` draws from its own seed, so results do not depend on thread
/// scheduling; every (replicate, fold) metric is checkpointed (bit-exact) for resumption.
fn null_models(
    run: &NullRun<'_>,
    reps: usize,
    checkpoints: &CvCheckpoints,
    cancel: &AtomicBool,
    progress: &(dyn Fn(&str, f64) + Sync),
) -> Result<NullModelSummary, AppError> {
    let spec = run.data.stack.spec;
    let center = |c: usize| spec.center(c / spec.width, c % spec.width);
    let mut sets: Vec<(Vec<usize>, Folds)> = Vec::with_capacity(reps);
    for r in 0..reps {
        let mut rng = Rng::new(
            run.seed
                .wrapping_add((r as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
        );
        let pres = null_presences(run.eligible, run.weights, run.presences, &mut rng);
        let pc: Vec<(f64, f64)> = pres.iter().map(|&c| center(c)).collect();
        let folds = match run.method {
            "random_kfold" => random_kfold(pres.len(), run.background.len(), run.k, &mut rng),
            "spatial_grid" => spatial_grid_blocks(
                &pc,
                run.bc,
                run.block_km.unwrap_or(f64::NAN),
                run.k,
                &mut rng,
            ),
            _ => spatial_blocks(&pc, run.bc),
        };
        sets.push((pres, folds));
    }
    let tasks: Vec<(usize, usize)> = sets
        .iter()
        .enumerate()
        .flat_map(|(r, (_, f))| (0..f.k).map(move |fold| (r, fold)))
        .collect();
    let total = tasks.len();
    let done = AtomicUsizeCounter::default();
    let results: Vec<Result<(usize, Option<FoldMetrics>), AppError>> = tasks
        .par_iter()
        .map(|&(r, f)| {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Cancelled);
            }
            let name = format!("null-{r}-{f}");
            let metrics = match checkpoints.load_metrics(&name) {
                Some(m) => Some(m),
                None => {
                    let (pres, folds) = &sets[r];
                    match fit_fold(
                        run.data,
                        pres,
                        run.background,
                        folds,
                        f,
                        run.setting,
                        run.rf,
                        cancel,
                    ) {
                        Ok((m, _, _)) => {
                            checkpoints.save_metrics(&name, &m);
                            Some(m)
                        }
                        Err(AppError::Cancelled) => return Err(AppError::Cancelled),
                        // A degenerate null set (e.g. a fold without data) is left out.
                        Err(_) => None,
                    }
                }
            };
            let d = done.inc();
            progress(
                &format!("Null models: {d}/{total} models"),
                0.65 + 0.05 * d as f64 / total.max(1) as f64,
            );
            Ok((r, metrics))
        })
        .collect();
    let mut per_rep: Vec<Vec<Option<FoldMetrics>>> = sets.iter().map(|_| Vec::new()).collect();
    for res in results {
        let (r, m) = res?;
        per_rep[r].push(m);
    }
    let mean = |v: &[f64]| -> f64 {
        let w: Vec<f64> = v.iter().copied().filter(|x| x.is_finite()).collect();
        if w.is_empty() {
            f64::NAN
        } else {
            w.iter().sum::<f64>() / w.len() as f64
        }
    };
    let (mut null_cbi, mut null_or10) = (Vec::new(), Vec::new());
    for fold_metrics in &per_rep {
        // A replicate counts only when every fold was fitted (as the observed means).
        let Some(ms) = fold_metrics.iter().cloned().collect::<Option<Vec<_>>>() else {
            continue;
        };
        if ms.is_empty() {
            continue;
        }
        let cbis: Vec<f64> = ms.iter().filter_map(|m| m.cbi).collect();
        null_cbi.push(mean(&cbis));
        null_or10.push(mean(&ms.iter().map(|m| m.or10).collect::<Vec<_>>()));
    }
    let fin = |x: f64| x.is_finite().then_some(x);
    let stat = |observed: f64, nulls: &[f64], higher: bool| {
        let s = null_summary(observed, nulls, higher);
        NullStat {
            observed: fin(observed),
            mean: fin(s.mean),
            sd: fin(s.sd),
            z: fin(s.z),
            p_value: s.p_value,
            n: s.n,
        }
    };
    Ok(NullModelSummary {
        requested: reps,
        completed: null_cbi.len(),
        presences: run.presences,
        sampling: if run.weights.is_some() {
            "effort_weighted".into()
        } else {
            "uniform".into()
        },
        cbi: stat(run.observed.0.unwrap_or(f64::NAN), &null_cbi, true),
        or10: stat(run.observed.1, &null_or10, false),
    })
}

/// Area of applicability (Meyer & Pebesma 2021; CAST `aoa`) of the final model: training rows are
/// its presences and background, weights its permutation importance (normalised; equal weights
/// when every importance is 0), folds those of the cross-validation (background shared by all
/// random k-fold folds is spread over the folds by index). Returns the summary and the
/// dissimilarity index of every cell of `cells`.
fn area_of_applicability(
    data: &Data<'_>,
    presence_cells: &[usize],
    background: &[usize],
    folds: &Folds,
    permutation: &[(String, f64)],
    cells: &[usize],
    seed: u64,
) -> Result<(AoaSummary, Vec<f64>), AppError> {
    let nv = data.vars.len();
    let rows: Vec<Vec<f64>> = presence_cells
        .iter()
        .chain(background)
        .map(|&c| {
            let mut row = vec![0.0; nv];
            data.fill(c, &mut row);
            row
        })
        .collect();
    let labels: Vec<usize> = folds
        .presence
        .iter()
        .copied()
        .chain(
            folds
                .background
                .iter()
                .enumerate()
                .map(|(i, &f)| if f >= folds.k { i % folds.k.max(1) } else { f }),
        )
        .collect();
    let ids = data.names();
    let raw: Vec<f64> = ids
        .iter()
        .map(|id| {
            permutation
                .iter()
                .find(|(v, _)| v == id)
                .map_or(0.0, |(_, w)| *w)
        })
        .map(|w| if w.is_finite() && w > 0.0 { w } else { 0.0 })
        .collect();
    let total: f64 = raw.iter().sum();
    let weights: Vec<f64> = if total > 0.0 {
        raw.iter().map(|w| w / total).collect()
    } else {
        vec![1.0 / nv.max(1) as f64; nv]
    };
    let mut rng = Rng::new(seed);
    let aoa = Aoa::new(&rows, Some(&weights), &labels, &mut rng)
        .map_err(|e| AppError::Model(format!("area of applicability: {e}")))?;
    let di = data.map_rows(cells, |x| aoa.eval(x));
    let threshold = aoa.threshold();
    let outside = di.iter().filter(|d| **d > threshold).count();
    Ok((
        AoaSummary {
            threshold,
            fraction_outside: outside as f64 / cells.len().max(1) as f64,
            n_train: aoa.n_train(),
            weights: data
                .vars
                .iter()
                .zip(&weights)
                .map(|(&v, &w)| VariableWeight {
                    variable: data.stack.layers[v].name.clone(),
                    weight: w,
                })
                .collect(),
        },
        di,
    ))
}

/// Train one fold model (all presences and background outside fold `f`) and evaluate it on the
/// held-out presences and background of fold `f`. Background assigned to fold `folds.k` is shared
/// by every fold (random k-fold).
#[allow(clippy::too_many_arguments)]
fn fit_fold(
    data: &Data<'_>,
    presence_cells: &[usize],
    background: &[usize],
    folds: &Folds,
    f: usize,
    setting: &Setting,
    rf: RfConfig,
    cancel: &AtomicBool,
) -> Result<(FoldMetrics, usize, Fitted), AppError> {
    let s = split(presence_cells, background, folds, f);
    let model = fit_model(data, setting, &s.tr_p, &s.tr_b, rf, cancel)?;
    let metrics = evaluate(data, &model, &s, f)?;
    Ok((metrics, model.nonzero(), model))
}

/// Rule-based evidence level. Cut-offs are an explicit expert decision (ADR-0009):
/// - sample size: Wisz et al. 2008, van Proosdij et al. 2016;
/// - OR10 expected ≈ 0.10 for a model that is not overfit (Radosavljevic & Anderson 2014);
/// - continuous Boyce index: > 0 better than random, near 1 consistent (Hirzel et al. 2006);
/// - extrapolation share from MESS (Elith et al. 2010).
pub fn evidence_level(n: usize, v: &ValidationSummary, novel: f64, method: &str) -> Evidence {
    let cbi = v.cbi_mean.unwrap_or(f64::NEG_INFINITY);
    let spatial = matches!(method, "spatial_block" | "spatial_grid");
    let mut reasons = Vec::new();
    let check = |ok: bool, text: String, reasons: &mut Vec<String>| {
        reasons.push(format!("{} {text}", if ok { "✓" } else { "✗" }));
        ok
    };
    let supported = [
        check(
            n >= 50,
            format!("{n} presence cells (supported needs ≥ 50)"),
            &mut reasons,
        ),
        check(
            cbi >= 0.7,
            format!(
                "mean continuous Boyce index {} (supported needs ≥ 0.70)",
                fmt_opt(v.cbi_mean)
            ),
            &mut reasons,
        ),
        check(
            v.or10_mean <= 0.15,
            format!(
                "mean 10th-percentile omission {:.2} (supported needs ≤ 0.15; 0.10 expected)",
                v.or10_mean
            ),
            &mut reasons,
        ),
        check(
            novel <= 0.10,
            format!(
                "{:.0} % of the area extrapolated (supported needs ≤ 10 %)",
                novel * 100.0
            ),
            &mut reasons,
        ),
        check(
            spatial,
            "spatially blocked cross-validation".into(),
            &mut reasons,
        ),
    ]
    .iter()
    .all(|b| *b);
    let indicative = n >= 25 && cbi >= 0.4 && v.or10_mean <= 0.25 && novel <= 0.25;
    let mut level = if supported {
        EvidenceLevel::Supported
    } else if indicative {
        EvidenceLevel::Indicative
    } else {
        EvidenceLevel::Exploratory
    };
    if !supported && indicative {
        reasons.push(
            "Indicative: ≥ 25 presences, Boyce ≥ 0.40, omission ≤ 0.25, extrapolation ≤ 25 %."
                .into(),
        );
    } else if !supported {
        reasons.push("Exploratory: at least one indicative criterion not met (≥ 25 presences, Boyce ≥ 0.40, omission ≤ 0.25, extrapolation ≤ 25 %).".into());
    }
    // Null models (ADR-0014, opt-in): a model that does not beat random-location models on the
    // Boyce index is capped at Exploratory. Nothing is added when null models were not run.
    if let Some(nm) = &v.null_model {
        if nm.completed == 0 {
            reasons.push(
                "Null models requested but none could be fitted: significance not assessed.".into(),
            );
        } else if nm.cbi.p_value > NULL_ALPHA {
            if matches!(level, EvidenceLevel::Indicative | EvidenceLevel::Supported) {
                level = EvidenceLevel::Exploratory;
            }
            reasons.push(format!(
                "✗ Boyce index not better than {} random-location null models (p = {:.3} > {NULL_ALPHA}) — capped at Exploratory",
                nm.completed, nm.cbi.p_value
            ));
        } else {
            reasons.push(format!(
                "✓ Boyce index better than {} random-location null models (p = {:.3} ≤ {NULL_ALPHA})",
                nm.completed, nm.cbi.p_value
            ));
        }
    }
    reasons.push("No independent validation data: evidence comes from cross-validation of the same, possibly biased, records.".into());
    Evidence { level, reasons }
}

fn fmt_opt(v: Option<f64>) -> String {
    v.map_or_else(|| "undefined".into(), |x| format!("{x:.2}"))
}

fn map_train(e: niche_maxent::TrainError) -> AppError {
    match e {
        niche_maxent::TrainError::Cancelled => AppError::Cancelled,
        other => AppError::Model(other.to_string()),
    }
}

#[derive(Default)]
struct AtomicUsizeCounter(std::sync::atomic::AtomicUsize);

impl AtomicUsizeCounter {
    fn inc(&self) -> usize {
        self.0.fetch_add(1, Ordering::Relaxed) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(null_model: Option<NullModelSummary>) -> ValidationSummary {
        ValidationSummary {
            method: "spatial_block".into(),
            folds: vec![],
            cbi_mean: Some(0.9),
            or10_mean: 0.1,
            ormtp_mean: 0.0,
            auc_mean: 0.8,
            blocks: None,
            null_model,
            members: vec![],
        }
    }

    fn nulls(p_value: f64, completed: usize) -> NullModelSummary {
        let stat = NullStat {
            observed: Some(0.9),
            mean: Some(0.5),
            sd: Some(0.1),
            z: Some(4.0),
            p_value,
            n: completed,
        };
        NullModelSummary {
            requested: 20,
            completed,
            presences: 100,
            sampling: "uniform".into(),
            cbi: stat.clone(),
            or10: stat,
        }
    }

    #[test]
    fn without_null_models_the_evidence_rules_are_unchanged() {
        let e = evidence_level(100, &summary(None), 0.0, "spatial_block");
        assert_eq!(e.level, EvidenceLevel::Supported);
        assert!(!e.reasons.iter().any(|r| r.contains("null")));
        assert_eq!(e.reasons.len(), 6, "{:?}", e.reasons);
    }

    #[test]
    fn a_model_not_better_than_null_models_is_capped_at_exploratory() {
        let e = evidence_level(100, &summary(Some(nulls(0.2, 20))), 0.0, "spatial_block");
        assert_eq!(e.level, EvidenceLevel::Exploratory);
        assert!(
            e.reasons
                .iter()
                .any(|r| r.starts_with('✗') && r.contains("capped at Exploratory")),
            "{:?}",
            e.reasons
        );
        // Exactly at the significance level: not capped.
        let e = evidence_level(
            100,
            &summary(Some(nulls(NULL_ALPHA, 20))),
            0.0,
            "spatial_block",
        );
        assert_eq!(e.level, EvidenceLevel::Supported);
        assert!(
            e.reasons
                .iter()
                .any(|r| r.starts_with("✓ Boyce index better"))
        );
    }

    #[test]
    fn null_models_that_could_not_be_fitted_do_not_cap() {
        let e = evidence_level(100, &summary(Some(nulls(1.0, 0))), 0.0, "spatial_block");
        assert_eq!(e.level, EvidenceLevel::Supported);
        assert!(e.reasons.iter().any(|r| r.contains("none could be fitted")));
    }

    #[test]
    fn mtry_candidates() {
        assert_eq!(rf_mtry_candidates(1, true), vec![1]);
        assert_eq!(rf_mtry_candidates(3, true), vec![1]);
        assert_eq!(rf_mtry_candidates(4, true), vec![2]);
        assert_eq!(rf_mtry_candidates(7, false), vec![2]);
        assert_eq!(rf_mtry_candidates(7, true), vec![2, 3]);
        assert_eq!(rf_mtry_candidates(19, true), vec![4, 9]);
    }

    #[test]
    fn percentile_ranks_use_mid_ranks() {
        let r = [0.0, 0.0, 0.5, 1.0];
        assert_eq!(percentile_rank(&r, 0.0), 0.25);
        assert_eq!(percentile_rank(&r, 0.25), 0.5);
        assert_eq!(percentile_rank(&r, 0.5), 0.625);
        assert_eq!(percentile_rank(&r, 2.0), 1.0);
        assert_eq!(percentile_rank(&r, -1.0), 0.0);
        assert!(percentile_rank(&r, f64::NAN).is_nan());
        assert!(percentile_rank(&[], 0.5).is_nan());
    }

    #[test]
    fn algorithms_parse() {
        assert_eq!(Algorithm::parse("maxent").ok(), Some(Algorithm::Maxent));
        assert_eq!(Algorithm::parse("ensemble").ok(), Some(Algorithm::Ensemble));
        assert!(Algorithm::parse("RF").is_err());
    }

    #[test]
    fn spatial_grid_blocks_count_as_spatially_blocked_validation() {
        for (method, level) in [
            ("spatial_block", EvidenceLevel::Supported),
            ("spatial_grid", EvidenceLevel::Supported),
            ("random_kfold", EvidenceLevel::Indicative),
        ] {
            let e = evidence_level(100, &summary(None), 0.0, method);
            assert_eq!(e.level, level, "{method}");
        }
    }
}
