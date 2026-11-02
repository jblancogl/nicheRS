# Accessibility requirements

Date: 2026-10-15. Target: **WCAG 2.2 Level AA** (W3C Recommendation, 12 Dec 2024) [23].
[ASSUMPTION] Public-sector clients in France may additionally require RGAA conformance (RGAA is
aligned with WCAG); [OPEN] confirm with Natural Solutions.

## 1. Requirements by WCAG criterion (selection relevant to NicheRS)

| SC | Level | NicheRS requirement |
|---|---|---|
| 1.1.1 Non-text content | A | Map canvas has an accessible name and a text summary (§4); legend ramps have text equivalents (domain, ticks, meaning) |
| 1.3.1 Info & relationships | A | Tables are real `<table>` with headers; FieldSet/Legend groups; stepper is an ordered list with `aria-current="step"` |
| 1.4.1 Use of colour | A | Every state = icon + text + colour; folds = colour + pattern + number; observations = shape; extrapolation = hatch [23] |
| 1.4.3 Contrast (min) | AA | Text ≥ 4.5:1 (large ≥ 3:1); token pairs verified in [design-tokens](design-tokens.md) (e.g. muted-foreground 6.0:1 light, 7.9:1 dark) |
| 1.4.10 Reflow | AA | At 320 CSS px: single column read-only layout (W10); no 2-D scrolling except map and wide data tables (exempt content) |
| 1.4.11 Non-text contrast | AA | Input boundaries `--input` ≥ 3:1; focus ring ≥ 3:1; markers ≥ 3:1 against raster via dual stroke; map controls on solid backgrounds |
| 1.4.12 Text spacing | AA | No fixed-height text containers in panels |
| 1.4.13 Content on hover/focus | AA | Tooltips/hover cards dismissible (Esc), hoverable, persistent; caveats never tooltip-only |
| 2.1.1 Keyboard | A | All functions keyboard-operable, including map inspect, drawing alternatives, swipe, threshold (§3) |
| 2.1.2 No keyboard trap | A | Map canvas releases focus with Tab/Shift+Tab; Esc exits draw mode |
| 2.1.4 Character key shortcuts | A | Single-key shortcuts (`I`, `j`/`k`, `L`) active only when map/grid has focus, and can be turned off in Settings |
| 2.2.2 Pause, stop, hide | A | No auto-updating content except progress; live updates do not move focus |
| 2.4.3 Focus order | A | Context bar → panel → map controls → map; skip links "Skip to panel", "Skip to map", "Skip to results table" |
| 2.4.7 Focus visible / 2.4.11 Focus not obscured (min) | AA | 2 px ring + offset; sticky context bar and bottom sheet must not cover focused element (scroll-padding) |
| 2.5.7 Dragging movements | AA | Every drag (vertices, slider thumb, swipe divider, panel resize, bottom sheet) has a click/keyboard alternative |
| 2.5.8 Target size (min) | AA | Targets ≥ 24 × 24 CSS px or spaced per the 24 px circle rule [24]; map control buttons 32 px; observation markers are not sole selection targets (table alternative) |
| 3.2.6 Consistent help | A | Help entry in the same rail position on every page |
| 3.3.1 / 3.3.3 Error identification & suggestion | A/AA | FieldError with cause + fix; ecological error copy pattern |
| 3.3.7 Redundant entry | A | Batch/duplicate reuse config; species list never re-entered |
| 3.3.8 Accessible authentication | AA | No cognitive tests at login; password managers/paste allowed |
| 4.1.2 Name, role, value | A | shadcn primitives (Base UI/Radix) provide ARIA; custom map controls get roles/labels |
| 4.1.3 Status messages | AA | `role="status"` for run stage changes, autosave, threshold area readout (polite, debounced) |

## 2. Focus management

| Event | Focus goes to |
|---|---|
| Step Continue | Next step heading (`h2`, `tabindex=-1`) |
| Continue with errors | First invalid field; error summary announced |
| Dialog open/close | First focusable in dialog / back to invoking control |
| Run launched | Execution panel heading; announcement "Run #2 started" |
| Run completed (user elsewhere) | Focus unchanged; toast announced politely, with "Open results" action reachable via F6 region cycling |
| Row opened in batch side panel | Panel heading; Esc returns to the row |
| Map inspect | Popover content; Esc returns to map |
| Delete with undo | Toast receives no focus; Undo reachable via shortcut `Ctrl+Z` within 10 s |

Regions cycle with F6: rail → context bar → panel → map.

## 3. Keyboard map

| Scope | Key | Action |
|---|---|---|
| Global | Ctrl/⌘+K | Command palette |
| Global | F6 / Shift+F6 | Cycle regions |
| Global | ? | Shortcut help (Dialog) |
| Global | Ctrl/⌘+Z | Undo last reversible action |
| Map (focused) | Arrows | Pan (100 px) [25] |
| Map | + / − (Shift ×2) | Zoom [25] |
| Map | 0 | Zoom to study area |
| Map | I | Toggle inspect crosshair; arrows move it; Enter inspects |
| Map | L | Open layer control |
| Map | U | Toggle suitability ↔ uncertainty |
| Map | O | Toggle observations |
| Map | Esc | Exit draw/inspect mode |
| Draw/edit | Tab / Shift+Tab | Next/previous vertex |
| Draw/edit | Arrows (Shift) | Move vertex 1 px (10 px) |
| Draw/edit | Delete / Insert | Remove vertex / add midpoint |
| Draw/edit | Enter | Close polygon |
| Slider (threshold) | Arrows / PageUp-Down / Home-End | ±0.01 / ±0.10 / 0, 1 |
| Swipe | Arrows | Move divider 5 % |
| Batch grid | j / k, Enter, x | Next/prev row, open, select |
| Stepper | Alt+← / Alt+→ | Previous / next step (if valid) |

Single-key shortcuts are scoped and can be disabled (SC 2.1.4).

## 4. Map accessibility alternatives

[FACT] MapLibre provides keyboard pan/zoom/rotate but its KeyboardHandler docs describe no ARIA
semantics [25]. [ASSUMPTION] The WebGL canvas is opaque to screen readers. Therefore:

1. **Map summary** (`MapTableAlternative`, visually available via "Table view" toggle and always
   to screen readers): study area name & size; counts of observations (training/test/excluded);
   suitability distribution (histogram as table: 5 classes 0–0.2…0.8–1 with km² and %);
   predicted suitable area at current threshold with rule and range; extrapolated share;
   uncertainty summary.
2. **Zonal table**: suitability statistics by admin unit within the study area (mean, max,
   suitable km²), sortable, linked to map highlight.
3. **Observations table**: id, date, coordinates, source, status (training/test/excluded +
   reason), suitability at point.
4. **Inspect by coordinates/place**: search a place or enter coordinates → value readout.
5. **Exports**: CSV equivalents of every chart and map summary.
6. Canvas element: `role="region"` with `aria-label="Map: suitability for Lacerta bilineata"`
   and `aria-describedby` → summary; avoid `role="application"` on the whole map (it suppresses
   screen-reader navigation) [UNVERIFIED – prior knowledge].

## 5. Colour-blind safety

- Suitability ramps viridis/cividis are perceptually uniform and CVD-readable [16][17]; cividis
  available as option for viewers who prefer it.
- Uncertainty ramp is lightness-only (near-zero chroma) — readable under all CVD types and in
  greyscale.
- State colours never alone: icons ✓ ⚠ ✕ ◐ ○ plus text.
- Categorical palette limited to 5 Okabe–Ito colours with direct labels/shapes.
- Folds use number labels + hatch patterns in addition to colour.
- [REC] CI check: render key screens through CVD simulation (protan, deutan, tritan) and
  greyscale in visual regression tests ([OPEN] tool choice).

## 6. Motion

- Respect `prefers-reduced-motion: reduce`: no `flyTo` animations, no panel slide, spinners
  replaced by static icon + text, progress bar without shimmer.
- No auto-playing animation; no motion used to convey meaning on its own.

## 7. Language and content

- `lang="fr"` default, `lang="en"` when selected; scientific names wrapped in `<i lang="la">`.
- Plain-language definitions for SDM terms available via disclosure, not only tooltip.
- Units read correctly: use `<abbr title="kilomètres carrés">km²</abbr>`.

## 8. Testing

| Method | Frequency |
|---|---|
| axe-core in component tests (Vitest + Testing Library) and Playwright e2e | every PR |
| Keyboard-only walkthrough of J1 and J4 | every release |
| Screen reader (NVDA + Firefox, VoiceOver + Safari) on Setup, Execution, Results summary | every release |
| Contrast token check script (from design-tokens values) | on token change |
| CVD/greyscale snapshot review | on ramp or state colour change |

## References

[16] Crameri 2020; [17] Nuñez 2018; [23] WCAG 2.2; [24] Understanding 2.5.8; [25] MapLibre
KeyboardHandler — see [09-ux-research](../research/09-ux-research.md#references).
