# Interaction specifications

Date: 2026-10-15. Timing values marked [ASSUMPTION] are targets to validate in prototype.

## 1. Global

| Interaction | Spec |
|---|---|
| Primary action | Exactly one per panel state; `Enter` in last field of a step triggers it if valid |
| Autosave | Setup drafts saved 1 s after last change; context bar shows "Saved" / "Saving…" / "Not saved — retry" |
| Undo | Reversible ops (delete study/run, remove species from batch, reset Expert group) show Toast with Undo for 10 s; deletes are soft (restorable from project trash) |
| Confirmation | Only for irreversible or costly actions: cancel running run, permanent delete, batch relaunch of > 10 runs. Dialog names the object and consequence |
| Navigation guard | Leaving setup with invalid unsaved fields: none needed (autosave); leaving during upload: confirm |
| Feedback latency | < 100 ms visual ack; > 1 s show progress; > 10 s background with status entry [ASSUMPTION] |
| Command palette | ⌘/Ctrl+K: species, runs, layers, actions ("Export", "Toggle uncertainty") |
| Theme | System default; manual light/dark in Settings; map basemap follows theme; data ramps do not |

## 2. Setup stepper

- Steps are navigable (click completed or current+1); future steps disabled until prerequisites
  valid; state icons: ○ not started, ◐ in progress, ✓ valid, ⚠ valid with warnings, ✕ invalid.
- Changing an upstream step that invalidates downstream (e.g. study area change affects
  variable availability) marks downstream steps ⚠ "Review needed" with the reason; values kept
  where still valid.
- "Back" never discards input. Review step "Edit" links jump to the step and return to Review on
  Continue.
- Single species → Study; adding ≥ 2 species surfaces "Create a batch" (secondary), never forced.

## 3. Map interactions

| Interaction | Pointer | Keyboard / non-drag alternative (WCAG 2.5.7) |
|---|---|---|
| Pan/zoom | drag, wheel, pinch | Arrows pan, `+`/`-` zoom (MapLibre KeyboardHandler [25]); zoom buttons; "Zoom to study area" button |
| Rotate/pitch | disabled by default (2D analytical map) | `disableRotation()`; Expert setting to re-enable |
| Inspect value | click → InspectPopover | Focus map, `I` places crosshair at centre; arrows move crosshair; `Enter` inspects; "Show in table" |
| Draw study area | click vertices, double-click close | Coordinates entry form; admin-unit selection; file upload |
| Edit vertices | drag handles | Tab through vertices, arrows move 1 px (Shift = 10 px), `Delete` removes, `Insert` adds midpoint |
| Select observation | click marker | Observations table with row ↔ map highlight |
| Swipe compare | drag divider | Toggle button + arrow keys move divider 5 % |
| Hover preview (env var) | hover item | Focus item shows preview |

Map camera changes write to URL (debounced 500 ms). Reduced motion: `jumpTo` instead of `flyTo`.

## 4. Threshold control

- Components: rule Select (10th percentile training presence, minimum training presence, max
  TSS [OPEN default], fixed value), Slider 0–1 step 0.01, numeric Input (2 decimals), readout.
- Choosing a rule sets the value; moving the slider switches rule to "Custom (0.xx)" — rule label
  always visible.
- Readout: predicted suitable area (3 sf, km²), share of study area, sensitivity range (area at
  rule-set alternatives) and omission rate at this threshold vs expected.
- Map binary layer and area update locally on the cached raster histogram: target < 300 ms
  [ASSUMPTION]; final exact area confirmed server-side if histogram binning is approximate
  (readout shows "≈" until confirmed).
- Threshold persists per run in URL and in exports' filenames/metadata.

## 5. Execution and long-running feedback

| State | Visual | Text | Actions |
|---|---|---|---|
| Queued | ○ badge | "Queued — position 3" | Cancel |
| Running determinate | Progress bar + stage list | "Training · fold 3 of 5" | Cancel |
| Running indeterminate | Indeterminate bar (static under reduced motion) | Stage + "started 40 s ago" | Cancel |
| Stalled (no heartbeat > 60 s) | ⚠ | "No update for 1 min. The run may still be working." | Keep waiting · Cancel |
| Cancelling | spinner | "Stopping after current step…" | — |
| Completed | ✓ toast + context bar badge | "Run #2 completed in 4 min" | Open results |
| Completed with warnings | ⚠ toast | "Completed with 2 warnings — review before use" | Open results |
| Failed | ✕ | Stage + cause + consequence | Retry · alternatives · details |

- UI status ↔ API job state ([job-lifecycle §2](../architecture/job-lifecycle.md), canonical
  enum `queued, preparing, running, finalizing, cancelling, succeeded, failed, cancelled`):

  | API state | UI label | Note |
  |---|---|---|
  | `queued` | Queued (position n) | after recovery with `attempt > 1`: "Resumed after interruption (attempt 2)" — from `job.recovered` event |
  | `preparing` | Running · "Preparing inputs" stage | |
  | `running(stage)` | Running · stage name from `current_stage` | |
  | `finalizing` | Running · "Packaging outputs" | |
  | `cancelling` / `cancelled` | Cancelling… / Cancelled | |
  | `succeeded` | Completed, or Completed with warnings if warnings > 0 | "with warnings" is a UI derivation, not an API state |
  | `failed` | Failed | error code mapped to domain copy (§6) |
  | — | Draft | Study config not yet launched (no job) |

- Progress never moves backwards; if re-estimated, ETA range widens instead.
- ETA shown as a range or omitted; never second-precise countdown.
- Notifications: in-app toast always; browser notification only if user opted in.
- Live region announces stage transitions and terminal state only.

## 6. Error and warning copy pattern

`[What happened, in domain terms] · [Why it matters for the model] · [What you can do]` +
collapsed technical detail.

| Situation | Copy (en; fr is primary in product) |
|---|---|
| < 10 occurrences in area | "Only 7 occurrences fall inside the study area. Models trained on so few records are unreliable and validation cannot be computed. Enlarge the area, relax filters, or exclude this species." |
| Strong clustering | "Most occurrences lie within 2 km of roads. The model may learn survey effort rather than habitat. Consider bias correction (Expert › Sampling bias)." |
| Extrapolation > 10 % [ASSUMPTION] | "17 % of the study area has conditions outside those of the training data. Predictions there are extrapolations (hatched)." |
| Raster source down | see wireframe W11 |
| Fold without test points | "Fold 4 has no test occurrences; spatial validation used 4 folds. Consider larger blocks." |

## 7. Results interactions

- Layer switch (radio): Suitability (default) / Uncertainty / Binary; extrapolation overlay
  checkbox default on; observations default on.
- Swipe/side-by-side to compare suitability vs uncertainty without leaving the view.
- Tabs keep scroll position; tab in URL.
- "Duplicate & edit" creates a draft Study config from this run and opens Review.
- Mark as preferred: explicit, with optional note; shown in comparison and exports.

## 8. Batch interactions

- Creation is table-first (CSV paste/upload → resolution table with inline fix).
- Pre-flight exclusions default to "Exclude, keep listed" — user can override per row.
- Grid: virtualised; status filter chips with counts; sort by any column; multi-select → bulk
  actions (retry, exclude, export, compare ≤ 4).
- "Retry failed" only re-queues retryable failures; non-retryable listed with required change.
- Row click opens side panel; `j`/`k` move between rows while panel stays open.
- Batch export runs in background; partial completion allowed and labelled in manifest.

## 9. Forms (RHF + Zod)

- Validate on blur and on Continue; errors via `FieldError` linked with `aria-describedby`,
  `aria-invalid` on control, `data-invalid` on Field [5].
- On Continue with errors: focus first invalid field; summary Alert at top listing errors as links.
- Cross-field rules (e.g. hinge features require ≥ n occurrences) shown at group level.
- Numeric inputs accept locale decimal separator (`,` in fr) and show units in Input Group addon.

## 10. Responsive behaviour

| Width | Map pages (wizard, study) | Top bar | Other pages |
|---|---|---|---|
| < 768 px | Panel is a non-modal bottom sheet (`app/MapLayout.tsx`) | Search is an icon that opens a full-width field | Settings and administration navigation is a select |
| 768–1023 px | Panel is a 400 px left column, collapsible | Search stays an icon so the location trail keeps its room | Side navigation |
| ≥ 1024 px | Same, 420 px column | Search field centred | Side navigation |

- Bottom sheet positions: peek (22 % of the area under the top bar), half (45 %, default) and
  full (88 %). The handle is dragged and snaps to the nearest position; its button switches
  between half and full (WCAG 2.5.7 alternative to dragging). Focusing a text field opens the
  sheet fully so the on-screen keyboard does not hide it; drawing a polygon brings it down to peek
  and back to half when the drawing ends.
- The sheet publishes its height (`sheetInset` in `stores/map.ts`): fit-to-bounds pads by it and
  the "Layers and legend" panel ends above it.
- Tab lists scroll sideways instead of clipping their last tabs.
- Wide tables scroll sideways inside their container; the cut side fades out and the container
  takes keyboard focus only while it can scroll (`components/ui/table.tsx`).

## References

[5] shadcn Field; [25] MapLibre KeyboardHandler — see
[09-ux-research](../research/09-ux-research.md#references).
