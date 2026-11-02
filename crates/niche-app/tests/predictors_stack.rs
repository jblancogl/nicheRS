//! Terrain, land-cover and distance-to-water predictors in the environmental stack (ADR-0015,
//! Science v1 phase 4), offline: the CHELSA native grid, CHELSA windows and the Natural Earth
//! land mask are written into a fresh cache, and the additional predictors are synthetic rasters
//! handed to `build_stack_with` (production reads them with `niche_data::predictors`). A live
//! test over Provence with real data is `#[ignore]`d at the end.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use niche_app::dto::*;
use niche_app::env::{self, EnvStack, Window};
use niche_app::fit::fit_species;
use niche_app::{artifacts, report_html};
use niche_data::{DataError, Http};
use niche_geo::{GridSpec, Raster};
use serde_json::json;

/// The CHELSA v2.1 lattice (docs/architecture/predictors-v1.md).
const GRID: [f64; 5] = [
    -180.000_138_889,
    83.999_860_415,
    1.0 / 120.0,
    43_200.0,
    20_880.0,
];
const BOX: (f64, f64, f64, f64) = (5.3, 43.4, 5.7, 43.7);

fn root(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("niche-pred-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn area() -> StudyArea {
    let (w, s, e, n) = BOX;
    StudyArea {
        name: "Synthetic box near Aix-en-Provence".into(),
        geometry: json!({"type": "Polygon", "coordinates": [[[w, s], [e, s], [e, n], [w, n], [w, s]]]}),
        places: vec![],
    }
}

fn fc(features: serde_json::Value) -> String {
    json!({"type": "FeatureCollection", "features": features}).to_string()
}

/// Cache with the CHELSA grid, the Natural Earth layers (land everywhere around the box) and
/// CHELSA windows for BIO1 and BIO12 at the window `build_stack` will request.
fn offline_cache(dir: &Path, factor: usize) -> (Window, GridSpec) {
    std::fs::create_dir_all(dir.join("naturalearth")).expect("mkdir");
    std::fs::write(dir.join("chelsa_grid.json"), json!(GRID).to_string()).expect("grid");
    let land = fc(
        json!([{"type": "Feature", "properties": {}, "geometry": {"type": "Polygon",
        "coordinates": [[[0.0, 40.0], [10.0, 40.0], [10.0, 48.0], [0.0, 48.0], [0.0, 40.0]]]}}]),
    );
    std::fs::write(dir.join("naturalearth/ne_10m_land.geojson"), land).expect("land");
    for n in ["ne_10m_minor_islands", "ne_10m_lakes"] {
        std::fs::write(dir.join(format!("naturalearth/{n}.geojson")), fc(json!([{"type": "Feature", "properties": {}, "geometry": {"type": "Polygon", "coordinates": [[[9.0, 47.0], [9.1, 47.0], [9.1, 47.1], [9.0, 47.1], [9.0, 47.0]]]}}]))).expect("ne");
    }
    let win = env::native_window(&Http::new(), dir, BOX, factor).expect("window");
    let w = (win.row, win.col, win.rows, win.cols);
    for id in ["chelsa_bio1", "chelsa_bio12"] {
        let r = synthetic(id, w, win.spec());
        let bytes: Vec<u8> = r.data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let d = dir.join("chelsa").join(id);
        std::fs::create_dir_all(&d).expect("mkdir");
        std::fs::write(
            d.join(format!("{}_{}_{}x{}.f32", w.0, w.1, w.2, w.3)),
            bytes,
        )
        .expect("window");
    }
    (w, win.spec())
}

/// Deterministic pseudo-random value in [0, 1) for a cell and a salt.
fn hash01(r: usize, c: usize, salt: usize) -> f32 {
    let mut x = (r as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((c as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
        .wrapping_add((salt as u64).wrapping_mul(0x1656_67B1_9E37_79F9));
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

const GROUPS: [&str; 8] = [
    "tree", "shrub", "grass", "crop", "built", "bare", "water", "wetland",
];

/// Smooth synthetic fields on the native window. DEM slope is NoData in a 4×4 corner (as for an
/// absent sea tile); land-cover fractions are random compositions summing to 1.
fn synthetic(id: &str, w: Window, spec: GridSpec) -> Raster {
    let mut r = Raster::new(spec, f32::NAN);
    for row in 0..w.2 {
        for col in 0..w.3 {
            let (gr, gc) = (w.0 + row, w.1 + col);
            let v = match id {
                "chelsa_bio1" => 16.0 - 0.05 * row as f32 + 0.5 * hash01(gr, gc, 1),
                "chelsa_bio12" => 600.0 + 3.0 * col as f32 + 40.0 * hash01(gr, gc, 2),
                "dem_slope" if row < 4 && col < 4 => f32::NAN,
                "dem_slope" => 2.0 + 0.2 * (row as f32 - col as f32).abs() + hash01(gr, gc, 3),
                "dist_water" => {
                    (0.4 * (row as f32 - 15.0).abs() + 2.0 * hash01(gr, gc, 4)).min(25.0)
                }
                _ => {
                    let g = GROUPS
                        .iter()
                        .position(|g| id == format!("lc_{g}"))
                        .expect("synthetic layer");
                    let ws: Vec<f32> = (0..8).map(|k| hash01(gr, gc, 10 + k)).collect();
                    ws[g] / ws.iter().sum::<f32>()
                }
            };
            r.data[row * w.3 + col] = v;
        }
    }
    r
}

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

fn build(
    dir: &Path,
    layers: &[String],
    factor: usize,
    reads: &std::sync::Mutex<Vec<(String, Window)>>,
) -> Result<EnvStack, niche_app::AppError> {
    let read = |grid: &GridSpec, id: &str, w: Window| -> Result<Raster, DataError> {
        reads.lock().expect("lock").push((id.to_string(), w));
        let spec = GridSpec {
            west: grid.west + w.1 as f64 * grid.cell,
            north: grid.north - w.0 as f64 * grid.cell,
            cell: grid.cell,
            width: w.3,
            height: w.2,
        };
        Ok(synthetic(id, w, spec))
    };
    env::build_stack_with(
        &Http::new(),
        dir,
        &area(),
        layers,
        factor,
        false,
        &|_, _| {},
        &|| false,
        &read,
    )
}

/// The default (climate-only) path is unchanged: values are the mean-aggregated CHELSA windows,
/// the provenance keeps its single CHELSA block, no reader of additional predictors is called,
/// and the production entry point gives the same stack.
#[test]
fn climate_only_stack_is_unchanged() {
    let dir = root("climate");
    let factor = 2;
    let (w, spec) = offline_cache(&dir, factor);
    let layers = ids(&["chelsa_bio1", "chelsa_bio12"]);
    let reads = std::sync::Mutex::new(vec![]);
    let s = build(&dir, &layers, factor, &reads).expect("stack");
    assert!(reads.lock().expect("lock").is_empty());
    for (l, id) in s.layers.iter().zip(&layers) {
        let want = synthetic(id, w, spec).aggregate_mean(factor);
        assert_eq!(bits(&l.data), bits(&want.data), "{id}");
    }
    let keys: Vec<&String> = s.provenance.as_object().expect("object").keys().collect();
    assert_eq!(
        keys,
        ["citation", "land_mask", "layers", "licence", "source"],
        "provenance shape of climate-only stacks"
    );
    let p = env::build_stack(
        &Http::new(),
        &dir,
        &area(),
        &layers,
        factor,
        false,
        &|_, _| {},
    )
    .expect("production path");
    assert_eq!(p.valid, s.valid);
    assert_eq!(p.provenance, s.provenance);
    for (a, b) in p.layers.iter().zip(&s.layers) {
        assert_eq!(a.data, b.data);
        assert_eq!(a.name, b.name);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Mixed sources: same native window for every layer, same aggregation, NoData of one layer
/// removes the cell, per-source provenance with attributions, selection order kept.
#[test]
fn mixed_source_stack() {
    let dir = root("mixed");
    let factor = 2;
    let (w, spec) = offline_cache(&dir, factor);
    let layers = ids(&[
        "chelsa_bio1",
        "dem_slope",
        "lc_tree",
        "dist_water",
        "chelsa_bio12",
    ]);
    let reads = std::sync::Mutex::new(vec![]);
    let s = build(&dir, &layers, factor, &reads).expect("stack");
    let reads = reads.into_inner().expect("lock");
    assert_eq!(
        reads,
        vec![
            ("dem_slope".to_string(), w),
            ("lc_tree".to_string(), w),
            ("dist_water".to_string(), w),
        ],
        "every predictor read on the CHELSA window"
    );
    let order: Vec<&str> = s.layers.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(
        order,
        [
            "chelsa_bio1",
            "dem_slope",
            "lc_tree",
            "dist_water",
            "chelsa_bio12"
        ]
    );
    assert_eq!(
        s.layers[1].name,
        "SLOPE — Slope (mean of 90 m slopes, Horn)"
    );
    for (l, id) in s.layers.iter().zip(&layers) {
        let want = synthetic(id, w, spec).aggregate_mean(factor);
        assert_eq!(bits(&l.data), bits(&want.data), "{id}");
    }
    // The 4×4 native NoData corner of the slope makes the 2×2 aggregated corner NoData and invalid.
    for (r, c) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let i = s.spec.index(r, c);
        assert!(s.layers[1].data[i].is_nan());
        assert!(!s.valid[i]);
    }
    let i = s.spec.index(3, 3);
    assert!(s.valid[i] == s.area_mask[i]);
    // Provenance: CHELSA block kept, one entry per source with licence and attribution.
    let p = &s.provenance;
    assert_eq!(p["source"], niche_data::chelsa::SOURCE);
    let sources = p["sources"].as_array().expect("sources");
    let names: Vec<&str> = sources
        .iter()
        .map(|x| x["source"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        names,
        [
            niche_data::chelsa::SOURCE,
            niche_data::dem::SOURCE,
            niche_data::landcover::SOURCE
        ]
    );
    assert_eq!(sources[1]["attribution"], niche_data::dem::ATTRIBUTION);
    assert_eq!(
        sources[2]["attribution"],
        niche_data::landcover::ATTRIBUTION
    );
    assert_eq!(sources[2]["layers"], json!(["TREE", "DWAT"]));
    let layer_prov = p["layers"].as_array().expect("layers");
    assert_eq!(layer_prov[1]["id"], "dem_slope");
    assert_eq!(layer_prov[1]["licence"], niche_data::dem::LICENCE);
    assert!(
        layer_prov[2]["method"]
            .as_str()
            .unwrap_or("")
            .contains("10 m pixels")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cancellation_and_grid_mismatch() {
    let dir = root("cancel");
    offline_cache(&dir, 1);
    let layers = ids(&["chelsa_bio1", "lc_tree"]);
    let never = |_: &GridSpec, _: &str, _: Window| -> Result<Raster, DataError> {
        panic!("must not read after cancellation")
    };
    let r = env::build_stack_with(
        &Http::new(),
        &dir,
        &area(),
        &layers,
        1,
        false,
        &|_, _| {},
        &|| true,
        &never,
    );
    assert!(matches!(r, Err(niche_app::AppError::Cancelled)));
    // A predictor on another grid is refused (stacking never resamples, ADR-0010).
    let shifted = |grid: &GridSpec, _: &str, w: Window| -> Result<Raster, DataError> {
        let spec = GridSpec {
            west: grid.west + (w.1 + 1) as f64 * grid.cell,
            north: grid.north - w.0 as f64 * grid.cell,
            cell: grid.cell,
            width: w.3,
            height: w.2,
        };
        Ok(Raster::new(spec, 0.5))
    };
    let r = env::build_stack_with(
        &Http::new(),
        &dir,
        &area(),
        &layers,
        1,
        false,
        &|_, _| {},
        &|| false,
        &shifted,
    );
    assert!(
        matches!(r, Err(niche_app::AppError::Internal(_))),
        "{:?}",
        r.err()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Catalogue: CHELSA defaults unchanged, every additional predictor offered but not selected.
#[test]
fn catalogue_groups_and_defaults() {
    let cat = env::layer_catalogue(false);
    let defaults: Vec<&str> = cat
        .iter()
        .filter(|l| l.default_selected)
        .map(|l| l.id.as_str())
        .collect();
    let chelsa_defaults: Vec<&str> = niche_data::chelsa::LAYERS
        .iter()
        .filter(|l| l.default_selected)
        .map(|l| l.id)
        .collect();
    assert_eq!(
        defaults, chelsa_defaults,
        "ADR-0009 default selection unchanged"
    );
    assert_eq!(cat.len(), 19 + 12);
    for l in &cat {
        let g = l.group.as_str();
        if l.id.starts_with("chelsa_") {
            assert_eq!(g, "climate");
            assert!(l.attribution.is_none());
        } else {
            assert!(!l.default_selected, "{}", l.id);
            assert!(l.attribution.as_deref().is_some_and(|a| !a.is_empty()));
            assert!(l.caution.is_some(), "{}", l.id);
            assert!(["terrain", "land_cover", "water"].contains(&g), "{}", l.id);
        }
    }
    let by = |id: &str| cat.iter().find(|l| l.id == id).expect(id);
    assert_eq!(by("dem_slope").licence, "Copernicus WorldDEM-90 licence");
    assert_eq!(by("lc_tree").licence, "CC BY 4.0");
    assert_eq!(by("dist_water").group, "water");
    assert_eq!(by("lc_tree").unit, "fraction 0–1");
    // Demo catalogue: synthetic layers, all selected.
    assert!(
        env::layer_catalogue(true)
            .iter()
            .all(|l| l.default_selected)
    );
}

/// Land-cover fractions sum to 1: the pairwise Pearson filter keeps all eight (none is
/// pairwise-correlated above 0.7), so the dependency is reported by a warning, not by the filter.
#[test]
fn compositional_fractions_and_warnings() {
    let n = 5000;
    let cols: Vec<Vec<f64>> = (0..8)
        .map(|g| {
            (0..n)
                .map(|i| {
                    let ws: Vec<f32> = (0..8).map(|k| hash01(i, 7, 10 + k)).collect();
                    f64::from(ws[g] / ws.iter().sum::<f32>())
                })
                .collect()
        })
        .collect();
    let (kept, dropped) = niche_eval::collinearity_filter(&cols, 0.7);
    assert_eq!(kept.len(), 8, "{dropped:?}");
    let all: Vec<String> = GROUPS.iter().map(|g| format!("lc_{g}")).collect();
    let refs: Vec<&str> = all.iter().map(String::as_str).collect();
    let codes = |w: Vec<Warning>| w.into_iter().map(|w| w.code).collect::<Vec<_>>();
    assert_eq!(
        codes(env::predictor_warnings(&refs)),
        [
            "land_cover_temporal",
            "built_up_bias",
            "land_cover_composition"
        ]
    );
    assert_eq!(
        codes(env::predictor_warnings(&["chelsa_bio1", "dem_slope"])),
        ["surface_model"]
    );
    assert!(env::predictor_warnings(&["chelsa_bio1", "chelsa_bio12"]).is_empty());
    assert!(!env::uses_additional_predictors(&ids(&["chelsa_bio1"])));
    assert!(env::predictor_limits(&ids(&["chelsa_bio1"])).is_empty());
}

fn virtual_presences(s: &EnvStack, tree: usize, slope: usize) -> Vec<usize> {
    // A virtual species of wooded, gentle terrain: every valid cell with tree fraction above the
    // median and slope below it, thinned deterministically.
    let valid: Vec<usize> = (0..s.valid.len()).filter(|&i| s.valid[i]).collect();
    let med = |k: usize| {
        let mut v: Vec<f32> = valid.iter().map(|&i| s.layers[k].data[i]).collect();
        v.sort_by(f32::total_cmp);
        v[v.len() / 2]
    };
    let (mt, ms) = (med(tree), med(slope));
    valid
        .into_iter()
        .filter(|&i| s.layers[tree].data[i] > mt && s.layers[slope].data[i] < ms)
        .step_by(2)
        .collect()
}

/// The fitting pipeline (collinearity, MaxEnt, cross-validation, MESS, response curves) and the
/// reports run as-is on a mixed-source stack; reports list every source with its attribution.
#[test]
fn fit_and_reports_on_mixed_sources() {
    let dir = root("fit");
    let factor = 1;
    offline_cache(&dir, factor);
    let layers = ids(&[
        "chelsa_bio1",
        "chelsa_bio12",
        "dem_slope",
        "lc_tree",
        "lc_built",
        "dist_water",
    ]);
    let reads = std::sync::Mutex::new(vec![]);
    let s = build(&dir, &layers, factor, &reads).expect("stack");
    let pres = virtual_presences(&s, 3, 2);
    assert!(pres.len() >= 30, "{}", pres.len());
    let background: Vec<usize> = (0..s.valid.len()).filter(|&i| s.valid[i]).collect();
    let sp = SpeciesRef {
        key: 1,
        scientific_name: "Virtualis sylvatica (synthetic)".into(),
        own: None,
    };
    let model = ModelSettings {
        tuning: "fixed".into(),
        feature_classes: vec!["LQ".into()],
        beta_multipliers: vec![1.0],
        ..ModelSettings::default()
    };
    let validation = ValidationSettings::default();
    let never = AtomicBool::new(false);
    let out = fit_species(
        &s,
        &sp,
        &pres,
        &background,
        &model,
        &validation,
        3,
        None,
        None,
        &never,
        &|_, _| {},
    )
    .expect("fit");
    let r = out.result;
    assert_eq!(r.status, SpeciesRunStatus::Succeeded, "{:?}", r.error);
    assert!(
        r.variables_used.iter().any(|v| v.starts_with("TREE")),
        "{:?}",
        r.variables_used
    );
    assert!(
        out.mess
            .iter()
            .zip(&s.valid)
            .all(|(m, v)| !v || m.is_finite())
    );
    assert!(
        out.suitability
            .iter()
            .zip(&s.valid)
            .all(|(x, v)| !v || x.is_finite())
    );
    assert!(
        r.response_curves.iter().any(|c| c.variable == "dist_water"),
        "{:?}",
        r.variables_dropped
    );
    let spec = JobSpec {
        name: "mixed sources".into(),
        species: vec![sp],
        area: area(),
        layers: layers.clone(),
        resolution_factor: factor,
        filters: OccurrenceFilters::default(),
        model,
        validation,
        seed: 3,
        demo: false,
        projections: vec![],
        own_dataset: None,
    };
    let md = artifacts::methods_report(&r, &spec);
    let html = report_html::report_html(&r, &spec, None);
    for text in [&md, &html] {
        assert!(text.contains("Data sources"));
        assert!(text.contains("Copernicus WorldDEM-90 © DLR e.V. 2010-2014"));
        assert!(text.contains("© ESA WorldCover project 2021"));
        assert!(text.contains("Horn"));
        assert!(text.contains("truncated at 25 km"));
        assert!(text.contains("observer effort"));
    }
    // A climate-only spec keeps the previous wording and has no sources section.
    let climate = JobSpec {
        layers: ids(&["chelsa_bio1", "chelsa_bio12"]),
        ..spec
    };
    let html = report_html::report_html(&r, &climate, None);
    assert!(!html.contains("Data sources"));
    assert!(html.contains("Predictors are climate (CHELSA v2.1, 1981–2010) only"));
    assert!(!artifacts::methods_report(&r, &climate).contains("## Data sources"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Real data around 43.5° N 5.5° E: CHELSA + DEM slope + tree cover + distance to water, then a
/// demo-sized fit. `cargo test --release -p niche-app --test predictors_stack -- --ignored --nocapture`
#[test]
#[ignore = "network"]
fn live_provence_stack_and_fit() {
    let dir = root("live");
    let http = Http::new();
    let layers = ids(&[
        "chelsa_bio1",
        "chelsa_bio4",
        "chelsa_bio12",
        "chelsa_bio15",
        "dem_slope",
        "lc_tree",
        "dist_water",
    ]);
    let t = std::time::Instant::now();
    let progress =
        |m: &str, p: f64| println!("  [{:>5.1} s] {p:.2} {m}", t.elapsed().as_secs_f64());
    let s = env::build_stack(&http, &dir, &area(), &layers, 1, false, &progress).expect("stack");
    let fresh = t.elapsed();
    let t2 = std::time::Instant::now();
    let again =
        env::build_stack(&http, &dir, &area(), &layers, 1, false, &|_, _| {}).expect("cached");
    let cached = t2.elapsed();
    assert_eq!(again.valid, s.valid);
    let gi = s.grid_info();
    println!(
        "stack {}×{} cells, {} valid ({:.0} km²): fresh {:.1} s, cached {:.2} s",
        gi.width,
        gi.height,
        gi.cells_in_area,
        gi.area_km2,
        fresh.as_secs_f64(),
        cached.as_secs_f64()
    );
    for l in &s.layers {
        let v: Vec<f32> = (0..s.valid.len())
            .filter(|&i| s.valid[i])
            .map(|i| l.data[i])
            .collect();
        let (mn, mx) = v
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &x| (a.min(x), b.max(x)));
        let mean = v.iter().map(|&x| f64::from(x)).sum::<f64>() / v.len() as f64;
        let nan_in_area = (0..s.valid.len())
            .filter(|&i| s.area_mask[i] && s.land_mask[i] && l.data[i].is_nan())
            .count();
        println!(
            "  {:<45} min {mn:>8.3} max {mx:>8.3} mean {mean:>8.3}  NaN on land in area: {nan_in_area}",
            l.name
        );
    }
    let pres = virtual_presences(&s, 5, 4);
    let background: Vec<usize> = (0..s.valid.len()).filter(|&i| s.valid[i]).collect();
    let model = ModelSettings::default();
    let t3 = std::time::Instant::now();
    let out = fit_species(
        &s,
        &SpeciesRef {
            key: 1,
            scientific_name: "Virtualis sylvatica (synthetic)".into(),
            own: None,
        },
        &pres,
        &background,
        &model,
        &ValidationSettings::default(),
        42,
        None,
        None,
        &AtomicBool::new(false),
        &|_, _| {},
    )
    .expect("fit");
    let r = out.result;
    println!(
        "fit: {} presences, {} background, {:.1} s, {:?}; selected {} β={}; used {:?}; dropped {:?}",
        pres.len(),
        background.len(),
        t3.elapsed().as_secs_f64(),
        r.evidence.level,
        r.selected.feature_classes,
        r.selected.beta_multiplier,
        r.variables_used,
        r.variables_dropped
            .iter()
            .map(|d| format!("{}: {}", d.variable, d.reason))
            .collect::<Vec<_>>()
    );
    for c in &r.contributions {
        println!("  {:<45} {:>5.1} %", c.variable, c.percent);
    }
    assert_eq!(r.status, SpeciesRunStatus::Succeeded, "{:?}", r.error);
    // Same presences and background, climate layers only: effect of the extra predictors on
    // the extrapolated share (MESS < 0) and on the evidence level.
    let climate = ids(&["chelsa_bio1", "chelsa_bio4", "chelsa_bio12", "chelsa_bio15"]);
    let sc =
        env::build_stack(&http, &dir, &area(), &climate, 1, false, &|_, _| {}).expect("climate");
    assert_eq!(
        sc.valid, s.valid,
        "no cell lost to predictor NoData in this area"
    );
    let rc = fit_species(
        &sc,
        &SpeciesRef {
            key: 1,
            scientific_name: "Virtualis sylvatica (synthetic)".into(),
            own: None,
        },
        &pres,
        &background,
        &model,
        &ValidationSettings::default(),
        42,
        None,
        None,
        &AtomicBool::new(false),
        &|_, _| {},
    )
    .expect("fit")
    .result;
    println!(
        "MESS < 0: climate only {:.2} %, with SLOPE/TREE/DWAT {:.2} %; evidence {:?} vs {:?}",
        100.0 * rc.extrapolation.novel_fraction,
        100.0 * r.extrapolation.novel_fraction,
        rc.evidence.level,
        r.evidence.level
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}
