# ADR-0108 — The organisation's own occurrence data and independent presence–absence validation

- Status: **Proposed — pending review by Natural Solutions ecologists** (sources, absence rule,
  leakage rule, *Validated* criteria, concentration threshold) and by the data-protection
  referent (the design notes).
- Date: 2026-10-20 (Science v1, phase 7)
- Related: ADR-0009 (cleaning and evidence levels; §6 reserves *Validated*), ADR-0011
  (generalised coordinates), ADR-0105/0106 (organisations, isolation, support access), ADR-0203
  (execution manifest), plan the design notes (the design this ADR decides).
- Implementation: `crates/niche-app/src/owndata.rs` (import, normalisation, storage, sources),
  `crates/niche-app/src/independent.rs` (validation sites, metrics, evidence, report sections),
  `engine.rs` (validation of the spec, copy at submission, preview, run, manifest), `store.rs`
  (schema v11, table `datasets`), `env.rs` (licence rule), `niche-data/src/gbif.rs`
  (`match_name`), `crates/niche-server/src/datasets.rs` (routes), `web/src/features/owndata/`.
- Tests: `crates/niche-app/tests/own_data.rs`, unit tests in `owndata.rs` and `independent.rs`,
  `crates/niche-server/tests/api_accounts.rs::datasets_stay_within_the_organisation`.

## Context

Consultancies hold their own field data (inventories, transects, protocol plots), often in
Lambert-93 and often with *absences* — sites surveyed where the species was not found. Until now
NicheRS modelled GBIF records only, and every result said "No independent validation data". ADR-0009
§6 reserved a *Validated* level for a future evaluation on independent presence–absence data.

## Decision

### 1. Import (v0: CSV)

- The file is read and parsed **in the browser** (UTF-8, else Windows-1252; separator `;`, `,` or
  tab; hand-written RFC 4180 parser, no new dependency). The user maps columns: species name,
  X, Y (required); date or year, coordinate precision (m), presence/absence status, protocol or
  survey id (optional). Only these columns are sent, as strings, with the SHA-256 of the original
  file computed in the browser (`POST /api/v1/datasets`, JSON within the 8 MB limit).
- **Closed list of coordinate systems**: EPSG:4326, 2154 (Lambert-93), 3035 (LAEA Europe),
  25828–25831 (ETRS89 / UTM 28N–31N), 32628–32632 (WGS 84 / UTM 28N–32N). Transforms are those of
  `niche_geo::crs` (checked against PROJ 9.8.1); RGF93/ETRS89 are taken as WGS 84 (< 1 m, PROJ's
  ballpark transformation). Each system has a rectangle of plausible coordinates; values outside
  are rejected (`outside_crs_domain`), which catches a wrong system choice.
- Normalisation on the server (`owndata::normalize`, pure): decimal comma or point, spaces as
  thousands separators; dates `YYYY-MM-DD`, ISO timestamps, `YYYY-MM`, `YYYY`, `DD/MM/YYYY`,
  `DD-MM-YYYY`, `DD.MM.YYYY` → year; status compared case-insensitively to the values the user
  marks as "absent". Import rejections (`missing_taxon`, `bad_coordinates`, `outside_crs_domain`,
  `bad_date`, `bad_uncertainty`) are reported with counts and the file lines of the first 10;
  they are separate from the scientific cleaning rules. An empty date is not a rejection (the
  record is "undated" for the cleaning rules). At most 200 000 rows per import.
- A dataset is **immutable**: changing the mapping means a new import (new id).

### 2. Storage (plan §5)

- Table `datasets` (schema v11): id, organisation (`NULL` in local mode), creator, creation time,
  description and import report as JSON (counts, never records). Records in
  `<data_dir>/datasets/<id>/records.json` (versioned format `nichers-own-records` v1, original
  and WGS84 coordinates); `records_sha256` = SHA-256 of that file.
- **Copy at submission** into `jobs/<id>/input/` (`own-records.json`, `dataset.json`): resume
  after an interruption and later display do not depend on the dataset surviving; the copy is
  deleted with the study. The spec carries `own_dataset: {id, records_sha256}`; a changed
  checksum is refused (409).
- Deletion by the organisation at any time (row and files; submitted studies keep their copy);
  retention: same period as studies (`NICHE_RETENTION_DAYS`, same sweep). At most 20 datasets per
  organisation.

### 3. Use in a study (spec, opt-in)

Per species, `SpeciesRef.own = {values, role, gbif}` (omitted by default, so GBIF-only
specifications serialize unchanged and keep their `spec_sha256`):

| UI choice | `role` | `gbif` | Training presences | Validation sites |
|---|---|---|---|---|
| GBIF only | — (`own` absent) | — | GBIF | none |
| GBIF + the organisation's presences | `training` | true | own presences first, then GBIF | own absences (see §5) |
| The organisation's presences only | `training` | false | own presences; no GBIF request | own absences |
| GBIF for training, own data held out | `validation` | true (required) | GBIF | all own records |

`values` are the values of the taxon column that are this species (several names may go to one
species). Validation of the spec: own data and dataset go together; values non-empty and present
in the dataset; held-out data need GBIF records for training; a species appears once per study.

### 4. Cleaning (ADR-0009 §1 extended)

- Own presences become occurrences with licence `ORG_OWNED`, `basis_of_record =
  HUMAN_OBSERVATION`, dataset `own:<id>`, and pass the **same rules in the same order**.
- **Licence rule**: `ORG_OWNED` is always accepted (the organisation guarantees its rights; no
  GBIF record carries this code).
- **Absence rule** (`absence`, first): an absence record is never a presence. It is reported
  as an exclusion (“Absence record (surveyed, not found): not used as a presence”).
- **Order of sources**: own presences first, then GBIF, each in file order. The one-record-per-cell
  rule therefore attributes a cell recorded by both to the organisation's data; the GBIF record is
  `duplicate_cell` and counted as "in own cells". No record-by-record deduplication between
  sources (plan §3.7).
- `own_data_concentrated` (Info): the bounding box of the own presence cells covers < 10 % of the
  study area's bounding box — "areas not surveyed are not absences". Threshold to be confirmed.

### 5. Independent presence–absence validation

Computed after the final prediction, from the final suitability raster; it never changes the
model.

- **Sites**: held-out data: every record of the species; training role: its absences only. Same
  rules as presences (dated, year, precision, study area, land, predictor data); no licence rule;
  no thinning. One site per grid cell; a cell with at least one presence record is a presence
  site (detection wins), else an absence site; cells with both outcomes are counted.
- **Leakage rule**: a validation record in a cell used as a training presence (GBIF or own) is
  **excluded** (`training_cell`), not merely flagged: the model already knows that outcome, and
  SINP observations republished in GBIF (via INPN) would otherwise validate themselves. Background
  cells are not excluded (background is not absence). With the training role, every own presence
  is a training presence, so no independent presence site remains and no metric is computed: the
  result says so and suggests holding the dataset out.
- **Metrics** (`niche_eval::pa`): presence–absence AUC; sensitivity, specificity, TSS and kappa at
  two thresholds **of the training data** — P10 and max SSS (Liu et al. 2013) — never optimised on
  the validation sites; Brier score and an equal-count calibration table (2–10 bins). Metrics need
  ≥ 10 presence and ≥ 10 absence sites. Spatial spread: extent (km), share of the study area's
  extent, mean nearest-neighbour distance between sites.
- **Calibration caveat**: MaxEnt cloglog (and the random forest of ADR-0017) give relative
  suitability, not calibrated probabilities. Brier and the calibration table are shown as
  diagnostics with that caveat; outputs are never relabelled as probabilities.
- References: Allouche, Tsoar & Kadmon 2006 (*J. Appl. Ecol.* 43:1223–1232); Leroy et al. 2018
  (*J. Biogeogr.* 45:1994–2002: TSS misleads without quality PA data and depends on prevalence when
  errors are unbalanced — report sensitivity and specificity with it); Liu, White & Newell 2013
  (*J. Biogeogr.* 40:778–789); Brier 1950 (*Mon. Weather Rev.* 78:1–3).

### 6. Evidence: the *Validated* level (ADR-0009 §6)

*Validated* is given only when **all** hold (proposed, conservative):

1. the data were held out from training (`role = validation`);
2. ≥ 30 presence **and** ≥ 30 absence sites after cleaning and the leakage rule;
3. presence–absence AUC ≥ 0.75;
4. TSS ≥ 0.40 at the training max-SSS threshold;
5. the level is *Supported* after every cap (pooled taxa, null models) — i.e. every
   cross-validation criterion of *Supported* holds.

It is applied after all caps and is the only rule that can raise a level. When independent data
were used, the reason "No independent validation data…" is replaced by a sentence giving the
dataset, site counts, AUC and TSS, followed by the five criteria (✓/✗); when they were provided
but unusable, the reason says why. Otherwise the reasons are unchanged.

### 7. Provenance and confidentiality

- Manifest: `own_data` (source `file_import`, owner organisation, dataset id and name, file name,
  `file_sha256`, `records_sha256`, CRS and transform text, column mapping, absent values, rows
  read/imported, rejections by reason, presences/absences, synthetic flag, licence), per species
  `own_data` (taxon values, role, whether GBIF was used, counts per source, the independent
  validation: counts and metrics), `methods.own_data` and `methods.independent_validation`. **No
  coordinates** (plan §11.1): reproducibility comes from the study's copy while it exists, then
  from the client's file (`file_sha256`) + mapping + CRS + NicheRS version.
- Reports (md/html): "Occurrence data sources" (sources, role, counts; no coordinates; "Points are
  not reproduced in this report") and "Independent validation (presence–absence)" (method text,
  sites, metrics tables, calibration with its caveat, Validated criteria, references).
- Isolation: datasets are scoped by organisation exactly like projects; another organisation (and
  local mode for an organisation's dataset) gets 404 on every dataset route, on the preview and on
  `POST /jobs`. Administrators' support access to studies (ADR-0106) does not extend to datasets;
  the console does not list them. The organisation's points are visible only in its study's
  map and `occurrences.geojson` (`source: "own"`), like GBIF points.
- Logs: identifiers and counts only (`dataset imported` with id and row counts); never a value
  of the file. Import errors are 4xx and never quote a cell. Known limit: the HTTP client logs a
  GBIF URL at debug level when it retries, and the name-matching URL contains the taxon name;
  the default log level (info) does not show it.
- Only names leave the server, to GBIF's name matcher (`POST /api/v1/species/match`, ≤ 200 names,
  called by the browser as a separate step), never coordinates or counts.

### 8. Points decided (`plan-donnees-terrain.md` §11, recommended options)

1. Manifest **without points** (proposed option).
2. Taxon matching **by GBIF** (names sent to GBIF), automatic proposal only for an exact match at
   species rank or below; fuzzy matches to check; higher ranks not modelled; the user assigns.
3. Taxa absent from the GBIF backbone: **out of v0** (no internal keys).
4. `own_data_concentrated`: Info at < 10 % of the study extent — **to be confirmed** by an
   ecologist.
5. Retention of datasets: **same as studies**.
6. Default source when a dataset is chosen: **both** (GBIF + the organisation's presences).
7. Presence–absence data as independent validation and *Validated*: **decided here** (§5–6),
   the one point where this ADR goes beyond the plan's v0, on the programme's request.
8. Sensitive species: unchanged, see `donnees-sensibles-rgpd.md` (the organisation's points are
   treated like GBIF points: exact in its own study only).

### 9. Deviations from `plan-donnees-terrain.md`

- Internal records file: versioned JSON (`records.json`) instead of a CSV with a fixed header
  (exact float round trip, no quoting rules); the job copy is the same file.
- Taxon matching is a separate route (`POST /species/match`) called by the browser after the
  import, so that the import itself works offline and the user sees that names are sent.
- Datasets live in the data step of the wizard (Expert path), not in the species step: matching
  assigns the dataset's names to the species of the study and can add a matched species.
- The CRS list adds UTM 28N–32N (ETRS89 and WGS 84), already supported by `niche_geo::crs`.
- The spec carries the use per species (`SpeciesRef.own`) rather than one study-wide source.

## Consequences

- GBIF-only studies are unchanged: no new key in their spec, same cleaning input, same outputs
  (`occurrences.geojson` has no `source` key, results have no `by_source`).
- `EvidenceLevel` gains `validated` (API and UI); clients that list levels must handle it.
- Cleaning gains one rule (`absence`) and one licence case, both unreachable for GBIF records.

## Not done (v0)

- XLSX, GeoPackage, zipped Shapefile, GeoNature connector; Lambert II étendu, overseas systems.
- CLI import (`niche run --occurrences`): the normalisation is pure and ready for it.
- Protocol/survey ids are stored and counted but not yet used (e.g. survey-level blocking of
  validation sites).
- Detection probability (imperfect detection of absences) is not modelled: an absence is "not
  found", not "absent for sure".
