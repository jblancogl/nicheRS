# NicheRS UX specification

Date: 2026-10-15. Status: consolidated draft for review. This document is normative; details live
in the linked design documents.

| Topic | Source of detail |
|---|---|
| Research, personas, map-first vs workflow-first | [09-ux-research](../research/09-ux-research.md) |
| Principles & vocabulary | [design-principles](../design/design-principles.md) |
| Laws of Simplicity review gate | [laws-of-simplicity-audit](../design/laws-of-simplicity-audit.md) |
| Tokens, ramps, number formats | [design-tokens](../design/design-tokens.md) |
| Components | [component-architecture](../design/component-architecture.md) |
| IA, routes, layer stack | [information-architecture](../design/information-architecture.md) |
| Personas & journeys | [user-journeys](../design/user-journeys.md) |
| Wireframes | [wireframes](../design/wireframes.md) |
| Interaction details | [interaction-specifications](../design/interaction-specifications.md) |
| Accessibility | [accessibility](../design/accessibility.md) |

## 1. Product UX goal

Make sophisticated ecological modelling feel simple without hiding scientific complexity or
uncertainty. Primary user: consultant ecologist (not a data scientist); secondary: SDM expert,
reviewer, batch operator.

## 2. Architecture decisions

| # | Decision | Rationale |
|---|---|---|
| UX-1 | Hybrid: persistent MapLibre map + contextual right side panel; minimal left rail; context bar | Spatial steps and results need the map; context preserved (research §2.6) |
| UX-2 | Guided setup: Species → Study area → Environmental data → Model → Validation → Review & launch | Mirrors proven ordered SDM workflow (Wallace) with one primary action per step |
| UX-3 | Guided / Expert are two views of one Zod-validated `StudyConfig` | No divergence; Expert changes visible in Guided summary and Review |
| UX-4 | Runs immutable; "Duplicate & edit"; soft delete | Reproducibility, reversibility |
| UX-5 | Non-blocking execution with persistent status | ArcGIS Pro background geoprocessing model |
| UX-6 | Batch = first-class object with table-first views, sharing config components | Scale to hundreds of species; QGIS batch-table model |
| UX-7 | Suitability = fixed-domain viridis (cividis option); uncertainty = separate lightness ramp; extrapolation = hatch; VSUP deferred | Crameri 2020, Nuñez 2018, Correll 2018, pals caveat |
| UX-8 | Language: "suitability score (relative, 0–1)"; areas always with threshold rule + range; metrics with spread | Yackulic 2013 misinterpretation evidence; no unsupported certainty |
| UX-9 | URL encodes view state; TanStack Query for server state; Zustand only for transient cross-tree UI state | Shareable, reload-safe context |
| UX-10 | shadcn/ui with one pinned primitive base (Base UI default as of Jul 2026) | Consistency; [OPEN] verify Slider/Combobox |
| UX-11 | Desktop-first (≥ 1280 px) for authoring; narrow = read-only review | Instrument density; honest scope |
| UX-12 | WCAG 2.2 AA, with tabular map alternatives | Canvas inaccessible to AT |

## 3. Screens (summary)

S0 Workspace · S1–S6 Setup steps · S7 Execution · S8 Results (Summary / Validation / Variables /
Provenance) · S9 Batch · S10 Expert mode · S11 Comparison · S12 Narrow review · failure state.
Wireframes W0–W11.

## 4. Scientific-honesty rules (binding)

1. Never label model output as probability, chance or percentage. Legend title "Suitability score
   (relative, 0–1)" with disclosure explaining relativity and transform.
2. Never show a suitable-area figure without its threshold rule and a sensitivity range.
3. Never show a validation metric without the scheme, k and spread across folds.
4. Never show a confidence percentage. UncertaintyIndicator levels come from explicit,
   documented rules and always list their reasons and underlying metrics; "Not assessed" is valid.
5. Observations and predictions are always visually and verbally distinct (vector markers vs
   raster; "Observed" vs "Modelled" legend sections).
6. Extrapolation overlay is on by default where it exists.
7. Colour domain is fixed `[0, 1]` for suitability; stretching is Expert-only and flagged.
8. Warnings are shown before results summary; blocking-level warnings require acknowledgement
   before launch and are embedded in exports.
9. Exports carry provenance (config, data versions, software versions, threshold, date) and
   caveats.

## 5. UX acceptance criteria

Each criterion is testable; ids referenced in issues and tests.

### Workspace & navigation
- **AC-NAV-1** Context bar shows project, species (italic), run number and status on every
  project-scoped screen.
- **AC-NAV-2** Reloading any results URL restores layer, threshold, rule, tab and camera exactly.
- **AC-NAV-3** At ≥ 1280 px wide, the map occupies ≥ 55 % of viewport width in setup, execution
  and results.
- **AC-NAV-4** Global navigation has ≤ 5 items; Help is in the same position on every page.

### Setup
- **AC-SET-1** Each step panel has exactly one primary button.
- **AC-SET-2** A new user can reach "Launch" by accepting defaults with ≤ 8 primary actions after
  choosing a species and study area.
- **AC-SET-3** Every default displays its rationale and is editable.
- **AC-SET-4** Species search shows accepted name, authority, status, record count and source
  before commit; ambiguous names block progress until resolved.
- **AC-SET-5** Study area step shows area km², occurrences inside/outside, and excluded
  occurrences on the map with reason.
- **AC-SET-6** Environmental variables display human label, unit, source, resolution and period;
  collinear pairs above the configured ρ are flagged.
- **AC-SET-7** Changing an upstream step marks affected downstream steps "Review needed" without
  discarding still-valid values.
- **AC-SET-8** Review lists every non-default parameter (including Expert-only ones) and all
  pre-flight checks; launch is disabled with visible reason when a blocking check fails.
- **AC-SET-9** Setup drafts autosave; closing the browser loses nothing.
- **AC-SET-10** A new study first asks for a setup path, remembered in the browser: **Quick**
  (species → study area → "Check & launch"; one species; data, variables, resolution — the finest
  the area fits — and model keep the recommended settings without being shown) or **Expert** (every
  step). A Quick / Expert switch above the steps changes path at any time without losing choices.
  Both paths build the same JobSpec; Quick hides no data warning, and its review lists every
  non-default setting (changed in Expert mode or copied from a duplicated study) as AC-SET-8 requires (`web/src/features/setup/PathChooser.tsx`, `stores/draft.ts`).

### Expert mode
- **AC-EXP-1** All model parameters affecting outputs are reachable in Expert mode.
- **AC-EXP-2** Modified-parameter count is visible in both modes; each group resets to defaults.
- **AC-EXP-3** Invalid combinations are rejected before launch with field-level explanation.
- **AC-EXP-4** Config exports as JSON identical to what the run stores in provenance.

### Execution
- **AC-RUN-1** Execution shows current stage, fold i/k where applicable, elapsed time, an ETA
  range or none, last-update time, live warning count, and Cancel.
- **AC-RUN-2** The map and the rest of the app remain usable during a run; completion is
  notified wherever the user is.
- **AC-RUN-3** Cancel requires confirmation, stops the run, keeps logs and config.
- **AC-RUN-4** Failures state stage, cause in operational/ecological terms, consequence, and at
  least one action; technical details are collapsed but available.
- **AC-RUN-5** Progress never decreases; no spinner without text.

### Results
- **AC-RES-1** Results open on Suitability with fixed [0,1] viridis domain, observations and
  extrapolation overlay visible, legend with title, ticks and "What does this mean?".
- **AC-RES-2** Area card shows value (3 sf), unit, share of study area, threshold rule and
  sensitivity range.
- **AC-RES-3** Validation card shows continuous Boyce index and omission rate (observed vs
  expected) as mean ± SD with k and scheme; AUC appears only in the Validation tab, labelled
  "presence-background"; Validation tab shows per-fold table (metric roles per
  [07-scientific-validation](../research/07-scientific-validation.md)).
- **AC-RES-4** Uncertainty layer reachable in one action; uses a ramp distinct from suitability;
  legend states the metric and its units.
- **AC-RES-5** Threshold changes update binary map and area readout within 300 ms on reference
  hardware [ASSUMPTION], rule label always visible.
- **AC-RES-6** Clicking (or keyboard-inspecting) the map shows suitability, uncertainty,
  extrapolation status and nearest observation.
- **AC-RES-7** Provenance tab lists config, data sources + versions, software versions, seed,
  timestamps, checksums.
- **AC-RES-8** Copy lint finds no "probability", "%" adjacent to suitability, or area without rule
  in UI strings or exports.
- **AC-RES-9** "Completed with warnings" banner appears above summary cards and in exports.
- **AC-RES-10** Results open in a **Simple** or **Full** reading (switch above the results, kept
  in the URL as `view=`; default Simple for visitors on the Quick setup path, Full otherwise).
  Simple reading, in this order: every warning; what the map shows (relative suitability, share
  of hatched extrapolation, records kept or excluded); the area of suitable conditions with its
  band across threshold rules; the evidence level with all its reasons; data, climate source and
  model in one sentence each; what the map does not say. The map shows only the suitability layer
  with the extrapolation hatch. Metrics, variables, threshold rules and exports stay in Full
  (`web/src/features/results/SimpleSummary.tsx`).

### Exports
- **AC-EXPORT-1** Single entry point (ExportDialog) with presets Report / GIS data / Reproducibility;
  outputs embed threshold, rule, caveats and provenance; filenames include species, run, threshold.

### Batch
- **AC-BAT-1** CSV import resolves names into accepted / synonym / unresolved with inline fixes.
- **AC-BAT-2** Pre-flight lists species below occurrence minimum with default "exclude, keep
  listed" and per-row override.
- **AC-BAT-3** Grid shows status counts, filters by status, and remains responsive (scroll ≥ 50
  fps [ASSUMPTION]) with 500 rows.
- **AC-BAT-4** Failure causes are aggregated; "Retry failed" re-queues only retryable failures
  with unchanged config.
- **AC-BAT-5** Row selection opens results in a side panel without losing grid state.
- **AC-BAT-6** Comparison table shows n, CBI and omission with spread (no AUC across species), area with range, warnings; batch
  export works with partial completion and includes a manifest.

### Comparison
- **AC-CMP-1** 2–4 runs, synced cameras, shared legend domain, config diff table; no automatic
  "best model" label.

### Visual & accessibility
- **AC-VIS-1** All colours come from semantic tokens; light and dark modes pass contrast checks
  listed in design-tokens.
- **AC-VIS-2** No rainbow/red–green ramps; state never conveyed by colour alone.
- **AC-A11Y-1** WCAG 2.2 AA (axe: zero serious/critical violations on key screens).
- **AC-A11Y-2** J1 and J4 completable keyboard-only, including study area definition (via admin
  unit, upload or coordinate entry) and threshold setting.
- **AC-A11Y-3** Map has a textual/tabular alternative with the same summary information.
- **AC-A11Y-4** Reduced-motion preference removes all non-essential animation.

### Simplicity gate
- **AC-LOS-1** Every screen in §3 has a completed Laws of Simplicity matrix with all Checks passed
  before implementation sign-off.

### Usability validation
- **AC-USE-1** ≥ 4 of 5 Claire-profile testers complete J1 unaided in ≤ 15 min active time.
- **AC-USE-2** ≥ 4 of 5 testers correctly explain, after seeing results, that the score is
  relative and that the area depends on the threshold.

## 6. Open questions

Carried from research: uncertainty metrics produced by the engine; default threshold rule;
uncertainty lightness convention; narrow scope; regulatory report templates; primitive base
(Base UI vs Radix); RGAA requirement.

## References

See [09-ux-research](../research/09-ux-research.md#references).
