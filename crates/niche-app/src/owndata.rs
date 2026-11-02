//! The organisation's own occurrence data (ADR-0108): import of a CSV parsed in the browser
//! (only the chosen columns are sent), normalisation and reprojection to WGS84, storage per
//! organisation, matching of taxon names to the GBIF backbone, and the merge with GBIF records
//! before the cleaning rules of ADR-0009.
//!
//! Confidentiality (the design notes): records never leave the server,
//! are never written to the logs (identifiers and counts only), are visible to their
//! organisation only, and are deleted with the dataset (and the copy of a study with the study).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use niche_data::gbif::{self, Occurrence};
use niche_geo::crs::Crs;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::AppError;
use crate::accounts::{Principal, Scope};
use crate::artifacts::{sha256, write_atomic};
use crate::dto::{
    CrsOption, DatasetInfo, ExclusionCount, ImportReport, ImportRequest, OccurrencePoint,
    OwnDataRole, RejectedRows, SourceCount, TaxonMatch, TaxonValue, Warning,
};
use crate::env::{Cleaned, EnvStack};
use crate::store::{Store, db, now};

/// Licence code of the organisation's own records: always accepted by the licence rule (the
/// organisation guarantees its rights to use them; ADR-0108).
pub const ORG_OWNED: &str = "ORG_OWNED";
/// Rows of one import (the 8 MB request limit is the practical bound, about 100 000 rows).
pub const MAX_ROWS: usize = 200_000;
/// Active datasets per organisation.
pub const MAX_DATASETS: usize = 20;
/// Names matched per request (each is one GBIF request).
pub const MAX_MATCH_NAMES: usize = 200;
const MAX_TEXT: usize = 200;
const FORMAT: &str = "nichers-own-records";
const FORMAT_VERSION: u32 = 1;

/// The closed list of coordinate systems of the import (ADR-0108), with the rectangle of
/// plausible coordinates in the system's units (outside: `outside_crs_domain`).
const CRS_LIST: &[(u32, &str, &str, [f64; 4])] = &[
    (
        4326,
        "WGS 84 longitude / latitude (degrees)",
        "GPS, GBIF, GeoNature columns *_4326",
        [-180.0, -90.0, 180.0, 90.0],
    ),
    (
        2154,
        "RGF93 v1 / Lambert-93 (metres)",
        "Metropolitan France: SINP, most GIS of French consultancies",
        [0.0, 6_000_000.0, 1_300_000.0, 7_200_000.0],
    ),
    (
        3035,
        "ETRS89 / LAEA Europe (metres)",
        "European grids (EEA, INSPIRE)",
        [1_000_000.0, 500_000.0, 8_000_000.0, 6_500_000.0],
    ),
    (
        25828,
        "ETRS89 / UTM zone 28N (metres)",
        "Canary Islands",
        UTM_DOMAIN,
    ),
    (
        25829,
        "ETRS89 / UTM zone 29N (metres)",
        "Galicia, Portugal",
        UTM_DOMAIN,
    ),
    (
        25830,
        "ETRS89 / UTM zone 30N (metres)",
        "Central Spain",
        UTM_DOMAIN,
    ),
    (
        25831,
        "ETRS89 / UTM zone 31N (metres)",
        "Catalonia, Balearic Islands, eastern France",
        UTM_DOMAIN,
    ),
    (
        32628,
        "WGS 84 / UTM zone 28N (metres)",
        "GPS units set to UTM: Canary Islands",
        UTM_DOMAIN,
    ),
    (
        32629,
        "WGS 84 / UTM zone 29N (metres)",
        "GPS units set to UTM",
        UTM_DOMAIN,
    ),
    (
        32630,
        "WGS 84 / UTM zone 30N (metres)",
        "GPS units set to UTM",
        UTM_DOMAIN,
    ),
    (
        32631,
        "WGS 84 / UTM zone 31N (metres)",
        "GPS units set to UTM",
        UTM_DOMAIN,
    ),
    (
        32632,
        "WGS 84 / UTM zone 32N (metres)",
        "GPS units set to UTM: Corsica, Alps",
        UTM_DOMAIN,
    ),
];
const UTM_DOMAIN: [f64; 4] = [100_000.0, 0.0, 900_000.0, 9_400_000.0];

/// Coordinate systems offered by the import.
pub fn crs_options() -> Vec<CrsOption> {
    CRS_LIST
        .iter()
        .map(|(code, name, usage, _)| CrsOption {
            code: format!("EPSG:{code}"),
            name: (*name).into(),
            usage: (*usage).into(),
        })
        .collect()
}

/// The transform of a code of the closed list (`EPSG:2154`) and its plausible rectangle.
pub fn parse_crs(code: &str) -> Result<(Crs, [f64; 4]), AppError> {
    let n: Option<u32> = code
        .trim()
        .strip_prefix("EPSG:")
        .and_then(|c| c.parse().ok());
    n.and_then(|n| {
        CRS_LIST
            .iter()
            .find(|(c, ..)| *c == n)
            .and_then(|(c, _, _, d)| Crs::from_epsg(*c).map(|crs| (crs, *d)))
    })
    .ok_or_else(|| {
        AppError::Invalid(format!(
            "coordinate system {code} is not offered; choose one of: {}",
            CRS_LIST
                .iter()
                .map(|(c, ..)| format!("EPSG:{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })
}

/// Text of the transform for the manifest.
pub fn transform_text(code: &str) -> String {
    match code {
        "EPSG:4326" => "none (WGS84 longitude/latitude)".into(),
        c => format!(
            "{c} → EPSG:4326 with niche-geo (IOGP GN 7-2 formulas, checked against PROJ 9.8.1); RGF93/ETRS89 taken as WGS84 (< 1 m, PROJ's ballpark transformation)"
        ),
    }
}

/// One normalised record of an imported dataset. The original coordinates are kept with the
/// WGS84 ones, so that the reprojection can be checked and replayed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnRecord {
    /// Line of the original file (1 = header).
    pub line: u32,
    pub taxon: String,
    pub x: f64,
    pub y: f64,
    pub lng: f64,
    pub lat: f64,
    pub year: Option<i32>,
    pub uncertainty_m: Option<f64>,
    /// Surveyed and not found (status column value in the absent list).
    pub absent: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub survey: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct RecordsFile {
    format: String,
    version: u32,
    crs: String,
    records: Vec<OwnRecord>,
}

/// Bytes of the normalised records file (`records.json`); its SHA-256 identifies the dataset's
/// content (`records_sha256`).
pub fn records_bytes(crs: &str, records: &[OwnRecord]) -> Result<Vec<u8>, AppError> {
    serde_json::to_vec(&RecordsFile {
        format: FORMAT.into(),
        version: FORMAT_VERSION,
        crs: crs.into(),
        records: records.to_vec(),
    })
    .map_err(|e| AppError::Internal(e.to_string()))
}

/// Records of a records file.
pub fn read_records(path: &Path) -> Result<Vec<OwnRecord>, AppError> {
    let bytes = std::fs::read(path)
        .map_err(|_| AppError::NotFound("the records of the organisation's dataset".into()))?;
    let f: RecordsFile =
        serde_json::from_slice(&bytes).map_err(|e| AppError::Storage(e.to_string()))?;
    if f.format != FORMAT || f.version != FORMAT_VERSION {
        return Err(AppError::Storage(format!(
            "unsupported own-records file {} v{}",
            f.format, f.version
        )));
    }
    Ok(f.records)
}

fn clean_text(s: &str) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    t.chars().take(MAX_TEXT).collect()
}

/// A number written by a person or a spreadsheet: `,` or `.` decimal, spaces (also no-break) as
/// thousands separators; with both `,` and `.`, the comma separates thousands.
pub fn parse_number(s: &str) -> Option<f64> {
    let t: String = s
        .trim()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}' && *c != '\u{202f}' && *c != '\'')
        .collect();
    if t.is_empty() {
        return None;
    }
    let t = if t.contains('.') {
        t.replace(',', "")
    } else {
        t.replace(',', ".")
    };
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// A date that is neither empty nor in one of the accepted forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnreadableDate;

/// Year of a date: `YYYY-MM-DD`, ISO timestamps, `YYYY-MM`, `YYYY`, `DD/MM/YYYY`, `DD-MM-YYYY`,
/// `DD.MM.YYYY` (and the same with one-digit days and months). `Err` when not empty and not
/// readable; `Ok(None)` when empty.
pub fn parse_year(s: &str) -> Result<Option<i32>, UnreadableDate> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let ok = |y: i32| (1600..=2200).contains(&y);
    let digits = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit());
    // Year first: 2024, 2024-05, 2024-05-17, 2024-05-17T08:00:00Z, 2024/05/17.
    let head: String = t.chars().take_while(char::is_ascii_digit).collect();
    if head.len() == 4 {
        let rest = &t[4..];
        if (rest.is_empty() || rest.starts_with(['-', '/', 'T', ' ']))
            && let Ok(y) = head.parse::<i32>()
            && ok(y)
        {
            return Ok(Some(y));
        }
        return Err(UnreadableDate);
    }
    // Day first: 17/05/2024, 17-05-2024, 17.05.2024, optionally followed by a time.
    let date = t.split([' ', 'T']).next().unwrap_or(t);
    let parts: Vec<&str> = date.split(['/', '-', '.']).collect();
    if parts.len() == 3
        && parts.iter().all(|p| digits(p))
        && parts[0].len() <= 2
        && parts[1].len() <= 2
        && parts[2].len() == 4
        && let Ok(y) = parts[2].parse::<i32>()
        && ok(y)
    {
        return Ok(Some(y));
    }
    Err(UnreadableDate)
}

/// Normalise an import: interpret the values, reproject to WGS84, reject unreadable rows (with
/// their file lines) and describe the result. Pure; called by the HTTP route and tests.
pub fn normalize(req: &ImportRequest) -> Result<(Vec<OwnRecord>, ImportReport), AppError> {
    let c = &req.columns;
    let n = c.taxon.len();
    if n == 0 {
        return Err(AppError::Invalid("the file has no data rows".into()));
    }
    if n > MAX_ROWS {
        return Err(AppError::Invalid(format!(
            "at most {MAX_ROWS} rows per import ({n} sent); split the file"
        )));
    }
    let same = |v: Option<&Vec<String>>| v.is_none_or(|v| v.len() == n);
    if c.x.len() != n
        || c.y.len() != n
        || !same(c.date.as_ref())
        || !same(c.uncertainty.as_ref())
        || !same(c.status.as_ref())
        || !same(c.survey.as_ref())
        || req.lines.as_ref().is_some_and(|l| l.len() != n)
    {
        return Err(AppError::Invalid(
            "every column of the import must have one value per row".into(),
        ));
    }
    let (crs, dom) = parse_crs(&req.crs)?;
    let absent: HashSet<String> = req
        .absent_values
        .iter()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect();
    let mut records = Vec::with_capacity(n);
    let mut rejected: BTreeMap<&'static str, (usize, Vec<u32>)> = BTreeMap::new();
    let mut reject = |reason: &'static str, line: u32| {
        let e = rejected.entry(reason).or_insert((0, Vec::new()));
        e.0 += 1;
        if e.1.len() < 10 {
            e.1.push(line);
        }
    };
    for i in 0..n {
        let line = req
            .lines
            .as_ref()
            .and_then(|l| l.get(i).copied())
            .unwrap_or(i as u32 + 2);
        let taxon = clean_text(&c.taxon[i]);
        if taxon.is_empty() {
            reject("missing_taxon", line);
            continue;
        }
        let (Some(x), Some(y)) = (parse_number(&c.x[i]), parse_number(&c.y[i])) else {
            reject("bad_coordinates", line);
            continue;
        };
        if !(dom[0]..=dom[2]).contains(&x) || !(dom[1]..=dom[3]).contains(&y) {
            reject("outside_crs_domain", line);
            continue;
        }
        let (lng, lat) = crs.to_wgs84(x, y);
        if !(lng.is_finite() && lat.is_finite() && (-90.0..=90.0).contains(&lat)) {
            reject("outside_crs_domain", line);
            continue;
        }
        let year = match c.date.as_ref().map(|d| parse_year(&d[i])) {
            None => None,
            Some(Ok(y)) => y,
            Some(Err(UnreadableDate)) => {
                reject("bad_date", line);
                continue;
            }
        };
        let uncertainty_m = match c.uncertainty.as_ref().map(|u| u[i].trim()) {
            None | Some("") => None,
            Some(u) => match parse_number(u).filter(|v| *v >= 0.0) {
                Some(v) => Some(v),
                None => {
                    reject("bad_uncertainty", line);
                    continue;
                }
            },
        };
        let is_absent = c
            .status
            .as_ref()
            .is_some_and(|s| absent.contains(&s[i].trim().to_lowercase()));
        let survey = c
            .survey
            .as_ref()
            .map(|s| clean_text(&s[i]))
            .filter(|s| !s.is_empty());
        records.push(OwnRecord {
            line,
            taxon,
            x,
            y,
            lng,
            lat,
            year,
            uncertainty_m,
            absent: is_absent,
            survey,
        });
    }
    let mut taxa: HashMap<&str, TaxonValue> = HashMap::new();
    for r in &records {
        let t = taxa.entry(&r.taxon).or_insert_with(|| TaxonValue {
            value: r.taxon.clone(),
            records: 0,
            presences: 0,
            absences: 0,
        });
        t.records += 1;
        if r.absent {
            t.absences += 1;
        } else {
            t.presences += 1;
        }
    }
    let mut taxa: Vec<TaxonValue> = taxa.into_values().collect();
    taxa.sort_by(|a, b| b.records.cmp(&a.records).then(a.value.cmp(&b.value)));
    let bounds_wgs84 = (!records.is_empty()).then(|| {
        records.iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, r| {
                [
                    b[0].min(r.lng),
                    b[1].min(r.lat),
                    b[2].max(r.lng),
                    b[3].max(r.lat),
                ]
            },
        )
    });
    let absences = records.iter().filter(|r| r.absent).count();
    let rejected_total: usize = rejected.values().map(|r| r.0).sum();
    let mut warnings = Vec::new();
    if req.synthetic {
        warnings.push(Warning::new(
            "synthetic",
            crate::dto::Severity::Info,
            "Synthetic dataset: generated records for tests and demonstrations, not field observations.",
        ));
    }
    if rejected_total * 10 > n {
        warnings.push(
            Warning::new(
                "many_rejected",
                crate::dto::Severity::Warning,
                format!("{rejected_total} of {n} rows were rejected: check the columns and the coordinate system chosen."),
            )
            .with("rejected", rejected_total)
            .with("rows", n),
        );
    }
    if let (Crs::Wgs84, Some(b)) = (crs, bounds_wgs84)
        && b[0] >= 27.0
        && b[2] <= 72.0
        && b[1] >= -32.0
        && b[3] <= 45.0
        && rejected.get("outside_crs_domain").is_none_or(|r| r.0 == 0)
    {
        // Every longitude looks like a European latitude and every latitude like a European
        // longitude: probably swapped columns.
        warnings.push(Warning::new(
            "axes_swapped",
            crate::dto::Severity::Warning,
            "Longitudes are all between 27 and 72 and latitudes between −32 and 45: the longitude and latitude columns may be swapped.",
        ));
    }
    if c.status.is_some() && absences == 0 {
        warnings.push(Warning::new(
            "no_absences",
            crate::dto::Severity::Info,
            "A status column was chosen but no row has one of the values marked as absent: every record is a presence.",
        ));
    }
    let report = ImportReport {
        rows_read: n,
        rows_imported: records.len(),
        rejected: rejected
            .into_iter()
            .map(|(reason, (count, lines))| RejectedRows {
                reason: reason.into(),
                count,
                lines,
            })
            .collect(),
        bounds_wgs84,
        presences: records.len() - absences,
        absences,
        with_survey: records.iter().filter(|r| r.survey.is_some()).count(),
        taxa,
        warnings,
    };
    Ok((records, report))
}

fn org_of(scope: &Scope) -> Option<&str> {
    match scope {
        Scope::All => None,
        Scope::Org(o) => Some(o.as_str()),
    }
}

/// Directory of a dataset's records.
pub fn dataset_dir(data_dir: &Path, id: &str) -> PathBuf {
    data_dir.join("datasets").join(id)
}

/// The copy of a dataset made for a study at submission (`jobs/<id>/input/`).
pub fn job_input_dir(data_dir: &Path, job: &str) -> PathBuf {
    data_dir.join("jobs").join(job).join("input")
}

/// Description of the dataset copied into a study (no records).
pub fn job_dataset_info(data_dir: &Path, job: &str) -> Result<DatasetInfo, AppError> {
    serde_json::from_slice(
        &std::fs::read(job_input_dir(data_dir, job).join("dataset.json"))
            .map_err(|_| AppError::NotFound("the study's copy of the dataset".into()))?,
    )
    .map_err(|e| AppError::Storage(e.to_string()))
}

impl Store {
    /// Datasets visible in `scope` (an organisation's own; local mode: those without
    /// organisation), newest first.
    pub fn dataset_list(&self, scope: &Scope) -> Result<Vec<DatasetInfo>, AppError> {
        let org = org_of(scope);
        self.with(|c| {
            let mut st = c
                .prepare("SELECT info FROM datasets WHERE org_id IS ?1 ORDER BY created_at DESC")
                .map_err(db)?;
            let rows = st
                .query_map([org], |r| r.get::<_, String>(0))
                .map_err(db)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db)?;
            rows.iter()
                .map(|t| serde_json::from_str(t).map_err(|e| AppError::Storage(e.to_string())))
                .collect()
        })
    }

    /// The dataset if `scope` may see it; another organisation's is "not found" (ADR-0105 §2).
    pub fn dataset_get(&self, id: &str, scope: &Scope) -> Result<DatasetInfo, AppError> {
        let org = org_of(scope);
        let info: Option<String> = self.with(|c| {
            c.query_row(
                "SELECT info FROM datasets WHERE id=?1 AND org_id IS ?2",
                params![id, org],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)
        })?;
        let info = info.ok_or_else(|| AppError::NotFound("dataset".into()))?;
        serde_json::from_str(&info).map_err(|e| AppError::Storage(e.to_string()))
    }

    fn dataset_insert(
        &self,
        info: &DatasetInfo,
        scope: &Scope,
        by: Option<&Principal>,
    ) -> Result<(), AppError> {
        let text = serde_json::to_string(info).map_err(|e| AppError::Internal(e.to_string()))?;
        let org = org_of(scope);
        self.with(|c| {
            let n: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM datasets WHERE org_id IS ?1",
                    [org],
                    |r| r.get(0),
                )
                .map_err(db)?;
            if n as usize >= MAX_DATASETS {
                return Err(AppError::Conflict(format!(
                    "at most {MAX_DATASETS} imported datasets per organisation; delete one first"
                )));
            }
            c.execute(
                "INSERT INTO datasets(id, org_id, created_by, created_at, info) VALUES(?1,?2,?3,?4,?5)",
                params![info.id, org, by.map(|p| p.user_id.as_str()), info.created_at, text],
            )
            .map_err(db)
            .map(|_| ())
        })
    }

    fn dataset_remove(&self, id: &str) -> Result<(), AppError> {
        self.with(|c| {
            c.execute("DELETE FROM datasets WHERE id=?1", [id])
                .map_err(db)
                .map(|_| ())
        })
    }

    /// Datasets created before `cutoff` (retention).
    fn datasets_before(&self, cutoff: &str) -> Result<Vec<String>, AppError> {
        self.with(|c| {
            let mut st = c
                .prepare("SELECT id FROM datasets WHERE created_at < ?1")
                .map_err(db)?;
            st.query_map([cutoff], |r| r.get(0))
                .map_err(db)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db)
        })
    }
}

impl crate::Engine {
    /// Import a dataset for the organisation of `scope`: normalise, store the records
    /// (`datasets/<id>/records.json`) and the description. Errors are 4xx and never quote a
    /// value of the file.
    pub fn import_dataset(
        &self,
        req: &ImportRequest,
        scope: &Scope,
        by: Option<&Principal>,
    ) -> Result<DatasetInfo, AppError> {
        let name = clean_text(&req.name);
        if name.is_empty() {
            return Err(AppError::Invalid("give the dataset a name".into()));
        }
        let sha_ok =
            req.file_sha256.len() == 64 && req.file_sha256.chars().all(|c| c.is_ascii_hexdigit());
        if !sha_ok {
            return Err(AppError::Invalid(
                "file_sha256 must be the SHA-256 of the file (64 hexadecimal characters)".into(),
            ));
        }
        let (records, report) = normalize(req)?;
        if records.is_empty() {
            return Err(AppError::Invalid(format!(
                "no row could be imported ({} rejected): check the columns and the coordinate system",
                report.rows_read
            )));
        }
        let crs = format!(
            "EPSG:{}",
            parse_crs(&req.crs)?
                .0
                .epsg()
                .ok_or_else(|| AppError::Internal("crs".into()))?
        );
        let bytes = records_bytes(&crs, &records)?;
        let id = uuid::Uuid::now_v7().to_string();
        let info = DatasetInfo {
            id: id.clone(),
            name,
            created_at: now(),
            file_name: clean_text(&req.file_name),
            file_sha256: req.file_sha256.to_lowercase(),
            records_sha256: sha256(&bytes),
            crs,
            absent_values: req.absent_values.iter().map(|v| clean_text(v)).collect(),
            mapping: req
                .mapping
                .iter()
                .map(|(k, v)| (clean_text(k), clean_text(v)))
                .collect(),
            synthetic: req.synthetic,
            report,
        };
        let dir = dataset_dir(&self.cfg.data_dir, &id);
        write_atomic(&dir.join("records.json"), &bytes)?;
        if let Err(e) = self.store().dataset_insert(&info, scope, by) {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(e);
        }
        tracing::info!(dataset_id = %id, rows = info.report.rows_read, imported = info.report.rows_imported, "dataset imported");
        Ok(info)
    }

    /// Delete a dataset (row and records). Studies already submitted keep their copy.
    pub fn delete_dataset(&self, id: &str, scope: &Scope) -> Result<(), AppError> {
        self.store().dataset_get(id, scope)?;
        self.remove_dataset(id)
    }

    fn remove_dataset(&self, id: &str) -> Result<(), AppError> {
        let dir = dataset_dir(&self.cfg.data_dir, id);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(AppError::Storage(format!("{}: {e}", dir.display()))),
        }
        self.store().dataset_remove(id)
    }

    /// Delete datasets older than the retention period, like finished studies.
    pub fn sweep_expired_datasets(&self) -> Result<Vec<String>, AppError> {
        if self.cfg.retention_days == 0 {
            return Ok(Vec::new());
        }
        let cutoff = jiff::Timestamp::now()
            .checked_sub(jiff::SignedDuration::from_hours(
                24 * self.cfg.retention_days.min(100_000) as i64,
            ))
            .map_err(|e| AppError::Internal(e.to_string()))?
            .to_string();
        let ids = self.store().datasets_before(&cutoff)?;
        for id in &ids {
            self.remove_dataset(id)?;
            tracing::info!(dataset_id = %id, "expired dataset deleted");
        }
        Ok(ids)
    }

    /// Records of a dataset of `scope` (data preview), checked against its checksum.
    pub fn dataset_records(
        &self,
        id: &str,
        scope: &Scope,
    ) -> Result<(DatasetInfo, Vec<OwnRecord>), AppError> {
        let info = self.store().dataset_get(id, scope)?;
        let path = dataset_dir(&self.cfg.data_dir, id).join("records.json");
        let bytes = std::fs::read(&path)
            .map_err(|_| AppError::NotFound("the records of the dataset".into()))?;
        if sha256(&bytes) != info.records_sha256 {
            return Err(AppError::Storage(format!(
                "records of dataset {id} do not match their checksum"
            )));
        }
        Ok((info, read_records(&path)?))
    }

    /// Copy a dataset into a study at submission (`jobs/<job>/input/`): the study then neither
    /// depends on the dataset surviving nor sees later changes. Returns the description.
    pub(crate) fn copy_dataset_to_job(
        &self,
        dataset: &crate::dto::DatasetRef,
        scope: &Scope,
        job: &str,
    ) -> Result<DatasetInfo, AppError> {
        let info = self.store().dataset_get(&dataset.id, scope)?;
        if info.records_sha256 != dataset.records_sha256 {
            return Err(AppError::Conflict(
                "the organisation's dataset changed since the study was prepared; choose it again"
                    .into(),
            ));
        }
        let bytes =
            std::fs::read(dataset_dir(&self.cfg.data_dir, &dataset.id).join("records.json"))
                .map_err(|_| AppError::NotFound("the records of the dataset".into()))?;
        if sha256(&bytes) != info.records_sha256 {
            return Err(AppError::Storage(format!(
                "records of dataset {} do not match their checksum",
                dataset.id
            )));
        }
        let dir = job_input_dir(&self.cfg.data_dir, job);
        write_atomic(&dir.join("own-records.json"), &bytes)?;
        let desc =
            serde_json::to_vec_pretty(&info).map_err(|e| AppError::Internal(e.to_string()))?;
        write_atomic(&dir.join("dataset.json"), &desc)?;
        Ok(info)
    }

    /// The copy of the organisation's dataset of a study (description and records).
    pub fn job_dataset(&self, job: &str) -> Result<(DatasetInfo, Vec<OwnRecord>), AppError> {
        let dir = job_input_dir(&self.cfg.data_dir, job);
        let info: DatasetInfo = serde_json::from_slice(
            &std::fs::read(dir.join("dataset.json"))
                .map_err(|_| AppError::NotFound("the study's copy of the dataset".into()))?,
        )
        .map_err(|e| AppError::Storage(e.to_string()))?;
        Ok((info, read_records(&dir.join("own-records.json"))?))
    }

    /// Match names to the GBIF backbone (only the names are sent). Automatic acceptance: exact
    /// match at species rank or below; the user confirms or chooses another taxon.
    pub fn match_names(&self, names: &[String]) -> Result<Vec<TaxonMatch>, AppError> {
        let mut seen = HashSet::new();
        let names: Vec<String> = names
            .iter()
            .map(|n| clean_text(n))
            .filter(|n| !n.is_empty() && seen.insert(n.clone()))
            .collect();
        if names.len() > MAX_MATCH_NAMES {
            return Err(AppError::Invalid(format!(
                "at most {MAX_MATCH_NAMES} names per request ({} sent)",
                names.len()
            )));
        }
        use rayon::prelude::*;
        let http = self.http();
        Ok(names
            .par_iter()
            .with_max_len(1)
            .map(|name| match gbif::match_name(http, name) {
                Ok(m) => TaxonMatch {
                    accepted: auto_accept(&m),
                    name: name.clone(),
                    key: m.key,
                    scientific_name: m.scientific_name,
                    rank: m.rank,
                    status: m.status,
                    match_type: m.match_type,
                    confidence: m.confidence,
                },
                Err(_) => TaxonMatch {
                    name: name.clone(),
                    key: None,
                    scientific_name: None,
                    rank: None,
                    status: None,
                    match_type: "ERROR".into(),
                    confidence: None,
                    accepted: false,
                },
            })
            .collect())
    }
}

/// Exact match at species rank or below (a higher rank would pool several niches, ADR-0009).
pub fn auto_accept(m: &gbif::NameMatch) -> bool {
    m.key.is_some()
        && m.match_type == "EXACT"
        && matches!(
            m.rank.as_deref(),
            Some("SPECIES" | "SUBSPECIES" | "VARIETY" | "FORM")
        )
}

/// The records of a species: those whose taxon value is one of `values`.
pub fn select<'a>(records: &'a [OwnRecord], values: &[String]) -> Vec<&'a OwnRecord> {
    let set: HashSet<&str> = values.iter().map(String::as_str).collect();
    records
        .iter()
        .filter(|r| set.contains(r.taxon.as_str()))
        .collect()
}

/// An own presence as an occurrence for the cleaning rules (ADR-0108): licence `ORG_OWNED`,
/// human observation.
pub fn to_occurrence(r: &OwnRecord, dataset_id: &str) -> Occurrence {
    Occurrence {
        key: u64::from(r.line),
        lng: r.lng,
        lat: r.lat,
        year: r.year,
        uncertainty_m: r.uncertainty_m,
        licence: ORG_OWNED.into(),
        basis_of_record: "HUMAN_OBSERVATION".into(),
        dataset_key: format!("own:{dataset_id}"),
        generalised: None,
    }
}

/// Training records of a species from both sources (ADR-0108): own presences first, then GBIF
/// records, so that a cell given by both is attributed to the organisation's data and the GBIF
/// record counts as `duplicate_cell`. Own absences of a training dataset are not presences:
/// rule `absence`, before every other rule.
pub struct Training {
    pub cleaned: Cleaned,
    /// Records passed to the rules, both sources (absences included).
    pub records: usize,
}

/// Exclusion reason of the absence rule.
pub const ABSENCE_REASON: &str = "Absence record (surveyed, not found): not used as a presence";

/// Clean the training records of a species with its own data. `own`: the species' records of
/// the dataset (training role) and the dataset id; `gbif_records`: empty when GBIF is not used.
pub fn clean_training(
    gbif_records: &[Occurrence],
    own: &[&OwnRecord],
    dataset_id: &str,
    stack: &EnvStack,
    filters: &crate::dto::OccurrenceFilters,
    seed: u64,
) -> Training {
    let presences: Vec<&&OwnRecord> = own.iter().filter(|r| !r.absent).collect();
    let absences: Vec<&&OwnRecord> = own.iter().filter(|r| r.absent).collect();
    let mut records: Vec<Occurrence> = presences
        .iter()
        .map(|r| to_occurrence(r, dataset_id))
        .collect();
    let n_own = records.len();
    records.extend(gbif_records.iter().cloned());
    let mut cleaned = crate::env::clean(&records, stack, filters, seed);
    for (i, p) in cleaned.points.iter_mut().enumerate() {
        p.source = Some(if i < n_own { "own" } else { "gbif" }.into());
    }
    if !absences.is_empty() {
        cleaned.exclusions.insert(
            0,
            ExclusionCount {
                reason: ABSENCE_REASON.into(),
                count: absences.len(),
            },
        );
        cleaned
            .points
            .extend(absences.iter().map(|r| OccurrencePoint {
                lng: r.lng,
                lat: r.lat,
                year: r.year,
                status: "absence".into(),
                source: Some("own".into()),
            }));
    }
    Training {
        records: records.len() + absences.len(),
        cleaned,
    }
}

/// Records per source from the points of a species (training and validation records), with the
/// GBIF records falling in a cell already given by the organisation's data. `gbif_used`: false
/// when the species uses no GBIF records (no `gbif` row).
pub fn source_counts(
    points: &[OccurrencePoint],
    stack: &EnvStack,
    gbif_used: bool,
) -> Vec<SourceCount> {
    let cell_of = |lng: f64, lat: f64| {
        stack
            .spec
            .cell_of(lng, lat)
            .map(|(r, c)| stack.spec.index(r, c))
    };
    let own_cells: HashSet<usize> = points
        .iter()
        .filter(|p| p.source.as_deref() == Some("own") && p.status == "retained")
        .filter_map(|p| cell_of(p.lng, p.lat))
        .collect();
    let mut out = Vec::new();
    for source in ["own", "gbif"] {
        let mut statuses: BTreeMap<String, usize> = BTreeMap::new();
        let (mut n, mut in_own) = (0, 0);
        for p in points
            .iter()
            .filter(|p| p.source.as_deref() == Some(source))
        {
            n += 1;
            *statuses.entry(p.status.clone()).or_insert(0) += 1;
            if source == "gbif"
                && p.status == "duplicate_cell"
                && cell_of(p.lng, p.lat).is_some_and(|c| own_cells.contains(&c))
            {
                in_own += 1;
            }
        }
        if (source == "gbif" && !gbif_used) || (source == "own" && n == 0) {
            continue;
        }
        out.push(SourceCount {
            source: source.into(),
            records: n,
            retained_cells: statuses.get("retained").copied().unwrap_or(0),
            statuses,
            in_own_cells: (source == "gbif" && !own_cells.is_empty()).then_some(in_own),
        });
    }
    out
}

/// The organisation's data of one species: how it is used, the dataset and its records.
pub struct OwnInput<'a> {
    pub use_: &'a crate::dto::OwnSpeciesData,
    pub dataset: &'a DatasetInfo,
    pub records: &'a [OwnRecord],
}

/// Occurrence data of a species ready for fitting: the cleaned training presences, the records
/// considered, the independent validation sites and the counts per source.
pub struct SpeciesData {
    pub cleaned: Cleaned,
    /// Records passed to the training rules (both sources; absences included).
    pub records: usize,
    pub sites: Option<crate::independent::Sites>,
    pub by_source: Vec<SourceCount>,
}

/// Training presences and validation sites of a species. Without the organisation's data this
/// is exactly [`crate::env::clean`] on the GBIF records (GBIF-only studies are unchanged).
pub fn species_data(
    gbif_records: &[Occurrence],
    own: Option<OwnInput<'_>>,
    gbif_used: bool,
    stack: &EnvStack,
    filters: &crate::dto::OccurrenceFilters,
    seed: u64,
) -> SpeciesData {
    let Some(own) = own else {
        return SpeciesData {
            cleaned: crate::env::clean(gbif_records, stack, filters, seed),
            records: gbif_records.len(),
            sites: None,
            by_source: Vec::new(),
        };
    };
    let selected = select(own.records, &own.use_.values);
    let id = own.dataset.id.as_str();
    let (training, sites) = match own.use_.role {
        OwnDataRole::Training => {
            let t = clean_training(gbif_records, &selected, id, stack, filters, seed);
            let absences: Vec<&OwnRecord> = selected.iter().filter(|r| r.absent).copied().collect();
            let sites = (!absences.is_empty()).then(|| {
                crate::independent::sites(&absences, stack, filters, &t.cleaned.presence_cells)
            });
            (t, sites)
        }
        OwnDataRole::Validation => {
            let mut t = clean_training(gbif_records, &[], id, stack, filters, seed);
            let s = crate::independent::sites(&selected, stack, filters, &t.cleaned.presence_cells);
            t.cleaned.points.extend(s.points.iter().cloned());
            (t, Some(s))
        }
    };
    let by_source = source_counts(&training.cleaned.points, stack, gbif_used);
    SpeciesData {
        records: training.records,
        cleaned: training.cleaned,
        sites,
        by_source,
    }
}

/// Training presence cells given by the organisation's data.
pub fn own_presence_cells(points: &[OccurrencePoint], stack: &EnvStack) -> Vec<usize> {
    points
        .iter()
        .filter(|p| p.source.as_deref() == Some("own") && p.status == "retained")
        .filter_map(|p| {
            stack
                .spec
                .cell_of(p.lng, p.lat)
                .map(|(r, c)| stack.spec.index(r, c))
        })
        .collect()
}

/// `own_data_concentrated` (Info, ADR-0108 §3.7; threshold proposed, to be confirmed by the
/// ecologists): the bounding box of the presence cells given by the organisation's data covers
/// less than 10 % of the study area's bounding box.
pub fn concentration_warning(stack: &EnvStack, own_cells: &[usize]) -> Option<Warning> {
    if own_cells.len() < 2 {
        return None;
    }
    let share = crate::independent::bbox_share(stack, own_cells)?;
    (share < 0.10).then(|| {
        Warning::new(
            "own_data_concentrated",
            crate::dto::Severity::Info,
            format!(
                "The organisation's presence cells span {:.0} % of the study area's extent: field data are often grouped (transects, study sites). Areas that were not surveyed are not absences.",
                share * 100.0
            ),
        )
        .with("share", share)
    })
}

/// Role label in reports.
pub fn role_text(role: OwnDataRole, gbif: bool) -> &'static str {
    match (role, gbif) {
        (OwnDataRole::Validation, _) => {
            "held out for independent validation (training on GBIF records only)"
        }
        (OwnDataRole::Training, true) => "presences used for training together with GBIF records",
        (OwnDataRole::Training, false) => "presences used for training (no GBIF records)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::ImportColumns;

    fn req(crs: &str, x: &[&str], y: &[&str]) -> ImportRequest {
        let n = x.len();
        ImportRequest {
            name: "Test".into(),
            file_name: "test.csv".into(),
            file_sha256: "0".repeat(64),
            crs: crs.into(),
            columns: ImportColumns {
                taxon: vec!["Testudo hermanni".into(); n],
                x: x.iter().map(|s| (*s).to_string()).collect(),
                y: y.iter().map(|s| (*s).to_string()).collect(),
                ..Default::default()
            },
            lines: None,
            absent_values: vec![],
            mapping: BTreeMap::new(),
            synthetic: true,
        }
    }

    #[test]
    fn numbers_and_years() {
        assert_eq!(parse_number("652469,0227"), Some(652469.0227));
        assert_eq!(parse_number(" 652 469.5 "), Some(652_469.5));
        assert_eq!(parse_number("652\u{a0}469,5"), Some(652_469.5));
        assert_eq!(parse_number("1,234.5"), Some(1234.5));
        assert_eq!(parse_number("-3,7"), Some(-3.7));
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("abc"), None);
        assert_eq!(parse_number("NaN"), None);
        assert_eq!(parse_year("2024-05-17"), Ok(Some(2024)));
        assert_eq!(parse_year("2024-05-17T08:30:00Z"), Ok(Some(2024)));
        assert_eq!(parse_year("2024"), Ok(Some(2024)));
        assert_eq!(parse_year("2024-05"), Ok(Some(2024)));
        assert_eq!(parse_year("17/05/2024"), Ok(Some(2024)));
        assert_eq!(parse_year("7/5/2024 10:00"), Ok(Some(2024)));
        assert_eq!(parse_year("17.05.2024"), Ok(Some(2024)));
        assert_eq!(parse_year("17-05-2024"), Ok(Some(2024)));
        assert_eq!(parse_year(""), Ok(None));
        assert_eq!(parse_year("mai 2024"), Err(UnreadableDate));
        assert_eq!(parse_year("05/17/24"), Err(UnreadableDate));
        assert_eq!(parse_year("0024"), Err(UnreadableDate));
    }

    /// Lambert-93 → WGS84 against PROJ 9.8.1 reference points (the same as niche-geo's tests):
    /// Paris, Brest, Corsica, Nice.
    #[test]
    fn lambert93_rows_are_reprojected() {
        let refs = [
            (652469.0227, 6862035.2594, 2.3522, 48.8566),
            (146632.9785, 6836262.3267, -4.4861, 48.3904),
            (1228546.0453, 6199420.6440, 9.45, 42.7),
            (1043310.2914, 6298259.4456, 7.26, 43.7),
        ];
        let xs: Vec<String> = refs
            .iter()
            .map(|r| format!("{}", r.0).replace('.', ","))
            .collect();
        let ys: Vec<String> = refs.iter().map(|r| format!("{}", r.1)).collect();
        let xr: Vec<&str> = xs.iter().map(String::as_str).collect();
        let yr: Vec<&str> = ys.iter().map(String::as_str).collect();
        let (recs, rep) = normalize(&req("EPSG:2154", &xr, &yr)).expect("normalise");
        assert_eq!(rep.rows_imported, 4);
        for (r, e) in recs.iter().zip(&refs) {
            // 1e-7° ≈ 1 cm.
            assert!(
                (r.lng - e.2).abs() < 1e-7 && (r.lat - e.3).abs() < 1e-7,
                "{r:?}"
            );
            assert_eq!((r.x, r.y), (e.0, e.1), "original coordinates kept");
        }
    }

    #[test]
    fn rows_are_rejected_with_their_lines() {
        let mut r = req(
            "EPSG:2154",
            &["652469", "abc", "2.35", "652469", "652469", "652469"],
            &[
                "6862035", "6862035", "48.85", "6862035", "6862035", "6862035",
            ],
        );
        r.columns.taxon[3] = "  ".into();
        r.columns.date = Some(vec![
            "2020".into(),
            "".into(),
            "".into(),
            "".into(),
            "hier".into(),
            "".into(),
        ]);
        r.columns.uncertainty = Some(vec![
            "10".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
            "-5".into(),
        ]);
        let (recs, rep) = normalize(&r).expect("normalise");
        assert_eq!((rep.rows_read, rep.rows_imported), (6, 1));
        let by: BTreeMap<&str, &RejectedRows> = rep
            .rejected
            .iter()
            .map(|x| (x.reason.as_str(), x))
            .collect();
        assert_eq!(by["bad_coordinates"].lines, vec![3]);
        assert_eq!(
            by["outside_crs_domain"].lines,
            vec![4],
            "degrees in Lambert-93"
        );
        assert_eq!(by["missing_taxon"].lines, vec![5]);
        assert_eq!(by["bad_date"].lines, vec![6]);
        assert_eq!(by["bad_uncertainty"].lines, vec![7]);
        assert_eq!(recs[0].year, Some(2020));
        assert_eq!(recs[0].uncertainty_m, Some(10.0));
        assert!(rep.warnings.iter().any(|w| w.code == "many_rejected"));
    }

    #[test]
    fn absences_taxa_and_limits() {
        let mut r = req(
            "EPSG:4326",
            &["2.35", "2.36", "2.37"],
            &["48.85", "48.86", "48.87"],
        );
        r.columns.taxon = vec!["Bufo bufo".into(), "Bufo  bufo".into(), "Rana".into()];
        r.columns.status = Some(vec!["Présent".into(), "Non observé".into(), "".into()]);
        r.absent_values = vec!["non observé".into()];
        let (recs, rep) = normalize(&r).expect("normalise");
        assert!(!recs[0].absent && recs[1].absent && !recs[2].absent);
        assert_eq!((rep.presences, rep.absences), (2, 1));
        assert_eq!(
            rep.taxa[0].value, "Bufo bufo",
            "spaces normalised, values merged"
        );
        assert_eq!((rep.taxa[0].records, rep.taxa[0].absences), (2, 1));
        // Column lengths must agree; unknown systems are refused.
        let mut bad = r.clone();
        bad.columns.y.pop();
        assert!(matches!(normalize(&bad), Err(AppError::Invalid(_))));
        let mut bad = r.clone();
        bad.crs = "EPSG:27572".into();
        assert!(matches!(normalize(&bad), Err(AppError::Invalid(m)) if m.contains("EPSG:2154")));
        // Swapped axes are flagged.
        let (_, rep) =
            normalize(&req("EPSG:4326", &["43.5", "44.0"], &["5.0", "5.5"])).expect("ok");
        assert!(rep.warnings.iter().any(|w| w.code == "axes_swapped"));
        assert!(crs_options().iter().any(|c| c.code == "EPSG:2154"));
    }

    #[test]
    fn records_file_round_trips() {
        let (recs, _) = normalize(&req("EPSG:4326", &["2.35"], &["48.85"])).expect("normalise");
        let b = records_bytes("EPSG:4326", &recs).expect("bytes");
        let dir = std::env::temp_dir().join(format!("niche-own-{}", std::process::id()));
        let p = dir.join("records.json");
        write_atomic(&p, &b).expect("write");
        assert_eq!(read_records(&p).expect("read"), recs);
        let _ = std::fs::remove_dir_all(dir);
    }
}
