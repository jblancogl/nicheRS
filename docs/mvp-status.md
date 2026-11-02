# MVP status against the Definition of Done (master prompt, Phase 7)

Date: 2026-10-15. Each item lists how it was verified. "Verified" means executed in this
repository on this date, not inferred.

| # | Criterion | Status | Verification |
|---|---|---|---|
| 1 | Start the application from a standalone executable | Done | `scripts/build-release.sh` → `target/dist/niche` (12 MB, stripped). `ldd`: libc, libm, libgcc_s only. No Java/R/Python/Node at runtime. |
| 2 | The executable serves the React application | Done | UI embedded with rust-embed; SPA fallback; screenshots in `docs/design/ui-review-2026-10-15.md`; `scripts/ui_smoke.py` passes. |
| 3 | Select a supported species and territory | Done | GBIF backbone search; draw / GeoJSON import / presets; multi-species batches. |
| 4 | Real occurrence and environmental data ingested | Done | GBIF occurrence API (spatially stratified when capped, 429-aware), CHELSA v2.1 via HTTP range reads (cross-checked against an independent decoder), Natural Earth land mask. 6 real-species runs + a 10-species France batch. |
| 5 | A scientifically defined model can be trained | Done | MaxEnt port reproducing maxent.jar 3.4.4 on 16/16 cases (11 synthetic + 5 real), `docs/benchmarks/phase3-results.md`; tuning and selection rules in ADR-0009. |
| 6 | Model validation results available | Done | Spatial block CV per fold: continuous Boyce, OR10, ORmtp, presence-background AUC; tuning table; evidence level with reasons. |
| 7 | Habitat suitability predictions generated | Done | Cloglog suitability COG, uncertainty (fold SD) COG, MESS COG; XYZ tiles; point sampling. |
| 8 | Suitable habitat area with documented methodology | Done | Exact WGS84 ellipsoidal cell areas (tested against the WGS84 total surface), band across P10 / max SSS / MTP thresholds; ADR-0009/0010; methods report. |
| 9 | Maps explored interactively | Done | MapLibre workspace: layers, legends, opacity, threshold control, click-to-inspect, occurrences overlay. |
| 10 | Results exported | Done | suitability/uncertainty/mess GeoTIFF (COG layout), occurrences GeoJSON, `.lambdas`, `report.md`, printable `report.html` (maps, response curves, tables; print to PDF), `response_curves.csv`, `manifest.json`. |
| 11 | Assumptions, limitations, provenance accessible | Done | UI warnings and data tab; `report.md` (limits section); execution manifest (sources, URLs, seeds, spec hash, checksums, method choices). |
| 12 | Computational pipeline covered by automated tests | Done (backend); partial (UI) | `cargo test --workspace`: unit tests (kernel, eval, geo, data), golden tests vs maxent.jar, offline end-to-end demo (truth recovery ρ > 0.8), interruption/recovery; live network tests (`--ignored`); UI smoke test script (not yet in CI). |
| 13 | Reference benchmarks reproducible | Done | `scripts/gen_synthetic.py`, `scripts/compare_maxent.py`, real specs in `tests/real-specs/`; methodology in `docs/benchmarks/methodology.md`. |
| 14 | No misrepresentation of scores as probabilities or unsupported confidence | Done | Wording enforced in API texts, legends, reports (ADR-0007); `ui_smoke.py` checks the results page wording; no confidence percentages anywhere. |

## Science v1 (2026-10-20)

Programme `docs/science-v1-plan.md` (branch `feat/science-v1`; summary, decision list and "not
verified" list at the top of its progress log). The 14 items above are unchanged; Science v1 adds
**opt-in** methods in Expert mode, each behind an ADR that is *Proposed — pending review by
Natural Solutions ecologists*. With the default specification, the synthetic demo's outputs and
`spec_sha256` were checked identical before and after every phase, and the MaxEnt kernel was not
modified.

| Area | What exists | ADR | Default |
|---|---|---|---|
| Generalised coordinates | Search-API records flagged as generalised by the publisher are excluded unless their stated uncertainty is within the limit; counted, warned, SINP sentence in reports | ADR-0011 | **on** (only search-API flagged records; downloads and the mirror get `generalisation_flags_unavailable`) |
| Sampling bias | Target-group background from the regional mirror's effort grids, effort layer, bias diagnostic, distance thinning, bias-matched null models | ADR-0013 | off |
| Evaluation | Null-model significance (with an evidence cap), spatial grid blocks sized by autocorrelation, area of applicability (AOA) | ADR-0014 | off |
| Predictors | Copernicus DEM (elevation, slope, roughness), ESA WorldCover fractions, distance to water | ADR-0015 | none selected |
| Future climate | CHELSA CMIP6 projections (3 periods × 3 SSPs × 5 GCMs), change classes with GCM agreement, future MESS | ADR-0016 | off |
| Second engine | Down-sampled random forest and a MaxEnt + RF rank ensemble | ADR-0017 | MaxEnt |
| Own field data | CSV import (WGS84, Lambert-93, LAEA, UTM), presences in training, independent presence–absence validation, *Validated* level | ADR-0108 | off |
| Decision support | Survey priority, project-footprint overlay, multi-species richness/hotspots, comparison of two studies | ADR-0018 | on demand only |

Verified per phase (`cargo fmt`, `clippy -D warnings`, `cargo test --workspace` — 261 passed,
0 failed, 21 ignored after phase 8 —, web typecheck/lint/build, demo bit-identity, live reads of
real CHELSA/CMIP6/DEM/WorldCover windows over small Provence boxes, one live search-API study of
*Testudo hermanni*). **Not verified**: none of the new methods has been run end to end on a real
species with a conclusion drawn from it; effort grids have not been built from a real region
archive (the hosted mirror is format 1 until its next refresh). `scripts/ui_smoke.py` passes
end to end since the QA of 2026-10-20 (every Science v1 option on demo studies, EN/FR/ES).
Details in the plan.

## Scale (gate G6)

See `docs/benchmarks/g6-france-batch.md`.

## Known gaps (post-MVP backlog)

Updated 2026-10-20 (Science v1).

Closed or addressed as opt-in since the MVP (each pending ecologists' review, see above):
bias correction (target-group background, thinning), independent presence–absence validation
with the organisation's own data, a second engine (random forest, ensemble), terrain / land
cover / water predictors, future climate, decision-support outputs, CRS transforms for imported
points (Lambert-93, LAEA Europe, UTM 28N–32N). Server mode: since 2026-10-16 `--auth accounts`
provides organisations, invitation links, sessions and quotas (ADR-0105); without it, bind to
loopback or put behind an authenticating proxy.

Remaining:

- **Validation on real species** of every Science v1 method (random vs target-group background,
  climate-only vs climate + land cover, RF vs MaxEnt, projections, survey priority), then a
  decision by ADR on which become defaults.
- **Effort grids in production**: the hosted mirror needs its format-2 refresh; real `fr-es`
  sizes are unmeasured.
- **Generalisation flags** for GBIF downloads and the mirror (DWCA or SQL download formats).
- **Predictor rasters**: no import of user-supplied predictor rasters; the GeoTIFF reader accepts
  EPSG:4326 grids only (no raster reprojection — the new CRS transforms apply to points).
- **Predictors not done**: soils, CORINE Land Cover / OSO, categorical land cover, neighbourhood
  fractions, river networks, year filter on records when land cover is used; checksums of cached
  DEM/WorldCover tiles in the manifest.
- **Own data**: CSV only (no XLSX, GeoPackage, Shapefile, GeoNature connector), no Lambert II or
  overseas systems, no CLI import (`niche run --occurrences`); survey ids not used; imperfect
  detection of absences not modelled.
- **Future climate**: no dispersal or land-cover scenarios, no future AOA/MOP, no evidence
  statement for projections; CMIP6 licence to confirm with CHELSA.
- **Random forest**: no member-disagreement layer, no seed-sensitivity report, no categorical
  subset splits.
- **Decision support**: no CLI command, no stratified sampling within priority strata, no
  footprint buffers, no corrected stacked richness.
- **Licences**: lawyer's confirmation of the Copernicus DEM GLO-90 licence advisable.
- No TOML config file / `niche config show`; no SBOM generation; no Windows/macOS builds tested.
- UI: no axe accessibility audit, no vertex editing, single polygon import; `scripts/ui_smoke.py`
  not in CI (passes locally, QA 2026-10-20).
