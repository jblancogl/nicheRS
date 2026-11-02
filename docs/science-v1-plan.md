# Science v1 — programme plan

Date: 2026-10-20. Branch: `feat/science-v1`. Owner: Natural Solutions (decision "apply everything",
session of 2026-10-20).

## Why

The MVP answers "where are the conditions favourable to a species, and how much can we trust
the answer?" with presence-only GBIF data and climate alone. A study of the code on 2026-10-20
found the scientific core unchanged since the MVP, while the main weaknesses for consultancy use
are scientific (sampling bias, climate-only predictors, no independent validation) and in the
outputs (a map per species instead of an answer to the client's decision). This programme
addresses them.

## Rules for every phase

- **Defaults of ADR-0009 do not change.** Every new method is opt-in (Expert mode, API field with
  a default equal to today's behaviour), so results of existing specs are bit-identical. Each new
  ADR is *Proposed — pending review by Natural Solutions ecologists*; making a method the default
  is a later ADR after that review.
- The MaxEnt kernel (`niche-maxent`) is not modified; `cargo test` golden tests stay green and
  `scripts/compare_maxent.py` stays at 100 % PASS.
- Pure crates stay pure (no I/O, no Tokio/Rayon in `niche-maxent`, `niche-eval`, the new
  `niche-rf`); heavy work never on the async runtime.
- API contract changes in `crates/niche-app/src/dto.rs` and `web/src/api/types.ts` together;
  UI texts in EN/FR/ES; help articles updated; manifest records every new input and setting.
- Wording: relative suitability, never probability (except calibrated output of the
  presence–absence pathway, labelled as such); no confidence percentages; warnings are never hidden.
- New data sources only with a licence allowing commercial use, attribution in reports.
- Each phase ends with the Definition of Done of `AGENTS.md` and a commit.

## Phases

| # | Phase | Content | ADR |
|---|---|---|---|
| 1a | `niche-eval` additions | distance thinning, spatial grid blocks + autocorrelation range, area of applicability (AOA), null-model statistics, presence–absence metrics | — |
| 1b | `niche-rf` crate | down-sampled (balanced) random forest for presence–background data (second engine) | ADR-0017 |
| 1c | `niche-data` additions | GBIF generalisation flags, mirror sampling-effort grids by class, taxon class lookup, CHELSA CMIP6 layers | — |
| 1d | `niche-geo` / `niche-data` predictors | CRS transforms (EPSG:2154/3035/3857/UTM), Copernicus DEM (elevation, slope), ESA WorldCover fractional cover, distance to water | — |
| 2 | Bias and cleaning | generalised-coordinate rule, optional distance thinning, target-group background, sampling-effort layer and bias diagnostic | ADR-0011, ADR-0013 |
| 3 | Evaluation | null-model significance, spatial grid blocks sized by autocorrelation, AOA layer | ADR-0014 |
| 4 | Predictors | topography, land cover fractions, distance to water in the catalogue | ADR-0015 |
| 5 | Future climate | CHELSA CMIP6 projections, change maps, future extrapolation | ADR-0016 |
| 6 | Second engine | random forest and MaxEnt+RF ensemble as opt-in algorithms | ADR-0017 |
| 7 | Own field data | CSV import (presences and absences, projected CRSs), independent presence–absence validation, *Validated* level | ADR-0108 |
| 8 | Decision outputs | survey-priority layer, project-footprint overlay, multi-species richness/hotspots, comparison of studies | ADR-0018 |
| 9 | Documentation | MVP status, executive summary, roadmap, offer documents (help content updated in each phase) | — |

Progress and verification of each phase are recorded below as phases complete.

## Progress log

(filled in per phase: what was built, what was verified, what was not)

### Summary (phase 9, 2026-10-20)

Consolidated from the phase entries below and the ADRs; nothing here was re-measured in phase 9
(documentation only: `cargo test` and the UI checks were not re-run for this summary). Phases
were merged in the order 1a, 1b, 1c, 1d, 3, 2, 4, 7, 6, 5, 8; the entries below keep their
original order.

Every new ADR is **Proposed — pending review by Natural Solutions ecologists**. The default
specification is unchanged by every phase: the synthetic demo's outputs were checked
byte-identical (except generation time, `created_at`, `job_id` and the report checksum) and the
`spec_sha256` identical after phases 2, 3, 4, 5, 6, 7 and 8, and `crates/niche-maxent` was not
modified. The only behaviour that applies without opting in is the ADR-0011 cleaning rule, which
affects only GBIF **search-API** records flagged as generalised by their publisher (downloads and
the regional mirror carry no flag and get a warning instead).

| Phase | Status | ADR | Default | Main verification | Main open points |
|---|---|---|---|---|---|
| 1a `niche-eval` additions | Done (library) | — (`architecture/eval-methods-v1.md`) | not used by itself; wired by phases 2, 3, 7 | Pure functions, unit tests (k-d tree vs brute force, thinning distances, AOA, nulls, PA metrics); frozen `Rng` sequences unchanged | — |
| 1b `niche-rf` | Done (library) | ADR-0017 | not used by itself; wired by phase 6 | Unit tests; measured fit/prediction costs in ADR-0017 | see phase 6 |
| 1c `niche-data` additions | Done (data layer) | ADR-0114 "Format 2" (amendment) | format-1 mirror snapshots keep working | 855/855 CMIP6 URLs answer 200; 64 CMIP6 headers on the baseline grid; effort-grid build +15 % on a 2 M-record synthetic archive | `fr-es` effort-grid sizes are estimates; hosted mirror still format 1 until its next refresh; CMIP6 licence confirmation |
| 1d predictors (`niche-geo`, `niche-data`) | Done (library) | — (`architecture/predictors-v1.md`; ADR-0015 later) | not used by itself; wired by phase 4 | CRS transforms vs IOGP examples (0.01 m) and PROJ 9.8.1 (< 1 mm); live DEM and WorldCover reads (Provence) | DEM licence: lawyer's confirmation advisable |
| 2 Bias and cleaning | Done | ADR-0011, ADR-0013 | ADR-0011 rule **on** (search-API flagged records only); thinning and target-group background **off** | Demo bit-identical; live *Testudo hermanni* (search API): 339 records flagged, 108 change exclusion reason, presences/metrics/evidence unchanged | Target-group background never run on a real mirror with effort grids; no random vs target-group comparison on real species; no flags for downloads/mirror |
| 3 Evaluation | Done | ADR-0014 | off (null models, grid blocks, AOA) | Demo bit-identical; options leave the model byte-identical; resume of null models identical; nulls 2.8 s on the demo, AOA 0.05–0.23 s on 1 M synthetic cells | No real GBIF/CHELSA run; evidence rule (p ≤ 0.05 on Boyce) and clamp band to review |
| 4 Predictors | Done | ADR-0015 | no new layer selected | Demo bit-identical; offline stack tests; live Provence stack 11.4–12.7 s fresh, 0.02–0.04 s cached | No climate-only vs climate + land-cover comparison on real species; CORINE/OSO, soils, year filter not done |
| 5 Future climate | Done | ADR-0016 | off (no projection) | Present outputs byte-identical with and without projections; ensemble resume byte-identical; live 10 CMIP6 windows 32.3 MB in 1.8 s | No real species projected end to end; national-extent transfer estimated, not measured; CMIP6 licence |
| 6 Second engine | Done | ADR-0017 (+ ADR-0002 pointer) | MaxEnt | Demo bit-identical; RF/ensemble on the demo; resume byte-identical; RF 500 trees: fit 0.04 s, 1 M-cell prediction 4.65 s | RF and ensemble only on synthetic data; RF weaker than MaxEnt under spatial blocks on the demo's small setting; seed sensitivity not reported |
| 7 Own field data | Done | ADR-0108 (+ ADR-0009 pointers) | off (no dataset chosen) | Demo bit-identical; SYNTHETIC Lambert-93 survey held out → PA AUC 0.83, TSS 0.50; isolation tests between organisations; browser import | No real client dataset; CSV only; no CLI import; data-protection referent's review |
| 8 Decision outputs | Done | ADR-0018 | on demand only (nothing computed unless asked) | Study files unchanged by analyses; footprint areas exact on demo-grid footprints; richness checked cell by cell; comparison twin runs D = ρ = 1 | No real species (browser check and FR/ES analysis warnings: QA 2026-10-20) |
| 9 Documentation | Done (this entry) | — | — | Internal links checked (see below) | Help content was updated per phase; not reviewed again here |

Cross-phase notes:

- Phase 3 left bias-matched (effort-weighted) null models as a hook; phase 2 implemented them
  (ADR-0013 §2), so that open point of phase 3 is closed.
- Phase 1a's presence–absence metrics were wired by phase 7 (independent validation).
- Measured numbers in this programme come from the synthetic demo, synthetic benchmarks, live
  reads of real layers over small Provence boxes, and one live search-API study (*Testudo
  hermanni*). None of the new methods has been run end to end on a real species with a real
  conclusion drawn from it.

#### What still needs the ecologists

Every item below is a choice made to get a working, conservative implementation; none should
become a default before review and a new ADR amending ADR-0009.

- **ADR-0009 (existing defaults)**: still not reviewed — cleaning rules, default variables,
  selection rule (OR10 then Boyce), P10 for areas, evidence-level cut-offs.
- **ADR-0011 generalised coordinates**: position of the rule (option A3-ii, before the
  uncertainty rule); the 10 % warning threshold and whether to cap the evidence level; whether
  non-generalised records *without* uncertainty should be excluded too; adopting DWCA/SQL
  downloads to get the flags in production.
- **ADR-0013 sampling bias**: the target group (GBIF class by default; orders, families or named
  survey schemes; per-species choices in multi-species studies); fallback when fewer than
  min(1000, background points, valid cells) cells have records; records as a proxy for effort;
  whether the bias diagnostic should ever affect the evidence level; whether thinning should be a
  default and at which distance.
- **ADR-0014 evaluation**: the evidence rule (cap at *Exploratory* when the held-out Boyce index
  is not better than null models at p ≤ 0.05); the grid-block clamp band [max(5 km, 2 cells),
  shorter extent / 3], especially at national scale; whether the AOA should become an evidence
  criterion; its 10 % warning.
- **ADR-0015 predictors** (the design notes §8): land-cover source (WorldCover only,
  or CORINE/OSO); class groups and the group left out; year filter on records; priority order in
  the collinearity filter; built-up excluded or allowed with a warning; scale (cell or ≈ 3 km
  neighbourhood); which relief and water layers are useful; the climate-only vs climate + land
  cover comparison on five real species.
- **ADR-0016 future climate**: P10 as the single change threshold; equal GCM weights; the majority
  rule for pale classes; the 10 % extrapolation warning; whether projections need their own
  evidence statement.
- **ADR-0017 random forest**: 1:1 presence:background balance for all taxa; categorical subset
  splits; the rank-averaged ensemble and whether it should ever be the default; 500 trees and the
  optional `mtry` grid; out-of-bag scores for the RF thresholds.
- **ADR-0108 own field data**: default source "both" (GBIF + the organisation's presences); the
  absence rule (an absence is never a presence, and is "not found", not "absent for sure"); the
  leakage rule (validation sites in training presence cells excluded); the minimum of 10 + 10
  sites for metrics; the *Validated* criteria (held out, ≥ 30 presence and ≥ 30 absence sites,
  PA AUC ≥ 0.75, TSS ≥ 0.40 at the training max-SSS threshold, *Supported* after every cap); the
  10 % `own_data_concentrated` threshold (own presences' extent vs the study area's). Also the
  data-protection referent
  (the design notes).
- **ADR-0018 decision outputs**: the multiplicative priority score; the median sampling split;
  P10 as the class threshold; the 10 % candidate and hotspot shares; *Indicative* as the minimum
  evidence level for aggregation; whether *Exploratory* species should be refused in survey
  priority.

#### Not verified (consolidated)

- **Real species, end to end**: no real-species study with the target-group background, null
  models, grid blocks, AOA, additional predictors (the live tests used virtual species on real
  layers), future projections (virtual species on real CMIP6 layers), the random forest or the
  ensemble, own data (synthetic CSV only) or decision outputs. Only the ADR-0011 rule was run
  live on a real species.
- **Effort grids on the real mirror**: never built from a real region archive; the hosted mirror
  is format 1 until its next weekly refresh builds a format-2 snapshot, so target-group studies
  there currently fall back to the random background with `target_group_unavailable`. The `fr-es`
  sizes (50–250 MB `effort.bin`, ≈ 2 GB temporary disk) are estimates.
- **Generalisation flags** for downloads and the mirror (needs DWCA or SQL downloads); until then
  such species carry `generalisation_flags_unavailable`.
- **Licences**: CC0 for CHELSA's CMIP6-based files to be confirmed with CHELSA (Dirk Karger,
  WSL); Copernicus DEM GLO-90 licence grants the rights without naming commercial use, a lawyer's
  confirmation is advisable. Attribution is given in every report either way.
- **National-extent costs are extrapolated**: projections ≈ 30 MB per layer per GCM for a 10° tall
  area, up to ≈ 3 GB on first use for 7 layers × 15 runs; RF ≈ 23 s per species for metropolitan
  France at 500 trees (from the 1 M-cell measurement); AOA, grid blocks and terrain/land-cover
  reads not measured on national stacks; DEM/WorldCover cache for France + Spain estimated.
- **UI**: `ui_smoke_hosted.py` not run; `scripts/ui_smoke.py` passes end to end since the QA of
  2026-10-20 (every option of phases 2–8 on demo studies, EN/FR/ES) but is not in CI; the data
  source line of the data review and *Data & exports* (e.g. "Synthetic demo records") and the
  per-setting rows of a study comparison stay in English (see *QA* below).
- **Other**: RF seed sensitivity not reported to users; checksums of cached DEM/WorldCover tiles
  not in the manifest; log capture of own-data imports not tested automatically; demo preview and
  demo run thin with different seeds.

### Phase 2 — Bias and cleaning (2026-10-20)

**Built** (ADR-0011 and ADR-0013, both *Proposed*):

- Generalised coordinates (ADR-0011, a change of a cleaning rule, implemented because it only
  removes points of unknown precision): rule `generalised` before the uncertainty rule
  (generalised and uncertainty missing or > `max_uncertainty_m`), counts in
  `data.generalised`, warnings `generalised_records` (Info; Warning at ≥ 10 % of the records in
  the area) and `generalisation_flags_unavailable` (downloads and mirror), status `generalised`
  in `occurrences.geojson`, SINP sentence for real data in the file description and reports,
  manifest `download_format`, `generalisation_flags_available` and per-species counts. ADR-0009 §1
  amended with a pointer.
- Distance thinning (opt-in, `filters.thin_km`, 0–100 km): spThin after the per-cell rule,
  seeded by the study seed and the species key, status/reason `thinned`.
- Target-group background (opt-in, `model.background_method = "target_group"`,
  `model.target_group` class, default the species' GBIF class): effort surface from the regional
  mirror's effort grids summed on the modelling grid (synthetic surface in the demo), background
  drawn uniformly among cells with records, fallback to the random background (same cells) with
  `target_group_unavailable` when unavailable; `effort.tif`, tile layer `effort`, `effort` in
  the point sample; bias-matched (effort-weighted) null models; `sampling_bias` diagnostic and
  `effort_concentration` Info (no effect on the evidence level); reports say "no bias correction
  was applied" only when true, with references (Phillips et al. 2009, Barber et al. 2022,
  Vollering et al. 2019, Bohl et al. 2019, Aiello-Lammens et al. 2015, Chapman 2020).
- API (`dto.rs` + `types.ts`), engine validation, manifest (`execution-manifest.md`),
  `api-design.md`, UI (Expert settings for background method, target class and thinning in the model step;
  `generalised`/`thinned` markers and legend rows in the data step; exclusion reasons and the new
  warnings worded in EN/FR/ES; effort map layer and legend, sampling-bias, background and
  generalised-records panels in the Data tab, method lines in Summary and Validation), help
  sections `bias`, `targetgroup`, `thinning`, `generalised` in EN/FR/ES.

**Verified:**

- Bit-identical defaults: `niche demo` with the build of `f07f9b2` and with this phase gives
  byte-identical `suitability.tif`, `uncertainty.tif`, `mess.tif`, `model.lambdas`,
  `response_curves.csv`, `occurrences.geojson` and `result.json`; `report.md` and `report.html`
  differ only in their generation time; same `spec_sha256`.
- Live search-API study (*Testudo hermanni*, 5.5–7.0 E, 43.0–43.8 N, factor 5, 1 509 records,
  network on 2026-10-20) with the builds before and after: 339 records flagged generalised, of
  which 108 not excluded earlier (licence) move from "Coordinate uncertainty > 1000 m" (280 →
  172) to the new reason (108); total exclusions, presences (61), metrics and evidence level
  unchanged; manifest `download_format: "search API (JSON)"`, flags available, counts per
  species; SINP sentence present; `generalised_records` raised as Warning (≥ 10 % of the
  records in the area).
- `cargo fmt --all --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`:
  clean. `cargo test --workspace`: 214 passed, 0 failed, 19 ignored (31 suites), including
  `crates/niche-app/tests/bias_options.rs` (5 tests: default bit-identity, thinning determinism
  and reporting, target group + 19 effort-weighted nulls on the demo with manifest checks,
  fallback identical to the random background, mirror effort grid summed on a modelling grid),
  `clean()` tests 1–4 of donnees-sensibles-rgpd.md §7 plus thinning and target-group sampling,
  warning and validation tests in `engine.rs`, diagnostic tests in `bias.rs`.
- `cd web && npm run typecheck && npm run lint && npm run build`: pass.
- API smoke (local server, demo): target-group study succeeded; `effort` tiles 200 PNG; sample
  returns `effort`; preview with `thin_km` returns `thinned` points; `thin_km: 500` → 422 with
  the validation message.
- `git diff --stat` shows no change under `crates/niche-maxent` (kernel untouched; golden tests
  green as part of `cargo test`).

**Not done / not verified:**

- The target-group path on real data: no regional mirror with effort grids on this machine, so
  the real effort query, class lookup and fallback reasons were exercised only offline (snapshot
  built in a test, synthetic demo surface); no comparison of random vs target-group background on
  real species.
- Generalisation flags for downloads and the mirror (needs DWCA or SQL downloads); until then
  every download/mirror species carries `generalisation_flags_unavailable`.
- Ecologists' review: rule position, 10 % threshold, choice of target group (class), thinning
  distance, whether the diagnostic should affect the evidence level.
- No browser check of the new UI (Playwright not installed here); `scripts/ui_smoke.py` not run.
- In the demo the preview draws its synthetic records with seed 1 and the run with the study
  seed (behaviour from before this phase), so thinning counts of a demo preview and a demo run
  differ; for real data they are the same.
- FR/ES wording of the English `fallback_reason` relies on patterns of the current English text.

### Phase 3 — Evaluation (2026-10-20)

**Built** (ADR-0014, *Proposed*; every option off by default and omitted from the spec JSON
when off):

- Null-model significance of the selected setting (`validation.null_models`, 0 or 19–200):
  uniform null presences over valid cells (weights hook left for phase 2's target-group
  surface), same partitioning and settings, mean held-out Boyce and OR10, empirical one-sided p;
  Rayon over (replicate, fold), cancellation, bit-exact checkpoints for resume. Evidence capped
  at *Exploratory* when the Boyce index is not better than nulls at p ≤ 0.05.
- `spatial_grid` validation: square blocks of `block_km` or, automatically, the median variogram
  range of the predictors clamped to [max(5 km, 2 cells), shorter extent / 3]; k (default 5),
  k achieved and the autocorrelation analysis recorded; both spatial methods satisfy the
  "spatially blocked" evidence criterion.
- Area of applicability (`validation.aoa`): `aoa.tif`, tile layer `aoa`, `aoa_di` in samples,
  summary and warning > 10 % outside; no effect on the evidence level.
- API (`dto.rs` + `types.ts`), engine validation (400 messages), `report.md`/`report.html`
  sections with references, manifest fields (`execution-manifest.md`), UI (Expert controls,
  Validation tab, map layer and legend, summary line, warnings, help articles in EN/FR/ES).

**Verified:**

- Bit-identical defaults: the demo (`niche demo`) with the build of `d7eca78` and with this
  phase gives byte-identical rasters, lambdas, curves, occurrences, `result.json` and
  `report.html`; `report.md` differs only in its timestamp; same `spec_sha256`.
- `cargo fmt --all --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`:
  clean. `cargo test --workspace`: all suites pass (159 tests, 0 failed), including the new
  `crates/niche-app/tests/eval_options.rs` (4 tests: default bit-identity, grid blocks + 19 nulls
  + AOA on the demo with model outputs unchanged, user block size and validation errors, null
  models interrupted and resumed with identical outputs) and 4 evidence-rule unit tests.
- `cd web && npm run typecheck && npm run lint && npm run build`: pass.
- `git diff --stat` shows no change under `crates/niche-maxent` (kernel untouched; golden tests
  green as part of `cargo test`).
- Timings (8 threads): 20 null models × 5 folds on the demo 2.8 s (whole species ≈ 10 s); AOA
  on the demo 0.03 s (79 344 cells); AOA for 1 M cells × 7 synthetic correlated predictors
  0.05–0.23 s (release).

**Not done / not verified:**

- No run on real GBIF/CHELSA data (network not used); AOA and grid-block behaviour on real
  stacks and national extents is unmeasured.
- Bias-matched (weighted) nulls: hook only, waits for phase 2.
- `scripts/ui_smoke.py` (local server, Playwright headless Chromium): the demo flow ran to the
  results page (screenshot checked), then the script stopped at its help-language step, which
  still expects language items directly in the user menu (they moved into a submenu in
  `ec599be`, before this phase); the script was not changed. A study with every phase 3 option
  was also opened in the browser: Validation tab (null models, block design), AOA overlay and
  legend rendered without console errors; `aoa` tiles, `aoa_di` samples and the 422 message for
  5 null models were checked through the API.
- Ecologists' review of the proposed evidence rule and of the clamp band.

### Phase 4 — Predictors (2026-10-20)

**Built** (ADR-0015, *Proposed*; no new layer selected by default):

- Catalogue (`env::layer_catalogue`): the 19 CHELSA layers (same defaults) then 12 predictors of
  `niche_data::predictors` — terrain (`dem_elevation`, `dem_slope`, `dem_roughness`), land-cover
  fractions (`lc_tree` … `lc_wetland`), `dist_water`. `LayerInfo` gains `group`
  (`climate`/`terrain`/`land_cover`/`water`) and `attribution` (`dto.rs` + `types.ts`).
- `build_stack`: every predictor read on the same native CHELSA window, aggregated by the same
  factor (mean of valid sub-cells), grid checked against the CHELSA window (1e-9 cell); valid
  cells need every layer finite; per-layer progress with source; cancellation between layers
  (`build_stack_cancellable`, used by the job runner); test seam `build_stack_with` (reader of
  additional predictors passed in). Provenance: CHELSA block unchanged for climate-only stacks;
  with additional predictors, per-layer source/vintage/licence/citation/attribution/method,
  `sources[]` and `aggregation`. Manifest `methods.predictors`.
- Warnings when the predictor is kept by the collinearity filter: `land_cover_temporal`,
  `built_up_bias`, `land_cover_composition` (all eight fractions), `surface_model`.
- Reports (`report.md`, `report.html`), only when an additional predictor is selected: *Data
  sources* section (each source, vintage, layers, licence + URL, attribution, per-layer method,
  aggregation text: DEM mean / Horn slope at 90 m, WorldCover fractions from 10 m pixels,
  distance truncated at 25 km), source-specific interpretation limits, citations.
- UI: variables step grouped by source group with headings, intro, source/licence/attribution per
  group, shared caution shown once, cautions translated (FR/ES), land-cover hint and warning when
  all eight are selected, "Climate only (recommended)" reset, per-group "Clear"; results show
  units next to variable names (contributions, curves, used/dropped). Help article *Terrain, land
  cover & water* (EN/FR/ES, 6 sections), variables/getting-started/limits/glossary updated;
  warning texts in EN/FR/ES. `scripts/ui_smoke.py` article list includes `predictors`.
- Docs: ADR-0015, ADR-0006 pointer, `predictors-v1.md` status, `execution-manifest.md` section.

**Verified:**

- `cargo fmt --all --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`:
  clean. `cargo test --workspace`: 203 passed, 0 failed, 20 ignored (network), including
  `crates/niche-app/tests/predictors_stack.rs` (6 offline tests: climate-only stack unchanged
  through the new and the production entry points; mixed-source window/aggregation/NoData/
  provenance; cancellation and shifted-grid refusal; catalogue groups and unchanged defaults;
  pairwise filter keeps all eight compositional fractions → warning; fit + both reports on a
  mixed stack).
- `cd web && npm run typecheck && npm run lint && npm run build`: pass.
- Bit-identical defaults: `niche demo` built from `feat/science-v1` (f07f9b2) and from this phase:
  every output file identical except `report.md` (timestamp line) and `manifest.json`
  (`created_at`, `job_id`, checksum of `report.md`); same `spec_sha256`.
- Live (`cargo test --release -p niche-app --test predictors_stack -- --ignored --nocapture`,
  fresh cache): box 5.3–5.7° E 43.4–43.7° N, BIO1/4/12/15 + SLOPE + TREE + DWAT, 49 × 37 cells,
  1 728 valid (1 077 km²): stack 11.4–12.7 s fresh (WorldCover ≈ 4 s per layer, DEM ≈ 1 s),
  0.02–0.04 s cached; no NaN on land in the box; fit on a synthetic virtual species (113
  presences, default settings) 0.9–1.5 s. The first live run found a 2·10⁻¹⁴° origin difference
  of the cropped distance raster; fixed by the tolerant grid check.
- Browser (local server, Playwright headless): variables step in EN/FR/ES renders the four
  groups, cautions and land-cover warning; no console errors.
- `crates/niche-maxent` unchanged.

**Not done / not verified:**

- Year filter on records when land cover is used; CORINE/OSO; categorical class; neighbourhood
  fractions; soils; river networks (ADR-0015 "Not done").
- Checksums of cached DEM/WorldCover tiles in the manifest.
- Real-species comparison climate-only vs climate + land cover (`occupation-du-sol.md` §6/§8);
  MESS effect not measured on a real species (the live test used all valid cells as background,
  so MESS was 0 % in both runs).
- Layer names and the English source strings of the catalogue are not translated in the UI
  (cautions are).
- Ecologists' review of ADR-0015.

### Phase 6 — Second engine (2026-10-20)

**Built** (ADR-0017 stays *Proposed*; new section *Integration*; ADR-0002 pointer):

- API (`dto.rs` + `types.ts`): `model.algorithm` `maxent` (default, omitted from the JSON) \|
  `rf` \| `ensemble`; `model.rf_trees` (100–2000, absent = 500); `model.rf_tune_mtry`. Results:
  `selected.algorithm`/`rf_trees`/`rf_mtry`/`rf_seed`, RF rows in `tuning` (`algorithm`, `mtry`),
  `validation.members` (ensemble), `contributions[].rf_importance`, `partial_dependence`. All new
  fields optional, so older `result.json` files load. Engine validation: unknown algorithm, trees
  out of range, `rf_*` with `maxent` → 400.
- `fit.rs`: one pipeline for the three algorithms (`Setting`/`Fitted`, `split`, `fit_model`,
  `evaluate`, `training_presence_scores`); MaxEnt arithmetic and order unchanged. Down-sampled
  RF from `niche-rf` with trees fitted in parallel (`Trainer::fit_tree` in Rayon) and parallel
  prediction; same folds, selection rule (optional `mtry` grid {⌊√p⌋, ⌊p/2⌋}), null models,
  MESS, AOA (weights = RF permutation importance; ensemble = mean of the members' normalised
  importances), thresholds, areas, fold-SD uncertainty and evidence rules. RF thresholds use
  out-of-bag scores at the training presences. Ensemble = equal-weight mean of the members'
  mid-rank percentiles among their predictions at the (training) background, evaluated fold by
  fold as an ensemble of fold models; its metrics drive the evidence level. RF explanations:
  permutation importance (3 repeats, ≤ 2000 background points) and partial dependence (51
  points, 300 rows; new `niche_rf::partial_dependence_of` to parallelise over variables).
- Checkpoints: RF fold checkpoints keep metrics only (`cv-rf-{combo}-{fold}.json`); the selected
  setting's fold forests are refitted after a resume (deterministic, 0.04 s per 500-tree forest on
  500 + 10 000 points, vs ≈ 3 MB per stored forest). Per-species forest seed (mod 2^53) recorded.
- Outputs: `model.rf.json` (forest, format v1, JSON), `partial_dependence.csv`, `model.lambdas`
  only when a MaxEnt model exists; algorithm-specific `suitability.tif` description; reports
  (md/html) with method text, member table, references (Breiman 2001; Chen, Liaw & Breiman 2004;
  Valavi et al. 2021 *Ecography* 44:1731–1742; Valavi et al. 2022 *Ecol. Monogr.* 92:e01486;
  Friedman 2001 — checked against Crossref and the paper text) and the "relative suitability
  index, not a probability" sentence; manifest `methods.random_forest`, `methods.ensemble`,
  `software.engine` (`execution-manifest.md`, `api-design.md`).
- UI (EN/FR/ES): Expert model step *Algorithm* (MaxEnt recommended / RF / ensemble, short
  explanation and cost hint, trees, "also try mtry = p/2"; MaxEnt grid hidden for RF), summary
  and review texts; results: algorithm in Summary/Data/simple view, importance bars per
  algorithm, "Partial dependence (random forest)" vs "Response curves", ensemble members table,
  tuning table with RF rows, map legend and file descriptions per algorithm. Help article
  *Modelling algorithms* (5 sections) in EN/FR/ES; exports table and limits updated;
  `scripts/ui_smoke.py` article list includes `algorithms`.

**Verified:**

- `cargo fmt --all --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`:
  clean. `cargo test --workspace`: 229 passed, 0 failed, 21 ignored (33 suites), including the
  new `crates/niche-app/tests/rf_engine.rs` (6 tests: MaxEnt wire form and outputs unchanged and
  no RF keys/files; RF on the demo — metrics, files, reports, manifest; ensemble on the demo with
  the mtry option; RF + 19 null models + AOA; an interrupted ensemble study with null models
  resumed from RF metric checkpoints gives byte-identical outputs; validation errors), new unit
  tests (mtry candidates, mid-rank percentiles, algorithm parsing) and a `niche-rf` test that the
  per-variable partial dependence equals the full one.
- `cd web && npm run typecheck && npm run lint && npm run build`: pass.
- Bit-identical defaults: `niche demo` with the build of `9a88072` and with this phase: every
  output file identical except the generation time in `report.md`/`report.html` and
  `manifest.json` (`created_at`, `job_id`, checksum of `report.md`); same `spec_sha256`,
  `methods` and `software`.
- Timings (release, 8 threads): forest fit 0.04 s (500 trees) / 0.08 s (1000) on 500 + 10 000
  points × 7 predictors; prediction of 1 M cells 4.65 s / 9.96 s → default 500 trees. Demo spec
  (79 344 cells, 152 presences, 10 000 background): MaxEnt 5.0 s, RF 2.7 s, ensemble 7.9 s
  (wall, `niche run`); `model.rf.json` 1.8 MB. Held-out metrics (spatial blocks): MaxEnt Boyce
  0.53 / OR10 0.00 / AUC 0.67, RF 0.47 / 0.04 / 0.69, ensemble 0.52 / 0.02 / 0.68.
- Browser (local server, agent-browser, Spanish UI): model step with the algorithm options,
  results of an RF and an ensemble study (importance, partial dependence, members table, legend)
  and the help article rendered; no console errors.
- `git diff --stat` shows no change under `crates/niche-maxent`.

**Not done / not verified:**

- No run on real species; RF and ensemble only on the synthetic demo. On the demo's small test
  setting RF is clearly weaker than MaxEnt under spatial blocks (Boyce 0.22 vs 0.57) — expected
  from trees' lack of extrapolation, but not studied on real data.
- Seed sensitivity of RF metrics (Boyce 0.51 vs 0.47 between two seeds on the demo) is not
  reported to users.
- Member-disagreement map for the ensemble, CV-weighted ensembles, categorical subset splits,
  and a compressed/binary forest format (JSON kept; checkpoints avoid storing forests).
- `scripts/ui_smoke.py` not run (Playwright for Python not installed here).
- Ecologists' review of ADR-0017 and its integration choices (500 trees, OOB thresholds,
  rank-averaged ensemble).

### Phase 5 — Future climate (2026-10-20)

**Built** (ADR-0016, *Proposed*; off by default and omitted from the spec JSON when empty):

- API (`dto.rs` + `types.ts`): `JobSpec.projections[]` of `{period, ssp, gcms[]}` (CHELSA
  catalogue values); results `projections[]` (`ProjectionResult`: threshold, climate and static
  variables, cells/missing cells, present and future suitable km², stable/gained/lost/unsuitable
  km² and the part on which every GCM agrees, mean SD across GCMs, novel fraction, `per_gcm[]`,
  `synthetic`); sample `projections[]`. Engine validation: catalogue values, ≥ 1 GCM, no repeated
  GCM or scenario, ≤ 15 scenario × GCM runs, ≥ 1 climate layer → 400. Older results load.
- `projection.rs`: future stack on the same native window and aggregation as the present one
  (only the model's climate predictors are read; terrain/land-cover/water kept at present values,
  warning `static_predictors_in_projection`); prediction through the same path as the present map
  for MaxEnt, RF and the ensemble (`fit::Projector`, ensemble ranks against the present training
  background); GCM mean and SD, change classes at the present P10 with GCM agreement, exact areas,
  future MESS against the training reference (lowest over GCMs; warning
  `projection_extrapolation` > 10 %), cells valid now but missing in the future left out and
  counted (`projection_missing_cells`). Outputs `future_<period>_<ssp>_{mean,sd,change,agreement,
  mess}.tif`, `future_areas.csv`, report sections (method, areas with the GCM range, per-GCM
  table, HTML change maps pale where < majority agree and hatched where extrapolated, limits,
  references checked on Crossref), manifest `methods.projections` and `future_climate` (scenarios,
  every URL read, source, licence, CMIP6/ISIMIP/WCRP citation). Tile layers
  `future_<period>_<ssp>_{mean,sd,change,mess}`.
- Resume: present outputs first, then `projection-state.json` (floats by their bits) and one
  checkpoint per GCM run; an interrupted species resumes without refitting; `result.json` still
  last (`job-lifecycle.md`). Progress per GCM run, cancellation between runs.
- Demo: SYNTHETIC future layers (`demo::future_layers`), labelled in every output.
- `niche-data`: `Http::range_bytes_received()` (bytes of range requests) for the measurements.
- UI (EN/FR/ES): Expert model step *Future climate projections* (period, SSP with a plain
  explanation, GCMs with "All five (recommended)", run count, cost note, inline errors that block
  the step), review line; results *Future* tab (scenario cards: present → future chip with the
  GCM range, class table with "all models agree", SD, extrapolated share, per-GCM table, show on
  map, limits), Summary chips, map period/scenario selector with change/mean/SD layers, change
  legend (pale = < majority agree), future extrapolation hatching, point readings per scenario,
  warnings and file descriptions translated. Help article *Future climate projections* (5
  sections), variables and limits texts updated; `scripts/ui_smoke.py` article list.
- Docs: ADR-0016, `execution-manifest.md`, `api-design.md`, `job-lifecycle.md`,
  `data-sources-v1.md`.

**Verified:**

- `cargo fmt --all --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`:
  clean. `cargo test --workspace`: 237 passed, 0 failed, 22 ignored (34 suites), including
  `crates/niche-app/tests/projections.rs` (default wire form and outputs unchanged, present
  outputs byte-identical with and without a projection; MaxEnt on the demo with 2 scenarios ×
  5 + 2 GCMs — area identities, P10 area equal to the present one, files, synthetic labels,
  reports, manifest, tiles and samples; RF projection; an ensemble study stopped between GCM runs
  resumes without refitting and gives byte-identical outputs, run 6 times; validation errors) and
  unit tests of `projection.rs` (change classes, agreement, areas, MESS and per-GCM values on a
  hand-computed toy grid; layer-name parsing; run checkpoint round trip).
- `cd web && npm run typecheck && npm run lint && npm run build`: pass.
- Bit-identical defaults: `niche demo` with the build of `14715a6` and with this phase: every
  output identical except the generation time in `report.md` (and so its checksum, `created_at`
  and `job_id` in `manifest.json`); same `spec_sha256`, `methods`, `software`.
- Live (`cargo test --release -p niche-app --test projections -- --ignored --nocapture`, fresh
  cache, 2026-10-20): Provence 1° box, virtual species on real BIO1/4/12/15 (model kept BIO1 and
  BIO12), 2041–2070 SSP3-7.0 × 5 GCMs: 10 CMIP6 windows in 1.8 s, 32.3 MB of range reads
  (≈ 3.2 MB, 0.15–0.25 s per layer window of 121 rows), 0 bytes and 0.0 s from the cache; no
  future NaN on land. Present stack: 13.9 MB, 3.5 s.
- Cost (release, 8 threads, demo at factor 1, 86 400 cells, 1 scenario × 5 GCMs): projections
  0.2 s with MaxEnt (study 1.47 → 1.61 s), 2.2 s with the ensemble (study 4.7 → 6.8 s).
- Browser (local server, agent-browser, Spanish UI): model step editor, Future tab, map selector,
  change layer with hatching and legend, point readings, translated warnings; no console errors.
- `crates/niche-maxent` unchanged.

**Not done / not verified:**

- No real species projected end to end through the engine (the live test projects a virtual
  species on real layers); no study at national extent (transfer estimated ≈ 30 MB per layer per
  GCM for a 10° tall area, not measured).
- Licence of the CMIP6-based CHELSA files still to confirm with CHELSA (open point in ADR-0016);
  attribution is given regardless.
- Dispersal scenarios, land-cover scenarios, future MOP/AOA, per-GCM rasters, comparison between
  periods; the evidence level does not cover projections.
- `scripts/ui_smoke.py` not run (Playwright for Python not installed).
- Ecologists' review of ADR-0016 (P10 as the only change threshold, equal GCM weights, majority
  rule, 10 % extrapolation warning).

### Phase 7 — Own field data and independent validation (2026-10-20)

**Built** (ADR-0108, *Proposed*; nothing changes unless a species uses the organisation's data):

- Import (`POST /api/v1/datasets`): CSV read and parsed in the browser (UTF-8 / Windows-1252,
  `;` `,` tab, RFC 4180, SHA-256 of the file), column mapping (species, X, Y; optional date,
  precision, status, protocol), closed CRS list (`GET /coordinate-systems`: 4326, 2154, 3035,
  UTM 28N–32N), absence values; only chosen columns sent; server normalisation
  (`owndata::normalize`: decimal commas, five date forms, CRS domains, rejections with file
  lines), report, warnings (`many_rejected`, `axes_swapped`, `no_absences`, `synthetic`).
- Storage: schema v11 `datasets` (org-scoped, JSON description, no records), records in
  `datasets/<id>/records.json` (versioned JSON, original + WGS84 coordinates), copy into
  `jobs/<id>/input/` at submission (checksum checked, 409 when changed), deletion, same retention
  as studies, ≤ 20 per organisation; routes list/get/delete, all 404 for another organisation.
- GBIF name matching (`POST /species/match`, names only, auto-proposal for exact species-rank
  matches), assignment of names to the study's species, "add to the study".
- Use per species (`SpeciesRef.own = {values, role, gbif}`, `JobSpec.own_dataset`): GBIF + own
  presences (default when a dataset is chosen), own only (no GBIF request), or GBIF for training
  with own data held out. Own presences through the ADR-0009 rules with licence `ORG_OWNED` and
  the `absence` rule, own first; per-source counts (`data.by_source`, GBIF records in own cells);
  `own_data_concentrated` Info.
- Independent presence–absence validation after the final prediction (`independent.rs`): same
  cleaning, one site per cell, leakage rule (sites in training presence cells excluded), PA AUC,
  sensitivity/specificity/TSS/kappa at the training P10 and max-SSS thresholds, Brier and
  calibration table as diagnostics, spread; *Validated* level (held out, ≥ 30 + 30 sites,
  AUC ≥ 0.75, TSS ≥ 0.40, Supported) applied after every cap; the "No independent validation
  data…" reason replaced when independent data were used.
- Manifest `own_data` and per-species `own_data` (checksums, mapping, counts, metrics; no
  coordinates), `methods.own_data`/`independent_validation`; report.md/html sections
  "Occurrence data sources" and "Independent validation (presence–absence)" with references
  (Allouche et al. 2006; Leroy et al. 2018; Liu et al. 2013; Brier 1950).
- UI (EN/FR/ES): "The organisation's field data" in the data step (datasets, import wizard,
  report, matching table, sources per species), source breakdown in the data step and Data tab,
  Independent validation section in the Validation tab, `Validated` badge, map markers and legend
  (own squares, teal diamonds for validation presences/absences), review line, warning texts,
  help article *Your own field data* (9 sections) and the evidence article updated.
- Docs: ADR-0108, ADR-0009 pointers (§1, §6), `execution-manifest.md`, `api-design.md` §4.9,
  `plan-donnees-terrain.md` status line, `scripts/ui_smoke.py` article list.

**Verified:**

- Bit-identical defaults: `niche demo` built from `9a88072` and from this phase: every species
  output identical (`suitability.tif`, `uncertainty.tif`, `mess.tif`, `model.lambdas`,
  `response_curves.csv`, `occurrences.geojson`, `result.json`, `report.html`); `report.md`
  differs only in its generation line; `manifest.json` only in `created_at`, `job_id` and the
  checksum of `report.md`; same `spec_sha256`.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`: clean.
  `cargo test --workspace`: 232 passed, 0 failed, 20 ignored (33 suites), including `crates/niche-app/tests/own_data.rs`
  (4 tests: SYNTHETIC Lambert-93 survey of the demo species held out → 130 presence / 258
  absence sites, PA AUC 0.83, TSS 0.50 at max SSS, leakage rule excluding the rows placed on
  training records, witness coordinate absent from manifest/reports/result; own presences with
  and without the demo records; checks, copy and deletion; default spec JSON and outputs
  unchanged), unit tests of `owndata.rs` (numbers, dates, Lambert-93 against PROJ reference
  points to 1e-7°, rejections with lines, absences, swapped axes, records file) and
  `independent.rs` (each Validated criterion), `gbif::NameMatch` parsing, and
  `api_accounts.rs::datasets_stay_within_the_organisation` (another organisation gets 404 on
  every dataset route, preview and `POST /jobs`; 409 on a changed checksum; copy kept after the
  dataset's deletion and removed with the study; no response quotes the witness value).
- `cd web && npm ci && npm run typecheck && npm run lint && npm run build`: pass.
- Browser (local server, agent-browser/Chromium, Spanish UI): demo → expert → data step →
  import of a SYNTHETIC Windows-1252 `;` CSV with decimal commas in Lambert-93 (columns, CRS and
  absence value detected), 300/300 rows imported, GBIF matcher answered "none" for the virtual
  species, name assigned, "held out" chosen, preview showed 105 / 193 sites and 2 leakage
  exclusions, study run: Validation tab AUC 0.841, TSS 0.53 at max SSS, calibration with its
  caveat, all Validated criteria ✓ except Supported (level Indicative, unchanged); Data tab
  source breakdown; no console errors; server log has only the dataset id and counts.
- `crates/niche-maxent` unchanged.

**Not done / not verified:**

- No real client dataset (none may be committed); GeoNature exports, XLSX, GeoPackage,
  Shapefile, Lambert II and overseas systems; the CLI import (lot 7).
- Ecologists' review: *Validated* criteria, leakage rule, minimum of 10 + 10 sites, the 10 %
  concentration threshold, default source "both".
- Log capture is not tested automatically (checked by code reading and one server log); a GBIF
  retry is logged at debug level with the name-matching URL (contains the taxon name).
- Survey/protocol ids are stored but not used (e.g. blocking validation by survey); imperfect
  detection of absences is not modelled.
- `scripts/ui_smoke.py` not run (Playwright for Python not installed here); `ui_smoke_hosted.py`
  not run. The hosted-mode UI (two organisations in a browser) was checked only through the API
  test.

### Phase 8 — Decision outputs (2026-10-20)

**Built** (ADR-0018, *Proposed*; every output is computed on demand from a finished study's
stored files, nothing is refitted, the study's outputs and manifest are never modified):

- Survey priority per species (`crates/niche-app/src/analyses/priority.rs`): mid-rank percentile
  of suitability × (1 − percentile of target-group effort) when `effort.tif` exists, otherwise ×
  percentile of the distance to the nearest known record (retained, thinned, held-out presences)
  with a `priority_distance_proxy` warning; four classes at the P10 threshold × median sampling
  split; candidate areas = 8-connected "prospect first" cells among the top 10 % by score, ranked
  by mean priority (`top_n` ≤ 100). `priority.tif`, `priority_class.tif`, `candidates.geojson`,
  `candidates.csv`. Wording "areas worth surveying".
- Project footprint (`footprint.rs`, `niche-geo/src/overlay.rs`): exact ellipsoidal area of the
  footprint ∩ each cell (Green's theorem on the authalic area element, Sutherland–Hodgman per
  cell); per species and rule (P10 headline, MTP/max SSS band) suitable km², share of the
  footprint, share of the species' suitable area; mean/max suitability, uncertainty, MESS and AOA
  shares, evidence; warnings (smaller than a cell, outside the study area, extrapolation);
  mandatory disclaimer; ≤ 50 per study. `footprint.geojson`, `footprint_species.csv`.
- Multi-species aggregation (`richness.rs`): species ≥ minimum evidence (default Indicative,
  Exploratory excluded and said so); `richness.tif` (species with suitable conditions at their own
  P10), `suitability_sum.tif` (index), `contributing.tif`, `hotspot.tif` (top 10 % of cells by
  count then summed suitability); stacking-overprediction caveat; refused below two species.
- Comparison of two studies (`comparison.rs`): same species, same grid (refused otherwise with
  "different grids … same study area and resolution"); `difference.tif` (B − A), Schoener's D,
  Spearman's ρ, mean (absolute) difference, P10 agreement km², metrics side by side, settings diff
  as dotted paths; `comparison.csv`.
- Every analysis: `jobs/{id}/analyses/{aid}/` written atomically with `analysis.json`,
  `report.html` (method, tables, maps, references, disclaimer, synthetic banner) and a manifest
  addendum `nichers-analysis-manifest/1` (study manifest SHA-256, inputs and outputs with
  SHA-256); deterministic ids for priority/aggregation/comparison (stored one returned with 200).
  API `GET|POST /jobs/{id}/analyses`, `GET|DELETE /jobs/{id}/analyses/{aid}`, `…/files/{name}`,
  `…/tiles/{layer}/…` (`crates/niche-server/src/analyses.rs`): reading follows the study's
  access, creating/deleting by the owning organisation, compared study must be the
  organisation's; blocking pool. `dto.rs` + `types.ts`.
- Second synthetic demo species *Virtualis secundus (SYNTHETIC)* (key 1000002) for multi-species
  demos; other keys keep the first species' records.
- UI (EN/FR/ES): results tab *Decision support* (four sections, map layers and legends,
  footprint upload/draw and outline, downloads, warnings), workspace menu entries "Compare with…"
  and "Species with suitable conditions", demo species step can add the second species; help
  article *Decision support* (6 sections); `scripts/ui_smoke.py` article list.
- Docs: ADR-0018 (citations checked against Crossref), `execution-manifest.md` (analysis
  manifest addendum), `api-design.md` §4.15.

**Verified:**

- Bit-identical defaults: `niche demo` built from `science-v1-p7` (faeb6fe) and from this phase:
  `suitability.tif`, `uncertainty.tif`, `mess.tif`, `model.lambdas`, `response_curves.csv`,
  `occurrences.geojson`, `result.json`, `report.html` identical; `report.md` differs only in its
  generation line; `manifest.json` only in `created_at`, `job_id` and the checksum of `report.md`.
  In tests the first demo species' outputs are byte-identical with and without the second species
  in the study, and study files are byte-identical before and after creating analyses.
- `cargo fmt --all --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`:
  clean. `cargo test --workspace`: 261 passed, 0 failed, 21 ignored (35 suites), including
  `crates/niche-app/tests/decision_outputs.rs` (6 tests: second species leaves the first
  unchanged; study unchanged + manifest addendum checksums + idempotent ids + deletion with the
  study; priority with the synthetic target-group effort; footprint areas exact on cell-aligned
  and half-cell footprints of the demo grid, partial and refused footprints; richness checked cell
  by cell on the two virtual species with hotspot ranking; comparison random vs target-group,
  twin runs D = ρ = 1, refusal on a coarser grid, other species, unknown study), unit tests
  (`overlay.rs` 5, `analyses` 8 incl. a toy-grid footprint), and `api_accounts.rs`
  (`organisations_only_see_their_own_jobs` covers the new routes; new
  `analyses_stay_within_the_organisation`: comparison with another organisation's study 404 both
  ways, footprint delete 404 for the other organisation, analyses deleted with the study).
- Demo numbers (factor 2, 19 836 cells): priority with effort — classes 25 060 / 8 920 / 8 231 /
  6 234 km², 5 candidate groups; distance proxy — 152 known records, median 9.4 km, 27 groups;
  aggregation (Indicative + Supported species) — 0/1/2 species in 3 505 / 11 856 / 4 475 cells,
  hotspots 10.07 % of the area; comparison random vs target group — D 0.872, ρ 0.757.
- `cd web && npm ci && npm run typecheck && npm run lint && npm run build`: pass.
- `git diff --stat` shows no change under `crates/niche-maxent`.

**Not done / not verified:**

- No browser check of the new tab and workspace entries (typecheck/lint/build only);
  `scripts/ui_smoke.py` not run.
- No run on real species; survey priority with a real regional effort grid untested.
- Analysis warnings shown in English in the UI (no FR/ES wording by code yet); map clicks do not
  sample analysis layers.
- Stratified sampling within priority strata, buffers around footprints, corrected stacked
  richness, significance of D, comparisons of more than two studies, projections (phase 5) in
  these outputs, a CLI command.
- Ecologists' review of ADR-0018 (score, median split, P10 classes, 10 % candidate and hotspot
  shares, minimum evidence level).

## QA (2026-10-20)

End-to-end QA of phases 2–8 on the synthetic demo (local server, headless Chromium through
Playwright for Python in a venv under `target/`; screenshots under `target/qa/`, not committed).

**`scripts/ui_smoke.py`** — fixed (language choice moved into the user menu's submenu, `ec599be`)
and extended; passes end to end on a fresh data directory (`UI smoke: OK`, ≈ 2.5 min). It runs an
Expert demo study with every option (≈ 5 km grid; own-data import of a SYNTHETIC Lambert-93 CSV
written by the script, held out for validation; ensemble with 100 trees; target-group background;
5 km thinning; spatial grid blocks; 20 null models; AOA; one projection with one GCM), a
two-species demo study, then checks: results wording (ADR-0007), Validation tab (null models,
independent validation), map layers and legends (AOA hatching, sampling effort, future change),
Future tab, Decision support (survey priority with the effort surface, footprint smaller than a
cell with its warning, multi-species stack with its caveat, comparison of the two studies), FR/ES
labels and absence of known English server texts, no side-panel overflow / hidden results tab /
over-wide null-model table at 1440, 820 and 390 px, help (index lists the 17 articles in each
language, forbidden wording), console errors.

**Found and fixed** (commit `8429446`):

- **Expert mode crashed the app** (React #185, "Maximum update depth") on any draft without
  projections — every fresh draft: `useDraft((s) => s.projections ?? [])` returned a new array on
  each render (since phase 5, `e78faaf`). Stable empty constant.
- English-only texts in FR/ES: analysis warnings of Decision support (worded by code and params
  like other warnings; `footprint_smaller_than_cell` now carries `cells`), richness exclusion
  reasons, evidence-level reasons (null models, independent validation, pooled taxa, criteria),
  dropped-variable reasons, variogram model names, algorithm/validation/background codes in the
  study comparison, predictor names, units and sources (by catalogue id,
  `web/src/components/domain/layerNames.ts`; the demo's result labels "<name> (SYNTHETIC)" were
  not resolved at all).
- Layout: results tabs (6) scrolled out of sight in the side panel (Decision support invisible) —
  they wrap; null-model table 1101 px wide in a 387 px panel — fits (verdict on its own row);
  side-panel table cells wrap; decision section toggles truncated in FR; wizard footer overflowed
  at 390 px in ES; own-data source select and dataset name overflowed the panel.
- Footprint areas "-0 ha" / "-0 %" (empty `f64` sum is −0.0 in Rust): fold from +0.0; number
  formatting never signs zero.
- Help *Known limits*: independent validation is now an Expert option (ADR-0108), EN/FR/ES.

**Not fixed:** data source line of the data review and *Data & exports* ("Synthetic demo records
(virtual species)", "GBIF occurrence search API, accessed …") and the per-setting rows of a study
comparison (spec paths, by design technical) stay in English; wide data tables (footprint 10
columns, candidate areas, per-GCM) still scroll horizontally inside the panel; the species-name
table of the own-data import overflows into its scroller; FR/ES fallback-reason regexes still
depend on the English text (they match all 8 current server reasons); es-ES does not group
4-digit numbers ("9650 km²", Intl minimum grouping); `ui_smoke_hosted.py` not run.

**Integration test** `crates/niche-app/tests/combined_options.rs`: target-group background +
thinning + spatial grid blocks + 20 null models + AOA + ensemble + one projection + SYNTHETIC
own-data validation in one demo study; every option leaves its result, outputs and manifest
sections exist, and a study interrupted during cross-validation resumes (reusing the saved
models) to byte-identical species outputs and an identical manifest (dataset ids, times and the
checksums of files containing them excepted). ≈ 9 s; passed 5/5 runs.

**Definition of Done:** `cargo fmt --all --check` ok; `cargo clippy --workspace --all-targets -- -D
warnings` ok; `cargo test --workspace` 270 passed, 0 failed, 22 ignored; web typecheck, lint,
build ok; `git diff c09cdd1 -- crates/niche-maxent` empty; `cargo build --release && python3 -I
scripts/compare_maxent.py` (Java 25, `maxent.jar` present): 16/16 cases pass (regenerated
timings in `docs/benchmarks/phase3-results.md` not committed).
