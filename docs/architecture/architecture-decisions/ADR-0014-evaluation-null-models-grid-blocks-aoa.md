# ADR-0014 — Evaluation: null models, spatial grid blocks sized by autocorrelation, area of applicability

- Status: **Proposed — pending review by Natural Solutions ecologists**
- Date: 2026-10-20 (Science v1, phase 3)
- Related: ADR-0009 (defaults and evidence levels, unchanged), ADR-0007 (output semantics),
  ADR-0104 (resume after interruption), ADR-0203 (execution manifest).
- Implementation: `crates/niche-app/src/{fit.rs,dto.rs,engine.rs,checkpoint.rs,artifacts.rs,report_html.rs}`,
  `crates/niche-server/src/studies.rs` (tiles), `web/` (Expert settings, Validation tab, map
  layer, help). Pure functions: `crates/niche-eval` (`docs/architecture/eval-methods-v1.md`).
- Tests: `crates/niche-app/tests/eval_options.rs`, unit tests in `fit.rs`.

## Context

ADR-0009 validates a model by spatially blocked cross-validation with 4 blocks cut at the median
latitude and longitude of the presences, and flags extrapolation with MESS. Three gaps are known
in the literature and were raised in the 2026-10-20 study of the code:

1. **Is the model better than chance?** A held-out Boyce index of 0.4 can be reached by a model
   fitted to random points when records are clustered or the study area has strong environmental
   gradients. Null models (Raes & ter Steege 2007, *Ecography* 30:727–736; Bohl et al. 2019,
   *J. Biogeogr.* 46:1101–1111; ENMeval 2.0 `ENMnulls`, Kass et al. 2021, *Methods Ecol.
   Evol.* 12:1602–1608) compare the observed metric with models refitted, with the same
   settings and partitions, on random locations.
2. **How large should spatial blocks be?** The 4 median blocks have a size set by the data,
   not by the spatial structure of the environment. Roberts et al. 2017 (*Ecography*
   40:913–929) recommend blocks at least as large as the range of spatial autocorrelation;
   blockCV (Valavi et al. 2019, *Methods Ecol. Evol.* 10:225–232) sizes square blocks by the
   median variogram range of the predictors.
3. **Where may the model be applied?** MESS checks each variable's range one at a time. The
   area of applicability (AOA; Meyer & Pebesma 2021, *Methods Ecol. Evol.* 12:1620–1633) also
   flags new *combinations* of predictor values, weighted by variable importance, with a
   threshold derived from the cross-validation itself.

The pure functions were implemented and tested in phase 1a (`niche-eval`); this ADR decides how
the application uses them.

## Decision

All three methods are **opt-in** (Expert mode and API). Their defaults are "off", the new
fields are omitted from the specification JSON when off, and a default specification gives
bit-identical outputs and the same specification hash as before (verified: see Evidence).

### 1. Null models — `ValidationSettings.null_models: usize` (default 0 = off)

- Only for the **selected** setting (feature classes, β) — ENMeval's `ENMnulls` also tests one
  chosen setting; repeating the whole tuning grid per null would multiply the cost by the grid
  size for no change in the question asked.
- For each replicate r = 1…N: draw as many null presences as the species has presence cells,
  without replacement, **uniformly over the cells with predictor data** (`null_presences`; the
  code passes an optional weight vector, `None` today, so that phase 2 can draw bias-matched
  nulls from a target-group effort surface as Bohl et al. 2019 recommend). Partition them with
  the study's validation method (same k; for `spatial_grid` the same block size, not
  re-estimated), fit every fold with the selected setting and the same background, and compute
  the mean held-out Boyce index and OR10 exactly as for the real model. A replicate counts only
  if every fold could be fitted.
- Summaries with `null_summary`: mean, sample SD, z and the **empirical one-sided p = (1 + #nulls
  at least as good) / (N + 1)** (North et al. 2002), for the Boyce index (higher is better) and
  OR10 (lower is better). The empirical p does not assume normality (ENMeval reports a normal
  approximation from z; z is reported too).
- Bounds: N = 0 or **19 ≤ N ≤ 200**. With N nulls the smallest attainable p is 1/(N + 1), so
  fewer than 19 could never reach 0.05 and would cap every model. The UI offers 20, 50 and 100.
- Seeding: replicate r uses its own stream derived from the job seed, the species key and r, so
  results do not depend on thread scheduling; (replicate, fold) tasks run in parallel with Rayon
  and stop at the next task on cancellation. Each (replicate, fold) metric is checkpointed
  bit-exactly next to the cross-validation checkpoints, with a fingerprint of all inputs, so an
  interrupted study resumes with identical results (test
  `null_models_interrupted_midway_resume_with_identical_results`).
- **Evidence rule (proposed).** When null models were run and at least one replicate was
  fitted, a model whose held-out Boyce index is **not better than the null models at p ≤ 0.05**
  is capped at *Exploratory*, with the reason line "✗ Boyce index not better than N
  random-location null models (p = … > 0.05) — capped at Exploratory" and the warning
  `not_better_than_null`. A significant result adds a "✓" reason line and never raises a level.
  When null models were not run, nothing is added and ADR-0009 applies unchanged. OR10 against
  nulls is reported but not used in the rule (OR10 of a threshold set on training data is less
  informative against random points; ENMeval reports both, without a rule).
- Wording: "better / not better than random-location models". Never "confidence"; the p-value is
  not a probability that the map is right.

### 2. Spatial grid blocks — `method: "spatial_grid"`, `block_km: Option<f64>`

- New validation method next to `spatial_block` (still the default) and `random_kfold`.
  Square blocks (`spatial_grid_blocks`), assigned to k folds balancing presence counts.
  `folds` is k (2–10); when the field is absent from the JSON it defaults to **5** for this
  method (blockCV's common choice) and 4 otherwise.
- `block_km = None` (automatic): median of the practical variogram ranges of the predictors
  used (after the collinearity filter), fitted on the background sample
  (`autocorrelation_range_km`, ≤ 2000 points), **clamped to [max(5 km, 2 cells), shorter side of
  the background extent / 3]**. Justification: blocks smaller than a few cells no longer separate
  training and test data; the upper bound keeps at least 3 × 3 blocks over the area so that 5
  folds can normally be filled (blockCV warns that blocks larger than a fraction of the extent
  leave too few blocks per fold). Climatic ranges are often hundreds of km (on the synthetic
  demo at 30″, 84–266 km for a 236 × 222 km region), so the upper bound is frequently active; the
  warning `block_size_clamped` (info) says so. If no variable can be fitted, one fifth of the
  shorter side is used with the warning `block_size_fallback`. A user value must be within
  1–1000 km.
- Recorded in `validation.blocks`: block size used, `chosen_by` (`user` | `autocorrelation`),
  k requested and achieved, and the autocorrelation analysis (median before clamping, band,
  points, lag cutoff, per-variable range, model, `at_upper_bound`). When fewer blocks than k
  contain presences, the function returns fewer folds: the warning `fewer_folds` is raised; with
  a single block the species fails with a clear message (nothing to validate against).
- **Evidence:** the ADR-0009 criterion "spatially blocked validation" accepts both
  `spatial_block` and `spatial_grid`.

### 3. Area of applicability — `ValidationSettings.aoa: bool` (default false)

- Placed in `ValidationSettings`, not `ModelSettings`: it never changes the fitted model; it is an
  assessment of where the cross-validated performance applies and it uses the validation folds.
- Training rows: presences + background of the final model. Weights: permutation importance of
  the final model, normalised to sum 1 (equal weights if all are 0). Folds: those of the
  cross-validation (background shared by all random k-fold folds is spread over the folds by
  index). DI is computed for every valid cell in parallel; threshold = upper whisker of the
  cross-validated training DI (CAST).
- Outputs: `aoa.tif` (DI per cell; registered with checksums like `mess.tif`), map tile layer
  `aoa` (hatched in purple, diagonals opposite to the MESS hatching, outside the AOA), `aoa_di`
  in the point sample, `SpeciesResult.aoa {threshold, fraction_outside, n_train, weights}`,
  sections in `report.md`/`report.html`, warning `aoa_outside` when more than 10 % of the study
  area is outside (same wording style as the MESS warning).
- **No effect on the evidence level.** MESS remains the extrapolation criterion of ADR-0009. The
  AOA is new in NicheRS, has not been reviewed against MESS on real studies, and its threshold
  depends on the CV design; using it as a criterion is a decision for the ecologists' review.

### Manifest, reports, API

- `manifest.json`: `spec.validation` carries the new fields when set; `methods` gains
  `spatial_grid`, `null_models`, `area_of_applicability` descriptions only when used
  (`evidence_rules` becomes "ADR-0009, ADR-0014"); each species entry carries `validation`
  (with `blocks`, `null_model`) and `area_of_applicability` when computed.
- Engine validation (`400` with a clear message): unknown method; `spatial_grid` folds outside
  2–10; `block_km` with another method or outside 1–1000 km; `null_models` not 0 nor 19–200.
- Contract: `dto.rs` and `web/src/api/types.ts` together; every new result field is optional and
  omitted when absent, so stored results deserialize unchanged.

## Evidence (verification, 2026-10-20)

- Bit-identity: the synthetic demo (`niche demo`, default spec, 25 settings) run with the build
  before this change (`d7eca78`) and after it gives byte-identical `suitability.tif`,
  `uncertainty.tif`, `mess.tif`, `model.lambdas`, `response_curves.csv`, `occurrences.geojson`,
  `result.json` and `report.html`; `report.md` differs only in its generation time line; the
  specification hash (`spec_sha256`) is identical. Test
  `default_settings_are_bit_identical_to_explicitly_disabled_options` keeps this checked.
- The evaluation options do not change the model: with null models and AOA on, the suitability,
  uncertainty and MESS rasters, the lambdas, folds and thresholds are byte-identical to the same
  study without them (test `grid_blocks_null_models_and_aoa_on_the_demo`).
- Demo result (factor 1, 25 settings, spatial grid, 20 nulls): blocks 70 km (median range
  199 km, clamped), 5 folds; observed Boyce 0.44 vs null 0.05 ± 0.20, p = 0.048; observed OR10
  0.11 vs null 0.21 ± 0.10, p = 0.19; 0 % outside the AOA (threshold 0.225).

## Costs (measured, 8-thread laptop, `dev` profile with the scientific crates at opt-level 3)

| Item | Measure |
|---|---|
| Null models | demo, 20 replicates × 5 folds = 100 fits of the selected setting (LQHP, β 4): **2.8 s**, against ≈ 10 s for the whole species fit (25 settings × 5 folds). Cost scales as N × k fits of the selected setting; 100 nulls ≈ 4–5 × the tuning grid of a fixed-setting study. |
| AOA, demo | 79 344 cells, 10 152 training rows, 3 predictors: **0.03–0.04 s**. |
| AOA, realistic size | 1 000 000 cells (≈ metropolitan France's land at 30″), 10 152 rows × 7 correlated synthetic predictors, release build: build 0.05 s; DI 0.05 s (smooth latent fields) to 0.23 s (with 10 % independent noise per variable) (`aoa_timing_at_a_realistic_grid_size`, ignored test). Not yet measured on real CHELSA stacks. |
| Spatial grid | One variogram fit per predictor on ≤ 2000 points (O(m²) pairs); included in the species fit time, not timed separately. |

## Consequences

- Default results, hashes and evidence levels do not change.
- Studies that opt into null models can only go down in evidence level (cap), never up; this is
  the conservative direction.
- Grid blocks often yield fewer than k folds in small or patchy study areas; the result says so.
- Null models make a study up to several times slower; the UI states the cost.
- Uniform nulls ignore sampling bias: with strongly biased records, nulls are easier to beat
  (Bohl et al. 2019). Bias-matched nulls need phase 2's target-group effort layer.

## What would be needed to make them defaults (a later ADR)

1. Review by Natural Solutions ecologists of the evidence rule (p ≤ 0.05 on the Boyce index,
   cap at Exploratory) on real client species, including bias-matched nulls from phase 2.
2. For grid blocks: comparison with the 4 median blocks on real studies (k achieved, metric
   stability), and a decision on the clamp band for national-scale areas.
3. For the AOA: comparison with MESS on real CHELSA stacks (agreement, share flagged), timing on
   national grids, and a decision on whether it should become an evidence criterion.
4. A compute budget per study in hosted mode for null models (quota counts species, not fits).

## Alternatives considered

- Null models over the whole tuning grid (ENMeval allows it): rejected, cost × grid size.
- Normal-approximation p from z (ENMeval): reported z, but the empirical p is used for the rule.
- Environmental (cluster) blocks, buffered LOO, NNDM/kNNDM: not implemented (phase 1a scope).
- Making the AOA an evidence criterion now: rejected until reviewed (above).
