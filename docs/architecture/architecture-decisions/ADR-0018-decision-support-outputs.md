# ADR-0018 — Decision-support outputs: survey priority, project footprint, multi-species aggregation, comparison of studies

- Status: **Proposed — pending review by Natural Solutions ecologists**
- Date: 2026-10-20 (Science v1, phase 8)
- Related: ADR-0007 (wording: relative suitability, never probability, never "range" or
  "occupied area"), ADR-0009 §5 (thresholds MTP / P10 / max SSS, exact ellipsoidal areas) and §6
  (evidence levels), ADR-0013 (target-group effort surface, `effort.tif`), ADR-0014 (AOA),
  ADR-0107 (studies and projects, organisation isolation), ADR-0203 (execution manifest).
- Implementation: `crates/niche-app/src/analyses/` (pure computations: `priority.rs`,
  `footprint.rs`, `richness.rs`, `comparison.rs`, `report.rs`), `crates/niche-app/src/engine/analyses.rs`
  (reading study outputs, writing `analyses/`, manifest addendum), `crates/niche-geo/src/overlay.rs`
  (exact polygon ∩ cell areas), `crates/niche-geo/src/colormap.rs` (priority, classes,
  diverging, hotspot), `crates/niche-server/src/analyses.rs` (routes), `crates/niche-app/src/demo.rs`
  (second synthetic species), `dto.rs` + `web/src/api/types.ts`, UI (results tab *Decision
  support*, workspace entries) and help article *Decision support*.
- Tests: `crates/niche-app/tests/decision_outputs.rs`, unit tests in `analyses/*` and
  `niche-geo/src/overlay.rs`, `crates/niche-server/tests/api_accounts.rs`
  (`organisations_only_see_their_own_jobs`, `analyses_stay_within_the_organisation`).

## Context

A study gives one suitability map per species with its evidence level. The clients of
the design notes §2 decide other things: where to send surveyors (park managers,
inventory planning), what a project footprint may affect (pre-diagnostic of an impact study),
which areas matter for several species of a list (species-list screening), and how much a
methodological choice changes the answer (random vs target-group background, MaxEnt vs random
forest, climate only vs climate + land cover). The tool does not replace a field inventory or an
impact assessment (§2); these outputs must say so and must not turn relative suitability into
presence.

## Decision

Four analyses, **computed on demand from a finished study's stored outputs** (GeoTIFFs and
`result.json`). Nothing is refitted, no default changes, and a study's own files and execution
manifest are never modified, so every study — default or not — stays bit-identical. Requests are
`POST /api/v1/jobs/{id}/analyses` (§5).

### 1. Survey priority (per species): "areas worth surveying"

Within the study area (cells with a prediction):

- `s` = mid-rank percentile of the cell's relative suitability.
- **With the target-group effort surface** (`effort.tif`, ADR-0013): `e` = mid-rank percentile of
  the cell's target-group records; **priority = s × (1 − e)**; a cell is *little sampled* when
  `e < 0.5` (the less-sampled half; when most cells have no record, every cell without records).
- **Without it** (random background), the stated proxy is the **distance to the nearest known
  record of the species** (retained, thinned and held-out presences, from `occurrences.geojson`;
  distance transform of `niche-geo`): `d` = mid-rank percentile of that distance;
  **priority = s × d**; *little sampled* = `d > 0.5` (farther than the median). A warning says
  that distance to the species' own records is not sampling effort.
- Classes at the species' **P10 threshold** (ADR-0009 §5 headline): 1 *prospect first* (≥ P10,
  little sampled), 2 *known* (≥ P10, well sampled), 3 lower suitability and little sampled,
  4 lower suitability and well sampled.
- **Candidate areas**: 8-connected groups of class-1 cells among the **10 % of the study-area
  cells with the highest priority score**, ranked by area-weighted mean priority (ties: larger
  area first); for each: centroid, bounding box, exact km², mean priority, mean/max suitability,
  mean effort or distance, extrapolated (MESS < 0) share. `top_n` 1–100, default 20.

Why this method: model-guided sampling — surveying where predicted suitability is high — finds
new populations more efficiently than random sampling (Guisan et al. 2006: field survey of a rare
Swiss plant and simulations, improvement by a factor of 1.8 to 4 over simple random sampling;
Le Lay et al. 2010: 240 field sites, six new sites of a rare plant found only by model-based
sampling, discovery rate of common species doubled). Crossing suitability with effort turns that into "high suitability *not yet*
surveyed", which is the question of a survey plan; the effort surface of the target group is
the sampling effort the study already uses for bias correction (Phillips et al. 2009). Ranks
(percentiles) rather than raw values make the product scale-free (records per cell are
heavy-tailed; relative suitability differs between algorithms) and bounded 0–1. A 2 × 2
classification at P10 keeps the "suitable conditions" definition users already see.

Outputs: `priority.tif` (score 0–1), `priority_class.tif` (1–4), `candidates.geojson`
(centroid points with `bbox`), `candidates.csv`, `report.html`. Wording: "areas worth
surveying", never "where the species is".

### 2. Project-footprint overlay (pre-diagnostic)

The user uploads (or draws) a footprint — GeoJSON Polygon/MultiPolygon in WGS84, same parsing and
200 000-vertex limit as study areas; at most 50 footprints per study. For **every species** of
the study:

- the footprint is intersected **exactly** with the model grid: each cell contributes the
  ellipsoidal area (WGS84) of its overlap with the footprint (`niche_geo::overlay`: Green's
  theorem on the authalic area element, edges straight in lon/lat as in RFC 7946;
  Sutherland–Hodgman clipping per cell). Polygon areas equal `quad_area_km2` for cell-aligned
  rectangles (tests to 1e-9 relative);
- per threshold rule — **P10 headline**, band MTP – max SSS: suitable km² inside the footprint,
  share of the footprint (over the part with predictions), species' suitable km² in the whole
  study area and the share of it the footprint covers;
- area-weighted mean and the maximum relative suitability, mean uncertainty (fold SD),
  extrapolated share (MESS < 0), outside-AOA share (when computed), evidence level and algorithm.

Warnings: footprint smaller than one model cell (values describe the cells it touches),
footprint partly outside the study area or over cells without predictions, extrapolation inside
the footprint. **Mandatory disclaimer** on the page, the CSV description and the UI: "does not
replace a field inventory or an impact assessment; low suitability is not evidence of absence".

Outputs: `footprint.geojson` (as submitted, with name and area), `footprint_species.csv`
(one row per species and rule), `report.html`.

### 3. Multi-species aggregation (stacked SDM)

For studies with ≥ 2 species, over the species whose evidence level is **at least the chosen
minimum (default *Indicative*: *Exploratory* models are excluded by default, and the page says
so)**; at least two must qualify:

- `richness.tif`: number of species with relative suitability ≥ their own P10 threshold, worded
  **"number of species with suitable conditions"**;
- `suitability_sum.tif`: summed relative suitability, documented as an **index** for ranking
  cells, not an expected number of species (relative suitability is not a probability);
- `contributing.tif`: species with a prediction in the cell;
- `hotspot.tif`: the **top 10 % of the study-area cells** ranked by count, ties broken by summed
  suitability then cell order (so the share is 10 % up to one cell). The 10 % cut is a
  convention — hotspots are defined as the richest fixed share of grid cells in the literature
  (e.g. Prendergast et al. 1993; Orme et al. 2005), with shares that vary between studies — not an
  ecological threshold.

Caveat (Info warning, report, UI): stacked binary maps **overpredict** the number of species
present — they ignore biotic interactions, dispersal and the number of species a place can hold
(Guisan & Rahbek 2011; D'Amen et al. 2017), and thresholding each species before stacking is
itself a source of bias (Calabrese et al. 2014, who recommend summing calibrated probabilities —
not available here, since relative suitability is not calibrated, hence the "index" wording of
the sum). Computed **on demand**
rather than at job end, so per-species outputs and single-species studies are untouched.

### 4. Comparison of two studies of one species

Between two finished studies **of the organisation** that model the same species (GBIF key):

- refused unless both suitability grids are the same lattice (size, origin, cell within 10⁻⁶
  cell): "the two studies are on different grids … run the second study with the same area and
  resolution as the first". Nothing is resampled;
- over cells with a prediction in both: `difference.tif` = **B − A**; **Schoener's D** of the two
  surfaces, each normalised to sum 1 (Schoener 1968, applied to SDM surfaces by Warren et al.
  2008); **Spearman's ρ** (Pearson correlation of mid-rank percentiles); mean and mean absolute
  difference per cell; agreement of the two P10 maps in km² (both / only A / only B / neither);
- side by side: algorithm, evidence level, presences, CBI, OR10, presence-background AUC,
  validation method, background actually used, threshold areas, extrapolated and outside-AOA
  shares, variables; and the **settings diff** of the two specifications as dotted JSON paths
  (`null` = the default, omitted from the specification; the area geometry compared by a
  checksum).

It is a sensitivity check; the page says it does not say which study is right, and that
metrics from different validation designs or backgrounds are not directly comparable.

### 5. Storage, traceability, API

- Each analysis is a directory `jobs/{id}/analyses/{aid}/` written in a temporary directory and
  renamed (appears atomically): `analysis.json` (the API object), its files, `report.html` and a
  **manifest addendum** `manifest.json` (`nichers-analysis-manifest/1`; fields in
  `execution-manifest.md`) with the SHA-256 of the study's `manifest.json`, the `spec_sha256`,
  every study file read with its SHA-256, the request (footprint geometry by checksum), the
  method parameters and every output with its SHA-256. The study manifest stays immutable.
- Ids: `priority-{key}`, `richness-{level}`, `comparison-{other job id}-{key}` are deterministic
  (computed once; the same request returns the stored analysis with `200`), footprints get
  `footprint-{uuid v7}` (`201` each time).
- Isolation (ADR-0107): reading follows the study's access (administrators' support access
  included, logged); creating and deleting are for the owning organisation only; a compared
  study must also be the organisation's (`404` otherwise). Analyses live inside the study
  directory, so deleting the study (or its retention expiry) deletes them. A comparison stored
  under study A keeps its own difference map and B's checksums after B is deleted.
- Work runs in the blocking pool (`spawn_blocking`), never on the async runtime (ADR-0103);
  costs are a few raster passes (the demo's 19 836 cells: < 0.1 s per analysis in debug builds).

### 6. Second synthetic demo species

To demonstrate and test the aggregation offline, the demo gains *Virtualis secundus
(SYNTHETIC)* (GBIF-like key `1000002`, warmer and drier optimum, partly overlapping the first
species). Only this key gets its records; every other key keeps the first species' records, so
existing demo studies (and the `niche demo` outputs) are unchanged.

## Evidence (2026-10-20, synthetic demo, factor 2, 19 836 cells, fixed LQ/β 1, 2 000 background points)

- Survey priority with the synthetic target-group effort: median effort 0 records/cell (most
  cells unsampled), classes 25 060 / 8 920 / 8 231 / 6 234 km² (1–4), 5 candidate groups in
  the top 10 %. Without effort (random background): distance proxy from 152 known records,
  median distance 9.4 km, 27 candidate groups, first one 465 km² at mean priority 0.56.
- Aggregation of the two virtual species (evidence *Indicative* and *Supported*): cells with 0 /
  1 / 2 species with suitable conditions = 3 505 / 11 856 / 4 475; hotspots 1 984 cells
  (10.07 % of the area), all with 2 species.
- Comparison random vs target-group background (same species, same grid): D = 0.872,
  ρ = 0.757, mean |B − A| = 0.127; P10 both / only A / only B = 27 779 / 3 243 / 6 200 km².
  Two runs of the same specification: D = 1, ρ = 1, no setting differs.

## Costs

- Disk: per analysis, one or two float32 rasters of the study grid plus small files (≈ 2 ×
  4 bytes per cell before compression).
- No change to job runtime; no schema change (filesystem only).

## Limits and open points (for the ecologists' review)

- **Survey priority**: the median split, the multiplicative score and P10 as the class threshold
  are choices to review; the distance proxy ignores places surveyed without records; effort is
  records of a class, not visits or observers; candidate groups can be large and irregular
  (centroid outside the group: use the bounding box); no stratified random draw of sampling
  points within strata (Guisan et al. 2006, Le Lay et al. 2010 sampled within suitability
  classes) — a candidate for a later step; the model's evidence level is shown but not used to
  refuse an *Exploratory* species.
- **Footprint**: values are those of the cells touched (no sub-cell information); buffers around
  the footprint (disturbance distances) are not computed; no aggregation across footprints.
- **Aggregation**: no correction of stacked richness (e.g. probability ranking rules or SESAM
  constraints, Guisan & Rahbek 2011) — the outputs are labelled instead; species of different
  algorithms are stacked as they are; hotspot share fixed at 10 %.
- **Comparison**: same species and same grid only (no resampling); no significance test of D
  (niche equivalency tests of Warren et al. 2008 need the occurrence data and randomisation);
  differences in the training data are visible only through the settings diff and counts.
- Not done: comparisons of more than two studies, projections (phase 5) in these outputs, a CLI
  command.

## References

- Calabrese J.M., Certain G., Kraan C., Dormann C.F. (2014). Stacking species distribution
  models and adjusting bias by linking them to macroecological models. *Global Ecology and
  Biogeography* 23:99–112. doi:10.1111/geb.12102
- D'Amen M., Rahbek C., Zimmermann N.E., Guisan A. (2017). Spatial predictions at the community
  level: from current approaches to future frameworks. *Biological Reviews* 92:169–187.
  doi:10.1111/brv.12222
- Guisan A., Broennimann O., Engler R., Vust M., Yoccoz N.G., Lehmann A., Zimmermann N.E. (2006).
  Using niche-based models to improve the sampling of rare species. *Conservation Biology*
  20:501–511. doi:10.1111/j.1523-1739.2006.00354.x
- Guisan A., Rahbek C. (2011). SESAM – a new framework integrating macroecological and species
  distribution models for predicting spatio-temporal patterns of species assemblages. *Journal of
  Biogeography* 38:1433–1444. doi:10.1111/j.1365-2699.2011.02550.x
- Le Lay G., Engler R., Franc E., Guisan A. (2010). Prospective sampling based on model ensembles
  improves the detection of rare species. *Ecography* 33:1015–1027.
  doi:10.1111/j.1600-0587.2010.06338.x
- Orme C.D.L. et al. (2005). Global hotspots of species richness are not congruent with endemism
  or threat. *Nature* 436:1016–1019. doi:10.1038/nature03850
- Phillips S.J., Dudík M., Elith J., Graham C.H., Lehmann A., Leathwick J., Ferrier S. (2009).
  Sample selection bias and presence-only distribution models: implications for background and
  pseudo-absence data. *Ecological Applications* 19:181–197. doi:10.1890/07-2153.1
- Prendergast J.R., Quinn R.M., Lawton J.H., Eversham B.C., Gibbons D.W. (1993). Rare species,
  the coincidence of diversity hotspots and conservation strategies. *Nature* 365:335–337.
  doi:10.1038/365335a0
- Schoener T.W. (1968). The *Anolis* lizards of Bimini: resource partitioning in a complex
  fauna. *Ecology* 49:704–726. doi:10.2307/1935534
- Snyder J.P. (1987). *Map Projections — A Working Manual*. USGS Professional Paper 1395.
- Warren D.L., Glor R.E., Turelli M. (2008). Environmental niche equivalency versus
  conservatism: quantitative approaches to niche evolution. *Evolution* 62:2868–2883.
  doi:10.1111/j.1558-5646.2008.00482.x

Citations checked against Crossref (titles, journals, volumes, pages) on 2026-10-20.
