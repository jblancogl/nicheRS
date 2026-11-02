# Data pipeline — provenance-aware ingestion and processing

Status: proposed design · Date: 2026-10-15 · Basis: `docs/research/06-biodiversity-data.md`,
`docs/research/12-security-and-licensing.md`. Tags as in the research brief.

## 1. Goals

1. Every number in a NicheRS output is traceable to: source request, provider version, DOI,
   licence, filters and exclusions, software version, random seeds.
2. Re-running a run from its manifest on another machine yields identical inputs (byte-identical
   artefacts where the provider allows; otherwise an explicit "provider drift" report).
3. Licence and sensitivity constraints are data, not documentation: they propagate to outputs
   and are enforced at export.
4. Caches are content-addressed, shareable between projects of one user, and safe to delete.

## 2. Stages

```
S0 Project spec ──► S1 Taxon resolution ──► S2 Occurrence acquisition ──► S3 Normalisation
     │                                                                        │
     │                                                       S4 Cleaning & filtering (reason codes)
     │                                                                        │
     │                                                       S5 Licence & sensitivity gate
     │                                                                        │
     └──► S6 Predictor acquisition ──► S7 Harmonisation (grid) ──► S8 Thinning & extraction
                                                                              │
                                                       S9 Sufficiency gate ──► Model input bundle
```

| Stage | Input | Output artefact | Key decisions recorded |
|---|---|---|---|
| S0 Project spec | User UI/API | `project.json` | study area (geometry hash), CRS, target resolution, time window, `commercial` flag, sensitivity policy |
| S1 Taxon resolution | names | `taxa.parquet` | TAXREF version (e.g. 18.0) and GBIF `checklistKey` (COL XR `7ddf754f-…` or legacy backbone); `cd_nom`, `cd_ref`, GBIF/COL id, match type/confidence; synonyms included [FACT GBIF COL XR switch, doc 06 [5]] |
| S2 Occurrence acquisition | taxa + area + window | raw blobs (DwC-A zip, Parquet, CSV, GeoNature export, user upload) | provider, canonical request, `query_hash`, DOI, download key, licence, `retrieved_at` |
| S3 Normalisation | raw blob | `occ_raw.parquet` (GeoParquet) | field mapping version; original IDs kept; `recordedBy` dropped by default (doc 12) |
| S4 Cleaning | occ_raw | `occ_clean.parquet` + `exclusions.parquet` | ordered filter list with params; each excluded record → reason code |
| S5 Licence & sensitivity gate | occ_clean | `occ_usable.parquet` | NC exclusion if commercial; generalised/sensitive handling; `commercial_ok`, `export_points_ok` flags |
| S6 Predictor acquisition | dataset ids + bbox | COG tiles in cache | dataset version, URL, checksum, licence, attribution |
| S7 Harmonisation | COGs | `predictors.vrt`/stacked COG on project grid | resampling method per layer (bilinear for continuous, mode/fraction for categorical), CRS (EPSG:3035 default for EU [ASSUMPTION]), nodata, aggregation from 10 m to grid |
| S8 Thinning & extraction | occ_usable + grid | `presence.parquet`, `background.parquet` | thinning method + seed; background strategy (random / target-group) + seed |
| S9 Sufficiency gate | presence | gate result | effective n vs thresholds (van Proosdij et al. 2016; doc 06 [23]); block/warn |

All stages are pure functions of (inputs' content hashes, parameters, tool version) → stage
output key = `blake3(stage_name ‖ stage_schema_version ‖ canonical_params ‖ input_hashes)`.
[REC] This makes the pipeline memoisable and makes "what changed" diffable.

## 3. Reason codes (S4/S5)

| Code | Meaning |
|---|---|
| `NO_COORDS` | missing/invalid coordinates |
| `COORD_ZERO`, `COORD_SWAPPED`, `OUT_OF_RANGE` | lat/lon sanity |
| `OUTSIDE_AREA` | outside study area (after buffer) |
| `UNCERTAINTY_GT` | `coordinateUncertaintyInMeters` > threshold |
| `UNCERTAINTY_NULL` | null uncertainty (only if policy excludes) |
| `GENERALISED` | `dataGeneralizations` or SINP level ≥ 1 coarser than grid |
| `CENTROID_COUNTRY`, `CENTROID_ADMIN`, `CAPITAL`, `INSTITUTION`, `SEA` | CoordinateCleaner-style tests (Zizka et al. 2019) |
| `GBIF_ISSUE:<flag>` | excluded GBIF issue flags (configurable list) |
| `BASIS_EXCLUDED` | e.g. FOSSIL_SPECIMEN, LIVING_SPECIMEN |
| `ABSENCE` | `occurrenceStatus = ABSENT` |
| `DATE_OUTSIDE_WINDOW`, `DATE_MISSING` | temporal filter vs climate baseline |
| `TAXON_UNRESOLVED`, `TAXON_EXCLUDED` | taxonomy |
| `DUPLICATE` | same taxon/cell/date or cross-provider ID match |
| `LICENCE_NC` | CC BY-NC in commercial project |
| `THINNED` | removed by thinning (S8, separate table) |
| `NODATA_PREDICTOR` | falls on nodata in any predictor (S8) |

## 4. Manifests

### 4.1 Source manifest (`manifest.source.v1`)

```jsonc
{
  "schema": "nichers.manifest.source/1",
  "source_id": "gbif:download:0000005-200127171203522",
  "provider": "GBIF",
  "endpoint": "https://api.gbif.org/v1/occurrence/download/request",
  "request": { "format": "SIMPLE_PARQUET", "checklistKey": "7ddf754f-...", "predicate": { } },
  "query_hash": "sha256:…",          // of RFC 8785 canonical JSON of request
  "provider_version": { "taxonomy": "COL XR 2025-10-07", "taxref": "18.0" },
  "doi": "10.15468/dl.xxxxxx",
  "licence": { "aggregate": "CC-BY-NC-4.0", "per_record_field": "license" },
  "attribution": "GBIF.org (2026-10-15) GBIF Occurrence Download https://doi.org/…",
  "retrieved_at": "2026-10-15T09:12:44Z",
  "content": { "sha256": "…", "bytes": 123456, "media_type": "application/zip" },
  "record_count": 48211,
  "account_ref": "gbif-user-id-only",   // never credentials
  "tool": { "name": "nichers", "version": "0.1.0", "git": "abc123" }
}
```

DOI and aggregate licence for GBIF downloads are returned by the download metadata endpoint
[FACT, doc 06 [1]]. Search-API sources have `doi: null` and `citation_action: "register_derived_dataset"`
(GBIF derived datasets [FACT, doc 06 [6]]).

### 4.2 Stage manifest (`manifest.stage.v1`)

Fields: `stage`, `stage_schema_version`, `inputs[]` (content hashes), `params` (canonical),
`filters_applied[]` (ordered `{code, params, excluded_count}`), `seed`, `outputs[]`
(path, sha256, rows), `started_at`, `duration_ms`, `warnings[]`, `tool`.

### 4.3 Run lock (`run.lock.json`)

Aggregates all source + stage manifests of a model run, plus: derived `commercial_ok`
(AND over sources), `export_points_ok`, attribution block (deduplicated), DOIs list, platform
info (OS, CPU features used, GDAL/PROJ versions, PROJ grid availability). Exports (maps,
reports, ZIP bundles) embed `run.lock.json`. [REC] Re-run = `nichers replay run.lock.json`.

## 5. Cache layout

```
$NICHERS_HOME/                      # default: OS data dir; configurable
  cache/
    blobs/blake3/ab/cd/<hash>       # immutable raw downloads & stage outputs (content-addressed)
    blobs/.tmp/                     # atomic write-then-rename staging
    index.sqlite                    # key → blob hash, source manifests, last access, licence flags
  datasets/
    chelsa-v2.1/<var>/<tile>.tif    # symlink/hardlink views onto blobs, COG
    copdem-glo30/...
  projects/<project-id>/
    project.json
    runs/<run-id>/
      run.lock.json
      manifests/*.json
      occ/*.parquet
      predictors/stack.vrt
      outputs/*.tif
```

Rules [REC]:
- Blobs immutable; written to `.tmp` then `rename` (atomic on same FS); hash verified on read.
- Restricted-licence blobs (`commercial_ok=false`, user-specific credentials) flagged in index;
  never included in shared/exported caches.
- Eviction by LRU on `last_access`, never evicting blobs referenced by a pinned run.
- Provider immutability: GBIF download DOIs and versioned COG URLs are cacheable forever; search
  API results and "latest" endpoints are cached with TTL and stored with `retrieved_at`.
- P2 (server): `cache/` per tenant for restricted/sensitive data; shared cache only for
  CC0/CC BY/Etalab public rasters.

## 6. Versioned formats

| Artefact | Format | Versioning |
|---|---|---|
| Occurrences (all stages) | GeoParquet (WKB point, EPSG:4326 + projected x/y columns) | `nichers.occ` schema version in Parquet key-value metadata |
| Exclusions | Parquet `(record_id, source_id, stage, reason_code, detail)` | same |
| Rasters | Cloud-Optimized GeoTIFF, internal tiling, explicit nodata, `NICHERS_*` GDAL metadata tags with source hash | per-file metadata |
| Predictor stack | GDAL VRT referencing COG blobs (generated, never user-supplied — doc 12 GDAL hardening) | VRT regenerated from manifest |
| Manifests | JSON, RFC 8785 canonicalisation for hashing, `schema` field `nichers.<kind>/<major>` | JSON Schema files in repo; breaking change → major bump + migrator |
| Index | SQLite with `PRAGMA user_version` migrations | integer |

[ASSUMPTION] Crate choice (geoparquet/arrow/parquet 60.0.0, gdal 0.19.0, rusqlite 0.40.2 —
versions per doc 12 [6]) to be confirmed by the I/O research doc.

## 7. Reproducibility levels

| Level | Guarantee | Sources |
|---|---|---|
| R1 exact | Byte-identical inputs refetchable | GBIF download DOI, versioned COGs (CHELSA 2.1, CLC 2018, WorldCover v200, COP-DEM) |
| R2 cached | Exact only from cache/export bundle | GBIF search API, OpenObs exports, GeoNature exports, user uploads |
| R3 drifting | Provider changes silently | "latest" endpoints; flagged in report |

[REC] Reports state the reproducibility level per source; model-ready bundles for R2 sources
include the normalised occurrence Parquet (subject to licence/sensitivity export flags).

## 8. Open questions

- [OPEN] Projected CRS default (EPSG:3035 LAEA Europe vs EPSG:2154 Lambert-93 for France-only).
- [OPEN] Whether to store per-record licence or only per-dataset licence (GBIF is per record).
- [OPEN] GeoNature connector auth model per instance.
- [OPEN] Policy for null coordinate uncertainty (keep vs exclude) as project default.
