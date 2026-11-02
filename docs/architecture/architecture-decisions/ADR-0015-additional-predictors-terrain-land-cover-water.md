# ADR-0015 — Additional predictors: terrain, land cover, distance to water

- Status: **Proposed — pending review by Natural Solutions ecologists**
- Date: 2026-10-20 (Science v1, phase 4)
- Related: ADR-0006 (commercial-use licences), ADR-0009 §2 (default predictors, unchanged),
  ADR-0010 (native CHELSA grid, no resampling, land mask), ADR-0201 (no abstraction without two
  implementations), ADR-0203 (execution manifest). Working paper:
  the design notes (land-cover proposal, French). Library:
  `docs/architecture/predictors-v1.md` (phase 1d).
- Implementation: `crates/niche-app/src/{env.rs,engine.rs,dto.rs,artifacts.rs,report_html.rs}`,
  `crates/niche-data/src/{predictors.rs,dem.rs,landcover.rs}`, `web/` (variables step grouped by
  source, warnings, help article *Terrain, land cover & water* in EN/FR/ES).
- Tests: `crates/niche-app/tests/predictors_stack.rs` (offline, plus one `#[ignore]` live test).

## Context

NicheRS models with CHELSA v2.1 1981–2010 climate only. Many generalist species come out
*Exploratory*: at 1 km, climate does not say where a species finds habitat. Land cover is what
ecologists ask for first; relief and water follow (`occupation-du-sol.md` §3.9 priority: land
cover > relief > water > soils). Phase 1d built the data side (`niche_data::predictors`): every
layer is computed on the native CHELSA 30″ window, so it stacks without resampling.

## Decision

1. **Catalogue.** `GET /api/v1/layers` lists the 19 CHELSA layers (unchanged, same defaults),
   then 12 additional predictors. `LayerInfo` gains `group` (`climate`, `terrain`, `land_cover`,
   `water`) and `attribution` (in `dto.rs` and `web/src/api/types.ts`).

   | group | ids | unit | source |
   |---|---|---|---|
   | terrain | `dem_elevation`, `dem_slope`, `dem_roughness` | m, °, m | Copernicus DEM GLO-90 |
   | land_cover | `lc_tree`, `lc_shrub`, `lc_grass`, `lc_crop`, `lc_built`, `lc_bare`, `lc_water`, `lc_wetland` | fraction 0–1 | ESA WorldCover 2021 v200 |
   | water | `dist_water` | km | ESA WorldCover 2021 v200 |

2. **Defaults: none selected.** `default_selected = false` for all twelve. ADR-0009 §2 is
   unchanged; a default study is bit-identical (stack, rasters, lambdas, `result.json`,
   reports) to the previous version.

3. **Fractional cover, not a categorical class** (`occupation-du-sol.md` §3.1–3.2): continuous
   variables go through the existing pipeline (aggregation, collinearity, MESS, response curves)
   with no change to the MaxEnt kernel. The categorical path stays unconnected in the application.

4. **Methods** (details and verification in `predictors-v1.md`):
   - every source pixel goes to the CHELSA cell containing its centre (no resampling); a cell is
     NoData when fewer than 50 % of its source pixels are valid;
   - elevation = mean of 3″ heights; slope = mean of 3″ slopes, Horn (1981) at 90 m with metric
     pixel sizes, computed **before** aggregation; roughness = SD of 3″ heights;
   - land cover = share of valid 10 m WorldCover pixels in eight class groups (tree 10; shrub 20;
     grass 30; crop 40; built 50; bare 60/70/100; water 80; wetland 90/95), summing to 1;
   - distance to water = exact Euclidean distance transform to cells with water fraction ≥ 0.5,
     window padded by 25 km, **truncated at 25 km**, so values do not depend on the study extent;
   - in `build_stack`, every layer is read on the **same native window** as CHELSA and aggregated
     by the same factor (mean of valid sub-cells, NoData when more than half are missing); the
     grid of each predictor is checked against the CHELSA window (same size, origin within 1e-9
     cell) and refused otherwise; a cell is valid only where every selected layer is finite.
     Cancellation is checked between layers; progress names each layer and its source.

5. **Collinearity**: unchanged (greedy pairwise Pearson, user priority order). Climate is listed
   first and new selections are appended, so climate is kept before land cover (§3.4 of the
   working paper). The eight fractions sum to 1: an exact linear dependency that the pairwise
   filter does **not** detect (verified: on random compositions all eight are kept). Mitigation:
   a hint in the variables step, and a `land_cover_composition` warning when all eight survive
   the filter.

6. **Cautions, never hidden** (shown in the variables step, the results warnings and the reports):
   `land_cover_temporal` (WorldCover 2021 vs 1981–2010 baseline and records since 1981),
   `built_up_bias` (built-up ≈ observer effort), `surface_model` (DEM is a DSM: canopy and
   buildings), distance to water misses streams and ponds. A cell at 0 % of a class absent from
   the training cells is flagged by MESS; the help explains it.

7. **Provenance and attribution.** Each additional layer's provenance (`source`, `vintage`,
   `licence`, `licence_url`, `citation`, `attribution`, `method`, `window`) is in the manifest
   `environment.layers[]`, and `environment.sources[]` lists each source once. `report.md` and
   `report.html` gain a *Data sources* section (source, vintage, layers, licence with URL,
   attribution, method per layer, aggregation) and source-specific interpretation limits. A
   climate-only study keeps its previous `environment` block and report text.

## Sources and licences (verified 2026-10-20; `predictors-v1.md`)

| source | licence | verification | required notice |
|---|---|---|---|
| Copernicus DEM GLO-90 (COP-DEM-GLO-90-F), `s3://copernicus-dem-90m` | Copernicus WorldDEM-90 "Full, Free & Open" licence, Art. 3–5: free, worldwide, reproduction, distribution, adaptation, combination | <https://dataspace.copernicus.eu/sites/default/files/media/files/2025-06/copernicus_contributing_mission_data_access_v2_cop_dem_licenses.pdf>; <https://registry.opendata.aws/copernicus-dem/> | "produced using Copernicus WorldDEM-90 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved" + liability sentence; no suggestion of endorsement |
| ESA WorldCover 10 m 2021 v200, `s3://esa-worldcover` | CC BY 4.0 | <https://esa-worldcover.org/en/data-access>; <https://creativecommons.org/licenses/by/4.0/> | "© ESA WorldCover project 2021 / Contains modified Copernicus Sentinel data (2021) processed by ESA WorldCover consortium" |

Both allow commercial use with attribution (AGENTS.md rule 6). The DEM licence grants the rights
without naming commercial use; a lawyer's confirmation is advisable (`occupation-du-sol.md` §2).
The restricted GLO-30-R/EEA-10-R licences in the same PDF do not apply to GLO-90-F.

## Evidence (verification, 2026-10-20)

- Offline tests (`predictors_stack.rs`): climate-only stacks equal the mean-aggregated CHELSA
  windows with the previous provenance keys, through both the new and the production entry
  point; mixed stacks read every predictor on the CHELSA window, keep the selection order,
  aggregate identically, drop cells where one layer is NoData, carry per-source attribution;
  cancellation and a shifted grid are refused; catalogue defaults equal the CHELSA defaults; the
  fitting pipeline and both reports run on a mixed stack.
- Live (`cargo test --release -p niche-app --test predictors_stack -- --ignored --nocapture`,
  fresh cache, home connection): study box 5.3–5.7° E, 43.4–43.7° N (49 × 37 cells, 1 728 valid,
  1 077 km²), BIO1/4/12/15 + slope + tree cover + distance to water: **11.4–12.7 s** fresh
  (CHELSA 0.9 s, DEM 0.8–1.0 s, WorldCover tree cover 3.8–4.5 s, distance to water 4.2–4.5 s
  for the 25 km padding), **0.02–0.04 s** from cache. No NaN on land in the box for any layer.
  The live run also found that the distance raster's origin differed from the CHELSA window by
  2·10⁻¹⁴° (float arithmetic of the crop); the grid check now uses a 1e-9-cell tolerance.
- Kernel untouched (`crates/niche-maxent` unchanged; golden tests green).

## Consequences

- Studies can describe habitat structure; species responding to forest cover, slope or water get
  a better-founded model, at the price of more cautions to read.
- Every added predictor can only increase the share of extrapolated cells (MESS minimum), which
  can lower the evidence level. This is intended.
- A first study in a region costs a few seconds per degree (WorldCover ≈ 4–5 s and 8 MB per
  degree); later studies read the instance cache.
- Default studies are unchanged; the API change is additive (`group`, `attribution`).

## What is needed to make some of them a default (a later ADR amending ADR-0009)

Ecologists' review of `occupation-du-sol.md` §8, in particular:
1. source (WorldCover only, or CORINE 2018 / OSO for European / French studies);
2. class groups and which group to leave out by default;
3. a year filter when land cover is used (± years around 2021) and its effect on sample sizes;
4. priority order in the collinearity filter (climate first);
5. built-up: excluded, or allowed with the warning;
6. scale (cell only or ≈ 3 km neighbourhood);
7. which of relief and water are useful for NS studies;
8. measured comparison climate-only vs climate + land cover on the five test species of
   `occupation-du-sol.md` §6 (OR10, CBI, extrapolated share, evidence level, dropped variables).

## Not done

- Year filter on records when land cover is used (`occupation-du-sol.md` §3.5).
- Categorical dominant class; neighbourhood fractions; CORINE/OSO; soils; river networks.
- Checksums of cached source tiles in the manifest.
- The comparison of §8 point 8 on real species.
