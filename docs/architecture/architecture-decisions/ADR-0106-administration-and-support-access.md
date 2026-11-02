# ADR-0106 — Natural Solutions administration and support access to client runs

- Status: Accepted (2026-10-16). Product decisions (session of 2026-10-16):
  per-user administrator flag; administrators may see everything; four screens.
- Extends ADR-0105 (hosted mode). Implementation: `crates/niche-app/src/accounts.rs`
  (administration queries), `crates/niche-server/src/lib.rs` (`/api/v1/admin/*`),
  `crates/niche-cli/src/app_cmd.rs` (`niche admin user promote|demote`),
  `web/src/features/admin/AdminPage.tsx`.

## Context

The v0 administration was command-line only (`niche admin`). Running the service for several
clients needs a comfortable web console for Natural Solutions staff: organisations and quotas,
invitations and accounts, usage for invoicing, and what the server is doing.

## Decision

1. **Administrator = a flag on one account** (`users.is_admin`, schema v3). It is set and removed
   only on the server (`niche admin user promote|demote <email>`); no HTTP route can grant it, so a
   stolen session cannot create administrators. Removal takes effect on the next request.
   *Amended by ADR-0112 (2026-10-19): administrators may grant the flag from the console to
   members of the operator organisation, re-entering their password; removal stays CLI only.*
2. **Administration API** `/api/v1/admin/*`, behind the session layer: 403 for members, 404 in local
   mode. Organisations (overview with this month's quota, users, runs, pending invitations; create;
   change quota; disable/enable), invitations (create — the link is returned once, `no-store`;
   list pending by hash; revoke one), accounts (list with role and last sign-in; disable/enable;
   reset link), usage (per organisation and month; CSV with one line per modelled species) and
   activity (all unfinished runs + the 100 most recent finished, every organisation, metadata only).
3. **Support access: administrators can open every organisation's runs** (results, maps, files,
   manifest, events) and cancel them. The workspace still *lists* only their own organisation's
   runs; client runs are reached from the Activity screen. **Deleting stays with the owning
   organisation.** Each (administrator, run of another organisation) access is logged at INFO at most
   once per hour per pair (`administrator accessed another organisation's job`).
4. **No self lock-out:** an administrator cannot disable their own account or organisation from
   the web.
5. **Last sign-in** (`users.last_login_at`) is recorded when a session is opened.
6. **Account erasure** (GDPR right to erasure, added 2026-10-16): `niche admin user delete <email>
   --yes` or `POST /api/v1/admin/users/delete {email, confirm_email}` (the console asks for the
   e-mail to be typed again). It deletes the user row (e-mail, name, password hash, last sign-in),
   sessions and reset links, and clears `jobs.created_by`; the organisation's jobs and usage ledger
   stay. An administrator cannot erase their own account.
7. **Logs carry account ids, never e-mail addresses** (`user_id`, `admin_id`): server logs are kept
   longer than accounts and are not covered by erasure. The id ↔ e-mail mapping lives only in the
   database, so erasing an account also anonymises its past log lines.

## Consequences

- Client data is visible to Natural Solutions administrators. The terms of service must say so
  (support access, logged) before the first client — the design notes §2.
- The access log is a server log line, not a tamper-proof audit trail. *Superseded by ADR-0113
  (2026-10-19): support access and every administration change are recorded in an audit table.*
- Tests: `administration_queries` (`crates/niche-app/tests/accounts.rs`), `administration_api` and
  `administration_is_absent_in_local_mode` (`crates/niche-server/tests/api_accounts.rs`); schema
  migration v1 → v3 and v2 → v3 in `v1_database_is_migrated_and_newer_is_refused`.
