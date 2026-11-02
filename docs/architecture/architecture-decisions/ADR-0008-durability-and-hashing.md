# ADR-0008 — SQLite `synchronous=FULL` for job state; SHA-256 for manifests and artifacts

- Status: Accepted (resolves a conflict between `11-backend-architecture.md` and ADR-0102 / `execution-manifest.md`)

## Decision
1. SQLite in WAL mode with `synchronous=FULL`. Job-state writes are low-frequency; `NORMAL` can
   lose the last committed transactions on power loss, which could drop a "completed" marker
   for results already written. The throughput cost is irrelevant at NicheRS write rates.
2. One hash everywhere: **SHA-256** for content-addressed artifacts and manifests. BLAKE3 is faster,
   but SHA-256 is what STAC (`file:checksum` multihash), RO-Crate and external verification tools
   expect; hashing cost is negligible next to model training and raster I/O (to be confirmed by
   the stage benchmarks; revisit only with measurements).
