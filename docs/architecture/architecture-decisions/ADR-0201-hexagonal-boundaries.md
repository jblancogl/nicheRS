# ADR-0201: Hexagonal crate boundaries for the scientific core

- Status: Proposed
- Date: 2026-10-15
- Deciders: NicheRS architecture (proposal by research agent; to be approved by product owner and lead developer)
- Related: `../engineering-principles.md` §1, 12-factor brief, ADR-0203

## Context

NicheRS must run the same SDM engine in local, server and batch modes, possibly in WASM. Its first priority is scientific correctness, which must be checkable by golden tests that use no network, no database and no raster library. The product owner requires DDD/hexagonal separation of the domain from transport, UI, database, geo libraries and infrastructure. The same product owner also requires that complexity be justified by measurable value.

## Decision

Use a single Cargo workspace with these crates. Arrows show allowed normal dependencies.

| Layer | Crate | Contents | May depend on |
|---|---|---|---|
| Domain | `niche-core` | Value types (`GridSpec`, `Grid<f32>`, `PointTable`, `Species`, `StudyArea` geometry in a modelling CRS), invariants, ubiquitous-language glossary, manifest data types | std, serde, thiserror, ndarray (or equivalent numeric array) |
| Domain | `niche-maxent` | Feature transforms, MaxEnt fitting and prediction | `niche-core`, rayon (CPU parallelism allowed; no I/O) |
| Domain | `niche-eval` | Partitioning (k-fold, spatial blocks), metrics, thresholds | `niche-core` |
| Application | `niche-app` | Use cases (create project, run pipeline, rerun from manifest, verify), job lifecycle, manifest assembly, **port traits** | domain crates |
| Adapter | `niche-geo` | Raster/vector I/O, reprojection, COG/GeoJSON/GeoPackage/STAC writing, conversion to `Grid` | `niche-core`, chosen geo stack |
| Adapter | `niche-sources` | GBIF client, file importers, predictor catalogue, download cache | `niche-app` ports, reqwest |
| Adapter | `niche-store` | SQLite job repository, FS artifact store (atomic writes) | `niche-app` ports, rusqlite |
| Driver | `niche-api` | Axum handlers, DTOs, OpenAPI (ADR-0202) | `niche-app` |
| Driver | `niche-cli` | clap subcommands (`doctor`, `benchmark`, `rerun`, …) | `niche-app` |
| Root | `niche` (bin) | Composition root, config loading, embedded frontend | all |

Ports (traits in `niche-app`) are limited at MVP to `OccurrenceSource`, `EnvironmentSource`, `ArtifactStore` and `JobRepository`.

**Rule:** a new trait needs either two production implementations or a test double that removes an external dependency (network, disk, time-consuming I/O). Closed sets (feature classes, metrics, partitioning strategies, output transforms, the single algorithm) are `enum`s and functions. There is no `Algorithm` trait until a second algorithm is scheduled; then use `enum ModelSpec`.

## Alternatives

1. **Single crate with modules.** Simplest, but there is no compiler-enforced boundary. GDAL or Tokio types leak into science code over time.
2. **Full tactical DDD** (aggregates, repositories per entity, domain events, CQRS). Large ceremony for a computation pipeline with few transactional invariants.
3. **Trait per external library** (raster trait, HTTP trait, logging trait). Abstraction without a second implementation; it hides library capabilities.
4. **Separate process for the domain.** This is the process-isolation question, not boundary design. Deferred to runtime/security decisions.

## Evidence

- Hexagonal architecture (ports and adapters) [E27, UNVERIFIED – prior knowledge]. DDD [E28, UNVERIFIED – prior knowledge].
- 12-factor brief: three deployment modes sharing one engine; backing services behind interfaces (IV); object storage later.
- The WASM target (`wasm32-unknown-unknown`) cannot use std filesystem or network APIs. A successful build of the domain crates is therefore a mechanical proof of I/O-freedom [ASSUMPTION: rayon usage needs a feature gate or a sequential fallback for wasm].

## Trade-offs

- More crates mean somewhat longer clean builds, but incremental builds improve, because domain changes don't rebuild Axum and Axum changes don't rebuild the domain [ASSUMPTION].
- Adapter-to-domain conversion (for example a raster library buffer to `Grid`) may copy memory. Measure it; allow zero-copy views where profiling shows a cost.

## Risks

- The boundary erodes if checks are not automated.
- `niche-core` becomes a dumping ground. Keep it to types and invariants; algorithms live in their own crates.
- Rayon inside domain crates complicates WASM. Mitigate with a `parallel` feature flag.

## Compliance

- CI script over `cargo metadata`: the normal-dependency closure of `niche-core`, `niche-maxent` and `niche-eval` must not contain `tokio`, `axum`, `hyper`, `reqwest`, `rusqlite`, `sqlx`, `gdal*`, `proj*` or any adapter crate.
- `clippy.toml` `disallowed-methods` blocks `std::fs`/`std::net` in domain crates.
- CI: `cargo build --target wasm32-unknown-unknown -p niche-core -p niche-eval -p niche-maxent --no-default-features`.

## Reversibility

High. Merging crates is trivial. Splitting later is costly, which is why the split happens at day 1. Ports can be removed if their second implementation never materializes.

## Rationale

The boundaries buy the two things ranked highest, correctness (testable pure core, golden tests) and reproducibility (manifest types in core), plus deployment-mode reuse. The "no trait without two implementations" rule keeps the cost proportional to that value.
