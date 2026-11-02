//! Regional mirror of GBIF occurrences (ADR-0114): one GBIF download covering a whole region
//! (France and Spain), refreshed weekly in the background and stored on disk indexed by taxon,
//! so a study inside the region gets its records in seconds instead of waiting in GBIF's
//! download queue (observed 2–23 min, `docs/benchmarks/gbif-batch-downloads.md`).
//!
//! Equivalence with a per-study download: the mirror download uses the same predicate as
//! [`crate::gbif_download::predicate`] (coordinates, no geospatial issue, present) with the
//! region's geometry instead of the study's box and no taxon filter, and records are indexed by
//! `speciesKey` and `taxonKey` exactly as [`crate::gbif_download`] matches them. A study is served
//! only when its whole area lies inside the region's cells, so every record inside the study area
//! is in the mirror; records of the study box outside the region (other countries, used by no
//! model) are the only difference.
//!
//! Storage: `<dir>/meta.json`, `<dir>/index.bin` (taxon → bucket range) and 256 bucket files of
//! fixed-size records sorted by taxon. Building is memory-bounded: a first pass appends records
//! to 256 unsorted buckets, a second pass sorts one bucket at a time.
//!
//! Format 2 adds sampling-effort grids (target-group background, Phillips et al. 2009,
//! Ecological Applications 19:181): for every record with coordinates and a taxonomic class
//! (including records not identified to species), the number of records per (class,
//! 30-arc-second cell) — `effort.bin` and `effort-index.json`, see [`Snapshot::effort`].
//! They are built in the same memory-bounded way: the first pass appends (class, cell) pairs to
//! files by band of 20 grid rows, a third pass sorts and counts one band at a time.
//! Format-1 snapshots still open; they have no effort grids.

use std::collections::HashMap;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::DataError;
use crate::gbif::Occurrence;
use crate::gbif_download::{BatchData, DownloadInfo};

/// Bump when the on-disk layout changes. Snapshots older than [`MIN_FORMAT`] do not open (the
/// next refresh rebuilds them); format 1 still opens and is replaced at the next weekly refresh.
/// 2: sampling-effort grids by class (`effort.bin`, `effort-index.json`).
pub const FORMAT: u32 = 2;
/// Oldest format [`Snapshot::open`] still reads (format 1: no effort grids, studies unaffected).
const MIN_FORMAT: u32 = 1;
const BUCKETS: usize = 256;
/// gbifID u64, lng f64, lat f64, uncertainty f64 (NaN = none), year i32 (MIN = none),
/// dataset u32 (MAX = none), licence u8, basis u8.
const REC: usize = 42;
/// Raw (unsorted) bucket record: taxon key u32 + record.
const RAW: usize = 4 + REC;
const LICENCES: [&str; 4] = ["CC0_1_0", "CC_BY_4_0", "CC_BY_NC_4_0", "OTHER"];

/// Effort grid: global 30-arc-second cells aligned to (−180°, +90°).
pub const EFFORT_CELLS_PER_DEGREE: u32 = 120;
const EFFORT_COLS: u32 = 360 * EFFORT_CELLS_PER_DEGREE;
const EFFORT_ROWS: u32 = 180 * EFFORT_CELLS_PER_DEGREE;
/// Grid rows per temporary band file while building (1/6°; the fr-es region touches ~190).
const EFFORT_BAND_ROWS: u32 = 20;
const EFFORT_BANDS: usize = (EFFORT_ROWS / EFFORT_BAND_ROWS) as usize;
/// Temporary pair: class index u16 + cell u32.
const EFFORT_PAIR: usize = 6;
/// Stored run: cell u32 + count u32.
const EFFORT_RUN: usize = 8;

/// Effort-grid cell of a point: `row · 43200 + col` with `col = ⌊(lng + 180) · 120⌋`,
/// `row = ⌊(90 − lat) · 120⌋` (a point on the east or south edge of the globe goes to the last
/// cell). `None` outside [−180, 180] × [−90, 90] or for non-finite coordinates.
pub fn effort_cell(lng: f64, lat: f64) -> Option<u32> {
    // NaN and infinities are outside both ranges.
    if !((-180.0..=180.0).contains(&lng) && (-90.0..=90.0).contains(&lat)) {
        return None;
    }
    let k = f64::from(EFFORT_CELLS_PER_DEGREE);
    let col = (((lng + 180.0) * k).floor() as u32).min(EFFORT_COLS - 1);
    let row = (((90.0 - lat) * k).floor() as u32).min(EFFORT_ROWS - 1);
    Some(row * EFFORT_COLS + col)
}

/// Centre (lng, lat) of an effort-grid cell.
pub fn effort_cell_centre(cell: u32) -> (f64, f64) {
    let k = f64::from(EFFORT_CELLS_PER_DEGREE);
    let (row, col) = (cell / EFFORT_COLS, cell % EFFORT_COLS);
    (
        -180.0 + (f64::from(col) + 0.5) / k,
        90.0 - (f64::from(row) + 0.5) / k,
    )
}

/// One class of `effort-index.json`: its runs are `effort.bin[offset .. offset + cells]`
/// (8-byte runs: cell u32, count u32, little-endian, sorted by cell).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffortEntry {
    pub class: String,
    /// First run (in runs, not bytes).
    pub offset: u64,
    /// Cells with at least one record.
    pub cells: u64,
    /// Records counted (sum of the counts).
    pub records: u64,
}

/// A region served by a mirror: a set of `step`-degree cells (column, row) covering its land
/// with a margin of about 10 km.
#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub id: &'static str,
    pub name: &'static str,
    pub step: f64,
    /// Sorted (column, row) cell indices: cell (i, j) spans [i·step, (i+1)·step) × [j·step, …).
    /// The mirror's download covers exactly these cells.
    pub cells: &'static [(i16, i16)],
    /// Sorted open-sea cells around the region (no land of any country). A study area may cross
    /// them: records at sea are excluded by cleaning, so no usable record is missing.
    pub sea: &'static [(i16, i16)],
    /// The region cells as GBIF WKT: their union as valid polygons, exterior rings
    /// counter-clockwise (GBIF reads a clockwise ring as its complement; cells sharing edges are
    /// not a valid MULTIPOLYGON). Generated with the cells.
    pub wkt: &'static str,
}

/// Regions a server can mirror (`NICHE_GBIF_MIRROR`).
pub const REGIONS: &[Region] = &[crate::gbif_mirror_cells::FR_ES];

pub fn region(id: &str) -> Option<Region> {
    REGIONS.iter().copied().find(|r| r.id == id)
}

impl Region {
    /// The cell is in the mirror, or open sea.
    fn has(&self, i: i64, j: i64) -> bool {
        let (Ok(i), Ok(j)) = (i16::try_from(i), i16::try_from(j)) else {
            return false;
        };
        self.cells.binary_search(&(i, j)).is_ok() || self.sea.binary_search(&(i, j)).is_ok()
    }

    fn cell_of(&self, x: f64, y: f64) -> (i64, i64) {
        (
            (x / self.step).floor() as i64,
            (y / self.step).floor() as i64,
        )
    }

    /// A point of an outline is covered when its cell is; on a grid line (an area ending exactly
    /// at 49° N), when any of the cells it borders is.
    fn has_point(&self, x: f64, y: f64) -> bool {
        let (i, j) = self.cell_of(x, y);
        let on = |v: f64| (v / self.step).fract() == 0.0;
        let is: &[i64] = if on(x) { &[i, i - 1] } else { &[i] };
        let js: &[i64] = if on(y) { &[j, j - 1] } else { &[j] };
        is.iter().any(|&a| js.iter().any(|&b| self.has(a, b)))
    }

    /// Whether a study area (GeoJSON Polygon or MultiPolygon, WGS84) lies entirely inside the
    /// region: every cell its outline crosses, and every cell whose centre it contains, is a
    /// region cell or open sea. Unknown geometry types are not covered (the study then downloads as before).
    pub fn covers(&self, geometry: &serde_json::Value) -> bool {
        let Some(polygons) = polygons(geometry) else {
            return false;
        };
        if polygons.is_empty() {
            return false;
        }
        for rings in &polygons {
            for ring in rings {
                for w in ring.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    // Densify each edge to a quarter of a cell so no crossed cell is skipped.
                    let len = ((b.0 - a.0).abs()).max((b.1 - a.1).abs());
                    let n = (len / (self.step / 4.0)).ceil().max(1.0) as usize;
                    for k in 0..=n {
                        let t = k as f64 / n as f64;
                        if !self.has_point(a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)) {
                            return false;
                        }
                    }
                }
            }
            // Interior: cells whose centre is inside the polygon (holes excluded).
            let Some((w, s, e, n)) = bounds(rings) else {
                return false;
            };
            let (i0, j0) = self.cell_of(w, s);
            let (i1, j1) = self.cell_of(e, n);
            for i in i0..=i1 {
                for j in j0..=j1 {
                    let c = ((i as f64 + 0.5) * self.step, (j as f64 + 0.5) * self.step);
                    if inside(rings, c) && !self.has(i, j) {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn wkt(&self) -> &'static str {
        self.wkt
    }

    /// The download request: [`crate::gbif_download::predicate`] without the taxon filter, over
    /// the region instead of a study box.
    pub fn predicate(&self, creator: &str) -> serde_json::Value {
        serde_json::json!({
            "creator": creator,
            "sendNotification": false,
            "format": "SIMPLE_CSV",
            "predicate": {"type": "and", "predicates": [
                {"type": "within", "geometry": self.wkt()},
                {"type": "equals", "key": "HAS_COORDINATE", "value": "true"},
                {"type": "equals", "key": "HAS_GEOSPATIAL_ISSUE", "value": "false"},
                {"type": "equals", "key": "OCCURRENCE_STATUS", "value": "PRESENT"}
            ]}
        })
    }
}

type Ring = Vec<(f64, f64)>;

fn polygons(g: &serde_json::Value) -> Option<Vec<Vec<Ring>>> {
    let ring = |r: &serde_json::Value| -> Option<Ring> {
        r.as_array()?
            .iter()
            .map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?)))
            .collect()
    };
    let poly =
        |p: &serde_json::Value| -> Option<Vec<Ring>> { p.as_array()?.iter().map(ring).collect() };
    let coords = g.get("coordinates")?;
    match g.get("type")?.as_str()? {
        "Polygon" => Some(vec![poly(coords)?]),
        "MultiPolygon" => coords.as_array()?.iter().map(poly).collect(),
        _ => None,
    }
}

fn bounds(rings: &[Ring]) -> Option<(f64, f64, f64, f64)> {
    let pts = rings.first()?;
    let (mut w, mut s, mut e, mut n) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in pts {
        w = w.min(x);
        e = e.max(x);
        s = s.min(y);
        n = n.max(y);
    }
    (w <= e && s <= n).then_some((w, s, e, n))
}

/// Even-odd point-in-polygon over the outer ring and its holes.
fn inside(rings: &[Ring], (x, y): (f64, f64)) -> bool {
    let mut c = false;
    for r in rings {
        for w in r.windows(2) {
            let ((x1, y1), (x2, y2)) = (w[0], w[1]);
            if (y1 > y) != (y2 > y) && x < (x2 - x1) * (y - y1) / (y2 - y1) + x1 {
                c = !c;
            }
        }
    }
    c
}

/// What a snapshot holds (`meta.json`, written last: its presence marks a complete snapshot).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotMeta {
    pub format: u32,
    pub region: String,
    pub download: DownloadInfo,
    /// When the snapshot finished building (RFC 3339).
    pub built_at: String,
    pub records: u64,
    pub taxa: u64,
    pub bytes: u64,
    /// datasetKey of each dataset index used in the records.
    pub datasets: Vec<String>,
    /// basisOfRecord of each basis index.
    pub bases: Vec<String>,
    /// Records counted in the effort grids (with coordinates and a class; format 2, 0 before).
    #[serde(default)]
    pub effort_records: u64,
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    key: u32,
    bucket: u16,
    start: u64,
    count: u32,
}

/// A built snapshot, opened read-only (cheap to share between jobs).
#[derive(Debug)]
pub struct Snapshot {
    pub dir: PathBuf,
    pub meta: SnapshotMeta,
    index: Vec<Entry>,
    /// Effort classes sorted by name (empty for format 1).
    effort: Vec<EffortEntry>,
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> DataError + '_ {
    move |e| DataError::Cache(format!("{}: {e}", path.display()))
}

impl Snapshot {
    pub fn open(dir: &Path) -> Result<Self, DataError> {
        let meta_path = dir.join("meta.json");
        let meta: SnapshotMeta =
            serde_json::from_slice(&std::fs::read(&meta_path).map_err(io(&meta_path))?)
                .map_err(|e| DataError::Cache(format!("{}: {e}", meta_path.display())))?;
        if !(MIN_FORMAT..=FORMAT).contains(&meta.format) {
            return Err(DataError::Cache(format!(
                "{}: snapshot format {} (expected {MIN_FORMAT} to {FORMAT})",
                dir.display(),
                meta.format
            )));
        }
        let mut effort: Vec<EffortEntry> = if meta.format >= 2 {
            let p = dir.join("effort-index.json");
            serde_json::from_slice(&std::fs::read(&p).map_err(io(&p))?)
                .map_err(|e| DataError::Cache(format!("{}: {e}", p.display())))?
        } else {
            Vec::new()
        };
        effort.sort_by(|a, b| a.class.cmp(&b.class));
        let idx_path = dir.join("index.bin");
        let raw = std::fs::read(&idx_path).map_err(io(&idx_path))?;
        if raw.len() % 18 != 0 {
            return Err(DataError::Cache(format!(
                "{}: truncated index",
                idx_path.display()
            )));
        }
        let index = raw
            .chunks_exact(18)
            .map(|c| Entry {
                key: u32::from_le_bytes([c[0], c[1], c[2], c[3]]),
                bucket: u16::from_le_bytes([c[4], c[5]]),
                start: u64::from_le_bytes([c[6], c[7], c[8], c[9], c[10], c[11], c[12], c[13]]),
                count: u32::from_le_bytes([c[14], c[15], c[16], c[17]]),
            })
            .collect();
        Ok(Self {
            dir: dir.to_path_buf(),
            meta,
            index,
            effort,
        })
    }

    /// Classes with an effort grid, sorted (empty for a format-1 snapshot).
    pub fn effort_classes(&self) -> Vec<String> {
        self.effort.iter().map(|e| e.class.clone()).collect()
    }

    /// Effort-grid summary of one class (cells, records), if the snapshot has it.
    pub fn effort_entry(&self, class: &str) -> Option<&EffortEntry> {
        self.effort
            .binary_search_by(|e| e.class.as_str().cmp(class))
            .ok()
            .map(|i| &self.effort[i])
    }

    /// Sampling effort of `class` (e.g. "Aves"): (lng, lat of the cell centre, records) for each
    /// 30-arc-second cell with records whose centre lies in `bbox` (west, south, east, north;
    /// inclusive), in cell order (north to south, then west to east). `Ok(None)` when the
    /// snapshot has no grid for the class (format 1, or no record of that class); cells without
    /// records are not listed (their count is 0).
    ///
    /// Reads only what is needed: two binary searches in the class's sorted runs find the rows
    /// of the box, then that slice is read and filtered by column.
    pub fn effort(
        &self,
        class: &str,
        bbox: (f64, f64, f64, f64),
    ) -> Result<Option<Vec<(f64, f64, u32)>>, DataError> {
        let Some(e) = self.effort_entry(class) else {
            return Ok(None);
        };
        let (w, s, ea, n) = bbox;
        let k = f64::from(EFFORT_CELLS_PER_DEGREE);
        // Rows whose centre may lie in [s, n] (one row of margin; exact test below).
        let row_of = |lat: f64| ((90.0 - lat) * k - 0.5).clamp(0.0, f64::from(EFFORT_ROWS - 1));
        let (r0, r1) = (row_of(n).floor() as u32, row_of(s).ceil() as u32);
        if e.cells == 0 || !(w <= ea && s <= n) {
            return Ok(Some(Vec::new()));
        }
        let path = self.dir.join("effort.bin");
        let mut f = std::fs::File::open(&path).map_err(io(&path))?;
        let mut cell_at = |i: u64| -> Result<u32, DataError> {
            let mut b = [0u8; 4];
            f.seek(SeekFrom::Start((e.offset + i) * EFFORT_RUN as u64))
                .and_then(|_| f.read_exact(&mut b))
                .map_err(io(&path))?;
            Ok(u32::from_le_bytes(b))
        };
        // First run with cell >= target.
        let mut lower = |target: u64| -> Result<u64, DataError> {
            let (mut lo, mut hi) = (0u64, e.cells);
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                if u64::from(cell_at(mid)?) < target {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            Ok(lo)
        };
        let cols = u64::from(EFFORT_COLS);
        let a = lower(u64::from(r0) * cols)?;
        let b = lower((u64::from(r1) + 1) * cols)?;
        let mut out = Vec::new();
        if b <= a {
            return Ok(Some(out));
        }
        let mut buf = vec![0u8; ((b - a) as usize) * EFFORT_RUN];
        f.seek(SeekFrom::Start((e.offset + a) * EFFORT_RUN as u64))
            .and_then(|_| f.read_exact(&mut buf))
            .map_err(io(&path))?;
        for r in buf.chunks_exact(EFFORT_RUN) {
            let cell = u32::from_le_bytes([r[0], r[1], r[2], r[3]]);
            let count = u32::from_le_bytes([r[4], r[5], r[6], r[7]]);
            let (x, y) = effort_cell_centre(cell);
            if x >= w && x <= ea && y >= s && y <= n {
                out.push((x, y, count));
            }
        }
        Ok(Some(out))
    }

    /// Records of one taxon (species or below) inside `bbox` (west, south, east, north),
    /// inclusive like GBIF's `within` box, in the order GBIF delivered them.
    pub fn records(
        &self,
        taxon: u64,
        bbox: (f64, f64, f64, f64),
    ) -> Result<Vec<Occurrence>, DataError> {
        let Ok(key) = u32::try_from(taxon) else {
            return Ok(Vec::new());
        };
        let Ok(i) = self.index.binary_search_by_key(&key, |e| e.key) else {
            return Ok(Vec::new());
        };
        let e = self.index[i];
        let path = self.dir.join(format!("b{:03}.bin", e.bucket));
        let mut f = std::fs::File::open(&path).map_err(io(&path))?;
        f.seek(SeekFrom::Start(e.start * REC as u64))
            .map_err(io(&path))?;
        let mut buf = vec![0u8; e.count as usize * REC];
        f.read_exact(&mut buf).map_err(io(&path))?;
        let (w, s, ea, n) = bbox;
        let mut out = Vec::new();
        for r in buf.chunks_exact(REC) {
            let o = decode(r, &self.meta);
            if o.lng >= w && o.lng <= ea && o.lat >= s && o.lat <= n {
                out.push(o);
            }
        }
        Ok(out)
    }

    /// The records of `taxa` inside `bbox` as a [`BatchData`] (one file per taxon in `out`),
    /// like a per-study download; `info` cites the mirror's download with the records used.
    pub fn batch(
        &self,
        taxa: &[u64],
        bbox: (f64, f64, f64, f64),
        out: &Path,
    ) -> Result<(BatchData, HashMap<String, u64>), DataError> {
        let _ = std::fs::remove_dir_all(out);
        std::fs::create_dir_all(out).map_err(io(out))?;
        let mut counts = HashMap::new();
        let mut datasets: HashMap<String, u64> = HashMap::new();
        let mut total = 0u64;
        let mut sorted = taxa.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        for &t in &sorted {
            let records = self.records(t, bbox)?;
            let path = out.join(format!("{t}.tsv"));
            let mut w = BufWriter::new(std::fs::File::create(&path).map_err(io(&path))?);
            for o in &records {
                *datasets.entry(o.dataset_key.clone()).or_insert(0) += 1;
                crate::gbif_download::write_split_line(&mut w, o).map_err(io(&path))?;
            }
            w.flush().map_err(io(&path))?;
            total += records.len() as u64;
            counts.insert(t, records.len() as u64);
        }
        let info = DownloadInfo {
            total_records: total,
            ..self.meta.download.clone()
        };
        Ok((
            BatchData::from_split(info, out.to_path_buf(), counts, sorted),
            datasets,
        ))
    }
}

fn decode(r: &[u8], meta: &SnapshotMeta) -> Occurrence {
    let u64_at = |o: usize| u64::from_le_bytes(r[o..o + 8].try_into().unwrap_or([0; 8]));
    let f64_at = |o: usize| f64::from_le_bytes(r[o..o + 8].try_into().unwrap_or([0; 8]));
    let year = i32::from_le_bytes(r[32..36].try_into().unwrap_or([0; 4]));
    let ds = u32::from_le_bytes(r[36..40].try_into().unwrap_or([0; 4]));
    let unc = f64_at(24);
    Occurrence {
        key: u64_at(0),
        lng: f64_at(8),
        lat: f64_at(16),
        uncertainty_m: (!unc.is_nan()).then_some(unc),
        year: (year != i32::MIN).then_some(year),
        dataset_key: meta.datasets.get(ds as usize).cloned().unwrap_or_default(),
        // Built from SIMPLE_CSV, which does not report generalisation.
        generalised: None,
        licence: LICENCES
            .get(r[40] as usize)
            .copied()
            .unwrap_or("OTHER")
            .to_string(),
        basis_of_record: meta.bases.get(r[41] as usize).cloned().unwrap_or_default(),
    }
}

fn encode(o: &Occurrence, dataset: u32, licence: u8, basis: u8) -> [u8; REC] {
    let mut r = [0u8; REC];
    r[0..8].copy_from_slice(&o.key.to_le_bytes());
    r[8..16].copy_from_slice(&o.lng.to_le_bytes());
    r[16..24].copy_from_slice(&o.lat.to_le_bytes());
    r[24..32].copy_from_slice(&o.uncertainty_m.unwrap_or(f64::NAN).to_le_bytes());
    r[32..36].copy_from_slice(&o.year.unwrap_or(i32::MIN).to_le_bytes());
    r[36..40].copy_from_slice(&dataset.to_le_bytes());
    r[40] = licence;
    r[41] = basis;
    r
}

impl Snapshot {
    /// Whether the taxon has records in the mirror (it is then a species or lower taxon).
    pub fn contains(&self, taxon: u64) -> bool {
        u32::try_from(taxon).is_ok_and(|k| self.index.binary_search_by_key(&k, |e| e.key).is_ok())
    }
}

/// Where one region's mirror lives: snapshots `<root>/<download key>/`, the one in use named by
/// `<root>/current`, the GBIF request in progress in `<root>/pending`, the archive being
/// downloaded in `<root>/incoming/`.
#[derive(Debug, Clone)]
pub struct MirrorDir {
    pub root: PathBuf,
}

/// What a refresh is doing (for the administration page and logs).
#[derive(Debug, Clone, PartialEq)]
pub enum RefreshStep {
    Requesting,
    /// GBIF is preparing the download (hours for a whole region).
    Waiting {
        key: String,
        status: String,
    },
    Downloading {
        key: String,
        bytes: u64,
        total: u64,
    },
    Building {
        key: String,
        records: u64,
    },
}

/// A pending region request older than this is requested again.
const REGION_PENDING_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(3 * 86_400);
/// GBIF may take many hours for hundreds of millions of records.
const REGION_MAX_WAIT: std::time::Duration = std::time::Duration::from_secs(48 * 3600);
const CHUNK: u64 = 256 << 20;

impl MirrorDir {
    pub fn new(data_dir: &Path, region: &Region) -> Self {
        Self {
            root: data_dir.join("gbif-mirror").join(region.id),
        }
    }

    /// The snapshot in use, if any (a damaged one is ignored: the next refresh replaces it).
    pub fn open_current(&self) -> Option<Snapshot> {
        let key = std::fs::read_to_string(self.root.join("current")).ok()?;
        let dir = self.root.join(key.trim());
        match Snapshot::open(&dir) {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!(error = %e, "GBIF mirror snapshot ignored");
                None
            }
        }
    }

    fn write_atomic(&self, name: &str, text: &str) -> Result<(), DataError> {
        let p = self.root.join(name);
        let tmp = self.root.join(format!(".{name}.tmp"));
        std::fs::write(&tmp, text)
            .and_then(|()| std::fs::rename(&tmp, &p))
            .map_err(io(&p))
    }

    /// Request (or resume) the region's download, wait for GBIF, download the archive in
    /// resumable chunks, build a snapshot and make it current. Every step resumes after an
    /// interruption: the pending request, the partial archive and the previous snapshot stay.
    /// Returns the new snapshot. Snapshots other than the new and the previous one are removed.
    pub fn refresh(
        &self,
        http: &crate::Http,
        creds: &crate::gbif_download::Credentials,
        region: &Region,
        built_at: &dyn Fn() -> String,
        cancelled: &(dyn Fn() -> bool + Sync),
        step: &dyn Fn(&RefreshStep),
    ) -> Result<Snapshot, DataError> {
        std::fs::create_dir_all(self.root.join("incoming")).map_err(io(&self.root))?;
        let pending = self.root.join("pending");
        let fresh = |p: &Path| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|a| a < REGION_PENDING_MAX_AGE)
        };
        let key = match std::fs::read_to_string(&pending)
            .ok()
            .map(|k| k.trim().to_string())
        {
            Some(k) if !k.is_empty() && fresh(&pending) => k,
            _ => {
                step(&RefreshStep::Requesting);
                let body = region.predicate(&creds.username);
                let key = http.post_json_basic(
                    "https://api.gbif.org/v1/occurrence/download/request",
                    &body.to_string(),
                    &creds.username,
                    &creds.password,
                )?;
                let key = key.trim().trim_matches('"').to_string();
                self.write_atomic("pending", &key)?;
                key
            }
        };
        let info = match crate::gbif_download::wait_for(
            http,
            &key,
            REGION_MAX_WAIT,
            std::time::Duration::from_secs(60),
            cancelled,
            &|s| {
                if let crate::gbif_download::Step::Waiting { status, .. } = s {
                    step(&RefreshStep::Waiting {
                        key: key.clone(),
                        status: (*status).to_string(),
                    });
                }
            },
        ) {
            Ok(i) => i,
            Err(e @ DataError::Cancelled) => return Err(e),
            Err(e) => {
                // Failed or lost at GBIF: the next attempt requests a new download.
                let _ = std::fs::remove_file(&pending);
                return Err(e);
            }
        };
        let zip = self.root.join("incoming").join(format!("{key}.zip"));
        if !zip.is_file() {
            http.download_resumable(
                &format!("https://api.gbif.org/v1/occurrence/download/request/{key}.zip"),
                &zip,
                CHUNK,
                cancelled,
                &|bytes, total| {
                    step(&RefreshStep::Downloading {
                        key: key.clone(),
                        bytes,
                        total,
                    })
                },
            )?;
        }
        let snap = self.install(
            &zip,
            region,
            info,
            &built_at(),
            true,
            cancelled,
            &|records| {
                step(&RefreshStep::Building {
                    key: key.clone(),
                    records,
                })
            },
        )?;
        let _ = std::fs::remove_file(&pending);
        let _ = std::fs::remove_dir_all(self.root.join("incoming"));
        Ok(snap)
    }

    /// Build a snapshot from a region archive (`info` describes its GBIF download), make it
    /// current, and remove snapshots other than it and the previous one (studies running now
    /// may still read the previous one). Also used to import an archive downloaded by hand.
    #[allow(clippy::too_many_arguments)]
    pub fn install(
        &self,
        zip: &Path,
        region: &Region,
        info: DownloadInfo,
        built_at: &str,
        remove_zip: bool,
        cancelled: &dyn Fn() -> bool,
        progress: BuildProgress<'_>,
    ) -> Result<Snapshot, DataError> {
        std::fs::create_dir_all(&self.root).map_err(io(&self.root))?;
        let key = info.key.clone();
        let building = self.root.join(format!("{key}.building"));
        let meta = build(
            zip, region, info, built_at, &building, remove_zip, cancelled, progress,
        )?;
        let dir = self.root.join(&key);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::rename(&building, &dir).map_err(io(&dir))?;
        let previous = std::fs::read_to_string(self.root.join("current"))
            .ok()
            .map(|k| k.trim().to_string());
        self.write_atomic("current", &key)?;
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                let keep = name == key
                    || Some(&name) == previous.as_ref()
                    || name == "incoming"
                    || !e.path().is_dir();
                if !keep {
                    let _ = std::fs::remove_dir_all(e.path());
                }
            }
        }
        tracing::info!(region = region.id, key = %key, records = meta.records, taxa = meta.taxa, "GBIF mirror snapshot ready");
        Snapshot::open(&dir)
    }
}

/// Progress of a build: records read so far.
pub type BuildProgress<'a> = &'a dyn Fn(u64);

/// Build a snapshot in `dir` from a GBIF SIMPLE_CSV archive of the region. `dir` is created
/// fresh; `meta.json` is written last, so an interrupted build is never mistaken for a snapshot.
/// `built_at` (RFC 3339) is recorded in the metadata.
#[allow(clippy::too_many_arguments)]
pub fn build(
    zip_path: &Path,
    region: &Region,
    download: DownloadInfo,
    built_at: &str,
    dir: &Path,
    remove_zip: bool,
    cancelled: &dyn Fn() -> bool,
    progress: BuildProgress<'_>,
) -> Result<SnapshotMeta, DataError> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    // Pass 1: append every record to its taxon's bucket (unsorted).
    let raw_path = |b: usize| dir.join(format!("raw{b:03}.bin"));
    let mut raw: Vec<BufWriter<std::fs::File>> = (0..BUCKETS)
        .map(|b| {
            let p = raw_path(b);
            std::fs::File::create(&p)
                .map(|f| BufWriter::with_capacity(64 << 10, f))
                .map_err(io(&p))
        })
        .collect::<Result<_, _>>()?;
    let mut datasets: Vec<String> = Vec::new();
    let mut dataset_ix: HashMap<String, u32> = HashMap::new();
    let mut bases: Vec<String> = Vec::new();
    // Effort pairs (class index, cell), appended to one file per band of grid rows, opened when
    // first needed (a region touches few bands).
    let band_path = |b: usize| dir.join(format!("effort{b:04}.tmp"));
    let mut bands: Vec<Option<BufWriter<std::fs::File>>> =
        (0..EFFORT_BANDS).map(|_| None).collect();
    let mut classes: Vec<String> = Vec::new();
    let mut class_ix: HashMap<String, u16> = HashMap::new();
    let mut effort_records = 0u64;
    let mut read = 0u64;
    let mut err: Option<DataError> = None;
    crate::gbif_download::for_each_row(zip_path, |row| {
        if err.is_some() {
            return false;
        }
        read += 1;
        if read.is_multiple_of(1_000_000) {
            progress(read);
            if cancelled() {
                err = Some(DataError::Cancelled);
                return false;
            }
        }
        // Sampling effort: every record with a class, identified to species or not.
        if let Some(class) = row.class
            && let Some(cell) = effort_cell(row.occurrence.lng, row.occurrence.lat)
        {
            let ix = match class_ix.get(class) {
                Some(&i) => Some(i),
                None if classes.len() < usize::from(u16::MAX) => {
                    let i = classes.len() as u16;
                    classes.push(class.to_string());
                    class_ix.insert(class.to_string(), i);
                    Some(i)
                }
                None => None,
            };
            if let Some(ix) = ix {
                let b = (cell / EFFORT_COLS / EFFORT_BAND_ROWS) as usize;
                if bands[b].is_none() {
                    match std::fs::File::create(band_path(b)) {
                        Ok(f) => bands[b] = Some(BufWriter::with_capacity(64 << 10, f)),
                        Err(e) => {
                            err = Some(DataError::Cache(format!("{}: {e}", dir.display())));
                            return false;
                        }
                    }
                }
                if let Some(w) = bands[b].as_mut()
                    && let Err(e) = w
                        .write_all(&ix.to_le_bytes())
                        .and_then(|()| w.write_all(&cell.to_le_bytes()))
                {
                    err = Some(DataError::Cache(format!("{}: {e}", dir.display())));
                    return false;
                }
                effort_records += 1;
            }
        }
        // Same matching as a per-study download: the record's species, and its own taxon when
        // that is another one (subspecies, synonym). Records not identified to species are not
        // served by a mirror (studies are of species or below).
        let Some(species) = row.species_key else {
            return true;
        };
        let ds = match row.dataset_key {
            Some(k) => *dataset_ix.entry(k.to_string()).or_insert_with(|| {
                datasets.push(k.to_string());
                (datasets.len() - 1) as u32
            }),
            None => u32::MAX,
        };
        let basis = match bases
            .iter()
            .position(|b| b == row.occurrence.basis_of_record.as_str())
        {
            Some(i) => i as u8,
            None if bases.len() < 255 => {
                bases.push(row.occurrence.basis_of_record.clone());
                (bases.len() - 1) as u8
            }
            None => 255,
        };
        let lic = LICENCES
            .iter()
            .position(|l| *l == row.occurrence.licence)
            .unwrap_or(3) as u8;
        let rec = encode(&row.occurrence, ds, lic, basis);
        let mut keys = [Some(species), row.taxon_key.filter(|t| *t != species)];
        for k in keys.iter_mut().flatten() {
            let Ok(k32) = u32::try_from(*k) else { continue };
            let w = &mut raw[k32 as usize % BUCKETS];
            if let Err(e) = w
                .write_all(&k32.to_le_bytes())
                .and_then(|()| w.write_all(&rec))
            {
                err = Some(DataError::Cache(format!("{}: {e}", dir.display())));
                return false;
            }
        }
        true
    })?;
    if let Some(e) = err {
        return Err(e);
    }
    for (b, mut w) in raw.into_iter().enumerate() {
        w.flush().map_err(io(&raw_path(b)))?;
    }
    let mut band_used = vec![false; EFFORT_BANDS];
    for (b, w) in bands.into_iter().enumerate() {
        if let Some(mut w) = w {
            w.flush().map_err(io(&band_path(b)))?;
            band_used[b] = true;
        }
    }
    progress(read);
    // The archive (tens of GB, observer names included) is no longer needed: free the disk
    // before the second pass writes the sorted buckets.
    if remove_zip {
        std::fs::remove_file(zip_path).map_err(io(zip_path))?;
    }
    // Pass 2: sort each bucket by taxon (stable: GBIF's order within a taxon) and index it.
    let mut index: Vec<u8> = Vec::new();
    let (mut records, mut taxa, mut bytes) = (0u64, 0u64, 0u64);
    for b in 0..BUCKETS {
        if cancelled() {
            return Err(DataError::Cancelled);
        }
        let rp = raw_path(b);
        let data = std::fs::read(&rp).map_err(io(&rp))?;
        let mut rows: Vec<&[u8]> = data.chunks_exact(RAW).collect();
        rows.sort_by_key(|r| u32::from_le_bytes([r[0], r[1], r[2], r[3]]));
        let out_path = dir.join(format!("b{b:03}.bin"));
        let mut out = BufWriter::new(std::fs::File::create(&out_path).map_err(io(&out_path))?);
        let mut i = 0usize;
        while i < rows.len() {
            let key = u32::from_le_bytes([rows[i][0], rows[i][1], rows[i][2], rows[i][3]]);
            let start = i;
            while i < rows.len() && rows[i][0..4] == key.to_le_bytes() {
                out.write_all(&rows[i][4..]).map_err(io(&out_path))?;
                i += 1;
            }
            index.push_entry(key, b as u16, start as u64, (i - start) as u32);
            taxa += 1;
        }
        out.flush().map_err(io(&out_path))?;
        records += rows.len() as u64;
        bytes += (rows.len() * REC) as u64;
        drop(rows);
        std::fs::remove_file(&rp).map_err(io(&rp))?;
    }
    // Index sorted by key across buckets.
    let mut entries: Vec<[u8; 18]> = index
        .chunks_exact(18)
        .map(|c| c.try_into().unwrap_or([0; 18]))
        .collect();
    entries.sort_by_key(|e| u32::from_le_bytes([e[0], e[1], e[2], e[3]]));
    let idx_path = dir.join("index.bin");
    std::fs::write(&idx_path, entries.concat()).map_err(io(&idx_path))?;
    // Pass 3: effort grids, one band at a time.
    let effort_bytes = build_effort(dir, &classes, &band_used, &band_path, cancelled)?;
    let meta = SnapshotMeta {
        format: FORMAT,
        region: region.id.to_string(),
        download,
        built_at: built_at.to_string(),
        records,
        taxa,
        bytes: bytes + (entries.len() * 18) as u64 + effort_bytes,
        datasets,
        bases,
        effort_records,
    };
    let meta_path = dir.join("meta.json");
    let tmp = meta_path.with_extension("tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(&meta).map_err(|e| DataError::Cache(e.to_string()))?,
    )
    .and_then(|()| std::fs::rename(&tmp, &meta_path))
    .map_err(io(&meta_path))?;
    Ok(meta)
}

/// Sort and count the effort pairs of each band file into `effort.bin` (runs of each class
/// contiguous and sorted by cell) and `effort-index.json`; removes the band files. Memory: one
/// band's pairs (8 bytes per record of the band) at a time. Returns the bytes written.
fn build_effort(
    dir: &Path,
    classes: &[String],
    band_used: &[bool],
    band_path: &dyn Fn(usize) -> PathBuf,
    cancelled: &dyn Fn() -> bool,
) -> Result<u64, DataError> {
    // Runs in (band, class, cell) order go to a temporary file; each class remembers where its
    // segment of each band is. Bands follow row order, so a class's segments in band order are
    // sorted by cell.
    let tmp_path = dir.join("effort-runs.tmp");
    let mut tmp = BufWriter::new(std::fs::File::create(&tmp_path).map_err(io(&tmp_path))?);
    let mut segments: Vec<Vec<(u64, u64)>> = vec![Vec::new(); classes.len()];
    let mut totals: Vec<u64> = vec![0; classes.len()];
    let mut written = 0u64; // runs
    for (b, _) in band_used.iter().enumerate().filter(|(_, u)| **u) {
        if cancelled() {
            return Err(DataError::Cancelled);
        }
        let bp = band_path(b);
        let data = std::fs::read(&bp).map_err(io(&bp))?;
        let mut keys: Vec<u64> = data
            .chunks_exact(EFFORT_PAIR)
            .map(|c| {
                (u64::from(u16::from_le_bytes([c[0], c[1]])) << 32)
                    | u64::from(u32::from_le_bytes([c[2], c[3], c[4], c[5]]))
            })
            .collect();
        drop(data);
        keys.sort_unstable();
        let mut i = 0usize;
        while i < keys.len() {
            let class = (keys[i] >> 32) as usize;
            let start = written;
            while i < keys.len() && (keys[i] >> 32) as usize == class {
                let k = keys[i];
                let mut n = 0u64;
                while i < keys.len() && keys[i] == k {
                    n += 1;
                    i += 1;
                }
                let cell = (k & 0xFFFF_FFFF) as u32;
                let count = u32::try_from(n).unwrap_or(u32::MAX);
                tmp.write_all(&cell.to_le_bytes())
                    .and_then(|()| tmp.write_all(&count.to_le_bytes()))
                    .map_err(io(&tmp_path))?;
                written += 1;
                if let Some(t) = totals.get_mut(class) {
                    *t += n;
                }
            }
            if let Some(seg) = segments.get_mut(class) {
                seg.push((start, written - start));
            }
        }
        drop(keys);
        std::fs::remove_file(&bp).map_err(io(&bp))?;
    }
    tmp.flush().map_err(io(&tmp_path))?;
    drop(tmp);
    // Gather each class's segments, classes by name.
    let mut order: Vec<usize> = (0..classes.len()).collect();
    order.sort_by(|&a, &b| classes[a].cmp(&classes[b]));
    let out_path = dir.join("effort.bin");
    let mut out = BufWriter::new(std::fs::File::create(&out_path).map_err(io(&out_path))?);
    let mut src = std::fs::File::open(&tmp_path).map_err(io(&tmp_path))?;
    let mut index: Vec<EffortEntry> = Vec::with_capacity(classes.len());
    let mut offset = 0u64;
    for c in order {
        let cells: u64 = segments[c].iter().map(|s| s.1).sum();
        if cells == 0 {
            continue;
        }
        for &(start, n) in &segments[c] {
            src.seek(SeekFrom::Start(start * EFFORT_RUN as u64))
                .map_err(io(&tmp_path))?;
            let copied = std::io::copy(&mut (&mut src).take(n * EFFORT_RUN as u64), &mut out)
                .map_err(io(&out_path))?;
            if copied != n * EFFORT_RUN as u64 {
                return Err(DataError::Cache(format!(
                    "{}: short effort segment",
                    tmp_path.display()
                )));
            }
        }
        index.push(EffortEntry {
            class: classes[c].clone(),
            offset,
            cells,
            records: totals[c],
        });
        offset += cells;
    }
    out.flush().map_err(io(&out_path))?;
    drop(src);
    std::fs::remove_file(&tmp_path).map_err(io(&tmp_path))?;
    let idx_path = dir.join("effort-index.json");
    let idx = serde_json::to_vec(&index).map_err(|e| DataError::Cache(e.to_string()))?;
    std::fs::write(&idx_path, &idx).map_err(io(&idx_path))?;
    Ok(offset * EFFORT_RUN as u64 + idx.len() as u64)
}

trait PushEntry {
    fn push_entry(&mut self, key: u32, bucket: u16, start: u64, count: u32);
}

impl PushEntry for Vec<u8> {
    fn push_entry(&mut self, key: u32, bucket: u16, start: u64, count: u32) {
        self.extend_from_slice(&key.to_le_bytes());
        self.extend_from_slice(&bucket.to_le_bytes());
        self.extend_from_slice(&start.to_le_bytes());
        self.extend_from_slice(&count.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("niche-mirror-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("tmp");
        d
    }

    const HEAD: &str = "gbifID\tdatasetKey\tdecimalLatitude\tdecimalLongitude\tcoordinateUncertaintyInMeters\tyear\ttaxonKey\tspeciesKey\tbasisOfRecord\tlicense\trecordedBy";

    fn archive(path: &Path, rows: &[&str]) {
        archive_with(path, HEAD, rows);
    }

    fn archive_with(path: &Path, head: &str, rows: &[&str]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(path).expect("zip"));
        z.start_file("0001.csv", zip::write::SimpleFileOptions::default())
            .expect("entry");
        z.write_all(head.as_bytes()).expect("w");
        for r in rows {
            z.write_all(b"\n").expect("w");
            z.write_all(r.as_bytes()).expect("w");
        }
        z.finish().expect("finish");
    }

    fn info() -> DownloadInfo {
        DownloadInfo {
            key: "0000001-261019000000000".into(),
            doi: Some("https://doi.org/10.15468/dl.mirror".into()),
            total_records: 7,
            created: "2026-10-19T00:00:00Z".into(),
            url: "https://www.gbif.org/occurrence/download/0000001-261019000000000".into(),
        }
    }

    const ROWS: &[&str] = &[
        // species 111 (and its subspecies 112), species 222 (record matched to synonym 999),
        // a genus-level record (no speciesKey), one record without coordinates.
        "1\tds-a\t43.6\t1.4\t10\t2015\t111\t111\tHUMAN_OBSERVATION\tCC_BY_4_0\tJane",
        "2\tds-b\t43.7\t1.5\t\t2001\t112\t111\tPRESERVED_SPECIMEN\thttp://creativecommons.org/publicdomain/zero/1.0/legalcode\tJohn",
        "3\tds-a\t40.4\t-3.7\t5\t\t999\t222\tHUMAN_OBSERVATION\tCC_BY_NC_4_0\t",
        "4\tds-a\t48.8\t2.3\t5\t2020\t5000\t\tHUMAN_OBSERVATION\tCC0_1_0\t",
        "5\tds-c\t\t2.3\t5\t2020\t111\t111\tHUMAN_OBSERVATION\tCC0_1_0\t",
        "6\tds-c\t48.85\t2.35\t1000.5\t2019\t111\t111\tFOSSIL_SPECIMEN\tCC0_1_0\t",
        "7\tds-b\t49.0\t8.4\t\t2018\t222\t222\tHUMAN_OBSERVATION\tCC_BY_4_0\t",
    ];

    #[test]
    fn mirror_gives_what_a_per_study_download_gives() {
        let d = tmp("equiv");
        let zip = d.join("region.zip");
        archive(&zip, ROWS);
        let region = crate::gbif_mirror_cells::FR_ES;
        let meta = build(
            &zip,
            &region,
            info(),
            "2026-10-19T01:00:00Z",
            &d.join("snap"),
            false,
            &|| false,
            &|_| {},
        )
        .expect("build");
        assert_eq!(
            meta.records,
            6 + 1,
            "rows with coordinates and a species: 5, record 2 also under 112"
        );
        let snap = Snapshot::open(&d.join("snap")).expect("open");
        let everywhere = (-180.0, -90.0, 180.0, 90.0);
        // Same records and fields as the per-study path reading the same archive.
        // (One taxon per request: requested together, a subspecies record goes to its species.)
        for t in [111u64, 112, 222, 999] {
            let per_study = crate::gbif_download::read_archive(&zip, &[t]).expect("read");
            let mut a = snap.records(t, everywhere).expect("records");
            let mut b = per_study.get(&t).cloned().unwrap_or_default();
            a.sort_by_key(|o| o.key);
            b.sort_by_key(|o| o.key);
            assert_eq!(a, b, "taxon {t}");
        }
        assert_eq!(snap.records(111, everywhere).expect("r").len(), 3);
        let r = snap.records(111, everywhere).expect("r");
        let rec2 = r.iter().find(|o| o.key == 2).expect("record 2");
        assert_eq!(
            (
                rec2.licence.as_str(),
                rec2.uncertainty_m,
                rec2.dataset_key.as_str()
            ),
            ("CC0_1_0", None, "ds-b")
        );
        let rec6 = r.iter().find(|o| o.key == 6).expect("record 6");
        assert_eq!(
            (rec6.uncertainty_m, rec6.year, rec6.basis_of_record.as_str()),
            (Some(1000.5), Some(2019), "FOSSIL_SPECIMEN")
        );
        // The box filter is inclusive like GBIF's.
        assert_eq!(
            snap.records(111, (1.4, 43.6, 1.45, 43.65))
                .expect("r")
                .len(),
            1
        );
        assert!(
            snap.records(5000, everywhere).expect("genus").is_empty(),
            "not identified to species"
        );
        assert!(
            snap.records(424_242, everywhere)
                .expect("absent")
                .is_empty()
        );
        // As a batch: one split file per taxon, no observer names, datasets counted.
        let (batch, datasets) = snap
            .batch(&[111, 222], everywhere, &d.join("extract"))
            .expect("batch");
        assert_eq!(batch.info.total_records, 5);
        assert_eq!(
            batch.info.doi.as_deref(),
            Some("https://doi.org/10.15468/dl.mirror")
        );
        assert_eq!(batch.load(111).expect("load").records.len(), 3);
        assert_eq!(datasets.get("ds-a"), Some(&2));
        let text = std::fs::read_to_string(d.join("extract/111.tsv")).expect("tsv");
        assert!(!text.contains("Jane") && !text.contains("John"));
        // A half-built snapshot (no meta.json) does not open.
        std::fs::remove_file(d.join("snap/meta.json")).expect("rm");
        assert!(Snapshot::open(&d.join("snap")).is_err());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn effort_cells_follow_the_global_30_arc_second_grid() {
        assert_eq!(effort_cell(-180.0, 90.0), Some(0));
        // Last cell of the first row, and the last cell of the grid (edges clamp inwards).
        assert_eq!(effort_cell(180.0, 90.0), Some(43_199));
        assert_eq!(effort_cell(180.0, -90.0), Some(21_599 * 43_200 + 43_199));
        // 1.4 E, 43.6 N: col ⌊181.4·120⌋ = 21768, row ⌊46.4·120⌋ = 5568.
        assert_eq!(effort_cell(1.4, 43.6), Some(5_568 * 43_200 + 21_768));
        assert_eq!(effort_cell(f64::NAN, 1.0), None);
        assert_eq!(effort_cell(181.0, 1.0), None);
        let (x, y) = effort_cell_centre(5_568 * 43_200 + 21_768);
        assert!((x - (1.4 + 1.0 / 240.0)).abs() < 1e-9 && (y - (43.6 - 1.0 / 240.0)).abs() < 1e-9);
        assert_eq!(effort_cell(x, y), Some(5_568 * 43_200 + 21_768));
    }

    const CLASS_HEAD: &str = "gbifID\tdatasetKey\tkingdom\tclass\tdecimalLatitude\tdecimalLongitude\tcoordinateUncertaintyInMeters\tyear\ttaxonKey\tspeciesKey\tbasisOfRecord\tlicense\trecordedBy";

    /// Effort grids count every record with coordinates and a class (identified to species or
    /// not), once per record, by 30-arc-second cell; counts are checked exactly.
    #[test]
    fn effort_grids_count_records_by_class_and_cell() {
        let d = tmp("effort");
        let zip = d.join("region.zip");
        let rows = [
            // Aves: three records in one cell (one of them genus-level, no speciesKey), one in
            // the neighbouring cell to the east, one in Madrid.
            "1\tds\tAnimalia\tAves\t43.5960\t1.4001\t\t2020\t111\t111\tHUMAN_OBSERVATION\tCC0_1_0\t",
            "2\tds\tAnimalia\tAves\t43.5950\t1.4070\t\t2020\t112\t111\tHUMAN_OBSERVATION\tCC0_1_0\t",
            "3\tds\tAnimalia\tAves\t43.5990\t1.4050\t\t2020\t5000\t\tHUMAN_OBSERVATION\tCC0_1_0\t",
            "4\tds\tAnimalia\tAves\t43.5990\t1.4090\t\t2020\t111\t111\tHUMAN_OBSERVATION\tCC0_1_0\t",
            "5\tds\tAnimalia\tAves\t40.4\t-3.7\t\t2020\t222\t222\tHUMAN_OBSERVATION\tCC0_1_0\t",
            // Magnoliopsida: one record in the first Aves cell, one class-level record.
            "6\tds\tPlantae\tMagnoliopsida\t43.5960\t1.4001\t\t2020\t333\t333\tPRESERVED_SPECIMEN\tCC_BY_4_0\t",
            "7\tds\tPlantae\tMagnoliopsida\t48.85\t2.35\t\t2020\t220\t\tHUMAN_OBSERVATION\tCC_BY_4_0\t",
            // Not counted: no class, no coordinates.
            "8\tds\tFungi\t\t43.5960\t1.4001\t\t2020\t444\t444\tHUMAN_OBSERVATION\tCC0_1_0\t",
            "9\tds\tAnimalia\tAves\t\t1.4001\t\t2020\t111\t111\tHUMAN_OBSERVATION\tCC0_1_0\t",
        ];
        archive_with(&zip, CLASS_HEAD, &rows);
        let meta = build(
            &zip,
            &crate::gbif_mirror_cells::FR_ES,
            info(),
            "t",
            &d.join("snap"),
            false,
            &|| false,
            &|_| {},
        )
        .expect("build");
        assert_eq!(meta.format, 2);
        assert_eq!(meta.effort_records, 7);
        let snap = Snapshot::open(&d.join("snap")).expect("open");
        assert_eq!(snap.effort_classes(), vec!["Aves", "Magnoliopsida"]);
        let everywhere = (-180.0, -90.0, 180.0, 90.0);
        let cell = |lng: f64, lat: f64| effort_cell(lng, lat).expect("cell");
        let as_cells = |v: Vec<(f64, f64, u32)>| -> Vec<(u32, u32)> {
            v.into_iter().map(|(x, y, n)| (cell(x, y), n)).collect()
        };
        let toulouse = cell(1.4001, 43.5960);
        let east = cell(1.4090, 43.5990);
        assert_eq!(east, toulouse + 1);
        let madrid = cell(-3.7, 40.4);
        let mut want = vec![(toulouse, 3), (east, 1), (madrid, 1)];
        want.sort_unstable();
        let aves = snap
            .effort("Aves", everywhere)
            .expect("read")
            .expect("grid");
        assert_eq!(as_cells(aves), want);
        let plants = snap
            .effort("Magnoliopsida", everywhere)
            .expect("read")
            .expect("grid");
        let mut want_p = vec![(toulouse, 1), (cell(2.35, 48.85), 1)];
        want_p.sort_unstable();
        assert_eq!(as_cells(plants), want_p);
        let e = snap.effort_entry("Aves").expect("entry");
        assert_eq!((e.cells, e.records), (3, 5));
        // A box selects by cell centre (inclusive): only the Toulouse cell, not its east
        // neighbour, not Madrid.
        let (cx, cy) = effort_cell_centre(toulouse);
        let tight = snap
            .effort("Aves", (cx, cy, cx, cy))
            .expect("read")
            .expect("grid");
        assert_eq!(tight, vec![(cx, cy, 3)]);
        let france = snap
            .effort("Aves", (-5.0, 41.0, 10.0, 51.0))
            .expect("read")
            .expect("grid");
        assert_eq!(france.iter().map(|c| c.2).sum::<u32>(), 4);
        assert!(
            snap.effort("Aves", (5.0, 41.0, 4.0, 51.0))
                .expect("r")
                .expect("g")
                .is_empty()
        );
        // No grid for an absent class.
        assert_eq!(snap.effort("Insecta", everywhere).expect("read"), None);
        // Records by taxon are as before (the species-level matching is unchanged).
        assert_eq!(snap.records(111, everywhere).expect("r").len(), 3);
        assert!(!d.join("snap/effort-runs.tmp").exists());
        assert!(
            std::fs::read_dir(d.join("snap"))
                .expect("ls")
                .flatten()
                .all(|e| !e.file_name().to_string_lossy().ends_with(".tmp")),
            "no temporary band file left"
        );
        let _ = std::fs::remove_dir_all(d);
    }

    /// Snapshots built before effort grids (format 1) still open and serve studies; they just
    /// have no effort grids. An archive without a `class` column gives no grid either.
    #[test]
    fn format_1_snapshots_still_open() {
        let d = tmp("format1");
        let zip = d.join("region.zip");
        archive(&zip, ROWS);
        build(
            &zip,
            &crate::gbif_mirror_cells::FR_ES,
            info(),
            "t",
            &d.join("snap"),
            false,
            &|| false,
            &|_| {},
        )
        .expect("build");
        let everywhere = (-180.0, -90.0, 180.0, 90.0);
        let snap = Snapshot::open(&d.join("snap")).expect("open");
        assert!(snap.effort_classes().is_empty(), "no class column");
        assert_eq!(snap.meta.effort_records, 0);
        // Turn it into a format-1 snapshot as written by the previous version.
        let meta_path = d.join("snap/meta.json");
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&meta_path).expect("meta")).expect("json");
        v["format"] = 1.into();
        v.as_object_mut().expect("obj").remove("effort_records");
        std::fs::write(&meta_path, v.to_string()).expect("write");
        std::fs::remove_file(d.join("snap/effort.bin")).expect("rm");
        std::fs::remove_file(d.join("snap/effort-index.json")).expect("rm");
        let old = Snapshot::open(&d.join("snap")).expect("format 1 opens");
        assert_eq!(old.meta.format, 1);
        assert!(old.effort_classes().is_empty());
        assert_eq!(old.effort("Aves", everywhere).expect("read"), None);
        assert_eq!(old.records(111, everywhere).expect("r").len(), 3);
        // Unknown future formats are refused.
        v["format"] = 3.into();
        std::fs::write(&meta_path, v.to_string()).expect("write");
        assert!(Snapshot::open(&d.join("snap")).is_err());
        let _ = std::fs::remove_dir_all(d);
    }

    /// Measurement for docs/architecture/data-sources-v1.md (run with `--ignored --nocapture`):
    /// build time, effort.bin size and query cost of a class with 2 M records over France.
    #[test]
    #[ignore = "measurement"]
    fn effort_measurement() {
        let d = tmp("effort-measure");
        let zip = d.join("region.zip");
        let n = 2_000_000u64;
        // Deterministic pseudo-random points over France, skewed towards a few hot spots.
        let mut x = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 11) as f64 / (1u64 << 53) as f64
        };
        let rows: Vec<String> = (0..n)
            .map(|i| {
                let (lng, lat) = if next() < 0.5 {
                    (-4.5 + 12.5 * next(), 42.5 + 8.5 * next())
                } else {
                    (2.2 + 0.4 * next(), 48.7 + 0.3 * next())
                };
                let class = if i % 3 == 0 { "Magnoliopsida" } else { "Aves" };
                format!(
                    "{i}\tds\tAnimalia\t{class}\t{lat:.5}\t{lng:.5}\t\t2020\t{}\t{}\tHUMAN_OBSERVATION\tCC0_1_0\t",
                    1000 + i % 500,
                    1000 + i % 500
                )
            })
            .collect();
        archive_with(
            &zip,
            CLASS_HEAD,
            &rows.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let t = std::time::Instant::now();
        let meta = build(
            &zip,
            &crate::gbif_mirror_cells::FR_ES,
            info(),
            "t",
            &d.join("snap"),
            false,
            &|| false,
            &|_| {},
        )
        .expect("build");
        println!(
            "build {:?} for {} records",
            t.elapsed(),
            meta.effort_records
        );
        // Same archive without a class column: the build without effort grids.
        let zip0 = d.join("region0.zip");
        archive_with(
            &zip0,
            &CLASS_HEAD.replace("\tclass\t", "\tnoclass\t"),
            &rows.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let t0 = std::time::Instant::now();
        let m0 = build(
            &zip0,
            &crate::gbif_mirror_cells::FR_ES,
            info(),
            "t",
            &d.join("snap0"),
            false,
            &|| false,
            &|_| {},
        )
        .expect("build");
        println!(
            "build without class column {:?} ({} effort records)",
            t0.elapsed(),
            m0.effort_records
        );
        let snap = Snapshot::open(&d.join("snap")).expect("open");
        for c in snap.effort_classes() {
            let e = snap.effort_entry(&c).expect("e");
            println!("{c}: {} cells, {} records", e.cells, e.records);
        }
        let size = std::fs::metadata(d.join("snap/effort.bin"))
            .expect("m")
            .len();
        let idx = std::fs::metadata(d.join("snap/effort-index.json"))
            .expect("m")
            .len();
        println!("effort.bin {size} bytes, effort-index.json {idx} bytes");
        for (name, b) in [
            ("department (1 x 1 deg)", (1.0, 43.0, 2.0, 44.0)),
            ("Paris region (0.5 x 0.3 deg)", (2.1, 48.7, 2.6, 49.0)),
            ("France", (-5.0, 42.0, 9.0, 51.5)),
        ] {
            let t = std::time::Instant::now();
            let v = snap.effort("Aves", b).expect("r").expect("g");
            let dt = t.elapsed();
            // Baseline: read the class's whole array and filter.
            let t2 = std::time::Instant::now();
            let all = snap
                .effort("Aves", (-180.0, -90.0, 180.0, 90.0))
                .expect("r")
                .expect("g");
            let scan: usize = all
                .iter()
                .filter(|c| c.0 >= b.0 && c.0 <= b.2 && c.1 >= b.1 && c.1 <= b.3)
                .count();
            println!(
                "{name}: {} cells in {dt:?} (binary search); full scan {scan} cells in {:?}",
                v.len(),
                t2.elapsed()
            );
        }
        if let Ok(st) = std::fs::read_to_string("/proc/self/status") {
            for l in st.lines().filter(|l| l.starts_with("VmHWM")) {
                println!("{l}");
            }
        }
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn a_cancelled_build_leaves_no_snapshot() {
        let d = tmp("cancel");
        let zip = d.join("region.zip");
        let rows: Vec<String> = (0..1_000_001)
            .map(|i| {
                format!("{i}\tds\t43.6\t1.4\t10\t2015\t111\t111\tHUMAN_OBSERVATION\tCC0_1_0\t")
            })
            .collect();
        archive(&zip, &rows.iter().map(String::as_str).collect::<Vec<_>>());
        let r = build(
            &zip,
            &crate::gbif_mirror_cells::FR_ES,
            info(),
            "t",
            &d.join("snap"),
            false,
            &|| true,
            &|_| {},
        );
        assert!(matches!(r, Err(DataError::Cancelled)));
        assert!(Snapshot::open(&d.join("snap")).is_err());
        let _ = std::fs::remove_dir_all(d);
    }

    fn square(w: f64, s: f64, e: f64, n: f64) -> serde_json::Value {
        serde_json::json!({"type": "Polygon", "coordinates": [[[w, s], [e, s], [e, n], [w, n], [w, s]]]})
    }

    #[test]
    fn coverage_of_study_areas() {
        let r = crate::gbif_mirror_cells::FR_ES;
        assert!(r.covers(&square(2.2, 48.8, 2.5, 48.9)), "Paris");
        assert!(r.covers(&square(-4.0, 40.0, -3.0, 41.0)), "Madrid");
        assert!(r.covers(&square(-16.9, 28.0, -16.1, 28.6)), "Tenerife");
        assert!(r.covers(&square(8.5, 41.5, 9.5, 42.9)), "Corsica");
        // Across the Pyrenees: Occitania's box takes in Andorra and part of Catalonia.
        assert!(r.covers(&square(-0.4, 42.3, 4.9, 45.1)), "Occitania box");
        assert!(!r.covers(&square(8.6, 50.0, 8.8, 50.2)), "Frankfurt");
        assert!(!r.covers(&square(-0.2, 51.4, 0.0, 51.6)), "London");
        assert!(
            !r.covers(&square(2.2, 48.8, 9.0, 50.5)),
            "Paris to Frankfurt"
        );
        // A box over Brittany and the open sea around it; not one reaching Cornwall.
        assert!(
            r.covers(&square(-6.0, 47.0, -1.0, 49.0)),
            "Brittany with sea"
        );
        assert!(
            !r.covers(&square(-6.0, 47.0, -1.0, 50.5)),
            "reaches the Channel Islands"
        );
        assert!(!r.covers(&serde_json::json!({"type": "Point", "coordinates": [2.3, 48.8]})));
        let multi = serde_json::json!({"type": "MultiPolygon", "coordinates": [
            [[[2.2, 48.8], [2.5, 48.8], [2.5, 48.9], [2.2, 48.8]]],
            [[[-3.8, 40.3], [-3.6, 40.3], [-3.6, 40.5], [-3.8, 40.3]]]]});
        assert!(r.covers(&multi));
    }

    /// MULTIPOLYGON WKT → GeoJSON (test helper).
    fn wkt_json(wkt: &str) -> serde_json::Value {
        let body = wkt
            .trim_start_matches("MULTIPOLYGON(")
            .trim_end_matches(')');
        let ring = |r: &str| -> serde_json::Value {
            r.trim_matches(|c| c == '(' || c == ')')
                .split(',')
                .map(|pt| {
                    let v: Vec<f64> = pt
                        .split_whitespace()
                        .map(|x| x.parse().expect("num"))
                        .collect();
                    serde_json::json!([v[0], v[1]])
                })
                .collect()
        };
        let polys: Vec<serde_json::Value> = body
            .split(")),((")
            .map(|p| {
                p.trim_matches(|c| c == '(' || c == ')')
                    .split("),(")
                    .map(ring)
                    .collect()
            })
            .collect();
        serde_json::json!({"type": "MultiPolygon", "coordinates": polys})
    }

    #[test]
    fn region_wkt_is_counter_clockwise_and_covers_every_cell() {
        let r = crate::gbif_mirror_cells::FR_ES;
        let wkt = r.wkt();
        assert!(wkt.starts_with("MULTIPOLYGON((("));
        // The first exterior ring is counter-clockwise (positive shoelace area).
        let first = &wkt["MULTIPOLYGON(((".len()..];
        let pts: Vec<(f64, f64)> = first
            .split(')')
            .next()
            .expect("ring")
            .split(',')
            .map(|p| {
                let mut it = p.split_whitespace().map(|v| v.parse::<f64>().expect("num"));
                (it.next().expect("x"), it.next().expect("y"))
            })
            .collect();
        let area2: f64 = pts
            .windows(2)
            .map(|w| w[0].0 * w[1].1 - w[1].0 * w[0].1)
            .sum();
        assert!(area2 > 0.0, "counter-clockwise ring");
        // Every region cell's centre is inside the geometry sent to GBIF.
        let polys = polygons(&wkt_json(wkt)).expect("parse");
        for &(i, j) in r.cells {
            let c = ((f64::from(i) + 0.5) * r.step, (f64::from(j) + 0.5) * r.step);
            assert!(
                polys.iter().any(|p| inside(p, c)),
                "cell {i},{j} outside the WKT"
            );
        }
        let p = r.predicate("someone");
        assert_eq!(p["predicate"]["predicates"][0]["type"], "within");
        assert!(p.to_string().contains("HAS_GEOSPATIAL_ISSUE"));
        assert!(!p.to_string().contains("TAXON_KEY"));
    }
}
