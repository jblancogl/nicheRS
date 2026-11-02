# Product-owner constraint: Twelve-Factor (adapted)

NicheRS follows https://12factor.net/ adapted to a local-first scientific instrument that can also run
as a server. NOT microservices/Kubernetes/cloud. Goals: one monorepo; one Rust executable per
platform; embedded React (shadcn/ui) frontend; reproducible science; minimal runtime deps; explicit
config; observable & recoverable execution; local-first with optional server deployment.

Per factor: I Codebase (monorepo, multiple deploy modes) · II Dependencies (Cargo.lock + frontend
lockfile, no undocumented native deps, SBOM) · III Config (env vars NICHE_HOST, NICHE_PORT,
NICHE_DATA_DIR, NICHE_CACHE_DIR, NICHE_LOG_LEVEL, NICHE_MAX_WORKERS, NICHE_MAX_MEMORY_MB; validated
config file for local prefs and reproducible experiment definitions; documented precedence &
defaults; no credentials/paths embedded) · IV Backing services (FS + SQLite default behind
interfaces; future object storage/external DB; no Redis/Postgres/cloud without need) · V
Build/release/run (frontend embedded at Rust build; prod needs no Node; version executable, data
schemas, models, reproducibility metadata) · VI Processes (durable job state in FS/SQLite; recovery
of interrupted jobs; bounded worker pool or child processes when justified) · VII Port binding
(Axum serves UI+API; loopback default; reverse-proxy supported) · VIII Concurrency (bounded; Tokio
for IO, Rayon/dedicated workers for CPU; no oversubscription) · IX Disposability (fast start,
graceful shutdown, job cancellation, atomic artifact writes, crash-safe metadata, recovery,
cleanup; never silently lose completed results) · X Dev/prod parity (Vite only in dev) · XI Logs
(tracing, stdout/stderr, structured, job/model/stage ids, timings; no sensitive coordinates; health
+ metrics where justified) · XII Admin processes as subcommands: niche doctor, migrate, cache
inspect, cache clean, datasets validate, models verify, benchmark.

Tensions to analyse explicitly: (1) stateless services vs local SQLite; (2) single executable vs
process isolation; (3) offline-first vs remote backing services; (4) reproducibility vs environment
config; (5) embedded assets vs frontend dev flexibility; (6) local FS vs horizontal scaling; (7)
graceful shutdown vs long training; (8) portability vs native geospatial libs. Pragmatic decisions,
not dogma.

Deployment modes: Local (standalone exe + embedded UI, local persistence), Server (same exe as
managed service, network exposure/storage/logging/security controls), Batch (headless CLI). All
share engine, config conventions, model formats, validation rules.

Acceptance: no Python/Java/R/Node at runtime; runtime deps documented; config externalized and
validated; jobs & artifacts survive restarts; graceful shutdown & interruption recovery tested;
structured logs; reproducible builds as far as toolchain allows; frontend+API shipped together;
no cloud needed locally; can evolve to hosted service without rewriting the core.
"Build NicheRS as a portable scientific instrument that can also operate as a production-grade
service—not as a cloud application that happens to run locally."
