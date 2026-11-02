# 11 — Backend architecture (Rust service, jobs, storage, streaming)

Status: research draft · Date: 2026-10-15 · Companion spec: `docs/architecture/api-design.md`

## 1. Scope

Design of the NicheRS server process: a single Rust executable that serves the REST API and the
embedded frontend, orchestrates long-running SDM jobs (data retrieval, MaxEnt training, cross-
validation, raster prediction, post-processing), persists job state in SQLite and stores scientific
artifacts on the local filesystem. Covers crate selection, async vs CPU-bound execution, durable job
state, quotas, cancellation, retries, idempotency, progress streaming (SSE), artifacts and caching,
errors, API versioning, reproducibility manifests, and the in-process vs worker-subprocess question.

Out of scope: the MaxEnt numerics themselves, raster I/O library choice, frontend, auth/multi-tenant
deployment (only hooks are noted).

Tag legend: [FACT] verified from a cited source on 2026-10-15; [ASSUMPTION]; [OPEN]; [REC];
[UNVERIFIED – prior knowledge].

## 2. Findings

### 2.1 Crate inventory (verified on crates.io API, 2026-10-15)

| Crate | Latest stable | Licence | Last release | Role | Verdict |
|---|---|---|---|---|---|
| axum | 0.8.9 | MIT | 2026-04-14 | HTTP routing, extractors, SSE | [REC] use |
| tokio | 1.53.3 | MIT | 2026-10-13 | async runtime, `sync::Semaphore`, `spawn_blocking` | [REC] use |
| tokio-util | 0.7.20 | MIT | 2026-10-09 | `CancellationToken` | [REC] use |
| tower | 0.5.3 | MIT | 2026-01-12 | middleware abstraction | [REC] use (via axum) |
| tower-http | 0.7.1 | MIT | 2026-08-31 | trace, compression, cors, limit, timeout, request-id, catch-panic, fs [22] | [REC] use |
| rayon | 1.12.0 | MIT OR Apache-2.0 | 2026-04-14 | data-parallel CPU work | [REC] use, dedicated pool |
| serde / serde_json | 1.0.229 / 1.0.154 | MIT OR Apache-2.0 | 2026-07 / 2026-10 | (de)serialisation | [REC] use |
| clap | 4.6.7 | MIT OR Apache-2.0 | 2026-09-14 | CLI (`niche serve`, `niche run`, `niche worker`) | [REC] use (derive) |
| tracing / tracing-subscriber | 0.1.44 / 0.3.23 | MIT | 2025-12 / 2026-03 | structured logs, spans per job | [REC] use |
| rusqlite | 0.40.2 | MIT | 2026-08-08 | SQLite driver (sync) | [REC] use, `bundled` |
| tokio-rusqlite | 0.8.0 | MIT | 2026-09-06 | async handle over a rusqlite connection thread | [REC] optional |
| sqlx | 0.9.0 | MIT OR Apache-2.0 | 2026-05-21 | async SQL, compile-time checked queries | alternative |
| utoipa / utoipa-axum | 6.0.0 / 0.3.0 | MIT OR Apache-2.0 | 2026-09-22 | OpenAPI 3.1 generation from code | [REC] use |
| aide | 0.15.1 | MIT OR Apache-2.0 | 2026-04-14 | alternative OpenAPI generator for axum | alternative |
| uuid (feature `v7`) | 1.28.0 | Apache-2.0 OR MIT | 2026-10-10 | time-ordered job ids (feature `v7` [23]) | [REC] use |
| ulid | 3.0.0 | MIT | 2026-07-16 | alternative sortable ids | alternative |
| blake3 | 1.8.7 | CC0-1.0 OR Apache-2.0 (OR Apache-2.0 WITH LLVM-exception) | 2026-08-20 | content addressing | [REC] use |
| sha2 | 0.11.1 | MIT OR Apache-2.0 | 2026-10-12 | SHA-256 where an external standard is expected | optional |
| rust-embed | 8.13.0 | MIT | 2026-10-07 | embed frontend build in binary | [REC] use |
| thiserror / anyhow | 2.0.21 / 1.0.104 | MIT OR Apache-2.0 | 2026-09 / 2026-07 | typed errors (lib) / app glue | [REC] use |
| problem_details | 0.10.0 | MIT OR Apache-2.0 | 2026-08-16 | RFC 9457 type (small, 156k downloads) | optional — a 40-line in-house type is sufficient |
| http-api-problem | 0.60.0 | Apache-2.0/MIT | 2025-01-06 | RFC 7807 type, stale | avoid |
| tikv-jemallocator | 0.7.0 | MIT/Apache-2.0 | 2026-05-25 | allocator with stats (memory accounting) | [OPEN] evaluate |
| nix | 0.31.3 | MIT | 2026-05-11 | `setrlimit`, signals for subprocess mode | later phase |

All licences above are permissive and compatible with proprietary or open-source distribution
[FACT, crates.io metadata]. Sources: [1]–[3].

### 2.2 Async I/O vs CPU-bound work

- [FACT] `spawn_blocking` tasks "cannot be aborted because they are not async"; runtime shutdown
  "will wait indefinitely for all started `spawn_blocking` to finish"; tokio docs point to rayon as
  "a good fit" for CPU-bound executors and warn that the blocking-thread limit is "very large by
  default" [4].
- [FACT] rayon `ThreadPoolBuilder::build()` creates an independent pool; `num_threads` defaults to
  `RAYON_NUM_THREADS` or logical CPUs; for detached `spawn()` work, a panic without a
  `panic_handler` aborts the process; `ThreadPool::install(op)` propagates a panic of `op` to the
  caller [5][6].
- Consequences [REC]:
  1. No numerics on tokio worker threads. Any loop > ~1 ms (feature expansion, optimiser, raster
     prediction, AUC/Boyce computation, reprojection) runs in the dedicated rayon pool.
  2. Do not use `spawn_blocking` as the compute executor: it is unbounded by default, not
     cancellable, and gives no CPU affinity/size control. Reserve it for short blocking I/O
     (GDAL open, SQLite calls if not using a connection thread, file hashing of small files).
  3. Bridge pattern: the async job driver acquires permits, then hands a closure to the rayon pool
     via `pool.spawn` + `tokio::sync::oneshot` to return the result (never call `install()` on a
     tokio worker thread — it blocks that thread). Wrap the closure in `catch_unwind` so a panic is
     sent back as an error instead of hitting the rayon panic handler (abort).
  4. Size: rayon pool = `cores − 1` (min 1) by default, configurable; tokio multi-thread runtime
     with 2–4 workers is ample for an API of this scale [ASSUMPTION].
  5. Progress and cancellation cross the boundary through lock-free primitives
     (`AtomicU64` counters, `CancellationToken::is_cancelled()` which is a plain sync `&self -> bool`
     [FACT, 7]) and a bounded `tokio::sync::mpsc`/`broadcast` channel using `try_send`/`blocking_send`
     from the compute side.

### 2.3 SQLite: rusqlite (bundled) vs sqlx

| Criterion | rusqlite + `bundled` | sqlx 0.9 (sqlite) |
|---|---|---|
| SQLite version | bundles SQLite 3.53.2 (rusqlite 0.40.1 / libsqlite3-sys 0.38.1) [FACT, 8] | `sqlite` feature enables `libsqlite3-sys/bundled` [FACT, 9] |
| API | sync; full SQLite surface (hooks, backup, blob, functions) [FACT, 8] | async; SQLite driver internally delegates to a worker thread [UNVERIFIED – prior knowledge] |
| Compile-time query checks | no (runtime) | yes (needs DB or offline metadata at build) |
| Migrations | `rusqlite_migration` or hand-rolled `user_version` | built-in `sqlx migrate` |
| Dependency weight | small | larger (macros, multi-DB abstraction) |
| Conflict risk | `libsqlite3-sys` must be single version in tree; sqlx 0.9 uses a version range to ease co-existence [FACT, 9] | same |

[REC] rusqlite with `bundled`, behind one **writer thread** (own `rusqlite::Connection`, commands over
a channel — `tokio-rusqlite` implements exactly this: "a thread is spawned for each opened
connection handle" [FACT, 10]) plus a small pool of read-only connections (N = 2–4). Rationale: the
workload is low-QPS metadata (jobs, events, artifacts), SQLite has one writer anyway, and a
single-writer actor removes `SQLITE_BUSY` handling and makes event-sequence numbering trivial.
sqlx's compile-time checks are attractive but not decisive for ~10 tables.

SQLite settings [FACT, 11 unless noted]:
- `journal_mode=WAL` (persistent): readers don't block the writer; "there can only be one writer at a
  time"; all processes must be on the same host — "WAL does not work over a network filesystem".
  → Data directory must be local disk, not NFS/SMB [REC].
- `synchronous=NORMAL` in WAL: commits may roll back after power loss/hard reset but DB stays
  consistent. [REC] Acceptable for job state *because* artifacts are content-addressed and jobs are
  re-runnable; use `FULL` only for the idempotency table if strict at-most-once is needed [OPEN].
- Auto-checkpoint at 1000 pages by default; long-lived read transactions block checkpoint progress
  → keep read transactions short; never hold a read txn across an SSE stream [REC].
- Also: `foreign_keys=ON`, `busy_timeout=5000`, `STRICT` tables [UNVERIFIED – prior knowledge, standard SQLite pragmas].

### 2.4 Filesystem-backed, content-addressed artifacts

Layout [REC]:

```
$NICHE_DATA/
  niche.db (+ -wal, -shm)
  cas/blake3/ab/cd/<64-hex>            immutable blobs (rasters COG, CSV, model JSON, PNG)
  cas/tmp/                              staging; write → fsync → rename into cas/
  jobs/<job_id>/manifest.json           reproducibility manifest (also stored as CAS blob)
  cache/datasets/<provider>/<key>/      downloaded env layers, occurrence pulls (keyed by request hash)
  cache/tiles/<artifact_hash>/{z}/{x}/{y}.png  derived, deletable
```

- Hash: BLAKE3 (fast, parallel, CC0/Apache) [FACT licence, 3]; store algorithm prefix in the id
  (`blake3:…`) so SHA-256 can be added for external interchange.
- Write protocol: stream to `cas/tmp/<uuid>` while hashing → `fsync` → `rename` (atomic within the
  same filesystem) → `fsync` dir → insert `artifacts` row. A crash leaves only tmp garbage, swept at
  start-up [REC; POSIX rename atomicity UNVERIFIED – prior knowledge].
- DB stores refs (`artifact(hash, size, media_type, created_at)`, `job_artifact(job_id, role, hash)`);
  GC = mark-and-sweep of unreferenced blobs older than a grace period.
- Serving: rasters as COG with HTTP Range; map tiles rendered lazily and cached under `cache/tiles`
  (derived data, never part of reproducibility).

### 2.5 Durable job state machine

States: `queued → preparing → running → finalizing → succeeded`; terminal also `failed`,
`cancelled`; transitional `cancelling`. A job spec expands into a DAG of **steps** (fetch
occurrences, fetch/crop layers, thin, build background, train fold k, evaluate, predict tile range,
threshold, summarise). Steps are the unit of checkpointing and retry.

Tables (sketch): `jobs(id, spec_hash, spec_json, state, priority, created_at, started_at,
finished_at, attempt, error_json, manifest_hash, idempotency_key)`, `steps(job_id, step_key, state,
attempt, output_hash, started_at, finished_at)`, `job_events(job_id, seq, ts, type, payload_json)`,
`idempotency(key, client, request_hash, job_id, created_at, expires_at)`.

Recovery on restart [REC]:
1. Any job in `preparing|running|finalizing|cancelling` with no live owner → mark the in-flight step
   `interrupted`; job goes to `queued` (or `cancelled` if it was `cancelling`), `attempt += 1`.
2. Completed steps whose `output_hash` exists in CAS are skipped (memoisation by
   `hash(step_kind, inputs_hashes, params, code_version)`), so resume is cheap and deterministic.
3. Emit a `job.recovered` event so SSE clients see the discontinuity.
4. Use a process "epoch" (`boot_id`) column to detect stale ownership; with a single process no
   leases are needed. Subprocess mode adds heartbeats (§2.11).

### 2.6 Bounded execution and quotas

Two independent admission gates in the scheduler (an async task fed by DB `queued` rows):

| Gate | Mechanism | Default |
|---|---|---|
| Concurrency | `Arc<Semaphore>` with `job_slots` permits (`acquire_owned`) | `max(1, cores/4)` [ASSUMPTION] |
| Memory | `Arc<Semaphore>` with permits = budget in MiB; job acquires `acquire_many_owned(estimate_mib)` | 70 % of `MemAvailable` or cgroup `memory.max` |
| CPU | the rayon pool size bounds total compute threads; per-job parallelism via nested `ThreadPool` or chunk counts | `cores − 1` |

[FACT] tokio's `Semaphore` is FIFO-fair; a large `acquire_many` at the head blocks later small
requests even if they would fit [12]. → [REC] use the FIFO behaviour deliberately (no starvation of
big jobs) but reject at submission any job whose estimate exceeds the total budget (HTTP 422
`resource-limit-exceeded`).

Memory estimate per job [ASSUMPTION — formulas, to be calibrated by benchmarks in
`docs/benchmarks`]:
- Training matrix: `n_samples × n_features × 8 B` (f64), with `n_samples = presences + background`
  (e.g. 10 000 background) and `n_features` after expansion (linear + quadratic + product + hinge +
  threshold can be ~10²–10³ for 20 variables). Example: 10 100 × 600 × 8 B ≈ 48 MB per fold copy.
- Prediction: never full-raster in memory; process in row/tile chunks: `chunk_cells × n_vars × 4 B`
  ×(threads). Full-raster arithmetic for scale: metropolitan France ≈ 5.5·10⁵ km²; at 100 m
  resolution ≈ 5.5·10⁷ cells × 20 vars × 4 B ≈ 4.4 GB — hence chunking is mandatory, and
  output is written incrementally to a COG/GeoTIFF.
- Estimate = `base + training + k_folds_parallel × fold + prediction_chunks + 25 % headroom`.
- Measure actual peak (`/proc/self/status` VmHWM per process, or jemalloc stats) and log
  `estimate vs actual` per job to refine the model.

cgroups v2 (optional hardening) [FACT, 13]: `memory.max` is the hard limit — if reached and not
reclaimable "the OOM killer is invoked in the cgroup" and "it's not going to kill any tasks outside of
this cgroup"; `memory.high` throttles without OOM; `cpu.max` caps bandwidth. Only meaningful per
process → requires subprocess mode (§2.11). In-process, a cgroup on the whole service only protects
the host, not other jobs.

### 2.7 Cancellation

- [REC] One `CancellationToken` per job, child tokens per step (`child_token`, cancelled when parent
  is [FACT, 7]); a process-wide root token for graceful shutdown.
- Compute code is cooperative: check `token.is_cancelled()` at every optimiser iteration (MaxEnt
  sequential/coordinate updates: iterations are ms-scale), every CV fold, every prediction chunk
  (e.g. every 256 rows), every download chunk. Return `Err(Cancelled)`; no partial artifacts are
  committed to CAS (staging files are deleted).
- API: `POST /jobs/{id}/cancel` → state `cancelling` (202) → `cancelled` when the step returns.
  Queued jobs cancel immediately. Cancellation is idempotent.
- Hard stop is impossible in-process (threads cannot be killed safely) [FACT for spawn_blocking, 4];
  subprocess mode allows SIGTERM → grace → SIGKILL.
- Graceful shutdown (SIGTERM): stop accepting jobs, cancel root token, wait up to N s, then exit;
  interrupted steps resume on next boot (§2.5).

### 2.8 Retries

- Classify errors: `Transient` (HTTP 429/5xx/timeouts from GBIF/remote raster sources, `SQLITE_BUSY`),
  `Permanent` (invalid spec, too few occurrences, non-convergence flagged as scientific failure),
  `Internal` (panic, I/O error on local disk).
- [REC] Retry only `Transient` at step level: exponential backoff with full jitter, max 5 attempts,
  honour `Retry-After`. Internal errors: one automatic retry of the step (to absorb transient
  resource issues) then fail. Never retry permanent/scientific failures — report them.
- Retries must be safe because steps are pure functions of their inputs written to CAS.

### 2.9 Idempotency

- [FACT] The IETF `Idempotency-Key` draft (-07) is **expired**, not an RFC [14]; it still provides
  usable semantics: optional fingerprint of the payload; 400 if a required key is missing; 422 if a key
  is reused with a different payload; 409 if the original request is still being processed [15].
- [REC] Two layers:
  1. HTTP layer: optional `Idempotency-Key` on `POST /jobs` and `POST /batches`; store
     `(key, request_hash, job_id)` for 24 h; replay returns the original response (201 → same body,
     plus header `Idempotent-Replayed: true`). Different payload → 422; in flight → 409.
  2. Domain layer: `spec_hash = blake3(canonical_json(spec_normalised) || engine_version)`. Canonical
     JSON: sorted keys, no insignificant whitespace, normalised numbers (RFC 8785 JCS
     [UNVERIFIED – prior knowledge]). If a `succeeded` job with the same `spec_hash` exists and
     the client sent `"reuse": "if_identical"`, return it (200 with `Content-Location`) instead of
     recomputing. Default: create a new job but memoise steps (§2.5), so the second run is fast and
     yields bit-identical outputs.

### 2.10 Progress streaming (SSE)

- [FACT] axum 0.8.9 `Sse` + `Event` (`id`, `event`, `data`, `json_data` with `json` feature, `retry`,
  `comment`) + `KeepAlive` [16][17].
- [FACT] Browsers send `Last-Event-ID` on reconnection when the last id was non-empty; `retry:` sets
  reconnection delay; comment lines are ignored by clients and the spec suggests one "every 15 seconds
  or so" for legacy proxies; HTTP 204 tells the client to stop reconnecting [18].
- Design [REC]:
  - Every event is persisted in `job_events` with a per-job monotonically increasing `seq`, and
    published on an in-memory `tokio::sync::broadcast` per job. SSE `id` = `seq`.
  - Handler: parse `Last-Event-ID` (or `?after=` for non-browser clients) → replay rows `seq > n`
    from DB → then subscribe to broadcast, dropping duplicates by `seq`. Subscribe *before* the replay
    query to avoid a gap. On `RecvError::Lagged`, re-query DB from the last delivered seq.
  - High-frequency progress (per iteration) is **coalesced**: at most ~4 `progress` events/s per job,
    and only the latest progress is persisted (or persisted at 1 Hz) to avoid DB churn; state
    transitions, step boundaries, warnings and logs are always persisted.
  - `KeepAlive` interval 15 s; `retry: 3000` on the first event.
  - When the job is terminal and replay is complete, send `job.finished` and close; a reconnect to a
    terminal job with `Last-Event-ID ≥ last seq` gets 204.
  - Must disable compression for `text/event-stream` responses (or ensure flush per event)
    [UNVERIFIED – prior knowledge: buffering compressors break SSE]; behind nginx set
    `X-Accel-Buffering: no`.

### 2.11 In-process vs worker subprocess

| | A. In-process (rayon pool) | B. `niche worker` subprocess (same binary re-exec) |
|---|---|---|
| Deployment | one process | still one binary; parent spawns `current_exe() worker --job <id>` |
| Crash isolation | panics caught via `catch_unwind` (only unwinding panics; `panic=abort` or aborts in C deps such as GDAL/PROJ terminate the process) [FACT, 19]; segfaults in C libs kill the server | full: a crash kills one job; parent records `failed(internal)` |
| Memory isolation | budget is advisory; allocation failure in Rust std aborts the process [UNVERIFIED – prior knowledge] | per-process `RLIMIT_AS`/cgroup `memory.max`; OOM kills only the worker [FACT cgroup scope, 13] |
| Hard cancel | no | SIGTERM/SIGKILL |
| Memory release | fragmentation retained by allocator in a long-lived server [ASSUMPTION] | returned to OS at exit |
| Overhead | none | process start (ms) + reload of inputs via memory-mapped CAS files; IPC for progress (stdout JSON lines) |
| Complexity | low | moderate: IPC protocol, heartbeat, orphan handling, Windows support (no `fork`, but `Command` spawn works) |
| Testing/debugging | simple | worker is also usable as CLI `niche run spec.json` → better reproducibility |

[REC] **MVP: A** with the following safeguards: `panic = "unwind"` in release profile, `catch_unwind`
around every step closure, memory-budget semaphore, chunked prediction, GDAL calls confined to
dedicated threads. **Design for B from day one**: the step executor takes `(StepSpec, inputs: CAS
refs) → (outputs: CAS refs, events)` with no shared in-memory state, and events are a serialisable
enum. Then subprocess mode is a transport swap (JSON lines over stdout/stdin), enabled by
`--isolation=process` once (a) any C dependency segfault is observed, (b) multi-user servers need
per-job memory caps, or (c) jobs > hours need hard kill.

### 2.12 Model & dataset caching

- Datasets (occurrences from GBIF etc., environmental layers): cache key = `hash(provider, query
  params normalised, provider dataset version/DOI, retrieval date bucket)`. Occurrence pulls are
  time-dependent → the manifest must record the GBIF download DOI/date; cache entries are immutable
  snapshots referenced by hash, never silently refreshed [REC].
- Raster layers: cache cropped/reprojected stacks per `(layer set, study area hash, resolution, CRS)`.
- Models: trained model = CAS blob; reused via step memoisation.
- In-memory: LRU of decoded layer stacks bounded by the memory budget (participates in the same
  semaphore) [REC].
- Eviction: size-capped LRU on `cache/` (not on `cas/` blobs referenced by jobs).

### 2.13 Errors (RFC 9457)

- [FACT] RFC 9457 (July 2023) obsoletes RFC 7807; media type `application/problem+json`; members
  `type`, `status`, `title`, `detail`, `instance`; clients must ignore unknown extension members;
  absent `type` = `about:blank` [20].
- [REC] Stable `type` URIs (`https://nichers.dev/problems/<slug>` or relative `/problems/<slug>`),
  extensions `code` (machine slug), `errors[]` (field-level: `pointer` JSON Pointer + `detail`),
  `request_id`, `job_id`. Job failures in `GET /jobs/{id}` reuse the same problem object in `error`.
  Never expose stack traces; log them with the `request_id`/`job_id` span.

### 2.14 API versioning

[REC] URI major version (`/api/v1`), additive changes within v1 (new fields/optional params/event
types; clients must ignore unknowns), breaking changes → `/api/v2` with ≥ 1 minor release of overlap,
`Deprecation`/`Sunset` headers on deprecated routes [UNVERIFIED – prior knowledge: RFC 8594 Sunset;
Deprecation header RFC 9745]. OpenAPI 3.1 document generated by utoipa (which emits `"openapi":
"3.1.0"` [FACT, 21]) served at `/api/v1/openapi.json`, committed and diffed in CI to detect
breaking changes.

### 2.15 Reproducibility manifest

Written at `finalizing`, stored in CAS, hash recorded on the job. Content [REC]:
- `nichers_version`, git commit, build profile, target triple, rustc version, `Cargo.lock` hash;
  versions of linked C libraries (GDAL, PROJ, SQLite).
- Normalised job spec + `spec_hash`.
- Inputs: each dataset with hash, provider, query, DOI/version, retrieval timestamp, licence.
- Study area geometry hash + CRS (EPSG / WKT2).
- Algorithm parameters including defaults actually applied (feature classes, regularisation
  multipliers, background count, thinning distance, CV scheme, thresholds).
- Seeds: root seed + derived per-step seeds (`seed_step = hash(root_seed, step_key)`), so parallel
  scheduling order does not change results.
- Determinism notes: thread count, whether float reductions are order-fixed (rayon reductions are not
  order-deterministic unless chunking is fixed [ASSUMPTION]) — [REC] use fixed-size chunks and
  ordered reductions for any value in reported metrics.
- Outputs: hash, role, media type, size. Step timing and peak memory (informational, excluded from
  equality checks).

## 3. Evidence and references

See References [1]–[21]. Crate metadata from `https://crates.io/api/v1/crates/<name>` queried
2026-10-15.

## 4. Alternatives

| Area | Chosen | Alternatives considered |
|---|---|---|
| Web framework | axum 0.8 | actix-web (own runtime model), poem; axum has tower ecosystem + first-class SSE |
| Job queue | in-house over SQLite | apalis / external broker (Redis/Postgres) — violates single-binary, overkill for one node |
| DB | SQLite (rusqlite) | sqlx-sqlite; PostgreSQL (multi-node only); embedded KV (redb) — loses ad-hoc SQL |
| Compute executor | dedicated rayon pool | `spawn_blocking` (unbounded, uncancellable), std threads, tokio `LocalSet` |
| Progress transport | SSE | WebSocket (bidirectional not needed; SSE replays via `Last-Event-ID` natively), polling (fallback) |
| OpenAPI | utoipa | aide; hand-written spec (drifts) |
| IDs | UUIDv7 | ULID, autoincrement (leaks counts, collides across instances) |
| Artifact store | local CAS | object storage (S3/MinIO) behind a trait for future server deployments |

## 5. Trade-offs

- Single writer: simplicity and no `BUSY` vs. write throughput ceiling — acceptable (events are
  coalesced; expected < 100 writes/s) [ASSUMPTION].
- `synchronous=NORMAL`: faster commits vs. losing the last transactions on power loss; mitigated by
  re-runnable steps.
- In-process MVP: lowest complexity vs. a C-library crash killing the whole server and all jobs.
- Step memoisation: fast resume/reruns vs. disk usage and the need for correct cache keys (a wrong
  key = silently stale science). Include engine version in every key.
- Coalesced progress: fewer writes vs. clients cannot replay every iteration (not needed).

## 6. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Heavy compute accidentally on tokio threads | API stalls, SSE timeouts | lint/review rule; `tokio-console`/task poll-time warnings; all numerics behind `Executor` trait |
| GDAL/PROJ segfault or abort | server down, all jobs interrupted | recovery on restart (§2.5); subprocess mode |
| Memory estimate wrong → OOM | process killed | measured peak logging; conservative headroom; chunking; subprocess+cgroup later |
| Nondeterministic parallel reductions | non-reproducible metrics | ordered reductions, fixed chunking, seeds per step |
| WAL on network FS | corruption/locking failures | refuse to start if data dir is on NFS/SMB (check `statfs` type) [REC] |
| Long SSE reads holding read txn | WAL growth | never hold txn during stream |
| Idempotency draft expired | semantics may change | document own policy; it is a server-local contract |
| `libsqlite3-sys` version conflicts | build failure | single SQLite crate (rusqlite only) |

## 7. Recommendations

1. [REC] Stack: axum 0.8 + tokio 1 + tower-http 0.7 (trace, compression excluding SSE, cors,
   request-id, limit, timeout, catch-panic) + rayon 1.12 dedicated pool + serde + clap 4 (derive) +
   tracing; rusqlite 0.40 `bundled` behind a single-writer thread; utoipa 6 for OpenAPI 3.1; uuid v7;
   blake3 CAS; rust-embed for the frontend.
2. [REC] Job engine = durable state machine in SQLite + step DAG memoised in CAS; restart recovery
   re-queues interrupted jobs and skips completed steps.
3. [REC] Admission via two semaphores (slots, memory MiB); reject impossible jobs at submit time.
4. [REC] Cooperative cancellation with `CancellationToken` checked per optimiser iteration / chunk.
5. [REC] SSE with persisted, sequenced events; `Last-Event-ID` replay; 15 s keep-alive.
6. [REC] RFC 9457 errors everywhere, `/api/v1` URI versioning, OpenAPI diff in CI.
7. [REC] Manifest per job with input hashes, versions, seeds, effective parameters.
8. [REC] In-process execution for MVP with `catch_unwind` + memory budget; executor interface
   designed so `niche worker` subprocess mode (same binary) can be added without API change.

## 8. Open questions

- [OPEN] Calibrated memory model for MaxEnt feature expansion and prediction chunk size — needs the
  benchmark suite.
- [OPEN] Do we need multi-user auth/quotas per user in v1 (affects scheduler fairness)?
- [OPEN] Deterministic results across different thread counts: required, or only for same machine?
- [OPEN] Will GDAL be linked (C FFI) or replaced by pure-Rust readers? Strongly affects the need for
  subprocess isolation.
- [OPEN] Retention policy for CAS and event logs on long-lived servers.
- [OPEN] Whether `synchronous=FULL` is needed for audit-grade job history.

## References

[1] crates.io API, crate metadata for axum, tokio, tokio-util, tower, tower-http, rayon, serde, serde_json, clap, tracing, tracing-subscriber — https://crates.io/api/v1/crates/{name} — accessed 2026-10-15.
[2] crates.io API, crate metadata for rusqlite, tokio-rusqlite, sqlx, deadpool-sqlite, r2d2_sqlite, utoipa, utoipa-axum, aide — https://crates.io/api/v1/crates/{name} — accessed 2026-10-15.
[3] crates.io API, crate metadata for uuid, ulid, blake3, sha2, rust-embed, thiserror, anyhow, problem_details, http-api-problem, tikv-jemallocator, nix, async-stream — https://crates.io/api/v1/crates/{name} — accessed 2026-10-15.
[4] tokio docs, `tokio::task::spawn_blocking` (tokio 1.53) — https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html — accessed 2026-10-15.
[5] rayon docs, `ThreadPoolBuilder` (rayon 1.12) — https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html — accessed 2026-10-15.
[6] rayon docs, `ThreadPool::install` — https://docs.rs/rayon/latest/rayon/struct.ThreadPool.html — accessed 2026-10-15.
[7] tokio-util docs, `CancellationToken` (0.7.20) — https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html — accessed 2026-10-15.
[8] rusqlite README (bundled SQLite 3.53.2; min SQLite 3.45.3) — https://github.com/rusqlite/rusqlite — accessed 2026-10-15.
[9] sqlx 0.9.0 docs, `sqlx::sqlite` (bundled libsqlite3-sys, version range) — https://docs.rs/sqlx/latest/sqlx/sqlite/index.html — accessed 2026-10-15.
[10] tokio-rusqlite 0.8.0 docs — https://docs.rs/tokio-rusqlite/latest/tokio_rusqlite/ — accessed 2026-10-15.
[11] SQLite, "Write-Ahead Logging" — https://www.sqlite.org/wal.html — accessed 2026-10-15.
[12] tokio docs, `tokio::sync::Semaphore` — https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html — accessed 2026-10-15.
[13] Linux kernel docs, "Control Group v2" (memory.max, memory.high, cpu.max) — https://docs.kernel.org/admin-guide/cgroup-v2.html — accessed 2026-10-15.
[14] IETF Datatracker, draft-ietf-httpapi-idempotency-key-header (status: expired, rev -07) — https://datatracker.ietf.org/doc/draft-ietf-httpapi-idempotency-key-header/ — accessed 2026-10-15.
[15] J. Jena, S. Dalal, "The Idempotency-Key HTTP Header Field", draft-ietf-httpapi-idempotency-key-header-07, 2025-10-15 — https://www.ietf.org/archive/id/draft-ietf-httpapi-idempotency-key-header-07.html — accessed 2026-10-15.
[16] axum 0.8.9 docs, `axum::response::sse` — https://docs.rs/axum/latest/axum/response/sse/index.html — accessed 2026-10-15.
[17] axum 0.8.9 docs, `axum::response::sse::Event` — https://docs.rs/axum/latest/axum/response/sse/struct.Event.html — accessed 2026-10-15.
[18] WHATWG HTML Living Standard, §9.2 Server-sent events — https://html.spec.whatwg.org/multipage/server-sent-events.html — accessed 2026-10-15.
[19] Rust std docs, `std::panic::catch_unwind` — https://doc.rust-lang.org/std/panic/fn.catch_unwind.html — accessed 2026-10-15.
[20] RFC 9457, M. Nottingham, E. Wilde, S. Dalal, "Problem Details for HTTP APIs", July 2023, DOI 10.17487/RFC9457 — https://www.rfc-editor.org/rfc/rfc9457.html — accessed 2026-10-15.
[21] utoipa repository README (OpenAPI 3.1, utoipa-axum, MIT OR Apache-2.0) — https://github.com/juhaku/utoipa — accessed 2026-10-15.
[22] tower-http 0.7.1 docs (middleware modules) — https://docs.rs/tower-http/latest/tower_http/ — accessed 2026-10-15.
[23] uuid 1.28.0 docs (feature `v7`) — https://docs.rs/uuid/latest/uuid/ — accessed 2026-10-15.
