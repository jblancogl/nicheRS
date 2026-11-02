//! Truth-recovery and invariant tests for the down-sampled random forest. There is no golden
//! reference for this engine (ADR-0017); these tests are its correctness evidence.
#![allow(clippy::unwrap_used)]

use std::time::Instant;

use niche_eval::rng::Rng;
use niche_rf::{
    Forest, RfData, RfError, RfSettings, Trainer, fit, partial_dependence, partial_dependence_of,
    permutation_importance,
};

const NAMES: [&str; 7] = [
    "t_mean", "precip", "noise1", "noise2", "noise3", "noise4", "noise5",
];

/// True relative suitability: Gaussian response on the first two variables (optimum 0.6 and
/// 0.3), the five others are pure noise. Predictors uniform on [0, 1].
fn truth(x: &[f64]) -> f64 {
    (-((x[0] - 0.6).powi(2) / (2.0 * 0.1f64.powi(2))
        + (x[1] - 0.3).powi(2) / (2.0 * 0.15f64.powi(2))))
    .exp()
}

/// Presences by rejection sampling from the truth (an inhomogeneous point pattern over a
/// uniform landscape), background uniform. Synthetic data, column-major.
fn synthetic(n_p: usize, n_bg: usize, seed: u64) -> RfData {
    let p = NAMES.len();
    let mut rng = Rng::new(seed);
    let mut presence = vec![Vec::with_capacity(n_p); p];
    let mut background = vec![Vec::with_capacity(n_bg); p];
    let mut x = vec![0.0; p];
    while presence[0].len() < n_p {
        x.iter_mut().for_each(|v| *v = rng.uniform());
        if rng.uniform() < truth(&x) {
            for (c, &v) in presence.iter_mut().zip(&x) {
                c.push(v);
            }
        }
    }
    for _ in 0..n_bg {
        for c in background.iter_mut() {
            c.push(rng.uniform());
        }
    }
    RfData {
        variables: NAMES.iter().map(|s| s.to_string()).collect(),
        categorical: vec![false; p],
        presence,
        background,
    }
}

fn settings(ntree: usize, seed: u64) -> RfSettings {
    RfSettings {
        ntree,
        seed,
        ..RfSettings::default()
    }
}

fn never() -> bool {
    false
}

#[test]
fn defaults_match_valavi() {
    let s = RfSettings::default();
    assert_eq!(s.ntree, 1000);
    assert_eq!(s.min_node_size, 1);
    assert_eq!(s.max_depth, None);
    assert_eq!(s.resolved_mtry(7), 2);
    assert_eq!(s.resolved_mtry(1), 1);
    assert_eq!(s.resolved_mtry(19), 4);
}

#[test]
fn deterministic_and_order_independent() {
    let data = synthetic(100, 1000, 1);
    let a = fit(&data, &settings(50, 7), &never).unwrap();
    let b = fit(&data, &settings(50, 7), &never).unwrap();
    assert_eq!(a, b);
    let c = fit(&data, &settings(50, 8), &never).unwrap();
    assert_ne!(a, c);

    // Trees fitted out of order (as a parallel caller would) give the same forest.
    let trainer = Trainer::new(&data, &settings(50, 7)).unwrap();
    let mut trees: Vec<_> = (0..50).rev().map(|i| (i, trainer.fit_tree(i))).collect();
    trees.sort_by_key(|(i, _)| *i);
    let d = trainer
        .finish(trees.into_iter().map(|(_, t)| t).collect())
        .unwrap();
    assert_eq!(a, d);
    assert_eq!(a.settings().mtry, Some(2));
}

#[test]
fn recovers_known_niche() {
    let train = synthetic(500, 10_000, 11);
    let test = synthetic(300, 3000, 12);
    let forest = fit(&train, &settings(500, 3), &never).unwrap();

    let pp = forest.predict_columns(&test.presence);
    let pb = forest.predict_columns(&test.background);
    assert!(pp.iter().chain(&pb).all(|&s| (0.0..=1.0).contains(&s)));
    let auc = niche_eval::auc(&pp, &pb);
    // Theoretical ceiling: AUC of the true suitability itself on the same test set.
    let rows = |cols: &[Vec<f64>]| -> Vec<f64> {
        (0..cols[0].len())
            .map(|i| truth(&cols.iter().map(|c| c[i]).collect::<Vec<_>>()))
            .collect()
    };
    let auc_truth = niche_eval::auc(&rows(&test.presence), &rows(&test.background));
    println!("held-out AUC: forest {auc:.4}, true suitability {auc_truth:.4}");
    assert!(auc > 0.8, "held-out AUC {auc}");

    let oob = forest.oob(&train).unwrap();
    println!("OOB AUC {:.4}", oob.auc);
    assert!(oob.auc > 0.8, "OOB AUC {}", oob.auc);

    let imp = permutation_importance(&forest, &train, 2, Some(2000), 5).unwrap();
    for v in &imp {
        println!(
            "importance {:8} dAUC {:+.4} ± {:.4} ({:.1} %)",
            v.variable, v.auc_decrease, v.sd, v.percent
        );
    }
    let mut order: Vec<usize> = (0..imp.len()).collect();
    order.sort_by(|&a, &b| imp[b].auc_decrease.total_cmp(&imp[a].auc_decrease));
    let top: Vec<&str> = order[..2]
        .iter()
        .map(|&i| imp[i].variable.as_str())
        .collect();
    assert!(
        top.contains(&"t_mean") && top.contains(&"precip"),
        "top {top:?}"
    );
    assert!((imp.iter().map(|v| v.percent).sum::<f64>() - 100.0).abs() < 1e-9);

    // Partial dependence peaks near the true optima (0.6 and 0.3).
    let pd = partial_dependence(&forest, &train, 41, 300, 9).unwrap();
    for (j, opt) in [(0usize, 0.6), (1, 0.3)] {
        let c = &pd[j];
        let k = (0..c.y.len())
            .max_by(|&a, &b| c.y[a].total_cmp(&c.y[b]))
            .unwrap();
        println!("PD {} peak at {:.3} (truth {opt})", c.variable, c.x[k]);
        assert!(
            (c.x[k] - opt).abs() < 0.15,
            "{} peak {}",
            c.variable,
            c.x[k]
        );
    }
    // Noise variables: flat curves compared with the informative ones.
    let amp = |j: usize| {
        let y = &pd[j].y;
        y.iter().cloned().fold(f64::MIN, f64::max) - y.iter().cloned().fold(f64::MAX, f64::min)
    };
    for j in 2..7 {
        assert!(amp(j) < amp(0) && amp(j) < amp(1));
    }
}

#[test]
fn constant_columns() {
    let mut data = synthetic(60, 600, 2);
    data.variables.push("constant".into());
    data.categorical.push(false);
    data.presence.push(vec![3.5; 60]);
    data.background.push(vec![3.5; 600]);
    let forest = fit(&data, &settings(100, 1), &never).unwrap();
    assert!(
        forest
            .trees()
            .iter()
            .all(|t| t.n_nodes() >= 1 && t.n_leaves() == t.n_nodes().div_ceil(2))
    );
    let imp = permutation_importance(&forest, &data, 1, None, 1).unwrap();
    assert_eq!(imp[7].auc_decrease, 0.0);
    let pd = partial_dependence(&forest, &data, 10, 50, 1).unwrap();
    assert_eq!(pd[7].x, vec![3.5]);
    // The single-variable entry point (used to parallelise over variables) gives the same curves.
    for (j, c) in pd.iter().enumerate() {
        assert_eq!(
            &partial_dependence_of(&forest, &data, j, 10, 50, 1).unwrap(),
            c
        );
    }
    assert!(matches!(
        partial_dependence_of(&forest, &data, 8, 10, 50, 1),
        Err(RfError::Shape)
    ));

    // All predictors constant: every tree is a single leaf (no split possible).
    let all_const = RfData {
        variables: vec!["a".into(), "b".into()],
        categorical: vec![false, true],
        presence: vec![vec![1.0; 5], vec![2.0; 5]],
        background: vec![vec![1.0; 50], vec![2.0; 50]],
    };
    let f = fit(&all_const, &settings(20, 1), &never).unwrap();
    assert!(f.trees().iter().all(|t| t.n_nodes() == 1));
    assert_eq!(f.predict(&[1.0, 2.0]), 0.5);
}

#[test]
fn few_presences() {
    let data = synthetic(10, 2000, 4);
    let forest = fit(&data, &settings(300, 4), &never).unwrap();
    let test = synthetic(200, 2000, 5);
    let auc = niche_eval::auc(
        &forest.predict_columns(&test.presence),
        &forest.predict_columns(&test.background),
    );
    println!("n_p = 10: held-out AUC {auc:.4}");
    assert!(auc > 0.7, "AUC with 10 presences {auc}");
    let oob = forest.oob(&data).unwrap();
    assert!(
        oob.presence
            .iter()
            .all(|x| x.is_nan() || (0.0..=1.0).contains(x))
    );
}

#[test]
fn serde_roundtrip_identical_predictions() {
    let data = synthetic(80, 800, 6);
    let forest = fit(&data, &settings(100, 6), &never).unwrap();
    let json = serde_json::to_string(&forest).unwrap();
    let back: Forest = serde_json::from_str(&json).unwrap();
    assert_eq!(forest, back);
    let test = synthetic(50, 500, 7);
    let a = forest.predict_columns(&test.background);
    let b = back.predict_columns(&test.background);
    assert!(a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()));

    // A corrupted checkpoint is rejected instead of looping or panicking at prediction.
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v["trees"][0]["left"][0] = serde_json::json!(0);
    assert!(serde_json::from_value::<Forest>(v).is_err());
}

#[test]
fn input_errors_and_cancel() {
    let mut data = synthetic(20, 100, 8);
    let cancelled = fit(&data, &settings(10, 1), &|| true);
    assert!(matches!(cancelled, Err(RfError::Cancelled)));
    data.background[3][5] = f64::NAN;
    assert!(matches!(
        fit(&data, &settings(10, 1), &never),
        Err(RfError::NonFinite(v)) if v == "noise2"
    ));
    data.background[3].pop();
    assert!(matches!(
        fit(&data, &settings(10, 1), &never),
        Err(RfError::Shape)
    ));
    let empty = RfData {
        variables: vec!["a".into()],
        categorical: vec![false],
        presence: vec![vec![]],
        background: vec![vec![1.0]],
    };
    assert!(matches!(
        fit(&empty, &settings(10, 1), &never),
        Err(RfError::NoPresence)
    ));
    let ok = synthetic(20, 100, 9);
    assert!(matches!(
        fit(&ok, &settings(0, 1), &never),
        Err(RfError::Settings(_))
    ));
    let forest = fit(&ok, &settings(5, 1), &never).unwrap();
    assert!(forest.predict(&[0.5; 3]).is_nan());
}

#[test]
fn max_depth_limits_trees() {
    let data = synthetic(100, 1000, 10);
    let s = RfSettings {
        ntree: 20,
        max_depth: Some(2),
        seed: 1,
        ..RfSettings::default()
    };
    let forest = fit(&data, &s, &never).unwrap();
    assert!(forest.trees().iter().all(|t| t.n_nodes() <= 7));
}

/// Performance sanity (run in release): `cargo test --release -p niche-rf --test forest
/// -- --ignored --nocapture timing`.
#[test]
#[ignore]
fn timing_1000_trees_500_presences() {
    let data = synthetic(500, 10_000, 13);
    let t = Instant::now();
    let forest = fit(&data, &RfSettings::default(), &never).unwrap();
    let fit_s = t.elapsed().as_secs_f64();
    let nodes: usize = forest.trees().iter().map(|t| t.n_nodes()).sum();
    let t = Instant::now();
    let pred = forest.predict_columns(&data.background);
    let pred_s = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let oob = forest.oob(&data).unwrap();
    let oob_s = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let _ = permutation_importance(&forest, &data, 1, Some(2000), 1).unwrap();
    let imp_s = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let _ = partial_dependence(&forest, &data, 50, 200, 1).unwrap();
    let pd_s = t.elapsed().as_secs_f64();
    let json = serde_json::to_string(&forest).unwrap();
    println!(
        "fit 1000 trees (500 pres + 500 bg each, 7 vars): {fit_s:.3} s, {nodes} nodes, JSON {:.1} MB",
        json.len() as f64 / 1e6
    );
    println!(
        "predict {} rows: {pred_s:.3} s; OOB: {oob_s:.3} s (AUC {:.3}); importance (2500 rows, 1 rep): {imp_s:.3} s; PD (50 × 200): {pd_s:.3} s",
        pred.len(),
        oob.auc
    );
}
