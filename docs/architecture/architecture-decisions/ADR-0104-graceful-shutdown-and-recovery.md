# ADR-0104: Graceful shutdown by interruption + checkpoint-based recovery

- Status: Proposed (2026-10-15)
- Related: [job-lifecycle.md](../job-lifecycle.md) §5–§6, tension 7, factor IX

## Context

Model training can take minutes to hours for hundreds of species; service managers send SIGTERM and
SIGKILL after a timeout [28]; desktop users close windows; machines crash. 12factor asks for graceful
shutdown by "returning the current job to the work queue", robustness "against sudden death", and
reentrant jobs [5]. Completed results must never be silently lost.

## Decision

1. Shutdown **interrupts** jobs (it does not wait for completion and does not cancel them): stop
   admitting → mark not-ready → stop HTTP accept (axum `with_graceful_shutdown` [14]) while actively
   ending SSE streams → cancel root token with reason `Shutdown` → workers stop at the next safe point,
   discard uncommitted output, record `interrupted(shutdown)` and requeue → after
   `NICHE_SHUTDOWN_GRACE_SECS` (default 30) exit regardless.
2. Recovery is the **primary** correctness path, shutdown only an optimisation: on startup (under the
   data-dir lock) every non-terminal job is reconciled and requeued from its last valid stage/unit
   checkpoint, bounded by `max_attempts`.
3. Checkpoint granularity: stage × model unit, with `input_hash` including engine version.
4. Artifacts committed with the atomic protocol; file-then-DB ordering; orphan sweep at startup.
5. Signals: Unix SIGTERM/SIGINT; Windows ctrl_c/ctrl_close/ctrl_shutdown/ctrl_logoff [27]; second
   signal → immediate exit. systemd `Type=notify` with `READY=1`/`STOPPING=1` via `sd-notify` [29].

## Alternatives

| Alternative | Why not |
|---|---|
| Wait for running jobs to finish | Unbounded stop time → SIGKILL anyway; bad UX. |
| Cancel jobs on shutdown (terminal) | Loses user intent; requires manual resubmission. |
| Mid-iteration optimiser snapshots | Complex, format-coupled; revisit if benchmarks show long single-unit training. |

## Evidence

12factor disposability [5]; axum graceful-shutdown example notes it waits for outstanding requests and
adds a timeout layer [15]; CancellationToken propagation [26]; systemd stop semantics [28].

## Trade-offs

+ Bounded shutdown time; no completed result lost; same code path for crash and shutdown.
− Work since last checkpoint redone; engine code must poll cancellation.

## Risks

Non-cooperative native calls ignore tokens → use subprocess isolation (ADR-0103). Windows console close
events may allow only a short time [UNVERIFIED – prior knowledge] → rely on recovery.

## Reversibility

High for policy (grace, max attempts); checkpoint schema changes need migrations.

## Rationale

Makes crash recovery and graceful shutdown one mechanism, testable by `kill -9` chaos tests, which is
the only way to credibly claim "jobs survive restarts".

## Verification

Integration tests: SIGTERM and SIGKILL in each stage; assert job reaches `succeeded` after restart with
artifact hashes identical to an uninterrupted run (same seeds); assert no committed artifact disappears.

## References

Ids as in [twelve-factor-compliance.md § References](../twelve-factor-compliance.md#references).
