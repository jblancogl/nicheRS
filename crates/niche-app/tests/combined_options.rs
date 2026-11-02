//! Science v1 options combined in one synthetic demo study (QA 2026-10-20): target-group
//! background (ADR-0013) with distance thinning, spatial grid blocks with null models and the
//! area of applicability (ADR-0014), the MaxEnt + random forest ensemble (ADR-0017), a future
//! projection (ADR-0016) and independent validation on the organisation's own presence–absence
//! data (ADR-0108). Each option is tested alone elsewhere; here they run together, the outputs
//! and every manifest section exist, and a study interrupted during cross-validation resumes to
//! byte-identical outputs. Every dataset here is generated and labelled SYNTHETIC.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use niche_app::accounts::Scope;
use niche_app::dto::*;
use niche_app::{Config, Engine, demo};
use niche_eval::rng::Rng;
use niche_geo::crs::Crs;

fn cfg(dir: &std::path::Path) -> Config {
    Config {
        data_dir: dir.join("data"),
        cache_dir: dir.join("cache"),
        max_records: 5000,
        gbif: None,
        secret_key: None,
        mirror: None,
        limits: Default::default(),
        retention_days: 0,
    }
}

const SEED: u64 = 9;
const TAXON: &str = "Virtualis demonstrandum";

/// A SYNTHETIC presence–absence survey of the demo's virtual species in Lambert-93: sites drawn
/// uniformly over the demo region, found with probability equal to its true relative
/// suitability (perfect detection). Not field data.
fn survey() -> ImportRequest {
    let g = demo::REGION;
    let max = (0..g.height)
        .flat_map(|r| (0..g.width).map(move |c| g.center(r, c)))
        .map(|(x, y)| demo::truth(x, y))
        .fold(0.0, f64::max);
    let mut rng = Rng::new(0x00C0_4B1E_D5E7);
    let mut cols = ImportColumns {
        status: Some(Vec::new()),
        date: Some(Vec::new()),
        ..Default::default()
    };
    for _ in 0..300 {
        let lng = g.west + 0.1 + rng.uniform() * (g.width as f64 * g.cell - 0.2);
        let lat = g.south() + 0.1 + rng.uniform() * (g.height as f64 * g.cell - 0.2);
        let found = rng.uniform() < demo::truth(lng, lat) / max;
        let (x, y) = Crs::Lambert93.from_wgs84(lng, lat);
        cols.taxon.push(TAXON.into());
        cols.x.push(format!("{x:.1}"));
        cols.y.push(format!("{y:.1}"));
        if let Some(s) = cols.status.as_mut() {
            s.push(if found { "present" } else { "absent" }.into());
        }
        if let Some(d) = cols.date.as_mut() {
            d.push("2022-06-01".into());
        }
    }
    let mapping = [
        ("taxon", "taxon"),
        ("x", "x"),
        ("y", "y"),
        ("status", "status"),
        ("date", "date"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    ImportRequest {
        name: "SYNTHETIC combined-options survey (Lambert-93)".into(),
        file_name: "synthetic_combined_survey.csv".into(),
        file_sha256: "c".repeat(64),
        crs: "EPSG:2154".into(),
        columns: cols,
        lines: None,
        absent_values: vec!["absent".into()],
        mapping,
        synthetic: true,
    }
}

fn spec(dataset: &DatasetInfo) -> JobSpec {
    JobSpec {
        name: "combined options".into(),
        species: vec![SpeciesRef {
            key: demo::SPECIES_KEY,
            scientific_name: demo::SPECIES_NAME.into(),
            own: Some(OwnSpeciesData {
                values: vec![TAXON.into()],
                role: OwnDataRole::Validation,
                gbif: true,
            }),
        }],
        area: StudyArea {
            name: "Demo".into(),
            geometry: demo::area_geojson(),
            places: vec![],
        },
        layers: demo::LAYERS.iter().map(|l| l.0.to_string()).collect(),
        resolution_factor: 2,
        filters: OccurrenceFilters {
            thin_km: Some(5.0),
            ..OccurrenceFilters::default()
        },
        model: ModelSettings {
            tuning: "fixed".into(),
            feature_classes: vec!["LQ".into()],
            beta_multipliers: vec![1.0],
            background_points: 2000,
            background_method: "target_group".into(),
            algorithm: "ensemble".into(),
            rf_trees: Some(100),
            ..ModelSettings::default()
        },
        validation: ValidationSettings {
            method: "spatial_grid".into(),
            folds: 4,
            block_km: None,
            null_models: 20,
            aoa: true,
        },
        seed: SEED,
        demo: true,
        projections: vec![ProjectionSpec {
            period: "2041-2070".into(),
            ssp: "ssp370".into(),
            gcms: vec!["GFDL-ESM4".into(), "UKESM1-0-LL".into()],
        }],
        own_dataset: Some(DatasetRef {
            id: dataset.id.clone(),
            records_sha256: dataset.records_sha256.clone(),
        }),
    }
}

type Files = BTreeMap<String, Vec<u8>>;

/// Output files of a species; the generation-time line of the reports and the dataset id are
/// the only content allowed to differ between two runs.
fn outputs(dir: &std::path::Path) -> Files {
    std::fs::read_dir(dir)
        .expect("species dir")
        .flatten()
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let mut bytes = std::fs::read(e.path()).expect("read");
            if name == "result.json" {
                // The same file imported in another data directory gets a new dataset id.
                let mut v: serde_json::Value = serde_json::from_slice(&bytes).expect("result");
                v["independent_validation"]["dataset_id"] = serde_json::Value::Null;
                bytes = serde_json::to_vec(&v).expect("json");
            }
            if name == "report.md" || name == "report.html" {
                let text = String::from_utf8(bytes).expect("utf-8");
                bytes = text
                    .lines()
                    .filter(|l| !l.starts_with("Generated by NicheRS"))
                    .map(|l| {
                        match (
                            l.find(" · Generated by NicheRS"),
                            l.find(" · GBIF taxon key"),
                        ) {
                            (Some(a), Some(b)) if a < b => format!("{}{}", &l[..a], &l[b..]),
                            _ => l.to_string(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    .into_bytes();
            }
            (name, bytes)
        })
        .collect()
}

/// The manifest without what legitimately differs between two runs (times, ids).
fn manifest(dir: &std::path::Path, id: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(dir.join("data/jobs").join(id).join("manifest.json"))
        .expect("manifest");
    let mut m: serde_json::Value = serde_json::from_str(&text).expect("manifest json");
    if let Some(o) = m.as_object_mut() {
        for k in ["created_at", "job_id"] {
            o.remove(k);
        }
    }
    m
}

#[test]
fn every_science_v1_option_together_completes_and_resumes_identically() {
    let root = std::env::temp_dir().join(format!("niche-combined-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let never = AtomicBool::new(false);

    // Reference: never interrupted.
    let ref_dir = root.join("reference");
    let e = Engine::open(cfg(&ref_dir)).expect("engine");
    let d = e
        .import_dataset(&survey(), &Scope::All, None)
        .expect("import");
    assert!(d.synthetic && d.report.presences >= 30 && d.report.absences >= 30);
    let s = spec(&d);
    let id = e.submit(s.clone()).expect("submit").id;
    e.run_job(&id, &never, &never);
    let res = e.results(&id).expect("results");
    assert_eq!(res.job.status, JobStatus::Succeeded, "{:?}", res.job.error);
    let r = &res.species[0];
    assert_eq!(r.status, SpeciesRunStatus::Succeeded, "{:?}", r.error);

    // Every option left its result.
    assert_eq!(r.selected.algorithm.as_deref(), Some("ensemble"));
    let bg = r.background_sampling.as_ref().expect("background sampling");
    assert_eq!(bg.method, "target_group", "{:?}", bg.fallback_reason);
    assert_eq!(r.validation.method, "spatial_grid");
    assert!(r.validation.blocks.is_some(), "grid block design");
    assert_eq!(r.validation.members.len(), 2, "ensemble members");
    let nm = r.validation.null_model.as_ref().expect("null models");
    assert_eq!(nm.requested, 20);
    assert!(nm.completed > 0);
    assert!(r.aoa.is_some(), "area of applicability");
    assert_eq!(r.projections.len(), 1, "one future scenario");
    assert!(r.projections[0].synthetic);
    let v = r
        .independent_validation
        .as_ref()
        .expect("independent validation");
    assert_eq!(v.role, OwnDataRole::Validation);
    assert!(v.synthetic && v.auc.is_some());
    assert!(
        r.data
            .exclusions
            .iter()
            .any(|x| x.reason.starts_with("Thinned")),
        "distance thinning removed presence cells: {:?}",
        r.data.exclusions
    );

    // Outputs.
    let species_dir = ref_dir.join("data/jobs").join(&id).join("0");
    let reference = outputs(&species_dir);
    for f in [
        "suitability.tif",
        "uncertainty.tif",
        "mess.tif",
        "aoa.tif",
        "effort.tif",
        "model.lambdas",
        "model.rf.json",
        "future_2041-2070_ssp370_mean.tif",
        "future_2041-2070_ssp370_sd.tif",
        "future_2041-2070_ssp370_change.tif",
        "future_2041-2070_ssp370_agreement.tif",
        "future_2041-2070_ssp370_mess.tif",
        "future_areas.csv",
        "occurrences.geojson",
        "result.json",
        "report.md",
        "report.html",
    ] {
        assert!(reference.contains_key(f), "missing output {f}");
    }

    // The manifest records every option.
    let m = manifest(&ref_dir, &id);
    for k in [
        "future_climate",
        "own_data",
        "synthetic_data",
        "spec",
        "spec_sha256",
        "random_seed",
        "software",
        "environment",
        "occurrence_source",
    ] {
        assert!(!m[k].is_null(), "manifest lacks {k}");
    }
    for k in [
        "algorithm",
        "ensemble",
        "random_forest",
        "background",
        "thinning",
        "spatial_grid",
        "null_models",
        "area_of_applicability",
        "projections",
        "own_data",
        "independent_validation",
        "evidence_rules",
    ] {
        assert!(!m["methods"][k].is_null(), "manifest lacks methods.{k}");
    }
    for k in [
        "background",
        "sampling_bias",
        "validation",
        "area_of_applicability",
        "projections",
        "own_data",
        "evidence",
        "artifacts",
    ] {
        assert!(
            !m["species"][0][k].is_null(),
            "manifest lacks species[0].{k}"
        );
    }
    assert_eq!(m["own_data"]["synthetic"], true);

    // Interrupted during cross-validation, then resumed by a new engine.
    let dir = root.join("resumed");
    let id2;
    {
        let e = Arc::new(Engine::open(cfg(&dir)).expect("engine"));
        let d2 = e
            .import_dataset(&survey(), &Scope::All, None)
            .expect("import");
        assert_eq!(d2.records_sha256, d.records_sha256);
        id2 = e.submit(spec(&d2)).expect("submit").id;
        let ckpt = dir.join("data/jobs").join(&id2).join("checkpoints/0");
        let stop = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let (s2, f2) = (stop.clone(), finished.clone());
        let watcher = std::thread::spawn(move || {
            while !f2.load(Ordering::Relaxed) {
                if std::fs::read_dir(&ckpt).map(|d| d.count()).unwrap_or(0) >= 3 {
                    s2.store(true, Ordering::Relaxed);
                    return;
                }
                std::thread::sleep(Duration::from_micros(200));
            }
        });
        e.run_job(&id2, &stop, &stop);
        finished.store(true, Ordering::Relaxed);
        watcher.join().expect("watcher");
        assert!(
            stop.load(Ordering::Relaxed),
            "the study was not interrupted"
        );
        assert_eq!(
            e.store().get(&id2).expect("db").expect("job").status,
            JobStatus::Running,
            "shutdown leaves the job resumable"
        );
    }
    let e = Engine::open(cfg(&dir)).expect("engine");
    e.store().recover_interrupted().expect("recover");
    e.run_job(&id2, &never, &never);
    let res2 = e.results(&id2).expect("results");
    assert_eq!(
        res2.job.status,
        JobStatus::Succeeded,
        "{:?}",
        res2.job.error
    );
    let events = e.store().events_after(&id2, 0).expect("events");
    assert!(
        events.iter().any(|ev| ev
            .message
            .contains("cross-validation models already trained")),
        "the resume reused the saved cross-validation models"
    );
    let resumed = outputs(&dir.join("data/jobs").join(&id2).join("0"));
    assert_eq!(
        reference.keys().collect::<Vec<_>>(),
        resumed.keys().collect::<Vec<_>>()
    );
    for (name, bytes) in &reference {
        assert!(resumed[name] == *bytes, "{name} differs after resuming");
    }
    // Same dataset content, imported under a new id and time in the second data directory;
    // the report and result checksums change with them (contents compared above).
    let mut m2 = manifest(&dir, &id2);
    let mut m1 = m;
    for x in [&mut m1, &mut m2] {
        let null = serde_json::Value::Null;
        x["spec"]["own_dataset"]["id"] = null.clone();
        x["spec_sha256"] = null.clone();
        x["own_data"]["dataset_id"] = null.clone();
        x["own_data"]["created_at"] = null.clone();
        x["species"][0]["own_data"]["independent_validation"]["dataset_id"] = null.clone();
        for f in ["report.md", "report.html", "result.json"] {
            // (The generation time in the reports may also change their length by a byte.)
            x["species"][0]["artifacts"][f]["sha256"] = null.clone();
            x["species"][0]["artifacts"][f]["bytes"] = null.clone();
        }
    }
    assert_eq!(m1, m2, "manifest differs after resuming");
    assert!(
        !dir.join("data/jobs")
            .join(&id2)
            .join("checkpoints")
            .exists()
    );
    let _ = std::fs::remove_dir_all(root);
}
