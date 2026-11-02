# Laws of Simplicity audit

Date: 2026-10-15. Mandatory review criteria [1]. Applies to every significant screen state; a
screen cannot be marked "design ready" while a row has an unresolved **Violation** or an
unchecked **Check**.

Laws (verbatim one-liners [1]): 1 **Reduce** — thoughtful reduction. 2 **Organize** — many
appear fewer. 3 **Time** — savings in time feel like simplicity. 4 **Learn** — knowledge makes
everything simpler. 5 **Differences** — simplicity and complexity need each other. 6 **Context**
— what lies in the periphery is not peripheral. 7 **Emotion** — more emotions are better than
less. 8 **Trust** — in simplicity we trust. 9 **Failure** — some things can never be made simple.
10 **The One** — subtract the obvious, add the meaningful.

Legend: **A** = how the law is applied; **R** = risk of violation; **C** = acceptance check.
Screen ids match [wireframes](wireframes.md) and [information-architecture](information-architecture.md).

## S0 Workspace (empty / project home)

| Law | A | R | C |
|---|---|---|---|
| Reduce | Left rail 4 items; one CTA "New study" | Template gallery, news, tips clutter | ≤ 1 primary button, ≤ 4 nav items |
| Organize | Studies grouped by project; status badges | Flat list of runs | Studies list groups runs under study |
| Time | Recent studies first; resume last view | — | "Continue" restores exact last view |
| Learn | Empty state explains the 6 steps in one line each | Long onboarding tour | Empty state ≤ 60 words, links to guide |
| Differences | Map shown even when empty (territory basemap) | Blank page | Basemap renders in empty state |
| Context | Context bar shows active project | Lost project context | Project name always visible |
| Emotion | Calm, plain copy | Gamification | No celebratory copy |
| Trust | Shows last-run status honestly (incl. failed) | Hiding failures | Failed runs visible in list |
| Failure | — | — | n/a |
| The One | "New study" | Multiple equal CTAs | Single primary |

## S1 Setup · Species

| Law | A | R | C |
|---|---|---|---|
| Reduce | One combobox; results show name, authority, rank, record count | Taxonomy tree browsing | Selection in ≤ 3 keystrokes + Enter for a known name |
| Organize | Synonyms collapsed under accepted name | Duplicate taxa listed | Synonyms grouped |
| Time | Recent species, CSV paste for many | Retyping | Recent list ≤ 5 |
| Learn | Shows source (GBIF/INPN [ASSUMPTION]) and record count before commit | Unknown provenance | Count + source shown pre-commit |
| Differences | 1 species → single study; ≥ 2 → batch offer | Batch forced on single user | Batch offer appears only at ≥ 2 |
| Context | Observations preview on map immediately | Map empty until later | Points render ≤ 2 s after selection (cached) |
| Emotion | — | — | — |
| Trust | Name resolution states "accepted / synonym of / ambiguous" | Silent resolution to wrong taxon | Ambiguous names block until resolved |
| Failure | Too few records → explain consequence ("< 10 occurrences: models are unreliable") | Generic "invalid" | Warning names the threshold and its rationale |
| The One | "Continue" | — | Single primary |

## S2 Setup · Study area

| Law | A | R | C |
|---|---|---|---|
| Reduce | Three ways: admin unit, draw, upload; default = suggested from occurrences (convex hull + buffer) | Too many geometry tools | ≤ 3 entry methods visible |
| Organize | Segmented control for method | Mixed toolbars | Toggle Group, one active method |
| Time | Suggested area pre-filled | Drawing from scratch | Suggestion present if ≥ 1 occurrence |
| Learn | Shows area km², occurrences inside/outside, raster coverage | User unaware of out-of-area points | Counters update live |
| Differences | Expert: buffer distance, CRS | CRS shown to novices | CRS in Expert only, default EPSG shown in Review |
| Context | Map is the editor | Modal editor | No modal |
| Emotion | Direct manipulation, snappy | Laggy drawing | Vertex drag ≤ 16 ms frame budget [ASSUMPTION] |
| Trust | Shows excluded occurrences explicitly (dashed ×) | Silent drop | Excluded count with reason |
| Failure | Invalid geometry → auto-repair offer with preview | Hard fail | Self-intersection repaired or explained |
| The One | "Use this study area" | — | Single primary |

## S3 Setup · Environmental data

| Law | A | R | C |
|---|---|---|---|
| Reduce | Recommended set pre-selected (collinearity-filtered) | 19 bioclim + land cover all on | Default set ≤ 10 vars [ASSUMPTION], rationale shown |
| Organize | Group by theme (climate, topography, land cover, soil) | Flat list | Accordion groups |
| Time | One-click accept | Per-variable review forced | Accept in 1 action |
| Learn | Each var: name, unit, source, resolution, year, thumbnail | Codes like `bio12` alone | Human label + code |
| Differences | Correlation matrix in Details | Matrix forced on novices | Matrix collapsed by default |
| Context | Hover var → preview on map | Separate preview page | Preview layer on map |
| Emotion | — | — | — |
| Trust | Resolution/CRS mismatch flagged; resampling stated | Silent resampling | Resampling method in Review |
| Failure | Data unavailable for area → clear cause + alternative | "Error 500" | Domain-language error |
| The One | "Use these N variables" | — | Single primary |

## S4 Setup · Model configuration

| Law | A | R | C |
|---|---|---|---|
| Reduce | Guided: MaxEnt, defaults; 0 required decisions | Feature classes, β, background exposed | Guided panel ≤ 3 visible controls |
| Organize | Guided / Expert tabs over one config | Two divergent configs | Expert edits reflected in Guided summary |
| Time | Presets ("Standard", "Conservative (simpler model)") | — | Preset selection in 1 click |
| Learn | Inline "what this changes" for each preset | Jargon | Each preset has ≤ 25-word explanation |
| Differences | Expert mode for Thomas | Expert params leak into Guided | Non-default Expert values badge "Modified (3)" |
| Context | Map shows background sample extent | — | Background extent toggle |
| Emotion | — | — | — |
| Trust | Defaults cite their source (e.g. maxnet defaults) | Unexplained magic numbers | Each default has provenance tooltip + doc link |
| Failure | Incompatible combos prevented by Zod schema with explanation | Run fails late | Validation before launch |
| The One | "Continue" | — | Single primary |

## S5 Setup · Validation plan

| Law | A | R | C |
|---|---|---|---|
| Reduce | Default: spatial block CV, k = 5 [ASSUMPTION] | Choice of 6 schemes up front | Default preselected |
| Organize | Scheme + preview of folds on map | — | Folds coloured with categorical palette + pattern |
| Time | — | — | — |
| Learn | Explains why spatial blocks (autocorrelation) | — | ≤ 40-word rationale |
| Differences | Random k-fold, jackknife in Expert | — | Expert-only schemes |
| Context | Folds on map | — | Map preview |
| Emotion | — | — | — |
| Trust | Warns when n too small for k | Silent fold failure | Warning with recommended k |
| Failure | Jackknife suggested for n < 25 [ASSUMPTION] | — | Suggestion shown |
| The One | "Continue" | — | Single primary |

## S6 Setup · Review & launch

| Law | A | R | C |
|---|---|---|---|
| Reduce | Summary list, non-defaults highlighted | Full parameter dump | Defaults collapsed, non-defaults listed |
| Organize | Same 5 sections as steps, each "Edit" | — | Edit returns to step, keeps state |
| Time | Estimated run time as range | Precise ETA | Range or "unknown" |
| Learn | Pre-flight checks list | — | Checks visible with pass/warn/fail |
| Differences | — | — | — |
| Context | Map shows area + occurrences + background | — | — |
| Emotion | Confidence from clarity | — | — |
| Trust | Warnings must be acknowledged, not hidden | Launch with silent warnings | Checkbox "I understand" per blocking-level warning |
| Failure | Blocking errors disable launch with reason | Disabled button without why | Disabled button has visible reason text |
| The One | "Launch model run" | — | Single primary |

## S7 Execution

| Law | A | R | C |
|---|---|---|---|
| Reduce | Stage list + one progress bar + cancel | Raw log first | Log collapsed |
| Organize | Stages: Fetch occurrences → Prepare rasters → Sample background → Train (fold i/k) → Predict → Evaluate → Package | Undifferentiated % | Stage names shown |
| Time | Non-blocking; user can navigate away; notification on completion | Blocking modal | Map remains interactive |
| Learn | Each stage has one-line description | — | — |
| Differences | Resources (CPU, memory) in Details | Resource meters for novices | Collapsed |
| Context | Run summary (species, area, config) above progress | — | — |
| Emotion | Calm; no spinner-only states | Anxiety from silence | Heartbeat "last update 3 s ago" |
| Trust | Warnings stream as they occur | Warnings only at end | Warning count live |
| Failure | Cancel with confirmation; partial logs kept; failure explains stage + cause + next step | Lost logs | Logs retained after cancel/fail |
| The One | "Cancel run" (destructive, secondary styling) — no primary while running | — | No competing primaries |

## S8 Results

| Law | A | R | C |
|---|---|---|---|
| Reduce | Map + 3 cards (Suitable area, Validation, Reliability) | 15 metrics | ≤ 3 summary cards above fold |
| Organize | Tabs: Summary · Validation · Variables · Provenance | — | 4 tabs max |
| Time | Threshold change updates map + area < 300 ms [ASSUMPTION] | Recompute server round-trip | Client-side reclass of cached raster |
| Learn | Legend disclosure "What does 0.7 mean?" | Unlabelled legend | Legend has title, units, domain, disclosure |
| Differences | Response curves, permutation importance in Variables tab | Forced on all | Tab, not default |
| Context | Observations overlay toggle; study area outline | Predictions shown without observations context | Obs toggle default on |
| Emotion | Clear completion statement | — | — |
| Trust | Area always with rule + range; CBI/omission ± SD (k); AUC only in Validation tab, labelled presence-background; extrapolation hatch on | Single-number claims | Copy lint passes |
| Failure | "Completed with warnings" banner lists consequences | Buried warnings | Banner above cards |
| The One | "Export" | Many export buttons | Single ExportDialog entry |

## S9 Batch (overview + detail)

| Law | A | R | C |
|---|---|---|---|
| Reduce | Status counts + table; one action "Retry failed" when failures exist, else "Export" | Per-row buttons everywhere | Row actions in menu |
| Organize | Group by status; filter chips | Unsorted 500 rows | Status filter + sort |
| Time | Retry failed only; reuse config | Rerun all | Retry scope = failed rows |
| Learn | Failure reasons aggregated ("23 species: < 10 occurrences") | 23 identical errors | Aggregated reasons |
| Differences | Comparison table with spread columns | — | — |
| Context | Clicking a row opens results in side panel, keeps table | Navigating away | Split view |
| Emotion | — | — | — |
| Trust | Excluded species listed with reason | Silent drop | Excluded count visible |
| Failure | Partial batch completion is a valid state | All-or-nothing | Export works with partial results, labelled |
| The One | Context-dependent single primary | — | Single primary |

## S10 Expert mode

| Law | A | R | C |
|---|---|---|---|
| Reduce | Grouped parameters, defaults shown greyed | Wall of inputs | Groups collapsed except edited |
| Organize | Groups: Features, Regularisation, Background, Sampling bias, Validation, Variables, Extrapolation/clamping, Output | — | 8 groups |
| Time | Reset group to default | — | Per-group reset |
| Learn | Each param: description, default, valid range, reference | — | Help per field |
| Differences | Exists only here | — | — |
| Context | Diff vs defaults always visible | Forgotten changes | "Modified (n)" badge |
| Emotion | — | — | — |
| Trust | Export config as JSON/R-equivalent [OPEN] | Opaque | JSON export |
| Failure | Zod cross-field rules (e.g. hinge requires n ≥ 15 [ASSUMPTION]) | Late failures | Pre-launch validation |
| The One | "Apply" | — | Single primary |

## S11 Comparison

| Law | A | R | C |
|---|---|---|---|
| Reduce | 2–4 runs max side by side; synced maps | 10 maps | Max 4 |
| Organize | Same legend domain across maps | Different domains | Fixed [0,1] enforced |
| Context | Synced pan/zoom; config diff table | — | Diff highlights changed params |
| Trust | Differences shown with spread; no "winner" label | Auto-ranking | No automatic "best model" badge |
| Other laws | as S8 | | |

## S12 Narrow / mobile (read-only review)

| Law | A | R | C |
|---|---|---|---|
| Reduce | Map + bottom sheet summary | Desktop panels squeezed | Single column |
| Context | Context bar collapses to species + run | Lost context | Species always visible |
| Trust | Same caveats as desktop | Caveats dropped for space | Caveat banner retained |
| Failure | Setup/expert not offered on narrow | Broken forms | "Open on desktop" message |

## Review procedure

1. Designer fills the matrix per screen state (default, loading, empty, error, warning).
2. Reviewer verifies each Check against the prototype; failures logged as issues labelled
   `simplicity:<law>`.
3. Re-audit on any change that adds a control to a panel.

## References

[1] J. Maeda, *The Laws of Simplicity*, https://lawsofsimplicity.com/ (accessed 2026-10-15).
