# NicheRS engineering principles (cross-cutting)

Status: Proposed · Date: 2026-10-15 · Author: architecture research agent · References: `[En]` ids, see end of file and `docs/research/refs/eng.md`.

Tags: [FACT] verified from a source read on 2026-10-15 · [ASSUMPTION] · [OPEN] · [REC] recommendation · [UNVERIFIED – prior knowledge].

## Scope

The product owner set 10 cross-cutting principles. For each one this document gives its relevance to NicheRS, the concrete problem it solves, implementation cost, milestone (MVP or later) and how compliance is tested. Governing rule: **complexity must be justified by measurable scientific or user value**. No abstraction, infrastructure or dependency is recommended only to claim adherence. Detailed decisions are in:

- `execution-manifest.md` (manifest schema v1)
- `architecture-decisions/ADR-0201-hexagonal-boundaries.md`
- `architecture-decisions/ADR-0202-openapi-contract-first.md`
- `architecture-decisions/ADR-0203-execution-manifest.md`
- `architecture-decisions/ADR-0204-ogc-standards-adoption.md`

Out of scope: choosing the raster I/O stack (GDAL vs pure Rust), the MaxEnt solver design and UI design. Other research documents cover these. This document only sets constraints on them.

## Findings: summary table

| # | Principle | Relevance | Problem solved | Cost | Milestone | Compliance test |
|---|---|---|---|---|---|---|
| 1 | DDD + hexagonal | High | Science code can be tested, reused (CLI/server/WASM) and audited without HTTP, DB or GDAL | Low if limited to crate boundaries and 4 ports; high if trait-per-thing | MVP (crate split day 1) | CI dependency-graph check; `niche-core` builds for `wasm32-unknown-unknown` |
| 2 | FAIR and reproducible | Critical (priority #2) | "Which data, settings and version produced this map?" for client reports and audits | Medium: hashing, seeds, schema | MVP: JSON manifest v1. Later: RO-Crate export | Schema validation; re-run from manifest gives identical hashes |
| 3 | Contract-first OpenAPI 3.1 | High (UI and API ship together, server mode later) | TS/Rust type drift; undocumented breaking changes | Low (utoipa + openapi-typescript) | MVP | Spec snapshot test; TS typecheck on regenerated client |
| 4 | Scientific golden tests | Critical (priority #1) | Silent numerical regressions; unproven equivalence with maxent.jar | Medium to high (fixtures, tolerance design) | MVP for MaxEnt core | Golden suite in CI with versioned tolerances |
| 5 | Observability and profiling per stage | High (priority #3: measurable perf) | Unknown bottlenecks at 100s of species; OOM on large areas | Low (tracing spans); medium (alloc stats) | MVP spans and timings; alloc stats behind a feature | Test layer asserts required spans and fields; `niche benchmark` |
| 6 | Security by design | Medium local, high server | Untrusted geodata parsers, localhost CSRF/DNS rebinding, DoS, sensitive-species leakage | Medium | MVP: L1-equivalent and resource limits. Server: ASVS L2 | Security regression tests, cargo-deny, fuzzing later |
| 7 | Agent-driven engineering | Medium (team velocity) | Agents breaking invariants (goldens, crate rules, spec) | Very low | MVP | CI runs the commands listed in AGENTS.md |
| 8 | Offline-first and progressive enhancement | High (field and consultancy laptops) | No network means no work; CDN or basemap dependence | Low to medium (offline basemap) | MVP | E2E demo run with networking disabled |
| 9 | OGC standards | High for outputs, low for APIs | Outputs open in QGIS/ArcGIS and catalogues without conversion | Low (COG, GeoJSON, STAC Item); medium (OGC API Processes) | MVP: GeoTIFF/COG, GeoJSON, GeoPackage import, STAC Item sidecar. Later: OGC API Processes | COG validation, STAC JSON Schema validation |
| 10 | Zero-config experience | High (non-data-scientist users) | First-run friction; legal risk of demo data | Low | MVP | Fresh-profile E2E: start, demo, map in 3 steps or fewer |

## 1. DDD + hexagonal architecture

**Relevance.** [REC] High. Priority #1 is scientific correctness. Correctness is only auditable if the algorithm can be exercised with plain in-memory data. The same engine must run in three deployment modes (local, server, batch, see `_BRIEF-12FACTOR.md`) and possibly in WASM.

**Problem solved.** It prevents HTTP DTOs, SQLite rows, GDAL dataset handles or Tokio types from leaking into MaxEnt and evaluation code. Without that separation, golden tests need a database and a raster library, and a WASM build becomes impossible.

**Design.** See ADR-0201 for the full decision.

- [REC] The domain is a set of crates with **no I/O dependencies**: `niche-core` (types and invariants), `niche-maxent` (algorithm) and `niche-eval` (metrics and partitioning). They work on in-memory grids (`GridSpec` plus `ndarray`/`Vec<f32>`) and point tables.
- [REC] Adapters: `niche-geo` (raster/vector read, write and reproject), `niche-sources` (GBIF and file import), `niche-store` (SQLite and filesystem artifacts).
- [REC] Application layer: `niche-app` holds use cases such as RunExperiment, job lifecycle and manifest assembly.
- [REC] Drivers: `niche-api` (Axum), `niche-cli` (clap).
- [REC] Composition root: `niche` (binary, embeds the frontend).
- [REC] Only **four ports (traits)** at MVP, each with two or more real implementations or a test double that has value:

| Port | MVP implementations | Why it is a trait |
|---|---|---|
| `OccurrenceSource` | GBIF API, file import (CSV/GeoPackage), fixture | Offline-first: file import must always work; tests must not use the network |
| `EnvironmentSource` | local raster files, remote catalogue download into cache | Same reason |
| `ArtifactStore` | local FS (atomic writes); in-memory for tests | Object storage later (12F-IV) |
| `JobRepository` | SQLite; in-memory for tests | Crash recovery tests (12F-VI/IX) |

**Where NOT to add traits.** [REC]

- Do not add an `Algorithm` trait before a second algorithm exists. Use a concrete MaxEnt module. When a second algorithm arrives, a closed `enum ModelSpec` is preferable to `dyn Trait`, because the manifest must serialize the exact config.
- Do not add traits for feature classes (linear, quadratic, product, hinge, threshold), metrics or partitioning strategies. These are closed sets: use enums and functions.
- Do not add a repository-per-entity, unit-of-work, CQRS or event bus.
- Do not add a logging or metrics abstraction. Use `tracing` directly.
- Do not add a trait over the raster library inside the domain. The domain never sees it, because adapters convert to `Grid`.
- Do not add a `Clock` trait. Timestamps belong to the app layer. Randomness is a `u64` seed value passed explicitly, not an injected RNG trait.

**DDD scope.** [REC] Adopt the *ubiquitous language* and *bounded contexts* parts (glossary: occurrence, background, presence-only, predictor, study area, feature class, regularization multiplier, partition, suitability, threshold, projection). Skip tactical-DDD ceremony (aggregates, domain events) unless a real invariant needs it. Evans' tactical patterns target transactional business systems [E28, UNVERIFIED – prior knowledge]. NicheRS is a computation pipeline.

**Cost.** Low: about 8 crates in one Cargo workspace and about 4 traits. The risk of over-splitting is controlled by the "no trait without two implementations" rule.

**Milestone.** MVP. Splitting crates later is expensive once dependencies have leaked.

**Compliance test.**
1. CI script over `cargo metadata` asserts that the normal (non-dev) dependency closure of `niche-core`, `niche-maxent` and `niche-eval` contains none of `tokio`, `axum`, `hyper`, `reqwest`, `rusqlite`/`sqlx`, `gdal*`, `proj*` or any adapter crate (`niche-geo`, `niche-sources`, `niche-store`). A `clippy.toml` `disallowed-methods` list forbids `std::fs::*` and `std::net::*` in those crates.
2. `cargo build -p niche-core -p niche-maxent -p niche-eval --target wasm32-unknown-unknown` in CI. This also proves the optional WASM path.
3. Golden tests (section 4) run against the domain crates only.

## 2. FAIR and reproducible science

**Relevance.** Critical. Reproducibility is priority #2. Consultancy deliverables (impact studies) may be challenged, and the client or authority must be able to trace a map back to its inputs. [ASSUMPTION] There is no formal regulatory requirement verified here.

**Problem solved.** It answers "exactly which occurrences, predictors, settings, seed and software version produced this raster, and can I get it again?"

**Standards reviewed.**

- **FAIR principles** (Wilkinson et al. 2016) [E30, UNVERIFIED – prior knowledge]. NicheRS is a tool, not a repository. The relevant parts are R1.2 (provenance) and I1/I2 (shared vocabularies), applied to outputs.
- **ODMAP v1.0** (Zurell et al. 2020, *Ecography* 43:1261–1277, doi:10.1111/ecog.04960). It is an SDM reporting protocol with five sections: Overview, Data, Model, Assessment, Prediction. The ODMAP web app exports CSV and DOCX [E15, FACT]. ODMAP is aimed at human reporting. [REC] NicheRS should *pre-fill* an ODMAP-structured report from the manifest (later milestone), because consultants must write methods sections anyway. That is measurable user value.
- **W3C PROV-O** (Recommendation, 30 April 2013): Entity, Activity, Agent; `used`, `wasGeneratedBy`, `wasAssociatedWith`, `wasDerivedFrom` [E16, FACT]. [REC] Use these concepts as *names* in the manifest (inputs = used, outputs = generated, `parent_runs` = derivation). Do not emit RDF in the MVP.
- **RO-Crate 1.3** is the current released spec [E5, FACT]. The **Workflow Run RO-Crate** profiles are Process Run Crate, Workflow Run Crate and Provenance Run Crate (Leo et al. 2024, *PLoS ONE*, doi:10.1371/journal.pone.0309210) [E4, FACT]. Process Run Crate ("execution of one or more tools contributing to the same computation") fits a NicheRS run.

**Decision.** See ADR-0203 and `execution-manifest.md`. [REC] The MVP emits one versioned, JSON-Schema-validated `manifest.json` per run (schema `nichers.run-manifest/1`). It records software build and lockfile hash, inputs with SHA-256, licence and source DOI (for example the GBIF download DOI), preprocessing steps, model config, seed scheme, validation strategy, output artifacts with SHA-256, and per-stage timings. **RO-Crate export comes later** as a mechanical mapping to a Process Run Crate (`ro-crate-metadata.json` plus artifacts in a zip). The trigger is a user request or a data-repository deposit (for example Zenodo). The manifest must not contain raw coordinates, only hashes and counts. This aligns with 12F-XI and sensitive-species handling.

**Cost.** Medium. SHA-256 hashing of inputs and outputs is I/O-bound; `sha2` 0.11.1 is MIT/Apache [E21, FACT]. Explicit seeding needs deterministic parallel reductions (for example a fixed chunking order in Rayon) to make "same manifest, same hashes" true. [ASSUMPTION] Bit-identity across OS/CPU is not guaranteed for floating point (FMA, SIMD, libm differences). Guarantee it on the same build and target, and guarantee tolerance-level equivalence elsewhere.

**Milestone.** MVP: manifest v1, `niche models verify` (re-hash artifacts) and `niche rerun <manifest>`. Later: RO-Crate export and an ODMAP pre-filled report.

**Compliance test.** Every run output is validated against the published JSON Schema (`jsonschema` crate 0.58.6 [E21, FACT]). The CI "rerun" test executes the demo, re-executes from the manifest, and asserts equal output SHA-256 on the same target. `niche models verify` detects a tampered artifact (negative test).

## 3. Contract-first API (OpenAPI 3.1)

**Relevance.** High. The React UI and the Rust API ship in one executable, but they are developed in different languages. Server mode exposes the API to third parties (scripts, QGIS plugins, later OGC facade).

**Problem solved.** It prevents TS/Rust type drift and unreviewed breaking changes.

**Evidence.**

| Option | Version (crates.io/npm, 2026-10-15) | OpenAPI | Notes |
|---|---|---|---|
| `utoipa` + `utoipa-axum` | 6.0.0 / 0.3.0 (2026-09-22), MIT/Apache, ~18 M recent downloads | 3.1 (docs list 3.1. The module doc says "Spec V3.2", an inconsistency to check) | Annotation/macro code-first; axum bindings [E17, FACT] |
| `aide` | 0.15.1 (2026-04-14), MIT/Apache, ~1 M recent | 3.1.0 only | Derives schemas via `schemars`, the same derive usable for the manifest JSON Schema [E18, FACT] |
| Hand-written YAML + server codegen | n/a | 3.1 | Strongest "contract-first" purity. Rust/axum server generators are less mature [UNVERIFIED – prior knowledge]. Two sources of truth need conformance tests |
| TS: `openapi-typescript` + `openapi-fetch` | 7.13.0 / 0.17.0 (2026-02-11), MIT | 3.0 and 3.1 | Types only, tiny runtime [E19, FACT; 3.1 per secondary source E31] |
| TS: `@hey-api/openapi-ts` | 0.99.0 (2026-06-22), MIT | — | Pre-1.0 and fast-moving; generates SDK [E19, FACT] |
| TS: `orval` | 8.43.0 (2026-10-14), MIT | — | Generates React Query hooks, mocks; heavier [E19, FACT] |

**Decision (ADR-0202).** [REC] Use **reviewed-contract, code-generated**: `utoipa` generates OpenAPI 3.1 from the Rust handler types. The generated document is **committed** as `api/openapi.json`, and that committed file is the contract reviewed in merge requests. The TS client types come from it via `openapi-typescript`, and calls go through `openapi-fetch`. Use TanStack Query hand-wrapping if needed rather than orval.

**Cost.** Low.

**Milestone.** MVP.

**Compliance test.**
1. Rust test `openapi_snapshot` serializes the spec and compares it to `api/openapi.json`. It fails on difference, with the fix command `cargo xtask openapi`.
2. The frontend CI step regenerates `src/api/schema.d.ts` and fails if `git diff` is non-empty, then runs `tsc --noEmit`.
3. Optional later: breaking-change diff against the last release tag with an OpenAPI diff tool (dev-only) [OPEN].
4. The spec is validated as OpenAPI 3.1 in CI.

## 4. Scientific golden tests

**Relevance.** Critical. "Correct" for a MaxEnt reimplementation means agreement with the reference implementation within justified tolerances on documented datasets.

**Reference datasets (MVP).** [FACT unless noted]

| Dataset | Content | Licence | Use |
|---|---|---|---|
| `bradypus` (maxnet R package) | 116 presences + 1000 background points × 14 predictors (13 numeric + categorical `ecoreg`) for *Bradypus variegatus*. Occurrences from Anderson & Handley 2001; predictors as in Phillips et al. 2006 | Package licence MIT (maxnet DESCRIPTION). Data-specific licence not separately stated [ASSUMPTION: covered by package licence] | Table-level training golden (no rasters needed) [E22] |
| `bradypus.csv.gz` (elapid) | Same species | elapid MIT [E24] | Cross-check of the table above |
| elapid test rasters (`test-raster-*.tif`, `test-point-samples.gpkg`) | Small synthetic rasters and points | MIT [E24] | Raster I/O and extraction goldens |
| Synthetic virtual species (generated, seeded) | Known true suitability function | Own, CC0 | Recovery tests: truth is known, which no jar comparison can provide |

**Reference implementation.** `reference/vendor/jar/maxent.jar`, version **3.4.4** (string found in `density/*.class`; manifest built 2020-11-19), SHA-256 `4c856e55412f70c5597b03cf9aaaf27e0782e0921f937262273b68bdcb8fee5e`, MIT licence [E23, FACT]. maxent.jar outputs are produced **offline by a dev script** (Java needed in dev/CI tooling only, never at runtime). They are committed as fixtures together with the jar hash and the exact command line.

**Comparison method.** [REC]

- **Custom tolerance comparisons** (`approx` 0.5.1 [E21]) for numeric outputs. They use a per-quantity tolerance table stored in `tests/golden/tolerances.toml`. Example quantities: lambdas (relative), raw/cloglog prediction per point (absolute plus Spearman rank), AUC (absolute), entropy, regularized training gain. [OPEN] Initial tolerance values must be *calibrated*, not invented. Measure jar-vs-jar variance under different thread counts and the maxnet-vs-jar differences, then set tolerances at a stated multiple of the observed variance. No equivalence threshold is asserted in this document.
- **`insta` 1.49.0** [E20] for *structural* snapshots: manifest shape with redacted timestamps and hashes, API JSON, metric tables. Floats go through `rounded_redaction(n)`. Insta is unsuitable as the primary numeric oracle: rounding is not tolerance, and values near a rounding boundary cause flaky diffs.
- **Rasters:** compare summary statistics, per-pixel max-abs and RMSE within tolerance, and a hash of the thresholded binary map. Do not store full float rasters for every case. Keep fixtures small (< 5 MB total) [REC].
- **Determinism test:** the same seed and thread count give bit-identical output on the same target.

**Policy for intentional output changes.** [REC]

1. Any change to golden fixtures or tolerances requires an MR labelled `science-change`, a written rationale, and approval by a domain reviewer (ecologist/modeller).
2. Bump `golden_suite_version` and the `niche-maxent` implementation version, both of which are recorded in every manifest.
3. Add a CHANGELOG "Scientific changes" entry with a before/after metrics table on all reference datasets.
4. Agents must never regenerate goldens unprompted (enforced in AGENTS.md and by CODEOWNERS on `tests/golden/`).

**Cost.** Medium to high, mostly tolerance calibration and fixture tooling.

**Milestone.** MVP for training and prediction on bradypus. Raster goldens and virtual-species recovery come with the first map output.

**Compliance test.** `cargo test -p niche-maxent --test golden` in every pipeline. The full extended suite runs nightly.

## 5. Observability and profiling per stage

**Relevance.** High. Priority #3 is *measurable* performance, and the product must scale to hundreds of species.

**Problem solved.** It locates bottlenecks per stage, predicts memory needs before an OOM, and shows users realistic progress and ETA.

**Design.** [REC]

- **Stages** (fixed vocabulary, also in the manifest): `ingest`, `features`, `train`, `validate`, `predict`, `raster_write`, `export`.
- **Spans:** one `tracing` span per stage with fields `job_id`, `run_id`, `species_key`, `stage`, `n_points`, `n_cells`, `n_features`. Durations come from span close events (`tracing` 0.1.44, `tracing-subscriber` 0.3.23 [E21]). Logs are JSON on stderr in server mode and human-readable locally (12F-XI). Coordinates are never logged.
- **Throughput:** derived per stage (points/s, cells/s, MB/s) and stored in `manifest.execution.stages[]`. Users see the same numbers in the UI ("prediction: 4.1 M cells in 12 s").
- **Memory:**
  - (a) Process peak RSS via `getrusage(RUSAGE_SELF).ru_maxrss` on Linux/macOS, and the platform equivalent on Windows (or the `memory-stats` crate). This is cheap and always on. Being process-wide and monotonic, it is useful per job only in batch mode.
  - (b) Per-stage allocated bytes and peak through a ~50-line counting `GlobalAlloc` wrapper behind Cargo feature `alloc-stats`. `stats_alloc` (0.1.10, last release 2022-03-30) is an alternative but stale [E21, FACT].
  - (c) Use `dhat` (0.3.3) for heap profiling in dev/bench builds only [E21].
- **CPU profiling:** `cargo flamegraph` (0.6.14) and `samply` (0.13.1, 2025-02-01) are dev tools and not dependencies [E21]. `criterion` (0.8.2) or `divan` (0.1.21) handle micro-benchmarks. `niche benchmark` (12F-XII) runs a fixed synthetic workload and prints a JSON report comparable across releases.
- **Not in MVP:** a Prometheus `/metrics` endpoint and OpenTelemetry export. Add them only for server mode when an operator needs them (12F-XI "where justified").

**Cost.** Low. Alloc stats are medium and feature-gated to avoid overhead on release builds.

**Milestone.** MVP: spans, timings, peak RSS, `niche benchmark`. Later: metrics endpoint.

**Compliance test.** A test subscriber layer collects spans during the demo run and asserts that each of the 7 stages appears with the required fields. A nightly `niche benchmark` compares against the stored baseline and flags regressions above a threshold set from observed run-to-run noise [OPEN: value].

## 6. Security by design

**ASVS version.** [FACT] OWASP ASVS **5.0.0** is the latest stable release, published 30 May 2025 per the GitHub release [E1, E2]. Its three levels are cumulative. The year comes from secondary sources [E2b], and the GitHub page omits the year.

**Target.** [REC] Local mode: ASVS 5.0 **L1** as an informal checklist. Server mode: **L2** before any multi-user deployment. Track this in `docs/security/asvs-checklist.md`, created when the server mode is scoped.

**STRIDE threat model (condensed).** [REC] STRIDE per Microsoft threat-modeling practice [E29, UNVERIFIED – prior knowledge].

| Threat | Local mode (loopback, single user) | Server mode |
|---|---|---|
| **S**poofing | A malicious web page in the user's browser calls `http://127.0.0.1:port` (CSRF, DNS rebinding). Mitigations: Host-header allow-list, Origin check on state-changing requests, a per-launch random token passed in the URL opened at startup | AuthN delegated to a reverse proxy (OIDC) or built-in sessions [OPEN]. ASVS L2 session controls |
| **T**ampering | Artifact or manifest edited on disk; detected by `niche models verify` (SHA-256) | Same, plus per-user storage isolation |
| **R**epudiation | Low relevance | Audit log of job submission/deletion with user id |
| **I**nformation disclosure | Sensitive-species coordinates in logs, manifests, crash reports. Rule: hashes and counts only | Cross-tenant access to occurrences. Authorization checks on every job and artifact id |
| **D**oS | Decompression bombs, huge raster dimensions, pathological polygons | Same, plus job flooding: per-user quotas and a bounded queue |
| **E**levation | Memory-unsafe native parsers (GDAL/PROJ, C/C++) fed untrusted files; zip-slip during archive extraction | Same. Consider parsing untrusted rasters in a sandboxed child process (later) |

**Controls (MVP).** [REC]

- Uploaded geodata is untrusted. Validate before parsing (magic bytes, declared size). Set limits: `NICHE_MAX_UPLOAD_MB`, max raster `width × height × bands`, max decompressed bytes, max vertices, max species per job.
- Run a memory pre-flight estimate against `NICHE_MAX_MEMORY_MB` and refuse the job early with an explanatory message.
- File-system hygiene: artifact paths are derived from ids only and never from user strings; canonicalize and check containment in `NICHE_DATA_DIR`; write atomically (temp file plus rename); use 0700/0600 permissions on Unix; reject archive entries with `..`, absolute paths or symlinks. `cap-std` 4.0.3 is available for capability-based FS [E21], but this is [REC] not needed for the MVP, because the containment checks are simpler.
- Supply chain: `cargo-deny` (0.20.2) for advisories, licences, bans and sources. It subsumes `cargo-audit` (0.22.2), so running both is redundant [E21; overlap UNVERIFIED – prior knowledge]. Add `npm audit` / lockfile pinning for the frontend at build time, and an SBOM via `cargo-cyclonedx` 0.5.9 (12F-II).

**Cost.** Medium. **Milestone.** MVP: limits, FS hygiene, localhost protections, cargo-deny. Server: ASVS L2, authN/Z, quotas. Later: fuzzing parsers with `cargo-fuzz`.

**Compliance test.** Regression tests: path-traversal upload, zip-slip archive, oversize raster header (refused before allocation), cross-origin POST to local API (403), a missing token (401). `cargo deny check` gates CI.

## 7. Agent-driven engineering (AGENTS.md)

**Evidence.** [FACT] AGENTS.md is an open Markdown format ("a README for agents"), stewarded by the Agentic AI Foundation under the Linux Foundation. It has no required fields and recommends project overview, build/test commands, code style, testing instructions and security considerations. Nested files are allowed: the nearest file wins, and explicit user prompts override it [E3].

**Content NicheRS's root AGENTS.md must contain.** [REC] Keep it under about 150 lines.

1. A one-paragraph product summary and the priority order (correctness > reproducibility > performance > geospatial > UX > deployment).
2. Exact commands: build, test, golden tests, lint (`cargo clippy -D warnings`, `cargo fmt --check`), `cargo deny check`, frontend `pnpm` scripts, `cargo xtask openapi`.
3. The crate map and dependency rules from ADR-0201, including the list of forbidden dependencies in domain crates.
4. Invariants agents must not break:
   - never edit `tests/golden/` or tolerances without being explicitly asked;
   - API changes require regenerating the spec and TS types;
   - manifest schema changes require a version bump;
   - no new dependency without a justification line in the MR;
   - no network in tests;
   - no real sensitive coordinates in fixtures or logs.
5. Where docs and ADRs live, and the glossary (ubiquitous language).
6. MR/commit conventions, including the `science-change` label.
7. Nested `AGENTS.md` only in `crates/niche-maxent/` (numerical conventions: f64 accumulation, deterministic reduction order) and `web/`.

[UNVERIFIED – prior knowledge] Claude Code reads `CLAUDE.md`. A one-line `CLAUDE.md` that references `AGENTS.md` avoids duplication.

**Cost.** Very low. **Milestone.** MVP (create with the repo skeleton).

**Compliance test.** A CI job extracts the fenced command blocks from AGENTS.md and runs them (or checks that each command is listed in `justfile`/`xtask`), so that stale instructions fail CI.

## 8. Offline-first and progressive enhancement

**Relevance.** High. Ecologists work on laptops, on site, and behind restrictive client networks. 12F tension (3) is offline-first vs remote backing services.

**Design.** [REC]

- Everything after data acquisition works with no network: training, validation, prediction, map viewing, export.
- Acquisition has an offline path (file import of occurrences CSV/GeoPackage and predictor GeoTIFFs) and an online enhancement (GBIF query, predictor catalogue download).
- Downloads go to `NICHE_CACHE_DIR`, content-addressed by SHA-256 and recorded in the manifest, so a cached download is reusable offline and auditable.
- The UI bundles all JS, CSS and fonts. No CDN.
- Basemap: the default is an offline low-zoom vector basemap (Natural Earth is public domain [UNVERIFIED – prior knowledge]) packaged as a single PMTiles file or rendered at small size. An online raster/vector basemap is used only if the user configures a URL. [OPEN] Measure the binary-size cost and decide between embedding and a first-run download.
- The UI shows the network state explicitly ("GBIF unavailable — import a file instead") rather than failing silently.

**Cost.** Low to medium (basemap packaging). **Milestone.** MVP.

**Compliance test.** The CI E2E job runs the zero-config demo flow inside a network-less namespace (`unshare -n` or a container with `--network none`) and asserts that the map and report are produced. A unit test asserts that the frontend bundle contains no absolute `http(s)://` asset URLs.

## 9. OGC standards

**Verified status** [FACT, E6–E14]:

| Standard | Version / doc | Status |
|---|---|---|
| OGC GeoTIFF | 1.1, 19-008r4 | Approved 2019-09-10 |
| OGC Cloud Optimized GeoTIFF | 1.0, 21-026 | Approved 2023-05-08, published 2023-07-14 |
| STAC spec | 1.1.0 | Released 11 Sep (GitHub release, 2024 [ASSUMPTION on year]) |
| OGC API – Processes – Part 1: Core | 1.0.0, 18-062r2 | Approved 2021-08-23, published 2021-12-20. Part 1 Core 2.0 was in progress with a June 2025 target in a 2024 tracker; approval not confirmed [OPEN] |
| OGC API – Features | Part 1 Core 1.0.1 (17-069r4, = ISO 19168-1:2020), Part 2 CRS 1.0.1, Part 3 Filtering 1.0.0; Parts 4–5 draft | Approved |
| OGC API – Tiles – Part 1: Core | 1.0, 20-057 | Approved |
| OGC API – Maps – Part 1: Core | 1.0.0, 20-058 | Listed as standard |
| GeoPackage | 1.4.0, 12-128r19 | Current |

**Adoption (ADR-0204).** [REC]

- **MVP, files:**
  - Suitability, uncertainty and binary rasters as GeoTIFF 1.1 laid out as **COG 1.0** (tiled, with overviews), Float32 plus nodata, with an explicit CRS.
  - Study areas and occurrences as **GeoJSON** (RFC 7946, WGS84) for API/UI exchange.
  - **GeoPackage 1.4** import, plus export of occurrences and study area for QGIS users.
  - A **STAC 1.1.0 Item JSON sidecar** per output raster (static file, no STAC API). It links to the manifest and carries projection and raster info. It costs a little JSON but makes outputs catalogue-ready and machine-discoverable (FAIR F/I).
- **Later (server mode):** an **OGC API – Processes 1.0 facade** (`/processes`, `/jobs`, `/jobs/{id}/results`) over the existing job model, *only when* an integration needs it (QGIS/WPS-style clients, institutional platforms). The job model is designed now so that it maps cleanly: async execute, status, results, dismiss.
- **Not planned:** OGC API Tiles/Maps (the UI's own tile endpoint is an internal detail) and OGC API Features (occurrence volumes per project are small; plain JSON endpoints are enough).

**Cost.** Low for MVP files, but COG writing depends on the raster stack choice. Medium for the Processes facade.

**Compliance test.** Validate COGs with a COG validator in CI (dev tool). Validate STAC Items against the official STAC 1.1.0 JSON Schemas (vendored). Round-trip read the GeoPackage. GeoJSON must parse with a strict parser, with coordinates checked within WGS84 bounds.

## 10. Zero-configuration product experience

**Design.** [REC]

- Double-click the executable or run `niche`. It creates the data, cache and config dirs at OS-standard locations (XDG on Linux, `%APPDATA%` on Windows, `~/Library/Application Support` on macOS), binds to `127.0.0.1` on a free port, opens the browser with the session token, and shows a first-run page with **"Try the demo"** and **"Start a project"**.
- Defaults come from the MaxEnt/maxnet literature (auto feature classes by sample size, regularization multiplier 1, cloglog output, 10 000 background points). Each default is shown with its source in the UI and recorded in the manifest. [ASSUMPTION] The values should be verified against maxent.jar 3.4.4 defaults in the solver research.
- Configuration is optional. Env vars and config file per 12F-III. `niche doctor` explains the effective config.

**Demo dataset candidates.**

| Candidate | Licence evidence | Pros | Cons |
|---|---|---|---|
| **Synthetic virtual species** on a synthetic landscape, generated deterministically at first run from a seed. Concept from Leroy et al. 2016 (`virtualspecies`) | Our own data, CC0. Do **not** port `virtualspecies` code: it is GPL (≥ 2), CRAN 1.6.1 [E25, FACT]. Reimplement the published concept | Zero legal risk; ~0 binary size; *known truth* lets the demo show model recovery and validation. Can be placed over France for familiarity | Must be labelled **"SYNTHETIC — not real biodiversity data"** in the UI, file names, manifest (`dataset.synthetic=true`) and every exported map |
| *Bradypus variegatus* (MaxEnt tutorial / maxnet) | maxnet: MIT package licence; data licence not separately stated [E22] | Classic, recognized, enables jar comparison | Table only (no rasters) in maxnet; tutorial rasters come from other distributions with unverified licence; South American species is unfamiliar to French users |

[REC] Ship the **synthetic virtual species** as the zero-config demo, and use **bradypus** as a golden-test fixture and an optional "classic benchmark" example. A real French species demo (for example from GBIF with CC0/CC BY records and an attribution file) is a later option after licence review [OPEN].

**Cost.** Low. **Milestone.** MVP.

**Compliance test.** E2E on a fresh, empty profile (no config, no network): start, click demo, reach a suitability map plus report in 3 or fewer interactions, within a time budget measured on the reference laptop [OPEN: value]. Assert that the "SYNTHETIC" label is present in the UI DOM, the GeoTIFF metadata and the manifest.

## Evidence and references

Listed in [References](#references). Version data from crates.io and npm was fetched via their APIs on 2026-10-15.

## Alternatives

- **Full tactical DDD and ports everywhere:** rejected. Cost is high and the trait count multiplies without measurable value.
- **RO-Crate as the primary manifest:** rejected for the MVP. JSON-LD adds authoring and validation cost, and users gain nothing until they deposit. See ADR-0203.
- **Hand-written OpenAPI spec:** rejected because of drift risk without mature axum server generation. See ADR-0202.
- **insta-only golden tests:** rejected. Rounding is not a tolerance model.
- **OpenTelemetry/Prometheus from day 1:** rejected for local mode; there is no operator.
- **OGC API Processes as the primary API:** rejected for the MVP. Its job vocabulary is too generic for UI needs. A facade comes later. See ADR-0204.
- **Bradypus as the demo:** rejected as primary because of the licence ambiguity of rasters and lower user relevance.

## Trade-offs

- Crate boundaries add compile units and some conversion code (adapter types to domain `Grid`). They buy testability and WASM.
- Committing generated artifacts (OpenAPI JSON, TS types) adds diff noise in MRs, but makes contract changes visible to reviewers.
- Deterministic reductions may cost some parallel throughput. Reproducibility outranks performance in the stated priority order.
- The offline basemap costs binary size or a first-run download.

## Risks

- Tolerance calibration done badly gives either flaky goldens or tolerances loose enough to hide real bugs.
- Floating-point non-portability across targets weakens "rerun gives identical hashes". The manifest must record target and CPU features, and docs must state the guarantee scope.
- GDAL (if chosen) concentrates parser risk for untrusted input.
- The utoipa 3.1/3.2 documentation inconsistency and pre-1.0 TS generators could churn.
- Bradypus data licence is stated only at package level.

## Recommendations

1. Day 1: workspace with the crate map of ADR-0201, CI dependency check and wasm build of domain crates.
2. MVP: manifest v1 (ADR-0203), utoipa spec snapshot plus openapi-typescript (ADR-0202), golden suite on bradypus with calibrated tolerances, stage spans, localhost protections plus resource limits plus cargo-deny, AGENTS.md, offline E2E, COG/GeoJSON/GeoPackage/STAC Item outputs (ADR-0204), synthetic demo.
3. Later: RO-Crate export, ODMAP pre-filled report, OGC API Processes facade, ASVS L2, fuzzing, metrics endpoint.

## Open questions

- [OPEN] Golden tolerance values: calibration protocol and acceptance by a domain reviewer.
- [OPEN] Cross-platform reproducibility guarantee (bit-identical vs tolerance).
- [OPEN] OGC API Processes Part 1 Core 2.0 status: re-check the OGC register before implementing the facade.
- [OPEN] Server-mode authentication: reverse-proxy OIDC vs built-in.
- [OPEN] Offline basemap: embed vs first-run download, and size budget.
- [OPEN] Sensitive-species rules (French SINP data-sensitivity practice) and whether outputs need coordinate blurring [ASSUMPTION that it applies].

## References

- [E1] OWASP ASVS project page. https://owasp.org/www-project-application-security-verification-standard/ (accessed 2026-10-15)
- [E2] OWASP ASVS GitHub releases, v5.0.0 (30 May). https://github.com/OWASP/ASVS/releases (accessed 2026-10-15). [E2b] Secondary: https://softwaremill.com/whats-new-in-asvs-5-0/ (May 2025 release)
- [E3] AGENTS.md. https://agents.md/ (accessed 2026-10-15)
- [E4] Workflow Run RO-Crate. https://www.researchobject.org/workflow-run-crate/ ; Leo S. et al. (2024) Recording provenance of workflow runs with RO-Crate. PLoS ONE. doi:10.1371/journal.pone.0309210 (accessed 2026-10-15)
- [E5] RO-Crate specification 1.3. https://www.researchobject.org/ro-crate/specification (accessed 2026-10-15)
- [E6] OGC API – Processes – Part 1: Core 1.0.0, OGC 18-062r2 (2021-12-20). https://docs.ogc.org/is/18-062r2/18-062r2.html (accessed 2026-10-15)
- [E7] NOAA OGC ACDO tracker, Dec 2024 (Processes Part 1 v2.0 target June 2025). https://vlab.noaa.gov/web/ogc-integrated-working-team/acdo-information (accessed 2026-10-15)
- [E8] STAC spec releases (v1.1.0). https://github.com/radiantearth/stac-spec/releases (accessed 2026-10-15)
- [E9] OGC Cloud Optimized GeoTIFF Standard 1.0, OGC 21-026. https://docs.ogc.org/is/21-026/21-026.html (accessed 2026-10-15)
- [E10] OGC GeoTIFF Standard 1.1, OGC 19-008r4. https://docs.ogc.org/is/19-008r4/19-008r4.html (accessed 2026-10-15)
- [E11] OGC API – Tiles – Part 1: Core, OGC 20-057. https://docs.ogc.org/is/20-057/20-057.html (accessed 2026-10-15)
- [E12] OGC API – Maps (Part 1 Core 1.0.0, 20-058). https://ogcapi.ogc.org/maps/ (accessed 2026-10-15)
- [E13] OGC API – Features overview. https://ogcapi.ogc.org/features/ (accessed 2026-10-15)
- [E14] GeoPackage 1.4.0 (OGC 12-128r19). https://www.geopackage.org/ (accessed 2026-10-15)
- [E15] Zurell D. et al. (2020) A standard protocol for reporting species distribution models. Ecography 43:1261–1277. doi:10.1111/ecog.04960 ; ODMAP app https://odmap.wsl.ch/ (accessed 2026-10-15)
- [E16] W3C PROV-O, Recommendation 2013-04-30. https://www.w3.org/TR/prov-o/ (accessed 2026-10-15)
- [E17] utoipa 6.0.0 / utoipa-axum 0.3.0. https://docs.rs/utoipa/ ; https://crates.io/crates/utoipa (accessed 2026-10-15)
- [E18] aide 0.15.1. https://docs.rs/aide/ (accessed 2026-10-15)
- [E19] npm registry: openapi-typescript 7.13.0, openapi-fetch 0.17.0, @hey-api/openapi-ts 0.99.0, orval 8.43.0. https://registry.npmjs.org/ (accessed 2026-10-15)
- [E20] insta 1.49.0, redactions docs. https://insta.rs/docs/redactions/ (accessed 2026-10-15)
- [E21] crates.io API data (version, date, licence): dhat 0.3.3, stats_alloc 0.1.10, approx 0.5.1, tracing 0.1.44, tracing-subscriber 0.3.23, tracing-chrome 0.7.2, cargo-deny 0.20.2, cargo-audit 0.22.2, cargo-cyclonedx 0.5.9, sha2 0.11.1, blake3 1.8.7, schemars 1.2.3, jsonschema 0.58.6, samply 0.13.1, flamegraph 0.6.14, criterion 0.8.2, divan 0.1.21, cap-std 4.0.3, memory-stats 1.2.0, sysinfo 0.39.6. https://crates.io/ (accessed 2026-10-15)
- [E22] maxnet R package, `bradypus` dataset docs and DESCRIPTION (MIT), local copy `reference/vendor/maxnet/`. Phillips S.J. et al. (2006) Ecol. Model. 190:231–259; Anderson R.P. & Handley C.O. (2001) Proc. Biol. Soc. Wash. 114:1–33
- [E23] Maxent LICENSE.md (MIT) and readme (3.4.4), local copy `reference/vendor/Maxent/`; `reference/vendor/jar/maxent.jar` SHA-256 4c856e55…ee5e
- [E24] elapid (MIT), local copy `reference/vendor/elapid/` (bradypus.csv.gz, test rasters)
- [E25] virtualspecies 1.6.1, CRAN, GPL (≥ 2). https://cran.r-project.org/package=virtualspecies (accessed 2026-10-15); Leroy B. et al. (2016) Ecography 39:599–607, doi:10.1111/ecog.01388 [UNVERIFIED – prior knowledge for pages/DOI]
- [E26] The Twelve-Factor App. https://12factor.net/
- [E27] Cockburn A. Hexagonal architecture (2005). https://alistair.cockburn.us/hexagonal-architecture/ [UNVERIFIED – prior knowledge]
- [E28] Evans E. (2003) Domain-Driven Design. Addison-Wesley [UNVERIFIED – prior knowledge]
- [E29] Microsoft Threat Modeling / STRIDE. https://learn.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats [UNVERIFIED – prior knowledge]
- [E30] Wilkinson M.D. et al. (2016) The FAIR Guiding Principles. Sci. Data 3:160018. doi:10.1038/sdata.2016.18 [UNVERIFIED – prior knowledge]
- [E31] Secondary comparison of TS OpenAPI generators (2026). https://www.pkgpulse.com/guides/orval-vs-openapi-typescript-vs-kubb-openapi-client-2026 (accessed 2026-10-15)
