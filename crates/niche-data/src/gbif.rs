//! GBIF species lookup and occurrence retrieval (public search API, no credentials).
//!
//! The search API caps paging at 100 000 records and returns records in an order that is not
//! spatially random. When more records exist than `max_records`, NicheRS takes a spatially
//! stratified subsample: the bounding box is split (quadtree, count-only queries) into cells
//! with ≤ `LEAF_TARGET` records and each cell contributes in proportion to its count. This keeps
//! the geographic spread of the data instead of the first N records of the API order. Production
//! reports should use a GBIF download with a DOI (docs/research/06-biodiversity-data.md).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::DataError;
use crate::http::Http;

const API: &str = "https://api.gbif.org/v1";
/// GBIF Backbone Taxonomy dataset key; pinned so keys stay interpretable (ADR-0006).
pub const BACKBONE: &str = "d7dddbf4-2cf0-4f39-9b2a-bb099caae36c";
const PAGE: usize = 300;

/// (west, south, east, north) in degrees.
type BBox = (f64, f64, f64, f64);
const LEAF_TARGET: u64 = 6_000;
const MAX_DEPTH: u32 = 5;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SpeciesHit {
    pub key: u64,
    pub scientific_name: String,
    pub canonical_name: String,
    pub rank: String,
    pub status: String,
    pub kingdom: Option<String>,
    pub family: Option<String>,
    pub vernacular_name: Option<String>,
}

fn s(v: &serde_json::Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(String::from)
}

pub fn search_species(http: &Http, q: &str) -> Result<Vec<SpeciesHit>, DataError> {
    let url = format!(
        "{API}/species/suggest?datasetKey={BACKBONE}&limit=12&q={}",
        encode(q)
    );
    let v = http.get_json(&url)?;
    let arr = v.as_array().ok_or_else(|| DataError::Format {
        url: url.clone(),
        msg: "expected array".into(),
    })?;
    Ok(arr
        .iter()
        .filter_map(|h| {
            Some(SpeciesHit {
                key: h.get("key")?.as_u64()?,
                scientific_name: s(h, "scientificName")?,
                canonical_name: s(h, "canonicalName").unwrap_or_default(),
                rank: s(h, "rank").unwrap_or_default(),
                status: s(h, "status").unwrap_or_default(),
                kingdom: s(h, "kingdom"),
                family: s(h, "family"),
                vernacular_name: s(h, "vernacularName"),
            })
        })
        .collect())
}

/// What NicheRS reads from the GBIF species API for one backbone key
/// (`/v1/species/{key}`). One request serves [`taxon_rank`], [`taxon_class`] and
/// [`species_name`]; answers are memoised in the process (see [`taxon_info`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TaxonInfo {
    pub key: u64,
    /// e.g. "SPECIES", "GENUS".
    pub rank: Option<String>,
    pub scientific_name: Option<String>,
    pub kingdom: Option<String>,
    /// Taxonomic class name (e.g. "Aves", "Magnoliopsida"); `None` for taxa above class or
    /// without a class in the backbone (some fungi, incertae sedis).
    pub class: Option<String>,
}

impl TaxonInfo {
    fn from_json(key: u64, v: &serde_json::Value) -> Self {
        Self {
            key,
            rank: s(v, "rank"),
            scientific_name: s(v, "scientificName"),
            kingdom: s(v, "kingdom"),
            class: s(v, "class").filter(|c| !c.trim().is_empty()),
        }
    }
}

/// Memoised species-API answers expire after this time, so a long-running server picks up
/// backbone changes (they are rare: the backbone is republished a few times a year).
const TAXON_TTL: std::time::Duration = std::time::Duration::from_secs(24 * 3600);
/// Bound on memoised taxa (a study uses tens; cleared when full).
const TAXON_MEMO_MAX: usize = 50_000;

type TaxonMemo = std::sync::Mutex<std::collections::HashMap<u64, (std::time::Instant, TaxonInfo)>>;

fn taxon_memo() -> &'static TaxonMemo {
    static MEMO: std::sync::OnceLock<TaxonMemo> = std::sync::OnceLock::new();
    MEMO.get_or_init(Default::default)
}

/// Species-API record of a backbone key, memoised in the process for 24 h (errors are not
/// memoised). [`taxon_rank`], [`taxon_class`] and [`species_name`] share it, so asking for the
/// rank and the class of a species costs one request.
pub fn taxon_info(http: &Http, key: u64) -> Result<TaxonInfo, DataError> {
    if let Ok(m) = taxon_memo().lock()
        && let Some((at, info)) = m.get(&key)
        && at.elapsed() < TAXON_TTL
    {
        return Ok(info.clone());
    }
    let url = format!("{API}/species/{key}");
    let v = http.get_json(&url)?;
    let info = TaxonInfo::from_json(key, &v);
    if let Ok(mut m) = taxon_memo().lock() {
        if m.len() >= TAXON_MEMO_MAX {
            m.clear();
        }
        m.insert(key, (std::time::Instant::now(), info.clone()));
    }
    Ok(info)
}

/// Taxonomic rank of a backbone key (e.g. "SPECIES", "GENUS").
pub fn taxon_rank(http: &Http, key: u64) -> Result<String, DataError> {
    taxon_info(http, key)?.rank.ok_or(DataError::Format {
        url: format!("{API}/species/{key}"),
        msg: "no rank".into(),
    })
}

/// Taxonomic class name of a backbone key (e.g. "Aves"), used to pick a target group for
/// sampling-effort grids ([`crate::gbif_mirror::Snapshot::effort`]). `Ok(None)` when the
/// backbone gives no class (taxa above class, some fungi). Shares [`taxon_info`]'s request.
pub fn taxon_class(http: &Http, key: u64) -> Result<Option<String>, DataError> {
    Ok(taxon_info(http, key)?.class)
}

/// Answer of the GBIF name matcher (`/species/match`) for one verbatim name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NameMatch {
    /// Backbone key to model: the accepted taxon when the name is a synonym.
    pub key: Option<u64>,
    pub scientific_name: Option<String>,
    pub rank: Option<String>,
    /// Taxonomic status of the matched name (`ACCEPTED`, `SYNONYM`, `DOUBTFUL`…).
    pub status: Option<String>,
    /// `EXACT`, `FUZZY`, `HIGHERRANK` or `NONE`.
    pub match_type: String,
    pub confidence: Option<u64>,
}

impl NameMatch {
    /// Parse a `/species/match` answer. A synonym resolves to its accepted name (the key whose
    /// GBIF records are modelled).
    pub fn from_json(v: &serde_json::Value) -> Self {
        let match_type = s(v, "matchType").unwrap_or_else(|| "NONE".into());
        let synonym = v.get("synonym").and_then(serde_json::Value::as_bool) == Some(true);
        let key = if synonym {
            v.get("acceptedUsageKey")
                .and_then(serde_json::Value::as_u64)
        } else {
            v.get("usageKey").and_then(serde_json::Value::as_u64)
        };
        let scientific_name = if synonym {
            s(v, "acceptedScientificName").or_else(|| s(v, "scientificName"))
        } else {
            s(v, "scientificName")
        };
        Self {
            key: key.filter(|_| match_type != "NONE"),
            scientific_name: scientific_name.filter(|_| match_type != "NONE"),
            rank: s(v, "rank"),
            status: s(v, "status"),
            match_type,
            confidence: v.get("confidence").and_then(serde_json::Value::as_u64),
        }
    }
}

/// Match one verbatim name to the GBIF backbone (`/species/match`, strict = false). Only the name
/// is sent.
pub fn match_name(http: &Http, name: &str) -> Result<NameMatch, DataError> {
    let url = format!(
        "{API}/species/match?datasetKey={BACKBONE}&strict=false&name={}",
        encode(name)
    );
    Ok(NameMatch::from_json(&http.get_json(&url)?))
}

pub fn species_name(http: &Http, key: u64) -> Result<String, DataError> {
    taxon_info(http, key)?
        .scientific_name
        .ok_or(DataError::Format {
            url: format!("{API}/species/{key}"),
            msg: "no scientificName".into(),
        })
}

/// One raw occurrence as returned by GBIF (fields NicheRS uses).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Occurrence {
    pub key: u64,
    pub lng: f64,
    pub lat: f64,
    pub year: Option<i32>,
    pub uncertainty_m: Option<f64>,
    pub licence: String,
    pub basis_of_record: String,
    pub dataset_key: String,
    /// The publisher generalised (blurred) the coordinates or withheld information
    /// (Darwin Core `dataGeneralizations` / `informationWithheld` non-empty). `None` when the
    /// source does not report these fields (SIMPLE_CSV downloads and the regional mirror built
    /// from them): unknown, never "not generalised". See [`generalisation_flags_available`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generalised: Option<bool>,
}

/// Where occurrence records come from, for [`generalisation_flags_available`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordSource {
    /// Public occurrence search API ([`fetch_occurrences`]).
    SearchApi,
    /// SIMPLE_CSV batch download ([`crate::gbif_download`]).
    Download,
    /// Regional mirror built from a SIMPLE_CSV download ([`crate::gbif_mirror`]).
    Mirror,
}

/// Whether records from `source` carry [`Occurrence::generalised`]. GBIF's SIMPLE_CSV format has
/// no `dataGeneralizations` / `informationWithheld` columns, so downloads and the mirror cannot
/// tell generalised records apart (the design notes §A2).
pub fn generalisation_flags_available(source: RecordSource) -> bool {
    matches!(source, RecordSource::SearchApi)
}

/// `Some(true)` when the search-API record has a non-blank `dataGeneralizations` or
/// `informationWithheld` (free text: only its presence is used), else `Some(false)`.
fn generalised_flag(r: &serde_json::Value) -> Option<bool> {
    let set = |k: &str| {
        r.get(k)
            .and_then(|x| x.as_str())
            .is_some_and(|t| !t.trim().is_empty())
    };
    Some(set("dataGeneralizations") || set("informationWithheld"))
}

/// One search-API result as an [`Occurrence`] (`None` without key or coordinates).
fn parse_occurrence(r: &serde_json::Value) -> Option<Occurrence> {
    Some(Occurrence {
        key: r.get("key")?.as_u64()?,
        lng: r.get("decimalLongitude")?.as_f64()?,
        lat: r.get("decimalLatitude")?.as_f64()?,
        year: r.get("year").and_then(|y| y.as_i64()).map(|y| y as i32),
        uncertainty_m: r
            .get("coordinateUncertaintyInMeters")
            .and_then(serde_json::Value::as_f64),
        licence: licence_code(&s(r, "license").unwrap_or_default()).to_string(),
        basis_of_record: s(r, "basisOfRecord").unwrap_or_default(),
        dataset_key: s(r, "datasetKey").unwrap_or_default(),
        generalised: generalised_flag(r),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OccurrenceFetch {
    /// RFC 3339 time the records were retrieved from GBIF (kept when served from cache).
    #[serde(default)]
    pub accessed_at: String,
    /// True when this result was served from the local cache.
    #[serde(default)]
    pub from_cache: bool,
    /// DOI of the GBIF download the records come from (batch downloads only).
    #[serde(default)]
    pub doi: Option<String>,
    pub records: Vec<Occurrence>,
    pub total_matching: u64,
    pub truncated: bool,
    pub query_url: String,
    pub requests: usize,
}

/// Normalize a GBIF licence URL to `CC0_1_0` / `CC_BY_4_0` / `CC_BY_NC_4_0` / `OTHER`.
pub fn licence_code(url: &str) -> &'static str {
    let u = url.to_ascii_lowercase();
    if u.contains("publicdomain/zero") || u.contains("cc0") {
        "CC0_1_0"
    } else if u.contains("by-nc") {
        "CC_BY_NC_4_0"
    } else if u.contains("licenses/by/") {
        "CC_BY_4_0"
    } else {
        "OTHER"
    }
}

fn bbox_query(taxon: u64, b: (f64, f64, f64, f64)) -> String {
    let (w, s, e, n) = b;
    let wkt = format!("POLYGON(({w} {s},{e} {s},{e} {n},{w} {n},{w} {s}))");
    format!(
        "{API}/occurrence/search?taxonKey={taxon}&hasCoordinate=true&hasGeospatialIssue=false&occurrenceStatus=PRESENT&geometry={}",
        encode(&wkt)
    )
}

fn count(http: &Http, taxon: u64, b: (f64, f64, f64, f64)) -> Result<u64, DataError> {
    let url = format!("{}&limit=0", bbox_query(taxon, b));
    let v = http.get_json(&url)?;
    v.get("count")
        .and_then(|c| c.as_u64())
        .ok_or(DataError::Format {
            url,
            msg: "no count".into(),
        })
}

fn page(
    http: &Http,
    taxon: u64,
    b: (f64, f64, f64, f64),
    offset: usize,
    limit: usize,
) -> Result<Vec<Occurrence>, DataError> {
    let url = format!("{}&limit={limit}&offset={offset}", bbox_query(taxon, b));
    let v = http.get_json(&url)?;
    let res = v
        .get("results")
        .and_then(|r| r.as_array())
        .ok_or_else(|| DataError::Format {
            url: url.clone(),
            msg: "no results".into(),
        })?;
    Ok(res.iter().filter_map(parse_occurrence).collect())
}

/// Fetch occurrences of `taxon` in bbox (west, south, east, north), at most ~`max_records`.
pub fn fetch_occurrences(
    http: &Http,
    taxon: u64,
    bbox: (f64, f64, f64, f64),
    max_records: usize,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<OccurrenceFetch, DataError> {
    let query_url = bbox_query(taxon, bbox);
    let total = count(http, taxon, bbox)?;
    let mut requests = 1;
    // Leaves of a count-only quadtree.
    let mut leaves: Vec<((f64, f64, f64, f64), u64)> = Vec::new();
    let mut stack = vec![(bbox, total, 0u32)];
    let needs_split = total as usize > max_records;
    while let Some((b, c, depth)) = stack.pop() {
        if cancelled() {
            return Err(DataError::Cancelled);
        }
        if c == 0 {
            continue;
        }
        if !needs_split || c <= LEAF_TARGET || depth >= MAX_DEPTH {
            leaves.push((b, c));
            continue;
        }
        let (w, s, e, n) = b;
        let (mx, my) = ((w + e) / 2.0, (s + n) / 2.0);
        for q in [
            (w, s, mx, my),
            (mx, s, e, my),
            (w, my, mx, n),
            (mx, my, e, n),
        ] {
            let qc = count(http, taxon, q)?;
            requests += 1;
            stack.push((q, qc, depth + 1));
        }
    }
    // Quota per leaf proportional to its count (all records when under the cap).
    let fraction = if total as usize > max_records {
        max_records as f64 / total as f64
    } else {
        1.0
    };
    let mut jobs: Vec<(BBox, usize, usize)> = Vec::new(); // (bbox, offset, limit)
    for (b, c) in &leaves {
        let quota = ((*c as f64 * fraction).ceil() as usize)
            .min(*c as usize)
            .min(100_000);
        let mut off = 0;
        while off < quota {
            let lim = PAGE.min(quota - off);
            jobs.push((*b, off, lim));
            off += lim;
        }
    }
    requests += jobs.len();
    // Two connections at most: the public API rate-limits (HTTP 429) aggressive clients.
    let results: Vec<Result<Vec<Occurrence>, DataError>> = std::thread::scope(|scope| {
        let chunks: Vec<_> = jobs.chunks(jobs.len().div_ceil(2).max(1)).collect();
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || {
                    let mut out = Vec::new();
                    for &(b, off, lim) in chunk {
                        if cancelled() {
                            return Err(DataError::Cancelled);
                        }
                        out.extend(page(http, taxon, b, off, lim)?);
                    }
                    Ok(out)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or(Err(DataError::Cancelled)))
            .collect()
    });
    let mut by_key: BTreeMap<u64, Occurrence> = BTreeMap::new();
    for r in results {
        for o in r? {
            by_key.insert(o.key, o);
        }
    }
    Ok(OccurrenceFetch {
        accessed_at: String::new(),
        from_cache: false,
        doi: None,
        truncated: (total as usize) > by_key.len(),
        records: by_key.into_values().collect(),
        total_matching: total,
        query_url,
        requests,
    })
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Version of the search-API cache entries. Bumped to 2 when [`Occurrence::generalised`] was
/// added: entries written before (version 1, no version in the file name) lack the flag and are
/// ignored and fetched again, never read as "not generalised".
pub const SEARCH_CACHE_VERSION: u32 = 2;

fn search_cache_file(
    cache_dir: &std::path::Path,
    taxon: u64,
    bbox: BBox,
    max_records: usize,
) -> std::path::PathBuf {
    let (w, s, e, n) = bbox;
    cache_dir.join("gbif").join(format!(
        "{taxon}_{w:.5}_{s:.5}_{e:.5}_{n:.5}_{max_records}.v{SEARCH_CACHE_VERSION}.json"
    ))
}

/// [`fetch_occurrences`] with an on-disk cache keyed by (taxon, bbox, cap). Entries older than
/// `max_age_days` are refreshed. The data review step and the job then use identical records,
/// and re-running a study does not re-download (the access date is preserved for provenance).
#[allow(clippy::too_many_arguments)]
pub fn fetch_occurrences_cached(
    http: &Http,
    cache_dir: &std::path::Path,
    taxon: u64,
    bbox: BBox,
    max_records: usize,
    max_age_days: u64,
    cancelled: &(dyn Fn() -> bool + Sync),
    now_rfc3339: &str,
) -> Result<OccurrenceFetch, DataError> {
    let file = search_cache_file(cache_dir, taxon, bbox, max_records);
    let fresh = std::fs::metadata(&file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age.as_secs() < max_age_days * 86_400);
    if fresh
        && let Some(mut f) = std::fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str::<OccurrenceFetch>(&t).ok())
    {
        f.from_cache = true;
        return Ok(f);
    }
    let mut f = fetch_occurrences(http, taxon, bbox, max_records, cancelled)?;
    f.accessed_at = now_rfc3339.to_string();
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d).map_err(|e| DataError::Cache(e.to_string()))?;
    }
    let tmp = file.with_extension("tmp");
    let body = serde_json::to_string(&f).map_err(|e| DataError::Cache(e.to_string()))?;
    std::fs::write(&tmp, body)
        .and_then(|()| std::fs::rename(&tmp, &file))
        .map_err(|e| DataError::Cache(e.to_string()))?;
    Ok(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_match_resolves_synonyms_and_refusals() {
        let exact = serde_json::json!({"usageKey": 2441370, "scientificName": "Testudo hermanni Gmelin, 1789",
            "rank": "SPECIES", "status": "ACCEPTED", "matchType": "EXACT", "confidence": 99});
        let m = NameMatch::from_json(&exact);
        assert_eq!((m.key, m.match_type.as_str()), (Some(2441370), "EXACT"));
        let syn = serde_json::json!({"usageKey": 1, "acceptedUsageKey": 2, "synonym": true,
            "scientificName": "Old name", "acceptedScientificName": "New name", "rank": "SPECIES",
            "status": "SYNONYM", "matchType": "EXACT"});
        let m = NameMatch::from_json(&syn);
        assert_eq!(
            (m.key, m.scientific_name.as_deref()),
            (Some(2), Some("New name"))
        );
        let none = serde_json::json!({"matchType": "NONE", "confidence": 100, "synonym": false});
        let m = NameMatch::from_json(&none);
        assert_eq!((m.key, m.match_type.as_str()), (None, "NONE"));
    }

    #[test]
    fn licences() {
        assert_eq!(
            licence_code("http://creativecommons.org/licenses/by-nc/4.0/legalcode"),
            "CC_BY_NC_4_0"
        );
        assert_eq!(
            licence_code("http://creativecommons.org/licenses/by/4.0/legalcode"),
            "CC_BY_4_0"
        );
        assert_eq!(
            licence_code("http://creativecommons.org/publicdomain/zero/1.0/legalcode"),
            "CC0_1_0"
        );
    }

    #[test]
    fn generalisation_flags_from_search_results() {
        let base = serde_json::json!({
            "key": 7, "decimalLongitude": 1.5, "decimalLatitude": 43.2,
            "license": "http://creativecommons.org/licenses/by/4.0/legalcode",
            "basisOfRecord": "HUMAN_OBSERVATION", "datasetKey": "ds"
        });
        let with = |k: &str, v: serde_json::Value| {
            let mut r = base.clone();
            r[k] = v;
            parse_occurrence(&r).expect("record")
        };
        assert_eq!(parse_occurrence(&base).expect("r").generalised, Some(false));
        assert_eq!(
            with(
                "informationWithheld",
                "Coordinate uncertainty increased to 27505m to protect threatened taxon".into()
            )
            .generalised,
            Some(true)
        );
        assert_eq!(
            with(
                "dataGeneralizations",
                "coord = floor(coord * 100) / 100".into()
            )
            .generalised,
            Some(true)
        );
        assert_eq!(
            with("dataGeneralizations", "  ".into()).generalised,
            Some(false)
        );
        assert_eq!(
            with("informationWithheld", serde_json::Value::Null).generalised,
            Some(false)
        );
        assert!(generalisation_flags_available(RecordSource::SearchApi));
        assert!(!generalisation_flags_available(RecordSource::Download));
        assert!(!generalisation_flags_available(RecordSource::Mirror));
    }

    /// A cache entry written before the flag existed (no version in its name, no field) is
    /// never served as "not generalised": the versioned name no longer matches it.
    #[test]
    fn old_search_cache_entries_are_not_reused() {
        let dir = std::path::Path::new("/cache");
        let bbox = (1.0, 43.0, 2.0, 44.0);
        let new = search_cache_file(dir, 5, bbox, 100);
        let old = dir
            .join("gbif")
            .join("5_1.00000_43.00000_2.00000_44.00000_100.json");
        assert_ne!(new, old);
        assert!(new.to_string_lossy().ends_with("_100.v2.json"));
        // Records without the field still deserialize, as unknown.
        let o: Occurrence = serde_json::from_str(
            r#"{"key":1,"lng":1.0,"lat":2.0,"year":null,"uncertainty_m":null,"licence":"CC0_1_0","basis_of_record":"X","dataset_key":"d"}"#,
        )
        .expect("old record");
        assert_eq!(o.generalised, None);
        // Unknown is not written out (outputs of SIMPLE_CSV paths are unchanged).
        assert!(
            !serde_json::to_string(&o)
                .expect("json")
                .contains("generalised")
        );
    }

    #[test]
    fn taxon_info_reads_class() {
        let v = serde_json::json!({"key": 2481433, "rank": "SPECIES", "kingdom": "Animalia",
            "class": "Aves", "scientificName": "Columba palumbus Linnaeus, 1758"});
        let t = TaxonInfo::from_json(2481433, &v);
        assert_eq!(t.class.as_deref(), Some("Aves"));
        assert_eq!(t.rank.as_deref(), Some("SPECIES"));
        assert_eq!(t.kingdom.as_deref(), Some("Animalia"));
        let k = TaxonInfo::from_json(
            1,
            &serde_json::json!({"rank": "KINGDOM", "kingdom": "Fungi"}),
        );
        assert_eq!(k.class, None);
    }

    #[test]
    fn url_encoding() {
        assert_eq!(encode("POLYGON((1 2))"), "POLYGON%28%281%202%29%29");
    }
}
