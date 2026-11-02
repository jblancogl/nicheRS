//! Network tests against the real CHELSA and GBIF services. Run with `cargo test -- --ignored`.

use niche_data::{Http, chelsa, gbif};

#[test]
#[ignore = "network"]
fn chelsa_window_brittany() {
    let http = Http::new();
    let dir = std::env::temp_dir().join("niche-live-cache");
    let g = chelsa::native_grid(&http, &dir).expect("grid");
    let c0 = ((-4.8 - g.west) / g.cell) as usize;
    let r0 = ((g.north - 48.9) / g.cell) as usize;
    for id in ["chelsa_bio1", "chelsa_bio4", "chelsa_bio12", "chelsa_bio15"] {
        let t = std::time::Instant::now();
        let r = chelsa::read_window(&http, &dir, id, (r0, c0, 120, 240)).expect("window");
        let v: Vec<f32> = r.data.iter().copied().filter(|x| !x.is_nan()).collect();
        let (mn, mx) = v
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &x| (a.min(x), b.max(x)));
        println!(
            "{id}: {} valid / {} cells, min {mn}, max {mx}, {:?}",
            v.len(),
            r.data.len(),
            t.elapsed()
        );
        assert!(!v.is_empty());
    }
}

#[test]
#[ignore = "network"]
fn gbif_occurrences() {
    let http = Http::new();
    let hits = gbif::search_species(&http, "Lacerta bilineata").expect("search");
    assert!(!hits.is_empty());
    let f = gbif::fetch_occurrences(&http, hits[0].key, (-5.0, 47.0, -1.0, 49.0), 2000, &|| {
        false
    })
    .expect("occ");
    println!(
        "total {} fetched {} requests {} truncated {}",
        f.total_matching,
        f.records.len(),
        f.requests,
        f.truncated
    );
    assert!(!f.records.is_empty());
}

#[test]
#[ignore = "network"]
fn chelsa_ocean_values() {
    let http = Http::new();
    let dir = std::env::temp_dir().join("niche-live-cache");
    let g = chelsa::native_grid(&http, &dir).expect("grid");
    // Bay of Biscay (open ocean) and a land point.
    for (lng, lat) in [(-6.0, 46.5), (-8.0, 45.0), (2.35, 48.85)] {
        let c0 = ((lng - g.west) / g.cell) as usize;
        let r0 = ((g.north - lat) / g.cell) as usize;
        let r = chelsa::read_window(&http, &dir, "chelsa_bio1", (r0, c0, 1, 1)).expect("window");
        println!("({lng},{lat}) bio1 = {}", r.data[0]);
    }
}

#[test]
#[ignore = "network"]
fn chelsa_bio12_points() {
    let http = Http::new();
    let dir = std::env::temp_dir().join("niche-live-cache");
    let g = chelsa::native_grid(&http, &dir).expect("grid");
    for (lng, lat) in [(-3.5, 48.0), (2.35, 48.85)] {
        let c0 = ((lng - g.west) / g.cell) as usize;
        let r0 = ((g.north - lat) / g.cell) as usize;
        let r = chelsa::read_window(&http, &dir, "chelsa_bio12", (r0, c0, 1, 1)).expect("window");
        println!("({lng},{lat}) bio12 = {:.4}", r.data[0]);
    }
}

/// A real (public) GBIF download fetched in 1 MiB range requests, resumed after an interruption,
/// is byte-identical to a plain download (ADR-0112: the region archive is tens of GB).
#[test]
#[ignore = "network"]
fn gbif_download_in_resumable_chunks() {
    let http = Http::new();
    let dir = std::env::temp_dir().join(format!("niche-live-resumable-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    // A 5.4 MB SIMPLE_CSV download (Emys orbicularis, 2026-10-14).
    let url = "https://api.gbif.org/v1/occurrence/download/request/0027314-260928105237408.zip";
    let whole = dir.join("whole.zip");
    http.download_to_file(url, &whole, 64 << 20)
        .expect("plain download");
    let chunked = dir.join("chunked.zip");
    // Interrupt after the second chunk, then resume.
    let n = std::sync::atomic::AtomicUsize::new(0);
    let stop = || n.load(std::sync::atomic::Ordering::Relaxed) >= 2;
    let first = http.download_resumable(url, &chunked, 1 << 20, &stop, &|_, _| {
        n.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });
    assert!(
        matches!(first, Err(niche_data::DataError::Cancelled)),
        "{first:?}"
    );
    assert!(
        chunked.with_extension("partial").is_file(),
        "partial file kept"
    );
    let size = http
        .download_resumable(url, &chunked, 1 << 20, &|| false, &|_, _| {})
        .expect("resumed");
    assert_eq!(
        std::fs::read(&whole).expect("a"),
        std::fs::read(&chunked).expect("b")
    );
    assert_eq!(size, std::fs::metadata(&whole).expect("m").len());
    let _ = std::fs::remove_dir_all(dir);
}

/// With a real GBIF archive (set NICHE_TEST_GBIF_ZIP to a SIMPLE_CSV download, e.g. the one
/// above), the regional mirror gives, for every taxon and box, exactly the records the
/// per-study download path reads from the same archive.
#[test]
#[ignore = "needs a real GBIF archive"]
fn mirror_equals_per_study_download_on_real_data() {
    use niche_data::gbif_download::{DownloadInfo, read_archive};
    use niche_data::gbif_mirror::{Snapshot, build, region};
    let Ok(zip) = std::env::var("NICHE_TEST_GBIF_ZIP") else {
        eprintln!("NICHE_TEST_GBIF_ZIP not set; skipped");
        return;
    };
    let zip = std::path::PathBuf::from(zip);
    let dir = std::env::temp_dir().join(format!("niche-live-mirror-{}", std::process::id()));
    let info = DownloadInfo {
        key: "test".into(),
        doi: None,
        total_records: 0,
        created: "2026-10-19T00:00:00Z".into(),
        url: String::new(),
    };
    let r = region("fr-es").expect("region");
    let meta = build(
        &zip,
        &r,
        info,
        "t",
        &dir.join("snap"),
        false,
        &|| false,
        &|_| {},
    )
    .expect("build");
    let snap = Snapshot::open(&dir.join("snap")).expect("open");
    // Effort grids: a real SIMPLE_CSV has the `class` column.
    let mut counted = 0u64;
    for c in snap.effort_classes() {
        let e = snap.effort_entry(&c).expect("entry");
        let cells = snap
            .effort(&c, (-180.0, -90.0, 180.0, 90.0))
            .expect("read")
            .expect("grid");
        assert_eq!(cells.len() as u64, e.cells);
        assert_eq!(cells.iter().map(|x| u64::from(x.2)).sum::<u64>(), e.records);
        println!("effort {c}: {} records in {} cells", e.records, e.cells);
        counted += e.records;
    }
    assert_eq!(counted, meta.effort_records);
    println!(
        "{} records with a class; effort.bin {} bytes",
        meta.effort_records,
        std::fs::metadata(dir.join("snap/effort.bin")).map_or(0, |m| m.len())
    );
    let mut keys: Vec<u64> = Vec::new();
    for t in 0..u64::from(u32::MAX) {
        if keys.len() >= meta.taxa as usize {
            break;
        }
        if snap.contains(t) {
            keys.push(t);
        }
        if t > 20_000_000 {
            break;
        }
    }
    assert!(!keys.is_empty());
    let boxes = [
        (-180.0, -90.0, 180.0, 90.0),
        (-5.5, 41.0, 9.8, 51.2),
        (-0.3, 43.3, 1.2, 44.1),
        (-9.5, 35.9, 4.5, 43.9),
    ];
    for &t in &keys {
        let per_study = read_archive(&zip, &[t]).expect("read");
        for b in boxes {
            let mut a = snap.records(t, b).expect("mirror");
            let mut want: Vec<_> = per_study
                .get(&t)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|o| o.lng >= b.0 && o.lng <= b.2 && o.lat >= b.1 && o.lat <= b.3)
                .collect();
            a.sort_by_key(|o| o.key);
            want.sort_by_key(|o| o.key);
            assert_eq!(a.len(), want.len(), "taxon {t} box {b:?}");
            assert!(a == want, "taxon {t} box {b:?}: records differ");
        }
        println!(
            "taxon {t}: {} records, identical in {} boxes",
            per_study.get(&t).map_or(0, Vec::len),
            boxes.len()
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// GBIF accepts the region's geometry (the same `within` WKT the mirror's download request
/// sends): a count over it answers, and matches the order of magnitude measured when the cells
/// were chosen (~327 M records, 2026-10-19).
#[test]
#[ignore = "network"]
fn gbif_accepts_the_region_geometry() {
    let r = niche_data::gbif_mirror::region("fr-es").expect("region");
    let wkt = r.wkt();
    println!(
        "{} polygons, {} characters",
        wkt.matches("((").count(),
        wkt.len()
    );
    let url = format!(
        "https://api.gbif.org/v1/occurrence/search?geometry={}&hasCoordinate=true&hasGeospatialIssue=false&occurrenceStatus=PRESENT&limit=0",
        wkt.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' =>
                    (b as char).to_string(),
                _ => format!("%{b:02X}"),
            })
            .collect::<String>()
    );
    let v = Http::new().get_json(&url).expect("GBIF answers");
    let n = v["count"].as_u64().expect("count");
    println!("{n} records in the region");
    assert!(n > 200_000_000, "{n}");
}

/// CHELSA CMIP6 future climatologies: their GeoTIFF headers give exactly the 1981–2010 grid
/// and no-data value (all 19 variables of one model/period/SSP, and BIO1 of every combination).
/// Scale/offset are printed (the reader applies each file's own).
#[test]
#[ignore = "network"]
fn chelsa_future_layers_share_the_baseline_grid() {
    use niche_geo::geotiff::GeoTiffInfo;
    let http = Http::new();
    let header = |u: String| {
        let src = niche_data::http::HttpSource::new(http.clone(), u.clone());
        GeoTiffInfo::read(&src).unwrap_or_else(|e| panic!("{u}: {e}"))
    };
    let mut checked = 0;
    let mut check = |id: &str, p: &str, g: &str, s: &str, base: &GeoTiffInfo| {
        let f = header(chelsa::future_url(id, p, g, s).expect("in catalogue"));
        assert_eq!(
            (f.width, f.height),
            (base.width, base.height),
            "{id} {p} {g} {s}"
        );
        assert_eq!(f.spec, base.spec, "{id} {p} {g} {s}: grid");
        // No-data values differ (future 0, baseline −99999, measured 2026-10-20): reported, see
        // docs/architecture/data-sources-v1.md.
        println!(
            "{id} {p} {g} {s}: scale {} offset {} nodata {:?} (baseline {} {} {:?})",
            f.scale, f.offset, f.nodata, base.scale, base.offset, base.nodata
        );
        checked += 1;
    };
    for n in 1..=19 {
        let id = format!("chelsa_bio{n}");
        let base = header(chelsa::url(&id).expect("url"));
        check(&id, "2041-2070", "MPI-ESM1-2-HR", "ssp370", &base);
    }
    let base = header(chelsa::url("chelsa_bio1").expect("url"));
    for p in chelsa::FUTURE_PERIODS {
        for g in chelsa::FUTURE_GCMS {
            for s in chelsa::FUTURE_SSPS {
                check("chelsa_bio1", p, g, s, &base);
            }
        }
    }
    println!("{checked} future headers checked");
}

/// What a raw 0 means in future files (declared no-data 0): values without the no-data mask at
/// dry land and ocean points, baseline vs 2041-2070 MPI-ESM1-2-HR ssp370.
#[test]
#[ignore = "network"]
fn chelsa_future_zero_values() {
    use niche_geo::geotiff::GeoTiffInfo;
    let http = Http::new();
    let dir = std::env::temp_dir().join("niche-live-cache");
    let g = chelsa::native_grid(&http, &dir).expect("grid");
    let points = [
        ("Bay of Biscay", -8.0, 45.0),
        ("Almeria", -2.4, 36.85),
        ("Fuerteventura", -14.0, 28.4),
        ("Algerian Sahara", 5.0, 25.0),
        ("Atacama", -69.9, -23.5),
        ("Paris", 2.35, 48.85),
    ];
    for id in [
        "chelsa_bio14",
        "chelsa_bio12",
        "chelsa_bio1",
        "chelsa_bio17",
    ] {
        for (url, label) in [
            (chelsa::url(id).expect("u"), "1981-2010"),
            (
                chelsa::future_url(id, "2041-2070", "MPI-ESM1-2-HR", "ssp370").expect("u"),
                "2041-2070",
            ),
        ] {
            let src = niche_data::http::HttpSource::new(http.clone(), url);
            let mut info = GeoTiffInfo::read(&src).expect("h");
            let nd = info.nodata;
            info.nodata = None;
            for (name, lng, lat) in points {
                let c0 = ((lng - g.west) / g.cell) as usize;
                let r0 = ((g.north - lat) / g.cell) as usize;
                let r = info.read_window(&src, r0, c0, 1, 1).expect("w");
                println!("{id} {label} (nodata {nd:?}) {name}: {}", r.data[0]);
            }
        }
    }
    // Through the public reader, a dry cell is 0 mm in both periods, not missing.
    let (lng, lat) = (-69.9, -23.5);
    let c0 = ((lng - g.west) / g.cell) as usize;
    let r0 = ((g.north - lat) / g.cell) as usize;
    let now = chelsa::read_window(&http, &dir, "chelsa_bio14", (r0, c0, 1, 1)).expect("now");
    let fut = chelsa::read_window_future(
        &http,
        &dir,
        "chelsa_bio14",
        "2041-2070",
        "MPI-ESM1-2-HR",
        "ssp370",
        (r0, c0, 1, 1),
    )
    .expect("future");
    println!("Atacama bio14: {} now, {} future", now.data[0], fut.data[0]);
    assert_eq!(now.data[0], 0.0);
    assert_eq!(fut.data[0], 0.0, "a raw 0 is data, not no-data");
}

/// A BIO1 window over Brittany is warmer in 2071–2100 under SSP5-8.5 than in 1981–2010 (sanity
/// check of units and scale/offset, not a scientific claim).
#[test]
#[ignore = "network"]
fn chelsa_future_window_brittany() {
    let http = Http::new();
    let dir = std::env::temp_dir().join("niche-live-cache");
    let g = chelsa::native_grid(&http, &dir).expect("grid");
    let c0 = ((-4.8 - g.west) / g.cell) as usize;
    let r0 = ((g.north - 48.9) / g.cell) as usize;
    let mean = |v: &[f32]| {
        let ok: Vec<f64> = v
            .iter()
            .filter(|x| !x.is_nan())
            .map(|&x| f64::from(x))
            .collect();
        ok.iter().sum::<f64>() / ok.len() as f64
    };
    let now = chelsa::read_window(&http, &dir, "chelsa_bio1", (r0, c0, 60, 120)).expect("now");
    let fut = chelsa::read_window_future(
        &http,
        &dir,
        "chelsa_bio1",
        "2071-2100",
        "GFDL-ESM4",
        "ssp585",
        (r0, c0, 60, 120),
    )
    .expect("future");
    assert_eq!(now.spec, fut.spec);
    let (a, b) = (mean(&now.data), mean(&fut.data));
    println!("bio1 mean 1981-2010 {a:.2} °C, 2071-2100 GFDL-ESM4 ssp585 {b:.2} °C");
    assert!(a > 5.0 && a < 20.0 && b > a, "{a} {b}");
}

/// The species API gives the class used to pick a sampling-effort grid; one request serves
/// rank and class.
#[test]
#[ignore = "network"]
fn gbif_taxon_class() {
    let http = Http::new();
    let hits = gbif::search_species(&http, "Columba palumbus").expect("search");
    let pigeon = hits.first().expect("hit").key;
    let hits = gbif::search_species(&http, "Quercus robur").expect("search");
    let oak = hits.first().expect("hit").key;
    for (key, want) in [
        (pigeon, Some("Aves")),
        (oak, Some("Magnoliopsida")),
        (1, None),
    ] {
        let info = gbif::taxon_info(&http, key).expect("info");
        let class = gbif::taxon_class(&http, key).expect("class");
        println!(
            "{key} {:?} {:?} class {class:?}",
            info.scientific_name, info.rank
        );
        assert_eq!(class.as_deref(), want, "{key}");
    }
    assert_eq!(gbif::taxon_rank(&http, 1).expect("rank"), "KINGDOM");
}

/// Search-API records carry the generalisation flag (Testudo hermanni in France: most records
/// are blurred by iNaturalist, the design notes §2).
#[test]
#[ignore = "network"]
fn gbif_generalised_flags_from_search_api() {
    let http = Http::new();
    let hits = gbif::search_species(&http, "Testudo hermanni").expect("search");
    let key = hits.first().expect("hit").key;
    let f =
        gbif::fetch_occurrences(&http, key, (5.5, 42.9, 7.0, 43.8), 300, &|| false).expect("occ");
    let g = f
        .records
        .iter()
        .filter(|o| o.generalised == Some(true))
        .count();
    let unknown = f.records.iter().filter(|o| o.generalised.is_none()).count();
    println!("{} records, {g} generalised", f.records.len());
    assert_eq!(unknown, 0);
    assert!(g > 0);
}
