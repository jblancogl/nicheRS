# ADR-0016 — Future climate projections

- Status: **Proposed — pending review by Natural Solutions ecologists**
- Date: 2026-10-20 (Science v1 phase 5, `docs/science-v1-plan.md`)
- Related: ADR-0007 (output semantics), ADR-0009 (defaults, unchanged), ADR-0010 (native grid),
  ADR-0014 (MESS/AOA), ADR-0015 (static predictors), ADR-0017 (algorithms), ADR-0104
  (resume), ADR-0203 (execution manifest).
- Data: `docs/architecture/data-sources-v1.md` §4 (CHELSA v2.1 CMIP6 layers, verified URLs, grid,
  no-data caveat, licence and citation).
- Implementation: `crates/niche-app/src/projection.rs`, `fit::Projector`, `engine::project_species`,
  `demo::future_layers`; tests `crates/niche-app/tests/projections.rs`.

## Context

Consultancy clients ask whether conditions favourable to a species will persist in a study area
under climate change. Species distribution models are routinely projected to future climates,
with well-known pitfalls (Elith, Kearney & Phillips 2010; Yates et al. 2018): model transfer to
novel climates, uncertainty between climate models (often larger than between SDM algorithms in
later periods; Thuiller et al. 2019), and the temptation to read suitable-climate maps as future
ranges. `docs/research/07-scientific-validation.md` lists "silent extrapolation in projections"
as a risk to be met with a MESS overlay and a share of area extrapolated.

Phase 1c made the CHELSA v2.1 CMIP6 climatologies readable on the same 30″ grid as the
1981–2010 baseline: 3 periods × 3 SSPs × 5 ISIMIP3b GCMs × BIO1–BIO19, all 855 URLs checked.

## Decision

An opt-in **projection** step after the fit (`JobSpec.projections`, empty by default and omitted
from the JSON, so existing specifications keep their hash and give bit-identical outputs):

| Item | Choice |
|---|---|
| Request | `projections[]` of `{period, ssp, gcms[]}`; values from the CHELSA catalogue; no repeated GCM or (period, ssp); at least one climate layer selected |
| Limit | at most **15 scenario × GCM runs per study** (one period × 3 SSPs × 5 GCMs, or 3 periods × 1 SSP × 5 GCMs). Each run reads every climate predictor of each species' model once and predicts every cell once; see *Costs* |
| What changes | only climate predictors (CHELSA BIO1–19; the demo's synthetic temperature and precipitation). Terrain, land-cover and water predictors (ADR-0015) stay at present values; warning `static_predictors_in_projection` names them |
| Future stack | same native CHELSA window, same aggregation (mean of valid sub-cells), same lattice check as the present stack (ADR-0010). Only the predictors kept by the species' collinearity filter are read. A cell valid at present but without a future value under some GCM is left out of every map and area of that scenario and counted (`cells_missing`, warning `projection_missing_cells`) |
| Prediction | the final model(s) through the **same prediction path** as the present map (`fit::Data::predict_fitted`): MaxEnt cloglog, RF vote share, or the ensemble. The ensemble ranks each member against its **present training-background predictions** (the references of the present map), so future values are on the present scale and comparable; a future value beyond the present range gets rank 0 or 1 |
| Combination | per cell, unweighted **mean** over the GCMs and population **SD** across them (`_mean.tif`, `_sd.tif`); every GCM equally weighted (no model weighting or selection beyond CHELSA's five ISIMIP3b models) |
| Change | present **P10 threshold** (the headline rule of ADR-0009) applied to the present map and to the GCM mean: 1 unsuitable in both, 2 no longer suitable, 3 newly suitable, 4 suitable in both (`_change.tif`). Agreement = number of GCMs whose own prediction is on the same side of the threshold as the mean (`_agreement.tif`); tiles and report maps show a class pale where fewer than a majority agree |
| Areas | sums of exact ellipsoidal cell areas: present and future suitable km², each class, the part of each class on which **every** GCM agrees, and per GCM future suitable / gained / lost km² (the spread reported as a range). Shares are of the present suitable area |
| Extrapolation | MESS (Elith et al. 2010) of each GCM's future predictors against the **same training reference** as the present map (background + presences); `_mess.tif` = lowest over GCMs (hatched on the map); `novel_fraction` = share of the projected area with MESS < 0 under at least one GCM, and per GCM; warning `projection_extrapolation` above 10 % (the present rule's cut-off). No effect on the evidence level, which describes the present model |
| Wording | "change in the area of suitable (climatic) conditions"; never "range shift", "future distribution" or a probability; *newly suitable* is conditions, not colonisation; **no dispersal**, demography, adaptation, biotic interaction or land-use change is modelled — stated in reports, help, file descriptions and the UI |
| Outputs | per scenario `future_<period>_<ssp>_{mean,sd,change,agreement,mess}.tif`; `future_areas.csv`; result `projections[]`; tile layers and point samples; report sections "Future climate projections" (method, table of areas with spread, per-GCM table, change maps in HTML, limits, references); manifest `methods.projections`, `future_climate` (scenarios, every URL read, source, licence, citation) |
| Demo | `demo::future_layers`: SYNTHETIC warming (invented function of period, SSP and model name) of the synthetic temperature, −4 %/°C of precipitation, ruggedness static; labelled synthetic in every output |

### Resume and the completion marker (ADR-0104)

`result.json` stays the species' completion marker, written last. With projections, a species
(1) writes its present outputs except the reports, (2) saves `projection-state.json` in its
checkpoint directory (result so far, file checksums, occurrence points for the report map, final
MaxEnt model with numbers by their bits, forest, training cells, P10 threshold; every float
stored by its bits, because this build's JSON parser does not round-trip every f64), keyed by a
fingerprint of the spec hash, species and grid, (3) saves each GCM run (`proj-*.bin`, suitability
as f64 bits, MESS as f32 bits) as it completes, checking cancellation between runs, then (4)
writes the projection files, `future_areas.csv`, the reports (rendered from the written rasters,
so a resumed species gives the same bytes) and `result.json`, and removes the checkpoints. A job
interrupted during (3) resumes at (2) without refitting and recomputes only missing runs; outputs
are byte-identical to an uninterrupted run (test). A species without a fitted model gets no
projection (`projection_not_computed`).

## Costs (measured 2026-10-20)

- Network (live test, Provence 1° box 5–6° E, 43.5–44.5° N, 121 × 121 native cells, model kept
  BIO1 and BIO12, 2041–2070 SSP3-7.0, 5 GCMs): 10 layer windows read in 1.8 s, **32.3 MB** of
  range requests (≈ 3.2 MB and 0.15–0.25 s per layer window); the second pass, from the window
  cache, 0.0 s and 0 bytes. The present stack (4 layers + grid header) read 13.9 MB in 3.5 s.
  CHELSA files are stored one row per strip, so the bytes of a window grow with its number of
  rows (≈ 27 kB per row per layer here), not its width: a 10° tall study area would need ≈ 30 MB
  per layer per GCM, so up to ≈ 3 GB for 7 layers × 15 runs on first use (estimate).
- Computation (release, 8 threads, demo at factor 1, 86 400 cells, one scenario × 5 GCMs): all
  projection work (5 predictions, aggregation, 5 GeoTIFFs, CSV, reports) 0.2 s with MaxEnt (whole
  study 1.47 → 1.61 s), 2.2 s with the ensemble (500 trees; study 4.7 → 6.8 s). Prediction
  dominates and scales like the present map (RF ≈ 5 s per million cells per forest, ADR-0017).
- Disk per run checkpoint: 12 bytes per valid cell (removed when the species completes).

## Consequences

- Clients get projections with their uncertainty and their extrapolation made visible, and
  method text and references to paste into reports.
- Projection maps can be misread as forecasts; the wording rules above, the limits list and the
  help article are the mitigation. The evidence level is not extended to projections.
- First use in a new region costs network transfer proportional to rows × layers × runs.

## Open points

- **Licence of the CMIP6-based CHELSA files**: the dataset-level CC0 is taken to cover them; to be
  confirmed with CHELSA (Dirk Karger, WSL). Attribution (CHELSA, CMIP6/Eyring et al. 2016, ISIMIP3b/
  Lange 2019, WCRP acknowledgement, the five models) is given in every report and manifest either
  way (`data-sources-v1.md` §4).
- Ecologists' review: P10 as the single change threshold; equal GCM weights; the majority rule
  for pale classes; 10 % extrapolation warning; whether projections should carry their own
  evidence statement.
- Not done: dispersal scenarios (none / unlimited / distance-limited), land-cover scenarios,
  MOP or AOA for future conditions, per-GCM outputs as files, period comparison maps, MESS on the
  GCM-mean climate, time-series of periods.

## References (checked against Crossref, 2026-10-20)

- Elith, J., Kearney, M. & Phillips, S. (2010) The art of modelling range-shifting species.
  *Methods in Ecology and Evolution* 1, 330–342. doi:10.1111/j.2041-210X.2010.00036.x
- Eyring, V. et al. (2016) Overview of the Coupled Model Intercomparison Project Phase 6 (CMIP6)
  experimental design and organization.
  *Geoscientific Model Development* 9, 1937–1958. doi:10.5194/gmd-9-1937-2016
- Karger, D.N. et al. (2017) Climatologies at high resolution for the earth's land surface areas.
  *Scientific Data* 4, 170122. doi:10.1038/sdata.2017.122
- Lange, S. (2019) Trend-preserving bias adjustment and statistical downscaling with ISIMIP3BASD
  (v1.0). *Geoscientific Model Development* 12, 3055–3070. doi:10.5194/gmd-12-3055-2019
- O'Neill, B.C. et al. (2016) The Scenario Model Intercomparison Project (ScenarioMIP) for CMIP6.
  *Geoscientific Model Development* 9, 3461–3482. doi:10.5194/gmd-9-3461-2016
- Thuiller, W., Guéguen, M., Renaud, J., Karger, D.N. & Zimmermann, N.E. (2019) Uncertainty in
  ensembles of global biodiversity scenarios. *Nature Communications* 10, 1446.
  doi:10.1038/s41467-019-09519-w
- Yates, K.L. et al. (2018) Outstanding challenges in the transferability of ecological models.
  *Trends in Ecology & Evolution* 33, 790–802. doi:10.1016/j.tree.2018.08.001
