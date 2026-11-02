//! Golden tests: NicheRS must reproduce maxent.jar 3.4.4 on the synthetic fixtures.
//!
//! References in `tests/reference/maxent-3.4.4/<case>/` were produced by
//! `scripts/compare_maxent.py` with the unmodified jar. Tolerances are those fixed in that
//! script before results were examined. Do not loosen them; see AGENTS.md.

use std::path::{Path, PathBuf};

use niche_cli::swd::load_training;
use niche_maxent::{
    FeatureClasses, MaxentModel, MaxentSettings, OutputFormat, permutation_importance,
    response_curves, train,
};

const LAMBDA_ABS: f64 = 1e-6;
const GAIN_ABS: f64 = 1e-4;
/// maxentResults.csv prints permutation importance with 4 decimals.
const PERMUTATION_ABS: f64 = 1e-4;
/// Response curves are cloglog predictions: same bound as the prediction check (gate G1).
const CURVE_Y_ABS: f64 = 1e-5;
/// Curve abscissae are printed by Java at full precision.
const CURVE_X_REL: f64 = 1e-9;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fc(l: bool, q: bool, p: bool, t: bool, h: bool) -> FeatureClasses {
    FeatureClasses {
        linear: l,
        quadratic: q,
        product: p,
        threshold: t,
        hinge: h,
    }
}

fn cases() -> Vec<(&'static str, &'static str, MaxentSettings, Vec<String>)> {
    let d = MaxentSettings::default;
    let noauto = |features| MaxentSettings {
        features,
        auto_features: false,
        ..d()
    };
    vec![
        ("s01_default", "s01_small", d(), vec![]),
        ("s02_default", "s02_medium", d(), vec![]),
        (
            "s02_beta0.5",
            "s02_medium",
            MaxentSettings {
                beta_multiplier: 0.5,
                ..d()
            },
            vec![],
        ),
        (
            "s02_beta2",
            "s02_medium",
            MaxentSettings {
                beta_multiplier: 2.0,
                ..d()
            },
            vec![],
        ),
        ("s03_default", "s03_large", d(), vec![]),
        (
            "s03_lq_noauto",
            "s03_large",
            noauto(fc(true, true, false, false, false)),
            vec![],
        ),
        (
            "s03_lqpth_noauto",
            "s03_large",
            noauto(fc(true, true, true, true, true)),
            vec![],
        ),
        (
            "s03_hinge_only",
            "s03_large",
            noauto(fc(false, false, false, false, true)),
            vec![],
        ),
        (
            "s04_categorical",
            "s04_categorical",
            d(),
            vec!["landcover".to_string()],
        ),
        ("s05_missing", "s05_missing", d(), vec![]),
    ]
}

fn results_csv(path: &Path) -> std::collections::HashMap<String, String> {
    let text = std::fs::read_to_string(path).expect("reference maxentResults.csv");
    let mut lines = text.lines();
    let h = lines.next().expect("header").split(',').map(String::from);
    let v = lines.next().expect("values").split(',').map(String::from);
    h.zip(v).collect()
}

#[test]
fn reproduces_maxent_jar_3_4_4() {
    let mut failures = Vec::new();
    for (case, ds, settings, categorical) in cases() {
        let fx = root().join("tests/fixtures/synthetic").join(ds);
        let refdir = root().join("tests/reference/maxent-3.4.4").join(case);
        let data = load_training(
            &fx.join("samples.csv"),
            &fx.join("background.csv"),
            &categorical,
            -9999.0,
        )
        .expect("fixture loads");
        let (model, report) = train(&data.swd, &settings, &()).expect("training succeeds");
        let reference = MaxentModel::from_lambdas(
            &std::fs::read_to_string(refdir.join("virtualis.lambdas")).expect("reference lambdas"),
        )
        .expect("reference parses");
        let ours = MaxentModel::from_lambdas(&model.to_lambdas()).expect("own lambdas parse");

        let names = |m: &MaxentModel| {
            m.features
                .iter()
                .map(|f| f.name.clone())
                .collect::<Vec<_>>()
        };
        if names(&ours) != names(&reference) {
            failures.push(format!("{case}: feature set differs"));
            continue;
        }
        let dl = ours
            .features
            .iter()
            .zip(&reference.features)
            .map(|(a, b)| (a.lambda - b.lambda).abs())
            .fold(0.0, f64::max);
        if dl > LAMBDA_ABS {
            failures.push(format!("{case}: max |Δλ| = {dl:e}"));
        }
        let res = results_csv(&refdir.join("maxentResults.csv"));
        let it: usize = res["Iterations"].parse().expect("iterations");
        if it != report.iterations {
            failures.push(format!(
                "{case}: iterations {} vs reference {it}",
                report.iterations
            ));
        }
        let gain: f64 = res["Regularized training gain"].parse().expect("gain");
        if (gain - report.regularized_gain).abs() > GAIN_ABS {
            failures.push(format!(
                "{case}: gain {} vs reference {gain}",
                report.regularized_gain
            ));
        }
        // Round-trip: predictions from the serialized model equal those of the in-memory model.
        let vars: Vec<String> = data.swd.vars.iter().map(|v| v.name.clone()).collect();
        let a = model.compile(&vars, true).expect("compile");
        let b = ours.compile(&vars, true).expect("compile");
        for i in (0..data.swd.background[0].len()).step_by(97) {
            let row: Vec<f64> = data.swd.background.iter().map(|c| c[i]).collect();
            let (pa, pb) = (
                a.predict(&row, OutputFormat::Cloglog),
                b.predict(&row, OutputFormat::Cloglog),
            );
            if (pa - pb).abs() > 1e-12 {
                failures.push(format!(
                    "{case}: lambdas round-trip changes prediction {pa} vs {pb}"
                ));
                break;
            }
        }
    }
    assert!(
        failures.is_empty(),
        "golden failures:\n{}",
        failures.join("\n")
    );
}

/// Permutation importance (maxentResults.csv) and marginal response curves
/// (`plots/<species>_<var>.dat`, from `scripts/gen_response_refs.py`) of maxent.jar 3.4.4.
#[test]
fn reproduces_maxent_jar_explanations() {
    let mut failures = Vec::new();
    for (case, ds, settings, categorical) in cases() {
        let fx = root().join("tests/fixtures/synthetic").join(ds);
        let refdir = root().join("tests/reference/maxent-3.4.4").join(case);
        let data = load_training(
            &fx.join("samples.csv"),
            &fx.join("background.csv"),
            &categorical,
            -9999.0,
        )
        .expect("fixture loads");
        let (model, _) = train(&data.swd, &settings, &()).expect("training succeeds");

        let res = results_csv(&refdir.join("maxentResults.csv"));
        for (var, pi) in permutation_importance(&model, &data.swd).expect("importance") {
            let want: f64 = res[&format!("{var} permutation importance")]
                .parse()
                .expect("reference importance");
            if (pi - want).abs() > PERMUTATION_ABS {
                failures.push(format!(
                    "{case}: {var} permutation importance {pi} vs reference {want}"
                ));
            }
        }

        let curves = response_curves(
            &model,
            &data.swd,
            settings.add_samples_to_background,
            OutputFormat::Cloglog,
        )
        .expect("curves");
        for c in curves {
            let path = refdir
                .join("plots")
                .join(format!("virtualis_{}.dat", c.variable));
            let text = std::fs::read_to_string(&path).expect("reference curve");
            let pts: Vec<(f64, f64)> = text
                .lines()
                .skip(1)
                .map(|l| {
                    let f: Vec<&str> = l.split(',').collect();
                    (f[1].parse().expect("x"), f[2].parse().expect("y"))
                })
                .collect();
            if pts.len() != c.x.len() {
                failures.push(format!(
                    "{case}: {} curve has {} points vs reference {}",
                    c.variable,
                    c.x.len(),
                    pts.len()
                ));
                continue;
            }
            let dx =
                c.x.iter()
                    .zip(&pts)
                    .map(|(a, (b, _))| (a - b).abs() / b.abs().max(1.0))
                    .fold(0.0, f64::max);
            let dy =
                c.y.iter()
                    .zip(&pts)
                    .map(|(a, (_, b))| (a - b).abs())
                    .fold(0.0, f64::max);
            if dx > CURVE_X_REL || dy > CURVE_Y_ABS {
                failures.push(format!(
                    "{case}: {} curve max rel |Δx| = {dx:e}, max |Δy| = {dy:e}",
                    c.variable
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "golden explanation failures:\n{}",
        failures.join("\n")
    );
}
