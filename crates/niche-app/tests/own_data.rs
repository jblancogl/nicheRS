//! The organisation's own occurrence data (ADR-0108) on the synthetic demo: import of a
//! SYNTHETIC presence–absence survey in Lambert-93, independent validation of the final
//! prediction with the leakage rule, own presences used for training (with and without the
//! demo records), deletion, and unchanged default specifications. Every dataset here is
//! generated and labelled synthetic; none is a field survey.

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

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

fn root(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("niche-own-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

const SEED: u64 = 5;
const TAXON: &str = "Virtualis demonstrandum";
/// A value that must never appear in outputs other than the organisation's own points
/// (the design notes §9): the manifest and reports carry no coordinates.
const WITNESS_X: &str = "654321,987";

fn spec(own: Option<OwnSpeciesData>, dataset: Option<&DatasetInfo>) -> JobSpec {
    JobSpec {
        name: "own data".into(),
        species: vec![SpeciesRef {
            key: demo::SPECIES_KEY,
            scientific_name: demo::SPECIES_NAME.into(),
            own,
        }],
        area: StudyArea {
            name: "Demo".into(),
            geometry: demo::area_geojson(),
            places: vec![],
        },
        layers: demo::LAYERS.iter().map(|l| l.0.to_string()).collect(),
        resolution_factor: 2,
        filters: OccurrenceFilters::default(),
        model: ModelSettings {
            tuning: "fixed".into(),
            feature_classes: vec!["LQ".into()],
            beta_multipliers: vec![1.0],
            background_points: 2000,
            ..ModelSettings::default()
        },
        validation: ValidationSettings::default(),
        seed: SEED,
        demo: true,
        projections: vec![],
        own_dataset: dataset.map(|d| DatasetRef {
            id: d.id.clone(),
            records_sha256: d.records_sha256.clone(),
        }),
    }
}

/// A SYNTHETIC survey of the demo's virtual species: `n` sites drawn uniformly over the demo
/// region, found with probability equal to the species' true relative suitability (perfect
/// detection), written in Lambert-93 with French decimal commas and day-first dates; plus
/// `leaks` presences at the coordinates of the demo's own training records (leakage rule) and
/// one witness row outside the study area.
fn survey(n: usize, leaks: usize) -> ImportRequest {
    let g = demo::REGION;
    let max = (0..g.height)
        .flat_map(|r| (0..g.width).map(move |c| g.center(r, c)))
        .map(|(x, y)| demo::truth(x, y))
        .fold(0.0, f64::max);
    let mut rng = Rng::new(0x005E_ED0F_D47A);
    let mut cols = ImportColumns {
        date: Some(Vec::new()),
        uncertainty: Some(Vec::new()),
        status: Some(Vec::new()),
        survey: Some(Vec::new()),
        ..Default::default()
    };
    let mut push = |lng: f64, lat: f64, found: bool, x_text: Option<&str>| {
        let (x, y) = Crs::Lambert93.from_wgs84(lng, lat);
        cols.taxon.push(TAXON.into());
        cols.x
            .push(x_text.map_or_else(|| format!("{x:.2}").replace('.', ","), String::from));
        cols.y.push(format!("{y:.2}").replace('.', ","));
        if let Some(d) = cols.date.as_mut() {
            d.push("17/05/2021".into());
        }
        if let Some(u) = cols.uncertainty.as_mut() {
            u.push("10".into());
        }
        if let Some(s) = cols.status.as_mut() {
            s.push(if found { "Présent" } else { "Non observé" }.into());
        }
        if let Some(s) = cols.survey.as_mut() {
            s.push("Synthetic protocol A".into());
        }
    };
    for _ in 0..n {
        let lng = g.west + 0.1 + rng.uniform() * (g.width as f64 * g.cell - 0.2);
        let lat = g.south() + 0.1 + rng.uniform() * (g.height as f64 * g.cell - 0.2);
        let found = rng.uniform() < demo::truth(lng, lat) / max;
        push(lng, lat, found, None);
    }
    for o in demo::occurrences(SEED).iter().take(leaks) {
        push(o.lng, o.lat, true, None);
    }
    push(2.4, 44.7, true, Some(WITNESS_X));
    let mut mapping = BTreeMap::new();
    for (k, v) in [
        ("taxon", "nom_cite"),
        ("x", "x_l93"),
        ("y", "y_l93"),
        ("date", "date_obs"),
        ("uncertainty", "precision_m"),
        ("status", "statut"),
        ("survey", "protocole"),
    ] {
        mapping.insert(k.to_string(), v.to_string());
    }
    ImportRequest {
        name: "Synthetic survey (Lambert-93)".into(),
        file_name: "synthetic_survey.csv".into(),
        file_sha256: "a".repeat(64),
        crs: "EPSG:2154".into(),
        columns: cols,
        lines: None,
        absent_values: vec!["Non observé".into()],
        mapping,
        synthetic: true,
    }
}

fn run(e: &Engine, spec: JobSpec) -> (String, SpeciesResult) {
    let id = e.submit(spec).expect("submit").id;
    let never = AtomicBool::new(false);
    e.run_job(&id, &never, &never);
    let res = e.results(&id).expect("results");
    assert_eq!(res.job.status, JobStatus::Succeeded, "{:?}", res.job.error);
    let r = res.species[0].clone();
    assert_eq!(r.status, SpeciesRunStatus::Succeeded, "{:?}", r.error);
    (id, r)
}

fn read(dir: &std::path::Path, id: &str, name: &str) -> String {
    std::fs::read_to_string(dir.join("data/jobs").join(id).join(name)).expect(name)
}

#[test]
fn held_out_presence_absence_data_validate_the_demo() {
    let dir = root("validation");
    let e = Engine::open(cfg(&dir)).expect("engine");
    let d = e
        .import_dataset(&survey(400, 6), &Scope::All, None)
        .expect("import");
    assert_eq!(d.report.rows_read, 407);
    assert_eq!(d.report.rows_imported, 407, "{:?}", d.report.rejected);
    assert!(
        d.report.absences > 100 && d.report.presences > 30,
        "{:?}",
        d.report
    );
    assert!(d.synthetic && d.report.warnings.iter().any(|w| w.code == "synthetic"));
    assert_eq!(d.report.taxa.len(), 1);
    let own = OwnSpeciesData {
        values: vec![TAXON.into()],
        role: OwnDataRole::Validation,
        gbif: true,
    };
    // The data review shows the sites before the model exists.
    let p = e
        .preview(&DataPreviewRequest {
            species_key: demo::SPECIES_KEY,
            area: spec(None, None).area,
            resolution_factor: 2,
            filters: OccurrenceFilters::default(),
            demo: true,
            seed: SEED,
            own_dataset: Some(DatasetRef {
                id: d.id.clone(),
                records_sha256: d.records_sha256.clone(),
            }),
            own: Some(own.clone()),
        })
        .expect("preview");
    let sites = p.validation_sites.expect("validation sites");
    assert!(
        sites.presence_sites >= 30 && sites.absence_sites >= 30,
        "{sites:?}"
    );
    assert!(p.by_source.iter().any(|s| s.source == "own"));

    let (id, r) = run(&e, spec(Some(own), Some(&d)));
    let v = r
        .independent_validation
        .clone()
        .expect("independent validation");
    assert_eq!(v.role, OwnDataRole::Validation);
    assert!(v.synthetic);
    // (The demo preview draws its synthetic records with seed 1 and the run with the study seed,
    // so the leakage counts of a demo preview and run differ; for real data they are the same.)
    assert_eq!(v.sites.records, sites.records);
    // Leakage rule: the survey rows placed on the demo's training records are excluded.
    assert!(
        v.sites.excluded.get("training_cell").copied().unwrap_or(0) >= 6,
        "{:?}",
        v.sites
    );
    assert!(v.sites.leakage_excluded >= 1);
    assert!(
        v.sites.excluded.get("outside_area").copied().unwrap_or(0) >= 1,
        "the witness row"
    );
    // The virtual species is recovered: the prediction separates surveyed presences from
    // absences well (the survey follows its true suitability).
    let auc = v.auc.expect("auc");
    assert!(auc > 0.75, "PA AUC {auc}");
    eprintln!(
        "synthetic PA validation: AUC {auc:.3}, thresholds {:?}, sites {:?}, level {:?}",
        v.thresholds, v.sites, r.evidence.level
    );
    assert_eq!(
        v.thresholds
            .iter()
            .map(|t| t.rule.as_str())
            .collect::<Vec<_>>(),
        ["p10", "max_sss"]
    );
    let t = &v.thresholds[1];
    assert_eq!(
        t.true_pos + t.false_neg,
        v.sites.presence_sites,
        "every presence site is evaluated"
    );
    assert_eq!(t.true_neg + t.false_pos, v.sites.absence_sites);
    assert!(v.brier.is_some() && !v.calibration.is_empty() && v.spread.is_some());
    // Evidence: the reason about missing independent data is replaced; the level follows the
    // documented criteria (Validated only with every criterion met).
    assert!(
        !r.evidence
            .reasons
            .iter()
            .any(|x| x.starts_with("No independent validation data"))
    );
    assert!(
        r.evidence
            .reasons
            .iter()
            .any(|x| x.starts_with("Independent presence–absence validation"))
    );
    assert_eq!(v.validated, r.evidence.level == EvidenceLevel::Validated);
    // Sources: the demo records train; the organisation's records only validate.
    let own_src = r
        .data
        .by_source
        .iter()
        .find(|s| s.source == "own")
        .expect("own");
    assert_eq!(own_src.retained_cells, 0);
    assert_eq!(own_src.records, 407);
    // Provenance without coordinates: manifest and reports carry checksums and counts only.
    let manifest = read(&dir, &id, "manifest.json");
    let m: serde_json::Value = serde_json::from_str(&manifest).expect("manifest");
    assert_eq!(m["own_data"]["records_sha256"], d.records_sha256.as_str());
    assert_eq!(m["own_data"]["crs"], "EPSG:2154");
    assert_eq!(m["own_data"]["synthetic"], true);
    assert!(m["methods"]["independent_validation"].is_string());
    assert!(m["species"][0]["own_data"]["independent_validation"]["auc"].is_number());
    for name in [
        "manifest.json",
        "0/report.md",
        "0/report.html",
        "0/result.json",
    ] {
        let t = read(&dir, &id, name);
        assert!(
            !t.contains("654321"),
            "{name} carries a coordinate of the file"
        );
    }
    let report = read(&dir, &id, "0/report.md");
    assert!(report.contains("## Independent validation (presence–absence)"));
    assert!(report.contains("## Occurrence data sources"));
    assert!(report.contains("Allouche"));
    assert!(report.contains("not a probability"));
    // The study keeps its own copy; the occurrence file marks the organisation's points.
    assert!(
        dir.join("data/jobs")
            .join(&id)
            .join("input/own-records.json")
            .exists()
    );
    let occ = read(&dir, &id, "0/occurrences.geojson");
    assert!(occ.contains("\"source\":\"own\"") && occ.contains("\"validation_absence\""));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn own_presences_train_with_or_without_the_demo_records() {
    let dir = root("training");
    let e = Engine::open(cfg(&dir)).expect("engine");
    let d = e
        .import_dataset(&survey(400, 0), &Scope::All, None)
        .expect("import");
    // Both sources: own presences first, absences never presences.
    let both = OwnSpeciesData {
        values: vec![TAXON.into()],
        role: OwnDataRole::Training,
        gbif: true,
    };
    let (_, r) = run(&e, spec(Some(both), Some(&d)));
    let own = r
        .data
        .by_source
        .iter()
        .find(|s| s.source == "own")
        .expect("own");
    let gbif = r
        .data
        .by_source
        .iter()
        .find(|s| s.source == "gbif")
        .expect("gbif");
    assert!(own.retained_cells > 20, "{own:?}");
    assert_eq!(
        own.statuses.get("absence").copied(),
        Some(d.report.absences)
    );
    assert!(gbif.records > 100 && gbif.in_own_cells.is_some());
    assert_eq!(
        r.data.presences,
        own.retained_cells + gbif.retained_cells,
        "training cells come from both sources"
    );
    assert_eq!(
        r.data.exclusions[0].reason,
        niche_app::owndata::ABSENCE_REASON,
        "the absence rule comes first"
    );
    // The organisation's licence is accepted by the licence rule.
    assert!(!own.statuses.contains_key("licence"));
    // Absences of a training dataset cannot validate: no independent presence site remains.
    let v = r.independent_validation.expect("sites of the absences");
    assert!(
        v.auc.is_none()
            && v.note
                .as_deref()
                .is_some_and(|n| n.contains("used for training"))
    );
    assert!(r.evidence.level != EvidenceLevel::Validated);
    assert!(
        r.evidence
            .reasons
            .iter()
            .any(|x| x.contains("not usable for validation"))
    );

    // Own data only: no demo records are fetched.
    let only = OwnSpeciesData {
        values: vec![TAXON.into()],
        role: OwnDataRole::Training,
        gbif: false,
    };
    let (id, r) = run(&e, spec(Some(only), Some(&d)));
    assert!(r.data.by_source.iter().all(|s| s.source == "own"));
    assert_eq!(r.data.presences, r.data.by_source[0].retained_cells);
    let m: serde_json::Value =
        serde_json::from_str(&read(&dir, &id, "manifest.json")).expect("manifest");
    assert_eq!(m["species"][0]["own_data"]["gbif_records_used"], false);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn datasets_are_checked_copied_and_deleted() {
    let dir = root("lifecycle");
    let e = Engine::open(cfg(&dir)).expect("engine");
    let org_a = Scope::Org("org-a".into());
    let d = e
        .import_dataset(&survey(50, 0), &org_a, None)
        .expect("import");
    let ddir = dir.join("data/datasets").join(&d.id);
    assert!(ddir.join("records.json").exists());
    // Another organisation (and local mode) cannot see, use or delete it.
    for other in [Scope::Org("org-b".into()), Scope::All] {
        assert!(matches!(
            e.store().dataset_get(&d.id, &other),
            Err(niche_app::AppError::NotFound(_))
        ));
        assert!(e.store().dataset_list(&other).expect("list").is_empty());
        assert!(matches!(
            e.delete_dataset(&d.id, &other),
            Err(niche_app::AppError::NotFound(_))
        ));
    }
    let own = OwnSpeciesData {
        values: vec![TAXON.into()],
        role: OwnDataRole::Validation,
        gbif: true,
    };
    // Local mode submits in Scope::All: the organisation's dataset is "not found".
    assert!(matches!(
        e.submit(spec(Some(own.clone()), Some(&d))),
        Err(niche_app::AppError::NotFound(_))
    ));
    // Consistency checks of the specification.
    let mut s = spec(Some(own.clone()), None);
    assert!(matches!(
        e.submit(s.clone()),
        Err(niche_app::AppError::Invalid(_))
    ));
    s.species[0].own = None;
    s.own_dataset = Some(DatasetRef {
        id: d.id.clone(),
        records_sha256: d.records_sha256.clone(),
    });
    assert!(matches!(e.submit(s), Err(niche_app::AppError::Invalid(_))));
    let mut bad = own.clone();
    bad.gbif = false;
    assert!(matches!(
        e.submit(spec(Some(bad), Some(&d))),
        Err(niche_app::AppError::Invalid(_))
    ));
    // In local mode: a changed checksum is refused, an unknown taxon value too.
    let local = e
        .import_dataset(&survey(50, 0), &Scope::All, None)
        .expect("import");
    let mut changed = spec(Some(own.clone()), Some(&local));
    if let Some(r) = changed.own_dataset.as_mut() {
        r.records_sha256 = "0".repeat(64);
    }
    assert!(matches!(
        e.submit(changed),
        Err(niche_app::AppError::Conflict(_))
    ));
    let mut unknown = own.clone();
    unknown.values = vec!["Bufo bufo".into()];
    assert!(matches!(
        e.submit(spec(Some(unknown), Some(&local))),
        Err(niche_app::AppError::Invalid(_))
    ));
    assert_eq!(
        std::fs::read_dir(dir.join("data/jobs"))
            .map(|r| r.count())
            .unwrap_or(0),
        0,
        "refused studies leave no copy behind"
    );
    // A study keeps its copy when the dataset is deleted; the copy goes with the study.
    let job = e.submit(spec(Some(own), Some(&local))).expect("submit").id;
    let copy = dir
        .join("data/jobs")
        .join(&job)
        .join("input/own-records.json");
    assert!(copy.exists());
    e.delete_dataset(&local.id, &Scope::All).expect("delete");
    assert!(!dir.join("data/datasets").join(&local.id).exists());
    assert!(
        e.store()
            .dataset_list(&Scope::All)
            .expect("list")
            .is_empty()
    );
    assert!(copy.exists(), "submitted studies keep their copy");
    e.cancel(&job).expect("cancel");
    e.delete_job(&job).expect("delete job");
    assert!(!copy.exists());
    // Organisation A deletes its own.
    e.delete_dataset(&d.id, &org_a).expect("delete");
    assert!(!ddir.exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn default_specifications_are_unchanged() {
    // A specification written before ADR-0108 reads without the new fields and serializes to
    // the same JSON, hence the same `spec_sha256`.
    let old = serde_json::to_string(&spec(None, None)).expect("json");
    let back: JobSpec = serde_json::from_str(&old).expect("read");
    assert!(
        !old.contains("\"own\":") && !old.contains("own_dataset"),
        "{old}"
    );
    assert_eq!(back, spec(None, None));
    assert_eq!(serde_json::to_string(&back).expect("json"), old);
    // A GBIF-only study writes no source, no per-source counts and no independent validation.
    let dir = root("default");
    let e = Engine::open(cfg(&dir)).expect("engine");
    let (id, r) = run(&e, spec(None, None));
    assert!(r.data.by_source.is_empty() && r.independent_validation.is_none());
    assert!(
        r.evidence
            .reasons
            .iter()
            .any(|x| x.starts_with("No independent validation data"))
    );
    let occ = read(&dir, &id, "0/occurrences.geojson");
    assert!(!occ.contains("\"source\""));
    let result = read(&dir, &id, "0/result.json");
    assert!(!result.contains("by_source") && !result.contains("independent_validation"));
    let m: serde_json::Value =
        serde_json::from_str(&read(&dir, &id, "manifest.json")).expect("manifest");
    assert!(m.get("own_data").is_none() && m["methods"].get("own_data").is_none());
    let _ = std::fs::remove_dir_all(dir);
}
