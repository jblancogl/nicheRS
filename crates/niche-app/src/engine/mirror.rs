//! Regional GBIF mirror in the engine (ADR-0114): when a study may use it, extracting its
//! records, the background refresh and the state shown to administrators.
//!
//! A study uses the mirror only when all of this holds; otherwise it downloads from GBIF exactly
//! as before (no study ever fails because of the mirror):
//! - the server has a mirror and its snapshot is at most `max_age_days` old;
//! - the study is not the synthetic demo;
//! - its whole area lies inside the region (open sea allowed);
//! - each species is in the mirror's index, or GBIF says it is a species or lower taxon (then it
//!   simply has no record in the region). Higher taxa keep the per-species search.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use niche_data::gbif;
use niche_data::gbif_download::BatchData;
use niche_data::gbif_mirror::{MirrorDir, RefreshStep, Snapshot};
use serde::{Deserialize, Serialize};

use super::{Engine, is_single_taxon};
use crate::AppError;
use crate::dto::{JobSpec, MirrorSnapshot, MirrorStatus, StudyArea};
use crate::env;

/// Progress of the background refresh (administration).
#[derive(Debug, Clone, Default)]
pub(super) struct State {
    state: Option<&'static str>,
    detail: Option<String>,
    updated_at: Option<String>,
}

/// Where a study's records came from when the mirror served them (`jobs/<id>/gbif-mirror.json`):
/// the snapshot, and the records used per dataset (what a GBIF derived-dataset citation needs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MirrorSource {
    pub region: String,
    pub region_name: String,
    pub download_key: String,
    pub doi: Option<String>,
    /// When GBIF created the region's download: the data are as of this date.
    pub created: String,
    pub built_at: String,
    pub records: u64,
    pub datasets: BTreeMap<String, u64>,
}

/// Free disk needed to refresh, as a multiple of the current snapshot (archive ≈ 2×, unsorted
/// buckets ≈ 1×, the new snapshot ≈ 1×, the previous one kept), or this much without one.
const FIRST_REFRESH_FREE_BYTES: u64 = 60 << 30;

impl Engine {
    /// The snapshot a study in `area` may use now, if any.
    pub(super) fn mirror_for(&self, area: &StudyArea) -> Option<Arc<Snapshot>> {
        let cfg = self.cfg.mirror?;
        let snap = self.mirror.lock().ok()?.clone()?;
        (self.mirror_usable(&snap, cfg.max_age_days) && cfg.region.covers(&area.geometry))
            .then_some(snap)
    }

    /// Whether a study in this area would read its records from the regional mirror now.
    pub fn mirror_serves(&self, area: &StudyArea) -> bool {
        self.mirror_for(area).is_some()
    }

    /// Import a region archive downloaded by hand (`niche mirror import`): GBIF must know the
    /// download `key` (its DOI and date go into every study's citation).
    pub fn mirror_import(
        &self,
        zip: &std::path::Path,
        key: &str,
    ) -> Result<MirrorStatus, AppError> {
        let cfg = self.cfg.mirror.ok_or_else(|| {
            AppError::Invalid("no regional mirror configured (NICHE_GBIF_MIRROR)".into())
        })?;
        let info = niche_data::gbif_download::wait_for(
            &self.http,
            key,
            Duration::from_secs(0),
            Duration::from_secs(1),
            &|| false,
            &|_| {},
        )?;
        let dir = MirrorDir::new(&self.cfg.data_dir, &cfg.region);
        let last = std::cell::Cell::new(0u64);
        let snap = dir.install(
            zip,
            &cfg.region,
            info,
            &crate::store::now(),
            false,
            &|| false,
            &|n| {
                if n >= last.get() + 10_000_000 {
                    last.set(n);
                    tracing::info!(records = n, "indexing GBIF mirror");
                }
            },
        )?;
        if let Ok(mut m) = self.mirror.lock() {
            *m = Some(Arc::new(snap));
        }
        self.mirror_status()
            .ok_or_else(|| AppError::Internal("mirror status unavailable".into()))
    }

    fn mirror_usable(&self, snap: &Snapshot, max_age_days: u64) -> bool {
        age_days(&snap.meta.download.created)
            .or_else(|| age_days(&snap.meta.built_at))
            .is_some_and(|d| d <= max_age_days as f64)
    }

    /// Whether the mirror answers for this taxon: it has records of it, or GBIF says it is a
    /// species or lower taxon (no record in the region is then the true answer).
    fn mirror_taxon(&self, snap: &Snapshot, taxon: u64) -> bool {
        snap.contains(taxon)
            || gbif::taxon_rank(&self.http, taxon).is_ok_and(|r| is_single_taxon(&r))
    }

    fn mirror_extract_dir(&self, job_id: &str) -> std::path::PathBuf {
        self.cfg.cache_dir.join("gbif-mirror-extracts").join(job_id)
    }

    /// A study's records from the mirror, as the batch a per-study download would give.
    /// `None` when no species of the study is answered by the mirror.
    pub(super) fn mirror_batch(
        &self,
        id: &str,
        spec: &JobSpec,
        snap: &Snapshot,
    ) -> Result<Option<BatchData>, AppError> {
        let keys: Vec<u64> = spec
            .species
            .iter()
            .filter(|sp| {
                !crate::artifacts::species_dir(&self.cfg.data_dir, id, sp.key)
                    .join("result.json")
                    .exists()
            })
            .map(|sp| sp.key)
            .filter(|&k| self.mirror_taxon(snap, k))
            .collect();
        if keys.is_empty() {
            return Ok(None);
        }
        let bbox = env::area_window(
            &self.http,
            &self.cfg.cache_dir,
            &spec.area,
            spec.resolution_factor,
        )?
        .bbox();
        let (batch, datasets) = snap.batch(&keys, bbox, &self.mirror_extract_dir(id))?;
        let region_name = self.cfg.mirror.map_or("", |m| m.region.name).to_string();
        let source = MirrorSource {
            region: snap.meta.region.clone(),
            region_name: region_name.clone(),
            download_key: snap.meta.download.key.clone(),
            doi: snap.meta.download.doi.clone(),
            created: snap.meta.download.created.clone(),
            built_at: snap.meta.built_at.clone(),
            records: batch.info.total_records,
            datasets: datasets.into_iter().collect(),
        };
        let path = self
            .cfg
            .data_dir
            .join("jobs")
            .join(id)
            .join("gbif-mirror.json");
        crate::artifacts::write_atomic(
            &path,
            &serde_json::to_vec_pretty(&source).map_err(|e| AppError::Internal(e.to_string()))?,
        )?;
        self.store.set_job_gbif_account(id, "mirror")?;
        self.store.event(
            id,
            &format!(
                "Occurrence records from the regional GBIF mirror ({region_name}; GBIF download of {}): {} records for {} species, no wait at GBIF",
                source.created.get(..10).unwrap_or(&source.created),
                source.records,
                keys.len()
            ),
            "info",
            None,
        )?;
        Ok(Some(batch))
    }

    /// One species from the mirror for the per-species path (data preview, species left out of
    /// the batch): records, total, truncated, source, query URL. `None` → search GBIF.
    pub(super) fn mirror_fetch(
        &self,
        taxon: u64,
        area: &StudyArea,
        bbox: (f64, f64, f64, f64),
    ) -> Option<(Vec<gbif::Occurrence>, u64, bool, String, Option<String>)> {
        let snap = self.mirror_for(area)?;
        if !self.mirror_taxon(&snap, taxon) {
            return None;
        }
        let records = match snap.records(taxon, bbox) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "regional GBIF mirror unreadable; searching GBIF");
                return None;
            }
        };
        let d = &snap.meta.download;
        let n = records.len() as u64;
        let source = format!(
            "Regional GBIF mirror ({}): GBIF occurrence download {} ({}), created {}, filtered to this study",
            self.cfg.mirror.map_or("", |m| m.region.name),
            d.doi.as_deref().unwrap_or("without DOI"),
            d.url,
            d.created
        );
        Some((records, n, false, source, Some(d.url.clone())))
    }

    pub(super) fn mirror_source(&self, job_id: &str) -> Option<MirrorSource> {
        let p = self
            .cfg
            .data_dir
            .join("jobs")
            .join(job_id)
            .join("gbif-mirror.json");
        serde_json::from_slice(&std::fs::read(p).ok()?).ok()
    }

    pub(super) fn drop_mirror_extract(&self, job_id: &str) {
        let _ = std::fs::remove_dir_all(self.mirror_extract_dir(job_id));
    }

    pub(super) fn mirror_status(&self) -> Option<MirrorStatus> {
        let cfg = self.cfg.mirror?;
        let snap = self.mirror.lock().ok().and_then(|m| m.clone());
        let st = self
            .mirror_state
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        let snapshot = snap.as_ref().map(|s| MirrorSnapshot {
            download_key: s.meta.download.key.clone(),
            doi: s.meta.download.doi.clone(),
            created: s.meta.download.created.clone(),
            built_at: s.meta.built_at.clone(),
            records: s.meta.records,
            taxa: s.meta.taxa,
            bytes: s.meta.bytes,
            usable: self.mirror_usable(s, cfg.max_age_days),
        });
        let state = st.state.unwrap_or(if self.cfg.gbif.is_none() {
            "disabled"
        } else if snapshot.is_some() {
            "ready"
        } else {
            "requesting"
        });
        Some(MirrorStatus {
            region: cfg.region.id.to_string(),
            name: cfg.region.name.to_string(),
            state: state.to_string(),
            snapshot,
            detail: st.detail.or_else(|| {
                self.cfg.gbif.is_none().then(|| {
                    "the mirror needs the shared GBIF account (NICHE_GBIF_USERNAME)".to_string()
                })
            }),
            updated_at: st.updated_at,
            refresh_days: cfg.refresh_days,
            max_age_days: cfg.max_age_days,
        })
    }

    fn set_mirror_state(&self, state: &'static str, detail: Option<String>) {
        if let Ok(mut s) = self.mirror_state.lock() {
            s.state = Some(state);
            s.detail = detail;
            s.updated_at = Some(crate::store::now());
        }
    }

    /// Keep the regional mirror fresh (blocking; run on its own thread). Waits for GBIF and
    /// downloads in the background; studies keep using the previous snapshot, or download from
    /// GBIF, meanwhile. Errors are retried hourly.
    pub fn mirror_loop(&self, stop: &AtomicBool) {
        let Some(cfg) = self.cfg.mirror else { return };
        let Some(creds) = self.cfg.gbif.clone() else {
            tracing::warn!(
                "NICHE_GBIF_MIRROR is set but there is no shared GBIF account: no mirror"
            );
            return;
        };
        let dir = MirrorDir::new(&self.cfg.data_dir, &cfg.region);
        let sleep = |d: Duration| {
            let until = std::time::Instant::now() + d;
            while std::time::Instant::now() < until && !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_secs(1));
            }
        };
        while !stop.load(Ordering::Relaxed) {
            let current = self.mirror.lock().ok().and_then(|m| m.clone());
            let due = current.as_ref().is_none_or(|s| {
                age_days(&s.meta.download.created)
                    .or_else(|| age_days(&s.meta.built_at))
                    .is_none_or(|d| d >= cfg.refresh_days as f64)
            });
            if !due {
                if let Ok(mut s) = self.mirror_state.lock() {
                    s.state = None;
                }
                sleep(Duration::from_secs(600));
                continue;
            }
            let need = current
                .as_ref()
                .map_or(FIRST_REFRESH_FREE_BYTES, |s| s.meta.bytes.saturating_mul(4));
            let free = crate::system::disk_space(&self.cfg.data_dir).map(|d| d.available_bytes);
            // A download already partly on disk needs less; the check is for a fresh start.
            let resuming = dir
                .root
                .join("incoming")
                .read_dir()
                .is_ok_and(|mut d| d.next().is_some());
            if let Some(free) = free
                && free < need
                && !resuming
            {
                let msg = format!(
                    "not enough disk to refresh: {:.0} GB free, about {:.0} GB needed",
                    free as f64 / 1e9,
                    need as f64 / 1e9
                );
                tracing::warn!(%msg, "GBIF mirror refresh postponed");
                self.set_mirror_state("error", Some(msg));
                sleep(Duration::from_secs(3600));
                continue;
            }
            self.set_mirror_state("requesting", None);
            self.mirror_at_gbif.store(true, Ordering::Relaxed);
            let step = |s: &RefreshStep| match s {
                RefreshStep::Requesting => self.set_mirror_state("requesting", None),
                RefreshStep::Waiting { key, status } => {
                    self.set_mirror_state(
                        "waiting",
                        Some(format!("GBIF download {key}: {status}")),
                    );
                }
                RefreshStep::Downloading { key, bytes, total } => {
                    self.mirror_at_gbif.store(false, Ordering::Relaxed);
                    self.set_mirror_state(
                        "downloading",
                        Some(format!(
                            "GBIF download {key}: {:.1} of {:.1} GB",
                            *bytes as f64 / 1e9,
                            *total as f64 / 1e9
                        )),
                    );
                }
                RefreshStep::Building { key, records } => self.set_mirror_state(
                    "building",
                    Some(format!(
                        "GBIF download {key}: {} million records indexed",
                        records / 1_000_000
                    )),
                ),
            };
            let result = dir.refresh(
                &self.http,
                &creds,
                &cfg.region,
                &crate::store::now,
                &|| stop.load(Ordering::Relaxed),
                &step,
            );
            self.mirror_at_gbif.store(false, Ordering::Relaxed);
            match result {
                Ok(snap) => {
                    if let Ok(mut m) = self.mirror.lock() {
                        *m = Some(Arc::new(snap));
                    }
                    if let Ok(mut s) = self.mirror_state.lock() {
                        *s = State::default();
                    }
                    // Extracts left by studies interrupted for good (normally removed at the end).
                    if let Ok(rd) =
                        std::fs::read_dir(self.cfg.cache_dir.join("gbif-mirror-extracts"))
                    {
                        for e in rd.flatten() {
                            let old = e
                                .metadata()
                                .and_then(|m| m.modified())
                                .ok()
                                .and_then(|t| t.elapsed().ok())
                                .is_some_and(|a| a > Duration::from_secs(2 * 86_400));
                            if old {
                                let _ = std::fs::remove_dir_all(e.path());
                            }
                        }
                    }
                }
                Err(niche_data::DataError::Cancelled) => break,
                Err(e) => {
                    tracing::warn!(error = %e, "GBIF mirror refresh failed; retrying in an hour");
                    self.set_mirror_state("error", Some(e.to_string()));
                    sleep(Duration::from_secs(3600));
                }
            }
        }
    }
}

/// Days since an RFC 3339 time (GBIF's `created` or our `built_at`).
fn age_days(ts: &str) -> Option<f64> {
    let t: jiff::Timestamp = ts.parse().ok()?;
    let secs = jiff::Timestamp::now().duration_since(t).as_secs_f64();
    Some(secs / 86_400.0)
}
