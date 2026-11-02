//! Regional GBIF mirror in the engine (ADR-0114): which studies it serves, and its state for
//! administrators. Offline: the snapshot is built from a small archive. Equivalence of its
//! records with a per-study download is tested in niche-data.

use std::io::Write;
use std::path::{Path, PathBuf};

use niche_app::dto::StudyArea;
use niche_app::{Config, Engine, Limits, MirrorConfig};
use niche_data::gbif_download::DownloadInfo;
use niche_data::gbif_mirror::{MirrorDir, region};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("niche-app-mirror-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn engine(dir: &Path, mirror: bool, max_age_days: u64) -> Engine {
    Engine::open(Config {
        data_dir: dir.join("data"),
        cache_dir: dir.join("cache"),
        max_records: 5000,
        gbif: None,
        secret_key: None,
        mirror: mirror.then(|| MirrorConfig {
            region: region("fr-es").expect("region"),
            refresh_days: 7,
            max_age_days,
        }),
        limits: Limits::default(),
        retention_days: 0,
    })
    .expect("engine")
}

/// A snapshot of a two-record archive whose GBIF download was created `days_ago`.
fn install(dir: &Path, days_ago: i64) {
    let zip = dir.join("region.zip");
    let mut z = zip::ZipWriter::new(std::fs::File::create(&zip).expect("zip"));
    z.start_file("0001.csv", zip::write::SimpleFileOptions::default())
        .expect("entry");
    z.write_all(
        b"gbifID\tdatasetKey\tdecimalLatitude\tdecimalLongitude\tyear\ttaxonKey\tspeciesKey\tbasisOfRecord\tlicense\n\
          1\tds\t48.85\t2.35\t2015\t111\t111\tHUMAN_OBSERVATION\tCC_BY_4_0\n\
          2\tds\t40.4\t-3.7\t2019\t111\t111\tHUMAN_OBSERVATION\tCC0_1_0\n",
    )
    .expect("write");
    z.finish().expect("finish");
    let created = jiff::Timestamp::now()
        .checked_sub(jiff::SignedDuration::from_hours(24 * days_ago))
        .expect("time")
        .to_string();
    let info = DownloadInfo {
        key: format!("0000001-{days_ago}"),
        doi: Some("https://doi.org/10.15468/dl.region".into()),
        total_records: 2,
        created,
        url: "https://www.gbif.org/occurrence/download/0000001".into(),
    };
    let r = region("fr-es").expect("region");
    MirrorDir::new(&dir.join("data"), &r)
        .install(
            &zip,
            &r,
            info,
            "2026-10-19T00:00:00Z",
            true,
            &|| false,
            &|_| {},
        )
        .expect("install");
}

fn area(w: f64, s: f64, e: f64, n: f64) -> StudyArea {
    StudyArea {
        name: "a".into(),
        geometry: serde_json::json!({"type": "Polygon", "coordinates": [[[w, s], [e, s], [e, n], [w, n], [w, s]]]}),
        places: vec![],
    }
}

#[test]
fn studies_inside_the_region_use_a_fresh_mirror() {
    let dir = tmp("serves");
    install(&dir, 2);
    let e = engine(&dir, true, 30);
    assert!(e.mirror_serves(&area(2.2, 48.8, 2.5, 48.9)), "Paris");
    assert!(e.mirror_serves(&area(-3.8, 40.3, -3.6, 40.5)), "Madrid");
    assert!(!e.mirror_serves(&area(13.3, 52.4, 13.5, 52.6)), "Berlin");
    assert!(
        !e.mirror_serves(&area(2.2, 48.8, 9.0, 50.5)),
        "Paris to Frankfurt"
    );
    // Administrators see the snapshot; without the shared GBIF account it is not refreshed.
    let st = e
        .system_status()
        .expect("status")
        .gbif_mirror
        .expect("mirror status");
    assert_eq!(st.region, "fr-es");
    assert_eq!(st.state, "disabled");
    let snap = st.snapshot.expect("snapshot");
    assert_eq!((snap.records, snap.taxa, snap.usable), (2, 1, true));
    assert_eq!(
        snap.doi.as_deref(),
        Some("https://doi.org/10.15468/dl.region")
    );
    // The archive was removed once read.
    assert!(!dir.join("region.zip").exists());
}

#[test]
fn an_old_mirror_or_none_is_not_used() {
    let dir = tmp("old");
    install(&dir, 40);
    let e = engine(&dir, true, 30);
    assert!(
        !e.mirror_serves(&area(2.2, 48.8, 2.5, 48.9)),
        "older than 30 days"
    );
    let st = e
        .system_status()
        .expect("status")
        .gbif_mirror
        .expect("status");
    assert!(!st.snapshot.expect("snapshot").usable);
    // A server without a mirror configured ignores one left on disk.
    let e = engine(&dir, false, 30);
    assert!(!e.mirror_serves(&area(2.2, 48.8, 2.5, 48.9)));
    assert!(e.system_status().expect("status").gbif_mirror.is_none());
}

#[test]
fn a_newer_snapshot_replaces_the_current_one_and_the_previous_is_kept() {
    let dir = tmp("replace");
    install(&dir, 9);
    install(&dir, 1);
    install(&dir, 0);
    let root = dir.join("data/gbif-mirror/fr-es");
    let current = std::fs::read_to_string(root.join("current")).expect("current");
    assert_eq!(current.trim(), "0000001-0");
    assert!(
        root.join("0000001-1").is_dir(),
        "previous kept for studies still reading it"
    );
    assert!(!root.join("0000001-9").exists(), "older removed");
    let e = engine(&dir, true, 30);
    assert_eq!(
        e.system_status()
            .expect("s")
            .gbif_mirror
            .expect("m")
            .snapshot
            .expect("s")
            .download_key,
        "0000001-0"
    );
}
