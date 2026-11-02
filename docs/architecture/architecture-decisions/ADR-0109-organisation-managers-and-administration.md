# ADR-0109 — Organisation managers; administration console for operations and pricing

- Status: Accepted (2026-10-19). Product decision (session of 2026-10-19):
  translate the console, show the system state, ready-to-send invitations, an organisation
  manager role, usage metrics for pricing, a more useful activity view — "implement them all".
- Amends ADR-0107 §5 ("No roles") and extends ADR-0105 (accounts) and ADR-0106 (administration).
  (ADR-0108 is reserved for organisation-owned occurrence data, the design notes.)
- Implementation: `crates/niche-app/src/accounts.rs` (`user_set_manager`, `check_member`,
  `revoke_link_in`, `metrics`), `store.rs` (schema v6), `system.rs`, `engine.rs`
  (`system_status`, `backup_db_if_due`), `crates/niche-server/src/lib.rs` (`/org/*`,
  `/admin/system`, `/admin/metrics`, `/admin/users/manager`), `web/src/features/admin/`.

## Context

Only Natural Solutions could invite people or reset passwords, so every client request went
through one person. Administrators needed `kubectl` to know the state of the instance, had no
data to set quotas and prices (the freemium question), and the console was English-only.

## Decision

1. **Organisation manager** (`users.is_manager`, schema v6). Granted by an NS administrator from
   the console (Users → Make manager) or `niche admin user manager <email> [--off]`. Within
   their own organisation only, a manager: lists members, creates and revokes invitations,
   creates password-reset links, disables / re-enables accounts. Not: quotas, other
   organisations, erasure (GDPR, stays with NS), roles. A manager can never act on their own
   account nor on an NS administrator; accounts of other organisations answer 404, like jobs.
   Studies still belong to the organisation; every member keeps the same rights on them
   (ADR-0107). Every action is logged with the manager's id (no e-mail in logs).
2. **Ready-to-send links.** NicheRS still sends no e-mail. Each invitation or reset link is
   shown once with a message in the recipient's language (EN/FR/ES, chosen independently of the
   sender's interface), signed with the sender's name and organisation.
3. **System state** (`GET /admin/system`, Système tab): version and build, running since,
   volume size and free space (statvfs, Unix), sizes of database, study outputs, cache, packs
   and copies; studies by state and failures of the last 7 days; installed packs; GBIF account
   check (cached 10 min, credentials never shown); retention.
4. **Database copies.** Once a day the worker copies the SQLite database with `VACUUM INTO` to
   `<data_dir>/backups/niche-YYYYMMDD-HHMMSS.db`, keeping 7. They complement, not replace, the
   platform's volume backup (Longhorn), which the application cannot see: the deployment
   declares it in `NICHE_VOLUME_BACKUP` and the console shows that text.
5. **Metrics** (`GET /admin/metrics?month=`): per organisation, studies, species requested,
   succeeded / failed / cancelled, total computing time and time per species of succeeded
   studies, GBIF waiting time (mean, max; data preparation ≥ 1 s, from `jobs.prep_*_at`, schema
   v6), failures by cause. Quota bars turn amber at 80 % (used + reserved) and red when full.
6. **Activity**: failure cause per study (`gbif`, `climate`, `area`, `records`, `model`,
   `server`, `other`, classified from the error message) with a filter; queued studies show
   their GBIF wait (minutes, last check, link to the download on gbif.org).
7. **Console in EN/FR/ES**, like the rest of the interface.

## Consequences

- Clients can grow their own team without NS; NS keeps quotas, organisations and erasure.
- The failure classification is heuristic (message patterns): good for grouping, not for
  statistics in a report; "other" collects what it does not recognise.
- Metrics count studies created in the month; usage for invoicing stays the ledger of modelled
  species (ADR-0105), unchanged.
