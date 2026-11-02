# ADR-0115 — Deployments without downtime: leader lease and shared job state

- Status: Accepted (2026-10-19). Product decision (session of 2026-10-19):
  "production must always be available, deployments included".
- Amends ADR-0102 ("one process per data directory") and ADR-0104 (graceful shutdown).
- Implementation: `crates/niche-app/src/lease.rs`, `crates/niche-app/src/engine.rs`
  (`set_leader`, job files), `crates/niche-server/src/lib.rs` (`lead`), deployment configuration
  `apps/base/niche/deployment.yaml` (RollingUpdate, preStop).

## Context

The hosted instance ran one pod with `strategy: Recreate`: every deployment stopped the old pod
before starting the new one, so the site answered 503 for the length of a restart (startup,
boundary packs, probes). SQLite and the RWO volume ruled out two pods because both would run
studies, preparation and maintenance on the same data.

## Decision

1. **Two processes may share the data directory for a while** (same node, same volume): SQLite
   in WAL mode with `busy_timeout` serves both; every API request works in either.
2. **Leader lease**: an exclusive lock on `<data_dir>/leader.lock` (`flock`, released by the
   kernel when the holder exits or dies). Only the holder runs the background work — the study
   worker (and with it the recovery of interrupted studies, retention, database copies), the data
   preparation and the regional GBIF mirror. A new process serves requests immediately and polls
   the lease every 500 ms; it takes over within about a second of the old process exiting, then
   requeues the study the old one left interrupted, which resumes from its completed species.
   `/health` reports `leader`.
3. **Job state shared through files**, not memory, where a request may reach the other process:
   cancelling a running study (`jobs/<id>/.cancel`, watched by the running worker), continuing
   without the GBIF download (`jobs/<id>/.skip-download`) and the GBIF wait shown to users
   (`jobs/<id>/.gbif-wait.json`). They are removed when the job or its preparation ends.
4. **Kubernetes**: `RollingUpdate` with `maxSurge: 1`, `maxUnavailable: 0` — the new pod must be
   ready before the old one is stopped; a 5 s `preStop` sleep lets the ingress drop the old pod
   before it stops accepting connections; in-flight requests finish (graceful shutdown). The RWO
   volume allows both pods on the same node (the cluster has one node).
5. **Rule for future schema changes**: during a deployment the old binary keeps running on the
   database the new one has migrated. Migrations must stay additive (new tables, new nullable or
   defaulted columns); a removal or rename needs two releases (stop using, then drop).

## Consequences

- No 503 during deployments; a running study pauses for a few seconds at the handover and
  resumes in the new process (measured locally: lease taken 0.7–1.0 s after the old process
  stopped; a 4-species study interrupted at 19 % finished in the new process).
- In-memory caches (rasters, the mirror's progress text) are per process; during the overlap the
  non-leader may show less detail in the administration's mirror card.
- Two pods on two different nodes would still be refused by the RWO volume: scaling out needs
  another storage design.
