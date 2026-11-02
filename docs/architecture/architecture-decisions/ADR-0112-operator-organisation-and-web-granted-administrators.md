# ADR-0112 — Operator organisation and administrators named from the console

- Status: Accepted (2026-10-19). Product decisions (session of 2026-10-19):
  administrators can give the role from the administration console; only members of an
  **operator organisation** can hold it, "Natural Solutions" by default; removing the role stays on
  the command line; administrators found outside the operator organisation keep the role and are
  reported; showing the operator's name instead of "Natural Solutions" in the interface is later work.
- Amends ADR-0106 §1. Implementation: `crates/niche-app/src/store.rs` (schema v9),
  `crates/niche-app/src/accounts.rs` (`operator_org`, `org_set_operator`, `user_set_admin`,
  `check_password_of`, `admins_outside_operator`), `crates/niche-server/src/lib.rs`
  (`POST /admin/users/admin`, `admin_update_org`), `crates/niche-cli/src/app_cmd.rs`
  (`niche admin org operator`, `org create --operator`), `web/src/features/admin/AdminPage.tsx`.

## Context

ADR-0106 made the administrator flag CLI-only so that a stolen session cannot create
administrators. Naming a colleague then needs shell access to the cluster. Administrators see every
organisation's studies (ADR-0106 §3), so the role must stay with the staff that operates the
service, and a stolen cookie must still not be enough to create one.

## Decision

1. **Operator organisation**: `orgs.is_operator` (schema v9), at most one (partial unique index).
   Only its members can be granted the administrator flag, from the web or the CLI.
2. **Designated on the server only**: `niche admin org operator <name>` or
   `niche admin org create <name> --operator`. Choosing it decides who can become an
   administrator, so no HTTP route changes it. **Default "Natural Solutions"**: `org create`
   designates an organisation of that name when none is designated, and the v8 → v9 migration
   designates the organisation named Natural Solutions, otherwise the one every current
   administrator belongs to, otherwise none (granting then answers `409` until it is designated).
3. **Granting from the console**: `POST /api/v1/admin/users/admin {email, password}`. The
   administrator re-enters their own password; a wrong password is `403` and counts as a failed
   sign-in of that account (same throttle, `429`). Logged at WARN with ids only (ADR-0106 §7).
4. **Removal stays CLI only** (`niche admin user demote`), as does any recovery.
5. **Administrators outside the operator organisation keep the flag** (after a migration or a new
   designation). They are logged at WARN at every start-up and marked in the console and in
   `niche admin user list`, to be resolved by hand.
6. **The operator organisation cannot be disabled from the web** (`409`), whoever asks.

## Consequences

- A stolen session alone cannot create administrators; a stolen session plus the password can,
  but only inside the operator organisation, and the grant is logged.
- Production (2026-10-19): the organisation "Natural Solutions" exists and holds the
  administrator, so the migration designates it without intervention.
- Interface texts still say "Natural Solutions administrators"; taking the operator's name from
  the server is future work.
- Tests: `only_members_of_the_operator_organisation_become_administrators`, `administration_queries`
  and the v8 → v9 steps of `v1_database_is_migrated_and_newer_is_refused`
  (`crates/niche-app/tests/accounts.rs`);
  `administrators_grant_the_role_to_the_operator_organisation_with_their_password`
  (`crates/niche-server/tests/api_accounts.rs`).
