# NicheRS design principles

> Make sophisticated ecological modelling feel simple without hiding scientific complexity or
> uncertainty.

Date: 2026-10-15. Basis: [09-ux-research](../research/09-ux-research.md); review gate:
[laws-of-simplicity-audit](laws-of-simplicity-audit.md).

## 1. Character

A **precise scientific instrument**, not a dashboard or marketing site. Neutral surfaces, strong
typography, subtle borders, one accent, semantic state colours, light and dark modes. No
decorative gradients, illustrations, or animation that does not explain a state change. The
raster carries the colour; the chrome stays grey. Colour ramps are chosen for data semantics and
perceptual readability (see [design-tokens §3](design-tokens.md#3-raster-colour-ramps)), never
for brand.

## 2. Principles

Each principle lists the Maeda law(s) it operationalises [1] and a testable rule.

| # | Principle | Laws | Testable rule |
|---|---|---|---|
| P1 | **The map is the workspace.** It never disappears during spatial work; panels sit beside it. | Context, Organize | Map canvas ≥ 55 % of viewport width at ≥ 1280 px in setup, execution and results. |
| P2 | **One primary action per step.** | Reduce, The One | Exactly one `variant="default"` (primary) button per panel state. |
| P3 | **Defaults are computed, explained and editable.** | Time, Learn, Trust | Every default shows its rationale ("Recommended: 10 variables with ρ < 0.7 in this area") and an Edit affordance. |
| P4 | **Progressive disclosure, not hidden science.** Guided → Details → Expert. Nothing that affects results is unreachable; nothing optional is in the way. | Reduce, Differences, Organize | Expert controls write to the same config object; Review step lists every non-default value, including Expert-only ones. |
| P5 | **Observations ≠ predictions.** | Differences, Trust | Occurrences are vector markers (shape-coded), predictions are rasters; legends have separate "Observed" and "Modelled" sections; wording "observed" vs "predicted" everywhere. |
| P6 | **Never communicate unsupported certainty.** Suitability is a relative score; areas carry their threshold rule and a range; metrics carry spread and k; no fabricated confidence %. | Trust, Failure | Lint/copy review: no "%" next to suitability, no "probability", no area without rule; MetricCard refuses a value without its spread when spread exists. |
| P7 | **Uncertainty is always one step away, extrapolation is always visible.** | Context, Failure, Trust | Uncertainty layer reachable in 1 click from results; extrapolation hatch on by default. |
| P8 | **Recognition over recall.** Pick from lists with previews; show species names with taxonomy and counts; show layer thumbnails and units. | Learn, Time | No free-text field without autocomplete where a controlled vocabulary exists. |
| P9 | **Preserve context.** Active project / species / territory / run visible in a context bar; URL encodes view state; back button never loses work. | Context, Organize | Reload restores identical view (layer, threshold, zoom). |
| P10 | **Reversible by default.** Runs are immutable; edits create new runs; deletes are soft; cancel keeps partial logs. | Trust, Emotion | Every destructive action has undo (toast, 10 s) or confirmation naming the object. |
| P11 | **Long work is visible and non-blocking.** Stage, progress, elapsed, ETA range, warnings, cancel; the user can leave and come back. | Time, Trust | Any operation > 1 s shows progress; > 10 s runs in background with a persistent status entry. |
| P12 | **Errors speak ecology and operations, not stack traces.** Say what happened, why it matters for the model, what to do. | Failure, Learn | Every error has: title in domain terms, consequence, ≥ 1 action, technical detail collapsed. |
| P13 | **Direct manipulation on the map.** Draw/edit study areas, click to inspect values, drag threshold and see the map update — with keyboard equivalents. | Emotion, Time | Every map interaction has a non-drag alternative (WCAG 2.5.7). |
| P14 | **Same structure, different scale.** Single species and batch share the config panel and result components; batch adds aggregation, not a new vocabulary. | Organize, Learn | Batch config panel = single-run config panel component with "applies to N species". |
| P15 | **Calm emotion.** Satisfaction comes from clarity and responsiveness, not decoration. Success is stated plainly with what to check next. | Emotion | No confetti, no celebratory copy; completion message lists warnings first. |

## 3. Vocabulary (binding)

| Use | Avoid | Why |
|---|---|---|
| Suitability score (relative, 0–1) | probability, chance, likelihood, % | cloglog/logistic outputs are commonly misread as probability [21] |
| Predicted suitable area (rule: …) | habitat area, range, distribution | area depends on a chosen threshold |
| Observations / occurrences | data points, presences (alone) | distinguishes observed from modelled |
| Validation (spatial blocks, k = 5) | accuracy | AUC on presence-background is not accuracy |
| Extrapolation / outside training conditions | "uncertain area" | specific cause |
| Study (étude, estudio): species + study area + settings, with its progress and results | run, model run, job, travail, ejecución (in UI copy; "job" ok in API and internals) | domain object users recognise |
| Study area | AOI, extent | French practice "zone d'étude" |

## 4. Anti-patterns (reject in review)

- Rainbow/jet ramps; red–green state pairs without icons.
- KPI cards showing a single AUC or area without spread or rule.
- Modal dialogs over the map during spatial steps.
- "Advanced" toggles that change results silently.
- Spinners without stage text; progress bars that go backwards without explanation.
- Auto-stretched colour domains that make two species look equally suitable.
- Tooltips as the only place where a caveat lives.

## References

[1] J. Maeda, *The Laws of Simplicity*, https://lawsofsimplicity.com/ (accessed 2026-10-15).
[21] Yackulic et al. 2013, doi:10.1111/2041-210x.12004. Full list in
[09-ux-research](../research/09-ux-research.md#references).
