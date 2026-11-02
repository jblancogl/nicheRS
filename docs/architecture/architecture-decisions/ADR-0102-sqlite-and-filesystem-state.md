# ADR-0102: Durable state in SQLite (metadata) + local filesystem (artifacts)

- Status: Proposed (2026-10-15)
- Related: [job-lifecycle.md](../job-lifecycle.md), tension 1 and 6 in [twelve-factor-compliance.md](../twelve-factor-compliance.md);
  storage/driver details may be refined by `docs/research/11-backend-architecture.md`.

## Context

Jobs and artifacts must survive restarts and crashes; the product is local-first with zero install;
12factor wants stateless processes with state in backing services [3]. Outputs are large rasters and
model bundles; metadata is small and relational (jobs, stages, artifacts, models, datasets).

## Decision

1. **Metadata** in one SQLite file `data_dir/niche.db`: `journal_mode=WAL`, `synchronous=FULL`,
   `foreign_keys=ON`, `busy_timeout=5000`; one writer connection (serialised writes) + small read pool.
   `FULL` because WAL with `NORMAL` "might roll back following a power loss or system crash" [25] and
   job acknowledgement must be durable. Writes are small and infrequent, so the fsync cost is acceptable
   [ASSUMPTION — benchmark]. **Divergence:** 11-backend-architecture.md §2.3 proposes `NORMAL` (artifacts
   are content-addressed and re-runnable). Counter-argument: with `NORMAL`, a power loss can roll back the
   artifact-row commit of a completed step while its file survives; recovery then treats the file as an
   orphan and the result must be recomputed, and an acknowledged `202/201` job submission can vanish —
   contradicting "never silently lose completed results". Resolve with a benchmark of commit latency
   (expected low-QPS) [OPEN].
2. **Artifacts** as files in the content-addressed store `data_dir/cas/` (layout in 11-backend-architecture.md §2.4), written with the atomic protocol (temp in same
   dir → fsync → rename → fsync dir → DB row with content hash) [17][22][23]. DB never stores large blobs.
3. **Cache** (regenerable downloads) under `cache_dir`, content-addressed, deletable anytime.
4. **Single process per data dir** enforced by `std::fs::File::try_lock` on `niche.lock` (stable since
   Rust 1.89 [20]; no `fs4` dependency needed [19]). Sets MSRV ≥ 1.89.
5. Data dir must be on a local filesystem (WAL "does not work over a network filesystem" [24]); checked
   at startup and by `niche doctor`.
6. Access goes through `MetadataStore` / `ArtifactStore` traits so Postgres / object storage can be
   added later without engine changes.
7. Schema migrations: versioned, forward-only, backup copy before migrating; driver choice
   (`rusqlite` 0.40.2 [33] vs `sqlx`) deferred to the backend doc.

## Alternatives

| Alternative | Why not (now) |
|---|---|
| Postgres + S3/MinIO | Requires services → breaks local-first/zero-install. Kept as v2 behind traits. |
| Files only (JSON manifests, no DB) | No transactions/queries across jobs; harder crash consistency for multi-record state. |
| Embedded KV (sled/redb) | Less tooling, no SQL inspection by users/support; SQLite is ubiquitous. |
| Blobs inside SQLite | DB bloat, WAL growth, worse streaming of rasters to MapLibre/tiles. |

## Evidence

SQLite WAL & synchronous docs [24][25]; rename(2) atomic replace [22]; tempfile persist does not fsync
[17]; LWN safe-replace sequence [23]; std file locking [20].

## Trade-offs

+ Zero-install, transactional, inspectable; crash-consistent with a simple ordering rule.
− Single host, single process per data dir; no HA; network shares unsupported.

## Risks

Users placing data on SMB/NFS shares; Windows rename/dir-fsync durability weaker [ASSUMPTION]; WAL growth
under long readers [24] → periodic `wal_checkpoint`.

## Reversibility

Medium: traits make backend swaps possible; data migration tooling would be needed.

## Rationale

Satisfies "jobs & artifacts survive restarts" and "no cloud needed locally" with the smallest set of
moving parts; the process stays disposable even though the instance is stateful.

## References

Ids as in [twelve-factor-compliance.md § References](../twelve-factor-compliance.md#references).
