# Component architecture

Date: 2026-10-15. Stack: React + TypeScript, Vite, Tailwind v4, shadcn/ui, Lucide, TanStack
Query, Zustand (justified uses only), MapLibre GL JS, React Hook Form + Zod.

## 1. shadcn/ui baseline

- [FACT] As of the 2026-10-15 changelog, shadcn/ui lists "Base UI as the Default" (Jul 2026),
  React Aria variants (Jul 2026), CLI v4 (Mar 2026), a standalone `cn` package (Sep 2026), and the
  docs `components.json` example uses `"style": "base-nova"` [2][3]. [REC] Pin a single primitive
  base for the whole app in `components.json` (Base UI, the current default) and never mix
  variants; [OPEN] verify Slider (range, keyboard steps) and Combobox (async, virtualised) in the
  chosen base before committing.
- [REC] Components are copied into `src/components/ui/` (shadcn model) and are not edited for
  domain logic; domain components live in `src/components/domain/` and compose them.
- [REC] Use `--input`/state tokens from [design-tokens](design-tokens.md); no hard-coded colours.

### 1.1 Primitives used

| shadcn component [4] | Where |
|---|---|
| Sidebar | Global left rail (collapsed icon mode) |
| Resizable | Map ↔ side panel split |
| Breadcrumb, Badge | Context bar, status |
| Tabs | Panel modes' secondary tabs; Guided/Expert |
| Combobox / Command | SpeciesSelector, command palette (⌘K), layer search |
| Field, FieldSet, FieldLegend, FieldError, Input, Input Group, Select, Native Select, Radio Group, Checkbox, Switch, Slider, Toggle Group | All forms (RHF + Zod) |
| Accordion, Collapsible | Progressive disclosure ("Details", variable groups, Expert groups) |
| Card | MetricCard, summary blocks |
| Progress, Spinner | JobProgress |
| Alert, Alert Dialog | Warnings banner; confirm cancel/delete |
| Dialog, Sheet, Drawer | ExportDialog (Dialog), narrow-screen bottom sheet (Drawer), filters (Sheet) |
| Popover, Hover Card, Tooltip | Map inspect popover, legend disclosure, term definitions |
| Data Table (+ TanStack Table), Table, Pagination, Scroll Area | Batch grid, comparison, variable lists, map tabular alternative |
| Chart | Response curves, importance, fold metrics (Recharts-based [UNVERIFIED – prior knowledge]) |
| Toast | Completion, undo |
| Empty | Empty states |
| Item, Kbd, Separator, Skeleton, Button, Button Group, Dropdown Menu | General |

Not used: Carousel, Input OTP, Questionnaire (evaluated, [OPEN]), AI/agent components.

## 2. State and data

| Concern | Tool | Rule |
|---|---|---|
| Server state (projects, runs, metrics, layers metadata) | TanStack Query | Query keys `['run', runId]`, etc.; run status via SSE/WebSocket pushing into query cache [ASSUMPTION: transport decided in API docs] |
| Shareable view state | URL search params | layer, threshold, rule, camera, tab |
| Setup draft | React Hook Form + Zod; autosaved to server (debounced 1 s) | One `StudyConfig` Zod schema shared by Guided and Expert |
| Transient UI (panel width, draw mode, hovered feature, map instance ref) | Zustand (`useWorkspaceStore`) | Justified: cross-tree access between map and panel without prop drilling; nothing persisted there that must be shared |
| Map | MapLibre instance owned by `<MapCanvas>`; layers declared via `useMapLayer(spec)` hooks | Components never call map APIs imperatively outside hooks |

## 3. Domain components

Common conventions: every component accepts `className`; async components render
`loading` (Skeleton), `empty` (Empty), `error` (Alert with action) states; all numbers via
`formatScientific`.

### SpeciesSelector
- Purpose: pick 1..N taxa with name resolution.
- Props: `value: TaxonRef[]`, `onChange(TaxonRef[])`, `mode: 'single' | 'multi'`,
  `territoryId?: string` (to show in-area counts), `allowCsvImport?: boolean`, `disabled?`.
- States: idle · searching · results · no match · ambiguous (requires choice) · synonym
  (auto-mapped, shows "synonym of …") · selected · error (source unavailable, retry).
- Shows per item: scientific name (italic) + authority, vernacular (fr), rank, status, occurrence
  count + source + date.
- Primitives: Combobox/Command, Popover, Badge, Item, Skeleton, Dialog (CSV import), Data Table
  (resolution review).
- A11y: combobox ARIA pattern; results announced "12 results".

### TerritorySelector
- Purpose: define study area.
- Props: `value: StudyArea | null`, `onChange`, `methods: ('suggested'|'admin'|'draw'|'upload')[]`,
  `occurrences?: FeatureCollection`, `suggestion?: StudyArea`, `expert?: boolean` (buffer, CRS).
- States: empty · suggested · drawing · editing · uploading · validating · invalid (repair offer)
  · valid (area, in/out counts, coverage).
- Primitives: Toggle Group (method), Combobox (admin units), Button Group (undo/redo/edit),
  Input Group (buffer km), Alert (invalid geometry), Field.
- Map: registers draw layers via `useMapLayer`; keyboard vertex editing (see interaction spec).

### EnvVariableSelector
- Props: `available: EnvLayerMeta[]`, `value: string[]`, `recommended: string[]`,
  `correlation?: Matrix`, `onChange`, `onPreview(layerId|null)`, `expert?: boolean`.
- States: loading catalogue · recommended applied · custom · collinearity warning · unavailable
  for area · resolution mismatch (shows resampling method).
- Item display: human label, code (mono), unit, source, resolution, period, licence.
- Primitives: Accordion (themes), Checkbox, Item, Badge, Tooltip, Table (correlation matrix, also
  heatmap via Chart), Alert.

### ModelConfigPanel
- Props: `form: UseFormReturn<StudyConfig>`, `mode: 'guided' | 'expert'`, `onModeChange`,
  `presets: Preset[]`, `defaults: StudyConfig`, `readOnly?: boolean`.
- States: guided (preset) · expert (groups) · modified (n badge) · invalid (FieldError per field +
  summary) · read-only (in Results provenance).
- Expert groups: Features, Regularisation, Background, Sampling bias, Validation, Variables,
  Extrapolation/clamping, Output (transform, seed).
- Primitives: Tabs, Radio Group, Accordion, Field/FieldSet/FieldError, Input, Slider, Switch,
  Checkbox, Badge, Button ("Reset group").
- Rule: Guided and Expert edit the same RHF form; switching never loses values.

### MetricCard
- Props: `label: string`, `value: number | null`, `spread?: { kind: 'sd'|'range'|'ci'; low?: number; high?: number; sd?: number }`,
  `n?: { k?: number; label: string }`, `unit?: string`, `format: FormatSpec`, `caveat?: ReactNode`,
  `status?: 'ok'|'warning'|'danger'|'neutral'`, `definition: ReactNode` (required),
  `missingReason?: string`.
- States: loading · value · value with spread · missing (— + reason) · warning.
- Invariant: if the metric type declares a spread, the card refuses to render without it (dev
  assertion) — enforces "no unsupported certainty". No percentage formatting for scores.
- Primitives: Card, Badge, Hover Card/Popover (definition), Skeleton.

### SuitabilityLegend
- Props: `quantity: 'suitability'|'uncertainty'|'binary'|'extrapolation'`, `ramp: RampId`,
  `domain: [number, number]`, `stretched?: boolean`, `threshold?: { value: number; rule: string }`,
  `unit?: string`, `opacity: number`, `showObservations: boolean`.
- Renders: title ("Suitability score (relative, 0–1)"), continuous bar with ticks at 0/0.25/0.5/
  0.75/1 (tabular numerals), threshold tick, extrapolation hatch swatch, separate "Observed"
  section with marker shapes, "What does this mean?" disclosure (score is relative; not a
  probability of presence unless assumptions hold [19][21]).
- States: suitability · uncertainty · binary · stretched (flag) · compact (mobile).
- Primitives: Popover, Separator, Badge (Stretched).

### RasterLayerControl
- Props: `layers: LayerDescriptor[]`, `active: LayerId`, `onActiveChange`, `opacity`,
  `onOpacityChange`, `overlays: { extrapolation: boolean; observations: boolean; studyArea: boolean }`,
  `onOverlayChange`, `rampOptions: RampId[]`, `compare?: { mode: 'swipe'|'side'; target?: LayerId }`.
- Groups "Modelled" (radio — one prediction raster at a time) and "Observed" (checkboxes).
- Primitives: Dropdown Menu or Popover, Radio Group, Checkbox, Slider (opacity), Select (ramp:
  viridis/cividis), Toggle (swipe).

### JobProgress
- Props: `run: RunStatus` ({stage, stages[], fold?, percent?, startedAt, etaRange?, lastHeartbeat,
  warnings[], resources?}), `onCancel`, `variant: 'panel'|'row'|'badge'`.
- States: queued (position) · running (stage, fold i/k, % if determinate else indeterminate with
  stage text) · stalled (no heartbeat > 60 s [ASSUMPTION]: warning, not failure) · cancelling ·
  cancelled · completed · completed-with-warnings · failed (stage + cause + actions).
- Primitives: Progress, Spinner (static icon under reduced motion), Item list, Badge, Alert,
  Alert Dialog (cancel confirm), Collapsible (resources/log), Scroll Area (log).
- A11y: `role="status"` live region announcing stage changes only (not every % tick).

### ValidationReport
- Props: `metrics: FoldMetrics[]`, `scheme: ValidationScheme`, `threshold: Threshold`,
  `curves?: {roc?, boyce?}`, `warnings: Warning[]`.
- Content: scheme sentence; table per fold + mean ± SD. Primary: continuous Boyce index with P/E
  curve, omission rate at threshold vs expected; secondary: AUC labelled "AUC (presence-background)"
  with caveat that it compares models of the same species/extent only; fold map toggle
  (metric roles per [07-scientific-validation §2.3](../research/07-scientific-validation.md)).
- States: loading · complete · partial (some folds failed) · not applicable (e.g. n too small →
  explains why).
- Primitives: Table, Chart, Card, Alert, Tabs.

### UncertaintyIndicator
- Props: `assessment: { level: 'low'|'moderate'|'high'|'unknown'; reasons: Reason[] }`
  where each `Reason = { code, label, metric?: {name, value, unit}, rule: string }`,
  `variant: 'card'|'inline'|'badge'`.
- Behaviour: level is derived by explicit, documented rules (tiers defined in
  [07-scientific-validation](../research/07-scientific-validation.md), modelled on Araújo et al. 2019) (e.g. extrapolated share > 10 % of
  area → moderate [ASSUMPTION thresholds, to be fixed by science team]); the card always lists
  reasons with underlying metrics. **No percentage confidence is ever shown.** `unknown` is a
  first-class state ("Not assessed: replicate runs disabled").
- Primitives: Card, Badge (icon + text), Popover, Item.

### ExportDialog
- Props: `scope: { kind: 'run'|'batch'|'comparison'; ids: string[] }`, `formats: ExportFormat[]`,
  `defaults`, `onExport(spec)`.
- Options: presets "Report bundle" (PNG map + legend + scale + caveats, PDF/summary, CSV metrics,
  methods text fr/en), "GIS data" (GeoTIFF/COG suitability, uncertainty, binary at chosen
  threshold, GeoPackage occurrences + study area), "Reproducibility" (config JSON, provenance,
  data versions, checksums); custom selection; CRS selector; threshold included in filenames.
- States: choosing · preparing (progress, background) · ready (download links) · partial (batch:
  lists missing species) · failed.
- Primitives: Dialog, Radio Group, Checkbox, Select, Field, Progress, Alert.

### Supporting components
`MapCanvas`, `ContextBar`, `SetupStepper`, `PreflightChecklist`, `WarningsBanner`,
`ThresholdControl` (Slider + rule Select + numeric Input + area readout), `InspectPopover`,
`BatchGrid` (Data Table virtualised), `ConfigDiffTable`, `ProvenancePanel`, `MapTableAlternative`.

## 4. Folder layout

```
src/
  components/ui/          # shadcn copies (untouched)
  components/domain/      # components above
  features/{setup,execution,results,batch,compare,data}/
  map/                    # MapCanvas, useMapLayer, ramps (LUT), patterns
  lib/format/             # formatScientific, units
  lib/schema/             # Zod: StudyConfig, BatchConfig
  styles/tokens.css
```

## References

[2] shadcn/ui Theming; [3] Changelog; [4] Components; [19] Phillips 2017 / AMNH; [21] Yackulic
2013 — see [09-ux-research](../research/09-ux-research.md#references).
