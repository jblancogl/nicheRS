// NicheRS HTTP API v1 — TypeScript contract.
// Mirrors the Rust DTOs in crates/niche-app/src/dto.rs. Keep both in sync; the Rust side is
// authoritative. All coordinates are WGS84 longitude/latitude (RFC 7946).

export type Severity = "info" | "warning" | "blocking";

export interface Warning {
  code: string;
  severity: Severity;
  /** English text; interfaces word known codes in their own language from `params`. */
  message: string;
  params?: Record<string, string | number | boolean>;
}

export interface Health {
  status: "ok";
  version: string;
  build: string;
  /** Local mode only; null in hosted mode. */
  data_dir: string | null;
  /** Whether remote sources (GBIF, CHELSA) were reachable at the last check. */
  online: boolean;
  /** Hosted mode: sign-in, organisations and quotas (ADR-0105). */
  accounts: boolean;
  /** Installed administrative boundary packs (study area by place name), e.g. ["fr-es-2026.1"]. */
  boundaries?: string[];
  /** A study is being modelled now (a restart would interrupt it); null if unknown. */
  busy?: boolean | null;
}

export interface SpeciesHit {
  key: number;
  scientific_name: string;
  canonical_name: string;
  rank: string;
  status: string;
  kingdom?: string | null;
  family?: string | null;
  vernacular_name?: string | null;
}

export interface LayerInfo {
  id: string; // e.g. "chelsa_bio1"
  name: string; // "Annual mean temperature"
  short_name: string; // "BIO1"
  unit: string; // "°C"
  source: string; // "CHELSA v2.1 (1981–2010)"
  licence: string; // "CC0 1.0"
  default_selected: boolean;
  /** Present when the variable has a known problem (e.g. BIO8/9/18/19 artefacts). */
  caution?: string | null;
  /** Display group (ADR-0015). */
  group: LayerGroup;
  /** Notice required by the source licence (Copernicus DEM, ESA WorldCover); null for CC0. */
  attribution?: string | null;
}

export type LayerGroup = "climate" | "terrain" | "land_cover" | "water";

export type Geometry =
  | { type: "Polygon"; coordinates: number[][][] }
  | { type: "MultiPolygon"; coordinates: number[][][][] };

export interface StudyArea {
  name: string;
  geometry: Geometry;
  /** Administrative units the area was built from (traceability; absent when drawn or uploaded). */
  places?: PlaceRef[];
}

export interface PlaceRef {
  id: string;
  name: string;
  pack: string;
  attribution: string;
}

/** GET /api/v1/places?q= — an administrative unit of an installed boundary pack. */
export interface PlaceHit {
  id: string;
  name: string;
  level: "country" | "region" | "department" | "intercommunality" | "municipality" | "protected";
  /** Designation of a protected area, in the source language ("Parc naturel régional", "Parque Natural"). */
  designation?: string;
  country: string;
  /** Enclosing units, nearest first. */
  within: string[];
  bbox: [number, number, number, number];
  area_km2: number;
  pack: string;
  attribution: string;
}

export interface PlaceGeometry {
  place: PlaceHit;
  geometry: Geometry;
}

/** A queued study waiting for its GBIF download; GBIF prepares it in its own queue, checked every 10 s. */
export interface GbifWait {
  key: string | null;
  /** "requesting", then GBIF's status ("preparing", "running"…), then "downloading". */
  status: string;
  since: string;
  /** Last check with GBIF. */
  checked_at: string;
  /** GBIF's own page for the download. */
  url: string | null;
  slow_after_s: number;
  skip_after_s: number;
  search_api_max_records: number;
}

/** Whether a study area fits the modelling-grid limit at each offered resolution (its bounding box counts). */
export interface AreaCheck {
  max_cells: number;
  resolutions: ResolutionCheck[];
  /** Date (YYYY-MM-DD) of the regional GBIF mirror serving this area without waiting for GBIF (ADR-0114). */
  gbif_mirror?: string | null;
}

export interface ResolutionCheck {
  /** Aggregation factor of the 30-arc-second grid (≈ km). */
  factor: number;
  cells: number;
  fits: boolean;
}

export type Licence = "CC0_1_0" | "CC_BY_4_0" | "CC_BY_NC_4_0";

export interface OccurrenceFilters {
  year_min: number; // default 1981 (CHELSA baseline start)
  max_uncertainty_m: number; // default 1000
  licences: Licence[]; // default CC0 + CC BY (non-commercial excluded)
  /** Distance thinning of presence cells (km, 0 < x ≤ 100); omitted/null = off. Science v1 phase 2. */
  thin_km?: number | null;
}

export interface GridInfo {
  bounds: [number, number, number, number]; // west, south, east, north
  width: number;
  height: number;
  cell_size_deg: number;
  cells_in_area: number;
  area_km2: number;
}

export interface ExclusionCount {
  reason: string; // human-readable, e.g. "Coordinate uncertainty > 1000 m"
  count: number;
}

export interface OccurrencePoint {
  lng: number;
  lat: number;
  year?: number | null;
  /** "retained" or the exclusion reason code (basis_of_record, licence, undated, too_old,
   *  generalised, uncertain, outside_area, not_land, no_env, duplicate_cell, thinned; ADR-0108:
   *  absence, validation, validation_absence, validation_excluded). */
  status: string;
  /** "gbif" or "own" (the organisation's data, ADR-0108); absent for GBIF-only studies. */
  source?: "gbif" | "own" | null;
}

export interface DataPreviewRequest {
  species_key: number;
  area: StudyArea;
  resolution_factor: number;
  filters: OccurrenceFilters;
  demo?: boolean;
  /** Study seed: distance thinning is drawn from it (omitted = default seed). */
  seed?: number;
  /** The organisation's dataset and how this species uses it (ADR-0108); both or neither. */
  own_dataset?: DatasetRef | null;
  own?: OwnSpeciesData | null;
}

export interface DataPreview {
  species_key: number;
  scientific_name: string;
  source: string; // "GBIF occurrence search API" | "Synthetic demo"
  query_url?: string | null;
  records_total_matching: number; // as reported by the source
  records_fetched: number;
  truncated: boolean;
  retained_cells: number;
  exclusions: ExclusionCount[];
  licences: Record<string, number>;
  points: OccurrencePoint[];
  grid: GridInfo;
  warnings: Warning[];
  /** Records per source when the organisation's data are used (ADR-0108). */
  by_source?: SourceCount[];
  /** Independent presence–absence sites of the organisation's data (counts only). */
  validation_sites?: ValidationSites | null;
}

export type FeatureClassSet = "L" | "LQ" | "H" | "LQH" | "LQHP";

export interface ModelSettings {
  /** "auto": grid search over feature_classes × beta_multipliers, selected by validation. */
  tuning: "auto" | "fixed";
  feature_classes: FeatureClassSet[];
  beta_multipliers: number[];
  background_points: number; // default 10000
  collinearity_threshold: number; // |Pearson r|, default 0.7
  /** "random" (default; omitted when random) or "target_group" (ADR-0013). */
  background_method?: BackgroundMethod;
  /** target_group only: GBIF class of the target group (e.g. "Aves"); omitted/null = the species' own class. */
  target_group?: string | null;
  /** Modelling algorithm (ADR-0017): "maxent" (default; omitted when maxent), "rf" (down-sampled
   *  random forest) or "ensemble" (MaxEnt + RF, mean of their percentile ranks). */
  algorithm?: Algorithm;
  /** rf / ensemble only: trees per forest; omitted = 500 (DEFAULT_RF_TREES), 100–2000. */
  rf_trees?: number | null;
  /** rf / ensemble only: also evaluate mtry = floor(p/2) besides floor(sqrt(p)); omitted when false. */
  rf_tune_mtry?: boolean;
}

export type BackgroundMethod = "random" | "target_group";

export type Algorithm = "maxent" | "rf" | "ensemble";

export type ValidationMethod = "spatial_block" | "spatial_grid" | "random_kfold";

export interface ValidationSettings {
  /** spatial_block (default, 4 median blocks), spatial_grid (square blocks sized by spatial
   *  autocorrelation, ADR-0014) or random_kfold. */
  method: ValidationMethod;
  /** spatial_block is always 4; when omitted, 5 for spatial_grid and 4 otherwise. */
  folds: number;
  /** spatial_grid only: block side in km; omitted/null = automatic (autocorrelation range). */
  block_km?: number | null;
  /** Null models of the selected setting (0 or omitted = off). ADR-0014. */
  null_models?: number;
  /** Compute the area of applicability layer (aoa.tif). ADR-0014. */
  aoa?: boolean;
}

export interface SpeciesRef {
  key: number;
  scientific_name: string;
  /** The organisation's data used for this species (ADR-0108); omitted = GBIF only. */
  own?: OwnSpeciesData | null;
}

export interface JobSpec {
  name: string;
  species: SpeciesRef[];
  area: StudyArea;
  layers: string[];
  resolution_factor: number; // 1 = native 30 arc-seconds (~1 km)
  filters: OccurrenceFilters;
  model: ModelSettings;
  validation: ValidationSettings;
  seed: number;
  demo?: boolean;
  /** Future-climate projections (opt-in, ADR-0016); omitted when none. */
  projections?: ProjectionSpec[];
  /** The organisation's dataset used by some species (ADR-0108); omitted = GBIF only. */
  own_dataset?: DatasetRef | null;
}

/** One future scenario (ADR-0016): period × SSP under one or more CMIP6 models. */
export interface ProjectionSpec {
  period: FuturePeriod;
  ssp: FutureSsp;
  gcms: string[];
}

export type FuturePeriod = "2011-2040" | "2041-2070" | "2071-2100";
export type FutureSsp = "ssp126" | "ssp370" | "ssp585";

/** Projection of the final model(s) to one future scenario (ADR-0016). Areas in km²
 * (exact ellipsoidal cell areas) over the cells projected; present P10 threshold. */
export interface ProjectionResult {
  period: FuturePeriod;
  ssp: FutureSsp;
  gcms: string[];
  threshold: number;
  /** Predictor ids replaced by their future climatology. */
  climate_variables: string[];
  /** Display names of predictors kept at present values. */
  static_variables?: string[];
  cells: number;
  cells_missing: number;
  present_suitable_km2: number;
  future_suitable_km2: number;
  stable_km2: number;
  gained_km2: number;
  lost_km2: number;
  unsuitable_km2: number;
  stable_all_gcms_km2: number;
  gained_all_gcms_km2: number;
  lost_all_gcms_km2: number;
  mean_gcm_sd: number;
  /** Share of the projected area with MESS < 0 under at least one GCM. */
  novel_fraction: number;
  per_gcm: GcmProjection[];
  synthetic?: boolean;
}

export interface GcmProjection {
  gcm: string;
  future_suitable_km2: number;
  gained_km2: number;
  lost_km2: number;
  novel_fraction: number;
  mean_suitability: number;
}

/** Point values of one projection (change: 1 unsuitable in both, 2 lost, 3 gained, 4 suitable in both). */
export interface ProjectionSample {
  period: FuturePeriod;
  ssp: FutureSsp;
  /** Climate models of the scenario. */
  gcms: number;
  mean: number | null;
  sd: number | null;
  change: number | null;
  agreement: number | null;
  mess: number | null;
}

export type JobStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled" | "interrupted";

export type SpeciesRunStatus = "pending" | "running" | "succeeded" | "failed" | "skipped" | "cancelled";

export interface SpeciesRunSummary {
  key: number;
  scientific_name: string;
  status: SpeciesRunStatus;
  evidence_level?: EvidenceLevel | null;
  error?: string | null;
}

export interface Job {
  id: string;
  name: string;
  status: JobStatus;
  created_at: string; // RFC 3339
  started_at?: string | null;
  finished_at?: string | null;
  progress: number; // 0..1
  stage: string; // human-readable current stage
  species: SpeciesRunSummary[];
  error?: string | null;
  spec: JobSpec;
  /** Project grouping this study, if any. */
  project_id?: string | null;
  /** Display name of the account that created it (hosted mode). */
  created_by_name?: string | null;
}

/** A named group of an organisation's studies (e.g. a client file). */
export interface Project {
  id: string;
  name: string;
  created_at: string;
  studies: number;
  last_study_at?: string | null;
}

/** Server-sent event payload on GET /api/v1/jobs/{id}/events (event name "progress"). */
export interface JobEvent {
  seq: number;
  job_id: string;
  status: JobStatus;
  stage: string;
  progress: number;
  species_key?: number | null;
  message: string;
  level: "info" | "warning" | "error";
  at: string;
}

/** "validated": supported and confirmed on independent presence–absence data (ADR-0108). */
export type EvidenceLevel = "insufficient" | "exploratory" | "indicative" | "supported" | "validated";

export interface FoldMetrics {
  fold: number;
  n_test: number;
  cbi: number | null;
  or10: number;
  ormtp: number;
  auc: number;
}

export interface TuningRow {
  feature_classes: FeatureClassSet | "";
  beta_multiplier: number;
  or10_mean: number;
  cbi_mean: number | null;
  auc_mean: number;
  n_nonzero: number;
  selected: boolean;
  /** "rf" for random-forest rows (absent for MaxEnt rows); RF rows have empty feature_classes and
   *  beta_multiplier / n_nonzero 0. ADR-0017. */
  algorithm?: "rf" | null;
  /** Random-forest rows: variables tried at each split. */
  mtry?: number | null;
}

/** Selected setting. MaxEnt fields are empty/0 for "rf"; rf_* present for "rf" and "ensemble". */
export interface SelectedModel {
  feature_classes: FeatureClassSet | "";
  beta_multiplier: number;
  /** Absent = "maxent" (default and results before ADR-0017). */
  algorithm?: Algorithm | null;
  rf_trees?: number | null;
  rf_mtry?: number | null;
  rf_seed?: number | null;
}

/** Ensemble only: cross-validated metrics of each member on the same folds. */
export interface MemberValidation {
  algorithm: "maxent" | "rf";
  cbi_mean: number | null;
  or10_mean: number;
  auc_mean: number;
}

export interface Contribution {
  variable: string;
  /** MaxEnt percent contribution; for "rf", the random forest's permutation importance. */
  percent: number;
  /** MaxEnt permutation importance (null for "rf" and for results computed before it was reported). */
  permutation_importance?: number | null;
  /** Random-forest permutation importance, normalised to 100 ("rf" and "ensemble"). */
  rf_importance?: number | null;
}

export interface ThresholdArea {
  rule: "p10" | "mtp" | "max_sss";
  label: string;
  threshold: number;
  area_km2: number;
  fraction_of_study_area: number;
}

export interface SpeciesResult {
  species_key: number;
  scientific_name: string;
  status: SpeciesRunStatus;
  error?: string | null;
  evidence: { level: EvidenceLevel; reasons: string[] };
  data: {
    presences: number; // unique cells used for training
    background: number;
    records_fetched: number;
    exclusions: ExclusionCount[];
    source: string;
    query_url?: string | null;
    /** Coordinates generalised by publishers (ADR-0011); absent for synthetic data and older results. */
    generalised?: GeneralisedRecords | null;
    /** Records per source when the organisation's data are used (ADR-0108). */
    by_source?: SourceCount[];
  };
  variables_used: string[];
  variables_dropped: { variable: string; reason: string }[];
  selected: SelectedModel;
  tuning: TuningRow[];
  validation: {
    method: string;
    folds: FoldMetrics[];
    cbi_mean: number | null;
    or10_mean: number;
    ormtp_mean: number;
    auc_mean: number;
    /** spatial_grid only: how the blocks were sized and how many folds were obtained. */
    blocks?: BlockDesign | null;
    /** Null-model significance (absent when not requested). */
    null_model?: NullModelSummary | null;
    /** Ensemble only: the members' own metrics (the fields above are the ensemble's). */
    members?: MemberValidation[];
  };
  thresholds: ThresholdArea[];
  study_area_km2: number;
  extrapolation: { novel_fraction: number }; // share of study-area cells with MESS < 0
  /** Area of applicability (absent when not requested). */
  aoa?: AoaSummary | null;
  /** Background actually used (absent for the default random background). ADR-0013. */
  background_sampling?: BackgroundSampling | null;
  /** Sampling-bias diagnostic (present when a target-group effort surface exists). */
  sampling_bias?: SamplingBias | null;
  /** Independent presence–absence validation on the organisation's data (ADR-0108). */
  independent_validation?: IndependentValidation | null;
  uncertainty: { mean_fold_sd: number };
  contributions: Contribution[];
  /** Marginal response curves of the final MaxEnt model (absent in older results and for "rf"). */
  response_curves?: ResponseCurve[];
  /** Partial dependence of the random forest ("rf", "ensemble"; Friedman 2001): averaged over the
   *  other variables' observed values, not a MaxEnt response curve. */
  partial_dependence?: ResponseCurve[];
  bounds: [number, number, number, number];
  files: { name: string; description: string; media_type: string }[];
  warnings: Warning[];
  /** Future-climate projections (ADR-0016); absent when none was requested. */
  projections?: ProjectionResult[];
}

/** Records whose coordinates were generalised by the publisher (ADR-0011). */
export interface GeneralisedRecords {
  /** The source reports generalisation (search API yes; SIMPLE_CSV downloads and the mirror no). */
  flags_available: boolean;
  records: number;
  /** Excluded by the "generalised" rule (precision not stated or > max uncertainty). */
  excluded: number;
  /** Kept as presences (stated precision within the limit). */
  retained: number;
}

/** How the background was drawn when a target group was requested (ADR-0013). */
export interface BackgroundSampling {
  requested: BackgroundMethod;
  /** Method actually used: "target_group", or "random" after a fallback. */
  method: BackgroundMethod;
  target_group?: string | null;
  /** Cells of the study area with at least one target-group record. */
  eligible_cells: number;
  /** Why the target group could not be used (English). */
  fallback_reason?: string | null;
  /** The effort surface is synthetic (demo). */
  synthetic?: boolean;
}

/** Sampling-bias diagnostic from the target-group effort surface. Descriptive: no effect on the evidence level. */
export interface SamplingBias {
  target_group: string;
  records: number;
  cells_with_effort: number;
  share_of_area_with_effort: number;
  /** Highest records in one cell (effort-map legend). */
  max_cell_records: number;
  /** Share of presence cells in the most-surveyed cells covering ≈10 % of the study area. */
  presences_in_top10pct_effort_cells: number;
  /** Share of the study area those cells cover (≈0.10): the expected share without bias. */
  expected: number;
  synthetic?: boolean;
}

/** Block design of spatial_grid validation (ADR-0014). */
export interface BlockDesign {
  block_km: number;
  /** "user" (given in the settings) or "autocorrelation" (median variogram range, clamped). */
  chosen_by: "user" | "autocorrelation";
  k_requested: number;
  /** Folds obtained: fewer than requested when fewer blocks contain presences. */
  k_achieved: number;
  autocorrelation?: AutocorrelationRange | null;
}

export interface AutocorrelationRange {
  /** Median of the fitted ranges before clamping; null if no variable could be fitted. */
  median_km: number | null;
  min_km: number;
  max_km: number;
  n_points: number;
  cutoff_km: number;
  variables: {
    variable: string;
    range_km: number | null;
    model: "exponential" | "spherical" | "gaussian" | null;
    /** The range hit the search bound: a lower bound only. */
    at_upper_bound: boolean;
  }[];
}

/** Observed metric vs models fitted to random locations. Never a "confidence". */
export interface NullStat {
  observed: number | null;
  mean: number | null;
  sd: number | null;
  z: number | null;
  /** Empirical one-sided p = (1 + #nulls at least as good) / (n + 1). */
  p_value: number;
  n: number;
}

export interface NullModelSummary {
  requested: number;
  completed: number;
  presences: number;
  /** How null presences were drawn: "uniform" over cells with predictor data, or "effort_weighted"
   *  (bias-matched: proportional to target-group records per cell, ADR-0013). */
  sampling: string;
  /** Mean held-out Boyce index (higher is better). */
  cbi: NullStat;
  /** Mean held-out OR10 (lower is better). */
  or10: NullStat;
}

/** Area of applicability (Meyer & Pebesma 2021). */
export interface AoaSummary {
  /** Dissimilarity-index threshold: cells above it are outside the AOA. */
  threshold: number;
  fraction_outside: number;
  n_train: number;
  weights: { variable: string; weight: number }[];
}

/** Relative suitability (cloglog) as one variable varies, the others at their presence mean. */
export interface ResponseCurve {
  variable: string;
  name: string;
  categorical: boolean;
  x: number[];
  y: number[];
  /** Range of the training data; outside it the curve is an extrapolation (model clamped). */
  training_min: number;
  training_max: number;
}

export interface JobResults {
  job: Job;
  species: SpeciesResult[];
}

export type RasterLayer = "suitability" | "uncertainty" | "extrapolation" | "binary" | "aoa" | "effort";

export interface SampleValue {
  in_area: boolean;
  suitability: number | null;
  uncertainty: number | null;
  mess: number | null;
  /** Dissimilarity index of the area of applicability (only when aoa.tif exists). */
  aoa_di?: number | null;
  /** Target-group records in the cell (only when the study has an effort layer). */
  effort?: number | null;
  /** Future-climate projections at the point (ADR-0016). */
  projections?: ProjectionSample[];
}

// ---- hosted mode (ADR-0105) -----------------------------------------------------------------

/** Monthly quota of modelled species (calendar month, UTC). */
export interface QuotaStatus {
  month: string; // YYYY-MM
  total: number;
  used: number;
  reserved: number;
  remaining: number;
}

// ---- account and organisation properties (ADR-0110) ----

/** Whose GBIF account a study's download uses. */
export type GbifSource = "user" | "organisation" | "shared" | "none";

/** A configured GBIF account (never the password). */
export interface GbifAccountInfo {
  /** Hidden from members who are not managers, for the organisation's account. */
  username: string | null;
  checked_at: string | null;
  /** GBIF's answer at the last check (null: never checked, or GBIF unreachable). */
  ok: boolean | null;
}

/** GET /api/v1/account/gbif */
export interface GbifSettings {
  own: GbifAccountInfo | null;
  organisation: GbifAccountInfo | null;
  /** The server's shared account exists (the last resort). */
  shared: boolean;
  /** Account the user's next studies will use. */
  effective: GbifSource;
  /** The server can store accounts (NICHE_SECRET_KEY is set). */
  can_store: boolean;
}

/** GET /api/v1/org: the user's organisation (every member reads it; managers edit it). */
export interface OrgProfile {
  name: string;
  quota: QuotaStatus;
  members: number;
  gbif: GbifAccountInfo | null;
  shared_gbif: boolean;
  can_store: boolean;
  can_manage: boolean;
}

/** GET /api/v1/auth/me */
export interface Me {
  name: string;
  email: string;
  organisation: string;
  quota: QuotaStatus;
  /** Natural Solutions administrator (ADR-0106). */
  is_admin: boolean;
  /** Manager of their own organisation (ADR-0109): invites and manages its members. */
  is_manager: boolean;
  /** Interface language chosen by the account ("en" | "fr" | "es"); null = browser default. */
  lang?: string | null;
}

// ---- administration (ADR-0106), /api/v1/admin/* -------------------------------------------

export interface Org {
  id: string;
  name: string;
  monthly_quota: number;
  created_at: string;
  disabled_at: string | null;
  /** The operator organisation (ADR-0112): only its members can be administrators. */
  is_operator: boolean;
}

export interface OrgOverview extends Org {
  quota: QuotaStatus;
  users: number;
  runs: number;
  open_invites: number;
}

/** Unused, unexpired invitation; `id` is a hash, not a usable link. */
export interface InviteInfo {
  id: string;
  created_at: string;
  expires_at: string;
}

export interface UserInfo {
  id: string;
  email: string;
  name: string;
  org_name: string;
  created_at: string;
  disabled_at: string | null;
  is_admin: boolean;
  is_manager: boolean;
  last_login_at: string | null;
  /** Member of the operator organisation (ADR-0112): may be made an administrator. */
  in_operator_org: boolean;
}

/** `GET /api/v1/admin/users`: one page of accounts (by organisation, then e-mail). */
export interface UserPage {
  users: UserInfo[];
  /** Accounts matching the filter. */
  total: number;
  /** Pass as `offset` for the next page; null at the end. */
  next_offset: number | null;
}

/** `org`: an organisation's id or name; `q`: part of a name, an e-mail or an organisation name. */
export interface UserQuery {
  org?: string;
  q?: string;
  limit?: number;
}

/** Changes recorded in the audit trail (ADR-0113); newer servers may add values. */
export type AuditAction =
  | "org_created"
  | "org_quota_changed"
  | "org_disabled"
  | "org_enabled"
  | "operator_designated"
  | "invite_created"
  | "invite_revoked"
  | "reset_link_created"
  | "account_disabled"
  | "account_enabled"
  | "account_deleted"
  | "manager_granted"
  | "manager_removed"
  | "admin_granted"
  | "admin_removed"
  | "org_gbif_set"
  | "org_gbif_removed"
  | "support_access";

/** `GET /api/v1/admin/audit`: one recorded change. Names are the current ones (null once erased). */
export interface AuditEntry {
  id: number;
  at: string;
  /** `cli`: made on the server (`niche admin`), no account. */
  via: "web" | "cli";
  action: AuditAction | (string & {});
  actor_id: string | null;
  actor_name: string | null;
  org_id: string | null;
  org_name: string | null;
  user_id: string | null;
  user_name: string | null;
  user_email: string | null;
  job_id: string | null;
  job_name: string | null;
  /** Small facts of the change, e.g. `{from, to}` for a quota. */
  detail: Record<string, unknown> | null;
}

/** Newest first; pass `next` as `before` for older entries. */
export interface AuditPage {
  entries: AuditEntry[];
  next: number | null;
}

/** `org`: an organisation's history; `user`: what was done to or by an account (ids). */
export interface AuditQuery {
  org?: string;
  user?: string;
  limit?: number;
}

/** `POST /api/v1/admin/users/admin` (ADR-0112); `password` is the requesting administrator's. */
export interface GrantAdminRequest {
  email: string;
  password: string;
}

export interface UsageSummary {
  org_id: string;
  org_name: string;
  month: string;
  species: number;
  monthly_quota: number;
}

export type DataState = "pending" | "preparing" | "ready" | "nobatch";

export interface ActivityRow {
  id: string;
  name: string;
  org_id: string | null;
  org_name: string | null;
  created_by: string | null;
  status: JobStatus;
  data_state: DataState;
  stage: string;
  progress: number;
  species: number;
  created_at: string;
  started_at: string | null;
  finished_at: string | null;
  error: string | null;
  /** Failed studies: kind of problem. */
  cause: FailureCause | null;
  /** Queued studies waiting for GBIF. */
  gbif_wait?: GbifWait;
}

export type FailureCause = "gbif" | "climate" | "area" | "records" | "model" | "server" | "other";

/** State of the regional GBIF mirror (ADR-0114). */
export interface MirrorStatus {
  region: string;
  name: string;
  state: "ready" | "requesting" | "waiting" | "downloading" | "building" | "error" | "disabled";
  snapshot: {
    download_key: string;
    doi: string | null;
    created: string;
    built_at: string;
    records: number;
    taxa: number;
    bytes: number;
    usable: boolean;
  } | null;
  detail: string | null;
  updated_at: string | null;
  refresh_days: number;
  max_age_days: number;
}

export interface SystemStatus {
  version: string;
  build: string;
  started_at: string;
  data_dir: string;
  disk: { total_bytes: number; available_bytes: number } | null;
  storage: { database_bytes: number; jobs_bytes: number; cache_bytes: number; boundaries_bytes: number; backups_bytes: number };
  jobs: { total: number; queued: number; preparing: number; running: number; failed_7d: number };
  boundaries: { name: string; units: number }[];
  db_backups: { last_at: string | null; count: number; keep: number };
  volume_backup: string | null;
  gbif: { configured: boolean; user: string | null; ok: boolean | null; detail: string | null; checked_at: string | null };
  /** Regional GBIF mirror (ADR-0114); null when the server keeps none. */
  gbif_mirror: MirrorStatus | null;
  retention_days: number;
}

/** One organisation's studies in a month: outcomes, computing time, GBIF waits, failure causes. */
export interface OrgMetrics {
  org_name: string;
  month: string;
  studies: number;
  succeeded: number;
  failed: number;
  cancelled: number;
  species: number;
  compute_s: number;
  compute_s_per_species: number | null;
  gbif_wait_s_mean: number | null;
  gbif_wait_s_max: number | null;
  failures: Partial<Record<FailureCause, number>>;
}

/** A new one-time link: `path` is "/invite#…" or "/reset#…"; prefix the page origin. */
export interface LinkCreated {
  path: string;
  days_valid: number;
}

export type LinkKind = "invite" | "reset";

/** POST /api/v1/auth/link */
export interface LinkInfo {
  kind: LinkKind;
  org_name: string;
  email: string | null;
  expires_at: string;
}

export interface ProblemDetails {
  type: string;
  title: string;
  status: number;
  detail?: string;
}

// Endpoints (all JSON unless noted):
// GET  /api/v1/health                                   -> Health
// GET  /api/v1/species/search?q=                        -> SpeciesHit[]
// GET  /api/v1/layers                                   -> LayerInfo[]
// POST /api/v1/occurrences/preview   DataPreviewRequest -> DataPreview
// GET  /api/v1/places?q=&country=&limit=               -> PlaceHit[] (installed boundary packs)
// GET  /api/v1/places/at?lng=&lat=                      -> PlaceHit[] (units containing the point)
// GET  /api/v1/places/{id}                              -> PlaceGeometry
// GET  /api/v1/jobs                                     -> Job[]
// POST /api/v1/jobs[?project_id=]    JobSpec            -> Job (201)
// PATCH /api/v1/jobs/{id}  {name?, project_id? (null = no project)} -> Job
// GET  /api/v1/projects                                 -> Project[]
// POST /api/v1/projects   {name}                        -> Project (201; 409 duplicate name)
// PATCH /api/v1/projects/{id} {name}                    -> Project
// DELETE /api/v1/projects/{id}                          -> 204 (its studies are kept, without project)
// GET  /api/v1/jobs/{id}                                -> Job
// GET  /api/v1/jobs/{id}/events                         -> text/event-stream of JobEvent
// POST /api/v1/jobs/{id}/cancel                         -> Job
// GET  /api/v1/jobs/{id}/results                        -> JobResults
// GET  /api/v1/jobs/{id}/manifest                       -> execution manifest JSON
// GET  /api/v1/jobs/{id}/species/{key}/tiles/{layer}/{z}/{x}/{y}.png[?threshold=0.3] -> PNG
//      layer: suitability | uncertainty | extrapolation | binary | aoa (hatched outside the AOA)
//             | effort (target-group records per cell, log scale; only with a target-group background)
// GET  /api/v1/jobs/{id}/species/{key}/sample?lng=&lat= -> SampleValue
// GET  /api/v1/jobs/{id}/species/{key}/files/{name}     -> file download (?inline=1: sandboxed HTML report view)
// GET  /api/v1/jobs/{id}/species/{key}/occurrences     -> OccurrencePoint[] (status: retained | exclusion code)
// GET  /api/v1/layers?demo=true                         -> LayerInfo[] of the synthetic demo predictors
// DELETE /api/v1/jobs/{id}                              -> 204 (409 if not finished)
// Hosted mode only (404 in local mode); every other route then needs the session cookie (401):
// POST /api/v1/auth/link    {token}                     -> LinkInfo
// POST /api/v1/auth/accept  {token,name,email,password} -> Me + session cookie
// POST /api/v1/auth/reset   {token,password}            -> Me + session cookie
// POST /api/v1/auth/login   {email,password}            -> Me + session cookie (401, 429)
// POST /api/v1/auth/lang {lang}                       -> 204 (account interface language)
// POST /api/v1/auth/logout                              -> 204
// GET  /api/v1/auth/me                                  -> Me (401 without session)
// POST /api/v1/jobs over the monthly quota              -> 409 "Monthly quota reached"
// Administrators only (403 otherwise), ADR-0106:
// GET  /api/v1/admin/orgs                               -> OrgOverview[]
// POST /api/v1/admin/orgs   {name, monthly_quota}       -> Org (201; 409 duplicate name)
// PATCH /api/v1/admin/orgs/{id} {monthly_quota?, disabled?} -> Org
// GET  /api/v1/admin/orgs/{id}/invites                  -> InviteInfo[]
// POST /api/v1/admin/orgs/{id}/invites                  -> LinkCreated (201)
// POST /api/v1/admin/invites/{id}/revoke                -> 204
// GET  /api/v1/admin/orgs/{id or name}                  -> OrgOverview
// GET  /api/v1/admin/users[?org=&q=&offset=&limit=]     -> UserPage (100 per page, at most 500)
// GET  /api/v1/admin/users/{id}                         -> UserInfo
// POST /api/v1/admin/users/disable {email, disabled}    -> 204
// POST /api/v1/admin/users/reset   {email}              -> LinkCreated (201)
// POST /api/v1/admin/users/delete  {email, confirm_email} -> 204 (GDPR erasure; jobs kept)
// GET  /api/v1/admin/usage[?month=YYYY-MM]              -> UsageSummary[]
// GET  /api/v1/admin/usage.csv[?month=YYYY-MM]          -> text/csv, one line per modelled species
// GET  /api/v1/admin/activity[?org=]                    -> ActivityRow[]
// POST /api/v1/admin/users/admin {email, password}      -> 204 (ADR-0112; 403 wrong password,
//      409 outside the operator organisation, 429 too many wrong passwords)
// GET  /api/v1/admin/audit[?org=&user=&before=&limit=]  -> AuditPage (ADR-0113, newest first)
// GET  /api/v1/org/audit[?before=&limit=]               -> AuditPage (managers: their organisation)
// Errors: application/problem+json (ProblemDetails).

// ---- feedback (ADR-0111) ---------------------------------------------------------------------

export type FeedbackKind = "problem" | "idea" | "other";
/** Whether Natural Solutions has dealt with it. */
export type FeedbackStatus = "open" | "closed";

/** Screenshot, image or voice recording; `data` is standard base64. */
export interface FeedbackUpload {
  name: string;
  mime: string;
  data: string;
}

/** `POST /api/v1/feedback`: a message, attachments, or both. */
export interface FeedbackRequest {
  kind?: FeedbackKind;
  message?: string;
  page?: string | null;
  context?: Record<string, unknown>;
  attachments?: FeedbackUpload[];
}

export interface FeedbackFile {
  id: string;
  name: string;
  mime: string;
  size: number;
}

export interface Feedback {
  id: string;
  created_at: string;
  kind: FeedbackKind;
  message: string;
  page: string | null;
  context: Record<string, unknown>;
  /** null once the author's account has been erased. */
  author: string | null;
  author_email: string | null;
  org_name: string | null;
  status: FeedbackStatus;
  files: FeedbackFile[];
}

// ---- the organisation's own occurrence data (ADR-0108) ---------------------------------------

/** A dataset used by a study: id and the SHA-256 of its normalised records (checked at submission). */
export interface DatasetRef {
  id: string;
  records_sha256: string;
}

/** "training": presences join the training data (absences never do); "validation": held out,
 *  every record used only for the independent presence–absence validation. */
export type OwnDataRole = "training" | "validation";

export interface OwnSpeciesData {
  /** Values of the dataset's taxon column that are this species. */
  values: string[];
  role: OwnDataRole;
  /** GBIF records also used for training (always true for "validation"). */
  gbif: boolean;
}

/** Columns chosen in the browser, one array per column (same length). Values stay strings. */
export interface ImportColumns {
  taxon: string[];
  x: string[];
  y: string[];
  date?: string[] | null;
  uncertainty?: string[] | null;
  status?: string[] | null;
  survey?: string[] | null;
}

/** POST /api/v1/datasets. Only the chosen columns are sent; the file stays in the browser. */
export interface ImportRequest {
  name: string;
  file_name: string;
  /** SHA-256 of the original file, computed in the browser. */
  file_sha256: string;
  /** "EPSG:<code>" from GET /api/v1/coordinate-systems. */
  crs: string;
  columns: ImportColumns;
  /** File line of each row (1 = header). */
  lines?: number[] | null;
  absent_values: string[];
  mapping: Record<string, string>;
  synthetic?: boolean;
}

export type ImportRejection = "missing_taxon" | "bad_coordinates" | "outside_crs_domain" | "bad_date" | "bad_uncertainty";

export interface RejectedRows {
  /** One of `ImportRejection`. */
  reason: string;
  count: number;
  /** File lines of the first 10 rows. */
  lines: number[];
}

export interface TaxonValue {
  value: string;
  records: number;
  presences: number;
  absences: number;
}

export interface ImportReport {
  rows_read: number;
  rows_imported: number;
  rejected: RejectedRows[];
  bounds_wgs84?: [number, number, number, number] | null;
  presences: number;
  absences: number;
  with_survey: number;
  taxa: TaxonValue[];
  warnings: Warning[];
}

/** An imported dataset (never its records). */
export interface DatasetInfo {
  id: string;
  name: string;
  created_at: string;
  file_name: string;
  file_sha256: string;
  records_sha256: string;
  crs: string;
  absent_values: string[];
  mapping: Record<string, string>;
  synthetic?: boolean;
  report: ImportReport;
}

export interface CrsOption {
  code: string;
  name: string;
  usage: string;
}

/** GBIF backbone match of one name (POST /api/v1/species/match). */
export interface TaxonMatch {
  name: string;
  key?: number | null;
  scientific_name?: string | null;
  rank?: string | null;
  status?: string | null;
  /** EXACT, FUZZY, HIGHERRANK, NONE, or ERROR when GBIF could not answer. */
  match_type: string;
  confidence?: number | null;
  /** Proposed automatically (exact, species rank or below); the user confirms. */
  accepted: boolean;
}

export interface SourceCount {
  source: "gbif" | "own";
  records: number;
  retained_cells: number;
  /** Records per status code (retained, duplicate_cell, absence, validation…). */
  statuses: Record<string, number>;
  /** gbif only: records in a cell already given by the organisation's data. */
  in_own_cells?: number | null;
}

export interface ValidationSites {
  records: number;
  /** Records excluded by the cleaning rules or the leakage rule (training_cell), by code. */
  excluded: Record<string, number>;
  presence_sites: number;
  absence_sites: number;
  conflicting_cells: number;
  leakage_excluded: number;
}

export interface PaThreshold {
  /** "p10" or "max_sss". */
  rule: string;
  threshold: number;
  sensitivity?: number | null;
  specificity?: number | null;
  tss?: number | null;
  kappa?: number | null;
  true_pos: number;
  false_pos: number;
  false_neg: number;
  true_neg: number;
}

/** A diagnostic only: suitability is not a probability of presence. */
export interface CalibrationRow {
  lower: number;
  upper: number;
  mean_suitability: number;
  observed_presence_share: number;
  n: number;
}

export interface SiteSpread {
  extent_km: [number, number];
  share_of_study_extent: number;
  mean_nearest_km?: number | null;
}

export interface IndependentValidation {
  dataset_id: string;
  dataset_name: string;
  role: OwnDataRole;
  synthetic?: boolean;
  sites: ValidationSites;
  auc?: number | null;
  thresholds: PaThreshold[];
  brier?: number | null;
  prevalence?: number | null;
  calibration: CalibrationRow[];
  spread?: SiteSpread | null;
  /** Why metrics are missing or limited (English). */
  note?: string | null;
  /** Every criterion of the Validated level is met. */
  validated: boolean;
}

// ---- Decision-support analyses (ADR-0018; mirrors dto.rs) ---------------------------------

/** `POST /jobs/{id}/analyses`: on-demand analysis of a finished study (no refit). */
export type AnalysisRequest =
  | { kind: "priority"; species_key: number; top_n?: number | null }
  | { kind: "footprint"; name: string; geometry: Geometry }
  | { kind: "richness"; min_evidence?: EvidenceLevel | null }
  | { kind: "comparison"; other_job_id: string; species_key: number };

export type AnalysisKind = AnalysisRequest["kind"];

export type AnalysisLayerId =
  | "priority"
  | "priority_class"
  | "richness"
  | "suitability_sum"
  | "hotspot"
  | "contributing"
  | "difference";

export interface AnalysisLayer {
  id: AnalysisLayerId;
  min: number;
  max: number;
}

export interface Analysis {
  id: string;
  kind: AnalysisKind;
  job_id: string;
  created_at: string;
  /** English display title. */
  title: string;
  species_key?: number | null;
  /** The study is the synthetic demo. */
  synthetic?: boolean;
  files: { name: string; description: string; media_type: string }[];
  layers: AnalysisLayer[];
  warnings: Warning[];
  priority?: PriorityResult | null;
  footprint?: FootprintResult | null;
  richness?: RichnessResult | null;
  comparison?: ComparisonResult | null;
}

export type PriorityClassId = "prospect_first" | "known" | "low_unsampled" | "low_sampled";

export interface PriorityClass {
  /** 1–4, as in `priority_class.tif`. */
  code: number;
  id: PriorityClassId;
  cells: number;
  area_km2: number;
  fraction_of_study_area: number;
}

export interface PriorityCandidate {
  rank: number;
  lng: number;
  lat: number;
  /** West, south, east, north. */
  bbox: [number, number, number, number];
  cells: number;
  area_km2: number;
  mean_priority: number;
  mean_suitability: number;
  max_suitability: number;
  mean_effort?: number | null;
  mean_distance_km?: number | null;
  extrapolated_share: number;
}

export interface PriorityResult {
  species_key: number;
  scientific_name: string;
  evidence_level: EvidenceLevel;
  /** "effort" (target-group sampling effort) or "distance" (proxy: distance to known records). */
  method: "effort" | "distance";
  target_group?: string | null;
  effort_synthetic?: boolean;
  known_records?: number | null;
  /** The species' P10 threshold. */
  suitability_threshold: number;
  /** Median effort (records per cell) or median distance (km). */
  sampling_split: number;
  study_area_km2: number;
  classes: PriorityClass[];
  candidates: PriorityCandidate[];
  candidate_groups: number;
  /** Candidate cells: "prospect first" cells with a score ≥ this (top 10 % of the area). */
  candidate_min_priority: number;
}

export interface FootprintThreshold {
  rule: "p10" | "mtp" | "max_sss";
  label: string;
  threshold: number;
  suitable_km2: number;
  share_of_footprint: number;
  species_suitable_km2: number;
  share_of_species_area: number;
}

export interface FootprintSpecies {
  species_key: number;
  scientific_name: string;
  evidence_level?: EvidenceLevel | null;
  algorithm?: string | null;
  thresholds: FootprintThreshold[];
  mean_suitability?: number | null;
  max_suitability?: number | null;
  mean_uncertainty?: number | null;
  extrapolated_share?: number | null;
  outside_aoa_share?: number | null;
  note?: string | null;
}

export interface FootprintResult {
  name: string;
  geometry: Geometry;
  area_km2: number;
  in_study_km2: number;
  cells: number;
  mean_cell_km2: number;
  species: FootprintSpecies[];
}

export interface RichnessSpecies {
  species_key: number;
  scientific_name: string;
  evidence_level?: EvidenceLevel | null;
  threshold?: number | null;
  suitable_km2?: number | null;
  /** Why it is left out (English). */
  reason?: string | null;
}

export interface RichnessBin {
  species: number;
  cells: number;
  area_km2: number;
}

export interface HotspotSummary {
  target_share: number;
  cells: number;
  area_km2: number;
  fraction_of_study_area: number;
  min_species: number;
}

export interface RichnessResult {
  min_evidence: EvidenceLevel;
  threshold_rule: "p10";
  included: RichnessSpecies[];
  excluded: RichnessSpecies[];
  study_area_km2: number;
  max_species: number;
  max_suitability_sum: number;
  histogram: RichnessBin[];
  hotspot: HotspotSummary;
}

export interface ComparedStudy {
  job_id: string;
  name: string;
  algorithm: string;
  evidence_level: EvidenceLevel;
  presences: number;
  cbi_mean?: number | null;
  or10_mean: number;
  auc_mean: number;
  validation_method: string;
  background: string;
  thresholds: ThresholdArea[];
  extrapolated_share: number;
  outside_aoa_share?: number | null;
  variables: string[];
}

export interface SettingDiff {
  /** Dotted path in the study specification. */
  path: string;
  /** null = absent (default). */
  a: unknown;
  b: unknown;
}

export interface BinaryAgreement {
  both_km2: number;
  only_a_km2: number;
  only_b_km2: number;
  neither_km2: number;
}

export interface ComparisonResult {
  species_key: number;
  scientific_name: string;
  a: ComparedStudy;
  b: ComparedStudy;
  settings: SettingDiff[];
  common_cells: number;
  common_km2: number;
  schoener_d: number;
  spearman: number;
  mean_difference: number;
  mean_abs_difference: number;
  p10_agreement: BinaryAgreement;
}
