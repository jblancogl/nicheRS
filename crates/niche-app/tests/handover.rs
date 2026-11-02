//! Deployments without downtime (ADR-0115): a second server process on the same data directory
//! serves requests while the first still runs a study. Offline (synthetic demo data).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use niche_app::dto::*;
use niche_app::lease::Lease;
use niche_app::{Config, Engine, Limits, demo};

fn cfg(dir: &std::path::Path) -> Config {
    Config {
        data_dir: dir.join("data"),
        cache_dir: dir.join("cache"),
        max_records: 5000,
        gbif: None,
        secret_key: None,
        mirror: None,
        limits: Limits::default(),
        retention_days: 0,
    }
}

fn spec() -> JobSpec {
    JobSpec {
        name: "handover".into(),
        species: (0..3)
            .map(|i| SpeciesRef {
                key: i,
                scientific_name: format!("{} #{i}", demo::SPECIES_NAME),
                own: None,
            })
            .collect(),
        area: StudyArea {
            name: "Demo".into(),
            geometry: demo::area_geojson(),
            places: vec![],
        },
        layers: demo::LAYERS.iter().map(|l| l.0.to_string()).collect(),
        resolution_factor: 1,
        filters: OccurrenceFilters::default(),
        model: ModelSettings::default(),
        validation: ValidationSettings::default(),
        seed: 3,
        demo: true,
        projections: vec![],
        own_dataset: None,
    }
}

fn wait_for(e: &Engine, id: &str, done: impl Fn(&Job) -> bool) -> Job {
    let t0 = Instant::now();
    loop {
        let j = e.store().get(id).expect("get").expect("job");
        if done(&j) {
            return j;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(120),
            "timed out in {:?}",
            j.status
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn a_study_running_in_one_process_is_cancelled_through_the_other() {
    let dir = std::env::temp_dir().join(format!("niche-handover-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let old = Arc::new(Engine::open(cfg(&dir)).expect("old process"));
    let lease = Lease::try_acquire(&dir.join("data"))
        .expect("io")
        .expect("old leads");
    let stop = Arc::new(AtomicBool::new(false));
    let (o, s) = (old.clone(), stop.clone());
    let worker = std::thread::spawn(move || o.worker_loop(&s));
    let (o2, s2) = (old.clone(), stop.clone());
    let preparer = std::thread::spawn(move || o2.prepare_loop(&s2));
    // The new process serves requests but may not lead while the old one holds the lease.
    let new = Engine::open(cfg(&dir)).expect("new process");
    assert!(Lease::try_acquire(&dir.join("data")).expect("io").is_none());
    let id = new
        .submit(spec())
        .expect("submitted through the new process")
        .id;
    wait_for(&new, &id, |j| j.status == JobStatus::Running);
    new.cancel(&id).expect("cancel through the new process");
    let j = wait_for(&new, &id, |j| j.status != JobStatus::Running);
    assert_eq!(j.status, JobStatus::Cancelled);
    stop.store(true, Ordering::Relaxed);
    worker.join().expect("worker");
    preparer.join().expect("preparer");
    drop(lease);
    // The old process gone, the new one takes the lease.
    assert!(Lease::try_acquire(&dir.join("data")).expect("io").is_some());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn concurrent_processes_record_events_without_duplicate_sequences() {
    let dir = std::env::temp_dir().join(format!("niche-events-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let first = Arc::new(Engine::open(cfg(&dir)).expect("first process"));
    let second = Arc::new(Engine::open(cfg(&dir)).expect("second process"));
    let id = first.submit(spec()).expect("submit").id;
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = [first.clone(), second]
        .into_iter()
        .map(|engine| {
            let id = id.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                for _ in 0..100 {
                    engine
                        .store()
                        .event(&id, "concurrent", "info", None)
                        .expect("event");
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("event writer");
    }
    let events = first.store().events_after(&id, 0).expect("events");
    assert_eq!(events.len(), 201);
    assert!(
        events
            .iter()
            .enumerate()
            .all(|(i, event)| event.seq == i as i64 + 1)
    );
    drop(first);
    let _ = std::fs::remove_dir_all(dir);
}
