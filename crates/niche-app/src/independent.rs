//! Independent presence–absence validation of the final prediction with the organisation's own
//! data (ADR-0108). Computed after the model, from the final suitability raster; it never
//! changes the model.
//!
//! - **Sites.** The species' records of the dataset (role `validation`: all of them; role
//!   `training`: its absences only, its presences being training data) pass the same cleaning
//!   rules as presences (dated, year, precision, study area, land, predictor data), then are
//!   reduced to one site per grid cell: a cell with at least one presence record is a presence
//!   site (detection wins over non-detection), otherwise an absence site.
//! - **Leakage rule.** A record in a cell used as a training presence (from GBIF or the
//!   organisation) is excluded (`training_cell`): its outcome is already known to the model.
//!   This also removes the organisation's observations that GBIF republishes (SINP → INPN →
//!   GBIF) in the same cell. Background cells are not excluded (background is not absence).
//! - **Metrics** (Allouche et al. 2006; Leroy et al. 2018): presence–absence AUC, sensitivity,
//!   specificity, TSS and kappa at two thresholds of the *training* data (P10 and maximum
//!   sensitivity + specificity, never optimised on the validation data), Brier score and an
//!   equal-count calibration table. Suitability is not a probability: Brier and calibration are
//!   diagnostics of how the relative suitability ranks and spreads, never a relabelling.
//! - **Validated** (ADR-0009 §6, criteria proposed in ADR-0108, pending the ecologists' review):
//!   held-out data (role `validation`), ≥ 30 presence and ≥ 30 absence sites after the leakage
//!   rule, PA AUC ≥ 0.75, TSS ≥ 0.40 at the training max-SSS threshold, and every criterion of
//!   Supported met (the level is Supported after every cap).

use std::collections::{BTreeMap, HashSet};

use niche_eval::{ConfusionMatrix, KdTree, calibration_table, pa_metrics};

use crate::dto::{
    CalibrationRow, Evidence, EvidenceLevel, IndependentValidation, OccurrenceFilters,
    OccurrencePoint, OwnDataRole, PaThreshold, SiteSpread, ThresholdArea, ValidationSites,
};
use crate::env::EnvStack;
use crate::owndata::OwnRecord;

/// Fewer presence or absence sites than this: no metrics (they would be noise).
pub const MIN_PA_SITES: usize = 10;
/// Criteria of the Validated level (ADR-0108, proposed).
pub const VALIDATED_MIN_SITES: usize = 30;
pub const VALIDATED_MIN_AUC: f64 = 0.75;
pub const VALIDATED_MIN_TSS: f64 = 0.40;
/// Start of the evidence reason replaced when independent data were used.
pub const NO_INDEPENDENT_PREFIX: &str = "No independent validation data";

/// Validation sites of a species.
pub struct Sites {
    pub presence_cells: Vec<usize>,
    pub absence_cells: Vec<usize>,
    pub summary: ValidationSites,
    /// One point per record (`validation`, `validation_absence`, `validation_excluded`).
    pub points: Vec<OccurrencePoint>,
}

/// Clean the validation records and reduce them to one site per cell (see module docs).
pub fn sites(
    records: &[&OwnRecord],
    stack: &EnvStack,
    f: &OccurrenceFilters,
    training_cells: &[usize],
) -> Sites {
    let training: HashSet<usize> = training_cells.iter().copied().collect();
    let mut excluded: BTreeMap<String, usize> = BTreeMap::new();
    let mut cells: BTreeMap<usize, (bool, bool)> = BTreeMap::new();
    let mut leaked: HashSet<usize> = HashSet::new();
    let mut points = Vec::with_capacity(records.len());
    for r in records {
        let cell = stack
            .spec
            .cell_of(r.lng, r.lat)
            .map(|(row, col)| stack.spec.index(row, col))
            .filter(|&i| stack.area_mask[i]);
        let status: Result<usize, &str> = match (r.year, cell) {
            (None, _) => Err("undated"),
            (Some(y), _) if y < f.year_min => Err("too_old"),
            _ if r.uncertainty_m.is_some_and(|u| u > f.max_uncertainty_m) => Err("uncertain"),
            (_, None) => Err("outside_area"),
            (_, Some(i)) if !stack.land_mask[i] => Err("not_land"),
            (_, Some(i)) if !stack.valid[i] => Err("no_env"),
            (_, Some(i)) if training.contains(&i) => {
                leaked.insert(i);
                Err("training_cell")
            }
            (_, Some(i)) => Ok(i),
        };
        let code = match status {
            Ok(i) => {
                let e = cells.entry(i).or_insert((false, false));
                if r.absent {
                    e.1 = true;
                    "validation_absence"
                } else {
                    e.0 = true;
                    "validation"
                }
            }
            Err(code) => {
                *excluded.entry(code.to_string()).or_insert(0) += 1;
                "validation_excluded"
            }
        };
        points.push(OccurrencePoint {
            lng: r.lng,
            lat: r.lat,
            year: r.year,
            status: code.into(),
            source: Some("own".into()),
        });
    }
    let presence_cells: Vec<usize> = cells.iter().filter(|(_, v)| v.0).map(|(c, _)| *c).collect();
    let absence_cells: Vec<usize> = cells
        .iter()
        .filter(|(_, v)| !v.0)
        .map(|(c, _)| *c)
        .collect();
    let conflicting = cells.values().filter(|v| v.0 && v.1).count();
    Sites {
        summary: ValidationSites {
            records: records.len(),
            excluded,
            presence_sites: presence_cells.len(),
            absence_sites: absence_cells.len(),
            conflicting_cells: conflicting,
            leakage_excluded: leaked.len(),
        },
        presence_cells,
        absence_cells,
        points,
    }
}

fn fin(x: f64) -> Option<f64> {
    x.is_finite().then_some(x)
}

/// Bounding box of the study area's cells, in cell centres (west, south, east, north).
fn area_bbox(stack: &EnvStack) -> Option<[f64; 4]> {
    let cells: Vec<usize> = (0..stack.area_mask.len())
        .filter(|&i| stack.area_mask[i])
        .collect();
    cells_bbox(stack, &cells)
}

fn cells_bbox(stack: &EnvStack, cells: &[usize]) -> Option<[f64; 4]> {
    let w = stack.spec.width;
    cells.iter().map(|&c| stack.spec.center(c / w, c % w)).fold(
        None,
        |b: Option<[f64; 4]>, (x, y)| {
            Some(match b {
                None => [x, y, x, y],
                Some(b) => [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
            })
        },
    )
}

/// East–west and north–south size of a box in km, one cell added so that a single cell has the
/// size of a cell.
fn box_km(stack: &EnvStack, b: [f64; 4]) -> [f64; 2] {
    let (mx, my) = niche_geo::crs::metres_per_degree((b[1] + b[3]) / 2.0);
    let c = stack.spec.cell;
    [
        (b[2] - b[0] + c) * mx / 1000.0,
        (b[3] - b[1] + c) * my / 1000.0,
    ]
}

/// Area of the bounding box of `cells` as a share of the study area's bounding box.
pub fn bbox_share(stack: &EnvStack, cells: &[usize]) -> Option<f64> {
    let a = box_km(stack, area_bbox(stack)?);
    let s = box_km(stack, cells_bbox(stack, cells)?);
    let total = a[0] * a[1];
    (total > 0.0).then(|| (s[0] * s[1] / total).min(1.0))
}

/// Spread of the sites: extent, share of the study extent, mean nearest-neighbour distance.
pub fn spread(stack: &EnvStack, cells: &[usize]) -> Option<SiteSpread> {
    let b = cells_bbox(stack, cells)?;
    let w = stack.spec.width;
    let (mx, my) = niche_geo::crs::metres_per_degree((b[1] + b[3]) / 2.0);
    let pts: Vec<Vec<f64>> = cells
        .iter()
        .map(|&c| {
            let (x, y) = stack.spec.center(c / w, c % w);
            vec![x * mx / 1000.0, y * my / 1000.0]
        })
        .collect();
    let mean_nearest_km = KdTree::new(&pts).and_then(|t| {
        let d: Vec<f64> = (0..pts.len())
            .filter_map(|i| t.nearest_excluding(&pts[i], i).map(|(_, d)| d))
            .collect();
        (!d.is_empty()).then(|| d.iter().sum::<f64>() / d.len() as f64)
    });
    Some(SiteSpread {
        extent_km: box_km(stack, b),
        share_of_study_extent: bbox_share(stack, cells).unwrap_or(0.0),
        mean_nearest_km,
    })
}

/// Metrics of the final prediction on the sites. `thresholds`: those of the training data
/// (`p10`, `max_sss`).
#[allow(clippy::too_many_arguments)]
pub fn evaluate(
    dataset_id: &str,
    dataset_name: &str,
    role: OwnDataRole,
    synthetic: bool,
    sites: &Sites,
    suitability: &[f32],
    thresholds: &[ThresholdArea],
    stack: &EnvStack,
) -> IndependentValidation {
    let score = |cells: &[usize]| -> Vec<f64> {
        cells
            .iter()
            .filter_map(|&c| suitability.get(c).map(|&v| f64::from(v)))
            .filter(|v| v.is_finite())
            .collect()
    };
    let pres = score(&sites.presence_cells);
    let abs = score(&sites.absence_cells);
    let all: Vec<usize> = sites
        .presence_cells
        .iter()
        .chain(&sites.absence_cells)
        .copied()
        .collect();
    let mut v = IndependentValidation {
        dataset_id: dataset_id.into(),
        dataset_name: dataset_name.into(),
        role,
        synthetic,
        sites: sites.summary.clone(),
        auc: None,
        thresholds: Vec::new(),
        brier: None,
        prevalence: None,
        calibration: Vec::new(),
        spread: spread(stack, &all),
        note: None,
        validated: false,
    };
    if pres.len() < MIN_PA_SITES || abs.len() < MIN_PA_SITES {
        v.note = Some(if role == OwnDataRole::Training && pres.is_empty() {
            format!(
                "The organisation's presences were used for training, so no independent presence site remains ({} absence sites): mark the dataset as held out for validation to evaluate the model on it.",
                abs.len()
            )
        } else {
            format!(
                "Too few independent sites for presence–absence metrics: {} presence and {} absence sites after cleaning and the leakage rule (at least {MIN_PA_SITES} of each are needed).",
                pres.len(),
                abs.len()
            )
        });
        return v;
    }
    for rule in ["p10", "max_sss"] {
        let Some(t) = thresholds.iter().find(|t| t.rule == rule) else {
            continue;
        };
        let cm = ConfusionMatrix::at_threshold(&pres, &abs, t.threshold);
        v.thresholds.push(PaThreshold {
            rule: rule.into(),
            threshold: t.threshold,
            sensitivity: fin(cm.sensitivity()),
            specificity: fin(cm.specificity()),
            tss: fin(cm.tss()),
            kappa: fin(cm.kappa()),
            true_pos: cm.true_pos,
            false_pos: cm.false_pos,
            false_neg: cm.false_neg,
            true_neg: cm.true_neg,
        });
    }
    let m = pa_metrics(&pres, &abs, 0.5);
    v.auc = fin(m.auc);
    v.brier = fin(m.brier);
    v.prevalence = fin(m.prevalence);
    let bins = ((pres.len() + abs.len()) / 10).clamp(2, 10);
    v.calibration = calibration_table(&pres, &abs, bins)
        .into_iter()
        .map(|b| CalibrationRow {
            lower: b.lower,
            upper: b.upper,
            mean_suitability: b.mean_predicted,
            observed_presence_share: b.observed_frequency,
            n: b.n,
        })
        .collect();
    v
}

/// TSS at the training max-SSS threshold.
pub fn tss_max_sss(v: &IndependentValidation) -> Option<f64> {
    v.thresholds
        .iter()
        .find(|t| t.rule == "max_sss")
        .and_then(|t| t.tss)
}

/// Replace the "no independent validation data" reason, check the Validated criteria and raise
/// the level when all are met (ADR-0108). Call after every cap of the level.
pub fn apply_to_evidence(e: &mut Evidence, v: &mut IndependentValidation) {
    e.reasons.retain(|r| !r.starts_with(NO_INDEPENDENT_PREFIX));
    let synthetic = if v.synthetic { " (synthetic data)" } else { "" };
    let held_out = v.role == OwnDataRole::Validation;
    let Some(auc) = v.auc else {
        e.reasons.push(format!(
            "Independent data provided ({}{synthetic}) but not usable for validation: {} Evidence comes from cross-validation of the same, possibly biased, records.",
            v.dataset_name,
            v.note.as_deref().unwrap_or("no metrics.")
        ));
        return;
    };
    let tss = tss_max_sss(v);
    e.reasons.push(format!(
        "Independent presence–absence validation on the organisation's data ({}{synthetic}; {}): {} presence and {} absence sites, presence–absence AUC {auc:.2}, TSS {} at the training max-SSS threshold.",
        v.dataset_name,
        if held_out { "held out from training" } else { "absences of a training dataset" },
        v.sites.presence_sites,
        v.sites.absence_sites,
        tss.map_or("undefined".into(), |t| format!("{t:.2}"))
    ));
    let mut checks = Vec::new();
    let mut check = |ok: bool, text: String| {
        checks.push(format!("{} {text}", if ok { "✓" } else { "✗" }));
        ok
    };
    let all = [
        check(
            held_out,
            "validation data held out from training (validated needs held-out data)".into(),
        ),
        check(
            v.sites.presence_sites >= VALIDATED_MIN_SITES
                && v.sites.absence_sites >= VALIDATED_MIN_SITES,
            format!(
                "{} presence and {} absence independent sites (validated needs ≥ {VALIDATED_MIN_SITES} of each)",
                v.sites.presence_sites, v.sites.absence_sites
            ),
        ),
        check(
            auc >= VALIDATED_MIN_AUC,
            format!("presence–absence AUC {auc:.2} (validated needs ≥ {VALIDATED_MIN_AUC:.2})"),
        ),
        check(
            tss.is_some_and(|t| t >= VALIDATED_MIN_TSS),
            format!(
                "TSS {} at the training max-SSS threshold (validated needs ≥ {VALIDATED_MIN_TSS:.2})",
                tss.map_or("undefined".into(), |t| format!("{t:.2}"))
            ),
        ),
        check(
            e.level == EvidenceLevel::Supported,
            "every criterion of Supported met (validated needs Supported first)".into(),
        ),
    ]
    .iter()
    .all(|b| *b);
    e.reasons.extend(checks);
    v.validated = all;
    if all {
        e.level = EvidenceLevel::Validated;
        e.reasons.push(
            "Validated: the model is Supported and confirmed on independent presence–absence data (criteria proposed in ADR-0108, pending review by ecologists).".into(),
        );
    }
}

pub const REF_ALLOUCHE_2006: &str = "Allouche, O., Tsoar, A. & Kadmon, R. (2006) Assessing the accuracy of species distribution models: prevalence, kappa and the true skill statistic (TSS). Journal of Applied Ecology 43, 1223–1232.";
pub const REF_LEROY_2018: &str = "Leroy, B. et al. (2018) Without quality presence–absence data, discrimination metrics such as TSS can be misleading measures of model performance. Journal of Biogeography 45, 1994–2002.";
pub const REF_LIU_2013: &str = "Liu, C., White, M. & Newell, G. (2013) Selecting thresholds for the prediction of species occurrence with presence-only data. Journal of Biogeography 40, 778–789.";
pub const REF_BRIER_1950: &str = "Brier, G.W. (1950) Verification of forecasts expressed in terms of probability. Monthly Weather Review 78, 1–3.";

/// Method paragraph of the independent validation (reports).
pub const METHOD_TEXT: &str = "The final prediction was evaluated on the organisation's own presence–absence records (surveyed sites where the species was found or not found). These records passed the same cleaning rules as the training presences and were reduced to one site per grid cell (a cell with any detection is a presence site). Sites in cells used as training presences were excluded, so that no evaluated outcome was known to the model (this also removes observations republished in GBIF). Discrimination is measured by the presence–absence AUC, and by sensitivity, specificity, TSS and Cohen's kappa at two thresholds taken from the training data, never tuned on the validation data: the 10th-percentile training presence and the maximum of sensitivity + specificity (Allouche et al. 2006; Liu et al. 2013). TSS depends on prevalence when errors are unbalanced: read it with sensitivity and specificity (Leroy et al. 2018). Suitability is relative, not a probability of presence: the Brier score (Brier 1950) and the calibration table only describe how suitability values relate to observed outcomes at these sites, as a diagnostic; they do not turn suitability into a probability.";

fn f2(x: Option<f64>) -> String {
    x.map_or("–".into(), |v| format!("{v:.2}"))
}

fn rule_label(rule: &str) -> &str {
    match rule {
        "p10" => "10th percentile training presence",
        "max_sss" => "maximum sensitivity + specificity (training)",
        other => other,
    }
}

/// "Occurrence data sources" section of `report.md` (organisation's data used): sources and
/// counts, never coordinates.
pub fn sources_md(r: &crate::dto::SpeciesResult, spec: &crate::dto::JobSpec) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let Some(o) = spec
        .species
        .iter()
        .find(|x| x.key == r.species_key)
        .and_then(|x| x.own.as_ref())
    else {
        return s;
    };
    let _ = writeln!(s, "\n## Occurrence data sources\n");
    let _ = writeln!(
        s,
        "- The organisation's own data ({}): {}; taxon values {}. Licence: the organisation's own data (ADR-0108). Points are not reproduced in this report.",
        r.independent_validation
            .as_ref()
            .map_or("imported dataset".to_string(), |v| format!(
                "dataset “{}”{}",
                v.dataset_name,
                if v.synthetic { ", synthetic" } else { "" }
            )),
        crate::owndata::role_text(o.role, o.gbif),
        o.values
            .iter()
            .map(|v| format!("“{v}”"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for b in &r.data.by_source {
        let label = if b.source == "own" {
            "Organisation's data"
        } else if spec.demo {
            "Synthetic demo records"
        } else {
            "GBIF"
        };
        let detail = b
            .statuses
            .iter()
            .filter(|(k, _)| k.as_str() != "retained")
            .map(|(k, n)| format!("{k} {n}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            s,
            "- {label}: {} records, {} presence cells kept for training{}{}.",
            b.records,
            b.retained_cells,
            b.in_own_cells
                .map(|n| format!(", {n} records in cells already given by the organisation's data"))
                .unwrap_or_default(),
            if detail.is_empty() {
                String::new()
            } else {
                format!(" (other records: {detail})")
            }
        );
    }
    s
}

/// "Independent validation (presence–absence)" section of `report.md`.
pub fn validation_md(r: &crate::dto::SpeciesResult) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let Some(v) = &r.independent_validation else {
        return s;
    };
    let _ = writeln!(s, "\n## Independent validation (presence–absence)\n");
    let _ = writeln!(s, "{METHOD_TEXT}\n");
    let _ = writeln!(
        s,
        "- Data: “{}”{} — {}. {} records considered; {} presence and {} absence sites; {} cells with both outcomes counted as presence; {} sites excluded by the leakage rule.",
        v.dataset_name,
        if v.synthetic { " (SYNTHETIC)" } else { "" },
        if v.role == OwnDataRole::Validation {
            "held out from training"
        } else {
            "absences of a dataset whose presences were used for training"
        },
        v.sites.records,
        v.sites.presence_sites,
        v.sites.absence_sites,
        v.sites.conflicting_cells,
        v.sites.leakage_excluded
    );
    if !v.sites.excluded.is_empty() {
        let _ = writeln!(
            s,
            "- Records excluded: {}.",
            v.sites
                .excluded
                .iter()
                .map(|(k, n)| format!("{k} {n}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if let Some(sp) = &v.spread {
        let _ = writeln!(
            s,
            "- Spread: sites span {:.0} × {:.0} km ({:.0} % of the study area's extent); mean distance to the nearest other site {}.",
            sp.extent_km[0],
            sp.extent_km[1],
            sp.share_of_study_extent * 100.0,
            sp.mean_nearest_km
                .map_or("–".into(), |d| format!("{d:.1} km"))
        );
    }
    if let Some(n) = &v.note {
        let _ = writeln!(s, "- {n}");
    }
    if let Some(auc) = v.auc {
        let _ = writeln!(
            s,
            "- Presence–absence AUC {auc:.3}; Brier score {} (diagnostic); prevalence of the sites {}.",
            v.brier.map_or("–".into(), |b| format!("{b:.3}")),
            f2(v.prevalence)
        );
        let _ = writeln!(
            s,
            "\n| threshold (training data) | value | sensitivity | specificity | TSS | kappa | TP | FP | FN | TN |\n|---|---|---|---|---|---|---|---|---|---|"
        );
        for t in &v.thresholds {
            let _ = writeln!(
                s,
                "| {} | {:.3} | {} | {} | {} | {} | {} | {} | {} | {} |",
                rule_label(&t.rule),
                t.threshold,
                f2(t.sensitivity),
                f2(t.specificity),
                f2(t.tss),
                f2(t.kappa),
                t.true_pos,
                t.false_pos,
                t.false_neg,
                t.true_neg
            );
        }
        if !v.calibration.is_empty() {
            let _ = writeln!(
                s,
                "\nCalibration table (diagnostic only; suitability is not a probability of presence):\n\n| suitability range | mean suitability | share of presence sites | sites |\n|---|---|---|---|"
            );
            for c in &v.calibration {
                let _ = writeln!(
                    s,
                    "| {:.3}–{:.3} | {:.3} | {:.2} | {} |",
                    c.lower, c.upper, c.mean_suitability, c.observed_presence_share, c.n
                );
            }
        }
    }
    let _ = writeln!(
        s,
        "- Validated level: {} (criteria proposed in ADR-0108, pending review by ecologists: held-out data, ≥ {VALIDATED_MIN_SITES} presence and ≥ {VALIDATED_MIN_SITES} absence sites, AUC ≥ {VALIDATED_MIN_AUC}, TSS ≥ {VALIDATED_MIN_TSS} at the training max-SSS threshold, and Supported).",
        if v.validated {
            "criteria met"
        } else {
            "criteria not met"
        }
    );
    let _ = writeln!(
        s,
        "\nReferences: {REF_ALLOUCHE_2006} {REF_LEROY_2018} {REF_LIU_2013} {REF_BRIER_1950}"
    );
    s
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// "Occurrence data sources" section of `report.html`.
pub fn sources_html(r: &crate::dto::SpeciesResult, spec: &crate::dto::JobSpec) -> String {
    let md = sources_md(r, spec);
    if md.is_empty() {
        return md;
    }
    let items: String = md
        .lines()
        .filter_map(|l| l.strip_prefix("- "))
        .map(|l| format!("<li>{}</li>", esc(l)))
        .collect();
    format!("<h2>Occurrence data sources</h2><ul>{items}</ul>")
}

/// "Independent validation (presence–absence)" section of `report.html`.
pub fn validation_html(r: &crate::dto::SpeciesResult) -> String {
    let Some(v) = &r.independent_validation else {
        return String::new();
    };
    let mut s = format!(
        "<h2>Independent validation (presence–absence)</h2><p>{}</p><ul>",
        esc(METHOD_TEXT)
    );
    for l in validation_md(r)
        .lines()
        .filter_map(|l| l.strip_prefix("- "))
    {
        s.push_str(&format!("<li>{}</li>", esc(l)));
    }
    s.push_str("</ul>");
    if v.auc.is_some() {
        s.push_str("<table><thead><tr><th>Threshold (training data)</th><th class=\"n\">value</th><th class=\"n\">sensitivity</th><th class=\"n\">specificity</th><th class=\"n\">TSS</th><th class=\"n\">kappa</th><th class=\"n\">TP/FP/FN/TN</th></tr></thead><tbody>");
        for t in &v.thresholds {
            s.push_str(&format!(
                "<tr><td>{}</td><td class=\"n\">{:.3}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{}/{}/{}/{}</td></tr>",
                esc(rule_label(&t.rule)),
                t.threshold,
                f2(t.sensitivity),
                f2(t.specificity),
                f2(t.tss),
                f2(t.kappa),
                t.true_pos,
                t.false_pos,
                t.false_neg,
                t.true_neg
            ));
        }
        s.push_str("</tbody></table>");
        if !v.calibration.is_empty() {
            s.push_str("<p>Calibration table: a diagnostic only; suitability is not a probability of presence.</p><table><thead><tr><th>Suitability range</th><th class=\"n\">mean suitability</th><th class=\"n\">share of presence sites</th><th class=\"n\">sites</th></tr></thead><tbody>");
            for c in &v.calibration {
                s.push_str(&format!(
                    "<tr><td>{:.3}–{:.3}</td><td class=\"n\">{:.3}</td><td class=\"n\">{:.2}</td><td class=\"n\">{}</td></tr>",
                    c.lower, c.upper, c.mean_suitability, c.observed_presence_share, c.n
                ));
            }
            s.push_str("</tbody></table>");
        }
    }
    s.push_str("<ul class=\"cite\">");
    for x in [
        REF_ALLOUCHE_2006,
        REF_LEROY_2018,
        REF_LIU_2013,
        REF_BRIER_1950,
    ] {
        s.push_str(&format!("<li>{}</li>", esc(x)));
    }
    s.push_str("</ul>");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validation(
        p: usize,
        a: usize,
        auc: f64,
        tss: f64,
        role: OwnDataRole,
    ) -> IndependentValidation {
        IndependentValidation {
            dataset_id: "d".into(),
            dataset_name: "Survey".into(),
            role,
            synthetic: true,
            sites: ValidationSites {
                records: p + a,
                excluded: BTreeMap::new(),
                presence_sites: p,
                absence_sites: a,
                conflicting_cells: 0,
                leakage_excluded: 0,
            },
            auc: Some(auc),
            thresholds: vec![PaThreshold {
                rule: "max_sss".into(),
                threshold: 0.3,
                sensitivity: Some(0.8),
                specificity: Some(0.8),
                tss: Some(tss),
                kappa: None,
                true_pos: 0,
                false_pos: 0,
                false_neg: 0,
                true_neg: 0,
            }],
            brier: None,
            prevalence: None,
            calibration: vec![],
            spread: None,
            note: None,
            validated: false,
        }
    }

    fn evidence(level: EvidenceLevel) -> Evidence {
        Evidence {
            level,
            reasons: vec![
                "✓ 60 presence cells".into(),
                "No independent validation data: evidence comes from cross-validation of the same, possibly biased, records.".into(),
            ],
        }
    }

    #[test]
    fn validated_needs_every_criterion() {
        let mut e = evidence(EvidenceLevel::Supported);
        let mut v = validation(40, 45, 0.86, 0.55, OwnDataRole::Validation);
        apply_to_evidence(&mut e, &mut v);
        assert_eq!(e.level, EvidenceLevel::Validated);
        assert!(v.validated);
        assert!(
            !e.reasons
                .iter()
                .any(|r| r.starts_with(NO_INDEPENDENT_PREFIX))
        );
        assert!(
            e.reasons
                .iter()
                .any(|r| r.contains("40 presence and 45 absence sites"))
        );
        // Each failing criterion keeps the level.
        for (v, level) in [
            (
                validation(29, 45, 0.86, 0.55, OwnDataRole::Validation),
                EvidenceLevel::Supported,
            ),
            (
                validation(40, 45, 0.74, 0.55, OwnDataRole::Validation),
                EvidenceLevel::Supported,
            ),
            (
                validation(40, 45, 0.86, 0.39, OwnDataRole::Validation),
                EvidenceLevel::Supported,
            ),
            (
                validation(40, 45, 0.86, 0.55, OwnDataRole::Training),
                EvidenceLevel::Supported,
            ),
        ] {
            let mut e = evidence(EvidenceLevel::Supported);
            let mut v = v;
            apply_to_evidence(&mut e, &mut v);
            assert_eq!(e.level, level);
            assert!(!v.validated);
            assert!(e.reasons.iter().any(|r| r.starts_with('✗')));
        }
        // Not Supported (e.g. capped by pooled taxa or null models): never Validated, never
        // raised.
        let mut e = evidence(EvidenceLevel::Indicative);
        let mut v = validation(40, 45, 0.9, 0.7, OwnDataRole::Validation);
        apply_to_evidence(&mut e, &mut v);
        assert_eq!(e.level, EvidenceLevel::Indicative);
        // No metrics: the level is unchanged and the reason says why.
        let mut e = evidence(EvidenceLevel::Supported);
        let mut v = validation(3, 45, 0.9, 0.7, OwnDataRole::Validation);
        v.auc = None;
        v.note = Some("Too few independent sites.".into());
        apply_to_evidence(&mut e, &mut v);
        assert_eq!(e.level, EvidenceLevel::Supported);
        assert!(
            e.reasons
                .iter()
                .any(|r| r.contains("not usable for validation"))
        );
    }
}
