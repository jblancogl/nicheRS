# NicheRS — Roadmap and feasibility gates

Dates are not committed; milestones are ordered by the project priority
(correctness → reproducibility → performance → geospatial → UX → deployment).

## Feasibility gates

| Gate | Criterion (measurable) | Status |
|---|---|---|
| **G0 Licensing** | MaxEnt reimplementation legally possible; default datasets commercially usable | **PASSED** — MaxEnt MIT since 3.4.0; CHELSA CC0 replaces WorldClim (ADR-0006) |
| **G1 Kernel equivalence** | Rust reproduces maxent.jar 3.4.4 on ≥10 controlled SWD cases: identical feature set/order and iterations, max \|Δλ\| ≤ 1e-6, max \|Δcloglog\| ≤ 1e-5, \|Δgain\| ≤ 1e-4 | **PASSED** — 11/11 (`benchmarks/phase3-results.md`) |
| **G2 Kernel performance** | Rust single-thread training not slower than warm-JIT jar on the same inputs | **PASSED** on synthetic cases (see results); re-check on real data |
| G3 Real-data equivalence | Same as G1 on ≥5 real species (GBIF CC0/CC-BY occurrences, CHELSA v2.1 predictors), identical SWD inputs | **PASSED** — 5/5 (Lacerta bilineata, Quercus ilex, Salamandra salamandra, Erinaceus europaeus, Rana dalmatina). NicheRS samples its own background (seeded), so Java's raster RNG need not be ported |
| G4 Geospatial correctness | No reprojection needed (native CHELSA grid, ADR-0010); CHELSA window reads match an independent stdlib-Python decoder exactly (BIO12 at Paris 658.8, Brittany 1037.6); area vs WGS84 reference ≤ 1 km² on the whole ellipsoid | **PASSED** for the MVP scope (EPSG:4326 inputs only); reprojection of user rasters remains future work |
| G5 End-to-end MVP | Definition of Done (master prompt Phase 7) items 1–14 demonstrably met | see `docs/mvp-status.md` |
| G6 Scale | 100 species × France @1 km on one 8-core machine within a measured, documented budget; no unbounded memory | **PASSED (10 species measured)** — 22.5 min, 477 MB peak; compute 3.5 s/species, GBIF retrieval dominates (`benchmarks/g6-france-batch.md`) |

**No-go triggers**: G3 fails without an explainable cause (then ship maxent.jar as subprocess engine
and keep Rust for orchestration); G4 fails for the allow-listed CRSs (then make the GDAL feature the
default build).

## Milestones

| # | Milestone | Content | Exit |
|---|---|---|---|
| M1 | Scientific kernel | ✅ features, regularization, sequential optimizer, `.lambdas`, cloglog/logistic, golden tests. ☐ raster-style random background (Java RNG), ☐ replicates/CV splits, ✅ permutation importance, ✅ response curves (golden vs jar), ☐ property tests | G1 ✅, G3 |
| M2 | Geospatial | `niche-geo`: COG read/write, CRS allow-list, alignment/resampling by layer type, masks, occurrence extraction, tiled inference, equal-area suitable area | G4 |
| M3 | CLI | `niche train/predict/batch` with project JSON + execution manifest; `doctor`, `cache`, `models verify` | reproducible rerun from manifest |
| M4 | API | Axum `/api/v1` (jobs, SSE events, cancel, results, species search, health), SQLite job store, bounded queue, restart recovery | interruption test passes |
| M5 | React UX | guided setup → run → results map, threshold, validation and uncertainty panels, exports (design docs) | UX acceptance criteria in `architecture/ux-specification.md` |
| M6 | Embedded frontend | memory-serve embedding, dev override, build-ID check | single binary serves UI + API |
| M7 | Optimization | profile; parallel folds/species; shared env matrices; tile streaming | G6 |
| M8 | WASM (optional) | only with a measured browser-side need | ADR-0005 revisit |
| M9 | Engine 2 | ✅ (opt-in, 2026-10-20) down-sampled Random Forest (`crates/niche-rf`) and MaxEnt + RF rank ensemble, `model.algorithm`; MaxEnt stays the default. ☐ comparison on real species, ☐ ecologists' review | ADR-0002, ADR-0017 (*Proposed*) |

## Science v1 (2026-10-20, opt-in, pending ecologists' review)

Programme and verification record: `science-v1-plan.md`. Every item is off by default (except the
ADR-0011 cleaning rule, which only affects GBIF search-API records flagged as generalised) and its
ADR is *Proposed*.

| Item | Content | ADR | Status |
|---|---|---|---|
| S1 Generalised coordinates | publisher-generalised records excluded unless their precision is within the limit | ADR-0011 | ✅ implemented (search API); ☐ flags for downloads/mirror |
| S2 Sampling bias | target-group background (mirror effort grids, format 2), effort layer, bias diagnostic, distance thinning | ADR-0013, ADR-0114 | ✅ opt-in; ☐ effort grids on the hosted mirror (format-2 refresh) |
| S3 Evaluation | null models with an evidence cap, grid blocks sized by autocorrelation, AOA | ADR-0014 | ✅ opt-in |
| S4 Predictors | Copernicus DEM terrain, ESA WorldCover fractions, distance to water | ADR-0015 | ✅ opt-in; ☐ CORINE/OSO, soils, year filter |
| S5 Future climate | CHELSA CMIP6 projections, change classes, GCM agreement, future MESS | ADR-0016 | ✅ opt-in; ☐ dispersal and land-cover scenarios |
| S6 Second engine | = M9 | ADR-0017 | ✅ opt-in |
| S7 Own field data | CSV import, own presences, independent presence–absence validation, *Validated* level | ADR-0108 | ✅ opt-in; ☐ XLSX/GeoPackage/GeoNature, CLI import |
| S8 Decision support | survey priority, footprint overlay, richness/hotspots, comparison of studies | ADR-0018 | ✅ on demand; ☐ CLI command |

## Next steps (in order, 2026-10-20)

1. **Ecologists' review** of ADR-0009 (still pending) and of every Science v1 ADR; the list of
   choices to review is in `science-v1-plan.md` ("What still needs the ecologists") and, in
   French, the design notes.
2. **Real-species validation campaign**: run the new options on species Natural Solutions knows
   well, with independent presence–absence data where available — random vs target-group
   background, climate-only vs climate + land cover (the design notes §6), MaxEnt vs
   RF vs ensemble, null models and AOA vs MESS, survey priority; measure national-extent costs
   instead of extrapolating them.
3. **Mirror refresh to format 2** on the hosted instance (effort grids) and measurement of the
   real `fr-es` sizes; then the target-group path on real data.
4. **Licence confirmations**: CHELSA (WSL) for the CMIP6-based files; a lawyer for the Copernicus
   DEM GLO-90 licence; DWCA/SQL GBIF downloads to obtain generalisation flags.
5. **Decide defaults by ADR**: only after 1–2, a new ADR amending ADR-0009 for each method that
   becomes a default (with a measured before/after on real species).

The earlier "immediate next steps" of 2026-10-15 (G3 real-data golden cases, spatial block CV
with Boyce/omission, `niche-geo` spikes) are done, see the gates above and `mvp-status.md`;
optimizer profiling is not tracked here.
