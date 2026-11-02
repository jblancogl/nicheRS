# ADR-0005 — No WebAssembly in the MVP

- Status: Accepted
- Evidence: `docs/research/10-frontend-and-wasm.md`

## Decision
No WASM module in the MVP. Candidate uses (point inference, threshold recomputation, raster
statistics) are either tiny (TypeScript suffices) or need the full raster (server-side Rust is
closer to the data). Keep `niche-maxent` free of I/O so a `wasm32` build stays possible; add a
CI build check once the crate set stabilises.

## Revisit when
An offline/serverless viewer is required, or a measured browser-side operation exceeds an
interaction budget (e.g. >100 ms) in optimized TypeScript.
