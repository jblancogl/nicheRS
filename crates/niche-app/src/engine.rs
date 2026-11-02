//! Engine: the single entry point for every deployment mode (server, CLI batch).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use niche_data::gbif_download::Credentials;
use niche_data::{Http, gbif};
use niche_eval::rng::Rng;
use niche_geo::Raster;
use niche_geo::boundaries::{BoundaryPack, Place};
use niche_geo::geotiff::{FileSource, GeoTiffInfo};
use serde_json::json;

use crate::accounts::{Principal, Scope};
use crate::artifacts::{self, Outputs, species_dir, write_atomic};
use crate::bias::{self, EffortSurface};
use crate::checkpoint;
use crate::dto::*;
use crate::env::{
    self, Cleaned, EnvStack, build_stack, sample_background, sample_background_target_group,
    thin_seed,
};
use crate::fit::{FitOutput, fit_species};
use crate::gbif_accounts::{GbifOwner, JobOwner, StoredGbif};
use crate::projection;
use crate::store::{DataState, Store};
use crate::{AppError, demo};

/// Resolutions offered by the interface: aggregation factors of the 30-arc-second grid (≈ km).
pub const RESOLUTION_FACTORS: [usize; 4] = [1, 2, 5, 10];

/// Configuration (see docs/architecture/configuration.md).
#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    /// Upper bound on GBIF records retrieved per species (spatially stratified when exceeded).
    pub max_records: usize,
    /// GBIF account for batch downloads (DOI, no paging limits). `None` → search API only.
    pub gbif: Option<niche_data::gbif_download::Credentials>,
    /// Key sealing the GBIF accounts of users and organisations (ADR-0110). `None` → those
    /// accounts can be neither stored nor used; the shared account above still works.
    pub secret_key: Option<crate::secrets::SecretKey>,
    /// Regional GBIF mirror (ADR-0114). `None` → every study downloads from GBIF.
    pub mirror: Option<MirrorConfig>,
    pub limits: Limits,
    /// Finished jobs older than this are deleted (0 = keep forever).
    pub retention_days: u64,
}

/// A regional GBIF mirror: which region, how often it is refreshed, how old it may be used.
#[derive(Debug, Clone, Copy)]
pub struct MirrorConfig {
    pub region: niche_data::gbif_mirror::Region,
    pub refresh_days: u64,
    pub max_age_days: u64,
}

/// Per-job limits checked at submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_species_per_job: usize,
    /// Cells of the modelling grid over the study-area extent (land and sea); `None` = only the
    /// hard memory limit of the environmental stack applies.
    pub max_area_cells: Option<usize>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_species_per_job: 500,
            max_area_cells: None,
        }
    }
}

impl Limits {
    /// Hosted mode (`produit-v0.md` §4): ≤ 25 species; extent ≤ 2.5 M cells (metropolitan
    /// France at 1 km is 1721 × 1167 ≈ 2.0 M, the largest measured case).
    pub fn hosted() -> Self {
        Self {
            max_species_per_job: 25,
            max_area_cells: Some(2_500_000),
        }
    }
}

/// Longest side of a workspace thumbnail, in pixels (cards are ~300 CSS px; 2× for HiDPI).
const THUMBNAIL_PX: usize = 560;

/// Load every valid `.nrsb` pack of a directory; an unreadable pack is logged and skipped.
fn load_boundaries(dir: &std::path::Path) -> Vec<BoundaryPack> {
    let mut packs = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return packs;
    };
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().is_none_or(|x| x != "nrsb") {
            continue;
        }
        match std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| BoundaryPack::open(&b).map_err(|e| e.to_string()))
        {
            Ok(p) => {
                tracing::info!(
                    pack = p.pack(),
                    units = p.places().len(),
                    "boundary pack loaded"
                );
                packs.push(p);
            }
            Err(err) => tracing::warn!(file = %path.display(), %err, "boundary pack ignored"),
        }
    }
    packs.sort_by(|a, b| a.pack().cmp(b.pack()));
    packs
}

pub struct Engine {
    pub cfg: Config,
    http: Http,
    store: Store,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    rasters: Mutex<lru::Lru>,
    /// Administrative boundary packs installed in `<data_dir>/boundaries/` (self-hosted).
    boundaries: Vec<BoundaryPack>,
    /// Studies waiting for their GBIF download, by job id (in memory: a restart re-requests).
    waits: Mutex<HashMap<String, GbifWait>>,
    started_at: String,
    /// Last check of the GBIF account (administration), cached for a few minutes.
    gbif_check: Mutex<Option<(std::time::Instant, GbifAccount)>>,
    /// Regional GBIF mirror in use (ADR-0114), replaced when a refresh completes.
    mirror: Mutex<Option<Arc<niche_data::gbif_mirror::Snapshot>>>,
    mirror_state: Mutex<mirror::State>,
    /// The mirror's own GBIF download holds one of the shared account's slots.
    mirror_at_gbif: AtomicBool,
    /// This process holds the leader lease and runs the background work (ADR-0115).
    leader: AtomicBool,
}

mod analyses;
mod mirror;

/// GBIF usually prepares a download in 2–15 min (observed up to 23 min): past this, say so.
const GBIF_SLOW_AFTER: Duration = Duration::from_secs(20 * 60);
/// Past this, offer to continue without the download (search API, no DOI).
const GBIF_SKIP_AFTER: Duration = Duration::from_secs(30 * 60);

mod lru {
    use niche_geo::Raster;
    use std::sync::Arc;

    /// Tiny LRU for rasters served as tiles (bounded memory).
    #[derive(Default)]
    pub struct Lru {
        items: Vec<(String, Arc<Raster>)>,
    }

    impl Lru {
        pub fn get(&mut self, k: &str) -> Option<Arc<Raster>> {
            let i = self.items.iter().position(|(key, _)| key == k)?;
            let it = self.items.remove(i);
            let v = it.1.clone();
            self.items.push(it);
            Some(v)
        }
        pub fn remove_prefix(&mut self, prefix: &str) {
            self.items.retain(|(k, _)| !k.starts_with(prefix));
        }

        pub fn put(&mut self, k: String, v: Arc<Raster>) {
            self.items.retain(|(key, _)| key != &k);
            if self.items.len() >= 12 {
                self.items.remove(0);
            }
            self.items.push((k, v));
        }
    }
}

/// Bounds of the opt-in evaluation settings (ADR-0014).
const MIN_USER_BLOCK_KM: f64 = 1.0;
const MAX_USER_BLOCK_KM: f64 = 1000.0;
/// Each null model refits every fold of the selected setting: 200 × 5 folds = 1000 fits, about
/// ten times the default tuning grid.
const MAX_NULL_MODELS: usize = 200;
/// The smallest empirical p-value with n nulls is 1/(n + 1): at least 19 are needed for p ≤ 0.05,
/// otherwise every model would be capped at Exploratory.
const MIN_NULL_MODELS: usize = 19;
/// Largest distance-thinning distance (Science v1 phase 2): beyond it thinning would leave a
/// handful of presences in most study areas.
const MAX_THIN_KM: f64 = 100.0;
/// Bounds of the random-forest size (ADR-0017): below 100 trees the vote fraction is coarse
/// (steps of 0.01) and unstable between seeds; above 2000 prediction cost grows with no gain
/// reported for presence–background data.
const MIN_RF_TREES: usize = 100;
const MAX_RF_TREES: usize = 2000;

/// Records of one species as fetched: records, total matching, truncated, source text, query
/// URL, and where they came from (`None` for the synthetic demo).
type Fetched = (
    Vec<gbif::Occurrence>,
    u64,
    bool,
    String,
    Option<String>,
    Option<gbif::RecordSource>,
);

/// Share of a species' progress taken by the fit when future projections follow (ADR-0016).
const FIT_SHARE: f64 = 0.6;

fn spec_hash(spec: &JobSpec) -> String {
    artifacts::sha256(serde_json::to_string(spec).unwrap_or_default().as_bytes())
}

impl Engine {
    pub fn open(cfg: Config) -> Result<Self, AppError> {
        std::fs::create_dir_all(&cfg.data_dir)
            .map_err(|e| AppError::Storage(format!("{}: {e}", cfg.data_dir.display())))?;
        std::fs::create_dir_all(&cfg.cache_dir)
            .map_err(|e| AppError::Storage(format!("{}: {e}", cfg.cache_dir.display())))?;
        let store = Store::open(&cfg.data_dir.join("niche.db"))?;
        let boundaries = load_boundaries(&cfg.data_dir.join("boundaries"));
        let mirror = cfg.mirror.and_then(|m| {
            niche_data::gbif_mirror::MirrorDir::new(&cfg.data_dir, &m.region)
                .open_current()
                .map(Arc::new)
        });
        Ok(Self {
            cfg,
            http: Http::new(),
            store,
            cancels: Mutex::new(HashMap::new()),
            rasters: Mutex::new(lru::Lru::default()),
            boundaries,
            waits: Mutex::new(HashMap::new()),
            started_at: crate::store::now(),
            gbif_check: Mutex::new(None),
            mirror: Mutex::new(mirror),
            mirror_state: Mutex::new(mirror::State::default()),
            mirror_at_gbif: AtomicBool::new(false),
            leader: AtomicBool::new(false),
        })
    }

    /// Set by the server once it holds the leader lease ([`crate::lease::Lease`]).
    pub fn set_leader(&self, leader: bool) {
        self.leader.store(leader, Ordering::Relaxed);
    }

    pub fn is_leader(&self) -> bool {
        self.leader.load(Ordering::Relaxed)
    }

    /// A per-job file shared by the server processes of a deployment (ADR-0115): a request may
    /// reach the process that is not running the job.
    fn job_file(&self, id: &str, name: &str) -> std::path::PathBuf {
        self.cfg.data_dir.join("jobs").join(id).join(name)
    }

    fn touch(&self, path: &std::path::Path) -> Result<(), AppError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| AppError::Storage(format!("{}: {e}", dir.display())))?;
        }
        std::fs::write(path, b"").map_err(|e| AppError::Storage(format!("{}: {e}", path.display())))
    }

    /// Cancellation requested for a running job, by this or another server process.
    fn cancel_requested(&self, id: &str) -> bool {
        self.job_file(id, ".cancel").exists()
    }

    /// Largest modelling grid a study may use: the deployment limit, within the hard cap.
    pub fn max_model_cells(&self) -> usize {
        self.cfg
            .limits
            .max_area_cells
            .map_or(env::MAX_CELLS, |l| l.min(env::MAX_CELLS))
    }

    /// Grid cells of a study area at each resolution offered by the interface, against
    /// [`Self::max_model_cells`] — the same count [`Self::validate`] and the stack builder apply.
    pub fn check_area(&self, area: &StudyArea) -> Result<AreaCheck, AppError> {
        let max_cells = self.max_model_cells();
        let resolutions = RESOLUTION_FACTORS
            .iter()
            .map(|&factor| {
                let cells =
                    env::area_window(&self.http, &self.cfg.cache_dir, area, factor)?.model_cells();
                Ok(ResolutionCheck {
                    factor,
                    cells,
                    fits: cells <= max_cells,
                })
            })
            .collect::<Result<_, AppError>>()?;
        Ok(AreaCheck {
            max_cells,
            resolutions,
            gbif_mirror: self.mirror_for(area).map(|s| {
                s.meta
                    .download
                    .created
                    .get(..10)
                    .unwrap_or(&s.meta.download.created)
                    .to_string()
            }),
        })
    }

    /// Whether a study is being modelled now (deployments wait for this to be false).
    pub fn busy(&self) -> Result<bool, AppError> {
        Ok(self.store.running_count()? > 0)
    }

    /// Installed boundary packs (name, units).
    pub fn boundary_packs(&self) -> Vec<(String, usize)> {
        self.boundaries
            .iter()
            .map(|p| (p.pack().to_string(), p.places().len()))
            .collect()
    }

    fn place_hit(pack: &BoundaryPack, p: &Place) -> PlaceHit {
        // The unit's own source when the pack has several per country, else its country's.
        let attribution = p
            .source
            .and_then(|i| pack.sources().get(i))
            .or_else(|| pack.sources().iter().find(|s| s.country == p.country))
            .map_or_else(String::new, |s| {
                format!("{} — {}", s.attribution, s.licence)
            });
        PlaceHit {
            id: p.id.clone(),
            name: p.name.clone(),
            level: format!("{:?}", p.level).to_lowercase(),
            designation: p.designation.clone(),
            country: p.country.clone(),
            within: pack
                .lineage(&p.id)
                .iter()
                .skip(1)
                .map(|a| a.name.clone())
                .collect(),
            bbox: p.bbox,
            area_km2: p.area_km2,
            pack: pack.pack().to_string(),
            attribution,
        }
    }

    /// Administrative units by name across installed packs, best first.
    pub fn search_places(&self, q: &str, country: Option<&str>, limit: usize) -> Vec<PlaceHit> {
        niche_geo::boundaries::search_packs(&self.boundaries, q, country, limit.clamp(1, 50))
            .into_iter()
            .map(|(pack, p)| Self::place_hit(pack, p))
            .collect()
    }

    /// Units containing a point, largest first.
    pub fn places_at(&self, lng: f64, lat: f64) -> Vec<PlaceHit> {
        self.boundaries
            .iter()
            .flat_map(|pack| {
                pack.at(lng, lat)
                    .into_iter()
                    .map(move |p| Self::place_hit(pack, p))
            })
            .collect()
    }

    pub fn place_geometry(&self, id: &str) -> Result<PlaceGeometry, AppError> {
        for pack in &self.boundaries {
            if let Some(p) = pack.place(id) {
                let geometry = pack
                    .geojson(id)
                    .map_err(|e| AppError::Storage(e.to_string()))?;
                return Ok(PlaceGeometry {
                    place: Self::place_hit(pack, p),
                    geometry,
                });
            }
        }
        Err(AppError::NotFound(format!("place {id}")))
    }

    pub fn http(&self) -> &Http {
        &self.http
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn search_species(&self, q: &str) -> Result<Vec<gbif::SpeciesHit>, AppError> {
        let q = q.trim();
        if q.len() < 2 {
            return Ok(Vec::new());
        }
        Ok(gbif::search_species(&self.http, q)?)
    }

    pub fn layers(&self, demo: bool) -> Vec<LayerInfo> {
        env::layer_catalogue(demo)
    }

    fn validate(&self, spec: &JobSpec) -> Result<(), AppError> {
        if spec.species.is_empty() {
            return Err(AppError::Invalid("select at least one species".into()));
        }
        let max = self.cfg.limits.max_species_per_job;
        if spec.species.len() > max {
            return Err(AppError::Invalid(format!(
                "at most {max} species per job ({} selected)",
                spec.species.len()
            )));
        }
        let mut keys = std::collections::HashSet::new();
        if let Some(sp) = spec.species.iter().find(|s| !keys.insert(s.key)) {
            return Err(AppError::Invalid(format!(
                "{} is selected twice; each species appears once in a study",
                sp.scientific_name
            )));
        }
        validate_own_data(spec)?;
        if spec.layers.is_empty() {
            return Err(AppError::Invalid(
                "select at least one environmental variable".into(),
            ));
        }
        let catalogue = env::layer_catalogue(spec.demo);
        for l in &spec.layers {
            if !catalogue.iter().any(|c| &c.id == l) {
                return Err(AppError::Invalid(format!(
                    "unknown environmental layer {l}"
                )));
            }
        }
        if !(1..=20).contains(&spec.resolution_factor) {
            return Err(AppError::Invalid(
                "resolution factor must be between 1 and 20".into(),
            ));
        }
        let m = &spec.model;
        if !(100..=100_000).contains(&m.background_points) {
            return Err(AppError::Invalid(
                "background points must be between 100 and 100 000".into(),
            ));
        }
        if !(0.3..=1.0).contains(&m.collinearity_threshold) {
            return Err(AppError::Invalid(
                "collinearity threshold must be between 0.3 and 1.0".into(),
            ));
        }
        if m.feature_classes.is_empty()
            || m.beta_multipliers.is_empty()
            || m.feature_classes.len() * m.beta_multipliers.len() > 100
        {
            return Err(AppError::Invalid(
                "tuning grid must contain between 1 and 100 settings".into(),
            ));
        }
        // Opt-in bias and cleaning settings (ADR-0013, Science v1 phase 2).
        match m.background_method.as_str() {
            "random" => {
                if m.target_group.is_some() {
                    return Err(AppError::Invalid(
                        "a target group applies only to the target_group background method".into(),
                    ));
                }
            }
            "target_group" => {
                if let Some(g) = &m.target_group
                    && (g.trim().is_empty()
                        || g.len() > 64
                        || !g
                            .chars()
                            .all(|c| c.is_alphanumeric() || " -_(),.".contains(c)))
                {
                    return Err(AppError::Invalid(
                        "target group must be a GBIF class name (e.g. Aves), at most 64 characters; leave it empty for the species' own class".into(),
                    ));
                }
            }
            other => {
                return Err(AppError::Invalid(format!(
                    "background method must be random or target_group (got {other})"
                )));
            }
        }
        // Opt-in second engine (ADR-0017, Science v1 phase 6).
        let algorithm = crate::fit::Algorithm::parse(&m.algorithm)?;
        if algorithm == crate::fit::Algorithm::Maxent {
            if m.rf_trees.is_some() || m.rf_tune_mtry {
                return Err(AppError::Invalid(
                    "random-forest settings (rf_trees, rf_tune_mtry) apply only to the rf and ensemble algorithms".into(),
                ));
            }
        } else if let Some(t) = m.rf_trees
            && !(MIN_RF_TREES..=MAX_RF_TREES).contains(&t)
        {
            return Err(AppError::Invalid(format!(
                "random-forest trees must be between {MIN_RF_TREES} and {MAX_RF_TREES} (got {t})"
            )));
        }
        if let Some(km) = spec.filters.thin_km
            && !(km.is_finite() && km > 0.0 && km <= MAX_THIN_KM)
        {
            return Err(AppError::Invalid(format!(
                "thinning distance must be greater than 0 and at most {MAX_THIN_KM} km (got {km})"
            )));
        }
        let v = &spec.validation;
        if !matches!(
            v.method.as_str(),
            "spatial_block" | "spatial_grid" | "random_kfold"
        ) {
            return Err(AppError::Invalid(
                "validation method must be spatial_block, spatial_grid or random_kfold".into(),
            ));
        }
        // Opt-in evaluation settings (ADR-0014). Folds of random k-fold keep their historical
        // clamp to 2–10 (existing specifications stay valid).
        if v.method == "spatial_grid" && !(2..=10).contains(&v.folds) {
            return Err(AppError::Invalid(format!(
                "spatial grid validation needs between 2 and 10 folds (got {})",
                v.folds
            )));
        }
        if let Some(b) = v.block_km {
            if v.method != "spatial_grid" {
                return Err(AppError::Invalid(
                    "block size (block_km) applies only to spatial_grid validation".into(),
                ));
            }
            if !(b.is_finite() && (MIN_USER_BLOCK_KM..=MAX_USER_BLOCK_KM).contains(&b)) {
                return Err(AppError::Invalid(format!(
                    "block size must be between {MIN_USER_BLOCK_KM} and {MAX_USER_BLOCK_KM} km (got {b})"
                )));
            }
        }
        if v.null_models != 0 && !(MIN_NULL_MODELS..=MAX_NULL_MODELS).contains(&v.null_models) {
            return Err(AppError::Invalid(format!(
                "null models must be 0 (off) or between {MIN_NULL_MODELS} and {MAX_NULL_MODELS} (got {}): with fewer than {MIN_NULL_MODELS} the p-value cannot reach 0.05",
                v.null_models
            )));
        }
        // Opt-in future-climate projections (ADR-0016).
        projection::validate(spec)?;
        // A real study must not carry the demo's SYNTHETIC label into its outputs (AGENTS rule 5).
        if !spec.demo && spec.area.name.contains("SYNTHETIC") {
            return Err(AppError::Invalid(
                "this study area is the synthetic demo region; choose or draw the area for this real study (or rename it)".into(),
            ));
        }
        let polys = env::area_polygons(&spec.area)?;
        if spec.demo
            && !niche_geo::mask::rasterize_polygons(&polys, &demo::REGION)
                .iter()
                .any(|&inside| inside)
        {
            return Err(AppError::Invalid(
                "the synthetic demo has predictors only in its demo region (3–6° E, 43.8–45.8° N, south-east France) and this study area does not overlap it; keep the demo area, or model a real species to use another area".into(),
            ));
        }
        if let Some(limit) = self.cfg.limits.max_area_cells
            && !spec.demo
        {
            let cells = env::area_window(
                &self.http,
                &self.cfg.cache_dir,
                &spec.area,
                spec.resolution_factor,
            )?
            .model_cells();
            if cells > limit {
                return Err(AppError::Invalid(format!(
                    "the study area extent needs {cells} grid cells at this resolution (limit {limit}, about metropolitan France at 1 km); draw a smaller area or choose a coarser resolution"
                )));
            }
        }
        Ok(())
    }

    /// Queue a job (local mode and CLI: no owner, no quota).
    pub fn submit(&self, spec: JobSpec) -> Result<Job, AppError> {
        self.submit_as(spec, None)
    }

    /// Queue a job for a signed-in user: owned by their organisation and counted against its
    /// monthly quota.
    pub fn submit_as(&self, spec: JobSpec, owner: Option<&Principal>) -> Result<Job, AppError> {
        self.submit_in(spec, owner, None)
    }

    /// Submit a study, optionally inside a project of the owner's organisation (checked before
    /// anything is queued). The project is organisational only: not part of the spec or its hash.
    pub fn submit_in(
        &self,
        spec: JobSpec,
        owner: Option<&Principal>,
        project: Option<&str>,
    ) -> Result<Job, AppError> {
        self.validate(&spec)?;
        let scope = owner.map_or(Scope::All, Principal::scope);
        if let Some(p) = project {
            self.store.project(p, &scope)?;
        }
        let id = uuid::Uuid::now_v7().to_string();
        // The organisation's dataset (ADR-0108): another organisation's is "not found"; it is
        // copied into the study before the study exists, so the worker never sees it without.
        if let Some(d) = &spec.own_dataset {
            let info = self.copy_dataset_to_job(d, &scope, &id).inspect_err(|_| {
                let _ = std::fs::remove_dir_all(self.cfg.data_dir.join("jobs").join(&id));
            })?;
            let known: std::collections::HashSet<&str> =
                info.report.taxa.iter().map(|t| t.value.as_str()).collect();
            for sp in &spec.species {
                if let Some(o) = &sp.own
                    && o.values.iter().any(|v| !known.contains(v.as_str()))
                {
                    let _ = std::fs::remove_dir_all(self.cfg.data_dir.join("jobs").join(&id));
                    return Err(AppError::Invalid(format!(
                        "{}: a taxon value is not in the dataset",
                        sp.scientific_name
                    )));
                }
            }
        }
        let job = match self.store.create(&id, &spec, &spec_hash(&spec), owner) {
            Ok(j) => j,
            Err(e) => {
                let _ = std::fs::remove_dir_all(self.cfg.data_dir.join("jobs").join(&id));
                return Err(e);
            }
        };
        if project.is_none() {
            return Ok(job);
        }
        self.store.job_update(&id, &scope, None, Some(project))?;
        self.store
            .get(&id)?
            .ok_or_else(|| AppError::Internal("job vanished".into()))
    }

    /// The job, if `scope` may see it. Every job-level API route calls this first.
    pub fn job(&self, id: &str, scope: &Scope) -> Result<Job, AppError> {
        self.store
            .get_scoped(id, scope)?
            .ok_or_else(|| AppError::NotFound("job".into()))
    }

    /// Delete a finished job: its files, rows and cached rasters. The usage ledger is kept.
    pub fn delete_job(&self, id: &str) -> Result<(), AppError> {
        let job = self
            .store
            .get(id)?
            .ok_or_else(|| AppError::NotFound("job".into()))?;
        if !job.status.is_terminal() {
            return Err(AppError::Conflict(
                "this job is still queued or running; cancel it before deleting it".into(),
            ));
        }
        let dir = self.cfg.data_dir.join("jobs").join(id);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(AppError::Storage(format!("{}: {e}", dir.display()))),
        }
        self.store.delete(id)?;
        if let Ok(mut r) = self.rasters.lock() {
            r.remove_prefix(&format!("{id}/"));
        }
        Ok(())
    }

    /// Delete finished jobs older than the retention period. Returns the deleted ids.
    pub fn sweep_expired(&self) -> Result<Vec<String>, AppError> {
        if self.cfg.retention_days == 0 {
            return Ok(Vec::new());
        }
        let cutoff = jiff::Timestamp::now()
            .checked_sub(jiff::SignedDuration::from_hours(
                24 * self.cfg.retention_days.min(100_000) as i64,
            ))
            .map_err(|e| AppError::Internal(e.to_string()))?
            .to_string();
        let ids = self.store.finished_before(&cutoff)?;
        for id in &ids {
            self.delete_job(id)?;
            tracing::info!(job_id = %id, retention_days = self.cfg.retention_days, "expired job deleted");
        }
        // Imported datasets follow the same retention (ADR-0108).
        self.sweep_expired_datasets()?;
        Ok(ids)
    }

    /// State of the instance for administrators. `build` and `volume_backup` belong to the
    /// deployment and are filled by the server.
    pub fn system_status(&self) -> Result<SystemStatus, AppError> {
        use crate::system;
        let data = &self.cfg.data_dir;
        let db_bytes = ["niche.db", "niche.db-wal", "niche.db-shm"]
            .iter()
            .map(|f| system::dir_size(&data.join(f)))
            .sum();
        let backups = system::backups_dir(data);
        Ok(SystemStatus {
            version: env!("CARGO_PKG_VERSION").into(),
            build: String::new(),
            started_at: self.started_at.clone(),
            data_dir: data.display().to_string(),
            disk: system::disk_space(data),
            storage: StorageUse {
                database_bytes: db_bytes,
                jobs_bytes: system::dir_size(&data.join("jobs")),
                cache_bytes: system::dir_size(&self.cfg.cache_dir),
                boundaries_bytes: system::dir_size(&data.join("boundaries")),
                backups_bytes: system::dir_size(&backups),
            },
            jobs: self.store.job_counts()?,
            boundaries: self
                .boundary_packs()
                .into_iter()
                .map(|(name, units)| PackStatus { name, units })
                .collect(),
            db_backups: system::db_backups(&backups),
            volume_backup: None,
            gbif: self.gbif_account(),
            gbif_mirror: self.mirror_status(),
            retention_days: self.cfg.retention_days,
        })
    }

    /// Whether GBIF accepts the configured account (checked at most every 10 minutes).
    fn gbif_account(&self) -> GbifAccount {
        let Some(creds) = self.cfg.gbif.as_ref() else {
            return GbifAccount {
                configured: false,
                user: None,
                ok: None,
                detail: None,
                checked_at: None,
            };
        };
        if let Ok(c) = self.gbif_check.lock()
            && let Some((at, acc)) = c.as_ref()
            && at.elapsed() < Duration::from_secs(600)
        {
            return acc.clone();
        }
        let (ok, detail) = self.gbif_status(creds);
        let acc = GbifAccount {
            configured: true,
            user: Some(creds.username.clone()),
            ok,
            detail,
            checked_at: Some(crate::store::now()),
        };
        if let Ok(mut c) = self.gbif_check.lock() {
            *c = Some((std::time::Instant::now(), acc.clone()));
        }
        acc
    }

    /// Ask GBIF whether it accepts an account: `Some(true)`, `Some(false)` (refused), or `None`
    /// with the reason when GBIF could not answer.
    fn gbif_status(&self, creds: &Credentials) -> (Option<bool>, Option<String>) {
        let url = format!(
            "https://api.gbif.org/v1/occurrence/download/user/{}?limit=1",
            creds.username
        );
        match self
            .http
            .status_basic(&url, &creds.username, &creds.password)
        {
            Ok(200) => (Some(true), None),
            Ok(401 | 403) => (
                Some(false),
                Some("GBIF refused the credentials".to_string()),
            ),
            Ok(code) => (None, Some(format!("GBIF answered HTTP {code}"))),
            Err(_) => (None, Some("GBIF unreachable".to_string())),
        }
    }

    // ---- GBIF accounts of users and organisations (ADR-0110) -------------------------------

    fn secret_key(&self) -> Result<&crate::secrets::SecretKey, AppError> {
        self.cfg.secret_key.as_ref().ok_or_else(|| {
            AppError::Conflict(
                "this server cannot store GBIF accounts (NICHE_SECRET_KEY is not set)".into(),
            )
        })
    }

    fn account_info(stored: Option<StoredGbif>, show_username: bool) -> Option<GbifAccountInfo> {
        stored.map(|g| GbifAccountInfo {
            username: show_username.then_some(g.username),
            checked_at: g.checked_at,
            ok: g.ok,
        })
    }

    /// Usable for downloads: stored, not refused by GBIF at its last check, and openable.
    fn usable(&self, owner: &GbifOwner) -> Option<Credentials> {
        let g = self.store.gbif_get(owner).ok().flatten()?;
        if g.ok == Some(false) {
            return None;
        }
        let key = self.cfg.secret_key.as_ref()?;
        match key.open(&owner.tag(), &g.sealed) {
            Ok(password) => Some(Credentials {
                username: g.username,
                password,
            }),
            Err(e) => {
                tracing::warn!(account = %owner.tag(), error = %e, "stored GBIF account unusable");
                None
            }
        }
    }

    /// The account a study of this user would use now: theirs, their organisation's, the
    /// shared one.
    fn choose_account(
        &self,
        user_id: Option<&str>,
        org_id: Option<&str>,
    ) -> (GbifSource, Option<(GbifOwner, Credentials)>) {
        for (source, owner) in [
            (
                GbifSource::User,
                user_id.map(|u| GbifOwner::User(u.to_string())),
            ),
            (
                GbifSource::Organisation,
                org_id.map(|o| GbifOwner::Org(o.to_string())),
            ),
        ] {
            if let Some(owner) = owner
                && let Some(c) = self.usable(&owner)
            {
                return (source, Some((owner, c)));
            }
        }
        if self.cfg.gbif.is_some() {
            (GbifSource::Shared, None)
        } else {
            (GbifSource::None, None)
        }
    }

    /// `GET /account/gbif`.
    pub fn gbif_settings(&self, p: &Principal) -> Result<GbifSettings, AppError> {
        let own = self.store.gbif_get(&GbifOwner::User(p.user_id.clone()))?;
        let org = self.store.gbif_get(&GbifOwner::Org(p.org_id.clone()))?;
        Ok(GbifSettings {
            own: Self::account_info(own, true),
            organisation: Self::account_info(org, p.manages_org()),
            shared: self.cfg.gbif.is_some(),
            effective: self.choose_account(Some(&p.user_id), Some(&p.org_id)).0,
            can_store: self.cfg.secret_key.is_some(),
        })
    }

    /// `GET /org`.
    pub fn org_profile(&self, p: &Principal) -> Result<OrgProfile, AppError> {
        let org = self.store.gbif_get(&GbifOwner::Org(p.org_id.clone()))?;
        Ok(OrgProfile {
            name: p.org_name.clone(),
            quota: self.store.quota(&p.org_id, p.monthly_quota)?,
            members: self.store.member_count(&p.org_id)?,
            gbif: Self::account_info(org, p.manages_org()),
            shared_gbif: self.cfg.gbif.is_some(),
            can_store: self.cfg.secret_key.is_some(),
            can_manage: p.manages_org(),
        })
    }

    /// Store an account, sealed; it is checked separately ([`Engine::gbif_check`]).
    pub fn gbif_store(
        &self,
        owner: &GbifOwner,
        username: &str,
        password: &str,
    ) -> Result<(), AppError> {
        let username = username.trim();
        if username.is_empty() || username.len() > 100 || username.contains(['/', '?', '#', ' ']) {
            return Err(AppError::Invalid(
                "enter your GBIF user name (not your e-mail address)".into(),
            ));
        }
        if password.is_empty() || password.len() > 200 {
            return Err(AppError::Invalid("enter your GBIF password".into()));
        }
        let sealed = self.secret_key()?.seal(&owner.tag(), password)?;
        self.store.gbif_set(owner, username, &sealed)
    }

    /// Ask GBIF whether it accepts a stored account and remember the answer.
    pub fn gbif_check(&self, owner: &GbifOwner) -> Result<GbifAccountInfo, AppError> {
        let g = self
            .store
            .gbif_get(owner)?
            .ok_or_else(|| AppError::NotFound("GBIF account".into()))?;
        let password = self.secret_key()?.open(&owner.tag(), &g.sealed)?;
        let (ok, detail) = self.gbif_status(&Credentials {
            username: g.username.clone(),
            password,
        });
        self.store.gbif_mark_checked(owner, ok)?;
        if let Some(d) = detail {
            tracing::info!(account = %owner.tag(), detail = %d, "GBIF account checked");
        }
        Ok(
            Self::account_info(self.store.gbif_get(owner)?, true).unwrap_or(GbifAccountInfo {
                username: Some(g.username),
                checked_at: None,
                ok,
            }),
        )
    }

    /// The account a job's download uses, fixed the first time it is asked (preparation): its
    /// tag (`user:<id>`, `org:<id>`, `shared`, `none`), credentials and download-cache directory.
    /// Each account has its own cache, so a study never cites a download made with another
    /// organisation's account; the shared account keeps the original cache.
    fn job_account(&self, id: &str) -> Result<(String, Option<(Credentials, PathBuf)>), AppError> {
        let owner = self.store.job_owner(id)?.unwrap_or(JobOwner {
            org_id: None,
            created_by: None,
            gbif_account: None,
        });
        let fixed = owner.gbif_account.as_deref();
        let own = |o: &GbifOwner, c: Credentials| {
            let dir = self
                .cfg
                .cache_dir
                .join("gbif-accounts")
                .join(&crate::accounts::token_hash(&c.username)[..16]);
            (o.tag(), Some((c, dir)))
        };
        // A fixed account that is still usable stays; otherwise (removed, refused) choose again.
        if let Some(tag) = fixed {
            if let Some(o) = GbifOwner::parse(tag)
                && let Some(c) = self.usable(&o)
            {
                return Ok(own(&o, c));
            }
            if tag == "shared"
                && let Some(c) = self.cfg.gbif.clone()
            {
                return Ok(("shared".into(), Some((c, self.cfg.cache_dir.clone()))));
            }
        }
        let (source, chosen) =
            self.choose_account(owner.created_by.as_deref(), owner.org_id.as_deref());
        let out = match (source, chosen, self.cfg.gbif.clone()) {
            (_, Some((o, c)), _) => own(&o, c),
            (GbifSource::Shared, None, Some(c)) => {
                ("shared".into(), Some((c, self.cfg.cache_dir.clone())))
            }
            _ => ("none".into(), None),
        };
        if fixed != Some(out.0.as_str()) {
            self.store.set_job_gbif_account(id, &out.0)?;
        }
        Ok(out)
    }

    /// Whose GBIF account a job's download uses (fixed when its preparation starts).
    pub fn job_gbif_source(&self, id: &str) -> Result<GbifSource, AppError> {
        let (tag, _) = self.job_account(id)?;
        Ok(match tag.split(':').next() {
            Some("user") => GbifSource::User,
            Some("org") => GbifSource::Organisation,
            Some("shared") => GbifSource::Shared,
            _ => GbifSource::None,
        })
    }

    /// Daily copy of the database into `<data_dir>/backups/` (newest 7 kept). Returns the new
    /// copy, or `None` when the last one is less than a day old.
    pub fn backup_db_if_due(&self) -> Result<Option<std::path::PathBuf>, AppError> {
        use crate::system;
        let dir = system::backups_dir(&self.cfg.data_dir);
        if !system::backup_due(&dir) {
            return Ok(None);
        }
        std::fs::create_dir_all(&dir).map_err(|e| AppError::Storage(e.to_string()))?;
        let stamp = jiff::Timestamp::now().strftime("%Y%m%d-%H%M%S").to_string();
        let path = dir.join(format!("niche-{stamp}.db"));
        let tmp = dir.join(format!(".niche-{stamp}.tmp"));
        let _ = std::fs::remove_file(&tmp);
        self.store.vacuum_into(&tmp)?;
        std::fs::rename(&tmp, &path).map_err(|e| AppError::Storage(e.to_string()))?;
        system::prune(&dir, system::DB_BACKUPS_KEEP);
        tracing::info!(file = %path.display(), "database copied");
        Ok(Some(path))
    }

    /// The GBIF download a queued study is waiting for, if any (the preparing process keeps it
    /// in memory and in `jobs/<id>/.gbif-wait.json` for the other process of a deployment).
    pub fn gbif_wait(&self, id: &str) -> Option<GbifWait> {
        if let Some(w) = self.waits.lock().ok()?.get(id).cloned() {
            return Some(w);
        }
        let preparing = self
            .store
            .get(id)
            .ok()
            .flatten()
            .is_some_and(|j| j.status == JobStatus::Queued)
            && self.store.data_state(id).ok().flatten() == Some(DataState::Preparing);
        if !preparing {
            return None;
        }
        serde_json::from_slice(&std::fs::read(self.job_file(id, ".gbif-wait.json")).ok()?).ok()
    }

    /// Continue a queued study without its GBIF download: occurrences then come from the GBIF
    /// search API per species (no DOI, at most `max_records` per species). Only while the
    /// download is being prepared, and only once it has taken unusually long.
    pub fn skip_download(&self, id: &str) -> Result<(), AppError> {
        let job = self
            .store
            .get(id)?
            .ok_or_else(|| AppError::NotFound(format!("job {id}")))?;
        let waited = self
            .gbif_wait(id)
            .and_then(|w| w.since.parse::<jiff::Timestamp>().ok())
            .map(|t| jiff::Timestamp::now().duration_since(t).unsigned_abs());
        if job.status != JobStatus::Queued
            || self.store.data_state(id)? != Some(DataState::Preparing)
            || waited.is_none_or(|w| w < GBIF_SKIP_AFTER)
        {
            return Err(AppError::Conflict(
                "this study is not waiting for a slow GBIF download".into(),
            ));
        }
        // A file, not memory: seen by the process preparing the job, whichever received the
        // request (ADR-0115).
        self.touch(&self.job_file(id, ".skip-download"))?;
        self.store.event(
            id,
            &format!(
                "Continuing without the GBIF download at the user's request: occurrences come from the GBIF search API (no DOI, at most {} records per species)",
                self.cfg.max_records
            ),
            "warning",
            None,
        )?;
        Ok(())
    }

    /// The user chose to continue this queued study without its GBIF download.
    fn skipping(&self, id: &str) -> bool {
        self.job_file(id, ".skip-download").exists()
    }

    /// Record what the GBIF download of a study is doing (see [`GbifWait`]).
    fn note_download(&self, id: &str, step: &niche_data::gbif_download::Step) {
        use niche_data::gbif_download::Step;
        let Ok(mut waits) = self.waits.lock() else {
            return;
        };
        let now = crate::store::now();
        let mut gbif_created: Option<String> = None;
        let (key, status) = match step {
            Step::Requesting => (None, "requesting".to_string()),
            Step::Waiting {
                key,
                status,
                created,
                ..
            } => {
                // GBIF's own creation time, when given, is the true start of the wait.
                if let Some(t) = created.and_then(|c| c.parse::<jiff::Timestamp>().ok()) {
                    gbif_created = Some(t.to_string());
                }
                (Some(key.to_string()), status.to_lowercase())
            }
            Step::Downloading { key, .. } => (Some(key.to_string()), "downloading".into()),
            Step::Cached { .. } | Step::Splitting => {
                waits.remove(id);
                let _ = std::fs::remove_file(self.job_file(id, ".gbif-wait.json"));
                return;
            }
        };
        let since = gbif_created.unwrap_or_else(|| {
            waits
                .get(id)
                .filter(|w| w.key.is_none() || w.key == key)
                .map_or_else(|| now.clone(), |w| w.since.clone())
        });
        let wait = GbifWait {
            url: key
                .as_ref()
                .map(|k| format!("https://www.gbif.org/occurrence/download/{k}")),
            key,
            status,
            since,
            checked_at: now,
            slow_after_s: GBIF_SLOW_AFTER.as_secs(),
            skip_after_s: GBIF_SKIP_AFTER.as_secs(),
            search_api_max_records: self.cfg.max_records,
        };
        // For the other server process of a deployment (`gbif_wait`).
        if let Ok(bytes) = serde_json::to_vec(&wait) {
            let _ = crate::artifacts::write_atomic(&self.job_file(id, ".gbif-wait.json"), &bytes);
        }
        waits.insert(id.to_string(), wait);
    }

    fn end_download(&self, id: &str) {
        if let Ok(mut w) = self.waits.lock() {
            w.remove(id);
        }
        let _ = std::fs::remove_file(self.job_file(id, ".gbif-wait.json"));
        let _ = std::fs::remove_file(self.job_file(id, ".skip-download"));
    }

    pub fn cancel(&self, id: &str) -> Result<Job, AppError> {
        let job = self
            .store
            .get(id)?
            .ok_or_else(|| AppError::NotFound(format!("job {id}")))?;
        match job.status {
            JobStatus::Queued | JobStatus::Interrupted => {
                self.store
                    .set_status(id, JobStatus::Cancelled, "Cancelled before start", None)?;
                self.store.event(id, "Cancelled by user", "warning", None)?;
            }
            JobStatus::Running => {
                if let Some(flag) = self
                    .cancels
                    .lock()
                    .map_err(|_| AppError::Internal("lock".into()))?
                    .get(id)
                {
                    flag.store(true, Ordering::Relaxed);
                }
                // The job may be running in the other process of a deployment: it watches this.
                self.touch(&self.job_file(id, ".cancel"))?;
                self.store.event(
                    id,
                    "Cancellation requested; stopping at the next checkpoint",
                    "warning",
                    None,
                )?;
            }
            _ => {}
        }
        self.store
            .get(id)?
            .ok_or_else(|| AppError::NotFound(id.into()))
    }

    fn fetch(
        &self,
        species: &SpeciesRef,
        stack: &EnvStack,
        area: &StudyArea,
        demo: bool,
        seed: u64,
        cancel: &AtomicBool,
    ) -> Result<Fetched, AppError> {
        if !uses_gbif(species) {
            return Ok((
                Vec::new(),
                0,
                false,
                "The organisation's own data only (no GBIF records requested)".into(),
                None,
                None,
            ));
        }
        if demo {
            let o = demo::occurrences_for(species.key, seed);
            let n = o.len() as u64;
            return Ok((
                o,
                n,
                false,
                "Synthetic demo records (virtual species)".into(),
                None,
                None,
            ));
        }
        let s = stack.spec;
        if let Some((r, n, t, src, q)) =
            self.mirror_fetch(species.key, area, (s.west, s.south(), s.east(), s.north))
        {
            return Ok((r, n, t, src, q, Some(gbif::RecordSource::Mirror)));
        }
        let f = gbif::fetch_occurrences_cached(
            &self.http,
            &self.cfg.cache_dir,
            species.key,
            (s.west, s.south(), s.east(), s.north),
            self.cfg.max_records,
            30,
            &|| cancel.load(Ordering::Relaxed),
            &jiff::Timestamp::now().to_string(),
        )
        .map_err(|e| match e {
            niche_data::DataError::Cancelled => AppError::Cancelled,
            other => AppError::Data(other),
        })?;
        Ok((
            f.records,
            f.total_matching,
            f.truncated,
            format!(
                "GBIF occurrence search API, accessed {}{}",
                f.accessed_at,
                if f.from_cache {
                    " (served from local cache)"
                } else {
                    ""
                }
            ),
            Some(f.query_url),
            Some(gbif::RecordSource::SearchApi),
        ))
    }

    /// Data review step: occurrences retrieved, cleaned and summarized on the modelling grid.
    /// Local mode and CLI (every dataset without organisation).
    pub fn preview(&self, req: &DataPreviewRequest) -> Result<DataPreview, AppError> {
        self.preview_in(req, &Scope::All)
    }

    /// Data review within an organisation's scope (its own datasets only, ADR-0108).
    pub fn preview_in(
        &self,
        req: &DataPreviewRequest,
        scope: &Scope,
    ) -> Result<DataPreview, AppError> {
        let own_data = match (&req.own_dataset, &req.own) {
            (Some(d), Some(_)) => {
                let (info, records) = self.dataset_records(&d.id, scope)?;
                if info.records_sha256 != d.records_sha256 {
                    return Err(AppError::Conflict(
                        "the organisation's dataset changed; choose it again".into(),
                    ));
                }
                Some((info, records))
            }
            (None, None) => None,
            _ => {
                return Err(AppError::Invalid("own_dataset and own go together".into()));
            }
        };
        let layers: Vec<String> = env::layer_catalogue(req.demo)
            .into_iter()
            .filter(|l| l.default_selected)
            .map(|l| l.id)
            .take(1)
            .collect();
        let stack = build_stack(
            &self.http,
            &self.cfg.cache_dir,
            &req.area,
            &layers,
            req.resolution_factor,
            req.demo,
            &|_, _| {},
        )?;
        let name = if req.demo {
            demo::species_name(req.species_key).to_string()
        } else {
            gbif::species_name(&self.http, req.species_key)?
        };
        let species = SpeciesRef {
            key: req.species_key,
            scientific_name: name.clone(),
            own: req.own.clone(),
        };
        if req.own.is_some() {
            validate_own_data(&JobSpec {
                name: String::new(),
                species: vec![species.clone()],
                area: req.area.clone(),
                layers: Vec::new(),
                resolution_factor: req.resolution_factor,
                filters: req.filters.clone(),
                model: ModelSettings::default(),
                validation: ValidationSettings::default(),
                seed: req.seed,
                demo: req.demo,
                projections: Vec::new(),
                own_dataset: req.own_dataset.clone(),
            })?;
        }
        let never = AtomicBool::new(false);
        let (records, total, truncated, source, query_url, origin) =
            self.fetch(&species, &stack, &req.area, req.demo, 1, &never)?;
        let data = crate::owndata::species_data(
            &records,
            match (&own_data, &req.own) {
                (Some((info, recs)), Some(u)) => Some(crate::owndata::OwnInput {
                    use_: u,
                    dataset: info,
                    records: recs,
                }),
                _ => None,
            },
            uses_gbif(&species),
            &stack,
            &req.filters,
            thin_seed(req.seed, req.species_key),
        );
        let c = data.cleaned;
        let mut warnings = Vec::new();
        if !req.demo
            && let Some(w) = rank_warning(&gbif::taxon_rank(&self.http, req.species_key)?)
        {
            warnings.push(w);
        }
        data_warnings(
            &mut warnings,
            c.presence_cells.len(),
            truncated,
            total,
            records.len(),
            &c.licences,
            req.demo,
        );
        warnings.extend(generalised_warnings(
            &c,
            origin.map(gbif::generalisation_flags_available),
            req.filters.max_uncertainty_m,
        ));
        if own_data.is_some() {
            warnings.extend(crate::owndata::concentration_warning(
                &stack,
                &crate::owndata::own_presence_cells(&c.points, &stack),
            ));
        }
        Ok(DataPreview {
            species_key: req.species_key,
            scientific_name: name,
            source,
            query_url,
            records_total_matching: total,
            records_fetched: records.len(),
            truncated,
            retained_cells: c.presence_cells.len(),
            exclusions: c.exclusions,
            licences: c.licences,
            points: c.points,
            grid: stack.grid_info(),
            warnings,
            by_source: data.by_source,
            validation_sites: data.sites.map(|s| s.summary),
        })
    }

    /// Worker loop (blocking): runs queued jobs one at a time until `stop` is set. CPU work inside
    /// a job uses the global Rayon pool, sized by NICHE_MAX_WORKERS (ADR-0103).
    pub fn worker_loop(&self, stop: &AtomicBool) {
        match self.store.recover_interrupted() {
            Ok(ids) if !ids.is_empty() => tracing::warn!(jobs = ?ids, "requeued interrupted jobs"),
            Err(e) => tracing::error!(error = %e, "recovery failed"),
            _ => {}
        }
        let mut last_sweep: Option<std::time::Instant> = None;
        while !stop.load(Ordering::Relaxed) {
            if last_sweep.is_none_or(|t| t.elapsed() >= Duration::from_secs(3600)) {
                last_sweep = Some(std::time::Instant::now());
                if let Err(e) = self.sweep_expired() {
                    tracing::error!(error = %e, "retention sweep failed");
                }
                if let Err(e) = self.backup_db_if_due() {
                    tracing::error!(error = %e, "database copy failed");
                }
            }
            match self.store.next_queued() {
                Ok(Some(id)) => {
                    let flag = Arc::new(AtomicBool::new(false));
                    if let Ok(mut m) = self.cancels.lock() {
                        m.insert(id.clone(), flag.clone());
                    }
                    // Shutdown also cancels the running job; it is then marked interrupted and
                    // resumes on next start (completed species are kept).
                    let watcher_stop = std::thread::scope(|s| {
                        let f2 = flag.clone();
                        let job: &str = &id;
                        let h = s.spawn(move || {
                            while !f2.load(Ordering::Relaxed) {
                                if stop.load(Ordering::Relaxed) {
                                    f2.store(true, Ordering::Relaxed);
                                    return true;
                                }
                                if self.cancel_requested(job) {
                                    f2.store(true, Ordering::Relaxed);
                                    return false;
                                }
                                std::thread::sleep(Duration::from_millis(200));
                            }
                            false
                        });
                        self.run_job(&id, &flag, stop);
                        flag.store(true, Ordering::Relaxed);
                        h.join().unwrap_or(false)
                    });
                    tracing::info!(job_id = %id, shutdown = watcher_stop, "job loop iteration finished");
                    if let Ok(mut m) = self.cancels.lock() {
                        m.remove(&id);
                    }
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(400)),
                Err(e) => {
                    tracing::error!(error = %e, "queue poll failed");
                    std::thread::sleep(Duration::from_secs(2));
                }
            }
        }
    }

    /// Data-preparation loop (blocking, I/O only): requests and waits for the GBIF batch download
    /// of queued jobs, so the compute worker never sits idle in GBIF's queue (observed 23 min) while
    /// other jobs wait. GBIF limits concurrent downloads per account: at most three per account
    /// (users' and organisations' own accounts each count apart, ADR-0110), twelve in all.
    pub fn prepare_loop(&self, stop: &AtomicBool) {
        const PER_ACCOUNT: usize = 3;
        const MAX_IN_FLIGHT: usize = 12;
        std::thread::scope(|s| {
            let mut running: Vec<(String, std::thread::ScopedJoinHandle<'_, ()>)> = Vec::new();
            while !stop.load(Ordering::Relaxed) {
                running.retain(|(_, h)| !h.is_finished());
                if running.len() < MAX_IN_FLIGHT {
                    match self.next_to_prepare(&running, PER_ACCOUNT) {
                        Ok(Some((id, account))) => {
                            running.push((account, s.spawn(move || self.prepare_job(&id, stop))));
                            continue;
                        }
                        Ok(None) => {}
                        Err(e) => tracing::error!(error = %e, "preparation queue poll failed"),
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        });
    }

    /// Oldest pending job whose account has a free download slot, claimed for preparation.
    fn next_to_prepare<T>(
        &self,
        running: &[(String, T)],
        per_account: usize,
    ) -> Result<Option<(String, String)>, AppError> {
        for id in self.store.pending_preparation(50)? {
            // Studies served by the regional mirror wait for no GBIF download.
            let mirrored = self
                .store
                .get(&id)?
                .is_some_and(|j| !j.spec.demo && self.mirror_for(&j.spec.area).is_some());
            let account = if mirrored {
                "mirror".to_string()
            } else {
                self.job_account(&id)?.0
            };
            let limit = if account == "shared" && self.mirror_at_gbif.load(Ordering::Relaxed) {
                per_account.saturating_sub(1).max(1)
            } else {
                per_account
            };
            let busy = running.iter().filter(|(a, _)| *a == account).count();
            let free = matches!(account.as_str(), "none" | "mirror") || busy < limit;
            if free && self.store.claim_preparation(&id)? {
                return Ok(Some((id, account)));
            }
        }
        Ok(None)
    }

    /// Prepare one job's occurrence data. Outcome: `Ready` (batch cached, or nothing to
    /// prepare), `NoBatch` (failed: model with the search API), or back to `Pending` on shutdown.
    fn prepare_job(&self, id: &str, stop: &AtomicBool) {
        let span = tracing::info_span!("prepare", job_id = %id);
        let _g = span.enter();
        let cancelled = || {
            stop.load(Ordering::Relaxed)
                || self.skipping(id)
                || self
                    .store
                    .get(id)
                    .ok()
                    .flatten()
                    .is_none_or(|j| j.status != JobStatus::Queued)
        };
        let state = match self.store.get(id) {
            Ok(Some(job)) => match self.prefetch_batch(id, &job.spec, &cancelled) {
                Ok(_) => DataState::Ready,
                // The user chose to continue without the download (search API per species).
                Err(AppError::Cancelled) if self.skipping(id) => DataState::NoBatch,
                Err(AppError::Cancelled) if stop.load(Ordering::Relaxed) => DataState::Pending,
                Err(AppError::Cancelled) => DataState::Ready,
                Err(e) => {
                    let _ = self.store.event(
                        id,
                        &format!("Occurrence data could not be prepared in advance ({e}); the GBIF search API will be used per species"),
                        "warning",
                        None,
                    );
                    DataState::NoBatch
                }
            },
            Ok(None) => return,
            Err(e) => {
                tracing::error!(error = %e, "reading job for preparation");
                DataState::NoBatch
            }
        };
        self.end_download(id);
        if let Err(e) = self.store.set_data_state(id, state) {
            tracing::error!(error = %e, "saving data preparation state");
        }
    }

    /// Run one job to completion (also used by the CLI batch mode).
    pub fn run_job(&self, id: &str, cancel: &AtomicBool, shutdown: &AtomicBool) {
        let span = tracing::info_span!("job", job_id = %id);
        let _g = span.enter();
        let outcome = self.run_job_inner(id, cancel);
        self.end_download(id);
        let _ = std::fs::remove_file(self.job_file(id, ".cancel"));
        self.drop_mirror_extract(id);
        // A failed job keeps the stage it failed in, so users see where it stopped.
        let failed_stage = || {
            self.store
                .get(id)
                .ok()
                .flatten()
                .map(|j| j.stage)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Failed".to_string())
        };
        let (status, stage, err) = match &outcome {
            Ok(()) => (JobStatus::Succeeded, "Completed".to_string(), None),
            Err(AppError::Cancelled) if shutdown.load(Ordering::Relaxed) => (
                JobStatus::Running,
                "Interrupted by shutdown".to_string(),
                None,
            ),
            Err(AppError::Cancelled) => (JobStatus::Cancelled, "Cancelled".to_string(), None),
            Err(e) => (JobStatus::Failed, failed_stage(), Some(e.to_string())),
        };
        if status == JobStatus::Running {
            // Leave as running: recover_interrupted() requeues it on next start.
            let _ = self.store.event(
                id,
                "Stopped by server shutdown; will resume on next start",
                "warning",
                None,
            );
            return;
        }
        // Finished one way or another: nothing left to resume.
        if let Some(d) = checkpoint::checkpoint_dir(&self.cfg.data_dir, id, 0).parent() {
            checkpoint::clear(d);
        }
        // Event first: a progress stream ends once it sees a terminal status, and must not end
        // before the final message is written.
        let _ = self.store.event(
            id,
            &match &err {
                None if status == JobStatus::Succeeded => "Job completed".to_string(),
                None => "Job cancelled".to_string(),
                Some(e) => format!("Job failed: {e}"),
            },
            if err.is_some() { "error" } else { "info" },
            None,
        );
        let _ = self.store.set_status(id, status, &stage, err.as_deref());
        tracing::info!(status = status.as_str(), "job finished");
    }

    fn run_job_inner(&self, id: &str, cancel: &AtomicBool) -> Result<(), AppError> {
        let job = self
            .store
            .get(id)?
            .ok_or_else(|| AppError::NotFound(id.into()))?;
        let spec = job.spec.clone();
        self.store
            .set_status(id, JobStatus::Running, "Preparing environmental data", None)?;
        self.store.event(id, "Started", "info", None)?;
        let t0 = std::time::Instant::now();
        let progress_env = |msg: &str, p: f64| {
            let _ = self.store.set_progress(id, 0.15 * p, msg);
        };
        let stack = env::build_stack_cancellable(
            &self.http,
            &self.cfg.cache_dir,
            &spec.area,
            &spec.layers,
            spec.resolution_factor,
            spec.demo,
            &progress_env,
            &|| cancel.load(Ordering::Relaxed),
        )?;
        let gi = stack.grid_info();
        self.store.event(
            id,
            &format!("Environmental grid ready: {} × {} cells, {} land cells in study area ({:.0} km²) in {:.1} s", gi.width, gi.height, gi.cells_in_area, gi.area_km2, t0.elapsed().as_secs_f64()),
            "info",
            None,
        )?;
        let mut summaries = job.species.clone();
        let n_sp = spec.species.len();
        let mut checksums = serde_json::Map::new();
        let batch = if self.store.data_state(id)? == Some(DataState::NoBatch) {
            None
        } else {
            self.prefetch_batch(id, &spec, &|| cancel.load(Ordering::Relaxed))?
        };
        let download = batch.as_ref().map(|b| b.info.clone());
        for (i, sp) in spec.species.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Cancelled);
            }
            let dir = species_dir(&self.cfg.data_dir, id, sp.key);
            if let Ok(prev) = std::fs::read_to_string(dir.join("result.json"))
                && let Ok(r) = serde_json::from_str::<SpeciesResult>(&prev)
            {
                summaries[i].status = r.status;
                summaries[i].evidence_level = Some(r.evidence.level);
                if r.status == SpeciesRunStatus::Succeeded {
                    self.store.record_usage(id, sp.key)?;
                }
                self.store.set_species(id, &summaries)?;
                self.store.event(
                    id,
                    &format!(
                        "{}: already completed, skipped (resume)",
                        sp.scientific_name
                    ),
                    "info",
                    Some(sp.key),
                )?;
                continue;
            }
            summaries[i].status = SpeciesRunStatus::Running;
            self.store.set_species(id, &summaries)?;
            let base = 0.15 + 0.85 * i as f64 / n_sp as f64;
            let span = 0.85 / n_sp as f64;
            let name = sp.scientific_name.clone();
            let stage = |msg: &str, p: f64| {
                let _ = self
                    .store
                    .set_progress(id, base + span * p, &format!("{name}: {msg}"));
            };
            self.store.event(
                id,
                &format!("{name}: retrieving occurrences"),
                "info",
                Some(sp.key),
            )?;
            let res = self.run_species(
                id,
                &spec,
                sp,
                &stack,
                cancel,
                &stage,
                batch
                    .as_ref()
                    .filter(|b| uses_gbif(sp) && b.taxa.contains(&sp.key))
                    .and_then(|b| match b.load(sp.key) {
                        Ok(f) => Some(f),
                        Err(e) => {
                            tracing::warn!(species_key = sp.key, error = %e, "batch records unavailable; using search API");
                            None
                        }
                    }),
            );
            match res {
                Ok((r, sums)) => {
                    summaries[i].status = r.status;
                    summaries[i].evidence_level = Some(r.evidence.level);
                    if r.status == SpeciesRunStatus::Succeeded {
                        self.store.record_usage(id, sp.key)?;
                    }
                    checksums.insert(sp.key.to_string(), sums);
                    self.store.event(
                        id,
                        &format!(
                            "{name}: {} — evidence level {}",
                            format!("{:?}", r.status).to_lowercase(),
                            format!("{:?}", r.evidence.level).to_lowercase()
                        ),
                        if r.status == SpeciesRunStatus::Succeeded {
                            "info"
                        } else {
                            "warning"
                        },
                        Some(sp.key),
                    )?;
                }
                Err(AppError::Cancelled) => {
                    summaries[i].status = SpeciesRunStatus::Cancelled;
                    self.store.set_species(id, &summaries)?;
                    return Err(AppError::Cancelled);
                }
                Err(e) => {
                    // One species failing must not lose the others (batch robustness).
                    summaries[i].status = SpeciesRunStatus::Failed;
                    summaries[i].error = Some(e.to_string());
                    self.store.event(
                        id,
                        &format!("{name}: failed — {e}"),
                        "error",
                        Some(sp.key),
                    )?;
                }
            }
            self.store.set_species(id, &summaries)?;
        }
        self.write_manifest(id, &spec, &stack, download.as_ref())?;
        if summaries
            .iter()
            .all(|s| s.status == SpeciesRunStatus::Failed)
        {
            return Err(AppError::Model(
                "every species failed; see events for details".into(),
            ));
        }
        Ok(())
    }

    /// One GBIF download for every species of the job that still needs data and is a single
    /// taxon (species rank or below). Failures fall back to the per-species search API.
    ///
    /// The request (taxa, extent) is derived from the spec alone, so the preparation thread and
    /// the compute worker make the same request and the second finds it in the download cache.
    fn prefetch_batch(
        &self,
        id: &str,
        spec: &JobSpec,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<niche_data::gbif_download::BatchData>, AppError> {
        if spec.demo {
            return Ok(None);
        }
        if let Some(snap) = self.mirror_for(&spec.area) {
            match self.mirror_batch(id, spec, &snap) {
                Ok(Some(batch)) => return Ok(Some(batch)),
                Ok(None) => {}
                // Any problem with the mirror: the study downloads from GBIF as before.
                Err(e) => {
                    tracing::warn!(error = %e, "regional GBIF mirror not usable for this study");
                    let _ = self.store.event(
                        id,
                        &format!("Regional GBIF mirror not usable ({e}); downloading from GBIF"),
                        "warning",
                        None,
                    );
                }
            }
        }
        let Some((creds, download_cache)) = self.job_account(id)?.1 else {
            return Ok(None);
        };
        let mut keys = Vec::new();
        // Species modelled from the organisation's data only need no GBIF download.
        for sp in spec.species.iter().filter(|s| uses_gbif(s)) {
            if species_dir(&self.cfg.data_dir, id, sp.key)
                .join("result.json")
                .exists()
            {
                continue;
            }
            // Fitted before an interruption; only its future projections remain (ADR-0016).
            if !spec.projections.is_empty()
                && checkpoint::checkpoint_dir(&self.cfg.data_dir, id, sp.key)
                    .join(projection::STATE)
                    .exists()
            {
                continue;
            }
            match gbif::taxon_rank(&self.http, sp.key) {
                Ok(rank) if is_single_taxon(&rank) => keys.push(sp.key),
                _ => {}
            }
        }
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
        self.store.event(
            id,
            &format!("Requesting one GBIF download for {} species", keys.len()),
            "info",
            None,
        )?;
        let progress = |step: &niche_data::gbif_download::Step| {
            let _ = self.store.set_progress(id, 0.15, &step.describe());
            self.note_download(id, step);
        };
        match niche_data::gbif_download::fetch_batch(
            &self.http,
            &download_cache,
            &creds,
            &keys,
            bbox,
            30,
            cancelled,
            &progress,
        ) {
            Ok(batch) => {
                let info = &batch.info;
                self.store.event(
                    id,
                    &format!(
                        "GBIF download {} ready: {} records{}",
                        info.key,
                        info.total_records,
                        info.doi
                            .as_deref()
                            .map(|d| format!(", DOI {d}"))
                            .unwrap_or_default()
                    ),
                    "info",
                    None,
                )?;
                Ok(Some(batch))
            }
            Err(niche_data::DataError::Cancelled) => Err(AppError::Cancelled),
            Err(e) => {
                self.store.event(
                    id,
                    &format!("GBIF batch download failed ({e}); using the search API per species"),
                    "warning",
                    None,
                )?;
                Ok(None)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn run_species(
        &self,
        job_id: &str,
        spec: &JobSpec,
        sp: &SpeciesRef,
        stack: &EnvStack,
        cancel: &AtomicBool,
        stage: &(dyn Fn(&str, f64) + Sync),
        prefetched: Option<gbif::OccurrenceFetch>,
    ) -> Result<(SpeciesResult, serde_json::Value), AppError> {
        // Future-climate projections (ADR-0016): a species interrupted after its fit resumes
        // with its projections, without fitting again.
        let projecting = !spec.projections.is_empty();
        let proj_context = projecting.then(|| projection::context(&spec_hash(spec), sp.key, stack));
        if let Some(ctx) = &proj_context
            && let Some(pending) = projection::load_state(
                &checkpoint::checkpoint_dir(&self.cfg.data_dir, job_id, sp.key),
                ctx,
            )
        {
            self.store.event(
                job_id,
                &format!(
                    "{}: resumed after the model fit (present outputs already written; the fit is not repeated); continuing the future-climate projections",
                    sp.scientific_name
                ),
                "info",
                Some(sp.key),
            )?;
            return self.project_species(job_id, spec, sp, stack, pending, cancel, stage);
        }
        // With projections, the fit takes the first part of the species' progress.
        let fit_stage = |msg: &str, p: f64| {
            stage(msg, if projecting { FIT_SHARE * p } else { p });
        };
        let stage: &(dyn Fn(&str, f64) + Sync) = &fit_stage;
        stage("Retrieving occurrence records", 0.0);
        let t_fetch = std::time::Instant::now();
        let (records, total, truncated, source, query_url, origin) = match prefetched {
            Some(f) => {
                let mirrored = self.mirror_source(job_id);
                let origin = if mirrored.is_some() {
                    gbif::RecordSource::Mirror
                } else {
                    gbif::RecordSource::Download
                };
                let source = match mirrored {
                    Some(m) => format!(
                        "Regional GBIF mirror ({}): GBIF occurrence download {} ({}), created {}, filtered to this study",
                        m.region_name,
                        f.doi.as_deref().unwrap_or("without DOI"),
                        f.query_url,
                        f.accessed_at
                    ),
                    None => format!(
                        "GBIF occurrence download {} ({}), created {}",
                        f.doi.as_deref().unwrap_or("without DOI"),
                        f.query_url,
                        f.accessed_at
                    ),
                };
                (
                    f.records,
                    f.total_matching,
                    false,
                    source,
                    Some(f.query_url),
                    Some(origin),
                )
            }
            None => self.fetch(sp, stack, &spec.area, spec.demo, spec.seed, cancel)?,
        };
        let fetch_s = t_fetch.elapsed().as_secs_f64();
        let t_fit = std::time::Instant::now();
        // The organisation's data (ADR-0108): the study's copy, made at submission.
        let own_data = match &sp.own {
            Some(_) => Some(self.job_dataset(job_id)?),
            None => None,
        };
        let data = crate::owndata::species_data(
            &records,
            match (&own_data, &sp.own) {
                (Some((info, recs)), Some(u)) => Some(crate::owndata::OwnInput {
                    use_: u,
                    dataset: info,
                    records: recs,
                }),
                _ => None,
            },
            uses_gbif(sp),
            stack,
            &spec.filters,
            thin_seed(spec.seed, sp.key),
        );
        let records_considered = data.records;
        let validation_sites = data.sites;
        let by_source = data.by_source;
        let cleaned = data.cleaned;
        let mut rng = Rng::new(spec.seed);
        let (background, background_sampling, effort, bias_warnings) =
            self.background_for(spec, sp, stack, &mut rng);
        // Bias-matched null models only when the background itself is the target group's.
        let null_weights = effort.as_ref().filter(|_| {
            background_sampling
                .as_ref()
                .is_some_and(|b| b.method == "target_group")
        });
        let FitOutput {
            mut result,
            suitability,
            uncertainty,
            mess,
            aoa,
            model,
            forest,
            curves,
            cv_reused,
            cv_models,
            variable_ids,
        } = fit_species(
            stack,
            sp,
            &cleaned.presence_cells,
            &background,
            &spec.model,
            &spec.validation,
            spec.seed,
            null_weights.map(|e| e.counts.as_slice()),
            Some(checkpoint::checkpoint_dir(
                &self.cfg.data_dir,
                job_id,
                sp.key,
            )),
            cancel,
            stage,
        )?;
        if cv_reused > 0 {
            self.store.event(
                job_id,
                &format!(
                    "{}: resumed with {cv_reused} of {cv_models} cross-validation models already trained before the interruption (same inputs, identical results)",
                    sp.scientific_name
                ),
                "info",
                Some(sp.key),
            )?;
        }
        let fit_s = t_fit.elapsed().as_secs_f64();
        let t_write = std::time::Instant::now();
        result.data.records_fetched = records_considered;
        result.data.exclusions = cleaned.exclusions.clone();
        result.data.source = source;
        result.data.query_url = query_url;
        let flags = origin.map(gbif::generalisation_flags_available);
        result.data.generalised = flags.map(|available| GeneralisedRecords {
            flags_available: available,
            records: cleaned.generalised_records,
            excluded: cleaned.generalised_excluded,
            retained: cleaned.generalised_retained,
        });
        let sampling_bias = effort
            .as_ref()
            .map(|e| bias::diagnostic(stack, e, &cleaned.presence_cells));
        let mut dw = Vec::new();
        data_warnings(
            &mut dw,
            cleaned.presence_cells.len(),
            truncated,
            total,
            records.len(),
            &cleaned.licences,
            spec.demo,
        );
        dw.retain(|w| w.code != "too_few_presences");
        dw.extend(generalised_warnings(
            &cleaned,
            flags,
            spec.filters.max_uncertainty_m,
        ));
        dw.extend(bias_warnings);
        if let Some(b) = &background_sampling
            && b.method == "target_group"
            && let Some(w) = dw.iter_mut().find(|w| w.code == "sampling_bias")
        {
            let class = b.target_group.clone().unwrap_or_default();
            *w = Warning::new(
                "sampling_bias",
                Severity::Info,
                format!("Occurrence records come from opportunistic surveys; they reflect observer effort as well as habitat. Bias correction: background drawn from cells with records of the target group {class} (Phillips et al. 2009), so presences and background share the observers' bias; this corrects only the part of the bias the target group shares with the species."),
            )
            .with("method", "target_group")
            .with("class", class);
        }
        if let Some(d) = &sampling_bias {
            dw.push(effort_warning(d));
        }
        result.background_sampling = background_sampling;
        result.sampling_bias = sampling_bias;
        if !spec.demo {
            let rank = gbif::taxon_rank(&self.http, sp.key)?;
            if let Some(w) = rank_warning(&rank) {
                cap_evidence_for_pooled_taxa(&mut result.evidence, &rank);
                dw.insert(0, w);
            }
        }
        // The organisation's data (ADR-0108): records per source, concentration of its
        // presences, and the independent presence–absence validation of the final prediction,
        // after every cap of the evidence level (it alone can raise the level, to Validated).
        if let (Some((info, _)), Some(u)) = (&own_data, &sp.own) {
            result.data.by_source = by_source;
            dw.extend(crate::owndata::concentration_warning(
                stack,
                &crate::owndata::own_presence_cells(&cleaned.points, stack),
            ));
            if let Some(s) = &validation_sites {
                let mut v = crate::independent::evaluate(
                    &info.id,
                    &info.name,
                    u.role,
                    info.synthetic || spec.demo,
                    s,
                    &suitability,
                    &result.thresholds,
                    stack,
                );
                if result.status == SpeciesRunStatus::Succeeded {
                    crate::independent::apply_to_evidence(&mut result.evidence, &mut v);
                }
                result.independent_validation = Some(v);
            }
        }
        // Cautions of the terrain/land-cover/water predictors kept by the collinearity filter
        // (ADR-0015); none for climate-only studies.
        let used: Vec<&str> = stack
            .layers
            .iter()
            .filter(|l| result.variables_used.contains(&l.name))
            .map(|l| l.id.as_str())
            .collect();
        dw.extend(env::predictor_warnings(&used));
        result.warnings.splice(0..0, dw);
        let dir = species_dir(&self.cfg.data_dir, job_id, sp.key);
        let lambdas = model.as_ref().map(niche_maxent::MaxentModel::to_lambdas);
        let outputs = Outputs {
            stack,
            suitability,
            uncertainty,
            mess,
            aoa,
            effort: effort.as_ref().map(|e| e.raster_data(stack)),
            lambdas,
            forest: forest.as_ref(),
            points: &cleaned.points,
            curves: &curves,
        };
        // Projections need a fitted model; a species without one gets none (and says so).
        if let Some(ctx) = &proj_context
            && result.status == SpeciesRunStatus::Succeeded
        {
            let mut sink = artifacts::FileSink::new(&dir);
            artifacts::write_species_files(&mut sink, outputs, &result, spec)?;
            let (files, sums) = sink.finish();
            result.files = files;
            let p10 = result
                .thresholds
                .iter()
                .find(|t| t.rule == "p10")
                .map(|t| t.threshold)
                .ok_or_else(|| AppError::Internal("P10 threshold missing".into()))?;
            let pending = projection::Pending {
                projector: crate::fit::Projector {
                    algorithm: result.selected.algorithm().to_string(),
                    vars: variable_ids,
                    maxent: model,
                    forest,
                    presence: cleaned.presence_cells.clone(),
                    background,
                    p10,
                },
                result,
                sums,
                points: cleaned.points,
            };
            let ckdir = checkpoint::checkpoint_dir(&self.cfg.data_dir, job_id, sp.key);
            // Without the state an interruption only costs a new fit.
            if let Err(e) = projection::save_state(&ckdir, ctx, &pending) {
                tracing::warn!(species_key = sp.key, error = %e, "projection state not saved");
            }
            let _ = self.store.event(
                job_id,
                &format!(
                    "{}: {} records retrieved in {fetch_s:.1} s; cleaning, tuning, prediction in {fit_s:.1} s; present outputs written in {:.1} s",
                    sp.scientific_name,
                    records.len(),
                    t_write.elapsed().as_secs_f64()
                ),
                "info",
                Some(sp.key),
            );
            return self.project_species(job_id, spec, sp, stack, pending, cancel, stage);
        }
        if projecting {
            result.warnings.push(Warning::new(
                "projection_not_computed",
                Severity::Info,
                "No model was fitted for this species, so no future projection was made.",
            ));
        }
        let (files, sums) = artifacts::write_species(&dir, outputs, &result, spec)?;
        result.files = files;
        let bytes =
            serde_json::to_vec_pretty(&result).map_err(|e| AppError::Internal(e.to_string()))?;
        // result.json last: its presence marks the species as complete (resume checkpoint).
        write_atomic(&dir.join("result.json"), &bytes)?;
        checkpoint::clear(&checkpoint::checkpoint_dir(
            &self.cfg.data_dir,
            job_id,
            sp.key,
        ));
        let write_s = t_write.elapsed().as_secs_f64();
        tracing::info!(
            species_key = sp.key,
            records = records.len(),
            presences = cleaned.presence_cells.len(),
            fetch_s,
            fit_s,
            write_s,
            "species stages"
        );
        let _ = self.store.event(
            job_id,
            &format!(
                "{}: {} records retrieved in {fetch_s:.1} s; cleaning, tuning, prediction in {fit_s:.1} s; outputs written in {write_s:.1} s",
                sp.scientific_name,
                records.len()
            ),
            "info",
            Some(sp.key),
        );
        Ok((
            result,
            json!(
                sums.into_iter()
                    .collect::<std::collections::BTreeMap<_, _>>()
            ),
        ))
    }

    /// Future-climate projections of a fitted species whose present outputs are written
    /// (ADR-0016), then its reports and `result.json` (last: the species' completion marker).
    /// Every GCM run is checkpointed as it completes; cancellation is checked between runs.
    #[allow(clippy::too_many_arguments)]
    fn project_species(
        &self,
        job_id: &str,
        spec: &JobSpec,
        sp: &SpeciesRef,
        stack: &EnvStack,
        pending: projection::Pending,
        cancel: &AtomicBool,
        stage: &(dyn Fn(&str, f64) + Sync),
    ) -> Result<(SpeciesResult, serde_json::Value), AppError> {
        let t0 = std::time::Instant::now();
        let projection::Pending {
            mut result,
            sums,
            points,
            projector,
        } = pending;
        let dir = species_dir(&self.cfg.data_dir, job_id, sp.key);
        let ckdir = checkpoint::checkpoint_dir(&self.cfg.data_dir, job_id, sp.key);
        stage("Preparing the future-climate projections", FIT_SHARE);
        let prepared = projector.prepare(stack)?;
        let vars = &projector.vars;
        let climate: Vec<String> = vars
            .iter()
            .filter(|v| projection::is_climate(v))
            .cloned()
            .collect();
        let statics: Vec<String> = vars
            .iter()
            .filter(|v| !projection::is_climate(v))
            .map(|id| {
                stack
                    .layers
                    .iter()
                    .find(|l| &l.id == id)
                    .map_or_else(|| id.clone(), |l| l.name.clone())
            })
            .collect();
        let total = projection::runs(spec);
        let (mut done, mut reused) = (0usize, 0usize);
        let mut sink = artifacts::FileSink::new(&dir);
        let mut maps = Vec::with_capacity(spec.projections.len());
        let w = |d: Vec<f32>| {
            niche_geo::write::write_geotiff_f32(&env::raster(stack, d))
                .map_err(|e| AppError::Internal(e.to_string()))
        };
        for p in &spec.projections {
            let mut runs = Vec::with_capacity(p.gcms.len());
            for gcm in &p.gcms {
                if cancel.load(Ordering::Relaxed) {
                    return Err(AppError::Cancelled);
                }
                stage(
                    &format!(
                        "Future climate {} {}: {gcm} ({}/{total})",
                        p.period,
                        p.ssp,
                        done + 1
                    ),
                    FIT_SHARE + (1.0 - FIT_SHARE) * done as f64 / total as f64,
                );
                let run = match projection::load_run(
                    &ckdir,
                    &p.period,
                    &p.ssp,
                    gcm,
                    prepared.cells.len(),
                ) {
                    Some(r) => {
                        reused += 1;
                        r
                    }
                    None => {
                        let t = std::time::Instant::now();
                        let (layers, urls) = projection::future_layers(
                            &self.http,
                            &self.cfg.cache_dir,
                            spec,
                            stack,
                            vars,
                            &p.period,
                            &p.ssp,
                            gcm,
                        )?;
                        let read_s = t.elapsed().as_secs_f64();
                        let r = prepared.predict(layers)?;
                        tracing::info!(
                            species_key = sp.key,
                            period = %p.period,
                            ssp = %p.ssp,
                            gcm = %gcm,
                            layers_read = urls.len(),
                            cells = prepared.cells.len(),
                            read_s,
                            predict_s = t.elapsed().as_secs_f64() - read_s,
                            "projection run"
                        );
                        projection::save_run(&ckdir, &p.period, &p.ssp, gcm, &r);
                        r
                    }
                };
                runs.push(run);
                done += 1;
            }
            let agg = projection::aggregate(
                stack.spec.len(),
                &prepared.cells,
                &|c| stack.cell_area(c),
                &prepared.present,
                prepared.threshold,
                p,
                &runs,
            );
            drop(runs);
            let mut summary = agg.summary;
            summary.climate_variables = climate.clone();
            summary.static_variables = statics.clone();
            summary.synthetic = spec.demo;
            for (kind, data) in [
                ("mean", agg.mean),
                ("sd", agg.sd),
                ("change", agg.change.clone()),
                ("agreement", agg.agreement.clone()),
                ("mess", agg.mess.clone()),
            ] {
                sink.put(
                    &format!("{}.tif", projection::file_stem(&p.period, &p.ssp, kind)),
                    &projection::description(kind, &summary),
                    artifacts::TIFF,
                    &w(data)?,
                )?;
            }
            maps.push(crate::report_html::FutureMap {
                period: p.period.clone(),
                ssp: p.ssp.clone(),
                gcms: p.gcms.len(),
                change: agg.change,
                agreement: agg.agreement,
                mess: agg.mess,
            });
            result.projections.push(summary);
        }
        result.warnings.extend(projection::warnings(&result));
        sink.put(
            "future_areas.csv",
            "Areas of suitable conditions per future scenario (GCM mean) and per climate model, km² (exact ellipsoidal cell areas); present P10 threshold; no dispersal modelled",
            "text/csv",
            projection::areas_csv(&result).as_bytes(),
        )?;
        // Reports from the written rasters, so a resumed species gives the same reports.
        let has = |name: &str| result.files.iter().any(|f| f.name == name);
        let read = |name: &str| artifacts::read_tif(&dir.join(name));
        let suitability = read("suitability.tif")?;
        let uncertainty = read("uncertainty.tif")?;
        let mess = read("mess.tif")?;
        let aoa = if has("aoa.tif") {
            Some(read("aoa.tif")?)
        } else {
            None
        };
        let effort = if has("effort.tif") {
            Some(read("effort.tif")?)
        } else {
            None
        };
        let html = crate::report_html::report_html(
            &result,
            spec,
            Some(&crate::report_html::Maps {
                stack,
                suitability: &suitability,
                uncertainty: &uncertainty,
                mess: &mess,
                aoa: aoa.as_deref(),
                effort: effort.as_deref(),
                points: &points,
                future: &maps,
            }),
        );
        artifacts::write_reports(&mut sink, &result, spec, &html)?;
        let (files, more) = sink.finish();
        result.files.extend(files);
        let mut sums = sums;
        sums.extend(more);
        let bytes =
            serde_json::to_vec_pretty(&result).map_err(|e| AppError::Internal(e.to_string()))?;
        // result.json last: its presence marks the species as complete (resume checkpoint).
        write_atomic(&dir.join("result.json"), &bytes)?;
        checkpoint::clear(&ckdir);
        let _ = self.store.event(
            job_id,
            &format!(
                "{}: future-climate projections: {total} scenario × climate-model runs{} in {:.1} s",
                sp.scientific_name,
                if reused > 0 {
                    format!(" ({reused} taken from before the interruption)")
                } else {
                    String::new()
                },
                t0.elapsed().as_secs_f64()
            ),
            "info",
            Some(sp.key),
        );
        Ok((
            result,
            json!(
                sums.into_iter()
                    .collect::<std::collections::BTreeMap<_, _>>()
            ),
        ))
    }

    /// Background of a species: random (default, ADR-0009 §3) or target group (ADR-0013), with
    /// the method actually used, the effort surface when one exists, and warnings. A target
    /// group that cannot be used falls back to the random background (the same cells as a
    /// random-background study) with the warning `target_group_unavailable`.
    fn background_for(
        &self,
        spec: &JobSpec,
        sp: &SpeciesRef,
        stack: &EnvStack,
        rng: &mut Rng,
    ) -> (
        Vec<usize>,
        Option<BackgroundSampling>,
        Option<EffortSurface>,
        Vec<Warning>,
    ) {
        let n = spec.model.background_points;
        if spec.model.background_method != "target_group" {
            return (sample_background(stack, n, rng), None, None, Vec::new());
        }
        let valid = stack.valid.iter().filter(|v| **v).count();
        let (class, reason, effort, eligible) = match self.target_group_effort(spec, sp, stack) {
            Ok(e) => {
                let eligible = e.eligible_cells(stack);
                let need = bias::min_eligible_cells(n, valid);
                let class = Some(e.class.clone());
                if eligible >= need {
                    let bg = sample_background_target_group(stack, &e.counts, n, rng);
                    let info = BackgroundSampling {
                        requested: "target_group".into(),
                        method: "target_group".into(),
                        target_group: class,
                        eligible_cells: eligible,
                        fallback_reason: None,
                        synthetic: e.synthetic,
                    };
                    let mut warnings = Vec::new();
                    if bg.len() < n {
                        warnings.push(
                            Warning::new(
                                "target_group_background_smaller",
                                Severity::Info,
                                format!(
                                    "Only {eligible} cells of the study area have records of the target group {}: the background has {} cells instead of {n}.",
                                    e.class,
                                    bg.len()
                                ),
                            )
                            .with("eligible", eligible)
                            .with("requested", n)
                            .with("class", e.class.clone()),
                        );
                    }
                    return (bg, Some(info), Some(e), warnings);
                }
                let reason = format!(
                    "only {eligible} cells of the study area have records of {} (at least {need} needed)",
                    e.class
                );
                // The effort surface still exists: its map and the bias diagnostic are kept.
                (class, reason, (eligible > 0).then_some(e), eligible)
            }
            Err((class, reason)) => (class, reason, None, 0),
        };
        let bg = sample_background(stack, n, rng);
        let synthetic = effort.as_ref().is_some_and(|e| e.synthetic) || spec.demo;
        let w = Warning::new(
            "target_group_unavailable",
            Severity::Warning,
            format!("Target-group background not available: {reason}. The background was drawn at random over the study area, as by default, so no bias correction was applied."),
        )
        .with("reason", reason.clone())
        .with("class", class.clone().unwrap_or_default());
        (
            bg,
            Some(BackgroundSampling {
                requested: "target_group".into(),
                method: "random".into(),
                target_group: class,
                eligible_cells: eligible,
                fallback_reason: Some(reason),
                synthetic,
            }),
            effort,
            vec![w],
        )
    }

    /// Sampling effort of the species' target group on the study grid: from the regional
    /// mirror's effort grids (format 2), or synthetic for the demo. `Err((class, reason))` when
    /// it is not available.
    fn target_group_effort(
        &self,
        spec: &JobSpec,
        sp: &SpeciesRef,
        stack: &EnvStack,
    ) -> Result<EffortSurface, (Option<String>, String)> {
        let wanted = spec
            .model
            .target_group
            .as_deref()
            .map(str::trim)
            .filter(|g| !g.is_empty())
            .map(String::from);
        if spec.demo {
            return match wanted {
                Some(c) if c != bias::DEMO_CLASS => Err((
                    Some(c.clone()),
                    format!(
                        "the synthetic demo has an effort grid only for its own synthetic target group, not for {c}"
                    ),
                )),
                _ => Ok(bias::demo_effort(stack)),
            };
        }
        let snap = self.mirror_for(&spec.area).ok_or_else(|| {
            (
                wanted.clone(),
                "no regional GBIF mirror covers this study area (sampling-effort grids come from the mirror, ADR-0114)".to_string(),
            )
        })?;
        if snap.effort_classes().is_empty() {
            return Err((
                wanted,
                "the regional GBIF mirror has no sampling-effort grids yet (they are built at its next refresh)".into(),
            ));
        }
        let class = match wanted {
            Some(c) => c,
            None => match gbif::taxon_class(&self.http, sp.key) {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Err((None, "GBIF gives no class for this taxon".into()));
                }
                Err(e) => {
                    return Err((
                        None,
                        format!("the GBIF class of the species could not be looked up ({e})"),
                    ));
                }
            },
        };
        let s = stack.spec;
        match snap.effort(&class, (s.west, s.south(), s.east(), s.north)) {
            Ok(Some(points)) => Ok(EffortSurface::from_points(stack, &class, points, false)),
            Ok(None) => Err((
                Some(class.clone()),
                format!("the regional GBIF mirror has no records of class {class}"),
            )),
            Err(e) => Err((
                Some(class),
                format!("the regional GBIF mirror's effort grid could not be read ({e})"),
            )),
        }
    }

    fn write_manifest(
        &self,
        id: &str,
        spec: &JobSpec,
        stack: &EnvStack,
        download: Option<&niche_data::gbif_download::DownloadInfo>,
    ) -> Result<(), AppError> {
        // Whose GBIF account made the download (the kind only: user names stay out of outputs).
        let gbif_account = match self.store.job_owner(id)?.and_then(|o| o.gbif_account) {
            Some(t) if t.starts_with("user:") => "user",
            Some(t) if t.starts_with("org:") => "organisation",
            Some(t) if t == "shared" => "shared",
            Some(t) if t == "mirror" => "mirror",
            _ => "none",
        };
        // Records taken from the regional mirror: which snapshot, and records per dataset (what
        // a GBIF derived-dataset citation needs).
        let mirror = self.mirror_source(id).filter(|_| download.is_some());
        let mut species = Vec::new();
        let mut projected: Vec<SpeciesResult> = Vec::new();
        let mut flags_seen: Vec<bool> = Vec::new();
        for sp in &spec.species {
            let dir = species_dir(&self.cfg.data_dir, id, sp.key);
            let mut files = serde_json::Map::new();
            if let Ok(rd) = std::fs::read_dir(&dir) {
                let mut entries: Vec<_> = rd.filter_map(Result::ok).collect();
                entries.sort_by_key(std::fs::DirEntry::file_name);
                for e in entries {
                    if let Ok(b) = std::fs::read(e.path()) {
                        files.insert(
                            e.file_name().to_string_lossy().into(),
                            json!({"sha256": artifacts::sha256(&b), "bytes": b.len()}),
                        );
                    }
                }
            }
            let result: Option<serde_json::Value> =
                std::fs::read_to_string(dir.join("result.json"))
                    .ok()
                    .and_then(|t| serde_json::from_str(&t).ok());
            let mut entry = json!({
                "key": sp.key, "scientific_name": sp.scientific_name,
                "selected": result.as_ref().and_then(|r| r.get("selected").cloned()),
                "evidence": result.as_ref().and_then(|r| r.get("evidence").cloned()),
                "validation": result.as_ref().and_then(|r| r.get("validation").cloned()),
                "query_url": result.as_ref().and_then(|r| r.pointer("/data/query_url").cloned()),
                "artifacts": files,
            });
            // Area of applicability summary (ADR-0014), only when it was computed.
            if let Some(a) = result.as_ref().and_then(|r| r.get("aoa")) {
                entry["area_of_applicability"] = a.clone();
            }
            // Generalised coordinates (ADR-0011): absent for synthetic data.
            if let Some(g) = result.as_ref().and_then(|r| r.pointer("/data/generalised")) {
                entry["generalisation_flags_available"] = g["flags_available"].clone();
                entry["generalised_records"] = g["records"].clone();
                entry["generalised_excluded"] = g["excluded"].clone();
                entry["generalised_retained"] = g["retained"].clone();
                flags_seen.push(g["flags_available"].as_bool().unwrap_or(false));
            }
            // Background actually used and sampling-bias diagnostic (ADR-0013), when requested.
            if let Some(b) = result.as_ref().and_then(|r| r.get("background_sampling")) {
                entry["background"] = b.clone();
            }
            if let Some(b) = result.as_ref().and_then(|r| r.get("sampling_bias")) {
                entry["sampling_bias"] = b.clone();
            }
            // Future-climate projections (ADR-0016), when requested.
            if let Some(p) = result.as_ref().and_then(|r| r.get("projections")) {
                entry["projections"] = p.clone();
                if let Some(r) = result
                    .as_ref()
                    .and_then(|r| serde_json::from_value::<SpeciesResult>(r.clone()).ok())
                {
                    projected.push(r);
                }
            }
            // The organisation's data (ADR-0108): how they were used, counts per source and the
            // independent validation — counts and metrics, never coordinates.
            if let Some(o) = &sp.own {
                entry["own_data"] = json!({
                    "taxon_values": o.values,
                    "role": o.role,
                    "gbif_records_used": uses_gbif(sp),
                    "by_source": result.as_ref().and_then(|r| r.pointer("/data/by_source").cloned()),
                    "independent_validation": result.as_ref().and_then(|r| r.get("independent_validation").cloned()),
                });
            }
            species.push(entry);
        }
        let mut methods = json!({
            "algorithm": "MaxEnt (cloglog output)",
            "validation": spec.validation,
            "selection": "lowest mean OR10, then highest mean continuous Boyce index, then fewest coefficients",
            "uncertainty": "per-cell SD across cross-validation models of the selected setting",
            "extrapolation": "MESS (Elith et al. 2010) against background + presence environments",
            "area": "sum of exact WGS84 ellipsoidal cell areas",
            "evidence_rules": "ADR-0009",
        });
        // Opt-in evaluation methods (ADR-0014): recorded only when used.
        if spec.validation.method == "spatial_grid" {
            methods["spatial_grid"] = json!(match spec.validation.block_km {
                Some(_) =>
                    "square blocks of the given size (blockCV, Valavi et al. 2019), assigned to folds balancing presences",
                None =>
                    "square blocks sized by the median variogram range of the predictors on the background sample, clamped to [max(5 km, 2 cells), shorter extent / 3] (blockCV, Valavi et al. 2019)",
            });
        }
        if spec.validation.null_models > 0 {
            methods["null_models"] = json!(format!(
                "{} null presence sets drawn uniformly from cells with predictor data, partitioned and fitted like the selected setting; observed mean held-out Boyce index and OR10 vs nulls, empirical one-sided p (Raes & ter Steege 2007; Bohl et al. 2019); evidence capped at Exploratory when the Boyce index p > {} (ADR-0014)",
                spec.validation.null_models,
                crate::fit::NULL_ALPHA
            ));
        }
        if spec.validation.aoa {
            methods["area_of_applicability"] = json!(
                "dissimilarity index to the final model's presences and background, predictors weighted by permutation importance, threshold from cross-validated training DI (Meyer & Pebesma 2021); diagnostic only, not an evidence criterion (ADR-0014)"
            );
        }
        // Second engine (ADR-0017): recorded only when used, so MaxEnt manifests are unchanged.
        if spec.model.algorithm != "maxent" {
            let rf = json!({
                "engine": format!("niche-rf (down-sampled random forest, format version {})", niche_rf::FORMAT_VERSION),
                "trees": spec.model.rf_trees(),
                "per_tree_sample": "bootstrap of the n presences + n background points drawn with replacement (Valavi et al. 2021; Chen, Liaw & Breiman 2004)",
                "tree": "CART classification, Gini, fully grown (node size 1), midpoints between distinct values",
                "mtry": if spec.model.rf_tune_mtry {
                    "floor(sqrt(p)) and floor(p/2) evaluated under the same cross-validation, selected by lowest mean OR10 then highest mean Boyce index"
                } else {
                    "floor(sqrt(p)) (randomForest default)"
                },
                "output": "fraction of trees voting presence: a relative suitability index, not a probability (ADR-0007)",
                "seed": "per species: (study seed XOR (species key × 0x9E3779B9) XOR RF stream) modulo 2^53; tree i uses a stream derived from (seed, i); the value is in each species' `selected.rf_seed`",
                "importance": "permutation importance: mean drop in training presence–background AUC, 3 repeats, presences + up to 2000 background points, normalised to 100 (Breiman 2001)",
                "response": "partial dependence, 51 grid points over the training range, 300 presence/background rows (Friedman 2001)",
                "area_of_applicability_weights": "random-forest permutation importance (ensemble: mean of the two members' normalised importances)",
                "checkpoints": "fold metrics only; selected fold forests are refitted on resume (deterministic for the same inputs and seed)",
            });
            if spec.model.algorithm == "rf" {
                methods["algorithm"] = json!("Down-sampled random forest (vote fraction output)");
                methods["selection"] = json!(
                    "lowest mean OR10, then highest mean continuous Boyce index (among mtry candidates)"
                );
            } else {
                methods["algorithm"] = json!(
                    "Ensemble of MaxEnt (cloglog) and a down-sampled random forest: unweighted mean of the members' percentile ranks among their predictions at the training background points (mid-ranks for ties)"
                );
                methods["ensemble"] = json!(
                    "members tuned and selected separately (same folds and rules); the ensemble is evaluated fold by fold as the ensemble of the members' fold models (rank references: each fold's training background) and its metrics drive the evidence level; thresholds are computed on the ensemble output with the same rules (Valavi et al. 2022 average tuned members after rescaling to 0–1; ranks are used here so the two scales are comparable)"
                );
            }
            methods["random_forest"] = rf;
            methods["uncertainty"] = json!(
                "per-cell SD across cross-validation models of the selected setting (ensemble: across the fold ensembles)"
            );
        }
        // Additional predictors (ADR-0015): recorded only when used.
        if env::uses_additional_predictors(&spec.layers) {
            methods["predictors"] = json!(
                "terrain from Copernicus DEM GLO-90 (mean of 3″ heights, mean of 3″ Horn slopes computed before aggregation, SD of 3″ heights), land-cover fractions from ESA WorldCover 2021 (share of valid 10 m pixels per class group), distance to permanent water truncated at 25 km; all on the native CHELSA 30″ grid without resampling, then aggregated by the resolution factor as the mean (ADR-0015)"
            );
        }
        if spec.validation.null_models > 0
            || spec.validation.aoa
            || spec.validation.method == "spatial_grid"
        {
            methods["evidence_rules"] = json!("ADR-0009, ADR-0014");
        }
        // Cleaning rules: ADR-0009 §1 with the generalised-coordinate rule of ADR-0011.
        methods["cleaning"] = json!(
            "ADR-0009 §1; records whose coordinates were generalised by the publisher are excluded when their stated precision is missing or > max_uncertainty_m (ADR-0011)"
        );
        if let Some(km) = spec.filters.thin_km {
            methods["thinning"] = json!(format!(
                "presence cells thinned to at least {km} km apart after the one-record-per-cell rule (spThin, Aiello-Lammens et al. 2015; largest of up to {} random visiting orders, seeded by the study seed and the species key)",
                env::THIN_REPS
            ));
        }
        methods["background"] = json!(if spec.model.background_method == "target_group" {
            format!(
                "target group ({}): up to {} cells drawn uniformly among the land cells with predictor data and at least one record of the target group in the regional GBIF mirror's sampling-effort grid (Phillips et al. 2009; ADR-0013); random background when unavailable (see each species' `background`)",
                spec.model
                    .target_group
                    .as_deref()
                    .unwrap_or("the species' GBIF class"),
                spec.model.background_points
            )
        } else {
            format!(
                "{} random land cells with predictor data in the study area (ADR-0009 §3); no bias correction",
                spec.model.background_points
            )
        });
        if !spec.projections.is_empty() {
            methods["projections"] = json!(
                "final model(s) applied to CHELSA v2.1 CMIP6 climatologies (ISIMIP3b GCMs): climate predictors replaced per GCM on the same native window and aggregation, other predictors at present values; GCM mean and SD; change classes at the present P10 threshold with GCM agreement; future MESS against the training reference; exact ellipsoidal areas; no dispersal (ADR-0016)"
            );
        }
        // The organisation's dataset (ADR-0108): its description from the study's copy —
        // checksums, mapping, counts — never its records.
        let own_dataset = match &spec.own_dataset {
            Some(_) => {
                let info = crate::owndata::job_dataset_info(&self.cfg.data_dir, id)?;
                methods["own_data"] = json!(
                    "the organisation's own records (CSV import, ADR-0108): reprojected to WGS84, then the cleaning rules of ADR-0009 §1 like GBIF records; licence rule: always accepted (organisation's own data); absence records are never presences (rule `absence`); own presences are placed before GBIF records, so a cell given by both is attributed to the organisation's data"
                );
                methods["independent_validation"] = json!(format!(
                    "presence–absence validation of the final prediction on the organisation's records held out from training (or the absences of a training dataset): same cleaning rules, one site per grid cell (presence wins), sites in training presence cells excluded (leakage rule); AUC, sensitivity, specificity, TSS and kappa at the training P10 and max-SSS thresholds, Brier score and calibration table as diagnostics (suitability is not a probability) (Allouche et al. 2006; Leroy et al. 2018); metrics need ≥ {} presence and ≥ {} absence sites; Validated level when held out, ≥ {} sites of each, AUC ≥ {}, TSS ≥ {} and Supported (ADR-0108, proposed)",
                    crate::independent::MIN_PA_SITES,
                    crate::independent::MIN_PA_SITES,
                    crate::independent::VALIDATED_MIN_SITES,
                    crate::independent::VALIDATED_MIN_AUC,
                    crate::independent::VALIDATED_MIN_TSS
                ));
                methods["evidence_rules"] = json!(format!(
                    "{}, ADR-0108",
                    methods["evidence_rules"].as_str().unwrap_or("ADR-0009")
                ));
                Some(json!({
                    "source": "file_import",
                    "owner": "organisation",
                    "dataset_id": info.id,
                    "name": info.name,
                    "created_at": info.created_at,
                    "file_name": info.file_name,
                    "file_sha256": info.file_sha256,
                    "records_sha256": info.records_sha256,
                    "crs": info.crs,
                    "transform": crate::owndata::transform_text(&info.crs),
                    "mapping": info.mapping,
                    "absent_values": info.absent_values,
                    "rows_read": info.report.rows_read,
                    "rows_imported": info.report.rows_imported,
                    "rejected": info.report.rejected.iter().map(|r| json!({"reason": r.reason, "count": r.count})).collect::<Vec<_>>(),
                    "presences": info.report.presences,
                    "absences": info.report.absences,
                    "synthetic": info.synthetic,
                    "licence": "organisation's own data",
                }))
            }
            None => None,
        };
        let mut manifest = json!({
            "manifest_version": 1,
            "job_id": id,
            "created_at": jiff::Timestamp::now().to_string(),
            "software": {"name": "NicheRS", "version": env!("CARGO_PKG_VERSION"), "engine": match spec.model.algorithm.as_str() { "rf" => "niche-rf (down-sampled random forest)", "ensemble" => "niche-maxent (MaxEnt 3.4.4 port) + niche-rf (down-sampled random forest)", _ => "niche-maxent (MaxEnt 3.4.4 port)" }},
            "spec": spec,
            "spec_sha256": spec_hash(spec),
            "random_seed": spec.seed,
            "synthetic_data": spec.demo,
            "environment": stack.provenance,
            "grid": stack.grid_info(),
            "occurrence_source": if spec.demo { json!({"source": "synthetic"}) } else { json!({"source": if mirror.is_some() { "GBIF regional mirror (one GBIF download of the region, filtered to this study)" } else if download.is_some() { "GBIF occurrence download (batch)" } else { "GBIF occurrence search API" }, "download": download, "gbif_account": download.map(|_| gbif_account), "mirror": mirror, "citation": download.and_then(|d| d.doi.as_ref().map(|doi| format!("GBIF.org ({}) GBIF Occurrence Download {doi}", d.created.get(..10).unwrap_or(&d.created)))), "taxonomy": gbif::BACKBONE, "accessed_at": jiff::Timestamp::now().to_string(), "max_records": self.cfg.max_records, "download_format": if download.is_some() { "SIMPLE_CSV" } else { "search API (JSON)" }, "generalisation_flags_available": if flags_seen.is_empty() { download.is_none() } else { flags_seen.iter().all(|f| *f) }}) },
            "methods": methods,
            "species": species,
        });
        // Future climate read by the projections (ADR-0016): only when requested, so other
        // manifests keep their shape.
        if !spec.projections.is_empty() {
            manifest["future_climate"] = projection::manifest_block(spec, &projected);
        }
        if let Some(d) = own_dataset {
            manifest["own_data"] = d;
            if spec.species.iter().all(|s| !uses_gbif(s)) {
                manifest["occurrence_source"] =
                    json!({"source": "none: the organisation's own data only (see own_data)"});
            }
        }
        let bytes =
            serde_json::to_vec_pretty(&manifest).map_err(|e| AppError::Internal(e.to_string()))?;
        write_atomic(
            &self
                .cfg
                .data_dir
                .join("jobs")
                .join(id)
                .join("manifest.json"),
            &bytes,
        )?;
        Ok(())
    }

    /// Export the cleaned presences and the background sample of each species as MaxEnt SWD CSV
    /// files (variables after the collinearity filter), for verification with maxent.jar.
    pub fn export_swd(
        &self,
        spec: &JobSpec,
        out: &std::path::Path,
    ) -> Result<Vec<PathBuf>, AppError> {
        self.validate(spec)?;
        let stack = build_stack(
            &self.http,
            &self.cfg.cache_dir,
            &spec.area,
            &spec.layers,
            spec.resolution_factor,
            spec.demo,
            &|_, _| {},
        )?;
        let never = AtomicBool::new(false);
        let mut written = Vec::new();
        for sp in &spec.species {
            let (records, ..) = self.fetch(sp, &stack, &spec.area, spec.demo, spec.seed, &never)?;
            // Local mode (CLI): the organisation's datasets are those without organisation.
            let own = match (&spec.own_dataset, &sp.own) {
                (Some(d), Some(_)) => Some(self.dataset_records(&d.id, &Scope::All)?),
                _ => None,
            };
            let cleaned = crate::owndata::species_data(
                &records,
                match (&own, &sp.own) {
                    (Some((info, recs)), Some(u)) => Some(crate::owndata::OwnInput {
                        use_: u,
                        dataset: info,
                        records: recs,
                    }),
                    _ => None,
                },
                uses_gbif(sp),
                &stack,
                &spec.filters,
                thin_seed(spec.seed, sp.key),
            )
            .cleaned;
            let mut rng = Rng::new(spec.seed);
            let (background, ..) = self.background_for(spec, sp, &stack, &mut rng);
            let cols: Vec<Vec<f64>> = stack
                .layers
                .iter()
                .map(|l| background.iter().map(|&c| f64::from(l.data[c])).collect())
                .collect();
            let (kept, _) =
                niche_eval::collinearity_filter(&cols, spec.model.collinearity_threshold);
            let header = format!(
                "species,x,y,{}\n",
                kept.iter()
                    .map(|&k| stack.layers[k].id.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let rows = |label: &str, cells: &[usize]| -> String {
                let mut s = header.clone();
                for &c in cells {
                    let (x, y) = stack
                        .spec
                        .center(c / stack.spec.width, c % stack.spec.width);
                    let vals: Vec<String> = kept
                        .iter()
                        .map(|&k| format!("{}", stack.layers[k].data[c]))
                        .collect();
                    s.push_str(&format!("{label},{x},{y},{}\n", vals.join(",")));
                }
                s
            };
            let dir = out.join(sp.key.to_string());
            std::fs::create_dir_all(&dir).map_err(|e| AppError::Storage(e.to_string()))?;
            write_atomic(
                &dir.join("samples.csv"),
                rows("species", &cleaned.presence_cells).as_bytes(),
            )?;
            write_atomic(
                &dir.join("background.csv"),
                rows("background", &background).as_bytes(),
            )?;
            written.push(dir);
        }
        Ok(written)
    }

    pub fn results(&self, id: &str) -> Result<JobResults, AppError> {
        let job = self
            .store
            .get(id)?
            .ok_or_else(|| AppError::NotFound(format!("job {id}")))?;
        let mut species = Vec::new();
        for sp in &job.spec.species {
            let p = species_dir(&self.cfg.data_dir, id, sp.key).join("result.json");
            if let Ok(t) = std::fs::read_to_string(&p) {
                species
                    .push(serde_json::from_str(&t).map_err(|e| AppError::Storage(e.to_string()))?);
            }
        }
        Ok(JobResults { job, species })
    }

    pub fn manifest(&self, id: &str) -> Result<serde_json::Value, AppError> {
        let p = self
            .cfg
            .data_dir
            .join("jobs")
            .join(id)
            .join("manifest.json");
        let t = std::fs::read_to_string(&p).map_err(|_| {
            AppError::NotFound(format!("manifest for job {id} (job not finished?)"))
        })?;
        serde_json::from_str(&t).map_err(|e| AppError::Storage(e.to_string()))
    }

    /// Path of a downloadable file; only names listed in the species result are allowed
    /// (no path traversal).
    pub fn file_path(&self, id: &str, key: u64, name: &str) -> Result<(PathBuf, String), AppError> {
        let dir = species_dir(&self.cfg.data_dir, id, key);
        let t = std::fs::read_to_string(dir.join("result.json"))
            .map_err(|_| AppError::NotFound("result".into()))?;
        let r: SpeciesResult =
            serde_json::from_str(&t).map_err(|e| AppError::Storage(e.to_string()))?;
        let f = r
            .files
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| AppError::NotFound(format!("file {name}")))?;
        Ok((dir.join(&f.name), f.media_type.clone()))
    }

    /// Occurrence points with their cleaning status, as used for the species.
    pub fn occurrences(&self, id: &str, key: u64) -> Result<Vec<OccurrencePoint>, AppError> {
        let p = species_dir(&self.cfg.data_dir, id, key).join("occurrences.geojson");
        let t = std::fs::read_to_string(&p)
            .map_err(|_| AppError::NotFound(format!("occurrences for species {key}")))?;
        let v: serde_json::Value =
            serde_json::from_str(&t).map_err(|e| AppError::Storage(e.to_string()))?;
        let feats = v
            .get("features")
            .and_then(|f| f.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(feats
            .iter()
            .filter_map(|f| {
                let c = f.pointer("/geometry/coordinates")?.as_array()?;
                Some(OccurrencePoint {
                    lng: c.first()?.as_f64()?,
                    lat: c.get(1)?.as_f64()?,
                    year: f
                        .pointer("/properties/year")
                        .and_then(serde_json::Value::as_i64)
                        .map(|y| y as i32),
                    status: f.pointer("/properties/status")?.as_str()?.to_string(),
                    source: f
                        .pointer("/properties/source")
                        .and_then(serde_json::Value::as_str)
                        .map(String::from),
                })
            })
            .collect())
    }

    /// Raster of a species output (cached).
    pub fn raster(&self, id: &str, key: u64, layer: &str) -> Result<Arc<Raster>, AppError> {
        let owned;
        let file = match layer {
            "suitability" | "binary" => "suitability.tif",
            "uncertainty" => "uncertainty.tif",
            "extrapolation" | "mess" => "mess.tif",
            "aoa" => "aoa.tif",
            "effort" => "effort.tif",
            // Future-climate projections (ADR-0016): names checked against the catalogue.
            _ => match projection::parse_layer(layer) {
                Some((period, ssp, kind)) => {
                    owned = format!("{}.tif", projection::file_stem(period, ssp, kind));
                    owned.as_str()
                }
                None => return Err(AppError::NotFound(format!("layer {layer}"))),
            },
        };
        let cache_key = format!("{id}/{key}/{file}");
        if let Some(r) = self
            .rasters
            .lock()
            .map_err(|_| AppError::Internal("lock".into()))?
            .get(&cache_key)
        {
            return Ok(r);
        }
        let path = species_dir(&self.cfg.data_dir, id, key).join(file);
        let src = FileSource::open(&path)
            .map_err(|_| AppError::NotFound(format!("{file} for species {key}")))?;
        let info = GeoTiffInfo::read(&src).map_err(|e| AppError::Storage(e.to_string()))?;
        let r = Arc::new(
            info.read_window(&src, 0, 0, info.height, info.width)
                .map_err(|e| AppError::Storage(e.to_string()))?,
        );
        self.rasters
            .lock()
            .map_err(|_| AppError::Internal("lock".into()))?
            .put(cache_key, r.clone());
        Ok(r)
    }

    /// Change classes of a projection for display (ADR-0016): the class (1–4) plus 10 where at
    /// least a majority of the scenario's GCMs agree with it; NaN outside. Cached like rasters.
    pub fn projection_change_display(
        &self,
        id: &str,
        key: u64,
        layer: &str,
    ) -> Result<Arc<Raster>, AppError> {
        let (period, ssp, _) = projection::parse_layer(layer)
            .ok_or_else(|| AppError::NotFound(format!("layer {layer}")))?;
        let cache_key = format!("{id}/{key}/{period}/{ssp}/change-display");
        if let Some(r) = self
            .rasters
            .lock()
            .map_err(|_| AppError::Internal("lock".into()))?
            .get(&cache_key)
        {
            return Ok(r);
        }
        let t =
            std::fs::read_to_string(species_dir(&self.cfg.data_dir, id, key).join("result.json"))
                .map_err(|_| AppError::NotFound(format!("results for species {key}")))?;
        let r: SpeciesResult =
            serde_json::from_str(&t).map_err(|e| AppError::Storage(e.to_string()))?;
        let gcms = r
            .projections
            .iter()
            .find(|p| p.period == period && p.ssp == ssp)
            .map(|p| p.gcms.len())
            .ok_or_else(|| AppError::NotFound(format!("projection {period} {ssp}")))?;
        let need = projection::majority(gcms) as f32;
        let change = self.raster(id, key, &projection::file_stem(period, ssp, "change"))?;
        let agree = self.raster(id, key, &projection::file_stem(period, ssp, "agreement"))?;
        let data = change
            .data
            .iter()
            .zip(&agree.data)
            .map(|(&c, &a)| if a >= need { c + 10.0 } else { c })
            .collect();
        let out = Arc::new(Raster {
            spec: change.spec,
            data,
        });
        self.rasters
            .lock()
            .map_err(|_| AppError::Internal("lock".into()))?
            .put(cache_key, out.clone());
        Ok(out)
    }

    /// Small suitability map of a species (workspace cards): rendered once from its raster and
    /// kept next to its results (`thumbnail.png`, removed with the job).
    pub fn thumbnail(&self, id: &str, key: u64) -> Result<Vec<u8>, AppError> {
        let path = species_dir(&self.cfg.data_dir, id, key).join("thumbnail.png");
        if let Ok(b) = std::fs::read(&path) {
            return Ok(b);
        }
        let r = self.raster(id, key, "suitability")?;
        let png = niche_geo::tiles::overview(&r, THUMBNAIL_PX, |v| {
            let c = niche_geo::colormap::viridis(v);
            Some([c[0], c[1], c[2], 255])
        });
        if png.is_empty() {
            return Err(AppError::Internal("thumbnail rendering failed".into()));
        }
        write_atomic(&path, &png)?;
        Ok(png)
    }

    pub fn sample(&self, id: &str, key: u64, lng: f64, lat: f64) -> Result<SampleValue, AppError> {
        let s = self.raster(id, key, "suitability")?;
        let u = self.raster(id, key, "uncertainty")?;
        let m = self.raster(id, key, "extrapolation")?;
        // Only results computed with the area of applicability have `aoa.tif`.
        let a = self.raster(id, key, "aoa").ok();
        // Only results with a target-group effort surface have `effort.tif`.
        let ef = self.raster(id, key, "effort").ok();
        let v = s.sample(lng, lat);
        // Future-climate projections (ADR-0016), when the result has any.
        let mut projections = Vec::new();
        let scenarios: Vec<(String, String, usize)> =
            std::fs::read_to_string(species_dir(&self.cfg.data_dir, id, key).join("result.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<SpeciesResult>(&t).ok())
                .map(|r| {
                    r.projections
                        .into_iter()
                        .map(|p| (p.period, p.ssp, p.gcms.len()))
                        .collect()
                })
                .unwrap_or_default();
        for (period, ssp, gcms) in scenarios {
            let at = |kind: &str| -> Option<f32> {
                self.raster(id, key, &projection::file_stem(&period, &ssp, kind))
                    .ok()?
                    .sample(lng, lat)
                    .filter(|x| x.is_finite())
            };
            projections.push(ProjectionSample {
                mean: at("mean").map(f64::from),
                sd: at("sd").map(f64::from),
                change: at("change").map(|x| x as u8),
                agreement: at("agreement").map(|x| x as u8),
                mess: at("mess").map(f64::from),
                period,
                ssp,
                gcms,
            });
        }
        Ok(SampleValue {
            in_area: v.is_some(),
            suitability: v.map(f64::from),
            uncertainty: u.sample(lng, lat).map(f64::from),
            mess: m.sample(lng, lat).map(f64::from),
            aoa_di: a.and_then(|a| a.sample(lng, lat)).map(f64::from),
            effort: ef.and_then(|e| e.sample(lng, lat)).map(f64::from),
            projections,
        })
    }

    /// Largest records-per-cell value of a species' effort surface (legend and tile scale).
    pub fn effort_max(&self, id: &str, key: u64) -> Result<f64, AppError> {
        let t =
            std::fs::read_to_string(species_dir(&self.cfg.data_dir, id, key).join("result.json"))
                .map_err(|_| AppError::NotFound(format!("results for species {key}")))?;
        let r: SpeciesResult =
            serde_json::from_str(&t).map_err(|e| AppError::Storage(e.to_string()))?;
        r.sampling_bias
            .map(|b| b.max_cell_records)
            .ok_or_else(|| AppError::NotFound(format!("sampling effort for species {key}")))
    }

    /// Dissimilarity-index threshold of a species' area of applicability (cells above it are
    /// outside), from its result.
    pub fn aoa_threshold(&self, id: &str, key: u64) -> Result<f64, AppError> {
        let t =
            std::fs::read_to_string(species_dir(&self.cfg.data_dir, id, key).join("result.json"))
                .map_err(|_| AppError::NotFound(format!("results for species {key}")))?;
        let r: SpeciesResult =
            serde_json::from_str(&t).map_err(|e| AppError::Storage(e.to_string()))?;
        r.aoa
            .map(|a| a.threshold)
            .ok_or_else(|| AppError::NotFound(format!("area of applicability for species {key}")))
    }
}

/// Whether the species' training data include GBIF records (the synthetic records in the demo).
pub fn uses_gbif(sp: &SpeciesRef) -> bool {
    sp.own
        .as_ref()
        .is_none_or(|o| o.gbif || o.role == OwnDataRole::Validation)
}

/// Consistency of the organisation's data in a specification (ADR-0108). Ownership and checksum
/// are checked at submission, where the organisation is known.
fn validate_own_data(spec: &JobSpec) -> Result<(), AppError> {
    let using: Vec<&SpeciesRef> = spec.species.iter().filter(|s| s.own.is_some()).collect();
    match (&spec.own_dataset, using.is_empty()) {
        (None, false) => {
            return Err(AppError::Invalid(
                "a species uses the organisation's data but no dataset is given (own_dataset)"
                    .into(),
            ));
        }
        (Some(_), true) => {
            return Err(AppError::Invalid(
                "a dataset is given (own_dataset) but no species uses it".into(),
            ));
        }
        _ => {}
    }
    for sp in using {
        let Some(o) = &sp.own else { continue };
        if o.values.is_empty() || o.values.iter().any(|v| v.trim().is_empty()) {
            return Err(AppError::Invalid(format!(
                "{}: choose the values of the dataset's taxon column that are this species",
                sp.scientific_name
            )));
        }
        if o.values.len() > 200 {
            return Err(AppError::Invalid(format!(
                "{}: at most 200 taxon values per species",
                sp.scientific_name
            )));
        }
        if o.role == OwnDataRole::Validation && !o.gbif {
            return Err(AppError::Invalid(format!(
                "{}: data held out for validation need GBIF records for training (gbif: true)",
                sp.scientific_name
            )));
        }
    }
    Ok(())
}

/// Ranks that denote a single taxon with (presumably) one niche.
fn is_single_taxon(rank: &str) -> bool {
    matches!(rank, "SPECIES" | "SUBSPECIES" | "VARIETY" | "FORM")
}

fn rank_warning(rank: &str) -> Option<Warning> {
    (!is_single_taxon(rank)).then(|| {
        Warning::new(
            "pooled_taxa",
            Severity::Warning,
            format!(
                "The selected taxon is a {}: records of several species with different ecological niches are pooled into one model. Select a single species (ADR-0009).",
                rank.to_lowercase()
            ),
        )
        .with("rank", rank.to_lowercase())
    })
}

/// A model of pooled species violates the single-niche assumption of SDMs: its evidence level is
/// capped at Exploratory whatever the validation metrics (ADR-0009).
fn cap_evidence_for_pooled_taxa(e: &mut Evidence, rank: &str) {
    if matches!(
        e.level,
        EvidenceLevel::Indicative | EvidenceLevel::Supported
    ) {
        e.level = EvidenceLevel::Exploratory;
    }
    e.reasons.insert(
        0,
        format!(
            "✗ taxon rank {}: several species pooled — capped at Exploratory",
            rank.to_lowercase()
        ),
    );
}

/// Warnings about coordinates generalised by publishers (ADR-0011). `flags`: whether the source
/// reports generalisation (`None` for synthetic data: no warning).
fn generalised_warnings(c: &Cleaned, flags: Option<bool>, max_m: f64) -> Vec<Warning> {
    let mut w = Vec::new();
    match flags {
        Some(false) => w.push(Warning::new(
            "generalisation_flags_unavailable",
            Severity::Info,
            "This GBIF download format does not report coordinate generalisation; generalised records with small or missing uncertainty cannot be detected.",
        )),
        Some(true) if c.generalised_records > 0 => {
            // Warning when generalised records are ≥ 10 % of the records in the study area
            // (threshold proposed in ADR-0011, pending the ecologists' review).
            let share = if c.records_in_area > 0 {
                c.generalised_in_area as f64 / c.records_in_area as f64
            } else {
                0.0
            };
            let severity = if c.records_in_area > 0 && c.generalised_in_area * 10 >= c.records_in_area {
                Severity::Warning
            } else {
                Severity::Info
            };
            w.push(
                Warning::new(
                    "generalised_records",
                    severity,
                    format!(
                        "{} records were generalised by their publishers (sensitive species or observer request): {} excluded, {} retained with stated precision ≤ {max_m} m. Retained presences may under-represent the most sensitive sites.",
                        c.generalised_records, c.generalised_excluded, c.generalised_retained
                    ),
                )
                .with("n", c.generalised_records)
                .with("excluded", c.generalised_excluded)
                .with("retained", c.generalised_retained)
                .with("max_m", max_m)
                .with("share", share),
            );
        }
        _ => {}
    }
    w
}

/// Info line of the sampling-bias diagnostic (ADR-0013); never changes the evidence level.
fn effort_warning(d: &SamplingBias) -> Warning {
    Warning::new(
        "effort_concentration",
        Severity::Info,
        format!(
            "{:.0} % of the presence cells lie in the {:.0} % of the study area with most records of the target group {}{} ({:.0} % expected if presences ignored observer effort). Sampling diagnostic only; it does not change the evidence level.",
            d.presences_in_top10pct_effort_cells * 100.0,
            d.expected * 100.0,
            d.target_group,
            if d.synthetic { " (synthetic effort)" } else { "" },
            d.expected * 100.0
        ),
    )
    .with("share", d.presences_in_top10pct_effort_cells)
    .with("expected", d.expected)
    .with("class", d.target_group.clone())
    .with("synthetic", d.synthetic)
}

fn data_warnings(
    w: &mut Vec<Warning>,
    retained: usize,
    truncated: bool,
    total: u64,
    fetched: usize,
    licences: &std::collections::BTreeMap<String, usize>,
    demo: bool,
) {
    if demo {
        w.push(Warning::new("synthetic", Severity::Info, "Synthetic demonstration data: a virtual species on artificial predictors. Results illustrate the workflow only."));
    }
    if truncated {
        w.push(Warning::new(
            "truncated",
            Severity::Warning,
            format!("{total} records match; {fetched} were retrieved as a spatially stratified subsample. For final reports use a GBIF download (DOI)."),
        ).with("total", total).with("fetched", fetched));
    }
    if retained < crate::fit::MIN_PRESENCES {
        w.push(
            Warning::new(
                "too_few_presences",
                Severity::Blocking,
                format!(
                    "{retained} usable presence cells; at least {} are needed to fit a model.",
                    crate::fit::MIN_PRESENCES
                ),
            )
            .with("n", retained)
            .with("min", crate::fit::MIN_PRESENCES),
        );
    } else if retained < 25 {
        w.push(Warning::new("few_presences", Severity::Warning, format!("{retained} usable presence cells: results will be exploratory at best (≥ 25 recommended).")).with("n", retained));
    }
    if let Some(nc) = licences.get("CC_BY_NC_4_0")
        && *nc > 0
        && !demo
    {
        w.push(
            Warning::new(
                "non_commercial",
                Severity::Info,
                format!("{nc} records are CC BY-NC (excluded by default for commercial projects)."),
            )
            .with("n", *nc),
        );
    }
    w.push(Warning::new(
        "sampling_bias",
        Severity::Info,
        "Occurrence records come from opportunistic surveys; they reflect observer effort as well as habitat. No bias correction is applied.",
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_engine(name: &str) -> (Engine, std::path::PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("niche-wait-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let e = Engine::open(Config {
            data_dir: dir.join("data"),
            cache_dir: dir.join("cache"),
            max_records: 5000,
            gbif: None,
            secret_key: None,
            mirror: None,
            limits: Limits::default(),
            retention_days: 0,
        })
        .expect("engine");
        let spec = JobSpec {
            name: "wait".into(),
            species: vec![SpeciesRef {
                key: demo::SPECIES_KEY,
                scientific_name: demo::SPECIES_NAME.into(),
                own: None,
            }],
            area: StudyArea {
                name: "Demo region".into(),
                geometry: demo::area_geojson(),
                places: vec![],
            },
            layers: demo::LAYERS.iter().map(|l| l.0.to_string()).collect(),
            resolution_factor: 1,
            filters: OccurrenceFilters::default(),
            model: ModelSettings::default(),
            validation: ValidationSettings::default(),
            seed: 42,
            demo: true,
            projections: vec![],
            own_dataset: None,
        };
        let id = e.submit(spec).expect("submit").id;
        (e, dir, id)
    }

    #[test]
    fn a_slow_gbif_download_is_visible_and_can_be_skipped() {
        use niche_data::gbif_download::Step;
        let (e, dir, id) = demo_engine("slow");
        e.note_download(&id, &Step::Requesting);
        let w = e.gbif_wait(&id).expect("requesting");
        assert_eq!((w.status.as_str(), w.key.as_deref()), ("requesting", None));
        let key = "0012345-261019120000000";
        e.note_download(
            &id,
            &Step::Waiting {
                key,
                status: "PREPARING",
                elapsed: Duration::from_secs(60),
                created: None,
            },
        );
        let w2 = e.gbif_wait(&id).expect("waiting");
        assert_eq!(w2.status, "preparing");
        assert_eq!(
            w2.since, w.since,
            "the wait keeps its start once the key is known"
        );
        assert_eq!(
            w2.url.as_deref(),
            Some("https://www.gbif.org/occurrence/download/0012345-261019120000000")
        );
        // A download GBIF created earlier (same request before a restart) dates the wait.
        e.note_download(
            &id,
            &Step::Waiting {
                key,
                status: "RUNNING",
                elapsed: Duration::from_secs(10),
                created: Some("2026-10-19T08:32:44.213+00:00"),
            },
        );
        assert_eq!(
            e.gbif_wait(&id).expect("waiting").since,
            "2026-10-19T08:32:44.213Z"
        );
        if let Ok(mut waits) = e.waits.lock()
            && let Some(w) = waits.get_mut(&id)
        {
            w.since = w2.since.clone();
        }
        // Too early, and not in preparation: refused.
        assert!(matches!(e.skip_download(&id), Err(AppError::Conflict(_))));
        e.store
            .set_data_state(&id, DataState::Preparing)
            .expect("state");
        assert!(matches!(e.skip_download(&id), Err(AppError::Conflict(_))));
        // Waiting for 31 min: allowed, logged, and seen by the preparation's cancel check.
        if let Ok(mut waits) = e.waits.lock()
            && let Some(w) = waits.get_mut(&id)
        {
            w.since = (jiff::Timestamp::now() - jiff::SignedDuration::from_mins(31)).to_string();
        }
        e.skip_download(&id).expect("skip");
        assert!(e.skipping(&id));
        e.end_download(&id);
        assert!(e.gbif_wait(&id).is_none() && !e.skipping(&id));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// During a deployment two server processes share the data directory (ADR-0115): what the
    /// preparing process knows of a GBIF wait, and a request to continue without the download,
    /// reach it whichever process the request lands on.
    #[test]
    fn gbif_waits_and_skips_are_shared_between_processes() {
        use niche_data::gbif_download::Step;
        let (a, dir, id) = demo_engine("shared");
        let b = Engine::open(a.cfg.clone()).expect("second process");
        a.store
            .set_data_state(&id, DataState::Preparing)
            .expect("state");
        let created = (jiff::Timestamp::now() - jiff::SignedDuration::from_mins(40)).to_string();
        a.note_download(
            &id,
            &Step::Waiting {
                key: "0012345-261019120000000",
                status: "RUNNING",
                elapsed: Duration::from_secs(10),
                created: Some(&created),
            },
        );
        let seen = b.gbif_wait(&id).expect("the other process sees the wait");
        assert_eq!(seen.key.as_deref(), Some("0012345-261019120000000"));
        b.skip_download(&id)
            .expect("skip through the other process");
        assert!(a.skipping(&id), "the preparing process sees the request");
        a.end_download(&id);
        assert!(b.gbif_wait(&id).is_none() && !a.skipping(&id) && !b.skipping(&id));
        // Not preparing any more: a leftover file is not shown.
        a.note_download(
            &id,
            &Step::Waiting {
                key: "k",
                status: "RUNNING",
                elapsed: Duration::from_secs(1),
                created: None,
            },
        );
        a.store
            .set_data_state(&id, DataState::Ready)
            .expect("state");
        assert!(b.gbif_wait(&id).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pooled_taxa_are_flagged_and_capped() {
        assert!(rank_warning("SPECIES").is_none());
        assert!(rank_warning("SUBSPECIES").is_none());
        assert!(rank_warning("GENUS").is_some());
        let mut e = Evidence {
            level: EvidenceLevel::Supported,
            reasons: vec![],
        };
        cap_evidence_for_pooled_taxa(&mut e, "GENUS");
        assert_eq!(e.level, EvidenceLevel::Exploratory);
        assert!(e.reasons[0].contains("genus"));
        let mut e = Evidence {
            level: EvidenceLevel::Insufficient,
            reasons: vec![],
        };
        cap_evidence_for_pooled_taxa(&mut e, "FAMILY");
        assert_eq!(e.level, EvidenceLevel::Insufficient);
    }

    fn cleaned(records: usize, generalised: usize, excluded: usize, retained: usize) -> Cleaned {
        Cleaned {
            presence_cells: vec![],
            points: vec![],
            exclusions: vec![],
            licences: std::collections::BTreeMap::new(),
            generalised_records: generalised,
            generalised_excluded: excluded,
            generalised_retained: retained,
            records_in_area: records,
            generalised_in_area: generalised,
            thinned: 0,
        }
    }

    /// ADR-0011 warnings (the design notes §7 tests 3, 4 and 6).
    #[test]
    fn generalised_record_warnings() {
        // Synthetic data: nothing.
        assert!(generalised_warnings(&cleaned(20, 5, 3, 2), None, 1000.0).is_empty());
        // Download or mirror (SIMPLE_CSV): flags unavailable, whatever the counts.
        let w = generalised_warnings(&cleaned(20, 0, 0, 0), Some(false), 1000.0);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].code, "generalisation_flags_unavailable");
        assert_eq!(w[0].severity, Severity::Info);
        // Search API without generalised records: no warning.
        assert!(generalised_warnings(&cleaned(20, 0, 0, 0), Some(true), 1000.0).is_empty());
        // 1 of 20 records in the area (5 %): Info with both counts.
        let w = generalised_warnings(&cleaned(20, 1, 0, 1), Some(true), 1000.0);
        assert_eq!(w[0].code, "generalised_records");
        assert_eq!(w[0].severity, Severity::Info);
        assert_eq!(w[0].params["excluded"], 0);
        assert_eq!(w[0].params["retained"], 1);
        assert!(
            w[0].message
                .contains("0 excluded, 1 retained with stated precision ≤ 1000 m")
        );
        // 2 of 20 (10 %): Warning.
        let w = generalised_warnings(&cleaned(20, 2, 2, 0), Some(true), 1000.0);
        assert_eq!(w[0].severity, Severity::Warning);
    }

    #[test]
    fn invalid_bias_and_thinning_settings_are_rejected() {
        let (e, dir, _) = demo_engine("bias-settings");
        let base = JobSpec {
            name: "x".into(),
            species: vec![SpeciesRef {
                key: 0,
                scientific_name: demo::SPECIES_NAME.into(),
                own: None,
            }],
            area: StudyArea {
                name: "Demo".into(),
                geometry: demo::area_geojson(),
                places: vec![],
            },
            layers: vec!["demo_temp".into()],
            resolution_factor: 2,
            filters: OccurrenceFilters::default(),
            model: ModelSettings::default(),
            validation: ValidationSettings::default(),
            seed: 1,
            demo: true,
            projections: vec![],
            own_dataset: None,
        };
        assert!(e.validate(&base).is_ok());
        let msg = |f: &dyn Fn(&mut JobSpec)| {
            let mut s = base.clone();
            f(&mut s);
            match e.validate(&s) {
                Err(AppError::Invalid(m)) => m,
                other => panic!("expected a validation error, got {other:?}"),
            }
        };
        assert!(
            msg(&|s| s.model.background_method = "biased".into())
                .contains("random or target_group")
        );
        assert!(
            msg(&|s| s.model.target_group = Some("Aves".into()))
                .contains("target group applies only")
        );
        assert!(
            msg(&|s| {
                s.model.background_method = "target_group".into();
                s.model.target_group = Some("Aves; DROP".into());
            })
            .contains("GBIF class name")
        );
        for km in [0.0, -1.0, 100.5, f64::NAN] {
            assert!(msg(&|s| s.filters.thin_km = Some(km)).contains("thinning distance"));
        }
        let mut ok = base.clone();
        ok.model.background_method = "target_group".into();
        ok.filters.thin_km = Some(100.0);
        assert!(e.validate(&ok).is_ok());
        ok.model.target_group = Some("Aves".into());
        assert!(e.validate(&ok).is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }
}
