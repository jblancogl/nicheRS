# Twelve-Factor Compliance (adapted)

Status: proposal. Date: 2026-10-15. Implementation status for every factor: **not started — research
phase** (no code exists under `crates/` at the time of writing).

## Scope

How NicheRS applies the Twelve-Factor methodology [1] as a *local-first scientific instrument that can
also run as a service*, not as a cloud microservice. One matrix per factor, then the eight tensions from
the product-owner brief with pragmatic decisions. Details live in:
[configuration.md](configuration.md), [deployment-modes.md](deployment-modes.md),
[job-lifecycle.md](job-lifecycle.md), [observability.md](observability.md), ADR-0101..0104.

[FACT] The methodology text at 12factor.net states "Last updated 2017" in its footer [1]; we treat it
as a stable reference, not a living standard.

## Findings — factor matrix

Legend for *Status*: NS = not started — research phase.

### I. Codebase — "One codebase tracked in revision control, many deploys" [1]

| Item | Content |
|---|---|
| Requirement | One monorepo (Rust workspace + `web/` frontend); Local/Server/Batch are *deploys of the same build*, not forks. |
| Implementation | Cargo workspace `crates/` (engine, store, server, cli → one `niche` binary); `web/` Vite app embedded at build. Mode is a runtime setting (`niche serve --mode`, `niche run`). No per-customer branches; customer specifics = config + experiment files. |
| Status | NS |
| Deviations | `reference/vendor/` holds third-party reference implementations (Maxent, maxnet, elapid) for validation only; git-ignored, not part of the build. |
| Justification | Scientific validation needs reference code without shipping it. |
| Verification | CI: single pipeline builds one artifact per target triple; `git grep` for mode-specific `#[cfg(feature=...)]` that changes science → must be none. |

### II. Dependencies — "Explicitly declare and isolate dependencies" [1]

| Item | Content |
|---|---|
| Requirement | `Cargo.lock` + frontend lockfile committed; no undocumented native deps; SBOM. No Python/Java/R/Node at runtime. |
| Implementation | `Cargo.lock`, `web/pnpm-lock.yaml` (or npm lockfile) committed; `cargo build --locked`; SBOM via `cargo-cyclonedx` 0.5.9 [34]; dependency list embedded in the binary via `cargo-auditable` 0.7.7 [35]; `docs/RUNTIME-DEPS.md` lists every dynamically linked library per platform. |
| Status | NS |
| Deviations | If GDAL/PROJ are linked (see tension 8), they are native deps — allowed only statically linked or documented per platform. |
| Justification | Geospatial correctness may outweigh pure-Rust purity (decided in the geospatial research doc). |
| Verification | CI step: `ldd`/`otool -L`/`dumpbin /dependents` on the release binary, diff against the allow-list in RUNTIME-DEPS.md; fail on unknown libs. Container test: run binary in a minimal image with no Python/Java/R/Node present. |

### III. Config — "Store config in the environment" [1]

| Item | Content |
|---|---|
| Requirement | `NICHE_*` env vars; validated config file for local prefs; documented precedence & defaults; no embedded credentials/paths; experiment definitions separate. |
| Implementation | CLI > env > file > defaults with provenance; `niche config show/validate`; secrets via `*_FILE`; experiment files hashed into manifests ([configuration.md](configuration.md), ADR-0101). |
| Status | NS |
| Deviations | A config *file* exists (pure 12factor prefers env only). |
| Justification | Desktop users double-clicking an exe cannot be expected to set env vars; the file is the lowest non-default layer so env still wins in server deployments. |
| Verification | Table-driven tests: for every setting, assert value per layer combination; test that secret values never appear in `config show`, logs, panic messages; `grep` release binary strings for absolute home paths (`/home/`, `C:\Users`). |

### IV. Backing services — "Treat backing services as attached resources" [1]

| Item | Content |
|---|---|
| Requirement | FS + SQLite by default behind interfaces; future object storage/external DB possible; no Redis/Postgres/cloud without need. |
| Implementation | Traits `MetadataStore` (SQLite impl), `ArtifactStore` (local FS impl, later object storage), `DatasetSource` (GBIF, raster providers, local files). Remote data providers are attached via URL config and can be disabled (`NICHE_OFFLINE`). |
| Status | NS |
| Deviations | SQLite and FS are *embedded*, not attached via URL; they are not swappable at runtime by config alone. |
| Justification | Local-first, zero-install; SQLite WAL is single-host by design [24]. Interfaces keep a Postgres/S3 path open without rewriting the engine (ADR-0102). |
| Verification | Engine crate has no dependency on `rusqlite`/`std::fs` paths outside the store crates (enforced by `cargo deny`/workspace layering check); contract tests run against an in-memory store implementation. |

### V. Build, release, run — "Strictly separate build and run stages" [1]

| Item | Content |
|---|---|
| Requirement | Frontend embedded at Rust build; no Node in prod; version the executable, data schemas, model formats, reproducibility metadata. |
| Implementation | Build: `pnpm build` → `web/dist` → embedded (e.g. `rust-embed` 8.13.0 [32], which embeds at compile time in release and reads from FS in dev). Release = signed binary + SBOM + checksums, tagged `vX.Y.Z`. Binary reports `niche --version` = semver + git commit + build date + DB schema version + model-format version + experiment-schema version. |
| Status | NS |
| Deviations | No separate "release = build + config" object as in PaaS; release is the binary, config applied at run. |
| Justification | Desktop distribution model. |
| Verification | CI builds release with Node absent from the *runtime* stage; smoke test runs the binary on a clean VM and loads the UI. Reproducible-build check: two CI builds of the same commit compared (`diffoscope`), differences documented [OPEN: toolchain limits]. |

### VI. Processes — "Execute the app as one or more stateless processes" [1]

| Item | Content |
|---|---|
| Requirement | Durable job state in FS/SQLite; recovery of interrupted jobs; bounded worker pool or child processes when justified. |
| Implementation | All job/artifact state persisted before acknowledgement; in-memory state is only a cache of DB; on restart jobs are recovered ([job-lifecycle.md](job-lifecycle.md)); in-process workers by default, subprocess isolation optional (ADR-0103). |
| Status | NS |
| Deviations | **Major**: the process owns a stateful local SQLite + artifact tree; 12factor says processes are "stateless and share-nothing" and the filesystem is "a brief, single-transaction cache" [3]. |
| Justification | See tension 1. The *process* is disposable; the *data dir* is the backing store. Killing the process loses nothing committed. |
| Verification | Chaos test: `kill -9` during each job stage → restart → job resumes/finishes with identical artifact hashes; no committed artifact lost. |

### VII. Port binding — "Export services via port binding" [1]

| Item | Content |
|---|---|
| Requirement | Axum serves UI + API; loopback default; reverse proxy supported. |
| Implementation | `axum` 0.8.9 [14] self-contained HTTP server; `127.0.0.1:8740` default; `NICHE_BASE_PATH`, `NICHE_PUBLIC_URL`, `NICHE_TRUSTED_PROXIES` for proxies; Host/Origin allow-lists. |
| Status | NS |
| Deviations | None (TLS terminated by reverse proxy in server mode; no built-in TLS initially). |
| Justification | — |
| Verification | Integration test binds port 0, asserts UI + `/api` + `/healthz` served from one listener; test that non-loopback bind without auth is rejected. |

### VIII. Concurrency — "Scale out via the process model" [1]

| Item | Content |
|---|---|
| Requirement | Bounded; Tokio for IO, Rayon/dedicated workers for CPU; no oversubscription. |
| Implementation | Tokio runtime for HTTP/IO; one global Rayon pool sized `NICHE_CPU_THREADS`; job scheduler admits ≤ `NICHE_MAX_WORKERS` jobs and checks `NICHE_MAX_MEMORY_MB` estimates; CPU work enters Rayon via `spawn_blocking`/channel, never on Tokio workers. |
| Status | NS |
| Deviations | Scale-*up* within one process, not scale-*out* by process types. |
| Justification | Single-user/small-team scientific workloads; a single host with many cores is the target (tension 6). |
| Verification | Load test: submit 100 jobs, assert running ≤ max_workers, OS thread count ≤ tokio + rayon + small constant; HTTP p99 latency for `/healthz` stays < 100 ms during CPU saturation [REC target]. |

### IX. Disposability — "Maximize robustness with fast startup and graceful shutdown" [1]

| Item | Content |
|---|---|
| Requirement | Fast start, graceful shutdown, cancellation, atomic artifact writes, crash-safe metadata, recovery, cleanup; never silently lose completed results. |
| Implementation | SIGTERM/SIGINT (+ Windows ctrl events) → stop accepting → cancel tokens with reason `Shutdown` → grace period → exit; jobs are requeued with stage checkpoints; artifacts written tmp→fsync→rename→fsync dir→DB row (ADR-0104, [job-lifecycle.md](job-lifecycle.md)). 12factor: "all jobs are reentrant" / return job to queue on shutdown [5]. |
| Status | NS |
| Deviations | A training stage interrupted mid-way restarts from the stage start (not from the middle of an iteration). |
| Justification | Stage-level checkpoints are simpler and verifiable; MaxEnt fits per species are minutes, not days [ASSUMPTION — confirm with benchmarks]. |
| Verification | Tests: SIGTERM during each stage; `kill -9`; power-loss simulation (e.g. `dm-flakey` or VM snapshot) for SQLite `synchronous=FULL`; startup time < 1 s with 10 000 jobs in DB [REC target]. |

### X. Dev/prod parity — "Keep development, staging, and production as similar as possible" [1]

| Item | Content |
|---|---|
| Requirement | Vite only in dev. |
| Implementation | Dev: Vite dev server proxies `/api` to `niche serve`; prod: same Axum routes serve embedded assets. Same SQLite, same FS layout, same binary in dev and prod. |
| Status | NS |
| Deviations | Dev frontend uses HMR from Vite (different asset origin). |
| Justification | Tension 5. |
| Verification | CI e2e (Playwright) runs against the *release* binary with embedded assets, never against Vite. |

### XI. Logs — "Treat logs as event streams" [1]

| Item | Content |
|---|---|
| Requirement | `tracing`, stdout/stderr, structured, job/model/stage ids, timings, no sensitive coordinates; health + metrics where justified. |
| Implementation | `tracing-subscriber` 0.3.23 (`json` feature) [13]; pretty on TTY, JSON otherwise; spans carry ids; coordinate redaction by type; `/healthz`, `/readyz`, optional `/metrics` ([observability.md](observability.md)). Per-job event log also stored in SQLite for the UI (product feature, not the log stream). |
| Status | NS |
| Deviations | Optional `NICHE_LOG_FILE` with rotation for Local desktop mode. |
| Justification | 12factor assumes a process manager collects stdout [6]; a double-clicked desktop app has none. Off by default; never used in Server mode (journald/container runtime collect stdout). |
| Verification | Test captures stdout in JSON mode, validates each line parses and contains `job_id`/`stage` inside job spans; property test: random coordinates fed through engine never appear in captured logs at `info`. |

### XII. Admin processes — "Run admin/management tasks as one-off processes" [1]

| Item | Content |
|---|---|
| Requirement | `niche doctor`, `migrate`, `cache inspect`, `cache clean`, `datasets validate`, `models verify`, `benchmark`. |
| Implementation | Subcommands of the same binary, same config resolution, same release [7] ([observability.md § admin](observability.md#admin-subcommands)). Mutating commands take the data-dir lock. |
| Status | NS |
| Deviations | None. |
| Justification | — |
| Verification | Each subcommand has an integration test with documented exit codes; `niche migrate` tested on fixture DBs of every released schema version. |

## Tensions — pragmatic decisions

| # | Tension | Decision | Cost accepted |
|---|---|---|---|
| 1 | Stateless processes vs local SQLite | The **process** is stateless/disposable; the **data dir** is the backing service. All state is in SQLite+FS, committed before acknowledgement; memory holds only caches. One process per data dir (exclusive `File::try_lock`, std ≥ 1.89 [20]). | No two server processes on one data dir; no sticky-session-free horizontal scale (see 6). |
| 2 | Single executable vs process isolation | One executable; workers in-process by default; the *same* executable can re-exec itself as `niche worker --job <id>` (hidden subcommand) when `NICHE_JOB_ISOLATION=subprocess` (ADR-0103). | Two code paths to test; IPC protocol to version. |
| 3 | Offline-first vs remote backing services | Remote providers (GBIF, raster catalogues) are *optional attached resources* behind `DatasetSource`; content-addressed cache makes repeated runs offline; `NICHE_OFFLINE=true` forbids network; local file import always available. | Cache can grow large (quota + `cache clean`). |
| 4 | Reproducibility vs environment config | Env/config may only change *where/how fast*, never *what*. Experiment definition is a hashed file; env cannot override it; execution context recorded but not hashed; determinism tests across thread counts. | Engine must use deterministic reductions (ordered chunking) even when parallel — slight performance cost. |
| 5 | Embedded assets vs frontend dev flexibility | Release embeds `web/dist`; dev uses Vite + proxy; `rust-embed` reads from FS in debug builds [32]. E2E tests always against the embedded build. | Two serving paths (dev/prod) — limited to static assets only. |
| 6 | Local FS vs horizontal scaling | Not a goal for v1. Scale up (cores) first; scale out later by (a) sharding by data dir/tenant or (b) moving `MetadataStore` to Postgres and `ArtifactStore` to object storage behind the existing traits. SQLite WAL cannot span hosts [24]. | Server mode is single-node; HA requires the v2 path. |
| 7 | Graceful shutdown vs long training | Shutdown does not wait for training to finish: jobs are interrupted at the next cooperative cancellation point, completed stages are kept (checkpoints), job requeued with `attempt+1`; grace period configurable; `finalizing` commits are allowed to finish (bounded). | Work since the last stage checkpoint is redone. |
| 8 | Portability vs native geospatial libs | Prefer pure-Rust readers/writers where they meet correctness; any native lib (GDAL/PROJ) must be statically linked or bundled and listed in RUNTIME-DEPS.md; `niche doctor` reports versions. Final choice deferred to the geospatial research doc. | Possible larger binary / build complexity; or reduced format coverage. |

## Evidence and references

Claims about 12factor text are quoted from the pages in [1]–[7]; crate facts from crates.io API and
docs.rs pages read on 2026-10-15 (versions in the reference list).

## Alternatives

- **Strict 12factor (env-only, stateless, Postgres + Redis + S3)** — rejected: violates local-first,
  zero-install, offline use.
- **Desktop-only app (Tauri/Electron, no server mode)** — rejected: brief requires an optional service mode
  and headless batch; a single Axum server covers all three.
- **Ignore 12factor** — rejected by product owner; factors give a useful checklist for disposability,
  config and logs.

## Trade-offs

Accepting stateful local storage (VI) and scale-up (VIII) buys zero-install operation and simple
reasoning about consistency, at the cost of a single-node server mode.

## Risks

- SQLite on network filesystems (users pointing `NICHE_DATA_DIR` to SMB/NFS shares common in
  consultancies) → corruption/locking issues; WAL "does not work over a network filesystem" [24].
  Mitigation: `niche doctor` + startup check of filesystem type, refuse in server mode, warn locally.
- Windows shutdown events give limited time [UNVERIFIED – prior knowledge: console close/shutdown
  handlers get a few seconds]; recovery relies on restart path, not graceful path.

## Recommendations

[REC] Adopt the matrix as the acceptance checklist; each "Verification" row becomes a CI job or
release-checklist item before 1.0.

## Open questions

- [OPEN] Reproducible builds across macOS/Windows toolchains — feasible degree unknown.
- [OPEN] Whether subprocess isolation should be the default in Server mode (ADR-0103).

## References

Shared by all docs in `docs/architecture/` written by this agent (configuration, deployment-modes,
job-lifecycle, observability, ADR-0101..0104). Accessed 2026-10-15 unless stated.

- [1] The Twelve-Factor App, https://12factor.net/ (footer: "Last updated 2017").
- [2] 12factor — III. Config, https://12factor.net/config
- [3] 12factor — VI. Processes, https://12factor.net/processes
- [4] 12factor — VIII. Concurrency, https://12factor.net/concurrency
- [5] 12factor — IX. Disposability, https://12factor.net/disposability
- [6] 12factor — XI. Logs, https://12factor.net/logs
- [7] 12factor — XII. Admin processes, https://12factor.net/admin-processes
- [8] figment 0.10.19 (last release 2024-05-17, MIT OR Apache-2.0), https://docs.rs/figment/0.10.19/figment/
- [9] config 0.15.27 (2026-09-30, MIT OR Apache-2.0, MSRV 1.88), https://crates.io/crates/config
- [10] clap 4.6.7 `Arg::env` (feature `env`; MIT OR Apache-2.0), https://docs.rs/clap/4.6.7/clap/struct.Arg.html#method.env
- [11] directories 6.0.0 `ProjectDirs` (2025-01-12, MIT OR Apache-2.0), https://docs.rs/directories/6.0.0/directories/struct.ProjectDirs.html
- [12] etcetera 0.11.0 (2025-10-28, MIT OR Apache-2.0), https://docs.rs/etcetera/0.11.0/etcetera/
- [13] tracing-subscriber 0.3.23 `fmt::format::Json` (features `fmt`,`json`,`std`; MIT), https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/fmt/format/struct.Json.html
- [14] axum 0.8.9 `Serve::with_graceful_shutdown` (MIT), https://docs.rs/axum/0.8.9/axum/serve/struct.Serve.html
- [15] axum example graceful-shutdown, https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
- [16] metrics-exporter-prometheus 0.18.3 (2026-04-30, MIT AND Apache-2.0), https://docs.rs/metrics-exporter-prometheus/0.18.3/
- [17] tempfile 3.27.0 `NamedTempFile::persist` (MIT OR Apache-2.0), https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html
- [18] atomicwrites 0.4.4 (2024-09-19, MIT), https://crates.io/crates/atomicwrites
- [19] fs4 1.1.0 (2026-04-28, MIT OR Apache-2.0), https://crates.io/crates/fs4
- [20] Rust std `File::lock`/`try_lock` (stable since 1.89.0), https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock
- [21] Rust std `fs::rename`, https://doc.rust-lang.org/std/fs/fn.rename.html
- [22] rename(2), Linux man-pages, https://man7.org/linux/man-pages/man2/rename.2.html
- [23] J. Corbet, "Ensuring data reaches disk", LWN.net, 2011, https://lwn.net/Articles/457667/
- [24] SQLite, "Write-Ahead Logging", https://www.sqlite.org/wal.html
- [25] SQLite, PRAGMA synchronous / user_version / busy_timeout, https://www.sqlite.org/pragma.html
- [26] tokio-util 0.7.20 `CancellationToken` (MIT), https://docs.rs/tokio-util/0.7.20/tokio_util/sync/struct.CancellationToken.html
- [27] tokio `signal::windows` (ctrl_c, ctrl_break, ctrl_close, ctrl_logoff, ctrl_shutdown), https://docs.rs/tokio/latest/tokio/signal/windows/index.html
- [28] systemd.service(5), https://man7.org/linux/man-pages/man5/systemd.service.5.html
- [29] sd-notify 0.5.0 (2026-03-09, MIT OR Apache-2.0), https://docs.rs/sd-notify/0.5.0/sd_notify/
- [30] tower-http 0.7.1 `CorsLayer` (MIT; "No headers are sent by default"), https://docs.rs/tower-http/0.7.1/tower_http/cors/struct.CorsLayer.html
- [31] secrecy 0.10.3 (Apache-2.0 OR MIT), https://docs.rs/secrecy/0.10.3/secrecy/
- [32] rust-embed 8.13.0 (2026-10-07), https://docs.rs/rust-embed/8.13.0/rust_embed/
- [33] rusqlite 0.40.2 (2026-08-08, MIT), https://crates.io/crates/rusqlite
- [34] cargo-cyclonedx 0.5.9 (2026-03-19), https://crates.io/crates/cargo-cyclonedx
- [35] cargo-auditable 0.7.7 (2026-10-02), https://crates.io/crates/cargo-auditable
- [36] tokio 1.53.3 (2026-10-13), https://crates.io/crates/tokio

## Implementation status (2026-10-15, after the product build)

The matrix above was written before code existed ("not started"). Current state:

| Factor | Status | Evidence / verification |
|---|---|---|
| I Codebase | Implemented | One repo: Rust workspace (7 crates), `web/`, tests, benchmarks, docs. |
| II Dependencies | Implemented | `Cargo.lock`, `web/package-lock.json`; runtime deps of the binary: libc, libm, libgcc_s only (`ldd target/dist/niche`). SBOM generation not yet automated (deviation). |
| III Config | Partially | CLI flag > `NICHE_*` env > default implemented (`niche --help`); TOML file layer and `niche config show` not yet implemented (deviation, documented). No credentials needed. |
| IV Backing services | Implemented (local) | SQLite + filesystem behind `Store`/artifacts modules; remote data sources behind `niche-data`. |
| V Build/release/run | Implemented | `scripts/build-release.sh` (npm build → tests → `cargo build --profile dist`); UI embedded; build id in `/api/v1/health`. |
| VI Processes | Implemented | Job state in SQLite; outputs on disk; `result.json` checkpoint per species; resume after restart — test `crates/niche-app/tests/recovery.rs`. |
| VII Port binding | Implemented | Axum on `NICHE_HOST:NICHE_PORT` (default 127.0.0.1:8080); Host/Origin guards; `NICHE_ALLOWED_HOSTS` for reverse proxies. |
| VIII Concurrency | Implemented | Tokio for HTTP; one worker thread per instance; Rayon pool sized by `NICHE_MAX_WORKERS`; GBIF limited to 2 connections. |
| IX Disposability | Implemented | SIGINT/SIGTERM → graceful HTTP shutdown, SSE streams end, running job checkpointed and requeued; atomic writes (tmp → fsync → rename). |
| X Dev/prod parity | Implemented | Same binary and API in dev; Vite only proxies; `NICHE_UI_DIR` to serve a dev build. |
| XI Logs | Implemented | `tracing` to stderr, `NICHE_LOG_FORMAT=json`; job/species ids and stage timings in spans/events; coordinates not logged. Metrics endpoint not implemented (not yet justified). |
| XII Admin processes | Partially | `niche doctor`, `cache inspect/clean`, `run`, `demo`, `export-swd`; `migrate` is implicit (schema version check at open); `models verify`/`datasets validate` not yet. |
