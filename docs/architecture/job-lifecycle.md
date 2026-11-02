# Job Lifecycle, Checkpoints and Recovery

Status: proposal. Date: 2026-10-15. Implementation: not started — research phase.

> **Update 2026-10-16 (implemented, ADR-0105).** A queued job now also has a *data-preparation
> state* (`pending → preparing → ready | nobatch`, column `jobs.data_state`). A separate I/O thread
> requests and waits for the GBIF batch download; the compute worker only takes jobs whose data is
> `ready` (batch cached) or `nobatch` (preparation failed: per-species search API). An interrupted
> preparation returns to `pending` on restart. Among prepared jobs, organisations are served in
> turn. `DELETE /api/v1/jobs/{id}` removes finished jobs; a retention sweep deletes finished jobs
> older than `NICHE_RETENTION_DAYS`.
>
> **Update 2026-10-19 (implemented, ADR-0104 checkpoints).** Restarts (deployments) lose almost
> no work. Each cross-validation model (settings × fold) is saved as soon as it is trained in
> `jobs/<id>/checkpoints/<species>/cv-<combo>-<fold>.json` (numbers stored by their bits); a
> resumed species reuses them only when a fingerprint of its inputs (cells, environmental values,
> folds, grid, `checkpoint::CV_REVISION`) matches, so its results are bit-identical to an
> uninterrupted run (test `recovery.rs`), and the study log says how many models were reused.
> Checkpoints are removed once the species result is written. A GBIF download request keeps its
> key on disk (`<cache>/gbif-downloads/<id>.pending`) from the moment GBIF assigns it, so a restart
> waits for the same download instead of queueing a new one.
>
> **Update 2026-10-20 (implemented, ADR-0016 projections).** A species with future-climate
> projections writes its present outputs (rasters, models, curves, occurrences) first, then saves
> `checkpoints/<species>/projection-state.json` (result so far, final model with numbers stored by
> their bits, training cells, P10 threshold, fingerprint of the spec, species and grid), then each
> scenario × GCM run as `proj-<period>-<ssp>-<gcm>.bin` as soon as it is predicted. A species
> interrupted during its projections resumes from that state without fitting again (only the
> missing runs are computed; the study log says so) and gives byte-identical outputs (test
> `projections.rs`). Projection files, `future_areas.csv` and the reports are written after the
> last run; `result.json` is still written last and remains the species' completion marker.
>
> **Update 2026-10-19 (implemented).** While `preparing`, the engine keeps the GBIF wait in memory
> (`GET /api/v1/jobs/{id}/gbif-wait`: download key, GBIF status, since, last check every 10 s,
> link to gbif.org), so the interface shows a live wait instead of a frozen bar. GBIF usually
> takes 2–15 min (observed 23 min): past 20 min the interface says GBIF is slow; past 30 min the
> owning organisation may `POST /api/v1/jobs/{id}/skip-download`, which abandons the download and
> sets `nobatch` — occurrences then come from the GBIF search API per species (no DOI, at most
> `NICHE_MAX_RECORDS` per species, spatially stratified). The choice is logged as a warning event
> and the source is recorded in the results, as for a failed preparation.
Related: ADR-0102 (SQLite + FS state), ADR-0103 (workers), ADR-0104 (shutdown/recovery),
`docs/research/11-backend-architecture.md` and `docs/architecture/api-design.md` (HTTP/SSE surface,
written concurrently — the state names below are the canonical set; any divergence must be reconciled
in favour of one list).

**Consistency with 11-backend-architecture.md / api-design.md (read 2026-10-15):**
- Job `state` enum is the API's closed enum: `queued, preparing, running, finalizing, cancelling,
  succeeded, failed, cancelled`. The brief's "succeeding" = `finalizing`. `interrupted` is **not** a
  job state: it is recorded on the in-flight *step* and as a `job_event` (`job.recovered`), after which
  the job is `queued` again with `attempt += 1` (same as backend §2.5).
- "Stage" here = a named phase reported to users (`running(stage)` via `current_stage`); the backend's
  **steps** (DAG nodes, e.g. `train fold k`, `predict tile range`) are the unit of checkpointing. A stage
  groups steps; checkpoints below are per step.
- `spec_hash` = hash of the canonical, defaults-materialised experiment spec (backend §2.9).
- Artifacts live in the backend's content-addressed store (`cas/`, staging in `cas/tmp/`); the atomic
  protocol in §4 is the same one, stated in more detail.
- **Divergences to resolve:** (a) SQLite `synchronous`: backend proposes `NORMAL`, this doc and ADR-0102
  propose `FULL` (see ADR-0102 for rationale); (b) content hash: backend proposes BLAKE3 for CAS ids,
  `execution-manifest.md` uses SHA-256 for manifests — this doc says "content hash" and defers the
  algorithm to that resolution.

## Scope

States and transitions of a job, what is durable after each stage, the atomic artifact protocol,
restart recovery, graceful shutdown, idempotency and retries. Applies identically to Local, Server and
Batch modes.

## 1. Concepts

- **Job**: one execution of one materialised experiment definition (`spec_hash`) over 1..N
  species. A multi-species experiment is one job with N **model units** (`model_id`, one per species ×
  algorithm × partition config), so stages iterate over units and checkpoint per unit.
- **Stage**: an ordered, named step with declared inputs and outputs. Proposed list (to align with the
  engine research): `resolve_inputs` → `fetch_occurrences` → `fetch_predictors` → `prepare` (cleaning,
  thinning, background sampling) → `partition` → `train` → `evaluate` → `project` → `summarise`
  (thresholds, suitable area, uncertainty) → `report`.
- **Attempt**: one execution try of a job by a worker; `attempt` increments on requeue.
- **Checkpoint**: a committed stage (or stage × model unit) output, recorded in SQLite with content hash.

## 2. State machine

```
             submit
               │
               ▼
           ┌────────┐  cancel   ┌───────────┐
           │ queued │──────────▶│ cancelled │ (terminal)
           └───┬────┘           └───────────┘
       admitted│ (worker slot + memory budget)       ▲
               ▼                                     │
          ┌───────────┐   cancel   ┌────────────┐    │
          │ preparing │──────────▶│ cancelling │────┘ (worker acknowledges, cleanup done)
          └────┬──────┘           └────────────┘
               │ inputs resolved         ▲
               ▼                         │ cancel
          ┌─────────────────┐────────────┘
          │ running(stage)  │──── error ───────────────▶ failed (terminal)
          └────┬────────────┘
               │ last stage done                  shutdown / crash detected at startup
               ▼                                             │
          ┌────────────┐                                     ▼
          │ finalizing │  (final commit)        ┌──────────────────────┐
          └────┬───────┘                        │ interrupted (transient│
               ▼                                │ record, attempt n)    │
          succeeded (terminal)                  └──────────┬───────────┘
                                                attempt < max │ attempt ≥ max
                                                   ▼          ▼
                                       queued (requeued,   failed(E_INTERRUPTED_MAX)
                                       resume_from=stage)
```

| State | Meaning | Durable effects on entry | Allowed next |
|---|---|---|---|
| `queued` | Accepted and persisted; waiting for admission | job row + canonical experiment JSON committed **before** HTTP 202 / CLI ack | preparing, cancelled |
| `preparing` | Worker assigned; resolving inputs, estimating memory, creating `jobs/<id>/` | `attempt += 1`, `started_at`, worker id | running, failed, cancelling, interrupted |
| `running` | Executing `current_stage` (column) | per-stage row `stage_run(job_id, stage, unit, status, started_at)` | running (next stage), finalizing, failed, cancelling, interrupted |
| `finalizing` | All stages done; writing manifest and flipping artifacts to final | manifest written via atomic protocol | succeeded, interrupted (recovery re-verifies) |
| `succeeded` | Terminal; all artifacts committed and hash-verified | `finished_at`, `manifest_sha256` | — (immutable; re-run = new job) |
| `failed` | Terminal; error code + message + failing stage | `error_code`, `error_stage`, `error_detail` (redacted) | — (user may "retry" = new attempt via explicit API, see §7) |
| `cancelling` | User cancel requested; worker winding down | `cancel_requested_at` | cancelled |
| `cancelled` | Terminal; user intent | partial uncommitted outputs removed; committed stage checkpoints *kept* for cache reuse until GC | — |
| `interrupted` | Not a resting state: recorded event when a non-terminal job lost its worker (shutdown or crash) | `job_event(kind=interrupted, attempt, reason=shutdown|crash)` | queued (requeued) or failed |

Invariants [REC]:

1. Every transition is a single SQLite transaction guarded by `WHERE state = <expected>` (compare-and-set);
   0 rows updated = lost race → re-read.
2. Terminal states never change. `succeeded` artifacts are immutable.
3. A job is never acknowledged before its `queued` row is committed with `synchronous=FULL`
   (ADR-0102), so an acknowledged job survives power loss.
4. `cancelling` vs `interrupted` are distinct: user cancel is terminal; shutdown/crash is requeued.

## 3. Durable checkpoints per stage

Stages and their step-level checkpoints:

| Stage | Checkpoint (committed outputs, one per step) | Resume rule |
|---|---|---|
| resolve_inputs | resolved dataset versions + hashes (DB rows) | redo (cheap) |
| fetch_occurrences | raw occurrence snapshot per species in cache (content-addressed) + query provenance | skip if snapshot hash recorded and file verifies |
| fetch_predictors | raster tiles in cache (content-addressed) | skip per tile |
| prepare | cleaned occurrences, background points (Parquet/CSV + hash) per unit | skip per unit |
| partition | fold assignments per unit | skip per unit |
| train | model artifacts per unit (CAS blobs, e.g. `model.json`) | skip per unit; an in-progress unit restarts from scratch |
| evaluate | metrics JSON per unit | skip per unit |
| project | suitability raster per unit (COG/GeoTIFF) | skip per unit |
| summarise | thresholds, areas, uncertainty per unit + experiment-level table | skip per unit |
| report | report files, final manifest | redo |

- Each checkpoint row: `(job_id, stage, unit_key, input_hash, output_artifact_ids, completed_at)`.
  `input_hash` = hash of (spec_hash, upstream checkpoint hashes, engine version). A checkpoint is
  reused only if `input_hash` matches — so an engine upgrade between attempts invalidates checkpoints
  [REC] (reproducibility over speed; the manifest must reflect one engine version).
- Determinism: random seeds derive from `(experiment seed, unit_key, stage)`, so a re-run unit gives the
  same output as the interrupted one would have [REC].

## 4. Atomic artifact write protocol

For every file artifact:

1. Create temp file **in the target directory** (`tempfile::NamedTempFile::new_in(dir)` [17]) — same
   filesystem is mandatory: `rename` fails with `EXDEV` across mounts [22] and tempfile "cannot be
   persisted across filesystems" [17].
2. Stream-write content while computing the content hash on the fly.
3. `file.sync_all()` (fsync data + metadata). [FACT] tempfile's `persist` does **not** sync the file or
   directory [17], so we do it explicitly.
4. `persist(final_path)` → `rename(2)`, which atomically replaces the target so that "there is no point
   at which another process ... will find it missing" [22]. Use `persist_noclobber` for content-addressed
   paths that must never be overwritten.
5. fsync the containing directory (Unix: open dir, `sync_all`). Sequence per LWN [23]. On Windows,
   directory fsync is not available through `std` [ASSUMPTION — to verify]; `std::fs::rename` uses
   `MoveFileExW` with a fallback to `SetFileInformationByHandle`, flags not documented [21] → documented
   weaker durability guarantee on Windows; the recovery scan (§5) compensates.
6. SQLite transaction: insert the `artifacts` row (hash, size, media_type, created_at) and the
   `job_artifact`/`steps.output_hash` link (backend §2.4–2.5) — **in the same transaction**.

Ordering rule: **file durable first, DB row second**. Therefore after any crash:

| Observed at recovery | Meaning | Action |
|---|---|---|
| file without DB row | crash between 5 and 6 | delete (or move to `cas/tmp/orphans/` in debug) |
| `*.tmp`/`.tmpXXXX` file | crash before 4 | delete |
| DB row without file, or hash mismatch | external tampering / disk fault | mark artifact `missing`/`corrupt`; invalidate dependent checkpoints; job → requeue if non-terminal; if `succeeded`, flag in `models verify` and UI (never silently "fix") |

Multi-file artifacts (model bundle directory, tiled outputs): write into `cas/tmp/<uuid>/`,
fsync every file and the dir, then `rename` the directory to its final name (target must not exist),
fsync parent, then the DB transaction. With a CAS, most artifacts are single blobs and this case is rare. Crate choice: `tempfile` 3.27.0 [17] (actively released); not
`atomicwrites` 0.4.4 [18] (last release 2024-09) — a 40-line helper over tempfile covers our needs.

## 5. Restart recovery (startup sequence)

Runs before the HTTP listener reports ready (`/readyz` = 503 until done).

1. Acquire `data_dir/niche.lock` with `File::try_lock` (std ≥ 1.89 [20]); if `WouldBlock` → exit 75
   ("another niche process uses this data dir"). Makes the following steps race-free without leases.
2. Open SQLite; check schema version. Behind → Local: back up and auto-migrate; Server: refuse unless
   `niche migrate` was run (or `NICHE_AUTO_MIGRATE=true`) [REC].
3. Sweep: temp files and `cas/tmp/` entries (§4 table).
4. For each non-terminal job:
   - `queued` → unchanged.
   - `cancelling` → `cancelled` (cleanup uncommitted outputs).
   - `finalizing` → verify all artifacts + manifest; complete → `succeeded`; else treat as `running`.
   - `preparing` / `running` → record `interrupted(reason=crash|shutdown)`; if `attempt < max_attempts`
     → `queued` with `resume_from` = first stage/unit without valid checkpoint; else `failed`
     (`E_INTERRUPTED_MAX`). `reason=shutdown` if the previous run wrote a clean-shutdown marker for that
     job (§6), else `crash`.
5. Verify a sample (or all, configurable) of checkpoint hashes referenced by requeued jobs.
6. Start scheduler; set ready; `sd_notify READY=1` when under systemd [29].

Subprocess mode: children are spawned with a stdin pipe; a child exits when the pipe closes, so a dead
parent leaves no orphan workers writing to the data dir [REC]; additionally children take a shared
lock on a per-job lock file and the parent checks it before requeueing.

## 6. Graceful shutdown semantics

Triggers: SIGTERM, SIGINT (Unix); `ctrl_c`, `ctrl_close`, `ctrl_shutdown`, `ctrl_logoff` (Windows, tokio
[27]); a "Quit" action in the local UI [REC; endpoint not yet in api-design.md]. Sequence:

| t | Action |
|---|---|
| 0 | `readyz` → 503; `sd_notify STOPPING=1`; scheduler stops admitting; `POST /jobs` → 503 problem `shutting-down` + `Retry-After` (api-design.md). |
| 0 | `axum::serve(..).with_graceful_shutdown(signal)` stops accepting connections and waits for in-flight requests [14][15]. **SSE/streaming responses must observe the shutdown token and end their stream**, otherwise graceful shutdown waits on them indefinitely (the axum example adds a request timeout for exactly this reason [15]). |
| 0 | Root `CancellationToken` cancelled with reason `Shutdown`; child tokens per job/unit propagate downwards [26]. Engine loops poll `is_cancelled()` at safe points (per optimiser iteration batch, per raster tile, per species). |
| 0..grace | Workers: stop at next safe point, discard uncommitted partial output, write `interrupted(reason=shutdown)` and requeue in one txn. Jobs in `finalizing` are allowed to finish their commit. Already-committed checkpoints/artifacts are untouched. |
| grace (default 30 s) | Remaining workers abandoned (in-process: process exits; subprocess: SIGTERM then SIGKILL after 5 s). Their jobs are handled by startup recovery (§5) as `crash`. |
| exit | WAL checkpoint (`PRAGMA wal_checkpoint(TRUNCATE)` best-effort [24]), release lock, exit 0. A second signal during shutdown → immediate exit 130. |

Guarantee: **no completed (committed) result is lost** in any of these paths, because commit happens at
artifact granularity before the job advances.

## 7. Idempotency and retries

- **Submission idempotency**: `Idempotency-Key` header (API) / `--idempotency-key` (CLI), semantics as
  in api-design.md §2.2: replay with same fingerprint → original response (`Idempotent-Replayed: true`);
  same key + different body → 422; original still in flight → 409.
- **Experiment dedup** (opt-in, `"reuse": "if_identical"`): if a `succeeded` job exists with the same
  `spec_hash` (which includes engine version), return it instead of recomputing. Default: new job, but
  memoised steps make it cheap and bit-identical (backend §2.9).
- **Stage idempotency**: stages are pure functions of their `input_hash`; re-execution overwrites nothing
  (new temp → rename) and content-addressed cache entries use `persist_noclobber`.
- **Automatic retries**: only for (a) interruption (requeue, bounded by `NICHE_JOB_MAX_ATTEMPTS`) and (b)
  transient network errors inside fetch stages (exponential backoff with jitter, max 5 tries per request,
  honour `Retry-After`). Scientific/data errors (`E_TOO_FEW_OCCURRENCES`, `E_CRS_MISMATCH`, …) are **never**
  retried automatically.
- **Manual retry**: `POST /api/v1/jobs/{id}/retry` on `failed`/`cancelled` → `201` with a new job that
  reuses memoised steps (api-design.md §4.5); `succeeded` → `409 conflict-state`.
- Internal (non-transient, non-scientific) step errors: one automatic step retry, then fail (backend §2.8).

## Alternatives

- Ephemeral in-memory queue (lose jobs on restart) — violates acceptance criterion "jobs survive restarts".
- External queue (Redis/RabbitMQ NACK as in 12factor [5]) — unnecessary dependency for single-node.
- Mid-iteration optimiser snapshots — more complex, defer until benchmarks show long `train` stages.

## Risks

- Cooperative cancellation depends on engine code polling tokens; a long native call (e.g. a C library)
  cannot be interrupted → subprocess isolation is the escape hatch (ADR-0103).
- Windows durability of rename without directory fsync is weaker [ASSUMPTION]; covered by recovery scan.

## Open questions

- [OPEN] Granularity of `train` checkpoints for very large multi-species runs (per unit is assumed enough).
- [OPEN] Checkpoint GC policy for `cancelled`/`failed` jobs (age-based via `cache clean`).

## References

Ids as in [twelve-factor-compliance.md § References](twelve-factor-compliance.md#references).
