# ADR-0110 — Account and organisation pages; GBIF accounts of users and organisations

- Status: Accepted (2026-10-19). Product decisions (session of 2026-10-19):
  each user may use their own GBIF account; the organisation's account is the default for its
  members; the shared Natural Solutions account is the last resort; a cached download is reused
  only by the same account; "My account" covers name, password and language.
- Amends ADR-0105 ("Not in v0: per-client GBIF accounts") and extends ADR-0109 (managers).
- Implementation: `crates/niche-app/src/secrets.rs`, `gbif_accounts.rs`, `engine.rs`
  (`gbif_settings`, `gbif_store`, `gbif_check`, `job_account`, `prepare_loop`), `store.rs`
  (schema v7), `accounts.rs` (`user_set_name`, `change_password`), `crates/niche-server/src/lib.rs`
  (`/account*`, `/org`, `/org/gbif*`), `web/src/features/account/`, `web/src/features/org/`, `web/src/features/settings/`.

## Context

Every study downloaded its GBIF records with one shared account (Natural Solutions). GBIF limits
concurrent downloads per account, so the instance prepared at most three downloads at a time for
all clients; downloads appeared under NS's GBIF user (`donnees-sensibles-rgpd.md` §8); and users
had no page for their own properties (password changes went through reset links).

## Decision

1. **Order of accounts** for a study's batch download: the account of the user who created it,
   then their organisation's, then the shared account (`NICHE_GBIF_USERNAME`), then none (search
   API, no DOI). An account GBIF refused at its last check is skipped; "GBIF unreachable" keeps it.
2. **Fixed at preparation.** The chosen account is stored in `jobs.gbif_account` (`user:<id>`,
   `org:<id>`, `shared`, `none`) when the data preparation starts; the compute worker uses the same
   one. If it is later removed or refused, the next in line is chosen.
3. **Concurrency per account**: at most 3 downloads in preparation per account, 12 in all.
4. **Cache per account.** Downloads of a user's or organisation's account are cached under
   `<cache>/gbif-accounts/<hash of the GBIF user name>/`; the shared account keeps the original
   cache. A study never cites a download made with another organisation's account.
5. **Secrets.** Passwords are sealed with AES-256-GCM (`ring`, already in the dependency tree via
   rustls) under `NICHE_SECRET_KEY` (32 bytes, hex), bound to their row (associated data
   `user:<id>` / `org:<id>`). Without the key nothing can be stored and stored accounts are not
   used. Passwords are never returned by the API nor logged. Saving and checking are two calls
   (`PUT`, then `POST …/check`), so tests run offline.
6. **Outputs.** The manifest records `occurrence_source.gbif_account` = `user`, `organisation` or
   `shared` — never the user name.
7. **Pages.** "My account" (`/account`, every account): name, e-mail and organisation, interface
   language, password change (proving the current one; other sessions closed), own GBIF account
   with the account the next studies will use. "My organisation" (`/org`) is now open to every
   member: name, active members, quota of the month and the organisation's GBIF account (whether it
   exists; its user name for managers). Managers set it and keep managing members (ADR-0109).
8. **One settings area** (2026-10-19, to let both pages grow): `/account` (profile and language,
   `/security` password, `/gbif`) and `/org` (overview, `/members` and `/feedback` for managers,
   `/gbif`) share a side navigation and one URL per section, from a registry
   (`web/src/features/settings/sections.tsx`); the organisation overview links to every section the
   member may open. See `docs/design/information-architecture.md` §3.

## Consequences

- Clients with their own GBIF account no longer wait behind others' downloads, and their downloads
  are made under their own GBIF user; the shared account remains for discovery and the free tier.
- One more secret to deploy and keep (`NICHE_SECRET_KEY`); rotating it requires users to enter
  their accounts again (no re-encryption tool yet).
- The first study with a newly set account re-downloads data the shared cache may already hold.
- GBIF's real per-account limit is still unverified (`06-biodiversity-data.md`); 3 is the
  documented historical value.
