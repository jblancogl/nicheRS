//! Study-area size check (offline): cells per offered resolution against the hosted limit,
//! counted over the bounding box like the stack builder does.

use niche_app::dto::*;
use niche_app::engine::Limits;
use niche_app::{Config, Engine};

fn engine() -> (Engine, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("niche-area-check-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cache = dir.join("cache");
    std::fs::create_dir_all(&cache).expect("cache dir");
    // CHELSA v2.1 grid description (30″ cells), as cached after the first remote read.
    std::fs::write(
        cache.join("chelsa_grid.json"),
        "[-180.0001388888889, 84.00013888888888, 0.008333333333333333, 43200, 20880]",
    )
    .expect("grid");
    let cfg = Config {
        data_dir: dir.join("data"),
        cache_dir: cache,
        max_records: 5000,
        gbif: None,
        secret_key: None,
        mirror: None,
        limits: Limits::hosted(),
        retention_days: 0,
    };
    (Engine::open(cfg).expect("engine"), dir)
}

fn boxes(name: &str, parts: &[[f64; 4]]) -> StudyArea {
    let polys: Vec<_> = parts
        .iter()
        .map(|[w, s, e, n]| vec![vec![[*w, *s], [*e, *s], [*e, *n], [*w, *n], [*w, *s]]])
        .collect();
    StudyArea {
        name: name.into(),
        geometry: serde_json::json!({"type": "MultiPolygon", "coordinates": polys}),
        places: vec![],
    }
}

#[test]
fn far_apart_parts_need_a_coarser_resolution() {
    let (e, dir) = engine();
    // Peninsula and the Canary Islands: the grid spans the box around both.
    let spain = boxes(
        "España",
        &[[-9.3, 36.0, 3.3, 43.8], [-18.2, 27.6, -13.4, 29.5]],
    );
    let c = e.check_area(&spain).expect("check");
    assert_eq!(c.max_cells, 2_500_000);
    let fits: Vec<(usize, bool)> = c.resolutions.iter().map(|r| (r.factor, r.fits)).collect();
    assert_eq!(fits, [(1, false), (2, true), (5, true), (10, true)]);
    assert!(c.resolutions[0].cells > 5_000_000, "{:?}", c.resolutions[0]);

    // The peninsula alone fits at 1 km.
    let peninsula = boxes("Península", &[[-9.3, 36.0, 3.3, 43.8]]);
    assert!(e.check_area(&peninsula).expect("check").resolutions[0].fits);
    let _ = std::fs::remove_dir_all(dir);
}
