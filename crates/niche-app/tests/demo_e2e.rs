//! End-to-end run on the synthetic demo (offline): job lifecycle, outputs, and recovery of the
//! known virtual-species response.

use std::sync::atomic::AtomicBool;

use niche_app::dto::*;
use niche_app::{Config, Engine, demo};

fn engine() -> (Engine, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("niche-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = Config {
        data_dir: dir.join("data"),
        cache_dir: dir.join("cache"),
        max_records: 5000,
        gbif: None,
        secret_key: None,
        mirror: None,
        limits: Default::default(),
        retention_days: 0,
    };
    (Engine::open(cfg).expect("engine"), dir)
}

fn demo_spec(tuning: &str) -> JobSpec {
    JobSpec {
        name: "demo".into(),
        species: vec![SpeciesRef {
            key: demo::SPECIES_KEY,
            scientific_name: demo::SPECIES_NAME.into(),
            own: None,
        }],
        area: StudyArea {
            name: "Demo region".into(),
            geometry: demo::area_geojson(),
            places: vec![],
        },
        layers: demo::LAYERS.iter().map(|l| l.0.to_string()).collect(),
        resolution_factor: 1,
        filters: OccurrenceFilters::default(),
        model: ModelSettings {
            tuning: tuning.into(),
            feature_classes: vec!["LQ".into(), "LQH".into()],
            beta_multipliers: vec![1.0, 2.0],
            ..ModelSettings::default()
        },
        validation: ValidationSettings::default(),
        seed: 42,
        demo: true,
        projections: vec![],
        own_dataset: None,
    }
}

#[test]
fn demo_job_end_to_end() {
    let (e, dir) = engine();
    let job = e.submit(demo_spec("auto")).expect("submit");
    let never = AtomicBool::new(false);
    e.run_job(&job.id, &never, &never);
    let res = e.results(&job.id).expect("results");
    assert_eq!(res.job.status, JobStatus::Succeeded, "{:?}", res.job.error);
    let r = &res.species[0];
    assert_eq!(r.status, SpeciesRunStatus::Succeeded);
    println!("evidence {:?} {:?}", r.evidence.level, r.evidence.reasons);
    println!("validation {:?}", r.validation);
    println!("exclusions {:?}", r.data.exclusions);
    println!("thresholds {:?}", r.thresholds);
    assert!(r.data.presences >= 100, "presences {}", r.data.presences);
    // Deliberately bad demo records must be excluded with reasons.
    let reasons: Vec<&str> = r
        .data
        .exclusions
        .iter()
        .map(|x| x.reason.as_str())
        .collect();
    assert!(reasons.iter().any(|x| x.contains("before")), "{reasons:?}");
    assert!(
        reasons.iter().any(|x| x.contains("uncertainty")),
        "{reasons:?}"
    );
    assert!(reasons.iter().any(|x| x.contains("Licence")), "{reasons:?}");
    assert!(
        r.thresholds
            .iter()
            .all(|t| t.area_km2 > 0.0 && t.area_km2 <= r.study_area_km2)
    );
    assert!(r.validation.cbi_mean.unwrap_or(0.0) > 0.5);
    for f in [
        "suitability.tif",
        "uncertainty.tif",
        "mess.tif",
        "model.lambdas",
        "occurrences.geojson",
        "report.md",
        "report.html",
        "response_curves.csv",
    ] {
        assert!(r.files.iter().any(|x| x.name == f), "missing {f}");
    }
    // Explanations of the final model: permutation importance sums to 100, one curve per
    // variable used, sampled at 101 points within [0, 1].
    let pi: f64 = r
        .contributions
        .iter()
        .map(|c| c.permutation_importance.expect("permutation importance"))
        .sum();
    assert!(
        (pi - 100.0).abs() < 1e-6,
        "permutation importance sums to {pi}"
    );
    assert_eq!(r.response_curves.len(), r.variables_used.len());
    for c in &r.response_curves {
        assert_eq!(c.x.len(), 101, "{}", c.variable);
        assert!(
            c.y.iter().all(|y| (0.0..=1.0).contains(y)),
            "{}",
            c.variable
        );
        assert!(c.training_min < c.training_max && c.x[0] < c.training_min);
    }
    let html = std::fs::read_to_string(
        niche_app::artifacts::species_dir(&dir.join("data"), &job.id, 0).join("report.html"),
    )
    .expect("report.html");
    assert!(html.contains("SYNTHETIC DEMONSTRATION DATA"));
    assert!(html.contains("data:image/png;base64,"));
    assert!(html.contains("<polyline"));
    assert!(!html.to_lowercase().contains("probability of presence</b>"));
    // Recovery of the true response: Spearman(prediction, truth) over the grid.
    let s = e.raster(&job.id, 0, "suitability").expect("raster");
    let (mut p, mut t) = (Vec::new(), Vec::new());
    for row in (0..s.spec.height).step_by(3) {
        for col in (0..s.spec.width).step_by(3) {
            let v = s.get(row, col);
            if v.is_finite() {
                let (x, y) = s.spec.center(row, col);
                p.push(f64::from(v));
                t.push(demo::truth(x, y));
            }
        }
    }
    let rho = niche_eval::spearman(&p, &t).expect("rho");
    println!("Spearman(prediction, truth) = {rho:.3}");
    assert!(
        rho > 0.8,
        "model does not recover the virtual species: rho = {rho}"
    );
    let m = e.manifest(&job.id).expect("manifest");
    assert_eq!(m["synthetic_data"], true);
    let _ = std::fs::remove_dir_all(dir);
}
