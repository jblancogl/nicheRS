# Execution manifest — schema v1 (`nichers.run-manifest/1`)

Status: Proposed · Date: 2026-10-15 · Decision record: ADR-0203 · Related: `engineering-principles.md` §2, §5.
Tags: [FACT] verified · [ASSUMPTION] · [OPEN] · [REC].

## Scope

This document defines the per-run reproducibility record that NicheRS writes next to every set of run outputs: train, evaluate, predict or project. It records **what was used, how, by which software, with which randomness, and what was produced**. It must be enough to:

1. re-run the run (`niche rerun manifest.json`);
2. verify that artifacts were not altered (`niche models verify`);
3. write the methods section of a report (later: ODMAP pre-fill);
4. map mechanically to a Workflow Run RO-Crate (Process Run Crate profile) later.

Non-goals for v1: RDF/JSON-LD, full PROV graphs, and storage of raw occurrence coordinates (hashes and counts only).

## Findings: design rules

- [REC] **One file per run**: `<run_dir>/manifest.json`. UTF-8, written atomically (temp file then rename) *after* all artifacts are fsynced. A run with no manifest is incomplete (12F-IX).
- [REC] Version field `schema`: `"nichers.run-manifest/1"`. Minor additions are optional fields, keeping the same major. Removing or renaming a field or changing its semantics bumps the major. The JSON Schema is generated from the Rust types (`schemars` 1.2.3) and committed at `schemas/run-manifest-1.schema.json`. Tests validate against it with the `jsonschema` crate.
- [REC] **Hashes**: SHA-256, lowercase hex, prefixed `sha256:`. SHA-256 is used because it is universal in BagIt, RO-Crate and STAC file extension tooling; BLAKE3 is faster but less interoperable [ASSUMPTION about ecosystem support]. Directories and multi-file inputs are hashed as the SHA-256 of a sorted `path\tsha256\n` listing.
- [REC] **Paths** are relative to the run directory or to a content-addressed cache key. No absolute user paths, which are a privacy leak and non-portable.
- [REC] **No sensitive data**: no coordinates, usernames or hostnames. `host` holds only OS, architecture and CPU features relevant to floating-point reproducibility.
- [REC] **Effective config only**: record the parameters that affect results, with defaults resolved. Do not record ports or log levels. Worker count is recorded because reduction order may depend on it.
- [REC] **Vocabulary aligned with W3C PROV** [E16]: `inputs` (prov:used), `outputs` (prov:wasGeneratedBy), `software` / `agent` (prov:wasAssociatedWith), `parent_runs` (prov:wasDerivedFrom).
- [REC] **Integrity**: `integrity.manifest_sha256` is computed over the canonical JSON (RFC 8785 JCS [UNVERIFIED – prior knowledge]) of the document with the `integrity` member removed.

## Field list (v1)

`R` = required, `O` = optional.

| Path | R/O | Type | Meaning |
|---|---|---|---|
| `schema` | R | string | `"nichers.run-manifest/1"` |
| `run_id` | R | string (UUIDv7) | Time-ordered unique id |
| `kind` | R | enum | `train` \| `evaluate` \| `predict` \| `project` \| `pipeline` |
| `status` | R | enum | `succeeded` \| `failed` \| `cancelled` (manifests are also written for failures) |
| `created_at`, `started_at`, `finished_at` | R | RFC 3339 UTC | Timestamps |
| `parent_runs[]` | O | run_id | Derivation (for example a prediction from a trained model run) |
| `software.name`, `.version` | R | string | `nichers`, SemVer |
| `software.git_commit`, `.git_dirty` | R | string, bool | Build provenance |
| `software.target_triple`, `.rustc`, `.profile` | R | string | Build environment |
| `software.cargo_lock_sha256`, `.frontend_lock_sha256` | R | hash | Dependency lock (12F-II) |
| `software.components[]` | R | {name, version} | Scientific crates, for example `niche-maxent` |
| `software.golden_suite_version` | R | string | Golden suite that this build passed |
| `host.os`, `.arch`, `.cpu_features[]` | R | string | Float-reproducibility context (for example `avx2`, `fma`) |
| `execution.threads` | R | int | Effective worker count |
| `execution.deterministic` | R | bool | Fixed-order reductions enabled |
| `execution.stages[]` | R | {stage, duration_ms, peak_rss_mb?, alloc_peak_mb?, items, items_per_s} | Per-stage profiling (principle 5) |
| `species[]` | R | {scientific_name, taxon_key?, backbone?, backbone_version?} | Target taxa |
| `study_area` | R | {input_ref, crs, bbox[4]} | Reference to a study-area input; bbox in the study-area CRS |
| `grid` | R | {crs (EPSG or WKT2), resolution[2], width, height, origin[2]} | Modelling grid |
| `inputs[]` | R | object (below) | Every dataset used |
| `inputs[].id` | R | string | Local id referenced by steps |
| `inputs[].role` | R | enum | `occurrences` \| `background` \| `predictor` \| `study_area` \| `mask` \| `bias_layer` \| `model` |
| `inputs[].source` | R | {type, uri?, doi?, query?, retrieved_at?} | `type`: `gbif_download` \| `file_import` \| `catalogue` \| `builtin_demo` \| `run_output` |
| `inputs[].title`, `.version` | O | string | For example `WorldClim 2.1 bio1 30s` |
| `inputs[].licence` | R | SPDX id or URL or `unknown` | Licence propagation to outputs |
| `inputs[].citation` | O | string | Required attribution |
| `inputs[].synthetic` | R | bool | True for the demo virtual species |
| `inputs[].media_type`, `.size_bytes`, `.sha256` | R | — | Integrity |
| `inputs[].crs`, `.n_records`, `.band` | O | — | Descriptive |
| `preprocessing[]` | R | {step, params{}, inputs[], outputs[]} | Ordered steps, for example `dedupe_cells`, `spatial_thin`, `coordinate_clean`, `background_sample`, `extract_values`, `collinearity_filter` |
| `model.algorithm` | R | enum | `maxent` (v1) |
| `model.implementation`, `.implementation_version` | R | string | `niche-maxent`, SemVer |
| `model.reference` | O | {name, version, sha256, equivalence_suite} | For example maxent.jar 3.4.4 plus golden suite id |
| `model.config` | R | object | Fully resolved: `feature_classes[]`, `auto_features`, `regularization_multiplier`, `beta_overrides{}`, `max_iterations`, `convergence_threshold`, `n_background`, `add_samples_to_background`, `clamp`, `output_transform` (`cloglog`/`logistic`/`raw`), `categorical_predictors[]` |
| `model.config_sha256` | R | hash | Hash of canonical `model.config` (equality key for comparisons) |
| `randomness.master_seed` | R | u64 | The single user-visible seed |
| `randomness.prng` | R | string | For example `ChaCha8Rng (rand_chacha x.y)` |
| `randomness.derivation` | R | string | How stage seeds derive from the master seed, for example `"sha256(master‖stage‖species_key)[..8]"` |
| `randomness.stage_seeds{}` | R | map stage→u64 | Explicit for audit |
| `validation.strategy` | R | enum | `none` \| `random_kfold` \| `spatial_block` \| `checkerboard` \| `independent_test` \| `jackknife` |
| `validation.params` | R | object | `k`, `block_size_m`, `test_input_ref`, … |
| `validation.metrics[]` | R | {name, value, partition?, threshold_rule?} | For example `auc_test`, `cbi`, `omission_rate@10p` |
| `validation.thresholds[]` | O | {rule, value} | For example `maximum_training_sensitivity_plus_specificity` |
| `outputs[]` | R | object | Every artifact |
| `outputs[].path`, `.role` | R | string, enum | `role`: `model` \| `suitability` \| `binary` \| `uncertainty` \| `mess` \| `metrics` \| `response_curves` \| `report` \| `stac_item` |
| `outputs[].media_type`, `.size_bytes`, `.sha256` | R | — | Integrity |
| `outputs[].format_profile` | O | string | For example `ogc-cog-1.0`, `geojson-rfc7946`, `stac-item-1.1.0` |
| `outputs[].licence` | R | SPDX/URL | Derived from inputs (most restrictive or `see-inputs`) |
| `warnings[]` | R | {code, message, stage} | For example low sample size, extrapolation |
| `error` | O | {code, message, stage} | Present if `status != succeeded` |
| `integrity.manifest_sha256` | R | hash | Over canonical JSON without `integrity` |

## Example (abridged, demo run)

```json
{
  "schema": "nichers.run-manifest/1",
  "run_id": "0192f3a8-7c1e-7b2a-9d4e-5f6a7b8c9d0e",
  "kind": "pipeline",
  "status": "succeeded",
  "created_at": "2026-10-15T18:02:11Z",
  "started_at": "2026-10-15T18:02:11Z",
  "finished_at": "2026-10-15T18:02:39Z",
  "parent_runs": [],
  "software": {
    "name": "nichers",
    "version": "0.1.0",
    "git_commit": "3f2c9a1",
    "git_dirty": false,
    "target_triple": "x86_64-unknown-linux-gnu",
    "rustc": "1.xx.0",
    "profile": "release",
    "cargo_lock_sha256": "sha256:9b1d…",
    "frontend_lock_sha256": "sha256:44e0…",
    "components": [{"name": "niche-maxent", "version": "0.1.0"}, {"name": "niche-eval", "version": "0.1.0"}],
    "golden_suite_version": "g1"
  },
  "host": {"os": "linux", "arch": "x86_64", "cpu_features": ["avx2", "fma"]},
  "execution": {
    "threads": 8,
    "deterministic": true,
    "stages": [
      {"stage": "ingest", "duration_ms": 410, "peak_rss_mb": 96, "items": 300, "items_per_s": 731.7},
      {"stage": "features", "duration_ms": 220, "items": 10300, "items_per_s": 46818.2},
      {"stage": "train", "duration_ms": 6120, "items": 10300, "items_per_s": 1683.0},
      {"stage": "validate", "duration_ms": 15400, "items": 5, "items_per_s": 0.32},
      {"stage": "predict", "duration_ms": 3900, "items": 1440000, "items_per_s": 369230.8},
      {"stage": "raster_write", "duration_ms": 1200, "items": 1440000, "items_per_s": 1200000.0},
      {"stage": "export", "duration_ms": 300, "items": 6, "items_per_s": 20.0}
    ]
  },
  "species": [{"scientific_name": "Virtualis demonstrata (SYNTHETIC)", "taxon_key": null}],
  "study_area": {"input_ref": "area", "crs": "EPSG:2154", "bbox": [100000, 6050000, 1250000, 7120000]},
  "grid": {"crs": "EPSG:2154", "resolution": [1000, 1000], "width": 1150, "height": 1070, "origin": [100000, 7120000]},
  "inputs": [
    {"id": "occ", "role": "occurrences", "source": {"type": "builtin_demo", "uri": "builtin:demo/virtual-species-v1"},
     "licence": "CC0-1.0", "synthetic": true, "media_type": "application/geo+json",
     "size_bytes": 28412, "sha256": "sha256:aa01…", "n_records": 300},
    {"id": "bio_temp", "role": "predictor", "source": {"type": "builtin_demo", "uri": "builtin:demo/landscape-v1#temp"},
     "licence": "CC0-1.0", "synthetic": true, "media_type": "image/tiff; application=geotiff; profile=cloud-optimized",
     "size_bytes": 2310044, "sha256": "sha256:bb02…", "crs": "EPSG:2154"},
    {"id": "area", "role": "study_area", "source": {"type": "builtin_demo", "uri": "builtin:demo/area-v1"},
     "licence": "CC0-1.0", "synthetic": true, "media_type": "application/geo+json", "size_bytes": 1830, "sha256": "sha256:cc03…"}
  ],
  "preprocessing": [
    {"step": "dedupe_cells", "params": {}, "inputs": ["occ"], "outputs": ["occ_d"]},
    {"step": "background_sample", "params": {"n": 10000, "method": "uniform_in_area"}, "inputs": ["area"], "outputs": ["bg"]},
    {"step": "extract_values", "params": {"predictors": ["bio_temp"]}, "inputs": ["occ_d", "bg", "bio_temp"], "outputs": ["swd"]}
  ],
  "model": {
    "algorithm": "maxent",
    "implementation": "niche-maxent",
    "implementation_version": "0.1.0",
    "reference": {"name": "maxent.jar", "version": "3.4.4",
                  "sha256": "sha256:4c856e55412f70c5597b03cf9aaaf27e0782e0921f937262273b68bdcb8fee5e",
                  "equivalence_suite": "g1"},
    "config": {"feature_classes": ["linear", "quadratic", "hinge"], "auto_features": true,
               "regularization_multiplier": 1.0, "beta_overrides": {}, "max_iterations": 500,
               "convergence_threshold": 1e-5, "n_background": 10000, "add_samples_to_background": true,
               "clamp": true, "output_transform": "cloglog", "categorical_predictors": []},
    "config_sha256": "sha256:dd04…"
  },
  "randomness": {
    "master_seed": 42,
    "prng": "ChaCha8Rng",
    "derivation": "first 8 bytes of sha256(master_seed || stage || species_index), little-endian",
    "stage_seeds": {"background_sample": 1311768467463790320, "partition": 8312009923410023911}
  },
  "validation": {
    "strategy": "spatial_block",
    "params": {"k": 5, "block_size_m": 50000},
    "metrics": [{"name": "auc_test", "value": 0.83, "partition": "mean"},
                {"name": "omission_rate", "value": 0.11, "threshold_rule": "10th_percentile_training_presence"}],
    "thresholds": [{"rule": "10th_percentile_training_presence", "value": 0.27}]
  },
  "outputs": [
    {"path": "suitability.tif", "role": "suitability", "media_type": "image/tiff; application=geotiff; profile=cloud-optimized",
     "format_profile": "ogc-cog-1.0", "size_bytes": 3120044, "sha256": "sha256:ee05…", "licence": "CC0-1.0"},
    {"path": "suitability.stac.json", "role": "stac_item", "media_type": "application/geo+json",
     "format_profile": "stac-item-1.1.0", "size_bytes": 2210, "sha256": "sha256:ff06…", "licence": "CC0-1.0"},
    {"path": "model.json", "role": "model", "media_type": "application/json", "size_bytes": 5120, "sha256": "sha256:0a07…", "licence": "CC0-1.0"},
    {"path": "metrics.json", "role": "metrics", "media_type": "application/json", "size_bytes": 900, "sha256": "sha256:1b08…", "licence": "CC0-1.0"}
  ],
  "warnings": [{"code": "SYNTHETIC_DATA", "message": "Demo run on synthetic virtual species; not real biodiversity data.", "stage": "ingest"}],
  "integrity": {"manifest_sha256": "sha256:2c09…"}
}
```

The metric values, sizes and timings in the example are **illustrative placeholders**, not results.

## Implemented manifest (`manifest_version: 1`) — evaluation options (ADR-0014)

The manifest written today by `niche-app` (`engine.rs::write_manifest`) is a subset of the schema
above. Science v1 phase 3 adds, only when the options are used (a default study's manifest keeps
its previous keys):

| Path | Content |
|---|---|
| `spec.validation.method` | `spatial_block` \| `spatial_grid` \| `random_kfold` |
| `spec.validation.block_km` | user block size (km); absent = automatic |
| `spec.validation.null_models` | number of null replicates; absent = 0 (off) |
| `spec.validation.aoa` | `true` when the area of applicability was requested |
| `methods.spatial_grid`, `methods.null_models`, `methods.area_of_applicability` | method text with references; `methods.evidence_rules` = `"ADR-0009, ADR-0014"` |
| `species[].validation.blocks` | block size used, `chosen_by`, `k_requested`, `k_achieved`, autocorrelation analysis (median, clamp band, per-variable ranges) |
| `species[].validation.null_model` | `requested`, `completed`, `presences`, `sampling`, and for `cbi` and `or10`: observed, null mean, SD, z, empirical p, n |
| `species[].area_of_applicability` | `threshold`, `fraction_outside`, `n_train`, predictor weights |
| `species[].artifacts["aoa.tif"]` | checksum and size of the dissimilarity-index raster |

Null replicates are seeded from `random_seed`, the species key and the replicate index, so they are
reproducible from the manifest alone.

## Implemented manifest — bias and cleaning (ADR-0011, ADR-0013)

Science v1 phase 2 adds:

| Path | Content |
|---|---|
| `spec.filters.thin_km` | distance-thinning distance (km); absent = off |
| `spec.model.background_method`, `spec.model.target_group` | `target_group` and its GBIF class; absent = random background / the species' own class |
| `methods.cleaning` | cleaning rules: ADR-0009 §1 with the generalised-coordinate rule (ADR-0011); always present |
| `methods.background` | background method of the spec (random, or target group with its class); always present |
| `methods.thinning` | thinning method and distance, only when used |
| `occurrence_source.download_format` | `SIMPLE_CSV` (downloads and the regional mirror) or `search API (JSON)` (real data only) |
| `occurrence_source.generalisation_flags_available` | whether every species' records reported Darwin Core generalisation fields (search API: yes; downloads/mirror: no) |
| `species[].generalisation_flags_available`, `generalised_records`, `generalised_excluded`, `generalised_retained` | per species counts of records flagged as generalised by their publisher, excluded by the `generalised` rule and kept as presences (absent for synthetic data) |
| `species[].background` | method actually used (`target_group`, or `random` after a fallback with `fallback_reason`), class, eligible cells, `synthetic` |
| `species[].sampling_bias` | sampling-bias diagnostic: target group, records, cells with records, share of the area, largest count, share of presences in the most-surveyed ≈10 % and the expected share |
| `species[].artifacts["effort.tif"]` | checksum and size of the target-group effort raster (records per cell) |

Thinning is seeded from `random_seed` and the species key; the target-group background from
`random_seed` (the same draw as the random background when it falls back), so both are
reproducible from the manifest and the mirror snapshot it names.

## Implemented manifest — additional predictors (ADR-0015)

Science v1 phase 4 adds terrain, land-cover and distance-to-water predictors. A climate-only
study keeps exactly the previous `environment` block (`source`, `licence`, `citation` of CHELSA,
`layers[]` with `id`, `url`, `window`, and `land_mask`). When any additional predictor is
selected:

| Path | Content |
|---|---|
| `environment.layers[]` (additional predictors) | `id`, `source`, `vintage`, `licence`, `licence_url`, `citation`, `attribution`, `method`, `window` (same native CHELSA window as the climate layers) |
| `environment.sources[]` | one entry per source used (CHELSA, Copernicus DEM GLO-90, ESA WorldCover 2021): `source`, `vintage`, `licence`, `licence_url`, `citation`, `attribution` (null for CC0), `layers` (short names), `methods` |
| `environment.aggregation` | same native window for every layer, aggregation factor, mean of valid sub-cells, "a cell is used only where every layer has a value" |
| `environment.source`/`licence`/`citation` | present only when at least one CHELSA layer is selected |
| `methods.predictors` | aggregation methods of the DEM (mean heights, Horn slope at 90 m before aggregation, SD), WorldCover fractions from 10 m pixels, distance to water truncated at 25 km |
| `species[].…` warnings in `result.json` | `land_cover_temporal`, `built_up_bias`, `land_cover_composition`, `surface_model` when the predictor concerned is kept by the collinearity filter |

Source tiles are cached per instance (`dem/agg-v1/`, `worldcover/agg-v1/`, format version in each
file; `docs/architecture/predictors-v1.md`); their checksums are not yet recorded in the manifest
(not done in phase 4).

## Implemented manifest — second engine (ADR-0017)

Science v1 phase 6 adds the down-sampled random forest and the MaxEnt + RF ensemble as opt-in
algorithms. A MaxEnt study (the default) keeps exactly the previous manifest: `model.algorithm`,
`rf_trees` and `rf_tune_mtry` are omitted from the spec JSON when MaxEnt / default / off (same
`spec_sha256`), and no new `methods` key is written. With `rf` or `ensemble`:

| Path | Content |
|---|---|
| `spec.model.algorithm` | `rf` or `ensemble` |
| `spec.model.rf_trees`, `spec.model.rf_tune_mtry` | trees per forest (absent = 500) and whether `mtry = floor(p/2)` was also evaluated (absent = no) |
| `software.engine` | `niche-rf (down-sampled random forest)` or `niche-maxent (MaxEnt 3.4.4 port) + niche-rf (down-sampled random forest)` |
| `methods.algorithm`, `methods.selection`, `methods.uncertainty` | algorithm in words; selection among mtry candidates (`rf`); SD across the fold ensembles (`ensemble`) |
| `methods.random_forest` | `engine` (niche-rf and its format version), `trees`, `per_tree_sample`, `tree`, `mtry` rule, `output` semantics, `seed` derivation, `importance`, `response` (partial dependence), `area_of_applicability_weights`, `checkpoints` |
| `methods.ensemble` | ensemble only: member tuning, rank combination, fold-wise evaluation of the ensemble, thresholds on the ensemble output |
| `species[].selected.algorithm`, `rf_trees`, `rf_mtry`, `rf_seed` | the algorithm, forest size, selected `mtry` and the per-species forest seed (`random_seed` XOR species key × 0x9E3779B9 XOR a fixed RF stream, modulo 2^53 so that JSON readers using doubles read it exactly; tree *i* uses a stream derived from this seed and *i*) |
| `species[].validation.members` | ensemble only: each member's mean Boyce, OR10 and AUC on the same folds |
| `species[].artifacts["model.rf.json"]`, `["partial_dependence.csv"]` | checksum and size of the final forest (niche-rf format version 1, JSON) and of its partial-dependence curves; `model.lambdas` only when a MaxEnt model exists (MaxEnt, or the ensemble's MaxEnt member) |

The forest, its permutation importance (seed stream `PERMSEED`) and partial dependence (stream
`PDSEED!!`) are deterministic functions of the inputs and `rf_seed`, so a study is reproducible
from its manifest; trees are fitted in parallel without changing the result.

## Implemented manifest — future-climate projections (ADR-0016)

Science v1 phase 5 adds opt-in projections of the final model(s) to CHELSA v2.1 CMIP6 future
climatologies. Without projections nothing changes (`projections` omitted from the spec JSON, same
`spec_sha256`, no new key). With projections:

| Path | Content |
|---|---|
| `spec.projections[]` | `period`, `ssp`, `gcms[]` of each scenario |
| `methods.projections` | method in one sentence (climate predictors replaced per GCM on the same window and aggregation, others at present values, GCM mean and SD, change classes at the present P10 with GCM agreement, future MESS, exact areas, no dispersal) |
| `future_climate.scenarios` | the scenarios of the spec |
| `future_climate.source`, `licence`, `citation` | `chelsa::FUTURE_SOURCE`, `FUTURE_LICENCE`, `FUTURE_CITATION` (Karger et al. 2017, 2021; Eyring et al. 2016 CMIP6; Lange 2019 ISIMIP3b; WCRP acknowledgement and the five models). Demo: the synthetic source, empty citation |
| `future_climate.files` | sorted, de-duplicated URL of every CHELSA CMIP6 file read (each climate predictor kept by any species' model × GCM × scenario); empty for the demo |
| `future_climate.static_predictors`, `threshold`, `combination`, `extrapolation`, `area`, `ensemble_ranks`, `dispersal` | the rules applied, in words |
| `species[].projections` | each species' `ProjectionResult` (areas, agreement, novel fraction, per GCM) |
| `species[].artifacts["future_*.tif"]`, `["future_areas.csv"]` | checksums and sizes of the projection outputs |

Future windows are cached like the 1981–2010 ones (`chelsa-future/<period>/<GCM>/<ssp>/<id>/`);
their checksums are not recorded (as for the present windows).

## Implemented manifest — the organisation's own data and independent validation (ADR-0108)

Science v1 phase 7. Present only when a species uses the organisation's dataset; GBIF-only
manifests are unchanged. **No coordinates** are written: the dataset is identified by checksums,
and the records live in the study's copy (`jobs/<id>/input/own-records.json`, deleted with the
study).

| Path | Content |
|---|---|
| `spec.own_dataset` | `{id, records_sha256}`; `spec.species[].own` = `{values, role, gbif}` (taxon values of the file, `training` or `validation`, GBIF also used) |
| `own_data` | `source: "file_import"`, `owner: "organisation"`, `dataset_id`, `name`, `created_at`, `file_name`, `file_sha256` (computed in the browser), `records_sha256` (normalised records), `crs`, `transform` (text), `mapping` (field → column of the file), `absent_values`, `rows_read`, `rows_imported`, `rejected[] {reason, count}`, `presences`, `absences`, `synthetic`, `licence` |
| `occurrence_source` | unchanged, except `{"source": "none: the organisation's own data only (see own_data)"}` when no species uses GBIF records |
| `methods.own_data` | cleaning of the organisation's records (ADR-0009 §1 + absence rule + licence `ORG_OWNED` + order of sources) |
| `methods.independent_validation` | sites, leakage rule, metrics, thresholds of the training data, calibration as a diagnostic, thresholds of the *Validated* level |
| `methods.evidence_rules` | `…, ADR-0108` |
| `species[].own_data` | `taxon_values`, `role`, `gbif_records_used`, `by_source[]` (records, presence cells kept, records per status code, GBIF records in own cells), `independent_validation` (site counts, exclusions, leakage, AUC, thresholds with sensitivity/specificity/TSS/kappa and confusion counts, Brier, prevalence, calibration table, spread in km, note, `validated`) |

Reproducibility: while the study exists its copy replays the run exactly; afterwards the client's
own file (checked by `file_sha256`) with the recorded mapping, CRS and NicheRS version reproduces
the same normalisation (`records_sha256`).

## Analysis manifest addendum — decision-support outputs (ADR-0018)

Science v1 phase 8. Analyses of a finished study (survey priority, project footprint,
multi-species aggregation, comparison of two studies) are computed later and on demand, so they
cannot be listed in the study's `manifest.json` without rewriting it. Instead **the study manifest
stays immutable** and each analysis carries its own small manifest that points at it by
checksum:

```
jobs/<job_id>/
  manifest.json                  # study manifest, never modified after the run
  <species_key>/…                # study outputs, never modified
  analyses/<analysis_id>/
    analysis.json                # the API object (GET /jobs/{id}/analyses/{aid})
    manifest.json                # addendum below
    report.html                  # printable page
    *.tif | *.csv | *.geojson    # outputs of the analysis
```

`analysis_id`: `priority-<species_key>`, `richness-<min level>`, `comparison-<other job id>-<species_key>`
(deterministic: computed once), `footprint-<uuid v7>`. The directory is written under a temporary
name and renamed, so it appears complete or not at all; it is deleted with the study.

| Field | Content |
|---|---|
| `format` | `"nichers-analysis-manifest/1"` |
| `analysis_id`, `kind`, `created_at` | identity; `kind` = `priority` \| `footprint` \| `richness` \| `comparison` |
| `software` | `{name: "NicheRS", version}` |
| `decision` | `"ADR-0018 (Proposed)"` |
| `study` | `{job_id, manifest_sha256, spec_sha256}`: SHA-256 of the study's `manifest.json` bytes and the `spec_sha256` it records |
| `other_study` | comparison only: the same three fields for study B (`null` otherwise) |
| `synthetic_data` | the study (or the compared study) is the synthetic demo |
| `request` | the API request; for a footprint, `geometry` is replaced by `{file: "footprint.geojson", sha256}` |
| `method` | parameters of the method (score formula, percentile definition, splits, threshold rules, hotspot share, `top_n`, minimum evidence level…) |
| `inputs[]` | every study file read: `{job_id, species_key, file, sha256}` (`result.json`, `suitability.tif`, `mess.tif`, `uncertainty.tif`, `aoa.tif`, `effort.tif`, `occurrences.geojson` as applicable) |
| `outputs` | `{name: {sha256, bytes, media_type}}` for every file of the analysis, `analysis.json` included |

Reproducibility: the inputs' checksums tie the analysis to the exact study outputs; the same
NicheRS version recomputes identical rasters and tables from them (the methods are deterministic;
`report.html` differs only in its generation time). A comparison keeps study B's checksums even
after B is deleted.

## Mapping to standards (for later export)

| Manifest | RO-Crate 1.3 / Process Run Crate [E4, E5] | ODMAP v1.0 [E15] | PROV-O [E16] |
|---|---|---|---|
| run | `CreateAction` (`startTime`, `endTime`, `actionStatus`) | Overview | `prov:Activity` |
| software | `SoftwareApplication` (`instrument`) | Model: software | `prov:Agent` (`wasAssociatedWith`) |
| inputs[] | `File`/`Dataset` in `object` | Data | `prov:used` |
| outputs[] | `File` in `result` | Prediction | `prov:wasGeneratedBy` |
| model.config, randomness | `PropertyValue` parameters | Model: settings | activity attributes |
| validation | — (custom `PropertyValue`) | Assessment | — |
| parent_runs | — | — | `prov:wasDerivedFrom` |

[REC] RO-Crate export is a later milestone (ADR-0203). It writes `ro-crate-metadata.json` from this manifest. The manifest stays the source of truth.

## Evidence and references

- [E4] Workflow Run RO-Crate / Leo et al. 2024, doi:10.1371/journal.pone.0309210
- [E5] RO-Crate 1.3 spec
- [E15] Zurell et al. 2020 ODMAP, doi:10.1111/ecog.04960
- [E16] W3C PROV-O (2013)
- [E21] crates.io (`sha2` 0.11.1, `schemars` 1.2.3, `jsonschema` 0.58.6)
- [E23] maxent.jar 3.4.4 hash (local)

Full URLs are in `engineering-principles.md` and `docs/research/refs/eng.md`.

## Alternatives

- Emit RO-Crate JSON-LD directly. Rejected for v1: see ADR-0203.
- Store provenance only in SQLite. Rejected: it does not travel with exported files.
- Use BLAKE3 hashes. Deferred; it may be added as an extra field if hashing becomes a measured bottleneck.

## Trade-offs

- Hashing large predictor stacks costs I/O time. Mitigation: content-addressed cache, where a file is hashed once and keyed by (path, size, mtime).
- Recording `cpu_features` and thread count exposes reproducibility limits honestly, at the cost of more fields.

## Risks

- Schema churn early in development. Mitigation: keep v1 minimal and add optional fields.
- Licence propagation for outputs is non-trivial (CC BY-NC inputs). v1 records input licences; legal interpretation is [OPEN].

## Recommendations

1. Implement the Rust types in `niche-core::manifest` (pure data plus serde) and assembly in `niche-app`.
2. Generate and commit the JSON Schema; validate every manifest in tests.
3. Ship `niche rerun` and `niche models verify` in the MVP.

## Open questions

- [OPEN] Cross-target bit-reproducibility vs tolerance-level guarantee wording.
- [OPEN] Output licence derivation rules.
- [OPEN] Whether a `pipeline` run nests per-species sub-manifests (hundreds of species) or uses one manifest per species plus an index. Leaning: one per species plus an index file.
