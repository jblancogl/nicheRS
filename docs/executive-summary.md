# NicheRS — Executive summary

> **Update 2026-10-20 (Science v1).** The MVP's main scientific weaknesses for consultancy use —
> sampling bias, climate-only predictors, no independent validation, a map per species rather
> than an answer to the client's decision — are now addressed by **opt-in** methods in Expert mode
> (`science-v1-plan.md`): target-group background, distance thinning and a bias diagnostic
> (ADR-0013); exclusion of publisher-generalised coordinates (ADR-0011, the only rule applied by
> default, to GBIF search-API records only); null-model significance, autocorrelation-sized grid
> blocks and an area of applicability (ADR-0014); Copernicus DEM, ESA WorldCover and distance-to-
> water predictors (ADR-0015); CHELSA CMIP6 future projections with GCM agreement and future
> extrapolation (ADR-0016); a down-sampled random forest and a MaxEnt + RF ensemble (ADR-0017);
> import of the organisation's field data with independent presence–absence validation and a
> *Validated* level (ADR-0108); survey priority, project-footprint overlay, multi-species
> richness and study comparison (ADR-0018). Every ADR is *Proposed — pending review by Natural
> Solutions ecologists*; defaults are unchanged (the default demo is byte-identical, the MaxEnt
> kernel untouched, 261 tests pass). Measured so far, on synthetic data or small real windows
> only: e.g. 20 null models × 5 folds 2.8 s on the demo; a 500-tree forest fits in 0.04 s and
> predicts 1 M cells in 4.65 s; one CMIP6 layer window of 1° costs ≈ 3.2 MB of range reads;
> a Provence stack with slope, tree cover and distance to water builds in 11.4–12.7 s on first
> use. **Not verified**: no new method has been validated on a real species; effort grids have
> not been built from a real mirror (the hosted mirror is format 1 until its next refresh);
> national-extent costs are extrapolated; the CMIP6-file licence (CHELSA/WSL) and the Copernicus
> DEM licence (lawyer) await confirmation. Status and gaps: `mvp-status.md`; next steps:
> `roadmap.md`.

> **Update 2026-10-15 (product build).** The MVP is implemented: a single 12 MB executable serves the
> web application and API, ingests GBIF + CHELSA data, fits tuned MaxEnt models with spatial
> cross-validation, and reports suitability, uncertainty, extrapolation, suitable-conditions area
> and an evidence level. All 14 Definition-of-Done items are met (`mvp-status.md`). Gates G0–G6
> passed (`roadmap.md`): 16/16 cases reproduce maxent.jar 3.4.4 including 5 real species; a
> 10-species batch over France at 1 km runs in 22.5 min / 477 MB, of which modelling is ≈ 3.5 s
> per species (the rest is GBIF retrieval, now cached). The scientific defaults that were open
> questions are decided in ADR-0009 (evidence levels, thresholds, cleaning, selection) and
> ADR-0010 (native grid, exact areas, land mask — CHELSA has values over the ocean).
> The sections below are the original Phase 0 summary.

## Original Phase 0 summary (research + Phase 3 feasibility)

Date: 2026-10-15. Audience: Natural Solutions technical and scientific leads.
Detail lives in `docs/research/01–14`, `docs/architecture/` (incl. 16 ADRs), `docs/design/`,
`docs/benchmarks/` and `docs/roadmap.md`.

## 1. Overall feasibility

**Feasible, with conditions.** The scientific core — a MaxEnt engine that reproduces the reference
`maxent.jar` 3.4.4 — has been demonstrated in Rust (gate G1 passed: 11/11 controlled cases).
The main remaining risks are not computational: they are data licensing, honest communication of
uncertainty, and geospatial correctness without GDAL. None is a blocker; each has a mitigation and
a measurable gate (`roadmap.md`).

## 2. Recommended architecture

One executable per platform (Rust): Axum serves the REST API, SSE progress and the embedded React
UI; numerical work runs on a bounded Rayon pool, never on the async runtime; state in SQLite (WAL,
`synchronous=FULL`) plus a content-addressed artifact store; hexagonal boundaries with a pure,
I/O-free modelling kernel. React + TypeScript + Vite + Tailwind v4 + shadcn/ui + MapLibre GL JS,
map-centred workspace with a guided setup and an expert mode. Local, server and batch modes share
the same engine, config semantics and formats (twelve-factor, adapted).
See `architecture/system-architecture.md`.

## 3. Is Rust justified?

**Yes, but not for raw speed claims.** The reasons are: a single dependency-free binary, control of
numerical determinism, in-memory sharing of environmental data across many species, permissive
licensing (maxnet/elapid depend on GPL glmnet), and a strong web/server ecosystem (Axum/Tokio).
Measured on the synthetic suite, single-threaded Rust training is 1.5–2.2× faster than the
warm-JIT jar's optimizer and uses 7–35× less peak memory (JVM untuned) — a useful margin, measured
on this machine and these datasets only, not a general claim. Go offered no evidence-backed advantage.

## 4. Should MaxEnt be ported?

**Yes, path-faithfully — and it now is, for SWD training.** Implementing the MaxEnt objective is not
enough: the jar stops after damped greedy coordinate steps, so only a port of its exact algorithm
(feature generation, regularization tables, stale-expectation bookkeeping, tie-breaking) reproduces
its models. The Rust port matches the jar's feature set, order and iteration count exactly, with
λ differences ≤ 2·10⁻¹¹ and prediction differences ≤ 5·10⁻¹⁰ (`benchmarks/phase3-results.md`).
Not yet ported: raster background sampling RNG, replicates/CV, permutation importance, jackknife,
response curves, bias files. maxent.jar remains a test oracle, not a shipped component.

## 5. Does WASM add value?

**Not for the MVP** (ADR-0005). No identified browser-side operation needs it; threshold changes and
point queries are cheap in TypeScript or on the server. The kernel stays I/O-free so a WASM build
remains possible.

## 6. Single-binary distribution

**Feasible with an explicit definition**: one executable per OS/architecture, no Java/R/Python/Node
at runtime, plus a writable data directory (datasets, cache, SQLite, outputs). Achievable in pure
Rust only with a restricted, validated set of formats and CRSs (GeoTIFF/COG, GeoJSON, GeoPackage;
EPSG:4326/3035/2154/3857 initially); full GDAL support is an optional static-linked build feature
(ADR-0003). Code signing: Windows Artifact Signing; macOS needs a notarised `.pkg`/`.dmg`.

## 7. Main risks

| Risk | Type | Mitigation |
|---|---|---|
| Users read suitability as probability / occupied range | scientific | ADR-0007 wording, evidence levels, area bands, no confidence % |
| Sampling bias in GBIF/citizen-science data | scientific | bias diagnostics, target-group background option, provenance and exclusion reports |
| Silent extrapolation | scientific | MESS/AOA layers, clamping, hatched overlays |
| Default data not licensed for commercial use (WorldClim, ESDAC, GBIF CC BY-NC) | legal | CHELSA (CC0), SoilGrids, BY-NC exclusion by default (ADR-0006) |
| Port diverges on real data (ulp-level tie flips) | scientific | gate G3 on real species; golden tests in CI |
| Pure-Rust CRS/COG crates immature | technical | allow-list, spikes, GDAL feature fallback and CI cross-check |
| Local server attacked via DNS rebinding | security | loopback bind, Host/Origin checks, per-launch token |
| Server mode exposed without auth | security | server mode unsupported until auth + isolation exist |

## 8. Proposed MVP

One user, local mode: pick a species (GBIF/TAXREF), draw or import a study area in France, use
CHELSA + DEM + land-cover predictors at 1 km (100 m later), review data quality and exclusions,
run MaxEnt with per-species tuning under spatial block CV, explore the suitability map with
threshold control, Boyce/omission, uncertainty and extrapolation layers, compute a suitable-area
band in EPSG:3035, export COG + GeoJSON + execution manifest + methods text. Batch over a species
list via CLI. Definition of Done = master prompt Phase 7.

## 9. Phased plan

M1 kernel (✅ core, remaining items listed) → M2 geospatial → M3 CLI with manifests → M4 API/jobs →
M5 React UX → M6 embedded UI → M7 optimisation → M8 WASM only if justified → M9 second engine
(down-sampled Random Forest). Details and exit criteria: `roadmap.md`.

## 10. Go / no-go criteria

| Gate | Status |
|---|---|
| G0 licensing (engine + default data) | **GO** |
| G1 kernel equivalence on controlled data | **GO** (11/11) |
| G2 kernel not slower than reference | **GO** on synthetic data |
| G3 equivalence on ≥5 real species with raster backgrounds | pending — **no-go** ⇒ ship jar as subprocess engine |
| G4 geospatial accuracy vs PROJ/GDAL | pending — **no-go** ⇒ GDAL build becomes default |
| G5 MVP Definition of Done | pending |
| G6 100 species × France @1 km within a measured budget | pending |

## Open questions for Natural Solutions (1–2 decided in ADR-0009; 3–5 remain open)

1. Evidence-level cut-offs (data adequacy, Boyce, extrapolation share) — needs ecologist agreement.
2. Default suitability threshold rule for reports (P10 vs max SSS vs MTP band only).
3. French presence-absence datasets available for independent validation ("Supported" level).
4. Is exact maxent.jar equivalence a product requirement, or is "same objective" acceptable for new
   features (e.g. parallel tuning)? This decides how far the port must follow Java quirks.
5. Server mode: who would host it, and with which authentication?
