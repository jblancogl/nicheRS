# Evaluation methods v1 (`niche-eval`, Science v1 phase 1a)

Status: implemented in `crates/niche-eval`. Spatial grid blocks with the autocorrelation range,
the AOA and null models are wired into `niche-app` as opt-in evaluation settings by Science v1
phase 3 (ADR-0014, proposed); distance thinning and weighted null presences (target-group effort)
by phase 2 (ADR-0013, proposed); presence–absence metrics by phase 7 (independent validation
with the organisation's data, ADR-0108, proposed). Scientific
defaults stay governed by ADR-0009 until a new ADR adopts these methods.
Date: 2026-10-20.

All functions are pure: in-memory inputs, no I/O, no threads, no new dependencies. Every random
choice draws from the frozen `rng::Rng` (xoshiro256**), whose existing methods were not changed,
so results are bit-for-bit reproducible for a given seed. Coordinates are `(longitude, latitude)`
in WGS84 degrees. Unit tests live next to each function (`cargo test -p niche-eval`).

| Function / type | Module | Purpose |
|---|---|---|
| `haversine_km` | `spatial` | Great-circle distance |
| `thin_by_distance`, `thin_by_distance_reps` | `spatial` | Distance thinning of presences |
| `spatial_grid_blocks` | `spatial` | Square spatial blocks → CV folds |
| `autocorrelation_range_km` | `spatial` | Variogram range to size the blocks |
| `KdTree` | `kdtree` | Exact nearest-neighbour search |
| `Aoa`, `upper_whisker` | `aoa` | Area of applicability / dissimilarity index |
| `null_summary`, `null_presences` | `nulls` | Null-model significance |
| `sample_weighted_indices` | `nulls` | Weighted sampling without replacement |
| `ConfusionMatrix`, `pa_metrics`, `brier_score`, `calibration_table` | `pa` | Presence–absence validation |

## Great-circle distance

`haversine_km(a, b) -> f64`. Haversine formula on a sphere of the WGS84 mean radius
R1 = 6371.0088 km (IUGG). Error against the ellipsoid < 0.5 %, irrelevant at thinning/blocking
scales. Handles the antimeridian and poles; clamped for antipodes.

## Distance thinning (spThin)

`thin_by_distance(points, min_km, rng) -> Vec<usize>` and
`thin_by_distance_reps(points, min_km, reps, rng) -> Vec<usize>`.

- **Method.** Aiello-Lammens et al. 2015 (*Ecography* 38:541–545), R package spThin: visit
  points in random order, keep a point if no kept point lies at distance `< min_km`. The result
  is a maximal (not maximum) independent set; `_reps` repeats with successive draws and keeps
  the largest set (first on ties), as `thin(reps, max.files = 1)`.
- **Output.** Kept indices in ascending order.
- **Neighbour search.** Hash grid over Earth-centred 3-D Cartesian coordinates with cell edge =
  the chord equivalent to `min_km` (`2R·sin(min_km/2R)`, slightly enlarged against rounding);
  only the 27 neighbouring cells are scanned and the exact haversine distance decides. No
  singularity at poles or antimeridian (tested).
- **Edge cases.** Non-finite coordinates are never kept; `min_km ≤ 0` or NaN keeps every finite
  point; `min_km ≥ πR` keeps one point. Duplicates: only one survives for any `min_km > 0`.
- **Complexity.** Expected O(n) for spread points; worst case O(n·kept) when everything falls
  in a few cells. A unit test thins 50 000 points at 10 km and asserts < 5 s (no finer timing
  recorded).
- **Limits.** Greedy random order does not maximize the retained count; use `reps` (spThin's
  default is 100) if the count matters. Spherical Earth.

## Spatial grid blocks (blockCV)

`spatial_grid_blocks(pres, bg, block_km, k, rng) -> Folds`.

- **Method.** Valavi et al. 2019 (*Methods Ecol. Evol.* 10:225–232), blockCV `cv_spatial`
  with square blocks; rationale in Roberts et al. 2017 (*Ecography* 40:913–929).
- **Projection.** Local equirectangular approximation: `x = (λ − λmin)·cos φ₀·111.195 km`,
  `y = (φ − φmin)·111.195 km`, with φ₀ the mean latitude of all finite presences and background
  points, grid anchored at their south-west corner. Blocks are exactly `block_km` tall and
  `block_km·cos φ / cos φ₀` wide at latitude φ: fine for regional extents, distorted for
  continental-to-global ones. `cos φ₀` clamped at 0.01. Longitudes are not wrapped: areas
  crossing the antimeridian must be shifted to a continuous range (e.g. 0–360) by the caller.
- **Assignment.** Occupied blocks are shuffled (seeded), stably sorted by decreasing presence
  count and each assigned to the fold with the fewest presences so far (longest-processing-time
  greedy; ties → lowest fold). Background in occupied blocks follows its block; background-only
  blocks are shuffled, sorted by size and assigned to the fold with the fewest background points.
- **Guarantees.** Every fold has ≥ 1 presence whenever there are ≥ k occupied blocks. With fewer
  occupied blocks, the returned `Folds.k` is reduced to the number of occupied blocks (≥ 1):
  callers must read `k` from the result. Non-finite coordinates form one extra block (clean
  them beforehand).
- **Complexity.** O((n_pres + n_bg) log blocks).
- **Limits.** Fold presence balance depends on block granularity: few large blocks cannot be
  balanced. No checkerboard/systematic option, no hexagons, no environmental blocking.

## Spatial autocorrelation range (blockCV `cv_spatial_autocor`)

`autocorrelation_range_km(coords, values, rng) -> Option<RangeEstimate>`, with
`RangeEstimate { per_variable: Vec<Option<VariogramFit>>, median_km, n_points, cutoff_km }` and
`VariogramFit { model, range_km, nugget, partial_sill, at_upper_bound }`.

- **Method.** blockCV sizes blocks with the median of per-predictor variogram ranges, fitted by
  automap/gstat (Valavi et al. 2019). Here: ≤ 2000 points subsampled with `rng`; empirical
  semivariogram γ̂(h) = ½·mean (zᵢ − zⱼ)² in 15 equal-width lag bins up to a cutoff of one third of
  the largest pairwise distance (gstat default: one third of the bounding-box diagonal); lag =
  mean pair distance of the bin.
- **Fit.** Exponential, spherical and Gaussian models with nugget (automap's
  `autofitVariogram` also tries several families). Weighted least squares with gstat's default
  weights Nⱼ/hⱼ² (`fit.method = 7`). For each practical range on a 240-point log grid from
  3/20 of a bin width to 3 × cutoff, nugget and partial sill (≥ 0) have a closed-form solution;
  the (model, range) with the lowest weighted SSE wins. Reported range = practical range
  (exponential 3a, Gaussian √3·a, spherical a).
- **Deviation from the brief.** The brief asked for an exponential model only. On smooth
  synthetic fields of known scale (sum of Gaussian bumps, true 95 % decorrelation ≈ 173 km and
  ≈ 519 km) the exponential-only fit returned 429 km and 2172 km (at the search bound) because
  smooth fields have sigmoidal variograms near the origin. With model selection the estimates
  are 177 km and 471 km (Gaussian selected); exact exponential, spherical and Gaussian variograms
  are each recovered with the right family and range (tests).
- **Output.** `None` for a variable that is constant, has < 3 non-empty bins, or is pure nugget
  (no spatial structure). `at_upper_bound` flags ranges that hit 3 × cutoff (a lower bound only).
  Returns `None` overall with < 10 usable points or no fitted variable.
- **Complexity.** O(m²) haversine distances (m ≤ 2000 ⇒ 2·10⁶ pairs, bins cached as `u8`) plus
  O(m²) per variable.
- **Limits.** Isotropic, stationary model; trend in the variable inflates ranges. Sampling
  points should cover the study area evenly (e.g. random background), not the presences.

## k-d tree

`KdTree::new(rows) -> Option<KdTree>`, `nearest(q) -> Option<(usize, f64)>`,
`nearest_excluding(q, exclude) -> Option<(usize, f64)>`.

- **Method.** Friedman, Bentley & Finkel 1977. Split on the dimension of largest spread at the
  median, 16-point contiguous leaves, near-child-first search with plane-distance pruning and
  partial-distance early exit. Exact (tested against brute force up to 20 dimensions).
- **Complexity.** Build O(n log n). Query ≈ O(log n) for low intrinsic dimension; degrades to
  brute force for intrinsically high-dimensional data. Immutable, so safe to share across
  threads (the caller parallelizes; this crate does not).
- **Measured** (single thread, release, `aoa_query_throughput`, ignored test): 11 000 training
  rows × 19 variables driven by 3 latent factors (climate-like) → AOA build 0.11 s, 1.96 µs per
  query (≈ 12 s per 6 million cells single-threaded). Worst case, uniform 19-D noise → build
  1.9 s, 195 µs per query (≈ brute force). Real bioclimatic stacks are strongly correlated, so
  the first figure is the relevant one, but this has not yet been measured on real CHELSA data.

## Area of applicability (CAST)

`Aoa::new(rows, weights, folds, rng) -> Result<Aoa, AoaError>`, `eval(row) -> f64` (DI),
`threshold()`, `train_di()`, `mean_distance()`, `n_train()`; `upper_whisker(x)`.

- **Method.** Meyer & Pebesma 2021 (*Methods Ecol. Evol.* 12:1620–1633), R package CAST
  `trainDI`/`aoa`. Variables centred and scaled with the training mean and sample SD (R
  `scale()`), multiplied by importance weights (`None` = all 1; negative/non-finite → 0, as CAST
  zeroes negative weights). DI = distance to the nearest training point ÷ mean pairwise distance
  between training points. Common rescaling of weights cancels in the ratio, so CAST's weight
  normalization does not matter.
- **Threshold.** Training DI under CV: each training point against the training points of the
  *other* folds; threshold = upper whisker of `grDevices::boxplot.stats` (largest training DI not
  above upper Tukey hinge + 1.5 × hinge spread; R `fivenum` hinges). Cells with
  `eval(row) > threshold()` are outside the AOA.
- **Mean pairwise distance.** Exact for ≤ 2000 training rows; otherwise all pairs of a seeded
  2000-row subsample (≈ 2·10⁶ pairs; the estimate's relative error is far below the DI's
  meaningful precision).
- **Edge cases.** Rows with any non-finite value are excluded from training (`train_di` NaN).
  Constant variables contribute nothing. A query with a non-finite value in a weighted variable
  returns NaN. One fold only → leave-one-out nearest neighbour (CAST without CV folds). Errors:
  no usable rows, ragged rows, weights/folds length mismatch, all points coincident.
- **Complexity.** Build O(n log n) + one tree per fold + O(min(n, 2000)²·d). Query: one
  nearest-neighbour search (see k-d tree).
- **Limits.** Continuous predictors only (no categorical handling as CAST's dummy coding).
  The AOA is about predictor-space distance to training data; it does not certify model
  correctness inside it. Presences and background are both "training data", as in CAST applied
  to a fitted presence-background model.

## Null models (ENMeval `ENMnulls`)

`null_summary(observed, nulls, higher_is_better) -> NullSummary { mean, sd, z, p_value, n }` and
`null_presences(eligible, weights, n, rng) -> Vec<usize>`.

- **Method.** Raes & ter Steege 2007 (*Ecography* 30:727–736); Bohl et al. 2019 (*J.
  Biogeogr.* 46:1101–1111); ENMeval 2.0 `ENMnulls` (Kass et al. 2021). The caller refits the model
  with identical settings on each null presence set and passes the metric values here.
- **Statistics.** Mean and sample SD (n − 1, as R `sd()`) of finite nulls; z = (observed −
  mean)/sd, sign not flipped for lower-is-better metrics; empirical one-sided
  p = (1 + #nulls at least as good)/(n + 1) (North, Curtis & Sham 2002) — never 0, so 99 nulls can
  at best support p = 0.01. ENMeval reports a normal-approximation p from z; the empirical p does
  not assume normality.
- **Null presences.** `n` distinct elements of `eligible` without replacement, uniform or
  weighted (`weights[i]` ↔ `eligible[i]`, e.g. a target-group effort surface so nulls share the
  sampling bias of real records). Fewer returned when not enough have positive weight.
- **Limits.** Significance against random points says the model beats chance, not that it is
  ecologically right; spatially biased real data inflate it unless nulls are bias-matched.

## Weighted sampling without replacement

`sample_weighted_indices(weights, n, rng) -> Vec<usize>`.

- **Method.** Efraimidis & Spirakis 2006 (*Inf. Process. Lett.* 97:181–185), A-Res: key
  ln(u)/w with u ∈ (0, 1] (order-equivalent to u^(1/w) without underflow), keep the `n` largest
  keys, return them in decreasing key order (= successive draws proportional to weight among the
  remaining items; first-draw frequencies tested).
- **Edge cases.** Zero, negative and non-finite weights are never drawn; fewer than `n`
  positive weights → all of them. One uniform per positive weight in index order (deterministic).
- **Complexity.** O(N + n log n) (selection, then sort of the kept keys).

## Presence–absence metrics

`ConfusionMatrix::at_threshold(pres, abs, t)` with `sensitivity`, `specificity`, `tss`, `kappa`;
`pa_metrics(pres, abs, t) -> PaMetrics`; `brier_score(pres, abs)`;
`calibration_table(pres, abs, n_bins) -> Vec<CalibrationBin>`.

- **Method.** Allouche, Tsoar & Kadmon 2006 (*J. Appl. Ecol.* 43:1223–1232) for TSS vs kappa;
  Brier 1950; AUC is the existing Mann–Whitney `auc` applied to presences vs absences.
- **Convention.** Predicted present iff score ≥ threshold (consistent with `omission_rate`).
- **Calibration table.** Pooled finite scores sorted and cut into ≈ equal-count bins; ties are
  never split between bins, so bins can be unequal or fewer than requested. Each bin reports
  score range, mean predicted, observed presence frequency and count.
- **Caveats (must reach the UI).** Requires true absences from independent surveys; with
  background/pseudo-absences these metrics are not valid. Leroy et al. 2018 (*J. Biogeogr.*
  45:1994–2002) show TSS remains prevalence-dependent unless errors are balanced; report
  sensitivity and specificity next to it. Brier and calibration only make sense for
  probability-like outputs on PA data; MaxEnt cloglog/logistic are relative suitability
  transforms (AGENTS.md rule 3), so a calibration table for them describes the transform, not a
  probability of presence.

## Not covered here

Environmental (cluster) blocking, buffered LOO, NNDM/kNNDM, MOP, Sørensen index, partial ROC,
and the null-model refitting loop itself (an application-level concern, since it trains models).
