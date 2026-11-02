//! Decision-support outputs (ADR-0018) on the synthetic demo: survey priority with the
//! target-group effort surface and with the distance proxy, project footprints with exact areas,
//! the multi-species aggregation on the two virtual species, comparison of two studies and its
//! refusal on different grids. Analyses must never change a study's outputs or manifest.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use niche_app::accounts::Scope;
use niche_app::analyses::read_tif;
use niche_app::artifacts::sha256;
use niche_app::dto::*;
use niche_app::{AppError, Config, Engine, demo};
use niche_geo::Raster;
use niche_geo::area::quad_area_km2;

fn root(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("niche-decision-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn engine(dir: &Path) -> Engine {
    Engine::open(Config {
        data_dir: dir.join("data"),
        cache_dir: dir.join("cache"),
        max_records: 5000,
        gbif: None,
        secret_key: None,
        mirror: None,
        limits: Default::default(),
        retention_days: 0,
    })
    .expect("engine")
}

fn species(keys: &[u64]) -> Vec<SpeciesRef> {
    keys.iter()
        .map(|&k| SpeciesRef {
            key: k,
            scientific_name: demo::species_name(k).into(),
            own: None,
        })
        .collect()
}

/// A small, fast demo study (one fixed setting, coarser grid).
fn spec(keys: &[u64], factor: usize, background: &str) -> JobSpec {
    JobSpec {
        name: format!("decision {background} x{factor}"),
        species: species(keys),
        area: StudyArea {
            name: "Demo".into(),
            geometry: demo::area_geojson(),
            places: vec![],
        },
        layers: demo::LAYERS.iter().map(|l| l.0.to_string()).collect(),
        resolution_factor: factor,
        filters: OccurrenceFilters::default(),
        model: ModelSettings {
            tuning: "fixed".into(),
            feature_classes: vec!["LQ".into()],
            beta_multipliers: vec![1.0],
            background_points: 2000,
            background_method: background.into(),
            ..ModelSettings::default()
        },
        validation: ValidationSettings::default(),
        seed: 5,
        demo: true,
        own_dataset: None,
        projections: vec![],
    }
}

fn run(e: &Engine, spec: JobSpec) -> String {
    let id = e.submit(spec).expect("submit").id;
    let never = AtomicBool::new(false);
    e.run_job(&id, &never, &never);
    let res = e.results(&id).expect("results");
    assert_eq!(res.job.status, JobStatus::Succeeded, "{:?}", res.job.error);
    for r in &res.species {
        assert_eq!(r.status, SpeciesRunStatus::Succeeded, "{:?}", r.error);
    }
    id
}

/// Every file of a study outside `analyses/`, by relative path.
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("dir").flatten() {
            let p = e.path();
            let rel = p
                .strip_prefix(dir)
                .expect("prefix")
                .to_string_lossy()
                .to_string();
            if rel.starts_with("analyses") {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else {
                out.insert(rel, std::fs::read(&p).expect("read"));
            }
        }
    }
    out
}

fn job_dir(dir: &Path, id: &str) -> PathBuf {
    dir.join("data/jobs").join(id)
}

fn tif(path: PathBuf) -> Raster {
    read_tif(std::fs::read(path).expect("tif")).expect("geotiff")
}

fn analysis_dir(dir: &Path, id: &str, aid: &str) -> PathBuf {
    job_dir(dir, id).join("analyses").join(aid)
}

/// The analysis manifest lists every output with its checksum, the study manifest's checksum
/// and the checksum of every study file read.
fn check_manifest(dir: &Path, id: &str, a: &Analysis) -> serde_json::Value {
    let adir = analysis_dir(dir, id, &a.id);
    let m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(adir.join("manifest.json")).expect("manifest"))
            .expect("json");
    assert_eq!(m["format"], "nichers-analysis-manifest/1");
    assert_eq!(m["kind"], a.kind.as_str());
    let study_manifest = std::fs::read(job_dir(dir, id).join("manifest.json")).expect("study");
    assert_eq!(m["study"]["manifest_sha256"], sha256(&study_manifest));
    let outs = m["outputs"].as_object().expect("outputs");
    assert!(outs.contains_key("analysis.json"));
    for f in &a.files {
        let bytes = std::fs::read(adir.join(&f.name)).expect("output");
        assert_eq!(outs[&f.name]["sha256"], sha256(&bytes), "{}", f.name);
    }
    for i in m["inputs"].as_array().expect("inputs") {
        let p = job_dir(dir, i["job_id"].as_str().expect("job"))
            .join(i["species_key"].as_u64().expect("key").to_string())
            .join(i["file"].as_str().expect("file"));
        assert_eq!(i["sha256"], sha256(&std::fs::read(p).expect("input")));
    }
    assert!(!m["inputs"].as_array().expect("inputs").is_empty());
    let html =
        String::from_utf8(std::fs::read(adir.join("report.html")).expect("report")).expect("utf-8");
    assert!(
        html.contains("does not replace a field inventory")
            || html.contains("do not replace a field inventory")
    );
    assert!(html.contains("SYNTHETIC"));
    assert!(
        !html.to_lowercase().contains("probability of presence")
            || html.contains("not a probability")
    );
    m
}

#[test]
fn second_demo_species_leaves_the_first_unchanged() {
    for seed in [1, 5, 42] {
        let a = demo::occurrences(seed);
        for key in [0, 1, 7] {
            let b = demo::occurrences_for(key, seed);
            assert_eq!(a.len(), b.len());
            assert!(
                a.iter()
                    .zip(&b)
                    .all(|(x, y)| x.key == y.key && x.lng == y.lng && x.lat == y.lat)
            );
        }
        let s = demo::occurrences_for(demo::SECOND_SPECIES_KEY, seed);
        assert!(s.iter().zip(&a).any(|(x, y)| x.lng != y.lng));
    }
    assert_eq!(demo::species_name(0), demo::SPECIES_NAME);
    assert!(demo::species_name(demo::SECOND_SPECIES_KEY).contains("SYNTHETIC"));
}

#[test]
fn analyses_leave_the_study_unchanged_and_are_traceable() {
    let dir = root("trace");
    let e = engine(&dir);
    let solo = run(&e, spec(&[0], 2, "random"));
    let id = run(&e, spec(&[0, demo::SECOND_SPECIES_KEY], 2, "random"));
    // The first species' outputs do not depend on the second species being in the study.
    for f in [
        "suitability.tif",
        "mess.tif",
        "uncertainty.tif",
        "occurrences.geojson",
    ] {
        assert!(
            std::fs::read(job_dir(&dir, &solo).join("0").join(f)).expect("solo")
                == std::fs::read(job_dir(&dir, &id).join("0").join(f)).expect("pair"),
            "{f}"
        );
    }
    let before = snapshot(&job_dir(&dir, &id));
    let scope = Scope::All;

    // Survey priority without an effort surface: distance proxy.
    let (p, created) = e
        .create_analysis(
            &id,
            &AnalysisRequest::Priority {
                species_key: 0,
                top_n: Some(5),
            },
            &scope,
        )
        .expect("priority");
    assert!(created);
    assert_eq!(p.id, "priority-0");
    let pr = p.priority.as_ref().expect("priority result");
    assert_eq!(pr.method, "distance");
    assert!(pr.known_records.is_some_and(|n| n > 50));
    assert!(
        p.warnings
            .iter()
            .any(|w| w.code == "priority_distance_proxy")
    );
    assert!(pr.candidates.len() <= 5 && !pr.candidates.is_empty());
    let total: f64 = pr.classes.iter().map(|c| c.area_km2).sum();
    assert!((total - pr.study_area_km2).abs() < 1e-6 * total);
    check_manifest(&dir, &id, &p);
    // Asking again returns the same analysis, not a new one.
    let (again, created) = e
        .create_analysis(
            &id,
            &AnalysisRequest::Priority {
                species_key: 0,
                top_n: None,
            },
            &scope,
        )
        .expect("priority again");
    assert!(!created);
    assert_eq!(again, p);

    // Footprint and aggregation.
    let geometry = serde_json::json!({"type": "Polygon", "coordinates": [[[4.2, 44.6], [4.6, 44.6], [4.6, 44.9], [4.2, 44.9], [4.2, 44.6]]]});
    let (f, _) = e
        .create_analysis(
            &id,
            &AnalysisRequest::Footprint {
                name: "Wind farm (SYNTHETIC)".into(),
                geometry,
            },
            &scope,
        )
        .expect("footprint");
    check_manifest(&dir, &id, &f);
    let (r, _) = e
        .create_analysis(
            &id,
            &AnalysisRequest::Richness {
                min_evidence: Some(EvidenceLevel::Exploratory),
            },
            &scope,
        )
        .expect("richness");
    check_manifest(&dir, &id, &r);

    // Nothing of the study changed.
    assert!(
        before == snapshot(&job_dir(&dir, &id)),
        "study files changed"
    );
    assert_eq!(e.analyses(&id).expect("list").len(), 3);

    // Files are served by name only; deleting removes the directory.
    assert!(e.analysis_file(&id, &f.id, "footprint_species.csv").is_ok());
    assert!(e.analysis_file(&id, &f.id, "manifest.json").is_ok());
    assert!(e.analysis_file(&id, &f.id, "../manifest.json").is_err());
    assert!(e.analysis(&id, "../../x").is_err());
    e.delete_analysis(&id, &f.id).expect("delete");
    assert!(!analysis_dir(&dir, &id, &f.id).exists());
    assert_eq!(e.analyses(&id).expect("list").len(), 2);
    // Deleting the study deletes its analyses.
    e.delete_job(&id).expect("delete study");
    assert!(!job_dir(&dir, &id).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn survey_priority_with_the_target_group_effort_surface() {
    let dir = root("effort");
    let e = engine(&dir);
    let id = run(&e, spec(&[0], 2, "target_group"));
    let (p, _) = e
        .create_analysis(
            &id,
            &AnalysisRequest::Priority {
                species_key: 0,
                top_n: None,
            },
            &Scope::All,
        )
        .expect("priority");
    let pr = p.priority.as_ref().expect("result");
    eprintln!(
        "priority (effort): classes {:?}, groups {}, median effort {}",
        pr.classes
            .iter()
            .map(|c| (c.code, c.cells, c.area_km2.round()))
            .collect::<Vec<_>>(),
        pr.candidate_groups,
        pr.sampling_split
    );
    assert_eq!(pr.method, "effort");
    assert!(pr.effort_synthetic);
    assert_eq!(
        pr.target_group.as_deref(),
        Some(niche_app::bias::DEMO_CLASS)
    );
    assert!(pr.known_records.is_none());
    assert!(
        !p.warnings
            .iter()
            .any(|w| w.code == "priority_distance_proxy")
    );
    assert!(pr.candidates.len() <= 20);
    assert!(pr.candidate_groups >= pr.candidates.len());
    for w in pr.candidates.windows(2) {
        assert!(w[0].mean_priority >= w[1].mean_priority);
    }
    for c in &pr.candidates {
        assert!(c.mean_suitability >= pr.suitability_threshold - 1e-6);
        assert!(c.mean_effort.is_some() && c.mean_distance_km.is_none());
        assert!(c.lng >= c.bbox[0] - 1e-9 && c.lng <= c.bbox[2] + 1e-9);
    }
    let adir = analysis_dir(&dir, &id, &p.id);
    let score = tif(adir.join("priority.tif"));
    let classes = tif(adir.join("priority_class.tif"));
    let suit = tif(job_dir(&dir, &id).join("0/suitability.tif"));
    let effort = tif(job_dir(&dir, &id).join("0/effort.tif"));
    let mut seen = [0usize; 4];
    for i in 0..suit.data.len() {
        assert_eq!(suit.data[i].is_finite(), score.data[i].is_finite());
        if !suit.data[i].is_finite() {
            continue;
        }
        assert!((0.0..=1.0).contains(&score.data[i]));
        let code = classes.data[i] as usize;
        seen[code - 1] += 1;
        // Higher suitability = at or above the P10 threshold.
        assert_eq!(
            f64::from(suit.data[i]) >= pr.suitability_threshold,
            code <= 2,
            "cell {i}"
        );
        // Cells without target-group records are never "well sampled" when most cells have
        // none (median 0).
        if pr.sampling_split == 0.0 && effort.data[i] > 0.0 {
            assert!(code == 2 || code == 4);
        }
    }
    for (c, n) in pr.classes.iter().zip(seen) {
        assert_eq!(c.cells, n);
    }
    let csv = std::fs::read_to_string(adir.join("candidates.csv")).expect("csv");
    assert_eq!(csv.lines().count(), pr.candidates.len() + 1);
    let gj: serde_json::Value =
        serde_json::from_slice(&std::fs::read(adir.join("candidates.geojson")).expect("geojson"))
            .expect("json");
    assert_eq!(
        gj["features"].as_array().map(Vec::len),
        Some(pr.candidates.len())
    );
    assert!(
        gj["description"]
            .as_str()
            .is_some_and(|d| d.contains("Not where the species is"))
    );
    let html = std::fs::read_to_string(adir.join("report.html")).expect("html");
    assert!(html.contains("areas worth surveying"));
    assert!(html.contains("Guisan"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn footprint_areas_are_exact_on_the_model_grid() {
    let dir = root("footprint");
    let e = engine(&dir);
    let id = run(&e, spec(&[0, demo::SECOND_SPECIES_KEY], 2, "random"));
    let suit = tif(job_dir(&dir, &id).join("0/suitability.tif"));
    let g = suit.spec;
    // A footprint made of exactly 6 × 4 model cells, inside the study area.
    let (c0, r0) = (100usize, 40usize);
    let (w, n) = (g.west + c0 as f64 * g.cell, g.north - r0 as f64 * g.cell);
    let (east, s) = (w + 6.0 * g.cell, n - 4.0 * g.cell);
    let geometry = serde_json::json!({"type": "Polygon", "coordinates": [[[w, s], [east, s], [east, n], [w, n], [w, s]]]});
    let (a, _) = e
        .create_analysis(
            &id,
            &AnalysisRequest::Footprint {
                name: "Quarry".into(),
                geometry,
            },
            &Scope::All,
        )
        .expect("footprint");
    let f = a.footprint.as_ref().expect("result");
    let mut want = 0.0;
    let mut cells = Vec::new();
    for r in r0..r0 + 4 {
        let top = g.north - r as f64 * g.cell;
        for c in c0..c0 + 6 {
            want += quad_area_km2(top - g.cell, top, g.cell);
            cells.push((g.index(r, c), quad_area_km2(top - g.cell, top, g.cell)));
        }
    }
    assert!(
        (f.area_km2 - want).abs() < 1e-9 * want,
        "{} {want}",
        f.area_km2
    );
    assert!((f.in_study_km2 - want).abs() < 1e-9 * want);
    assert_eq!(f.cells, 24);
    assert_eq!(f.species.len(), 2);
    let r = &e.results(&id).expect("results").species;
    for (sp, res) in f.species.iter().zip(r) {
        let s = tif(job_dir(&dir, &id)
            .join(res.species_key.to_string())
            .join("suitability.tif"));
        assert_eq!(sp.thresholds.first().map(|t| t.rule.as_str()), Some("p10"));
        for t in &sp.thresholds {
            let inside: f64 = cells
                .iter()
                .filter(|(i, _)| f64::from(s.data[*i]) >= t.threshold)
                .map(|c| c.1)
                .sum();
            assert!(
                (t.suitable_km2 - inside).abs() < 1e-9 * want.max(1.0),
                "{}",
                t.rule
            );
            assert!((t.share_of_footprint - inside / want).abs() < 1e-12);
            assert!(t.share_of_species_area <= 1.0);
            assert!(t.species_suitable_km2 >= t.suitable_km2);
        }
        let mean = cells
            .iter()
            .map(|(i, a)| a * f64::from(s.data[*i]))
            .sum::<f64>()
            / want;
        assert!((sp.mean_suitability.expect("mean") - mean).abs() < 1e-9);
        assert!(sp.evidence_level.is_some());
        assert!(sp.outside_aoa_share.is_none());
    }
    // Half a cell: exact quadrangle, flagged as smaller than a cell.
    let half = serde_json::json!({"type": "Polygon", "coordinates": [[[w, n - g.cell], [w + g.cell / 2.0, n - g.cell], [w + g.cell / 2.0, n], [w, n], [w, n - g.cell]]]});
    let (h, _) = e
        .create_analysis(
            &id,
            &AnalysisRequest::Footprint {
                name: "Pylon".into(),
                geometry: half,
            },
            &Scope::All,
        )
        .expect("half");
    let hf = h.footprint.as_ref().expect("half result");
    let want = quad_area_km2(n - g.cell, n, g.cell / 2.0);
    assert!((hf.area_km2 - want).abs() < 1e-9 * want);
    assert_eq!(hf.cells, 1);
    assert!(
        h.warnings
            .iter()
            .any(|w| w.code == "footprint_smaller_than_cell")
    );
    // Partly outside the study area: share flagged; entirely outside: refused.
    let part = serde_json::json!({"type": "Polygon", "coordinates": [[[2.5, 44.0], [3.2, 44.0], [3.2, 44.2], [2.5, 44.2], [2.5, 44.0]]]});
    let (p, _) = e
        .create_analysis(
            &id,
            &AnalysisRequest::Footprint {
                name: "Road".into(),
                geometry: part,
            },
            &Scope::All,
        )
        .expect("partial");
    assert!(
        p.warnings
            .iter()
            .any(|w| w.code == "footprint_outside_study_area")
    );
    let away = serde_json::json!({"type": "Polygon", "coordinates": [[[8.0, 44.0], [8.2, 44.0], [8.2, 44.2], [8.0, 44.2], [8.0, 44.0]]]});
    assert!(matches!(
        e.create_analysis(
            &id,
            &AnalysisRequest::Footprint {
                name: "Far".into(),
                geometry: away
            },
            &Scope::All
        ),
        Err(AppError::Invalid(_))
    ));
    let bad = serde_json::json!({"type": "Point", "coordinates": [4.0, 44.0]});
    assert!(matches!(
        e.create_analysis(
            &id,
            &AnalysisRequest::Footprint {
                name: "Point".into(),
                geometry: bad
            },
            &Scope::All
        ),
        Err(AppError::Invalid(_))
    ));
    let csv = std::fs::read_to_string(analysis_dir(&dir, &id, &a.id).join("footprint_species.csv"))
        .expect("csv");
    // Two species × three rules.
    assert_eq!(csv.lines().count(), 1 + 6);
    let html =
        std::fs::read_to_string(analysis_dir(&dir, &id, &a.id).join("report.html")).expect("html");
    assert!(html.contains("low suitability is not evidence of absence"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn richness_stacks_the_two_virtual_species() {
    let dir = root("richness");
    let e = engine(&dir);
    let id = run(&e, spec(&[0, demo::SECOND_SPECIES_KEY], 2, "random"));
    let results = e.results(&id).expect("results").species;
    let (a, _) = e
        .create_analysis(
            &id,
            &AnalysisRequest::Richness {
                min_evidence: Some(EvidenceLevel::Exploratory),
            },
            &Scope::All,
        )
        .expect("richness");
    assert_eq!(a.id, "richness-exploratory");
    let r = a.richness.as_ref().expect("result");
    eprintln!(
        "richness: included {:?}, histogram {:?}, hotspot {:?}",
        r.included
            .iter()
            .map(|x| (&x.scientific_name, x.evidence_level))
            .collect::<Vec<_>>(),
        r.histogram
            .iter()
            .map(|b| (b.species, b.cells))
            .collect::<Vec<_>>(),
        r.hotspot
    );
    assert_eq!(r.included.len(), 2);
    assert!(
        a.warnings
            .iter()
            .any(|w| w.code == "stacking_overprediction")
    );
    let adir = analysis_dir(&dir, &id, &a.id);
    let rich = tif(adir.join("richness.tif"));
    let sum = tif(adir.join("suitability_sum.tif"));
    let contrib = tif(adir.join("contributing.tif"));
    let hot = tif(adir.join("hotspot.tif"));
    let s: Vec<Raster> = results
        .iter()
        .map(|x| {
            tif(job_dir(&dir, &id)
                .join(x.species_key.to_string())
                .join("suitability.tif"))
        })
        .collect();
    let t: Vec<f64> = results
        .iter()
        .map(|x| {
            x.thresholds
                .iter()
                .find(|t| t.rule == "p10")
                .expect("p10")
                .threshold
        })
        .collect();
    let mut n = 0usize;
    let mut both = 0usize;
    for i in 0..rich.data.len() {
        if !rich.data[i].is_finite() {
            assert!(s.iter().all(|x| !x.data[i].is_finite()));
            continue;
        }
        n += 1;
        let k = s
            .iter()
            .zip(&t)
            .filter(|(x, t)| f64::from(x.data[i]) >= **t)
            .count();
        assert_eq!(rich.data[i] as usize, k);
        let total: f32 = s.iter().map(|x| x.data[i]).sum();
        assert!((sum.data[i] - total).abs() < 1e-5);
        assert_eq!(contrib.data[i], 2.0);
        both += usize::from(k == 2);
    }
    // The two virtual species overlap in part only.
    assert!(both > 0 && both < n, "{both} of {n}");
    assert_eq!(r.histogram.iter().map(|b| b.cells).sum::<usize>(), n);
    // Hotspots: the top 10 % of cells, ranked by count first.
    let hot_cells: Vec<usize> = (0..hot.data.len())
        .filter(|&i| hot.data[i] == 1.0)
        .collect();
    assert_eq!(hot_cells.len(), (n as f64 * 0.1).ceil() as usize);
    assert_eq!(r.hotspot.cells, hot_cells.len());
    let min_hot = hot_cells
        .iter()
        .map(|&i| rich.data[i])
        .fold(f32::MAX, f32::min);
    for i in 0..hot.data.len() {
        if hot.data[i] == 0.0 {
            assert!(rich.data[i] <= min_hot);
        }
    }
    assert_eq!(r.hotspot.min_species, min_hot as usize);
    // A minimum level no species reaches is refused; so is a single-species study.
    let levels: Vec<EvidenceLevel> = results.iter().map(|x| x.evidence.level).collect();
    if levels
        .iter()
        .any(|l| l.rank() < EvidenceLevel::Validated.rank())
    {
        assert!(matches!(
            e.create_analysis(
                &id,
                &AnalysisRequest::Richness {
                    min_evidence: Some(EvidenceLevel::Validated)
                },
                &Scope::All
            ),
            Err(AppError::Invalid(_))
        ));
    }
    // Default (Indicative): Exploratory species are left out and said so.
    match e.create_analysis(
        &id,
        &AnalysisRequest::Richness { min_evidence: None },
        &Scope::All,
    ) {
        Ok((d, _)) => {
            let dr = d.richness.expect("default");
            assert_eq!(dr.min_evidence, EvidenceLevel::Indicative);
            assert!(
                dr.included
                    .iter()
                    .all(|x| x.evidence_level.is_some_and(|l| l.rank() >= 2))
            );
        }
        Err(AppError::Invalid(m)) => {
            assert!(levels.iter().filter(|l| l.rank() >= 2).count() < 2, "{m}");
            assert!(m.contains("Indicative"));
        }
        Err(other) => panic!("{other}"),
    }
    let solo = run(&e, spec(&[0], 2, "random"));
    assert!(matches!(
        e.create_analysis(
            &solo,
            &AnalysisRequest::Richness { min_evidence: None },
            &Scope::All
        ),
        Err(AppError::Invalid(_))
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn comparison_of_two_demo_studies_and_refusal_on_different_grids() {
    let dir = root("compare");
    let e = engine(&dir);
    let a = run(&e, spec(&[0], 2, "random"));
    let b = run(&e, spec(&[0], 2, "target_group"));
    let twin = run(&e, spec(&[0], 2, "random"));
    let coarse = run(&e, spec(&[0], 3, "random"));
    let other_species = run(&e, spec(&[demo::SECOND_SPECIES_KEY], 2, "random"));
    let scope = Scope::All;
    let (c, created) = e
        .create_analysis(
            &a,
            &AnalysisRequest::Comparison {
                other_job_id: b.clone(),
                species_key: 0,
            },
            &scope,
        )
        .expect("comparison");
    assert!(created);
    let cr = c.comparison.as_ref().expect("result");
    eprintln!(
        "comparison random vs target group: D {:.3}, rho {:.3}, mean |B-A| {:.3}, P10 both/only A/only B {:.0}/{:.0}/{:.0} km2",
        cr.schoener_d,
        cr.spearman,
        cr.mean_abs_difference,
        cr.p10_agreement.both_km2,
        cr.p10_agreement.only_a_km2,
        cr.p10_agreement.only_b_km2
    );
    assert!(
        cr.schoener_d > 0.5 && cr.schoener_d < 1.0,
        "{}",
        cr.schoener_d
    );
    assert!(cr.spearman > 0.0 && cr.spearman < 1.0, "{}", cr.spearman);
    assert_eq!(cr.a.background, "random");
    assert_eq!(cr.b.background, "target_group");
    assert!(
        cr.settings
            .iter()
            .any(|d| d.path == "model.background_method"
                && d.a.is_null()
                && d.b == serde_json::json!("target_group"))
    );
    // The difference map is B − A.
    let diff = tif(analysis_dir(&dir, &a, &c.id).join("difference.tif"));
    let sa = tif(job_dir(&dir, &a).join("0/suitability.tif"));
    let sb = tif(job_dir(&dir, &b).join("0/suitability.tif"));
    let mut n = 0;
    for i in 0..diff.data.len() {
        if diff.data[i].is_finite() {
            n += 1;
            assert!((diff.data[i] - (sb.data[i] - sa.data[i])).abs() < 1e-6);
        }
    }
    assert_eq!(n, cr.common_cells);
    let ag = &cr.p10_agreement;
    let total = ag.both_km2 + ag.only_a_km2 + ag.only_b_km2 + ag.neither_km2;
    assert!((total - cr.common_km2).abs() < 1e-6 * total);
    let m = check_manifest(&dir, &a, &c);
    let other_manifest = std::fs::read(job_dir(&dir, &b).join("manifest.json")).expect("b");
    assert_eq!(m["other_study"]["manifest_sha256"], sha256(&other_manifest));
    // Two runs of the same specification: identical surfaces.
    let (t, _) = e
        .create_analysis(
            &a,
            &AnalysisRequest::Comparison {
                other_job_id: twin.clone(),
                species_key: 0,
            },
            &scope,
        )
        .expect("twin");
    let tr = t.comparison.expect("twin result");
    assert!((tr.schoener_d - 1.0).abs() < 1e-12);
    assert!((tr.spearman - 1.0).abs() < 1e-12);
    assert_eq!(tr.mean_abs_difference, 0.0);
    assert!(tr.settings.is_empty(), "{:?}", tr.settings);
    // Different grids: refused with a clear message.
    match e.create_analysis(
        &a,
        &AnalysisRequest::Comparison {
            other_job_id: coarse.clone(),
            species_key: 0,
        },
        &scope,
    ) {
        Err(AppError::Invalid(msg)) => {
            assert!(msg.contains("different grids"), "{msg}");
            assert!(msg.contains("same study area and resolution"), "{msg}");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert!(!analysis_dir(&dir, &a, &format!("comparison-{coarse}-0")).exists());
    // Another species, itself, an unknown study.
    assert!(matches!(
        e.create_analysis(
            &a,
            &AnalysisRequest::Comparison {
                other_job_id: other_species,
                species_key: 0
            },
            &scope
        ),
        Err(AppError::Invalid(_))
    ));
    assert!(
        e.create_analysis(
            &a,
            &AnalysisRequest::Comparison {
                other_job_id: a.clone(),
                species_key: 0
            },
            &scope
        )
        .is_err()
    );
    assert!(matches!(
        e.create_analysis(
            &a,
            &AnalysisRequest::Comparison {
                other_job_id: "nope".into(),
                species_key: 0
            },
            &scope
        ),
        Err(AppError::NotFound(_))
    ));
    // No temporary directory is left behind by refused or failed analyses.
    let leftovers: Vec<_> = std::fs::read_dir(job_dir(&dir, &a).join("analyses"))
        .expect("analyses")
        .flatten()
        .filter(|x| x.file_name().to_string_lossy().starts_with('.'))
        .collect();
    assert!(leftovers.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
