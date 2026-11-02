# ADR-0113 — Audit trail and an administration built from sections

- Status: Accepted (2026-10-19). Product decision (session of 2026-10-19): make the
  administration scale — first a structured audit trail and an administration made of sections
  with a page per organisation and per account; split the server into modules and add server-side
  paging and filtering of the lists when the numbers ask for it.
- Supersedes the access-log consequence of ADR-0106. Implementation:
  `crates/niche-app/src/audit.rs`, `store.rs` (schema v10), `crates/niche-server/src/lib.rs`
  (`audit`, `GET /admin/audit`), `crates/niche-cli/src/app_cmd.rs` (`niche admin audit`),
  `web/src/features/admin/` (`sections.tsx`, `AdminPage.tsx`, `OrgDetail.tsx`, `UserDetail.tsx`,
  `AuditList.tsx`).

## Context

Administrators see client data (ADR-0106 §3) and can now create administrators from the web
(ADR-0112), but what they did was only in server log lines. The console was one page of six tabs
with every action in table rows, without a page for one organisation or account, so each new
feature had to fit into a row.

## Decision

1. **Audit table** `audit(id, at, actor_id, via, action, org_id, user_id, job_id, detail)`, schema
   v10, without foreign keys: rows outlive what they mention. `via` is `web` (an account) or `cli`
   (`niche admin` on the server, no account). `org_id` is the organisation concerned, also for an
   account's change, so an organisation's history includes its members'. `detail` holds small facts
   (`{"from": 20, "to": 50}`), never personal data.
2. **Recorded**: organisation created, quota changed, disabled/enabled, operator designated;
   invitation created/revoked; reset link created; account disabled/enabled/erased; manager and
   administrator roles granted/removed; organisation GBIF account set/removed (managers too); and
   **support access** (an administrator opening another organisation's study, at most once per hour
   per pair, as the log did). Every recording also logs one INFO line `audit` with ids only.
3. **Ids, never e-mail addresses** (ADR-0106 §7). Names and e-mails are joined when the trail is
   read; once an account is erased its rows remain without the person ("erased account").
4. **Written after the change**: a failed audit write is logged at ERROR and does not fail the
   request (the change is already made). Kept indefinitely; the table is small.
5. **Read**: `GET /api/v1/admin/audit?org=&user=&before=&limit=` (administrators), newest first, keyset
   pages (`next` → `before`), 50 by default and at most 200; `user` matches changes to and by the
   account. `niche admin audit [--org] [--user] [--limit]` on the server.
6. **Administration made of sections** (`web/src/features/admin/sections.tsx`), like the settings
   area (ADR-0110): groups *Clients* (Organisations, Accounts), *Operations* (Activity, Usage &
   invoicing, Feedback) and *Platform* (System, History); one URL per section (`/admin/<section>`),
   and per organisation and account (`/admin/orgs/<id>`, `/admin/users/<id>`). A new section is one
   registry entry and its texts. The former `/admin?tab=<x>` links redirect.
7. **Lists navigate, pages act**: the organisation page holds its quota, members, invitations,
   recent studies, history and the invite/disable actions; the account page holds its role and
   access actions and its history (what was done to it and by it).
8. **Lists are filtered and paged on the server** (added 2026-10-19): `GET /admin/users?org=&q=&
   offset=&limit=` → `UserPage {users, total, next_offset}` (100 per page, at most 500; `q` matches
   name, e-mail or organisation, `%` and `_` literally); `GET /admin/users/{id}`;
   `GET /admin/orgs/{id or name}`; `GET /admin/activity?org=` (that organisation's unfinished and
   100 most recent finished studies). The organisation overview is computed in a fixed number of
   queries whatever the number of organisations (it was four per organisation).
9. **Managers read their organisation's history** (added 2026-10-19): `GET /org/audit?before=&limit=`
   and My organisation → History, including support access by Natural Solutions, without links to
   the administration.
10. **Roles** (added 2026-10-19): a request's account carries `Role { Member, Manager, Admin }`
    (`is_admin()`, `manages_org()`); the database and the API keep the two flags.
11. **Server modules** (added 2026-10-19): `crates/niche-server/src/` `auth`, `admin`, `account`,
    `org`, `user_feedback`, `studies`; `lib.rs` keeps the options, state, errors, router, guards,
    the UI and `serve`.

## Consequences

- Answering "who changed this, and when" no longer needs server logs; the trail is not
  tamper-proof against someone with database access.
- The organisation list is still returned whole (one aggregated query): paging it is worth it only
  with hundreds of organisations. The activity view of every organisation shows the unfinished
  studies and the 100 most recent finished ones, as before.
- `Me.is_manager` is false for an administrator; administrators manage their organisation anyway.
- Tests: `audit_trail_pages_filters_and_outlives_erased_accounts` and the v9 → v10 step of
  `v1_database_is_migrated_and_newer_is_refused` (`crates/niche-app/tests/accounts.rs`);
  `administration_changes_and_support_access_are_in_the_audit_trail`,
  `managers_read_their_organisation_history_only` and the paging in `administration_api`
  (`crates/niche-server/tests/api_accounts.rs`); `accounts_are_searched_and_paged_on_the_server`
  and the activity and overview checks in `administration_queries`.
