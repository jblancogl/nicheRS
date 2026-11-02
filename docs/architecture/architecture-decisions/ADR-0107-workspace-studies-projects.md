# ADR-0107 — Organisation workspace, studies and projects

- Status: Accepted (2026-10-16). Product decisions (session of 2026-10-16):
  enter through the organisation's studies, then open the map; a job is called a **study** in
  the interface; group studies by **project** now.
- Extends ADR-0105 (hosted mode). Implementation: `crates/niche-app/src/projects.rs`,
  `crates/niche-app/src/store.rs` (schema v5), `crates/niche-server/src/lib.rs`
  (`/api/v1/projects`, `PATCH /api/v1/jobs/{id}`), `web/src/features/workspace/`,
  `web/src/app/router.tsx`, `web/src/app/MapLayout.tsx`.

## Context

NicheRS opened on a setup wizard over the map, as if every visit created something new. A
consultancy mostly comes back to follow what is running and to find earlier work, organised by
client or file. "Run" / "travail" / "ejecución" read as technical jargon.

## Decision

1. **Workspace first.** `/` is the organisation's workspace (local mode: "My studies"), a plain
   page without map: projects on the left (all studies, each project with its count, studies
   without project), studies of the selection on the right — in progress first (live progress),
   then a table (outcome by evidence level, author, date, duration), search, the study being
   prepared ("Continue" / "Discard"), and a welcome block while the organisation has no study.
2. **The map lives inside a study.** `/new` (setup wizard), `/jobs/:id` (progress) and
   `/jobs/:id/results` use the map layout, loaded lazily (MapLibre is not downloaded for the
   workspace). Each shows "Organisation › Project" above its title. `/jobs` redirects to `/`.
3. **Vocabulary.** A job is a *study* / *étude* / *estudio* in every interface string and in the
   help. The API, the database and the manifest keep `job`.
4. **Projects.** A project is a named group of an organisation's studies (unique name per
   organisation, case-insensitive; local mode: projects without organisation). Studies may have
   no project ("No project" bucket, the state of every study created before v5) — there is no
   implicit default project. Deleting a project keeps its studies, without project. A study is
   created in a project (`POST /jobs?project_id=`), moved or renamed (`PATCH /jobs/{id}`).
   Projects are **organisational only**: not in the job specification, its hash or the execution
   manifest, so they never affect reproducibility.
5. **Isolation and rights.** Another organisation's project or study answers 404. Only the
   owning organisation modifies projects and studies; administrators' support access
   (ADR-0106) stays read-only for these operations. No roles on studies and projects (ADR-0109 adds
   organisation managers for members and invitations only): every member can create, rename
   and delete projects.

## Consequences

- Schema v5 (`projects`, `jobs.project_id ON DELETE SET NULL`); migrations tested from v1–v4.
- The top bar loses its runs menu (the workspace is the list); finish notices remain.
- Not in this decision: project-level sharing with third parties, archiving, project
  descriptions or metadata (client, dates), moving several studies at once.
