# ADR-0105 — Hosted mode: accounts, organisations, invitation links and quotas

- Status: Accepted (2026-10-16). Product decisions in the design notes;
  technical plan in the design notes.
- Supersedes for hosted use: the `auth.mode` options sketched in `configuration.md` and the
  "server mode unsupported until auth + isolation exist" restriction of
  `docs/research/12-security-and-licensing.md` §3 (P2), for the scope listed below.
- Implementation: `crates/niche-app/src/{accounts.rs,store.rs,engine.rs}`,
  `crates/niche-server/src/lib.rs`, `crates/niche-cli/src/app_cmd.rs` (`niche admin`),
  `web/src/features/auth/`.

## Context

NicheRS v0 is offered by Natural Solutions as a hosted service to *bureaux d'études*: several client
organisations share one instance, each with an annual subscription and a monthly quota of
modelled species, invoiced outside the application. Until now the server had no notion of user
or owner: every job was visible to anyone reaching the API.

## Decision

1. **Two modes, one binary.** `NICHE_AUTH=none` (default) keeps the single-user local mode
   unchanged. `NICHE_AUTH=accounts` enables hosted mode. An enum, not a trait (ADR-0201).
2. **Organisation = tenant.** Users belong to exactly one organisation; no roles in v0. Jobs carry
   `org_id`; every job-level route checks it first and answers **404** for another organisation's
   job (no existence oracle). Caches of open data (CHELSA, land mask, GBIF archives) stay shared.
3. **Invitation is the only way in; NicheRS sends no e-mail.** `niche admin invite --org X` prints
   a one-time link `https://<host>/invite#<token>`. The token (256 random bits) is in the URL
   fragment, so browsers never send it to the server, logs or `Referer`; the page posts it in a
   body. Links expire after 7 days, are single-use and revocable; only their SHA-256 is stored.
   Password resets use the same mechanism (`niche admin user reset`), revoke earlier reset links
   and close every session of the account. Only Natural Solutions invites.
4. **Credentials.** E-mail (identifier only, case-insensitive) + password ≥ 12 characters, hashed
   with Argon2id (`argon2` 0.6, RustCrypto, MIT/Apache-2.0). Login errors are generic; a missing
   account costs the same hash verification as a wrong password. After 10 failures for one e-mail
   within 15 min, sign-in for that e-mail is refused (HTTP 429) until the window passes
   (in-memory, per process, at most 10 000 tracked addresses; strings that cannot be an account
   are not tracked). Unauthenticated work is bounded: at most 2 Argon2 computations run at once
   (each ~19 MiB), invitation and reset tokens are checked before hashing, and `/auth/*` bodies
   are limited to 16 KiB. Known residual risk: someone who knows a user's e-mail can keep that
   account blocked by failing on purpose; Natural Solutions can then issue a reset link. Keying
   on (IP, e-mail) needs trusted proxy addresses and is left for later.
5. **Sessions.** Random token in cookie `__Host-niche_session` (`HttpOnly; Secure; SameSite=Strict;
   Path=/`, 30 days); only its hash is stored. Disabling an account or organisation deletes its
   sessions (effective on the next request). Existing CSRF protections stay (Origin check, JSON
   bodies, `SameSite=Strict`).
6. **Quota unit: the modelled species.** A species counts when a model is fitted for it
   (`Succeeded`), in the UTC calendar month in which it finishes; `Skipped` (*Insufficient*),
   failed and cancelled species do not. Counting is idempotent per (job, species) so a resumed job
   is never counted twice. At submission, `used + reserved + requested ≤ quota` is checked under the
   same database lock as the insert (`reserved` = pending/running species of the organisation's
   unfinished jobs); otherwise HTTP 409 "Monthly quota reached". The usage ledger is independent of
   jobs: deleting a job never refunds.
7. **Limits in hosted mode:** ≤ 25 species per job; study-area extent ≤ 2.5 M modelling-grid cells
   (metropolitan France at 1 km is 1721 × 1167 ≈ 2.0 M, the largest measured case), checked before
   any download. Local mode keeps the previous limits (500 species).
8. **Retention and deletion.** `DELETE /api/v1/jobs/{id}` removes a finished job's files and rows
   (409 if not finished). Finished jobs older than `NICHE_RETENTION_DAYS` (default 365 in hosted
   mode, 0 = never in local mode) are deleted by the worker, at most once per hour.
9. **Fair queue and GBIF wait off the compute worker.** Jobs get a data-preparation state
   (`pending → preparing → ready | nobatch`). A separate I/O thread (≤ 3 in flight, GBIF limits
   concurrent downloads per account) requests and waits for the GBIF batch download; the compute
   worker only takes prepared jobs, so one client's cold download (observed 23 min,
   `docs/benchmarks/gbif-batch-downloads.md`) no longer blocks everyone. Both threads derive the
   request from the spec alone (same taxa, same grid-window extent), so the worker finds the archive
   in the download cache. Among prepared jobs, the organisation whose last job started longest ago
   goes first (round-robin), then the oldest job.
10. **Administration** is the `niche admin` CLI run on the server (organisations, quota, invitations,
    accounts, adoption of pre-existing jobs, usage CSV for invoicing). It writes the same SQLite
    database while the server runs (WAL, `busy_timeout`); this is the one exception to "single process
    per data dir" (ADR-0102, whose lock is not implemented yet): admin commands are short writers.
11. **Schema v2** is the first migration (v1 → v2 in one transaction); a newer schema is refused.
    Pre-existing jobs have no organisation; `niche admin org adopt <org>` assigns them.

## Not in v0 (explicitly)

Roles and permissions, invitations by members, sharing results with third parties, online payment,
storage quota in GB, per-client GBIF accounts (one NS account is shared; added by ADR-0110), on-premise packaging,
GeoNature connector, public API keys, SLA, sign-in rate limiting across replicas.

## Consequences

- The hosted instance must run behind TLS (`Secure` cookie) with `NICHE_ALLOWED_HOSTS` set.
  Browsers accept `Secure` cookies on `http://localhost` for development.
- The shared GBIF account means all organisations' downloads appear under Natural Solutions' GBIF
  user; DOIs and citations are unchanged.
- Every `/api/` response is `Cache-Control: private, no-store` in hosted mode (tiles included),
  so results do not outlive the session in a shared browser's HTTP cache; local mode keeps a
  one-day private cache for tiles.
- The progress stream (SSE) re-checks the session on every poll and ends when it is closed or the
  account or organisation is disabled.
- 5xx responses carry a reference to the server log line, not the internal message (paths, SQL).
- GBIF batch archives (which contain every exported column, observer names included) are deleted
  once split per taxon; only the fields NicheRS uses are cached, and entries older than the
  30-day cache age are pruned (`niche cache clean` also clears them).
- `/api/v1/health` no longer reveals the data directory in hosted mode.
- Tests: `crates/niche-app/tests/accounts.rs` (migration, links, sessions, quota, isolation,
  deletion, retention, queue) and `crates/niche-server/tests/api_accounts.rs` (HTTP, including a
  check that every declared job route is covered by the cross-organisation test);
  `scripts/ui_smoke_hosted.py` for the browser flow.
