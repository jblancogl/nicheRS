//! API data transfer objects. Authoritative contract; mirrored in `web/src/api/types.ts`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Blocking,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Warning {
    pub code: String,
    pub severity: Severity,
    /// English text (reports, logs, older clients).
    pub message: String,
    /// The values in `message`, so that interfaces can word it in their own language by `code`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, serde_json::Value>,
}

impl Warning {
    pub fn new(code: &str, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            params: BTreeMap::new(),
        }
    }

    pub fn with(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayerInfo {
    pub id: String,
    pub name: String,
    pub short_name: String,
    pub unit: String,
    pub source: String,
    pub licence: String,
    pub default_selected: bool,
    pub caution: Option<String>,
    /// Predictor group for display: `climate`, `terrain`, `land_cover` or `water` (ADR-0015).
    pub group: String,
    /// Notice that must accompany any output using the layer (Copernicus DEM, ESA WorldCover);
    /// `None` when the licence requires none (CHELSA, CC0).
    pub attribution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudyArea {
    pub name: String,
    pub geometry: serde_json::Value,
    /// Administrative units the area was built from (empty when drawn or uploaded). Kept for
    /// traceability; omitted when empty, so existing specifications keep their hash.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub places: Vec<PlaceRef>,
}

/// One administrative unit chosen as (part of) a study area.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaceRef {
    pub id: String,
    pub name: String,
    /// Boundary pack it comes from, e.g. `fr-es-2026.1`.
    pub pack: String,
    /// Attribution required by the boundary source.
    pub attribution: String,
}

/// A place found by `GET /api/v1/places` (administrative unit of an installed boundary pack).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaceHit {
    pub id: String,
    pub name: String,
    /// `country`, `region`, `department`, `intercommunality`, `municipality` or `protected`.
    pub level: String,
    /// Designation of a protected area in the source language ("Parc naturel régional",
    /// "Parque Natural", "Natura 2000 · ZPS").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designation: Option<String>,
    pub country: String,
    /// Enclosing units, nearest first (e.g. ["Drôme", "Auvergne-Rhône-Alpes", "France"]).
    pub within: Vec<String>,
    pub bbox: [f64; 4],
    pub area_km2: f64,
    pub pack: String,
    pub attribution: String,
}

/// `GET /api/v1/places/{id}`: a place and its boundary (GeoJSON MultiPolygon).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaceGeometry {
    pub place: PlaceHit,
    pub geometry: serde_json::Value,
}

/// A queued study waiting for its GBIF download (`GET /api/v1/jobs/{id}/gbif-wait`). GBIF
/// prepares the archive in its own queue; NicheRS checks it every 10 s, so `checked_at` shows
/// the study is alive however long GBIF takes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GbifWait {
    /// GBIF download key, once the request is accepted.
    pub key: Option<String>,
    /// `requesting`, then GBIF's status in lower case (`preparing`, `running`…), then
    /// `downloading` once the archive is ready.
    pub status: String,
    /// When the wait for this download started (RFC 3339).
    pub since: String,
    /// Last check with GBIF (RFC 3339).
    pub checked_at: String,
    /// GBIF's own page for the download.
    pub url: Option<String>,
    /// From this many seconds of waiting the interface says GBIF is slower than usual.
    pub slow_after_s: u64,
    /// From this many seconds the study may continue without the download (search API).
    pub skip_after_s: u64,
    /// Records per species the search API retrieves (limit of that fallback).
    pub search_api_max_records: usize,
}

/// State of the instance for administrators (`GET /api/v1/admin/system`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemStatus {
    pub version: String,
    /// Commit the binary was built from (filled by the server).
    pub build: String,
    pub started_at: String,
    pub data_dir: String,
    /// Size and free space of the volume holding the data directory (Unix only).
    pub disk: Option<DiskSpace>,
    pub storage: StorageUse,
    pub jobs: JobCounts,
    pub boundaries: Vec<PackStatus>,
    pub db_backups: DbBackups,
    /// How the whole volume is backed up, as declared by the deployment (`NICHE_VOLUME_BACKUP`).
    pub volume_backup: Option<String>,
    pub gbif: GbifAccount,
    /// Regional GBIF mirror (ADR-0114); `None` when the server has none configured.
    pub gbif_mirror: Option<MirrorStatus>,
    pub retention_days: u64,
}

/// State of the regional GBIF mirror (administration).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MirrorStatus {
    pub region: String,
    pub name: String,
    /// `ready`, `requesting`, `waiting` (GBIF prepares the download), `downloading`,
    /// `building`, `error`, `disabled` (no shared GBIF account).
    pub state: String,
    /// The snapshot studies use now (it stays in use while a newer one is prepared).
    pub snapshot: Option<MirrorSnapshot>,
    /// Progress or error, in English (logs and the console).
    pub detail: Option<String>,
    pub updated_at: Option<String>,
    /// Days between refreshes; a snapshot older than `max_age_days` is not used.
    pub refresh_days: u64,
    pub max_age_days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MirrorSnapshot {
    pub download_key: String,
    pub doi: Option<String>,
    /// When GBIF created the download (the data are as of this date).
    pub created: String,
    pub built_at: String,
    pub records: u64,
    pub taxa: u64,
    pub bytes: u64,
    /// Whether studies use it (not older than `max_age_days`).
    pub usable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiskSpace {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StorageUse {
    pub database_bytes: u64,
    pub jobs_bytes: u64,
    pub cache_bytes: u64,
    pub boundaries_bytes: u64,
    pub backups_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct JobCounts {
    pub total: u64,
    pub queued: u64,
    /// Queued studies whose data (GBIF download) is being prepared.
    pub preparing: u64,
    pub running: u64,
    /// Failed in the last 7 days.
    pub failed_7d: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackStatus {
    pub name: String,
    pub units: usize,
}

/// Daily copies of the SQLite database kept in `<data_dir>/backups/` (accounts, studies' state).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DbBackups {
    pub last_at: Option<String>,
    pub count: usize,
    pub keep: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GbifAccount {
    /// Batch downloads (DOI) are configured.
    pub configured: bool,
    pub user: Option<String>,
    /// GBIF accepted the credentials at the last check (`None`: not checked or unreachable).
    pub ok: Option<bool>,
    pub detail: Option<String>,
    pub checked_at: Option<String>,
}

/// One organisation's studies in a month (`GET /api/v1/admin/metrics?month=`): what they cost
/// in computing time and GBIF waiting, and why they failed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrgMetrics {
    pub org_name: String,
    pub month: String,
    pub studies: u64,
    pub succeeded: u64,
    pub failed: u64,
    pub cancelled: u64,
    /// Species requested in the month's studies.
    pub species: u64,
    /// Total computing time (start to end of finished studies), seconds.
    pub compute_s: f64,
    /// Computing time per species of succeeded studies, seconds.
    pub compute_s_per_species: Option<f64>,
    /// Time spent waiting for GBIF downloads (data preparation), seconds.
    pub gbif_wait_s_mean: Option<f64>,
    pub gbif_wait_s_max: Option<f64>,
    /// Failed studies by cause (`gbif`, `climate`, `area`, `records`, `model`, `server`, `other`).
    pub failures: BTreeMap<String, u64>,
}

/// Why a study failed, from its error message (administration: group recurring problems).
pub fn failure_cause(error: &str) -> &'static str {
    let e = error.to_lowercase();
    if e.contains("every species failed") || e.contains("model fitting") {
        "model"
    } else if e.contains("gbif") {
        "gbif"
    } else if e.contains("study area") || e.contains("grid cells") || e.contains("overlap") {
        "area"
    } else if e.contains("chelsa") || e.contains("predictor") || e.contains("layer") {
        "climate"
    } else if e.contains("presence") || e.contains("record") || e.contains("occurrence") {
        "records"
    } else if e.contains("storage") || e.contains("shutdown") || e.contains("interrupted") {
        "server"
    } else {
        "other"
    }
}

/// Whether a study area fits the modelling-grid limit at each offered resolution. The grid
/// covers the area's bounding box, so far-apart parts (islands, overseas territories) count.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AreaCheck {
    pub max_cells: usize,
    pub resolutions: Vec<ResolutionCheck>,
    /// Date (YYYY-MM-DD) of the regional GBIF mirror that serves this area's records without
    /// waiting for GBIF (ADR-0114); absent when studies here download from GBIF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gbif_mirror: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolutionCheck {
    /// Aggregation factor of the 30-arc-second grid (≈ km).
    pub factor: usize,
    /// Cells of the modelling grid over the area's bounding box (land and sea).
    pub cells: usize,
    pub fits: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaCheckRequest {
    pub area: StudyArea,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OccurrenceFilters {
    #[serde(default = "default_year_min")]
    pub year_min: i32,
    #[serde(default = "default_uncertainty")]
    pub max_uncertainty_m: f64,
    #[serde(default = "default_licences")]
    pub licences: Vec<String>,
    /// Distance thinning (opt-in, Science v1 phase 2): after the one-record-per-cell rule, keep
    /// presence cells at least this far apart (km, 0 < x ≤ 100; spThin, Aiello-Lammens et al.
    /// 2015). `None` = off; omitted from the JSON when off, so existing specifications keep
    /// their hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thin_km: Option<f64>,
}

fn default_year_min() -> i32 {
    1981
}
fn default_uncertainty() -> f64 {
    1000.0
}
fn default_licences() -> Vec<String> {
    vec!["CC0_1_0".into(), "CC_BY_4_0".into()]
}

impl Default for OccurrenceFilters {
    fn default() -> Self {
        Self {
            year_min: default_year_min(),
            max_uncertainty_m: default_uncertainty(),
            licences: default_licences(),
            thin_km: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GridInfo {
    pub bounds: [f64; 4],
    pub width: usize,
    pub height: usize,
    pub cell_size_deg: f64,
    pub cells_in_area: usize,
    pub area_km2: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExclusionCount {
    pub reason: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OccurrencePoint {
    pub lng: f64,
    pub lat: f64,
    pub year: Option<i32>,
    pub status: String,
    /// `gbif` or `own` (the organisation's data, ADR-0108); absent when the study uses GBIF
    /// only, so existing outputs are unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataPreviewRequest {
    pub species_key: u64,
    pub area: StudyArea,
    #[serde(default = "one")]
    pub resolution_factor: usize,
    #[serde(default)]
    pub filters: OccurrenceFilters,
    #[serde(default)]
    pub demo: bool,
    /// Seed of the study being prepared: distance thinning is drawn from it (same draw as the
    /// study run). Absent → the default study seed.
    #[serde(default = "default_seed")]
    pub seed: u64,
    /// The organisation's dataset used by this species (ADR-0108), with `own`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_dataset: Option<DatasetRef>,
    /// How the species uses that dataset (see [`SpeciesRef::own`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own: Option<OwnSpeciesData>,
}

fn one() -> usize {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataPreview {
    pub species_key: u64,
    pub scientific_name: String,
    pub source: String,
    pub query_url: Option<String>,
    pub records_total_matching: u64,
    pub records_fetched: usize,
    pub truncated: bool,
    pub retained_cells: usize,
    pub exclusions: Vec<ExclusionCount>,
    pub licences: BTreeMap<String, usize>,
    pub points: Vec<OccurrencePoint>,
    pub grid: GridInfo,
    pub warnings: Vec<Warning>,
    /// Records per source when the organisation's data are used (ADR-0108).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_source: Vec<SourceCount>,
    /// Independent presence–absence sites of the organisation's data, before the model exists
    /// (counts only; metrics come with the results).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_sites: Option<ValidationSites>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelSettings {
    #[serde(default = "auto")]
    pub tuning: String,
    #[serde(default = "default_fc")]
    pub feature_classes: Vec<String>,
    #[serde(default = "default_betas")]
    pub beta_multipliers: Vec<f64>,
    #[serde(default = "default_bg")]
    pub background_points: usize,
    #[serde(default = "default_r")]
    pub collinearity_threshold: f64,
    /// `random` (default: uniform over land cells with predictor data, ADR-0009 §3) or
    /// `target_group` (cells with records of a target group in the regional mirror's
    /// sampling-effort grid, Phillips et al. 2009; ADR-0013). Omitted from the JSON when
    /// `random`, so existing specifications keep their hash.
    #[serde(default = "random", skip_serializing_if = "is_random")]
    pub background_method: String,
    /// `target_group` only: GBIF class whose records form the target group (e.g. `Aves`);
    /// `None` = the species' own class (GBIF backbone).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_group: Option<String>,
    /// Modelling algorithm (ADR-0017): `maxent` (default, ADR-0002), `rf` (down-sampled random
    /// forest, Valavi et al. 2021) or `ensemble` (MaxEnt + down-sampled RF, mean of their
    /// percentile ranks over the background). Omitted from the JSON when `maxent`, so existing
    /// specifications keep their hash and give bit-identical results.
    #[serde(default = "maxent", skip_serializing_if = "is_maxent")]
    pub algorithm: String,
    /// `rf` / `ensemble` only: trees per forest; `None` = [`DEFAULT_RF_TREES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rf_trees: Option<usize>,
    /// `rf` / `ensemble` only: also evaluate `mtry = floor(p / 2)` besides the default
    /// `floor(sqrt(p))` under the same cross-validation, selected by the same sequential
    /// criteria (ADR-0017). Off by default (down-sampled RF is insensitive to tuning).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub rf_tune_mtry: bool,
}

/// Trees per random forest when `rf_trees` is not given (ADR-0017, Integration).
pub const DEFAULT_RF_TREES: usize = 500;

impl ModelSettings {
    /// Trees per forest actually used.
    pub fn rf_trees(&self) -> usize {
        self.rf_trees.unwrap_or(DEFAULT_RF_TREES)
    }
}

fn maxent() -> String {
    "maxent".into()
}
fn is_maxent(s: &str) -> bool {
    s == "maxent"
}

fn random() -> String {
    "random".into()
}
fn is_random(s: &str) -> bool {
    s == "random"
}

fn auto() -> String {
    "auto".into()
}
fn default_fc() -> Vec<String> {
    ["L", "LQ", "H", "LQH", "LQHP"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}
fn default_betas() -> Vec<f64> {
    vec![0.5, 1.0, 2.0, 3.0, 4.0]
}
fn default_bg() -> usize {
    10_000
}
fn default_r() -> f64 {
    0.7
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            tuning: auto(),
            feature_classes: default_fc(),
            beta_multipliers: default_betas(),
            background_points: default_bg(),
            collinearity_threshold: default_r(),
            background_method: random(),
            target_group: None,
            algorithm: maxent(),
            rf_trees: None,
            rf_tune_mtry: false,
        }
    }
}

/// Validation settings. Fields added by Science v1 phase 3 (ADR-0014) are optional, default to
/// "off" and are omitted from the JSON when off, so existing specifications keep their hash and
/// give bit-identical results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(from = "ValidationSettingsIn")]
pub struct ValidationSettings {
    /// `spatial_block` (default: 4 blocks at median latitude/longitude), `spatial_grid` (square
    /// blocks sized by spatial autocorrelation, ADR-0014) or `random_kfold`.
    pub method: String,
    /// Number of folds (`random_kfold`, `spatial_grid`; `spatial_block` always uses 4). When the
    /// field is absent: 5 for `spatial_grid`, 4 otherwise.
    pub folds: usize,
    /// `spatial_grid` only: block side in km; `None` = automatic (median autocorrelation range
    /// of the predictors, clamped; ADR-0014).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_km: Option<f64>,
    /// Null models for the selected setting (0 = off). ADR-0014.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub null_models: usize,
    /// Compute the area of applicability (`aoa.tif`). An evaluation of where the fitted model can
    /// be applied, not a model setting: it never changes the model, only adds a diagnostic layer.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub aoa: bool,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// Wire form of [`ValidationSettings`]: `folds` defaults by method.
#[derive(Deserialize)]
struct ValidationSettingsIn {
    #[serde(default = "spatial")]
    method: String,
    #[serde(default)]
    folds: Option<usize>,
    #[serde(default)]
    block_km: Option<f64>,
    #[serde(default)]
    null_models: usize,
    #[serde(default)]
    aoa: bool,
}

impl From<ValidationSettingsIn> for ValidationSettings {
    fn from(v: ValidationSettingsIn) -> Self {
        let folds = v
            .folds
            .unwrap_or(if v.method == "spatial_grid" { 5 } else { 4 });
        Self {
            method: v.method,
            folds,
            block_km: v.block_km,
            null_models: v.null_models,
            aoa: v.aoa,
        }
    }
}

fn spatial() -> String {
    "spatial_block".into()
}

impl Default for ValidationSettings {
    fn default() -> Self {
        Self {
            method: spatial(),
            folds: 4,
            block_km: None,
            null_models: 0,
            aoa: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeciesRef {
    pub key: u64,
    pub scientific_name: String,
    /// Records of the organisation's own dataset used for this species (ADR-0108). `None`
    /// (default) = GBIF only; omitted from the JSON, so existing specifications keep their hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own: Option<OwnSpeciesData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobSpec {
    #[serde(default)]
    pub name: String,
    pub species: Vec<SpeciesRef>,
    pub area: StudyArea,
    pub layers: Vec<String>,
    #[serde(default = "one")]
    pub resolution_factor: usize,
    #[serde(default)]
    pub filters: OccurrenceFilters,
    #[serde(default)]
    pub model: ModelSettings,
    #[serde(default)]
    pub validation: ValidationSettings,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default)]
    pub demo: bool,
    /// Future-climate projections of the final model(s) (opt-in, ADR-0016): CHELSA v2.1 CMIP6
    /// climatologies replace the climate predictors; terrain, land-cover and water predictors
    /// stay at present values. Empty = none; omitted from the JSON when empty, so existing
    /// specifications keep their hash and give bit-identical results.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projections: Vec<ProjectionSpec>,
    /// The organisation's own occurrence dataset used by some species (ADR-0108). `None`
    /// (default) = GBIF only; omitted from the JSON, so existing specifications keep their hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_dataset: Option<DatasetRef>,
}

/// One future scenario to project to (ADR-0016): a 30-year period and a shared socio-economic
/// pathway, under one or more general circulation models (all five are recommended: the spread
/// between them is part of the result).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionSpec {
    /// `2011-2040`, `2041-2070` or `2071-2100` (`niche_data::chelsa::FUTURE_PERIODS`).
    pub period: String,
    /// `ssp126`, `ssp370` or `ssp585` (`niche_data::chelsa::FUTURE_SSPS`).
    pub ssp: String,
    /// CMIP6 models (`niche_data::chelsa::FUTURE_GCMS`), at least one, no repeats.
    pub gcms: Vec<String>,
}

fn default_seed() -> u64 {
    20_261_015
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            "interrupted" => Self::Interrupted,
            _ => Self::Queued,
        }
    }
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpeciesRunStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceLevel {
    Insufficient,
    Exploratory,
    Indicative,
    Supported,
    /// Supported, and confirmed on independent presence–absence data (ADR-0009 §6; criteria in
    /// ADR-0108, proposed).
    Validated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeciesRunSummary {
    pub key: u64,
    pub scientific_name: String,
    pub status: SpeciesRunStatus,
    pub evidence_level: Option<EvidenceLevel>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub status: JobStatus,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub progress: f64,
    pub stage: String,
    pub species: Vec<SpeciesRunSummary>,
    pub error: Option<String>,
    pub spec: JobSpec,
    /// Project grouping this study, if any.
    #[serde(default)]
    pub project_id: Option<String>,
    /// Display name of the account that created it (hosted mode; `None` once erased).
    #[serde(default)]
    pub created_by_name: Option<String>,
}

/// A project: a named group of an organisation's studies (e.g. a client file).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: String,
    /// Studies in the project.
    pub studies: u64,
    /// Creation time of its most recent study.
    pub last_study_at: Option<String>,
}

/// `POST /api/v1/projects`, `PATCH /api/v1/projects/{id}`.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectRequest {
    pub name: String,
}

/// `PATCH /api/v1/jobs/{id}`: rename a study and/or move it (`project_id: null` = no project).
#[derive(Debug, Clone, Deserialize)]
pub struct JobUpdate {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub project_id: Option<Option<String>>,
}

/// A field that is present (even as `null`) is `Some`; an absent one stays `None` (`default`).
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobEvent {
    pub seq: i64,
    pub job_id: String,
    pub status: JobStatus,
    pub stage: String,
    pub progress: f64,
    pub species_key: Option<u64>,
    pub message: String,
    pub level: String,
    pub at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FoldMetrics {
    pub fold: usize,
    pub n_test: usize,
    pub cbi: Option<f64>,
    pub or10: f64,
    pub ormtp: f64,
    pub auc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TuningRow {
    pub feature_classes: String,
    pub beta_multiplier: f64,
    pub or10_mean: f64,
    pub cbi_mean: Option<f64>,
    pub auc_mean: f64,
    pub n_nonzero: usize,
    pub selected: bool,
    /// `rf` for random-forest rows (absent for MaxEnt rows), ADR-0017. RF rows leave
    /// `feature_classes` empty, `beta_multiplier` and `n_nonzero` at 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
    /// Random-forest rows: variables tried at each split.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtry: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThresholdArea {
    pub rule: String,
    pub label: String,
    pub threshold: f64,
    pub area_km2: f64,
    pub fraction_of_study_area: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Evidence {
    pub level: EvidenceLevel,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataSummary {
    pub presences: usize,
    pub background: usize,
    pub records_fetched: usize,
    pub exclusions: Vec<ExclusionCount>,
    pub source: String,
    pub query_url: Option<String>,
    /// Records generalised (blurred) by their publishers, ADR-0011. Absent for synthetic data
    /// and in results computed before the rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generalised: Option<GeneralisedRecords>,
    /// Records per source when the organisation's data are used (ADR-0108); empty otherwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_source: Vec<SourceCount>,
}

/// Coordinates generalised by the publisher (Darwin Core `dataGeneralizations` /
/// `informationWithheld`), ADR-0011.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneralisedRecords {
    /// The source reports the generalisation fields (GBIF search API: yes; SIMPLE_CSV
    /// downloads and the regional mirror: no, so generalised records cannot be detected).
    pub flags_available: bool,
    /// Records flagged as generalised.
    pub records: usize,
    /// Excluded by the `generalised` rule (precision not stated or > `max_uncertainty_m`).
    pub excluded: usize,
    /// Flagged records kept as presences (stated precision ≤ `max_uncertainty_m`).
    pub retained: usize,
}

/// How the background was drawn (present only when `target_group` was requested), ADR-0013.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackgroundSampling {
    /// Requested method (`target_group`).
    pub requested: String,
    /// Method actually used: `target_group`, or `random` after a fallback.
    pub method: String,
    /// Class of the target group (GBIF backbone), when known.
    pub target_group: Option<String>,
    /// Land cells with predictor data and at least one target-group record.
    pub eligible_cells: usize,
    /// Why the target group could not be used (fallback to random), in English.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    /// The effort surface is synthetic (demo).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub synthetic: bool,
}

/// Sampling-bias diagnostic from the target-group effort surface (ADR-0013). Descriptive: it
/// never changes the evidence level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SamplingBias {
    pub target_group: String,
    /// Target-group records in the study area's land cells with predictor data.
    pub records: u64,
    /// Cells with at least one record.
    pub cells_with_effort: usize,
    /// Share of the study-area cells with at least one record.
    pub share_of_area_with_effort: f64,
    /// Highest number of records in one cell (legend of the effort map).
    pub max_cell_records: f64,
    /// Share of the presence cells inside the cells with most records that make up about 10 %
    /// of the study area.
    pub presences_in_top10pct_effort_cells: f64,
    /// Share of the study area those cells cover: what the previous share would be if presences
    /// ignored effort (0.10, more when cells tie at the cut).
    pub expected: f64,
    /// The effort surface is synthetic (demo).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub synthetic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DroppedVariable {
    pub variable: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectedModel {
    /// MaxEnt setting (MaxEnt and the ensemble's MaxEnt member); empty / 0 for `rf`.
    pub feature_classes: String,
    pub beta_multiplier: f64,
    /// `rf` or `ensemble` (absent for MaxEnt, the default), ADR-0017.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
    /// Random forest (`rf`, `ensemble`): trees, selected `mtry` and the seed of every forest of
    /// the species (tree `i` uses a stream derived from this seed and `i`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rf_trees: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rf_mtry: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rf_seed: Option<u64>,
}

impl SelectedModel {
    /// Algorithm of the result: `maxent` when absent (results before ADR-0017).
    pub fn algorithm(&self) -> &str {
        self.algorithm.as_deref().unwrap_or("maxent")
    }
}

/// Cross-validated metrics of one member of an ensemble (selected setting, same folds).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemberValidation {
    /// `maxent` or `rf`.
    pub algorithm: String,
    pub cbi_mean: Option<f64>,
    pub or10_mean: f64,
    pub auc_mean: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationSummary {
    pub method: String,
    pub folds: Vec<FoldMetrics>,
    pub cbi_mean: Option<f64>,
    pub or10_mean: f64,
    pub ormtp_mean: f64,
    pub auc_mean: f64,
    /// Block design of `spatial_grid` validation (absent for the other methods).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocks: Option<BlockDesign>,
    /// Null-model significance of the selected setting (absent when not requested).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub null_model: Option<NullModelSummary>,
    /// Ensemble only: the members' own cross-validated metrics (the fields above are the
    /// ensemble's).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<MemberValidation>,
}

/// How the `spatial_grid` blocks were sized and how many folds were obtained (ADR-0014).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockDesign {
    /// Block side used, km.
    pub block_km: f64,
    /// `"user"` (given in the settings) or `"autocorrelation"` (median variogram range, clamped).
    pub chosen_by: String,
    /// Folds requested.
    pub k_requested: usize,
    /// Folds obtained (fewer when fewer blocks contain presences).
    pub k_achieved: usize,
    /// Autocorrelation analysis (automatic size only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autocorrelation: Option<AutocorrelationRange>,
}

/// Variogram ranges of the predictors used, on the background sample (blockCV
/// `cv_spatial_autocor`, Valavi et al. 2019).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutocorrelationRange {
    /// Median of the fitted ranges, km (before clamping); `None` if no variable could be fitted.
    pub median_km: Option<f64>,
    /// Band the block size is clamped to, km.
    pub min_km: f64,
    pub max_km: f64,
    /// Points of the variogram subsample and its maximum lag, km.
    pub n_points: usize,
    pub cutoff_km: f64,
    pub variables: Vec<VariableRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableRange {
    pub variable: String,
    /// Practical range, km; `None` when the variable could not be fitted (constant, no spatial
    /// structure).
    pub range_km: Option<f64>,
    /// `exponential` | `spherical` | `gaussian`.
    pub model: Option<String>,
    /// The range hit the search bound: a lower bound only.
    pub at_upper_bound: bool,
}

/// Observed metric against the metrics of models fitted to random locations (ADR-0014).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NullModelSummary {
    /// Null replicates requested.
    pub requested: usize,
    /// Null presence sets that were fitted (cancelled or failed fits are left out).
    pub completed: usize,
    /// Presences per null set (= presence cells of the species).
    pub presences: usize,
    /// How null presences were drawn: `"uniform"` over cells with predictor data.
    pub sampling: String,
    /// Mean held-out continuous Boyce index (higher is better).
    pub cbi: NullStat,
    /// Mean held-out 10th-percentile omission rate (lower is better).
    pub or10: NullStat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NullStat {
    pub observed: Option<f64>,
    /// Mean and sample SD of the null values; z = (observed − mean) / sd.
    pub mean: Option<f64>,
    pub sd: Option<f64>,
    pub z: Option<f64>,
    /// Empirical one-sided p = (1 + #nulls at least as good) / (n + 1).
    pub p_value: f64,
    /// Finite null values used.
    pub n: usize,
}

/// Area of applicability (Meyer & Pebesma 2021), ADR-0014.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AoaSummary {
    /// Dissimilarity-index threshold: cells with DI above it are outside the AOA.
    pub threshold: f64,
    /// Share of the study-area cells outside the AOA.
    pub fraction_outside: f64,
    /// Training rows (presences + background of the final model) used.
    pub n_train: usize,
    /// Predictor weights (normalised permutation importance of the final model; equal when
    /// all importances are 0).
    pub weights: Vec<VariableWeight>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableWeight {
    pub variable: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Extrapolation {
    pub novel_fraction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UncertaintySummary {
    pub mean_fold_sd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contribution {
    pub variable: String,
    /// MaxEnt percent contribution (gain added during training).
    pub percent: f64,
    /// MaxEnt permutation importance (drop in training AUC when the variable is permuted,
    /// normalised to percent). `None` in results computed before it was reported.
    #[serde(default)]
    pub permutation_importance: Option<f64>,
    /// Random-forest permutation importance (mean drop in training presence–background AUC when
    /// the variable is permuted, normalised to percent), for `rf` and `ensemble` (ADR-0017).
    /// For `rf`, `percent` holds the same value and `permutation_importance` is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rf_importance: Option<f64>,
}

/// Marginal response curve of the final model (MaxEnt `createProfiles`): relative suitability
/// (cloglog) as one variable varies, every other variable held at its mean over the presences.
/// Sampled every 5th of MaxEnt's 501 points; the full curve is in `response_curves.csv`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseCurve {
    /// Layer id (as in `.lambdas`).
    pub variable: String,
    /// Display name of the layer.
    pub name: String,
    pub categorical: bool,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// Range of the variable over the training presences and background: outside it the
    /// curve is an extrapolation (MaxEnt plots 10 % beyond each side, clamped).
    pub training_min: f64,
    pub training_max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    pub name: String,
    pub description: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpeciesResult {
    pub species_key: u64,
    pub scientific_name: String,
    pub status: SpeciesRunStatus,
    pub error: Option<String>,
    pub evidence: Evidence,
    pub data: DataSummary,
    pub variables_used: Vec<String>,
    pub variables_dropped: Vec<DroppedVariable>,
    pub selected: SelectedModel,
    pub tuning: Vec<TuningRow>,
    pub validation: ValidationSummary,
    pub thresholds: Vec<ThresholdArea>,
    pub study_area_km2: f64,
    pub extrapolation: Extrapolation,
    /// Area of applicability (absent when not requested).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aoa: Option<AoaSummary>,
    /// Background method actually used (absent for the default random background), ADR-0013.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_sampling: Option<BackgroundSampling>,
    /// Sampling-bias diagnostic (present when a target-group effort surface was used).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling_bias: Option<SamplingBias>,
    /// Independent presence–absence validation on the organisation's data (ADR-0108); absent
    /// when the species uses no such data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub independent_validation: Option<IndependentValidation>,
    pub uncertainty: UncertaintySummary,
    pub contributions: Vec<Contribution>,
    #[serde(default)]
    pub response_curves: Vec<ResponseCurve>,
    /// Partial dependence of the random forest (`rf`, `ensemble`; Friedman 2001): mean relative
    /// suitability (fraction of trees voting presence) as one variable varies, averaged over
    /// the observed values of the others. Not a MaxEnt response curve (ADR-0017).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub partial_dependence: Vec<ResponseCurve>,
    pub bounds: [f64; 4],
    pub files: Vec<FileInfo>,
    pub warnings: Vec<Warning>,
    /// Future-climate projections (ADR-0016), in the order of the specification. Absent when
    /// none was requested (and in results computed before ADR-0016).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projections: Vec<ProjectionResult>,
}

/// Projection of the final model(s) to one future scenario (ADR-0016). Suitable conditions use
/// the present P10 threshold; a cell is "suitable in the future" when the mean over GCMs reaches
/// it. Areas are sums of exact ellipsoidal cell areas over the cells projected (valid at present
/// and under every GCM). They describe a change in the area of suitable *conditions*: no
/// dispersal, land-use change or biotic interaction is modelled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionResult {
    pub period: String,
    pub ssp: String,
    pub gcms: Vec<String>,
    /// Present P10 threshold applied to the future suitability (relative suitability units).
    pub threshold: f64,
    /// Predictor ids replaced by their future climatology.
    pub climate_variables: Vec<String>,
    /// Display names of the predictors kept at present values (terrain, land cover, water).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub static_variables: Vec<String>,
    /// Cells projected, and cells valid at present but without a future value under some GCM
    /// (left out of the maps and areas).
    pub cells: usize,
    pub cells_missing: usize,
    /// Suitable at present / in the future (GCM mean ≥ threshold), km².
    pub present_suitable_km2: f64,
    pub future_suitable_km2: f64,
    /// Change classes, km²: suitable in both, newly suitable, no longer suitable, unsuitable
    /// in both.
    pub stable_km2: f64,
    pub gained_km2: f64,
    pub lost_km2: f64,
    pub unsuitable_km2: f64,
    /// Of each change class, the area where every GCM agrees with the class.
    pub stable_all_gcms_km2: f64,
    pub gained_all_gcms_km2: f64,
    pub lost_all_gcms_km2: f64,
    /// Mean over cells of the SD of suitability across GCMs.
    pub mean_gcm_sd: f64,
    /// Share of the projected cells' area with MESS < 0 under at least one GCM (environments
    /// outside the training range: extrapolation).
    pub novel_fraction: f64,
    pub per_gcm: Vec<GcmProjection>,
    /// The future layers are synthetic (demo).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub synthetic: bool,
}

/// One GCM's projection (spread across GCMs).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GcmProjection {
    pub gcm: String,
    pub future_suitable_km2: f64,
    pub gained_km2: f64,
    pub lost_km2: f64,
    /// Share of the projected area with MESS < 0 under this GCM.
    pub novel_fraction: f64,
    pub mean_suitability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobResults {
    pub job: Job,
    pub species: Vec<SpeciesResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SampleValue {
    pub in_area: bool,
    pub suitability: Option<f64>,
    pub uncertainty: Option<f64>,
    pub mess: Option<f64>,
    /// Dissimilarity index of the area of applicability (only when `aoa.tif` exists).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aoa_di: Option<f64>,
    /// Target-group records in the cell (only when `effort.tif` exists).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<f64>,
    /// Future-climate projections at the point (ADR-0016), in the result's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projections: Vec<ProjectionSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionSample {
    pub period: String,
    pub ssp: String,
    /// Climate models of the scenario.
    pub gcms: usize,
    /// GCM-mean relative suitability and SD across GCMs.
    pub mean: Option<f64>,
    pub sd: Option<f64>,
    /// Change class: 1 unsuitable in both, 2 lost, 3 gained, 4 suitable in both.
    pub change: Option<u8>,
    /// GCMs agreeing with the change class.
    pub agreement: Option<u8>,
    /// Lowest MESS across GCMs (negative: extrapolation).
    pub mess: Option<f64>,
}

// ---- hosted mode (ADR-0105) ----------------------------------------------------------------

pub use crate::accounts::{
    ActivityRow, InviteInfo, LinkInfo, LinkKind, Org, OrgOverview, QuotaStatus, UsageSummary,
    UserInfo, UserPage,
};
pub use crate::audit::{AuditEntry, AuditPage};

/// `GET /api/v1/auth/me`: the signed-in account, its organisation and this month's quota.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Me {
    pub name: String,
    pub email: String,
    pub organisation: String,
    pub quota: QuotaStatus,
    /// Natural Solutions administrator (ADR-0106).
    pub is_admin: bool,
    /// Manager of their own organisation (ADR-0109): invites and manages its members.
    pub is_manager: bool,
    /// Interface language chosen by the account (`en`, `fr`, `es`); `None` = browser default.
    pub lang: Option<String>,
}

/// `POST /api/v1/auth/lang`.
#[derive(Debug, Clone, Deserialize)]
pub struct LangRequest {
    pub lang: String,
}

// ---- account and organisation properties (ADR-0110) ----------------------------------------

/// `PATCH /api/v1/account`.
#[derive(Debug, Clone, Deserialize)]
pub struct AccountUpdate {
    pub name: String,
}

/// `POST /api/v1/account/password`.
#[derive(Clone, Deserialize)]
pub struct PasswordChange {
    pub current: String,
    pub new: String,
}

/// `PUT /api/v1/account/gbif`, `PUT /api/v1/org/gbif`. The password is never sent back.
#[derive(Clone, Deserialize)]
pub struct GbifCredentialsRequest {
    pub username: String,
    pub password: String,
}

/// Whose GBIF account a study's download uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GbifSource {
    /// The account of the user who creates the study.
    User,
    /// The account of their organisation.
    Organisation,
    /// The server's shared account (Natural Solutions).
    Shared,
    /// No account: records come from the GBIF search API, without DOI.
    None,
}

/// A configured GBIF account as shown to users (never the password).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GbifAccountInfo {
    /// Hidden from members who are not managers, for the organisation's account.
    pub username: Option<String>,
    pub checked_at: Option<String>,
    /// GBIF's answer at the last check (`None`: never checked, or GBIF unreachable).
    pub ok: Option<bool>,
}

/// `GET /api/v1/account/gbif`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GbifSettings {
    pub own: Option<GbifAccountInfo>,
    pub organisation: Option<GbifAccountInfo>,
    /// The server's shared account exists (the last resort).
    pub shared: bool,
    /// Account the user's next studies will use.
    pub effective: GbifSource,
    /// The server can store accounts (`NICHE_SECRET_KEY` is set).
    pub can_store: bool,
}

/// `GET /api/v1/org`: the user's organisation, for every member; managers edit it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrgProfile {
    pub name: String,
    pub quota: QuotaStatus,
    pub members: u64,
    pub gbif: Option<GbifAccountInfo>,
    pub shared_gbif: bool,
    pub can_store: bool,
    pub can_manage: bool,
}

// ---- administration (ADR-0106) -------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
    pub monthly_quota: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOrgRequest {
    pub monthly_quota: Option<u64>,
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserRequest {
    pub email: String,
}

/// Account erasure: `confirm_email` must repeat `email`.
#[derive(Debug, Clone, Deserialize)]
pub struct UserDeleteRequest {
    pub email: String,
    pub confirm_email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserDisableRequest {
    pub email: String,
    pub disabled: bool,
}

/// `POST /api/v1/admin/users/manager`: grant or remove the organisation-manager role.
#[derive(Debug, Clone, Deserialize)]
pub struct UserManagerRequest {
    pub email: String,
    pub manager: bool,
}

/// `POST /api/v1/admin/users/admin`: make a member of the operator organisation an
/// administrator (ADR-0112). `password` is the requesting administrator's own.
#[derive(Debug, Clone, Deserialize)]
pub struct GrantAdminRequest {
    pub email: String,
    pub password: String,
}

/// A freshly created one-time link: `path` is `/invite#<token>` or `/reset#<token>`; the page
/// prefixes the public origin. Shown once, never stored in clear.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LinkCreated {
    pub path: String,
    pub days_valid: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LinkRequest {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AcceptInviteRequest {
    pub token: String,
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

// ---- feedback (ADR-0110) --------------------------------------------------------------------

/// What the feedback is about; optional in the form (`other` by default).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackKind {
    Problem,
    Idea,
    #[default]
    Other,
}

impl FeedbackKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Problem => "problem",
            Self::Idea => "idea",
            Self::Other => "other",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "problem" => Self::Problem,
            "idea" => Self::Idea,
            _ => Self::Other,
        }
    }
}

/// Whether Natural Solutions has dealt with it (set from the administration).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackStatus {
    #[default]
    Open,
    Closed,
}

impl FeedbackStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "closed" {
            Self::Closed
        } else {
            Self::Open
        }
    }
}

/// A screenshot, image or voice recording sent with the feedback (`data`: standard base64).
#[derive(Debug, Clone, Deserialize)]
pub struct FeedbackUpload {
    pub name: String,
    pub mime: String,
    pub data: String,
}

/// `POST /api/v1/feedback`: a message, attachments, or both. `context` is what the page knows
/// (language, screen size, browser, current study) and is shown to Natural Solutions as is.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FeedbackRequest {
    #[serde(default)]
    pub kind: FeedbackKind,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default)]
    pub context: serde_json::Value,
    #[serde(default)]
    pub attachments: Vec<FeedbackUpload>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FeedbackFile {
    pub id: String,
    pub name: String,
    pub mime: String,
    pub size: u64,
}

/// One piece of feedback as the administration and the organisation page list it.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Feedback {
    pub id: String,
    pub created_at: String,
    pub kind: FeedbackKind,
    pub message: String,
    pub page: Option<String>,
    pub context: serde_json::Value,
    /// `None` for feedback of an erased account.
    pub author: Option<String>,
    pub author_email: Option<String>,
    pub org_name: Option<String>,
    pub status: FeedbackStatus,
    pub files: Vec<FeedbackFile>,
}

/// `PATCH /api/v1/admin/feedback/{id}`.
#[derive(Debug, Clone, Deserialize)]
pub struct FeedbackStatusRequest {
    pub status: FeedbackStatus,
}

/// `POST /api/v1/feedback` answer.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FeedbackCreated {
    pub id: String,
}

// ---- the organisation's own occurrence data (ADR-0108) --------------------------------------

/// A dataset of the organisation used by a study: identifier and the SHA-256 of its normalised
/// records (checked at submission: a dataset is immutable).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatasetRef {
    pub id: String,
    pub records_sha256: String,
}

/// How a species uses the organisation's dataset.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OwnDataRole {
    /// Presence records join the training data (with GBIF when `gbif` is true); absence records
    /// are excluded from training (rule `absence`) and evaluated as validation sites.
    #[default]
    Training,
    /// Held out: every record (presences and absences) is used only for the independent
    /// presence–absence validation of the model trained on GBIF records.
    Validation,
}

impl OwnDataRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::Validation => "validation",
        }
    }
}

/// Per-species use of the organisation's dataset (`SpeciesRef.own`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OwnSpeciesData {
    /// Values of the dataset's taxon column that are this species (matched to its GBIF key).
    pub values: Vec<String>,
    #[serde(default)]
    pub role: OwnDataRole,
    /// GBIF records (synthetic records in the demo) are also used for training; always true
    /// for `validation`.
    #[serde(default = "yes")]
    pub gbif: bool,
}

fn yes() -> bool {
    true
}

/// Columns of an import, chosen in the browser: one array per column, all of the same length
/// (`taxon`, `x`, `y` required). Values stay strings: they are interpreted on the server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ImportColumns {
    pub taxon: Vec<String>,
    pub x: Vec<String>,
    pub y: Vec<String>,
    #[serde(default)]
    pub date: Option<Vec<String>>,
    #[serde(default)]
    pub uncertainty: Option<Vec<String>>,
    #[serde(default)]
    pub status: Option<Vec<String>>,
    #[serde(default)]
    pub survey: Option<Vec<String>>,
}

/// `POST /api/v1/datasets`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportRequest {
    pub name: String,
    pub file_name: String,
    /// SHA-256 of the original file, computed in the browser (the file itself is not sent).
    pub file_sha256: String,
    /// `EPSG:<code>` from the closed list ([`CrsOption`]).
    pub crs: String,
    pub columns: ImportColumns,
    /// Line of the file each row comes from (1 = header); default: row index + 2.
    #[serde(default)]
    pub lines: Option<Vec<u32>>,
    /// Values of the status column meaning "absent / not observed" (compared case-insensitively).
    #[serde(default)]
    pub absent_values: Vec<String>,
    /// Column of the file chosen for each field (`taxon`, `x`, `y`, `date`…), for provenance.
    #[serde(default)]
    pub mapping: BTreeMap<String, String>,
    /// The file is synthetic (tests, demonstrations): labelled as such everywhere.
    #[serde(default)]
    pub synthetic: bool,
}

/// Rows of an import rejected for one reason (problems of the file, not scientific rules).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RejectedRows {
    /// `missing_taxon`, `bad_coordinates`, `outside_crs_domain`, `bad_date`, `bad_uncertainty`.
    pub reason: String,
    pub count: usize,
    /// File lines of the first 10 rows.
    pub lines: Vec<u32>,
}

/// One distinct value of the taxon column.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxonValue {
    pub value: String,
    pub records: usize,
    pub presences: usize,
    pub absences: usize,
}

/// What an import kept and rejected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportReport {
    pub rows_read: usize,
    pub rows_imported: usize,
    pub rejected: Vec<RejectedRows>,
    /// (west, south, east, north) of the imported records in WGS84; `None` when none.
    pub bounds_wgs84: Option<[f64; 4]>,
    pub presences: usize,
    pub absences: usize,
    /// Records with a protocol/survey identifier.
    pub with_survey: usize,
    pub taxa: Vec<TaxonValue>,
    pub warnings: Vec<Warning>,
}

/// An organisation's imported dataset (`GET /api/v1/datasets`). Never contains records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatasetInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub file_name: String,
    pub file_sha256: String,
    pub records_sha256: String,
    pub crs: String,
    pub absent_values: Vec<String>,
    pub mapping: BTreeMap<String, String>,
    #[serde(default)]
    pub synthetic: bool,
    pub report: ImportReport,
}

/// A coordinate reference system offered by the import (closed list, ADR-0108).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrsOption {
    pub code: String,
    pub name: String,
    /// Where it is used, in English.
    pub usage: String,
}

/// `POST /api/v1/species/match`: distinct names of a dataset's taxon column. Only the names
/// are sent to GBIF.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NameMatchRequest {
    pub names: Vec<String>,
}

/// GBIF backbone match of one name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxonMatch {
    pub name: String,
    pub key: Option<u64>,
    pub scientific_name: Option<String>,
    pub rank: Option<String>,
    pub status: Option<String>,
    /// `EXACT`, `FUZZY`, `HIGHERRANK`, `NONE`, or `ERROR` when GBIF could not answer.
    pub match_type: String,
    pub confidence: Option<u64>,
    /// Proposed automatically (exact match at species rank or below); the user confirms.
    pub accepted: bool,
}

/// Records of one source of a species (ADR-0108).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceCount {
    /// `gbif` (the synthetic records in the demo) or `own`.
    pub source: String,
    pub records: usize,
    /// Presence cells kept for training from this source.
    pub retained_cells: usize,
    /// Records per status code (`retained`, `duplicate_cell`, `absence`, `validation`…).
    pub statuses: BTreeMap<String, usize>,
    /// `gbif` only: records not kept because their cell is already given by the organisation's
    /// data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_own_cells: Option<usize>,
}

/// Independent presence–absence sites (one per grid cell) after cleaning and the leakage rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationSites {
    /// Records of the dataset considered for validation.
    pub records: usize,
    /// Records excluded by the cleaning rules or the leakage rule, by code.
    pub excluded: BTreeMap<String, usize>,
    pub presence_sites: usize,
    pub absence_sites: usize,
    /// Cells with both presence and absence records (counted as presence sites).
    pub conflicting_cells: usize,
    /// Sites dropped because their cell is a training presence cell (leakage rule).
    pub leakage_excluded: usize,
}

/// Metrics at one threshold taken from the training data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaThreshold {
    /// `p10` or `max_sss` (thresholds of the training data, as in `thresholds`).
    pub rule: String,
    pub threshold: f64,
    pub sensitivity: Option<f64>,
    pub specificity: Option<f64>,
    pub tss: Option<f64>,
    pub kappa: Option<f64>,
    pub true_pos: usize,
    pub false_pos: usize,
    pub false_neg: usize,
    pub true_neg: usize,
}

/// One bin of the calibration table: a diagnostic only, since suitability is not a probability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalibrationRow {
    pub lower: f64,
    pub upper: f64,
    pub mean_suitability: f64,
    pub observed_presence_share: f64,
    pub n: usize,
}

/// How spread out the validation sites are.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SiteSpread {
    /// East–west and north–south extent of the sites, km.
    pub extent_km: [f64; 2],
    /// Area of the sites' bounding box as a share of the study area's bounding box.
    pub share_of_study_extent: f64,
    /// Mean distance to the nearest other site, km (`None` with one site).
    pub mean_nearest_km: Option<f64>,
}

/// Independent presence–absence validation of the final model (ADR-0108).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndependentValidation {
    pub dataset_id: String,
    pub dataset_name: String,
    /// `validation` (held out) or `training` (absences of a training dataset).
    pub role: OwnDataRole,
    #[serde(default)]
    pub synthetic: bool,
    pub sites: ValidationSites,
    /// `None` when there are too few presence or absence sites (see `note`).
    pub auc: Option<f64>,
    pub thresholds: Vec<PaThreshold>,
    pub brier: Option<f64>,
    pub prevalence: Option<f64>,
    pub calibration: Vec<CalibrationRow>,
    pub spread: Option<SiteSpread>,
    /// Why metrics are missing or limited, in English.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Every criterion of the Validated level is met (ADR-0108).
    pub validated: bool,
}

// ---- Decision-support analyses (ADR-0018) --------------------------------------------------

impl EvidenceLevel {
    /// Order of the levels, lowest first (minimum-level filters).
    pub fn rank(self) -> u8 {
        match self {
            Self::Insufficient => 0,
            Self::Exploratory => 1,
            Self::Indicative => 2,
            Self::Supported => 3,
            Self::Validated => 4,
        }
    }
}

/// `POST /api/v1/jobs/{id}/analyses`: an on-demand analysis of a finished study. Nothing is
/// refitted and the study's own outputs and manifest are not changed (ADR-0018).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnalysisRequest {
    /// Survey priority of one species: relative suitability against sampling effort (or, without
    /// an effort surface, distance to the nearest known record).
    Priority {
        species_key: u64,
        /// Candidate areas listed (default 20, 1–100).
        #[serde(default)]
        top_n: Option<usize>,
    },
    /// Overlay of a project footprint (GeoJSON Polygon/MultiPolygon in WGS84) on every species.
    Footprint {
        name: String,
        geometry: serde_json::Value,
    },
    /// Multi-species aggregation (stacked P10 maps and summed relative suitability) over the
    /// species at or above `min_evidence` (default `indicative`).
    Richness {
        #[serde(default)]
        min_evidence: Option<EvidenceLevel>,
    },
    /// Comparison with another finished study of the same species on the same grid.
    Comparison {
        other_job_id: String,
        species_key: u64,
    },
}

/// A raster layer of an analysis, served as tiles (`…/analyses/{aid}/tiles/{layer}/…`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisLayer {
    /// `priority`, `priority_class`, `richness`, `suitability_sum`, `hotspot` or `difference`.
    pub id: String,
    /// Value range for the legend.
    pub min: f64,
    pub max: f64,
}

/// One analysis of a study, as stored in `jobs/{id}/analyses/{aid}/analysis.json` (next to its
/// manifest addendum) and returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Analysis {
    pub id: String,
    /// `priority`, `footprint`, `richness` or `comparison`.
    pub kind: String,
    pub job_id: String,
    pub created_at: String,
    /// Display title in English (species, footprint name or compared study).
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub species_key: Option<u64>,
    /// The study is the synthetic demo: nothing here describes a real species or place.
    #[serde(default)]
    pub synthetic: bool,
    pub files: Vec<FileInfo>,
    #[serde(default)]
    pub layers: Vec<AnalysisLayer>,
    pub warnings: Vec<Warning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<PriorityResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint: Option<FootprintResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub richness: Option<RichnessResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison: Option<ComparisonResult>,
}

/// Area of one survey-priority class.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriorityClass {
    /// 1 `prospect_first`, 2 `known`, 3 `low_unsampled`, 4 `low_sampled` (`priority_class.tif`).
    pub code: u8,
    pub id: String,
    pub cells: usize,
    pub area_km2: f64,
    pub fraction_of_study_area: f64,
}

/// A candidate area worth surveying: a connected group (8 neighbours) of `prospect_first`
/// cells, ranked by its mean priority score.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriorityCandidate {
    pub rank: usize,
    /// Area-weighted centroid of the cells (may fall outside an irregular group).
    pub lng: f64,
    pub lat: f64,
    /// West, south, east, north of the group.
    pub bbox: [f64; 4],
    pub cells: usize,
    pub area_km2: f64,
    pub mean_priority: f64,
    pub mean_suitability: f64,
    pub max_suitability: f64,
    /// Effort method: mean target-group records per cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_effort: Option<f64>,
    /// Distance method: mean distance to the nearest known record, km.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_distance_km: Option<f64>,
    /// Share of the group's area where the model extrapolates (MESS < 0).
    pub extrapolated_share: f64,
}

/// Survey priority of one species (ADR-0018 §1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriorityResult {
    pub species_key: u64,
    pub scientific_name: String,
    pub evidence_level: EvidenceLevel,
    /// `effort` (target-group sampling effort, ADR-0013) or `distance` (distance to the nearest
    /// known record of the species, the proxy used when no effort surface exists).
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_group: Option<String>,
    /// The effort surface is synthetic (demo).
    #[serde(default)]
    pub effort_synthetic: bool,
    /// Known records used by the distance method (retained, thinned and held-out presences).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_records: Option<usize>,
    /// Suitability threshold separating "higher" from "lower" suitability: the species' P10.
    pub suitability_threshold: f64,
    /// Effort (records per cell) or distance (km) at the median split between "little" and
    /// "well" sampled cells.
    pub sampling_split: f64,
    pub study_area_km2: f64,
    pub classes: Vec<PriorityClass>,
    pub candidates: Vec<PriorityCandidate>,
    /// Candidate groups in total (the list keeps the first `top_n`).
    pub candidate_groups: usize,
    /// Candidate cells are `prospect_first` cells with a priority score at or above this value
    /// (the top 10 % of the study-area cells by score).
    pub candidate_min_priority: f64,
}

/// Suitable conditions of one species inside a footprint at one threshold rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FootprintThreshold {
    /// `p10` (headline), `mtp`, `max_sss`.
    pub rule: String,
    pub label: String,
    pub threshold: f64,
    /// Area of the footprint over cells at or above the threshold, km².
    pub suitable_km2: f64,
    /// `suitable_km2` / footprint area over cells with predictions.
    pub share_of_footprint: f64,
    /// Species' area of suitable conditions in the whole study area at this rule, km².
    pub species_suitable_km2: f64,
    /// `suitable_km2` / `species_suitable_km2` (0 when the species has none).
    pub share_of_species_area: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FootprintSpecies {
    pub species_key: u64,
    pub scientific_name: String,
    /// `None` when the species has no model (failed or insufficient).
    pub evidence_level: Option<EvidenceLevel>,
    pub algorithm: Option<String>,
    pub thresholds: Vec<FootprintThreshold>,
    /// Area-weighted over the footprint (cells with predictions).
    pub mean_suitability: Option<f64>,
    pub max_suitability: Option<f64>,
    pub mean_uncertainty: Option<f64>,
    /// Share of the footprint where the model extrapolates (MESS < 0).
    pub extrapolated_share: Option<f64>,
    /// Share of the footprint outside the area of applicability (`None` when not computed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outside_aoa_share: Option<f64>,
    /// Why there are no values, in English.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Project-footprint overlay (ADR-0018 §2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FootprintResult {
    pub name: String,
    pub geometry: serde_json::Value,
    /// Exact ellipsoidal area of the footprint, km².
    pub area_km2: f64,
    /// Part of it over model cells with predictions (inside the study area), km².
    pub in_study_km2: f64,
    /// Model cells it overlaps.
    pub cells: usize,
    /// Mean area of those cells, km² (a footprint smaller than a cell is described by the cells
    /// it touches).
    pub mean_cell_km2: f64,
    pub species: Vec<FootprintSpecies>,
}

/// A species of a multi-species aggregation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RichnessSpecies {
    pub species_key: u64,
    pub scientific_name: String,
    pub evidence_level: Option<EvidenceLevel>,
    /// P10 threshold of the species (included species).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    /// Area of suitable conditions at P10 (included species), km².
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suitable_km2: Option<f64>,
    /// Why it is left out, in English (excluded species).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RichnessBin {
    /// Number of species with suitable conditions.
    pub species: usize,
    pub cells: usize,
    pub area_km2: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HotspotSummary {
    /// Share of the study-area cells targeted (0.10).
    pub target_share: f64,
    pub cells: usize,
    pub area_km2: f64,
    pub fraction_of_study_area: f64,
    /// Lowest stacked count among hotspot cells.
    pub min_species: usize,
}

/// Multi-species aggregation (ADR-0018 §3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RichnessResult {
    pub min_evidence: EvidenceLevel,
    /// `p10`.
    pub threshold_rule: String,
    pub included: Vec<RichnessSpecies>,
    pub excluded: Vec<RichnessSpecies>,
    pub study_area_km2: f64,
    pub max_species: usize,
    /// Highest summed relative suitability.
    pub max_suitability_sum: f64,
    pub histogram: Vec<RichnessBin>,
    pub hotspot: HotspotSummary,
}

/// One side of a comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparedStudy {
    pub job_id: String,
    pub name: String,
    pub algorithm: String,
    pub evidence_level: EvidenceLevel,
    pub presences: usize,
    pub cbi_mean: Option<f64>,
    pub or10_mean: f64,
    pub auc_mean: f64,
    pub validation_method: String,
    /// Background actually used: `random` or `target_group`.
    pub background: String,
    pub thresholds: Vec<ThresholdArea>,
    pub extrapolated_share: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outside_aoa_share: Option<f64>,
    pub variables: Vec<String>,
}

/// A setting that differs between the two specifications (dotted JSON path; `null` = absent,
/// i.e. the default omitted from the specification).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SettingDiff {
    pub path: String,
    pub a: serde_json::Value,
    pub b: serde_json::Value,
}

/// Agreement of the two P10 maps, km².
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BinaryAgreement {
    pub both_km2: f64,
    pub only_a_km2: f64,
    pub only_b_km2: f64,
    pub neither_km2: f64,
}

/// Comparison of two studies of one species (ADR-0018 §4). `difference.tif` = B − A.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonResult {
    pub species_key: u64,
    pub scientific_name: String,
    pub a: ComparedStudy,
    pub b: ComparedStudy,
    pub settings: Vec<SettingDiff>,
    /// Cells with a prediction in both studies.
    pub common_cells: usize,
    pub common_km2: f64,
    /// Schoener's D of the two surfaces, each normalised to sum 1 (Warren et al. 2008), 0–1.
    pub schoener_d: f64,
    /// Spearman rank correlation of the two surfaces over the common cells.
    pub spearman: f64,
    /// Mean of B − A and of |B − A| over the common cells.
    pub mean_difference: f64,
    pub mean_abs_difference: f64,
    pub p10_agreement: BinaryAgreement,
}
