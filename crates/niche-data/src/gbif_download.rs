//! GBIF occurrence downloads (asynchronous, authenticated) for whole batches of species.
//!
//! One request covers every species of a job: GBIF prepares a SIMPLE_CSV archive with all
//! matching records (no paging cap, no rate limit per page) and mints a DOI that must be cited.
//! This replaces hundreds of rate-limited search-API pages per species. Archives are cached on
//! disk (30 days) keyed by the request, so re-runs are instant and use identical data.
//!
//! SIMPLE_CSV has no `dataGeneralizations` / `informationWithheld` columns: records read here
//! have [`Occurrence::generalised`] = `None` (unknown), and the per-taxon split files do not
//! store the flag (their format is unchanged).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::DataError;
use crate::gbif::{Occurrence, OccurrenceFetch, licence_code};
use crate::http::Http;

const API: &str = "https://api.gbif.org/v1/occurrence/download";
const MAX_ZIP_BYTES: u64 = 8 << 30;
const MAX_WAIT: Duration = Duration::from_secs(3 * 3600);

#[derive(Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// Provenance of a batch download (goes into reports and the execution manifest).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadInfo {
    pub key: String,
    pub doi: Option<String>,
    pub total_records: u64,
    pub created: String,
    pub url: String,
}

fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

pub fn predicate(creator: &str, taxa: &[u64], bbox: (f64, f64, f64, f64)) -> serde_json::Value {
    let (w, s, e, n) = bbox;
    serde_json::json!({
        "creator": creator,
        "sendNotification": false,
        "format": "SIMPLE_CSV",
        "predicate": {"type": "and", "predicates": [
            {"type": "in", "key": "TAXON_KEY", "values": taxa.iter().map(u64::to_string).collect::<Vec<_>>()},
            {"type": "within", "geometry": format!("POLYGON(({w} {s},{e} {s},{e} {n},{w} {n},{w} {s}))")},
            {"type": "equals", "key": "HAS_COORDINATE", "value": "true"},
            {"type": "equals", "key": "HAS_GEOSPATIAL_ISSUE", "value": "false"},
            {"type": "equals", "key": "OCCURRENCE_STATUS", "value": "PRESENT"}
        ]}
    })
}

/// What a batch download is doing, for progress displays ([`Step::describe`] is the English
/// text kept in the job's stage and logs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step<'a> {
    /// A fresh archive of the same request is in the cache.
    Cached {
        key: &'a str,
    },
    /// Posting the download request.
    Requesting,
    /// GBIF is preparing the archive: its status (`PREPARING`, `RUNNING`…), checked just now.
    Waiting {
        key: &'a str,
        status: &'a str,
        elapsed: Duration,
        /// When GBIF created the download (RFC 3339): an identical request returns the same
        /// download, so after a restart the wait started earlier than this process's own clock.
        created: Option<&'a str>,
    },
    Downloading {
        key: &'a str,
        records: u64,
    },
    Splitting,
}

impl Step<'_> {
    pub fn describe(&self) -> String {
        match self {
            Self::Cached { key } => format!("Using cached GBIF download {key}"),
            Self::Requesting => "Requesting GBIF download for the whole batch".into(),
            Self::Waiting {
                status, elapsed, ..
            } => format!(
                "GBIF is preparing the download ({}; {} min elapsed)",
                status.to_lowercase(),
                elapsed.as_secs() / 60
            ),
            Self::Downloading { records, .. } => {
                format!("Downloading GBIF archive ({records} records)")
            }
            Self::Splitting => "Splitting GBIF archive per species".into(),
        }
    }
}

/// Request (or reuse from cache) a download for `taxa` in `bbox`, wait until GBIF has prepared
/// it, fetch the archive and split the records per requested taxon key.
#[allow(clippy::too_many_arguments)]
pub fn fetch_batch(
    http: &Http,
    cache_dir: &Path,
    creds: &Credentials,
    taxa: &[u64],
    bbox: (f64, f64, f64, f64),
    max_age_days: u64,
    cancelled: &(dyn Fn() -> bool + Sync),
    progress: &dyn Fn(&Step),
) -> Result<BatchData, DataError> {
    let mut sorted = taxa.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let body = predicate(&creds.username, &sorted, bbox);
    let id = fnv(&body["predicate"].to_string());
    let dir = cache_dir.join("gbif-downloads");
    std::fs::create_dir_all(&dir).map_err(|e| DataError::Cache(e.to_string()))?;
    prune(&dir, max_age_days);
    let meta_path = dir.join(format!("{id}.json"));
    let zip_path: PathBuf = dir.join(format!("{id}.zip"));
    let split_dir = dir.join(format!("{id}.split"));
    // The archive itself is deleted once split, so the cache entry is the metadata plus the
    // per-taxon files (or a not-yet-split archive after an interruption).
    let fresh = age(&meta_path).is_some_and(|a| a.as_secs() < max_age_days * 86_400)
        && (split_dir.join(SPLIT_DONE).is_file() || zip_path.is_file());
    let info = match std::fs::read_to_string(&meta_path)
        .ok()
        .and_then(|t| serde_json::from_str::<DownloadInfo>(&t).ok())
    {
        Some(info) if fresh => {
            progress(&Step::Cached { key: &info.key });
            info
        }
        _ => {
            // The key is kept from the moment GBIF assigns it, so a restart (deployment) waits
            // for the same download instead of queueing a new one at GBIF.
            let pending_path = dir.join(format!("{id}.pending"));
            let resumed = match pending_key(&pending_path) {
                Some(key) => match wait_ready(http, &key, cancelled, progress) {
                    Ok(info) => Some((key, info)),
                    // Failed, killed or erased at GBIF: request it again.
                    Err(DataError::Format { msg, .. }) => {
                        tracing::warn!(%key, %msg, "pending GBIF download unusable, requesting a new one");
                        None
                    }
                    Err(e) => return Err(e),
                },
                None => None,
            };
            let (key, info) = match resumed {
                Some(r) => r,
                None => {
                    progress(&Step::Requesting);
                    let key = http.post_json_basic(
                        &format!("{API}/request"),
                        &body.to_string(),
                        &creds.username,
                        &creds.password,
                    )?;
                    let key = key.trim().trim_matches('"').to_string();
                    write_atomic(&pending_path, key.as_bytes())?;
                    let info = wait_ready(http, &key, cancelled, progress)?;
                    (key, info)
                }
            };
            progress(&Step::Downloading {
                key: &key,
                records: info.total_records,
            });
            let link = format!("{API}/request/{key}.zip");
            http.download_to_file(&link, &zip_path, MAX_ZIP_BYTES)?;
            // A split of an older archive for the same request must not be reused.
            let _ = std::fs::remove_dir_all(&split_dir);
            let tmp = meta_path.with_extension("tmp");
            std::fs::write(
                &tmp,
                serde_json::to_string_pretty(&info).map_err(|e| DataError::Cache(e.to_string()))?,
            )
            .and_then(|()| std::fs::rename(&tmp, &meta_path))
            .map_err(|e| DataError::Cache(e.to_string()))?;
            let _ = std::fs::remove_file(&pending_path);
            info
        }
    };
    if cancelled() {
        return Err(DataError::Cancelled);
    }
    progress(&Step::Splitting);
    let counts = split_archive(&zip_path, &sorted, &split_dir)?;
    // The archive holds every column GBIF exports, observer names (recordedBy, identifiedBy)
    // included; the split keeps only the fields NicheRS uses. Keep nothing else.
    remove_file(&zip_path)?;
    Ok(BatchData {
        info,
        fresh,
        split_dir,
        counts,
        taxa: sorted,
    })
}

/// A prepared batch: records split into one file per taxon, loaded one species at a time so
/// memory is bounded by the largest species, not by the whole batch.
#[derive(Debug)]
pub struct BatchData {
    pub info: DownloadInfo,
    pub fresh: bool,
    split_dir: PathBuf,
    pub counts: HashMap<u64, u64>,
    /// Taxa included in the download request (a requested taxon with no records is a real zero).
    pub taxa: Vec<u64>,
}

impl BatchData {
    /// A batch already split into `<split_dir>/<taxon>.tsv` (regional mirror extracts).
    pub(crate) fn from_split(
        info: DownloadInfo,
        split_dir: PathBuf,
        counts: HashMap<u64, u64>,
        taxa: Vec<u64>,
    ) -> Self {
        Self {
            info,
            fresh: true,
            split_dir,
            counts,
            taxa,
        }
    }

    pub fn load(&self, taxon: u64) -> Result<OccurrenceFetch, DataError> {
        let path = self.split_dir.join(format!("{taxon}.tsv"));
        let cache = |e: String| DataError::Cache(format!("{}: {e}", path.display()));
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(cache(e.to_string())),
        };
        let mut records =
            Vec::with_capacity(self.counts.get(&taxon).copied().unwrap_or(0) as usize);
        for line in text.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 7 {
                return Err(cache("malformed line".into()));
            }
            records.push(Occurrence {
                key: f[0].parse().unwrap_or(0),
                lng: f[1].parse().map_err(|_| cache("bad lng".into()))?,
                lat: f[2].parse().map_err(|_| cache("bad lat".into()))?,
                year: f[3].parse().ok(),
                uncertainty_m: f[4].parse().ok(),
                licence: f[5].to_string(),
                basis_of_record: f[6].to_string(),
                // Splits made before the dataset column was kept have 7 fields.
                dataset_key: f.get(7).map(|d| (*d).to_string()).unwrap_or_default(),
                // Split files come from SIMPLE_CSV downloads or the mirror: not reported.
                generalised: None,
            });
        }
        Ok(OccurrenceFetch {
            accessed_at: self.info.created.clone(),
            from_cache: self.fresh,
            total_matching: records.len() as u64,
            truncated: false,
            query_url: self.info.url.clone(),
            requests: 1,
            doi: self.info.doi.clone(),
            records,
        })
    }
}

/// Marker written last in a split directory: per-taxon record counts.
const SPLIT_DONE: &str = "done.json";

/// A pending request older than this is not waited for again (GBIF prepares downloads in hours
/// at worst; an older key is more likely lost than queued).
const PENDING_MAX_AGE: Duration = Duration::from_secs(24 * 3600);

/// Key of a download requested before an interruption, if recent enough to wait for.
fn pending_key(path: &Path) -> Option<String> {
    let key = std::fs::read_to_string(path).ok()?.trim().to_string();
    (!key.is_empty() && age(path).is_some_and(|a| a < PENDING_MAX_AGE)).then_some(key)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), DataError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| DataError::Cache(e.to_string()))
}

fn age(path: &Path) -> Option<Duration> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .elapsed()
        .ok()
}

fn remove_file(path: &Path) -> Result<(), DataError> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(DataError::Cache(format!("{}: {e}", path.display())))
        }
        _ => Ok(()),
    }
}

/// Delete cache entries whose metadata is older than `max_age_days` (they would be downloaded
/// again anyway), and archives left behind by an interrupted split for more than a day.
/// Entries touched within the last day are kept, so a download in progress is never removed.
pub fn prune(dir: &Path, max_age_days: u64) {
    let day = Duration::from_secs(86_400);
    let stale = Duration::from_secs(max_age_days * 86_400);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        let (Some(stem), Some(ext)) = (
            path.file_stem().and_then(|s| s.to_str()),
            path.extension().and_then(|s| s.to_str()),
        ) else {
            continue;
        };
        let zip = dir.join(format!("{stem}.zip"));
        let split = dir.join(format!("{stem}.split"));
        let recent = |p: &Path| age(p).is_some_and(|a| a < day);
        match ext {
            "json"
                if age(&path).is_some_and(|a| a >= stale) && !recent(&zip) && !recent(&split) =>
            {
                let _ = std::fs::remove_dir_all(&split);
                let _ = std::fs::remove_file(&zip);
                let _ = std::fs::remove_file(&path);
            }
            "zip" if !recent(&path) && split.join(SPLIT_DONE).is_file() => {
                let _ = std::fs::remove_file(&path);
            }
            "pending" if pending_key(&path).is_none() => {
                let _ = std::fs::remove_file(&path);
            }
            _ => {}
        }
    }
}

/// One record of a split file: key, lng, lat, year, uncertainty, licence, basis, dataset.
/// [`Occurrence::generalised`] is not stored: splits only hold SIMPLE_CSV-derived records, for
/// which it is always unknown.
pub(crate) fn write_split_line(w: &mut impl std::io::Write, o: &Occurrence) -> std::io::Result<()> {
    let opt = |v: Option<String>| v.unwrap_or_default();
    writeln!(
        w,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        o.key,
        o.lng,
        o.lat,
        opt(o.year.map(|y| y.to_string())),
        opt(o.uncertainty_m.map(|u| u.to_string())),
        o.licence,
        o.basis_of_record,
        o.dataset_key
    )
}

/// Stream the archive into `<dir>/<taxon>.tsv` files (reused when `<dir>/done.json` exists).
fn split_archive(
    zip_path: &Path,
    taxa: &[u64],
    dir: &Path,
) -> Result<HashMap<u64, u64>, DataError> {
    use std::io::Write as _;
    let cache = |e: String| DataError::Cache(format!("{}: {e}", dir.display()));
    let done = dir.join(SPLIT_DONE);
    if let Some(c) = std::fs::read_to_string(&done)
        .ok()
        .and_then(|t| serde_json::from_str::<HashMap<u64, u64>>(&t).ok())
    {
        return Ok(c);
    }
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).map_err(|e| cache(e.to_string()))?;
    let mut writers: HashMap<u64, std::io::BufWriter<std::fs::File>> = HashMap::new();
    let mut counts: HashMap<u64, u64> = HashMap::new();
    let mut err: Option<std::io::Error> = None;
    for_each_record(zip_path, taxa, |taxon, o| {
        if err.is_some() {
            return;
        }
        let w = match writers.entry(taxon) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                match std::fs::File::create(dir.join(format!("{taxon}.tsv"))) {
                    Ok(f) => v.insert(std::io::BufWriter::new(f)),
                    Err(e) => {
                        err = Some(e);
                        return;
                    }
                }
            }
        };
        if let Err(e) = write_split_line(w, &o) {
            err = Some(e);
        }
        *counts.entry(taxon).or_insert(0) += 1;
    })?;
    if let Some(e) = err {
        return Err(cache(e.to_string()));
    }
    for (_, mut w) in writers {
        w.flush().map_err(|e| cache(e.to_string()))?;
    }
    std::fs::write(
        &done,
        serde_json::to_string(&counts).map_err(|e| cache(e.to_string()))?,
    )
    .map_err(|e| cache(e.to_string()))?;
    Ok(counts)
}

fn wait_ready(
    http: &Http,
    key: &str,
    cancelled: &(dyn Fn() -> bool + Sync),
    progress: &dyn Fn(&Step),
) -> Result<DownloadInfo, DataError> {
    wait_for(
        http,
        key,
        MAX_WAIT,
        Duration::from_secs(10),
        cancelled,
        progress,
    )
}

/// Poll a requested download every `poll` until GBIF has prepared it (or `max_wait` passes).
pub fn wait_for(
    http: &Http,
    key: &str,
    max_wait: Duration,
    poll: Duration,
    cancelled: &(dyn Fn() -> bool + Sync),
    progress: &dyn Fn(&Step),
) -> Result<DownloadInfo, DataError> {
    let t0 = Instant::now();
    let url = format!("{API}/{key}");
    loop {
        if cancelled() {
            return Err(DataError::Cancelled);
        }
        let v = http.get_json(&url)?;
        let status = v
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();
        match status.as_str() {
            "SUCCEEDED" => {
                return Ok(DownloadInfo {
                    key: key.to_string(),
                    doi: v
                        .get("doi")
                        .and_then(|d| d.as_str())
                        .map(|d| format!("https://doi.org/{d}")),
                    total_records: v
                        .get("totalRecords")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    created: v
                        .get("created")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    url: format!("https://www.gbif.org/occurrence/download/{key}"),
                });
            }
            "FAILED" | "KILLED" | "CANCELLED" | "FILE_ERASED" => {
                return Err(DataError::Format {
                    url,
                    msg: format!("GBIF download {key} ended with status {status}"),
                });
            }
            _ => {}
        }
        if t0.elapsed() > max_wait {
            return Err(DataError::Network {
                url,
                msg: format!(
                    "GBIF download not ready after {} h",
                    max_wait.as_secs() / 3600
                ),
            });
        }
        progress(&Step::Waiting {
            key,
            status: &status,
            elapsed: t0.elapsed(),
            created: v.get("created").and_then(|c| c.as_str()),
        });
        // Sleep in short steps so a shutdown is not held up by a long poll interval.
        let until = Instant::now() + poll;
        while Instant::now() < until {
            if cancelled() {
                return Err(DataError::Cancelled);
            }
            std::thread::sleep(Duration::from_millis(500).min(poll));
        }
    }
}

/// Parse the SIMPLE_CSV (tab-separated, unquoted) inside the archive into memory (tests, small
/// archives). Production paths stream with [`split_archive`].
pub fn read_archive(
    zip_path: &Path,
    taxa: &[u64],
) -> Result<HashMap<u64, Vec<Occurrence>>, DataError> {
    let mut out: HashMap<u64, Vec<Occurrence>> = HashMap::new();
    for_each_record(zip_path, taxa, |t, o| out.entry(t).or_default().push(o))?;
    Ok(out)
}

/// Stream every record of a requested taxon to `f(taxon, record)`.
fn for_each_record(
    zip_path: &Path,
    taxa: &[u64],
    mut f: impl FnMut(u64, Occurrence),
) -> Result<(), DataError> {
    let wanted: HashSet<u64> = taxa.iter().copied().collect();
    for_each_row(zip_path, |row| {
        let target = row
            .species_key
            .filter(|k| wanted.contains(k))
            .or_else(|| row.taxon_key.filter(|k| wanted.contains(k)));
        if let Some(t) = target {
            // Kept for citation (records per dataset): only for the requested taxa.
            let mut o = row.occurrence;
            o.dataset_key = row.dataset_key.unwrap_or_default().to_string();
            f(t, o);
        }
        true
    })
}

/// One record of a GBIF SIMPLE_CSV archive, with the keys it is matched by.
pub(crate) struct Row<'a> {
    pub occurrence: Occurrence,
    pub species_key: Option<u64>,
    pub taxon_key: Option<u64>,
    pub dataset_key: Option<&'a str>,
    /// Taxonomic class name (`class` column), if the archive has it and the record is classified.
    pub class: Option<&'a str>,
}

/// Stream every record with coordinates of the archive to `f`; `f` returns `false` to stop.
/// `occurrence.dataset_key` is left empty (one allocation saved per row); it is in `dataset_key`.
pub(crate) fn for_each_row(
    zip_path: &Path,
    mut f: impl FnMut(Row<'_>) -> bool,
) -> Result<(), DataError> {
    let cache = |e: String| DataError::Cache(format!("{}: {e}", zip_path.display()));
    let file = std::fs::File::open(zip_path).map_err(|e| cache(e.to_string()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| cache(e.to_string()))?;
    let idx = (0..zip.len())
        .find(|&i| {
            zip.by_index(i)
                .ok()
                .and_then(|f| f.name().ok().map(|n| n.ends_with(".csv")))
                .unwrap_or(false)
        })
        .ok_or_else(|| cache("no .csv in GBIF archive".into()))?;
    let entry = zip.by_index(idx).map_err(|e| cache(e.to_string()))?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .quoting(false)
        .flexible(true)
        .from_reader(entry);
    let headers = rdr.headers().map_err(|e| cache(e.to_string()))?.clone();
    let col = |name: &str| headers.iter().position(|h| h == name);
    let need = |name: &str| col(name).ok_or_else(|| cache(format!("missing column {name}")));
    let (c_id, c_lat, c_lng) = (
        need("gbifID")?,
        need("decimalLatitude")?,
        need("decimalLongitude")?,
    );
    let (c_species, c_taxon) = (col("speciesKey"), need("taxonKey")?);
    let (c_year, c_unc, c_lic, c_bor) = (
        col("year"),
        col("coordinateUncertaintyInMeters"),
        col("license"),
        col("basisOfRecord"),
    );
    let c_ds = col("datasetKey");
    let c_class = col("class");
    let mut rec = csv::StringRecord::new();
    while rdr
        .read_record(&mut rec)
        .map_err(|e| cache(e.to_string()))?
    {
        let get = |c: Option<usize>| c.and_then(|i| rec.get(i)).filter(|v| !v.is_empty());
        let parse_key = |c: Option<usize>| get(c).and_then(|v| v.parse::<u64>().ok());
        let (Some(lat), Some(lng)) = (
            get(Some(c_lat)).and_then(|v| v.parse().ok()),
            get(Some(c_lng)).and_then(|v| v.parse().ok()),
        ) else {
            continue;
        };
        let licence = licence_of(get(c_lic).unwrap_or_default()).to_string();
        let row = Row {
            species_key: parse_key(c_species),
            taxon_key: parse_key(Some(c_taxon)),
            dataset_key: get(c_ds),
            class: get(c_class),
            occurrence: Occurrence {
                key: get(Some(c_id)).and_then(|v| v.parse().ok()).unwrap_or(0),
                lng,
                lat,
                year: get(c_year).and_then(|v| v.parse().ok()),
                uncertainty_m: get(c_unc).and_then(|v| v.parse().ok()),
                licence,
                basis_of_record: get(c_bor).unwrap_or_default().to_string(),
                dataset_key: String::new(),
                generalised: None,
            },
        };
        if !f(row) {
            break;
        }
    }
    Ok(())
}

/// Licence of a raw archive value (URL or code): `CC0_1_0`, `CC_BY_4_0`, `CC_BY_NC_4_0`, `OTHER`.
pub(crate) fn licence_of(raw: &str) -> &'static str {
    if raw.contains('/') {
        licence_code(raw)
    } else {
        match raw {
            "CC0_1_0" => "CC0_1_0",
            "CC_BY_4_0" => "CC_BY_4_0",
            "CC_BY_NC_4_0" => "CC_BY_NC_4_0",
            _ => "OTHER",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn a_pending_request_is_reused_until_it_is_too_old() {
        let dir = std::env::temp_dir().join(format!("niche-gbif-pending-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let p = dir.join("abc.pending");
        assert_eq!(pending_key(&p), None);
        write_atomic(&p, b"0001234-261019120000000\n").expect("write");
        assert_eq!(pending_key(&p).as_deref(), Some("0001234-261019120000000"));
        prune(&dir, 30);
        assert!(p.is_file(), "a recent pending request survives pruning");
        let old = std::time::SystemTime::now() - PENDING_MAX_AGE - Duration::from_secs(60);
        std::fs::File::options()
            .write(true)
            .open(&p)
            .and_then(|f| f.set_modified(old))
            .expect("age");
        assert_eq!(pending_key(&p), None);
        prune(&dir, 30);
        assert!(!p.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn parses_simple_csv_archive_and_splits_by_taxon() {
        let dir = std::env::temp_dir().join(format!("niche-gbif-zip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("t.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&path).expect("file"));
        z.start_file("0001.csv", zip::write::SimpleFileOptions::default())
            .expect("entry");
        let rows = [
            "gbifID\tdatasetKey\tdecimalLatitude\tdecimalLongitude\tcoordinateUncertaintyInMeters\tyear\ttaxonKey\tspeciesKey\tbasisOfRecord\tlicense",
            "1\tds\t48.1\t-3.2\t10\t2015\t111\t111\tHUMAN_OBSERVATION\tCC_BY_4_0",
            "2\tds\t47.9\t-2.1\t\t2001\t999\t222\tPRESERVED_SPECIMEN\tCC0_1_0",
            "3\tds\t45.0\t1.0\t5\t2020\t333\t333\tHUMAN_OBSERVATION\tCC_BY_NC_4_0",
            "4\tds\t\t1.0\t5\t2020\t111\t111\tHUMAN_OBSERVATION\tCC0_1_0",
        ];
        z.write_all(rows.join("\n").as_bytes()).expect("write");
        z.finish().expect("finish");
        let m = read_archive(&path, &[111, 222]).expect("parse");
        assert_eq!(m[&111].len(), 1, "record without latitude is skipped");
        assert_eq!(m[&222].len(), 1, "matched through speciesKey");
        assert_eq!(m[&222][0].uncertainty_m, None);
        assert!(!m.contains_key(&333));
        assert_eq!(m[&111][0].licence, "CC_BY_4_0");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A cached download is split, its archive deleted, and the split alone stays a cache hit.
    #[test]
    fn archive_is_deleted_once_split() {
        let cache = std::env::temp_dir().join(format!("niche-gbif-del-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&cache);
        let dir = cache.join("gbif-downloads");
        std::fs::create_dir_all(&dir).expect("dir");
        let creds = Credentials {
            username: "u".into(),
            password: "p".into(),
        };
        let bbox = (-5.0, 47.0, -1.0, 49.0);
        let id = fnv(&predicate("u", &[111], bbox)["predicate"].to_string());
        let info = DownloadInfo {
            key: "k".into(),
            doi: Some("10.15468/dl.test".into()),
            total_records: 1,
            created: "2026-10-16".into(),
            url: "https://example.org".into(),
        };
        std::fs::write(
            dir.join(format!("{id}.json")),
            serde_json::to_string(&info).expect("json"),
        )
        .expect("meta");
        let mut z = zip::ZipWriter::new(
            std::fs::File::create(dir.join(format!("{id}.zip"))).expect("file"),
        );
        z.start_file("0001.csv", zip::write::SimpleFileOptions::default())
            .expect("entry");
        z.write_all(
            b"gbifID\tdecimalLatitude\tdecimalLongitude\tyear\ttaxonKey\tspeciesKey\tbasisOfRecord\tlicense\trecordedBy\n\
              1\t48.1\t-3.2\t2015\t111\t111\tHUMAN_OBSERVATION\tCC_BY_4_0\tJane Observer\n",
        )
        .expect("write");
        z.finish().expect("finish");
        let http = Http::new();
        let fetch = || fetch_batch(&http, &cache, &creds, &[111], bbox, 30, &|| false, &|_| {});
        let b = fetch().expect("batch from cache");
        assert!(b.fresh);
        assert!(!dir.join(format!("{id}.zip")).exists(), "archive deleted");
        assert_eq!(b.load(111).expect("load").records.len(), 1);
        let split = std::fs::read_to_string(dir.join(format!("{id}.split/111.tsv"))).expect("tsv");
        assert!(!split.contains("Jane"), "no observer name kept");
        // Second use: no archive, the split is still a valid cache entry (no download).
        let b = fetch().expect("batch from split");
        assert!(b.fresh);
        assert_eq!(b.counts[&111], 1);
        let _ = std::fs::remove_dir_all(cache);
    }

    #[test]
    fn prune_removes_stale_entries_and_split_archives_only() {
        let dir = std::env::temp_dir().join(format!("niche-gbif-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("old.split")).expect("dir");
        std::fs::create_dir_all(dir.join("new.split")).expect("dir");
        std::fs::create_dir_all(dir.join("left.split")).expect("dir");
        let past = std::time::SystemTime::now() - Duration::from_secs(40 * 86_400);
        let write = |name: &str, old: bool| {
            let p = dir.join(name);
            std::fs::write(&p, b"x").expect("write");
            if old {
                std::fs::File::options()
                    .write(true)
                    .open(&p)
                    .and_then(|f| f.set_modified(past))
                    .expect("mtime");
            }
        };
        // Stale entry: metadata 40 days old, split also old.
        write("old.json", true);
        write("old.split/done.json", true);
        let old_split = std::fs::File::open(dir.join("old.split")).expect("open");
        old_split.set_modified(past).expect("mtime");
        // Fresh entry with its archive still present but already split long ago.
        write("new.json", false);
        write("new.split/done.json", false);
        write("new.zip", true);
        // Archive left by an interrupted split (no done.json): kept for a later retry.
        write("left.json", false);
        write("left.zip", true);
        prune(&dir, 30);
        assert!(!dir.join("old.json").exists() && !dir.join("old.split").exists());
        assert!(dir.join("new.json").exists() && dir.join("new.split/done.json").exists());
        assert!(
            !dir.join("new.zip").exists(),
            "split archive with personal data removed"
        );
        assert!(dir.join("left.zip").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn credentials_are_redacted_in_debug() {
        let c = Credentials {
            username: "u".into(),
            password: "secret".into(),
        };
        assert!(!format!("{c:?}").contains("secret"));
    }
}
