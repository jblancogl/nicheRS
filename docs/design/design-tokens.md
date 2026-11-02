# NicheRS design tokens

Status: draft v0.1 (2026-10-15). Owner: UX. Related: [design-principles](design-principles.md),
[component-architecture](component-architecture.md), [accessibility](accessibility.md),
research basis in [09-ux-research](../research/09-ux-research.md).

## 1. Conventions

- [FACT] shadcn/ui themes through semantic CSS variables on `:root` overridden in `.dark`, every
  surface token paired with `-foreground`, oklch values, a `@theme inline` block mapping
  `--color-*` to the variables, `@custom-variant dark (&:is(.dark *))`, and a `--radius` base from
  which `--radius-sm…4xl` are derived (sm = 0.6×, md = 0.8×, lg = 1×, xl = 1.4×) [2].
- [REC] NicheRS keeps **every shadcn token name unchanged** (so registry components work
  unmodified) and adds a namespaced domain layer: `--state-*`, `--map-*`, `--ramp-*`, `--obs-*`.
- [REC] Components consume only semantic tokens; raw values appear only in this file and in
  `src/styles/tokens.css`. Data ramps are **theme-independent**: a suitability of 0.8 has the same
  colour in light and dark mode (only map chrome, basemap and UI change).
- Hex values below are the sRGB conversions of the oklch values (computed with the reference
  OKLab matrices [26]; script kept outside the repo). Contrast ratios are WCAG 2.x relative
  luminance ratios computed from those hex values [23].

## 2. Colour tokens (UI)

Neutral hue 265 (cool grey, chroma ≤ 0.006) keeps the chrome quiet so the raster carries the
colour. A single accent (blue, hue 250) is used for primary action, selection and focus only.
Blue was chosen because it sits at the low-saturation middle of viridis (≈ hue 228–266 at
L 0.45–0.52), so accent UI never reads as a "high suitability" value.

| Token | Light | Dark | Use | Contrast check |
|---|---|---|---|---|
| `--background` | `oklch(1 0 0)` #ffffff | `oklch(0.16 0.005 265)` #0c0d0f | app canvas | — |
| `--foreground` | `oklch(0.20 0.005 265)` #151618 | `oklch(0.96 0.003 265)` #f1f2f4 | body text | 18.1:1 / 17.4:1 |
| `--card` / `--popover` | `oklch(1 0 0)` | `oklch(0.19 0.005 265)` #131416 | panels, popovers | — |
| `--card-foreground` / `--popover-foreground` | = foreground | = foreground | | |
| `--muted` | `oklch(0.967 0.003 265)` #f3f4f6 | `oklch(0.23 0.006 265)` #1b1d20 | secondary surfaces, table stripes | — |
| `--muted-foreground` | `oklch(0.50 0.01 265)` #606369 | `oklch(0.72 0.01 265)` #a1a5ab | captions, units, metadata | 6.0:1 on bg, 5.5:1 on muted / 7.9:1, 6.8:1 |
| `--primary` | `oklch(0.47 0.11 250)` #215d96 | `oklch(0.72 0.11 250)` #6da9e7 | the one primary action, selection | 6.8:1 / 7.9:1 |
| `--primary-foreground` | `oklch(1 0 0)` | `oklch(0.16 0.005 265)` | text on primary | 6.8:1 / 7.9:1 |
| `--secondary` | `oklch(0.967 0.003 265)` | `oklch(0.23 0.006 265)` | secondary buttons | — |
| `--secondary-foreground` | = foreground | = foreground | | |
| `--accent` | `oklch(0.95 0.015 250)` | `oklch(0.26 0.03 250)` | hover/selected row (shadcn "accent" = hover surface) | — |
| `--accent-foreground` | = foreground | = foreground | | |
| `--destructive` | `oklch(0.53 0.19 27)` #c22826 | `oklch(0.70 0.17 25)` #f66d67 | delete/cancel run, errors | 5.8:1 / 6.8:1 |
| `--border` | `oklch(0.915 0.004 265)` #e1e3e6 | `oklch(0.30 0.006 265)` #2c2e31 | decorative dividers only | 1.3:1 (decorative, exempt) |
| `--input` | `oklch(0.64 0.006 265)` ≈#8a8c90 | `oklch(0.52 0.008 265)` #67696e | input/checkbox/slider-track boundary | ≥3.1:1 / 3.5:1 (SC 1.4.11) |
| `--ring` | `oklch(0.55 0.12 250)` #3275b4 | `oklch(0.68 0.12 250)` #5b9ddf | focus ring | 4.8:1 / 6.8:1 |
| `--chart-1…5` | see §6 categorical | same | non-raster charts | — |
| `--sidebar*` | = card / foreground / primary / accent / border / ring | dark equivalents | contextual side panel | — |

[REC] `--input` deliberately diverges from shadcn's default (which reuses the light `--border`
value); a text field whose only boundary is a 1.3:1 border fails WCAG 1.4.11 [23].

### 2.1 State tokens (domain layer)

State colour is never the sole cue: each state also has an icon (Lucide) and a text label.

| Token | Light | Dark | Lucide icon | Meaning |
|---|---|---|---|---|
| `--state-success` | `oklch(0.50 0.12 150)` #21763c (5.6:1) | `oklch(0.74 0.13 150)` #69c27e | `circle-check` | run completed, check passed |
| `--state-warning` | `oklch(0.55 0.12 70)` #9d6300 (5.0:1) | `oklch(0.80 0.13 80)` #e9b452 | `triangle-alert` | completed with caveats, low n, extrapolation |
| `--state-danger` | alias of `--destructive` | alias | `circle-x` | failed, blocking error |
| `--state-info` | `oklch(0.50 0.11 250)` #2a669f | `oklch(0.72 0.11 250)` | `info` | neutral notice |
| `--state-running` | alias of `--primary` | alias | `loader-circle` (static if reduced motion) | queued/running |
| `--state-neutral` | alias of `--muted-foreground` | alias | `circle-dashed` | draft, not run, cancelled |
| `--state-*-subtle` | same hue, `L 0.96 C 0.03` | same hue, `L 0.25 C 0.04` | — | alert/badge backgrounds |

### 2.2 Map chrome and observation tokens

| Token | Value (both themes unless noted) | Use |
|---|---|---|
| `--map-study-area` | light `oklch(0.20 0.005 265)`, dark `oklch(0.96 0 0)`; 1.5 px line + 1 px halo of opposite colour | study area boundary |
| `--map-mask` | `oklch(0.5 0 0 / 0.35)` | outside study area dim |
| `--map-nodata` | transparent; pattern `--map-nodata-pattern` = 45° hatch 1 px @ 6 px, `--muted-foreground` 40 % | no-data cells inside area |
| `--map-extrapolation` | cross-hatch 1 px @ 5 px, black 55 % + white 55 % dual stroke | MESS < 0 / outside training range |
| `--obs-presence` | fill `#ffffff`, stroke `#151618` 1.5 px, circle r = 4 px | occurrence used for training |
| `--obs-test` | same colours, triangle | held-out validation occurrence |
| `--obs-excluded` | stroke only, `#151618` dashed, × glyph | filtered occurrence (dup, thinned, outside area, uncertain coordinates) |
| `--obs-selected` | stroke `--ring` 2.5 px | selection |

[FACT] A white-fill/black-stroke marker reaches ≥ 3:1 against at least one of its two colours on
every viridis stop (white vs `#440154` 15.2:1; black vs `#fde725` 14.3:1; at `#21918c` 3.8:1 white,
4.7:1 black) — computed. Shape, not colour, separates training / test / excluded points.

## 3. Raster colour ramps

Basis: Crameri et al. 2020 — avoid rainbow and red–green maps; use perceptually uniform,
CVD-readable maps [16]. cividis is optimised for CVD with linearly increasing lightness [17].
Stops are matplotlib 3.10.8 samples at i/8 [18]; MapLibre `raster-color` (or a pre-rendered
256-entry LUT) interpolates linearly between them [REC: generate the 256-entry LUT from
matplotlib/colorcet at build time and treat these 9 stops as the documentation reference].

### 3.1 `--ramp-suitability` (default: viridis)

| Value | Hex | oklch |
|---|---|---|
| 0.000 | #440154 | oklch(0.285 0.137 318.1) |
| 0.125 | #472d7b | oklch(0.373 0.127 294.9) |
| 0.250 | #3b528b | oklch(0.448 0.098 266.0) |
| 0.375 | #2c728e | oklch(0.520 0.082 228.0) |
| 0.500 | #21918c | oklch(0.596 0.095 190.3) |
| 0.625 | #28ae80 | oklch(0.670 0.131 164.5) |
| 0.750 | #5ec962 | oklch(0.749 0.173 144.3) |
| 0.875 | #addc30 | oklch(0.832 0.193 124.5) |
| 1.000 | #fde725 | oklch(0.918 0.186 102.1) |

Lightness rises monotonically 0.285 → 0.918 (computed), so the map stays ordered in greyscale
print. High suitability = light/yellow.

### 3.2 `--ramp-suitability-cvd` (user option: cividis)

| Value | Hex | oklch |
|---|---|---|
| 0.000 | #00224e | oklch(0.259 0.090 256.5) |
| 0.125 | #1a386f | oklch(0.351 0.102 261.3) |
| 0.250 | #434e6c | oklch(0.427 0.052 268.6) |
| 0.375 | #61656f | oklch(0.507 0.017 268.3) |
| 0.500 | #7d7c78 | oklch(0.586 0.006 95.2) |
| 0.625 | #9b9476 | oklch(0.665 0.043 96.4) |
| 0.750 | #bcae6c | oklch(0.748 0.088 97.7) |
| 0.875 | #dec958 | oklch(0.832 0.134 98.8) |
| 1.000 | #fee838 | oklch(0.921 0.179 101.7) |

### 3.3 `--ramp-uncertainty` (separate quantity, separate ramp)

Monotonic lightness, near-zero chroma (warm grey, hue 60), so it cannot be confused with the
hue-rich suitability ramp. Convention: **darker = more uncertain** ("fog thickens").

| Value (normalised, see §3.5) | Hex | oklch |
|---|---|---|
| 0.000 (lowest) | #fff2e8 | oklch(0.97 0.020 60) |
| 0.125 | #e2d5cb | oklch(0.88 0.020 60) |
| 0.250 | #c5b8ae | oklch(0.79 0.020 60) |
| 0.375 | #a89c92 | oklch(0.70 0.020 60) |
| 0.500 | #8d8177 | oklch(0.61 0.020 60) |
| 0.625 | #72665e | oklch(0.52 0.020 60) |
| 0.750 | #584d45 | oklch(0.43 0.020 60) |
| 0.875 | #40362e | oklch(0.34 0.020 60) |
| 1.000 (highest) | #291f18 | oklch(0.25 0.020 60) |

[OPEN] Validate with users whether "darker = more uncertain" or "lighter/washed = more
uncertain" is read more intuitively; MacEachren et al. found fuzziness, lightness and
transparency among the more intuitive variables for uncertainty [11]. Keep the convention
global once chosen.

### 3.4 `--ramp-binary` (thresholded map)

| Class | Fill | Pattern | Legend label |
|---|---|---|---|
| Above threshold | #21918c at 85 % opacity | none | "Predicted suitable (≥ t, rule)" |
| Below threshold | transparent | none | "Below threshold" |
| Above threshold but extrapolated | #21918c 85 % | `--map-extrapolation` | "Suitable, but extrapolated" |

### 3.5 Combined suitability × uncertainty (optional view)

[REC] Phase 2 only: a Value-Suppressing Uncertainty Palette (VSUP) [13] via `vsup` (npm 1.1.0,
MIT) [14], 4 uncertainty levels × up to 8/4/2/1 value bins (tree quantisation). [FACT] The `pals`
R documentation strongly discourages a viridis-based VSUP because viridis' lightness change on
the value axis collides with the suppression axis [15]; the VSUP base ramp must therefore be
chosen and tested separately ([OPEN]). Default results use **side-by-side or toggle** maps plus
the extrapolation hatch overlay.

### 3.6 Ramp rules

1. Fixed domain `[0, 1]` for suitability; never stretch to min–max of the current raster (would
   make two species visually comparable when their values are not). A "stretch to data" option
   exists in Expert mode and flags the legend `Stretched: 0.03–0.71`.
2. Uncertainty domain is metric-specific (e.g. replicate SD `[0, 0.5]`) and shown in the legend
   with units; normalisation is never hidden.
3. No rainbow/jet/spectral/red–green ramps anywhere [16].
4. Raster opacity default 80 %; legend swatches render at the same opacity over the current
   basemap tone so legend and map match.

## 4. Typography

| Token | Value |
|---|---|
| `--font-sans` | `"Inter Variable", "Inter", ui-sans-serif, system-ui, sans-serif` [ASSUMPTION: self-hosted, OFL licence, verify before bundling] |
| `--font-mono` | `"JetBrains Mono", ui-monospace, SFMono-Regular, monospace` (IDs, hashes, parameter keys, logs) |
| Numerals | `font-variant-numeric: tabular-nums` (Tailwind `tabular-nums`) on all metric values, tables, legends, progress counters, coordinates; `slashed-zero` in mono |
| Feature flags | `font-feature-settings: "cv11", "ss01"` optional (Inter single-storey a); off by default |

Scale (rem at 16 px root; line-height in brackets):

| Token | Size | LH | Weight | Use |
|---|---|---|---|---|
| `--text-xs` | 0.75 | 1rem | 400/500 | units, axis ticks, badges |
| `--text-sm` | 0.875 | 1.25rem | 400/500 | **default UI text** (panels, tables, forms) |
| `--text-base` | 1 | 1.5rem | 400 | long-form help, reports |
| `--text-lg` | 1.125 | 1.75rem | 600 | panel titles |
| `--text-xl` | 1.25 | 1.75rem | 600 | page/step titles |
| `--text-2xl` | 1.5 | 2rem | 600 | MetricCard primary value |
| `--text-3xl` | 1.875 | 2.25rem | 600 | headline area estimate (one per results view) |

Tailwind v4 default type scale matches these sizes [UNVERIFIED – prior knowledge]; NicheRS uses
the defaults rather than custom steps. Minimum text size 12 px; nothing below `text-xs`.
Scientific names: italic, never bold-italic; authority in roman `--muted-foreground`.

## 5. Spacing, radii, elevation, motion, focus

| Token | Value | Notes |
|---|---|---|
| Spacing base | 4 px (Tailwind `--spacing: 0.25rem`) | dense instrument layout; panel padding 16 px, control gap 8 px, section gap 24 px |
| `--radius` | `0.375rem` (6 px) | smaller than shadcn default 0.625rem: precise, less "soft"; derived sm 3.6 px, md 4.8 px, lg 6 px, xl 8.4 px |
| Control height | 32 px (sm), 36 px (default) | ≥ 24 px target (SC 2.5.8) with spacing |
| `--shadow-xs` | `0 1px 2px oklch(0 0 0 / 0.06)` | inputs |
| `--shadow-sm` | `0 1px 3px oklch(0 0 0 / 0.10), 0 1px 2px oklch(0 0 0 / 0.06)` | floating map controls |
| `--shadow-md` | `0 4px 12px oklch(0 0 0 / 0.12)` | popovers, dialogs; dark: borders carry elevation, shadows at 2× alpha |
| `--duration-fast` | 120 ms | hover, toggles |
| `--duration-base` | 200 ms | panel open/close, sheet |
| `--ease-standard` | `cubic-bezier(0.2, 0, 0, 1)` | |
| Reduced motion | all durations → 0 ms; map `flyTo` → `jumpTo`; spinners → static icon + text | `prefers-reduced-motion: reduce` |
| Focus ring | `outline: 2px solid var(--ring); outline-offset: 2px` (Tailwind `focus-visible:ring-2 ring-ring ring-offset-2 ring-offset-background`) | ≥ 3:1 against adjacent colours; on the map canvas use a 2 px ring + 2 px `--background` halo |
| z-layers | map 0, map controls 10, side panel 20, sticky headers 30, popover 40, dialog 50, toast 60 | |

## 6. Categorical palette (`--chart-1…5`, non-raster)

For multi-species comparison charts and batch status charts only. Okabe–Ito subset
[UNVERIFIED – prior knowledge: CVD-safe categorical set]: `#0072b2`, `#e69f00`, `#009e73`,
`#cc79a7`, `#56b4e9`. Max 5 series; beyond 5, use small multiples, not more hues. Always paired
with direct labels or marker shapes.

## 7. CSS (shadcn/Tailwind v4 compatible excerpt)

```css
@import "tailwindcss";
@custom-variant dark (&:is(.dark *));

:root {
  --radius: 0.375rem;
  --background: oklch(1 0 0);
  --foreground: oklch(0.20 0.005 265);
  --card: oklch(1 0 0);            --card-foreground: oklch(0.20 0.005 265);
  --popover: oklch(1 0 0);         --popover-foreground: oklch(0.20 0.005 265);
  --primary: oklch(0.47 0.11 250); --primary-foreground: oklch(1 0 0);
  --secondary: oklch(0.967 0.003 265); --secondary-foreground: oklch(0.20 0.005 265);
  --muted: oklch(0.967 0.003 265); --muted-foreground: oklch(0.50 0.01 265);
  --accent: oklch(0.95 0.015 250); --accent-foreground: oklch(0.20 0.005 265);
  --destructive: oklch(0.53 0.19 27);
  --border: oklch(0.915 0.004 265);
  --input: oklch(0.64 0.006 265);
  --ring: oklch(0.55 0.12 250);
  --state-success: oklch(0.50 0.12 150);
  --state-warning: oklch(0.55 0.12 70);
  --state-info: oklch(0.50 0.11 250);
  --state-danger: var(--destructive);
  --state-running: var(--primary);
  --state-neutral: var(--muted-foreground);
  /* data ramps: theme independent, documented in §3; LUTs generated at build time */
}
.dark {
  --background: oklch(0.16 0.005 265);
  --foreground: oklch(0.96 0.003 265);
  --card: oklch(0.19 0.005 265);   --card-foreground: oklch(0.96 0.003 265);
  --popover: oklch(0.19 0.005 265);--popover-foreground: oklch(0.96 0.003 265);
  --primary: oklch(0.72 0.11 250); --primary-foreground: oklch(0.16 0.005 265);
  --secondary: oklch(0.23 0.006 265); --secondary-foreground: oklch(0.96 0.003 265);
  --muted: oklch(0.23 0.006 265);  --muted-foreground: oklch(0.72 0.01 265);
  --accent: oklch(0.26 0.03 250);  --accent-foreground: oklch(0.96 0.003 265);
  --destructive: oklch(0.70 0.17 25);
  --border: oklch(0.30 0.006 265);
  --input: oklch(0.52 0.008 265);
  --ring: oklch(0.68 0.12 250);
  --state-success: oklch(0.74 0.13 150);
  --state-warning: oklch(0.80 0.13 80);
  --state-info: oklch(0.72 0.11 250);
}
@theme inline {
  --color-background: var(--background);  --color-foreground: var(--foreground);
  --color-primary: var(--primary);        --color-primary-foreground: var(--primary-foreground);
  /* …all shadcn pairs… */
  --color-state-success: var(--state-success);
  --color-state-warning: var(--state-warning);
  --color-state-danger: var(--state-danger);
  --color-state-info: var(--state-info);
  --radius-sm: calc(var(--radius) * 0.6);
  --radius-md: calc(var(--radius) * 0.8);
  --radius-lg: var(--radius);
  --radius-xl: calc(var(--radius) * 1.4);
}
```

## 8. Numeric formatting rules

All numbers through one `formatScientific()` module wrapping `Intl.NumberFormat(locale)`; locale
default `fr-FR` (narrow no-break space thousands separator, comma decimal), `en-GB` available.
Units are always shown, separated by a no-break space.

| Quantity | Format | Example (fr-FR) | Never |
|---|---|---|---|
| Suitability score | 2 decimals, range 0–1, label "score" | `0,73` | `73 %`, "probability 0.73" |
| Threshold value | 2 decimals + rule name | `0,31 (10e centile présence)` | bare number |
| CBI, AUC | 2 decimals; mean ± SD across folds + k; AUC always suffixed "(présence-fond)" | `CBI 0,71 ± 0,09 (k = 5)` | 4+ decimals, single value without k, AUC compared across species |
| Omission rate | percentage, 0 decimals, + expected | `Omission 12 % (attendu 10 %)` | |
| Area | km², 3 significant figures; < 1 km² → ha; always with threshold rule; show range across sensitivity thresholds | `1 240 km² (870–1 610 selon le seuil)` | `1 239,57 km²` (false precision) |
| Proportion of study area | % 0 decimals (1 decimal if < 1 %) | `18 % de la zone d'étude` | |
| Counts | integer, thousands sep. | `1 204 occurrences` | |
| Coordinates | lon/lat 5 decimals (≈ 1 m) or projected metres 0 decimals, with CRS | `2,35222 E, 48,85661 N (WGS 84)` | |
| Resolution | metres or arc-seconds with CRS | `1 km (EPSG:3035)` | |
| Durations | `h min s`, ETA as range | `≈ 4–7 min restantes` | precise ETA countdown |
| Dates | ISO 8601 in metadata, locale long form in UI | `2026-10-15T14:02Z` / `15 oct. 2026, 16:02` | |
| Missing | em dash `—` with tooltip reason | `—` ("non calculé : n < 10") | `0`, `NaN`, empty |

Significant-figure rule: never display more precision than the quantity's uncertainty supports;
if SD is known, round the mean to the SD's first significant decimal.

## References

See [09-ux-research.md §References](../research/09-ux-research.md#references); ids [2] [11] [13]
[14] [15] [16] [17] [18] [23] [26] used here.
