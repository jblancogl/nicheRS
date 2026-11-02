# Observability and Admin Subcommands

Status: proposal. Date: 2026-10-15. Implementation: not started — research phase.

## Scope

Tracing spans and fields, log levels and formats, coordinate redaction, health/readiness, optional
Prometheus metrics, `niche doctor`, and the admin subcommand specification (factor XII).

## 1. Logging pipeline

- [FACT] 12factor: each process "writes its event stream, unbuffered, to stdout" and "never concerns
  itself with routing or storage of its output stream" [6].
- [REC] `tracing` + `tracing-subscriber` 0.3.23 with features `fmt`, `json`, `env-filter` [13].
  Human output → **stderr** (keeps stdout clean for CLI machine output such as `--format json`); in
  `serve` mode both are fine, we use stderr uniformly so `niche run ... > result.json` works.
  [Deviation note: 12factor says stdout; stderr is equally captured by journald/containers.]
- Format: `NICHE_LOG_FORMAT=auto` → pretty when stderr is a TTY, JSON lines otherwise. JSON uses
  `with_current_span(true)`, `with_span_list(true)`, `flatten_event(true)` [13] so `job_id` etc. are
  queryable without nested parsing.
- Filter: `NICHE_LOG_LEVEL` in `EnvFilter` syntax; `-v` = debug for `niche*` targets, `-vv` = trace,
  `-q` = warn.
- Optional `NICHE_LOG_FILE` (Local mode only): non-blocking appender with daily rotation, 7 files kept
  [REC]; crate choice deferred (`tracing-appender` [UNVERIFIED – not checked on crates.io in this pass]).

## 2. Levels

| Level | Use |
|---|---|
| error | Job failed, artifact corrupt, migration failed, unrecoverable IO. Always includes `error_code`. |
| warn | Retries, degraded mode (network FS, cache over quota), deprecated config, near memory budget. |
| info | Lifecycle: startup summary (version, mode, bind, data dir, schema), job state transitions, stage start/end with `duration_ms`, shutdown phases. |
| debug | Per-unit progress, cache hits/misses, SQL timings > 50 ms, HTTP request details. |
| trace | Inner loops (optimiser iterations) — sampled; never on by default. |

## 3. Spans and fields

| Span | Fields | Parent |
|---|---|---|
| `http.request` | `request_id` (ULID, echoed as `X-Request-Id`), `method`, `route` (template, not raw path), `status`, `duration_ms`, `user` (server mode, hashed/opaque id) | root |
| `job` | `job_id`, `spec_hash` (first 12 hex), `attempt`, `mode` | root (worker) |
| `stage` | `job_id`, `stage`, `duration_ms`, `units_total`, `units_done` | `job` |
| `model` | `model_id`, `species_key` (taxon key, e.g. GBIF `taxonKey`), `algorithm`, `unit_key` | `stage` |
| `fetch` | `source` (gbif, chelsa, local…), `bytes`, `cache_hit`, `retry` | `stage` |
| `db.tx` | `op`, `duration_ms` (debug only) | any |
| `artifact.commit` | `artifact_id`, `kind`, `size_bytes`, `sha256` (short) | `stage` |

- `duration_ms` is recorded on span close (custom layer or `FmtSpan::CLOSE` timing) [REC].
- Subprocess workers (ADR-0103) emit JSON lines on their stderr; the parent re-emits them with the
  child's span context (`job_id`, `attempt`, `worker_pid`).
- The per-job event log shown in the UI is a **separate** product feature persisted in SQLite
  (`job_event`), fed from the same instrumentation via a layer filtered on `job_id`.

## 4. Redaction of sensitive data

Occurrence coordinates of protected species are sensitive (e.g. nesting sites); consultancies may be
under NDA. Policy [REC]:

1. Coordinates live in types (`LonLat`, `Geometry`, `OccurrenceSet`) whose `Debug`/`Display` print
   only counts and a redacted marker (`LonLat(<redacted>)`, `OccurrenceSet{n=1432, crs=EPSG:4326}`).
   No `#[derive(Debug)]` on raw coordinate structs — enforced by a clippy `disallowed_types`/review rule.
2. Study-area bbox may be logged only at `debug`, rounded to 1° and only if
   `NICHE_LOG_REDACT_COORDS=false`.
3. Error messages carry row numbers / record ids, not coordinates ("row 214: latitude out of range").
4. Request bodies are never logged; query strings are logged with values stripped.
5. Secrets: `secrecy::SecretString` [31]; tokens never logged; `Authorization` header stripped.
6. Test: property test feeding random coordinates through the pipeline asserts none appear (as decimal
   strings at ≥ 3 decimals) in captured `info`/`debug` logs.

## 5. Health and readiness

| Endpoint | Semantics | Checks | Auth |
|---|---|---|---|
| `GET /healthz` | liveness: process responsive | none beyond responding; 200 `{"status":"ok","version":...}` | none (no sensitive data) |
| `GET /readyz` | ready to accept jobs | startup recovery done; DB reachable (`SELECT 1`) & schema current; data/cache dir writable; not shutting down; queue below `max_queued` | none; body detail only when authenticated |
| `GET /api/status` | UI dashboard | workers busy/total, queue length, memory budget used, cache size, version | authenticated |

Under systemd `Type=notify`, `READY=1` is sent when `/readyz` would first return 200 [28][29].

## 6. Metrics (optional)

[REC] Off by default (`NICHE_METRICS=false`); justified only in Server mode with an existing Prometheus.
Implementation: `metrics` facade + `metrics-exporter-prometheus` 0.18.3 using `install_recorder()` (no
own HTTP listener; we serve `PrometheusHandle` output on our Axum `/metrics`; requires calling
`run_upkeep` periodically) [16]. `/metrics` requires auth unless bound to loopback.

| Metric | Type | Labels |
|---|---|---|
| `niche_jobs_total` | counter | `outcome` (succeeded/failed/cancelled) |
| `niche_jobs_in_state` | gauge | `state` |
| `niche_stage_duration_seconds` | histogram | `stage`, `algorithm` |
| `niche_job_interruptions_total` | counter | `reason` (shutdown/crash) |
| `niche_fetch_bytes_total` | counter | `source`, `cache_hit` |
| `niche_cache_bytes` | gauge | `kind` |
| `niche_http_requests_total` / `_duration_seconds` | counter / histogram | `route`, `status` |
| `niche_worker_busy` | gauge | — |

No `species_key`/`job_id` labels (cardinality, and species lists can be confidential).

## 7. Admin subcommands

Common: same config resolution as `serve` (factor XII: "same codebase and config" [7]); `--format
table|json`; exit codes: 0 ok, 1 check failed / findings, 2 usage, 75 data dir locked, 78 config error.
"Lock" = needs exclusive data-dir lock (fails if a server is running on that data dir).

| Command | Purpose | Key flags | Lock |
|---|---|---|---|
| `niche doctor` | Environment diagnosis (below) | `--fix` (only safe fixes: create dirs, sweep temp), `--network` | read-only (no) / `--fix` (yes) |
| `niche migrate` | Apply DB schema + on-disk layout migrations | `--dry-run`, `--to <ver>`, `--no-backup` (refused in server mode) | yes |
| `niche cache inspect` | Size per kind/source/age, top entries, orphans | `--by source|kind|age` | no |
| `niche cache clean` | Remove regenerable cache entries | `--older-than 30d`, `--kind rasters`, `--auto` (to quota), `--dry-run`; never touches artifacts referenced by `succeeded` jobs | yes |
| `niche datasets validate <path|id>` | Validate occurrence files / rasters: schema, CRS, extent, nodata, resolution alignment, duplicate records | `--crs`, `--against <experiment>` | no |
| `niche models verify [model_id…]` | Recompute SHA-256 of all artifacts of succeeded models vs DB; check manifest completeness & format version | `--all`, `--repair-index` (re-register files from manifests, never rewrites content) | no (`--repair-index` yes) |
| `niche benchmark` | Run fixed reference workloads (bundled small datasets) and print timings + environment | `--suite quick|full`, `--threads`, `--out <file>` | no (uses temp data dir) |
| `niche config show|validate` | See configuration.md | | no |
| `niche probe --live|--ready` | HTTP GET to own `/healthz`/`/readyz` for container healthchecks (no curl in image) | `--url` | no |

### `niche doctor` checks

| Check | Pass condition | Severity |
|---|---|---|
| Version/build | prints version, commit, target, schema versions | info |
| Config | resolves and validates; lists sources | error |
| Data dir | exists/creatable, writable, free space ≥ 5 GB [REC], **local filesystem** (Linux `statfs` magic not NFS/CIFS/SMB; Windows not UNC/mapped network) — SQLite WAL needs same host [24] | error (server) / warn (local) |
| Cache dir | writable, size vs quota | warn |
| Same-volume staging | `cas/tmp/` on same filesystem as `cas/` (rename requirement [22]) | error |
| Lock | lock free or held by a live process (reports pid) | info |
| SQLite | `PRAGMA integrity_check` (quick by default), journal_mode=wal, schema current | error |
| Orphans | temp/staging files, artifact rows without files | warn |
| Resources | cores, memory vs `max_memory_mb`, `cpu_threads × max_workers` sanity | warn |
| Native libs | versions of any linked geospatial libs (if tension 8 resolves that way) | info |
| Network (`--network`) | reach configured providers (GBIF API, raster catalogues), proxy settings, TLS | warn |
| Server-mode safety | non-loopback bind ⇒ auth ≠ none; `allowed_hosts` set; secret file perms | error |
| Shutdown timing | `shutdown_grace_secs` < systemd `TimeoutStopUSec` when `INVOCATION_ID` set | warn |
| Clock | system time sane (not before build date) | warn |

## Alternatives

- OpenTelemetry OTLP export — deferred; adds deps and a collector; JSON logs + optional Prometheus cover
  single-node needs. Can be added as a `tracing` layer later without code changes in the engine.
- Log to file by default — rejected (factor XI), opt-in for desktop only.

## Risks

- Redaction relies on type discipline; a stray `format!("{lat}")` leaks. Mitigation: lint + property test.
- High-volume trace logs inside optimiser loops can dominate runtime; trace sampling and `enabled!` guards.

## Open questions

- [OPEN] Should the UI expose a "download diagnostics bundle" (doctor output + redacted logs) for support?

## References

Ids as in [twelve-factor-compliance.md § References](twelve-factor-compliance.md#references).
