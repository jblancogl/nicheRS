# ADR-0013 — Sampling bias: target-group background, effort layer, bias diagnostic and distance thinning

- Status: **Proposed — pending review by Natural Solutions ecologists**
- Date: 2026-10-20 (Science v1, phase 2)
- Related: ADR-0009 §3 (random background, unchanged default), ADR-0014 (null models: the
  weights hook is used here), ADR-0114 (regional GBIF mirror and its sampling-effort grids),
  ADR-0011 (generalised coordinates, same phase).
- Implementation: `crates/niche-app/src/bias.rs` (effort surface, synthetic demo effort,
  diagnostic), `env.rs` (`clean` thinning, `sample_background_target_group`), `engine.rs`
  (`background_for`, `target_group_effort`, validation, warnings, manifest, `effort` layer),
  `fit.rs` (weighted nulls), `artifacts.rs`, `report_html.rs`, `crates/niche-server/src/studies.rs`
  (`effort` tiles), `crates/niche-geo/src/colormap.rs` (`effort` ramp), `dto.rs` +
  `web/src/api/types.ts`, UI and help.
- Tests: `crates/niche-app/tests/bias_options.rs`, unit tests in `bias.rs`, `env.rs`, `engine.rs`.

## Context

Presence-only records (GBIF) are where people looked as much as where species live: near towns,
roads, reserves, popular sites. A random background (ADR-0009 §3) contrasts those biased
presences with an unbiased sample of the area, so MaxEnt partly models observer effort
(Phillips et al. 2009). ADR-0009 recorded this as a known cost ("no bias correction by default;
target-group background requires a defensible target group per species").

The **target-group background** (Phillips et al. 2009) draws the background from the locations
of records of a group of species surveyed the same way, so presences and background carry the
same spatial bias and it cancels out to the extent the group shares the species' bias. Barber et
al. (2022) found target-group backgrounds among the most effective corrections in Maxent; Vollering
et al. (2019) show that background placement matters as much as its size ("bunching up the
background"). The regional mirror (ADR-0114, format 2) now holds, per GBIF class, the number of
records per 30″ cell (data-sources-v1.md §2), which is the information needed.

Spatial clustering of presences is a second, related problem: many records in a few
well-surveyed places. **Distance thinning** (spThin, Aiello-Lammens et al. 2015; Boria et al.
2014) keeps presences at least a given distance apart.

## Decision

Everything is **opt-in**; the defaults of ADR-0009 do not change; new fields are omitted from
the specification JSON when off, so a default specification has the same hash and gives
bit-identical outputs.

### 1. Target-group background — `ModelSettings.background_method`, `ModelSettings.target_group`

- `background_method`: `random` (default) or `target_group`. `target_group`: a GBIF class name
  (e.g. `Aves`); absent = **the species' own class** from the GBIF backbone
  (`gbif::taxon_class`, memoised). Class is the grouping the mirror records; it is a pragmatic
  target group (same higher taxon, broadly similar recorders), not necessarily the ideal one
  (e.g. a single order or a survey scheme), which is a project decision the field leaves open.
- **Effort surface** on the modelling grid: for each modelling cell, the sum of the mirror's
  record counts of the class whose 30″ cell centre falls in it (`stack.spec.cell_of`), counted
  only in valid cells (study area ∧ land ∧ predictors).
- **Background**: up to `background_points` cells drawn **uniformly without replacement among
  the valid cells with at least one record** (the target group's record sites de-duplicated per
  cell, as Phillips et al. 2009 used target-group occurrence sites), seeded by the study seed.
  Not weighted by counts: the cells, not the records, are the sites; weighting would
  double-count prolific datasets (counts are records, not visits or observers).
- **Fallback, never failure**: when the surface is unavailable — no mirror covering the area,
  a format-1 snapshot without effort grids, no class for the taxon, the class lookup failing,
  no records of the class, or fewer eligible cells than `min(1000, background_points, valid
  cells)` — the species uses the **random background (the same cells a random-background study
  would draw)** with the warning `target_group_unavailable` (*Warning*) giving the reason, and
  the result records `background_sampling {requested: target_group, method: random,
  fallback_reason}`. A study never fails because the correction is unavailable; the reports say
  "no bias correction was applied" in that case.
- When fewer eligible cells than requested exist, the background is smaller (Info
  `target_group_background_smaller`).
- **Demo**: the synthetic demo has a synthetic effort surface (three "towns", a "valley road"
  and a sparse scatter; class "Synthetica (demo observers, SYNTHETIC)"), labelled synthetic in
  the result, layer and reports, so that the demo exercises the path offline. Any other class
  name in a demo study falls back (this is how the fallback is tested). The demo species'
  records are drawn from its true suitability, **not** from the synthetic effort, so on the demo
  the target-group background makes the model worse (it corrects a bias the records do not
  have): it demonstrates the mechanics, not the benefit.

### 2. Effort layer and bias-matched null models

- `effort.tif`: records of the target group per cell, NaN outside the valid cells, whenever an
  effort surface exists (also after a fallback for too few cells, as a diagnostic). Map layer
  `effort` (log-scale orange ramp, cells without records transparent), `effort` in the point
  sample.
- When the target-group background is used and null models are requested (ADR-0014), null
  presences are drawn **with probability proportional to the records per cell** (bias-matched
  nulls, Bohl et al. 2019; `NullModelSummary.sampling = "effort_weighted"`), so nulls share the
  real records' bias and are harder to beat than uniform nulls. Checkpoints of weighted nulls
  have their own context.

### 3. Bias diagnostic — `SpeciesResult.sampling_bias`

Whenever an effort surface exists: target group, records, cells with records and their share of
the study area, the largest count per cell, and the **share of presence cells inside the most
surveyed cells covering about 10 % of the study area**, against the share of the area those
cells cover (`expected`, 0.10 or more when cells tie at the cut; zero-effort cells never count).
A share well above `expected` says the records follow observer effort. Info warning
`effort_concentration`. **No effect on the evidence level** (no validated cut-off exists).

### 4. Distance thinning — `OccurrenceFilters.thin_km`

- Off by default; `0 < thin_km ≤ 100` km (validated). Applied **after** the one-record-per-cell
  rule, on the cell centres of the retained presences, with `thin_by_distance_reps`
  (spThin's greedy random-order algorithm; largest of up to 100 orders, fewer for very large sets
  to bound the work to ≈ 2·10⁶ point visits). Seeded by the study seed and the species key
  (`env::thin_seed`), so the preview, the run and a resumed run thin identically.
- Removed records get the status `thinned` and the exclusion reason "Thinned (closer than X km
  to a retained record)"; counted like the other reasons.

### 5. API, manifest, reports, UI

- Contract (`dto.rs` and `types.ts` together): `ModelSettings.background_method` and
  `target_group`, `OccurrenceFilters.thin_km`, `DataPreviewRequest.seed`;
  `SpeciesResult.background_sampling`, `SpeciesResult.sampling_bias`, `SampleValue.effort`,
  tile layer `effort`. All optional; stored results deserialize unchanged.
- Engine validation (400): unknown method; `target_group` with the random method; a class name
  that is empty, longer than 64 characters or has other characters than letters, digits,
  spaces and `-_(),.`; `thin_km` outside (0, 100].
- Manifest: `methods.background` (always, describing the method used by the spec),
  `methods.thinning` when used, per species `background` and `sampling_bias`.
- Reports: the background sentence, the sampling-bias diagnostic, the "Interpretation limits"
  sentence (says "no bias correction was applied" exactly when that is true), the thinning line
  and the references below.
- UI: Expert settings of the model step (background method, class, thinning distance), effort
  map layer and legend, sampling-bias panel in the Data tab, method lines in Summary and
  Validation, help articles in EN/FR/ES.

## Evidence (2026-10-20)

- Bit-identity: the default demo (`niche demo`) built from `f07f9b2` and from this change gives
  byte-identical `suitability.tif`, `uncertainty.tif`, `mess.tif`, `model.lambdas`,
  `response_curves.csv`, `occurrences.geojson` and `result.json`; `report.md` and `report.html`
  differ only in their generation time; same `spec_sha256`. Test
  `default_settings_are_bit_identical_to_explicitly_disabled_options` keeps this checked.
- Fallback = random: with an unavailable class, the rasters and lambdas are byte-identical to the
  random-background study (test `unavailable_target_group_falls_back_to_the_random_background`).
- Mirror effort grid → modelling grid: counts summed exactly per modelling cell, invalid cells and
  other classes left out (test `mirror_effort_grid_is_summed_on_the_modelling_grid`, offline
  snapshot).
- Demo, factor 2, seed 5, LQ β 1, 2000 background points, 19 nulls (synthetic): 6 213 of 19 836
  valid cells have synthetic records (31 %); background 2000 cells among them; 7 % of presence
  cells in the most-surveyed 10 % (10 % expected: the virtual species ignores effort); Boyce
  0.26 vs 0.70 with the random background, effort-weighted nulls p = 0.40 → capped at
  Exploratory. As stated above, this shows the method's mechanics and that a target group whose
  bias the records do not share degrades the model.
- Thinning: deterministic (same seed → same cells), every retained pair ≥ the distance (unit
  test); on the demo at 8 km, 2 runs identical.

## Costs

- Effort query: two binary searches and one read per class and study (0.1–3 ms for a 1° box to
  France, data-sources-v1.md §2); summing into the grid is linear in the cells with records.
- Thinning: expected O(n) per order; ≤ 2·10⁶ point visits in total.
- Bias-matched nulls cost the same as uniform nulls (ADR-0014).

## Limits and open points (for the ecologists' review)

1. Which target group: class by default; whether to allow orders, families or named survey
   schemes, and per-species choices in multi-species studies.
2. A target group corrects only the bias it shares with the species; species surveyed by
   specialists (e.g. bats, odonates) may not share their class's bias.
3. Records ≠ effort: a dataset posting many records at one point weighs as much as many
   observers (the background uses cells, the null weights use counts).
4. No real-data comparison yet (random vs target group on client species, with independent data).
5. Whether the diagnostic should ever affect the evidence level.
6. Whether thinning should become a default, and at which distance (often 1–10 km in the
   literature; tied to grid resolution and species mobility).

## References

- Aiello-Lammens, M.E., Boria, R.A., Radosavljevic, A., Vilela, B. & Anderson, R.P. (2015) spThin:
  an R package for spatial thinning of species occurrence records for use in ecological niche
  models. *Ecography* 38, 541–545.
- Barber, R.A., Ball, S.G., Morris, R.K.A. & Gilbert, F. (2022) Target-group backgrounds prove
  effective at correcting sampling bias in Maxent models. *Diversity and Distributions* 28,
  128–141.
- Bohl, C.L., Kass, J.M. & Anderson, R.P. (2019) A new null model approach to quantify
  performance and significance for ecological niche models of species distributions. *Journal
  of Biogeography* 46, 1101–1111.
- Boria, R.A., Olson, L.E., Goodman, S.M. & Anderson, R.P. (2014) Spatial filtering to reduce
  sampling bias can improve the performance of ecological niche models. *Ecological Modelling*
  275, 73–77.
- Phillips, S.J., Dudík, M., Elith, J., Graham, C.H., Lehmann, A., Leathwick, J. & Ferrier, S.
  (2009) Sample selection bias and presence-only distribution models: implications for
  background and pseudo-absence data. *Ecological Applications* 19, 181–197.
- Vollering, J., Halvorsen, R., Auestad, I. & Rydgren, K. (2019) Bunching up the background
  betters bias in species distribution models. *Ecography* 42, 1717–1727.
