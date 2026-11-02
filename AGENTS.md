# AGENTS.md — rules for humans and coding agents working on NicheRS

NicheRS is a species distribution modelling (SDM) instrument. Its outputs inform ecological
decisions, so **scientific correctness outranks everything else**:
correctness → reproducibility → measured performance → geospatial reliability → UX → deployment simplicity.

## Repository map

| Path | What it is |
|---|---|
| `crates/niche-maxent` | Pure modelling kernel (MaxEnt 3.4.4 port). No I/O, no async, no geo libraries. |
| `crates/niche-eval` | Pure evaluation: CV partitions, Boyce/OR10/AUC, thresholds, MESS, collinearity, frozen PRNG. |
| `crates/niche-rf` | Pure second engine: down-sampled (balanced) random forest for presence–background data (ADR-0017). |
| `crates/niche-geo` | GDAL-free raster/vector primitives: GeoTIFF window reader/writer, masks, exact areas, tiles. |
| `crates/niche-data` | Remote adapters: HTTP ranges, CHELSA v2.1, GBIF, Natural Earth land mask. |
| `crates/niche-app` | Use cases: cleaning, tuning/fit pipeline, evidence rules, SQLite job store, manifests. |
| `crates/niche-server` | Axum API v1, SSE, tiles, embedded UI (`web/dist`). |
| `crates/niche-cli` | `niche` executable: serve, run, demo, doctor, cache, export-swd, SWD tools. |
| `web/` | React + TS + Vite + Tailwind v4 + shadcn/ui + MapLibre UI; contract in `web/src/api/types.ts` mirrors `crates/niche-app/src/dto.rs`. |
| `tests/fixtures/synthetic` | Deterministic synthetic datasets (`scripts/gen_synthetic.py`). Synthetic, never presented as real. |
| `tests/reference/maxent-3.4.4` | Golden outputs of `maxent.jar` 3.4.4 for the fixtures. |
| `scripts/compare_maxent.py` | Runs maxent.jar and NicheRS on every case; writes `docs/benchmarks/phase3-results.md`. |
| `reference/vendor/` | Downloaded MaxEnt/maxnet/elapid sources and `maxent.jar` (gitignored, reference only). |
| `docs/` | Research, architecture, ADRs, design system, benchmarks. Read `docs/executive-summary.md` first. |

## Architectural boundaries

- `niche-maxent`, `niche-eval` and `niche-rf` must not depend on file systems, HTTP, databases, GDAL/PROJ,
  Tokio or Rayon. Inputs are in-memory matrices; cancellation is a trait (`CancelCheck`).
- Scientific defaults (cleaning, tuning grid, selection, evidence cut-offs) are ADR-0009; change
  them only through a new ADR. The API contract changes in `dto.rs` AND `web/src/api/types.ts` together.
- No trait/abstraction without two real implementations (ADR-0201).
- Heavy numerical work never runs on the async runtime (ADR-0103).
- New crates only when a boundary is real (e.g. `niche-geo` once raster I/O exists).

## Scientific rules (non-negotiable)

1. **MaxEnt compatibility is tested, not assumed.** Any change to `crates/niche-maxent/src/train.rs`
   or `model.rs` must keep `cargo test` golden tests green and
   `python3 -I scripts/compare_maxent.py` at 100 % PASS. Intentional output changes need an ADR
   and regenerated references, never loosened tolerances.
2. The port mirrors Java bookkeeping on purpose (stale expectations, unstable sort ties, sum
   order). Do not "clean up" these without re-running the comparison.
3. Never label MaxEnt output as a probability. Use "relative suitability" (cloglog/logistic are
   transforms, not calibrated probabilities). Never invent a confidence percentage.
4. Every result must be traceable: inputs, versions, seeds, settings (execution manifest,
   `docs/architecture/execution-manifest.md`).
5. Never present synthetic or placeholder data as real predictions; label demo data as synthetic.
6. Default datasets must be licensed for commercial use (WorldClim is NOT; see
   `docs/research/06-biodiversity-data.md`).

## Engineering rules

- Rust edition 2024, MSRV in `Cargo.toml`. No `unwrap()`/`expect()` in production paths;
  errors are typed (`thiserror`) in libraries, `anyhow` only in the binary.
- `unsafe` is forbidden workspace-wide.
- Do not add dependencies without checking licence (MIT/Apache/BSD/ISC/Zlib/CC0 only; no GPL
  in shipped code) and maintenance status.
- Do not claim speed-ups without numbers from `scripts/compare_maxent.py` or a criterion bench.

## Definition of done for any change

```bash
export PATH=$HOME/.cargo/bin:$PATH   # if cargo is not on PATH
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
# if the kernel changed (needs Java 11+ and reference/vendor/jar/maxent.jar):
cargo build --release && python3 -I scripts/compare_maxent.py
# frontend changes:
(cd web && npm run typecheck && npm run lint && npm run build)
# network smoke tests (optional): cargo test -p niche-data --test live -- --ignored
```

Update docs when behaviour changes. Report what was verified and what was not.

## Prohibited shortcuts

- Loosening a tolerance to make a golden test pass.
- Hiding warnings (data quality, extrapolation, insufficient samples) to simplify the UI.
- Committing downloaded datasets, credentials or real sensitive occurrence coordinates.
