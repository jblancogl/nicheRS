//! Network tests for the terrain and land-cover predictors (Copernicus DEM GLO-90, ESA
//! WorldCover 2021) over Provence, around 43.5° N 5.5° E. Run with
//! `cargo test --release -p niche-data --test live_predictors -- --ignored --nocapture`.
//! Each test uses a fresh cache so the timings include downloads and computation.

use niche_data::{Http, chelsa, dem, landcover, predictors};
use niche_geo::Raster;

fn fresh_cache(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("niche-live-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn stats(r: &Raster) -> (usize, f32, f32, f64) {
    let v: Vec<f32> = r.data.iter().copied().filter(|x| !x.is_nan()).collect();
    let (mn, mx) = v
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), &x| (a.min(x), b.max(x)));
    let mean = v.iter().map(|&x| f64::from(x)).sum::<f64>() / v.len().max(1) as f64;
    (v.len(), mn, mx, mean)
}

/// CHELSA-aligned 1° window whose edges are 0.5″ west/north of the round degrees (the CHELSA
/// lattice offset): it takes all its DEM pixels from one tile, except its southern row, which
/// also holds row 0 of the tile below.
fn degree_window(g: &niche_geo::GridSpec, lat0: f64, lon0: f64) -> (usize, usize, usize, usize) {
    let r0 = ((g.north - (lat0 + 1.0)) / g.cell).round() as usize;
    let c0 = ((lon0 - g.west) / g.cell).round() as usize;
    (r0, c0, 120, 120)
}

#[test]
#[ignore = "network"]
fn dem_tile_provence() {
    let http = Http::new();
    let dir = fresh_cache("dem");
    let g = chelsa::native_grid(&http, &dir).expect("grid");
    println!("CHELSA grid: {g:?}");
    let w = degree_window(&g, 43.0, 5.0);
    let t = std::time::Instant::now();
    let elev = dem::read_window(&http, &dir, &g, w, dem::DemLayer::Elevation).expect("elev");
    let first = t.elapsed();
    let t = std::time::Instant::now();
    let slope = dem::read_window(&http, &dir, &g, w, dem::DemLayer::Slope).expect("slope");
    let rough = dem::read_window(&http, &dir, &g, w, dem::DemLayer::Roughness).expect("rough");
    let cached = t.elapsed();
    for (name, r) in [
        ("elevation", &elev),
        ("slope", &slope),
        ("roughness", &rough),
    ] {
        let (n, mn, mx, mean) = stats(r);
        println!(
            "{name}: {n} valid / {} cells, min {mn:.1}, max {mx:.1}, mean {mean:.2}",
            r.data.len()
        );
    }
    let cache_bytes: u64 = walk(&dir.join("dem"));
    println!(
        "first read (download + aggregate, tiles touched by the window): {first:?}; two more layers from cache: {cached:?}; dem cache {cache_bytes} bytes"
    );
    // Provence 43–44° N, 5–6° E: from the Mediterranean and the Camargue edge (≈ 0 m; EGM2008
    // heights can be slightly negative) to the Monts de Vaucluse (Signal de Saint-Pierre, 1256 m;
    // max raw 3″ pixel in the tile: 1252.1 m at 43.98° N 5.485° E); Mont Ventoux and Lure lie
    // north of 44° N, outside the window.
    let (n, mn, mx, _) = stats(&elev);
    assert!(n > 10_000);
    assert!(mn < 50.0 && mx > 1000.0 && mx < 2000.0, "{mn} {mx}");
    let (_, smn, smx, smean) = stats(&slope);
    assert!(smn >= 0.0 && smx < 60.0 && smean > 2.0 && smean < 20.0);
    // Cell over Marseille's sea (5.25° E, 43.2° N) is 0 m elevation or NoData.
    if let Some(v) = elev.sample(5.25, 43.2) {
        println!("sea cell 5.25E 43.2N elevation {v}");
    }
    // Pic des Mouches (Sainte-Victoire, 1011 m; the raw 3″ pixel reads 1006.4 m): the 30″ cell
    // mean is lower because the cell also covers the steep southern face.
    if let Some(v) = elev.sample(5.6098, 43.5314) {
        println!("Pic des Mouches (Sainte-Victoire) 30″ cell: mean elevation {v} m");
    }
    let _ = std::fs::remove_dir_all(dir);
}

fn walk(p: &std::path::Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(p) else {
        return 0;
    };
    rd.filter_map(Result::ok)
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                walk(&p)
            } else {
                e.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

#[test]
#[ignore = "network"]
fn worldcover_degree_provence() {
    let http = Http::new();
    let dir = fresh_cache("worldcover");
    let g = chelsa::native_grid(&http, &dir).expect("grid");
    let list = landcover::tile_list(&http, &dir).expect("list");
    println!("{} WorldCover tiles listed", list.len());
    let (_, st) = landcover::block(&http, &dir, &g, &list, 43, 5).expect("block");
    let st = st.expect("computed, not cached");
    println!(
        "WorldCover 1° block N43E005: {} bytes in {} range requests, {:?} (download + decode + aggregate)",
        st.bytes_read, st.requests, st.elapsed
    );
    let t = std::time::Instant::now();
    let (_, again) = landcover::block(&http, &dir, &g, &list, 43, 5).expect("cached");
    assert!(again.is_none());
    println!("same block from cache: {:?}", t.elapsed());
    // Window strictly inside the degree (no neighbour blocks needed).
    let w = degree_window(&g, 43.0, 5.0);
    let inner = (w.0 + 1, w.1 + 1, 118, 118);
    let fr = landcover::read_fractions(&http, &dir, &g, inner).expect("fractions");
    let mut worst = 0.0f32;
    let mut valid = 0;
    for k in 0..fr[0].data.len() {
        let s: f32 = fr.iter().map(|f| f.data[k]).sum();
        if !s.is_nan() {
            valid += 1;
            worst = worst.max((s - 1.0).abs());
        }
    }
    println!("{valid} valid cells, max |Σ fractions − 1| = {worst:e}");
    for (name, f) in landcover::GROUPS.iter().zip(&fr) {
        let (_, _, _, mean) = stats(f);
        println!("  {name:8} mean fraction {mean:.3}");
    }
    assert!(valid > 10_000 && worst < 1e-5);
    let tree = stats(&fr[0]).3;
    assert!(
        tree > 0.2 && tree < 0.8,
        "Provence is largely wooded: {tree}"
    );
    let cache_bytes: u64 = walk(&dir.join("worldcover").join("agg-v1"));
    println!("worldcover block cache: {cache_bytes} bytes");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore = "network"]
fn dist_water_provence() {
    let http = Http::new();
    let dir = fresh_cache("distwater");
    let g = chelsa::native_grid(&http, &dir).expect("grid");
    // 0.25° × 0.25° around the Étang de Berre (5.1° E, 43.45° N).
    let r0 = ((g.north - 43.6) / g.cell) as usize;
    let c0 = ((5.0 - g.west) / g.cell) as usize;
    let w = (r0, c0, 30, 30);
    let p = predictors::padded_window(&g, w, predictors::DIST_WATER_MAX_KM);
    println!("window {w:?} padded to {p:?}");
    let t = std::time::Instant::now();
    let d = predictors::read_window_on_grid(&http, &dir, &g, "dist_water", w).expect("dist");
    println!(
        "dist_water (with WorldCover blocks of the padding): {:?}",
        t.elapsed()
    );
    let (n, mn, mx, mean) = stats(&d);
    println!("dist_water: {n} valid, min {mn:.2} km, max {mx:.2} km, mean {mean:.2} km");
    assert!(mn == 0.0 && mx <= 25.0);
    let _ = std::fs::remove_dir_all(dir);
}
