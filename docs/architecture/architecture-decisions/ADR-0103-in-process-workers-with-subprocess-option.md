# ADR-0103: In-process bounded workers by default, optional subprocess isolation

- Status: Proposed (2026-10-15)
- Related: tension 2 and factor VIII in [twelve-factor-compliance.md](../twelve-factor-compliance.md); [job-lifecycle.md](../job-lifecycle.md)

## Context

CPU-heavy model training/projection must not starve the HTTP/SSE server, must be bounded, and must be
cancellable. Some failure modes (panic in native code, memory blow-up, non-interruptible native calls)
can take down the whole process if workers are in-process. The product must remain one executable.

## Decision

1. Default `NICHE_JOB_ISOLATION=in-process`: a scheduler on Tokio admits ≤ `max_workers` jobs (and
   memory estimate ≤ `max_memory_mb`); CPU work runs on **one shared Rayon pool** of `cpu_threads`
   threads; never on Tokio worker threads. Cancellation via `tokio_util::sync::CancellationToken`
   hierarchy (root → job → unit) [26], polled at safe points. Panics are caught per job
   (`catch_unwind` at the job boundary) → job `failed(E_INTERNAL_PANIC)`.
2. Option `subprocess`: the same binary re-executes itself as hidden `niche worker --job <id>
   --attempt <n>`; communication over stdin/stdout with length-prefixed JSON messages (progress,
   checkpoint-committed, done/error); stderr carries JSON logs. Child writes artifacts with the same
   atomic protocol and its own SQLite connection (same host, WAL-safe [24]) — or reports to parent for
   DB writes [OPEN: pick one; parent-writes keeps a single writer]. Child exits when stdin closes.
   Cancellation = message, then SIGTERM, then SIGKILL after 5 s.
3. Recommended: in-process for Local and Batch; consider subprocess for Server once native geospatial
   libs are linked (tension 8) [REC].

## Alternatives

| Alternative | Why not |
|---|---|
| Always subprocess | Spawn + IPC overhead per job, harder debugging; unnecessary for pure-Rust engine. |
| Separate worker binary/service | Breaks single-executable goal; deployment complexity. |
| External queue + worker fleet | Cloud pattern; out of scope (tension 6). |
| Per-job Rayon pools | Oversubscription (jobs × threads > cores). |

## Evidence

12factor concurrency: process types, rely on OS process manager [4]; tokio-util CancellationToken
semantics (child cancelled with parent, not vice versa) [26].

## Trade-offs

+ Simple, fast, low memory by default; isolation available when needed without new artifact.
− Two execution paths; IPC protocol must be versioned and tested; in-process abort-type failures
  (OOM kill, segfault in native code) still kill the server — recovered by restart path.

## Risks

Shared Rayon pool means one huge job can delay others' parallel sections — mitigated by admission
control and per-unit task granularity.

## Reversibility

High: the job runner interface (`run(job, ctx) -> Result`) is identical for both modes.

## Rationale

Keeps the brief's "one executable, bounded concurrency, no oversubscription" while leaving a
process-isolation escape hatch where it is justified rather than paying for it everywhere.

## References

Ids as in [twelve-factor-compliance.md § References](../twelve-factor-compliance.md#references).
