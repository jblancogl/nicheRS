# Information architecture

Date: 2026-10-15. Basis: [09-ux-research §2.6](../research/09-ux-research.md#26-map-first-vs-workflow-first).

## 1. Object model (user-facing)

```
Workspace (organisation)
└── Project            e.g. "Parc éolien Causses 2026"
    ├── Study          species × study area × config (editable draft)
    │   └── Run        immutable execution of a Study config (#1, #2, …)
    │       ├── Layers         suitability, uncertainty, extrapolation, binary, folds
    │       ├── Metrics        validation, area estimates, variable importance
    │       ├── Warnings       pre-flight + runtime
    │       ├── Provenance     config JSON, data versions, software versions, seeds, hashes
    │       └── Exports        bundles generated from this run
    ├── Batch          N species × 1 study area × 1 config template → N Studies + Runs
    └── Comparison     saved selection of 2–4 Runs
Data catalogue (global)
├── Environmental layers   name, unit, source, resolution, CRS, temporal coverage, licence
├── Occurrence sources     GBIF, national sources [ASSUMPTION], user uploads
└── Territories            admin units, saved study areas
```

Rules: a Study's config is a draft until launched; launching snapshots it into a Run. Editing a
launched config means "Duplicate & edit" → new Run on the same Study. Batch children are normal
Studies (tagged with the batch id) so every single-run view works on them.

## 2. Navigation

| Level | Element | Content |
|---|---|---|
| Global (left rail, 56 px, icons + labels on hover/expand) | Projects · Batches · Data · Settings (+ Help at bottom, consistent position, WCAG 3.2.6) | ≤ 5 items |
| Context bar (top, 44 px) | Breadcrumb: Project › Study (species, italic) › Run #n · territory chip · status badge · run switcher | Always visible inside a project |
| Workspace | Map canvas (centre) + contextual side panel (right, 400–480 px, resizable) | Panel mode = Setup / Execution / Results / Compare |
| Secondary | Panel tabs inside mode (e.g. Results: Summary · Validation · Variables · Provenance) | ≤ 4 tabs |
| Utility | Command palette (Ctrl/Cmd+K): jump to species, run, layer, action | Recognition aid for experts |

## 3. Routes (URL = state)

| Route | View | Panel mode |
|---|---|---|
| `/projects` | Project list | — |
| `/projects/:p` | Project home: studies table + map of all study areas | — |
| `/projects/:p/studies/new` | Setup step 1 | Setup |
| `/projects/:p/studies/:s/setup/:step` (`species`, `area`, `environment`, `model`, `validation`, `review`) | Setup step | Setup |
| `/projects/:p/studies/:s/runs/:r` | Execution if running, Results if finished | Execution / Results |
| `…/runs/:r?tab=validation&layer=uncertainty&t=0.31&rule=p10&obs=1&z=8/44.1/3.2` | Results with full view state | Results |
| `/projects/:p/batches/new` | Batch creation (table-first) | — |
| `/projects/:p/batches/:b` | Batch grid; `?status=failed&row=:s` opens row in side panel | Batch |
| `/projects/:p/compare?runs=r1,r2` | Comparison (synced maps) | Compare |
| `/data/layers`, `/data/sources`, `/data/territories` | Catalogue | — |
| `/account[/security\|/gbif]`, `/org[/members\|/gbif\|/feedback\|/history]` | Settings (hosted mode): one area with a side navigation grouped by "My account" and "My organisation" (a select on narrow screens); sections listed in `web/src/features/settings/sections.tsx` | — |
| `/admin/<section>[/<id>]` (`overview`, `orgs`, `users`, `activity`, `usage`, `feedback`, `system`, `audit`) | Administration (administrators only, ADR-0113): `/admin` opens the overview (five figures, what needs attention — quotas from 80 %, failed studies, open feedback, system checks — and the latest studies, each linking to its section); side navigation grouped by Clients, Operations and Platform; lists become stacked cards below 768 px; a page per organisation (`/admin/orgs/<id>`) and per account (`/admin/users/<id>`) with its history; sections listed in `web/src/features/admin/sections.tsx`; `/admin?tab=<x>` redirects | — |

Settings sections are a registry (id, path, icon, who may open it, component; title and purpose
in `settings/messages.ts`): a new section adds one entry and appears in the navigation and, for
the organisation, as a card of its overview. A section a user may not open is absent from the
navigation and its URL returns to the area's first page.

[REC] View state (layer, threshold, rule, map camera, open tab) lives in URL search params so
links are shareable and reloads restore context (Context law). Server state via TanStack Query;
Zustand only for transient UI state not worth a URL (panel width, hovered feature, draw-tool mode).

## 4. Panel content per mode

| Mode | Header | Body (progressive disclosure) | Footer (one primary) |
|---|---|---|---|
| Workspace (implemented, `/`, ADR-0107) | Organisation (or “My studies”), counts and quota; **New study** | Projects (left) · study being prepared (Continue / Discard) · in progress (live) · studies table (outcome, author, date) · search · welcome block while there is no study | — (no map; studies and `/new` open the map layout) |
| Setup | Stepper (6 steps, current + completion state) | Guided fields → "Details" collapsible → "Expert" tab | Back · **Continue** / **Launch model run** |
| Execution | Run summary chip row | Stage list, progress, warnings, Details (resources, log) | **Cancel run** (destructive outline) |
| Results | Run status + warnings banner | Tabs: Summary (3 cards, threshold, layers) · Validation · Variables · Provenance | Duplicate & edit · **Export** |
| Batch row | Species + status | Mini results or failure explanation | Open full results / Retry |
| Compare | Run chips | Config diff table, metric table | **Export comparison** |

## 5. Map layer stack (bottom → top)

1. Basemap (light/dark neutral vector; labels subdued)
2. Hillshade (optional, off by default)
3. Prediction raster (suitability | uncertainty | binary) — **one at a time** in the main map
4. Extrapolation hatch overlay
5. Study area mask (outside dimmed) + outline
6. Validation folds (setup/validation tab only)
7. Observations (training ●, test ▲, excluded ×)
8. Basemap labels (re-drawn above rasters for legibility)
9. Interaction layers (draw handles, inspect marker)

Layer control lists layers in this order, grouped "Modelled" (3–4) and "Observed" (7).

## 6. Labelling and content rules

- Scientific names italic; vernacular (fr) in roman after.
- Every number has a unit or "score"; see [design-tokens §8](design-tokens.md#8-numeric-formatting-rules).
- Every legend has title, quantity, domain, ramp, and a "What does this mean?" disclosure.
- Status vocabulary: Draft · Queued · Running · Completed · Completed with warnings · Failed ·
  Cancelled (mapping to the API job enum in
  [interaction-specifications §5](interaction-specifications.md#5-execution-and-long-running-feedback)).
