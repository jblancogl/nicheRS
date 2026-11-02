# Low-fidelity wireframes (ASCII)

Date: 2026-10-15. Desktop reference 1440 × 900. Screen ids match the
[Laws of Simplicity audit](laws-of-simplicity-audit.md). `[■ Primary]` = the single primary
action; `[ Secondary ]`; `▸` collapsed disclosure; `●▲×` = observation markers (training, test,
excluded); `░` = raster; `▨` = extrapolation hatch.

## W0 Workspace shell

```
┌──┬──────────────────────────────────────────────────────────────────────────────────┐
│≡ │ Causses 2026 › Lacerta bilineata › Run #2  [Occitanie]  ✓ Completed   [Run ▾] ⌘K │ context bar 44px
├──┼───────────────────────────────────────────────────────┬──────────────────────────┤
│▣ │                                                       │ SIDE PANEL (400–480px,   │
│Pr│                    MAP CANVAS                         │ resizable)               │
│◫ │                                                       │                          │
│Ba│   [+][-][⌖]                                [Layers ▾] │ mode: Setup | Execution  │
│⛁ │                                                       │       | Results | Compare│
│Da│                                                       │                          │
│⚙ │                                                       │                          │
│  │  scale 10 km   EPSG:3857 view · data EPSG:3035         │                          │
│? │  ┌Legend──────────────┐                                │──────────────────────────│
│  │  └────────────────────┘                                │ footer: one primary      │
└──┴───────────────────────────────────────────────────────┴──────────────────────────┘
 rail 56px
```

## W1 Setup · Species (S1)

```
│ MAP: occurrence points ● appear as soon as a species is chosen     │ New study           │
│                                                                    │ ①Species ②Area ③Env │
│        ●  ●●                                                       │ ④Model ⑤Valid. ⑥Rev │
│      ●●●  ●                                                        │─────────────────────│
│         ●                                                          │ Species             │
│                                                                    │ [Lacerta bil…     ▾]│
│                                                                    │ ┌─────────────────┐ │
│                                                                    │ │Lacerta bilineata│ │
│                                                                    │ │ Daudin, 1802    │ │
│                                                                    │ │ accepted · 1 204│ │
│                                                                    │ │ occ. (GBIF, …)  │ │
│                                                                    │ └─────────────────┘ │
│                                                                    │ ▸ Occurrence filters│
│                                                                    │   (years, coord.    │
│                                                                    │   uncertainty, basis│
│                                                                    │ Adding ≥2 species?  │
│                                                                    │ [ Create a batch ]  │
│                                                                    │─────────────────────│
│                                                                    │      [■ Continue]   │
```

## W2 Setup · Study area (S2)

```
│  ┌───────── suggested area (hull + 20 km) ─────────┐               │ ②Study area         │
│  │░░░░ ● ●●   ●                                    │   ×  ×        │ Method              │
│  │░░ ●●●  ●         ●                              │  (outside)    │ (•Suggested)( Admin)│
│  │    ●                                            │               │ ( Draw )( Upload )  │
│  └──────────────────────────────────────────────────┘              │─────────────────────│
│  [✎ Edit vertices] [↶ Undo] [↷ Redo]                               │ Area   18 400 km²   │
│                                                                    │ Inside 1 150 occ.   │
│                                                                    │ Outside   54 occ. × │
│                                                                    │ Env. coverage 100 % │
│                                                                    │ ▸ Buffer, CRS (exp.)│
│                                                                    │─────────────────────│
│                                                                    │[Back][■ Use this    │
│                                                                    │        study area]  │
```

## W3 Setup · Environmental data (S3)

```
│ MAP: preview of hovered variable (greyscale ramp, legend w/ unit)  │ ③Environmental data │
│                                                                    │ Recommended: 8 vars │
│                                                                    │ collinearity ρ<0.7  │
│                                                                    │ ▾ Climate (5)       │
│                                                                    │  ☑ Mean ann. temp °C│
│                                                                    │    bio1 · CHELSA 1km│
│                                                                    │  ☑ Annual precip mm │
│                                                                    │  ☐ Temp. seasonality│
│                                                                    │    ⚠ ρ=0.86 w/ bio1 │
│                                                                    │ ▸ Topography (2)    │
│                                                                    │ ▸ Land cover (1)    │
│                                                                    │ ▸ Correlation matrix│
│                                                                    │ Resampling: bilinear│
│                                                                    │ to 1 km EPSG:3035   │
│                                                                    │─────────────────────│
│                                                                    │[Back][■ Use these 8]│
```

## W4 Setup · Model (S4) — Guided and Expert

```
Guided                                    Expert (same panel, tab)
┌─────────────────────────────┐           ┌─────────────────────────────┐
│④Model    [Guided|Expert]    │           │④Model  [Guided|Expert] Mod(3)│
│Algorithm  MaxEnt (maxnet)   │           │▾ Features      L Q H ☐P ☐T  │
│Preset                       │           │▾ Regularisation β [ 2.0 ] ↺  │
│ (•) Standard                │           │▸ Background  10 000 pts ↺    │
│     Balanced complexity…    │           │▸ Sampling bias  target-group │
│ ( ) Conservative            │           │▸ Validation (see step 5)     │
│     Simpler response curves │           │▸ Variables                   │
│ ▸ What will be computed     │           │▸ Extrapolation  clamp ☑ MESS☑│
│                             │           │▸ Output  cloglog · seed 42   │
│        [Back][■ Continue]   │           │ [Reset all]  [Back][■ Apply] │
└─────────────────────────────┘           └─────────────────────────────┘
```

## W5 Setup · Validation (S5) and Review (S6)

```
│ MAP: spatial blocks, fold 1–5 pattern+colour │ ⑤Validation             │ ⑥Review & launch     │
│ ┌──┬──┬──┬──┐                                │ (•)Spatial blocks k=5   │ Species  L. bilineata│
│ │1 │2▒│3 │4▒│                                │ ( )Random k-fold        │ Area 18 400 km²  Edit│
│ ├──┼──┼──┼──┤                                │ Block size auto 50 km   │ Env 8 vars       Edit│
│ │5▒│1 │2▒│3 │                                │ Why spatial? ▸          │ Model Standard   Edit│
│ └──┴──┴──┴──┘                                │ ⚠ fold 4: 7 test occ.   │ Valid. blocks k=5Edit│
│                                              │                         │ Pre-flight           │
│                                              │                         │ ✓ 1 150 occurrences  │
│                                              │                         │ ⚠ Records clustered  │
│                                              │                         │   near roads → bias  │
│                                              │                         │   ☐ I understand     │
│                                              │                         │ Est. 3–8 min         │
│                                              │ [Back][■ Continue]      │ [Back][■ Launch run] │
```

## W6 Execution (S7)

```
│ MAP stays interactive: area + observations                          │ Run #2 · Running    │
│                                                                    │ L. bilineata · 8 var│
│                                                                    │─────────────────────│
│                                                                    │ ✓ Fetch occurrences │
│                                                                    │ ✓ Prepare rasters   │
│                                                                    │ ✓ Sample background │
│                                                                    │ ◐ Train  fold 3 / 5 │
│                                                                    │ ○ Predict           │
│                                                                    │ ○ Evaluate          │
│                                                                    │ ○ Package outputs   │
│                                                                    │ [██████████░░░░] 58%│
│                                                                    │ Elapsed 2 min 10 s  │
│                                                                    │ ≈ 2–4 min remaining │
│                                                                    │ Updated 3 s ago     │
│                                                                    │ ⚠ 1 warning ▸       │
│                                                                    │ ▸ Resources & log   │
│                                                                    │─────────────────────│
│                                                                    │ [ Cancel run ] (red │
│                                                                    │   outline, confirm) │
```

## W7 Results (S8)

```
┌──────────────────────────────────────────────────────────────┬──────────────────────────┐
│ ⚠ Completed with 2 warnings  ▸ details                       │ [Summary|Valid.|Var.|Prov]│
│                                                              │ ┌ Predicted suitable area┐│
│   ░░░░▒▒▓▓██  ▨▨ (extrapolation hatch)                        │ │  3 150 km²             ││
│   ░░▒▒▓▓████  ●  ▲                                            │ │  2 400–3 900 km² across││
│   ░░░▒▒▓▓██                                                   │ │  thresholds · 17 % area││
│                                                              │ │  rule: 10th pct. pres. ││
│                         [Layers ▾]                           │ └────────────────────────┘│
│ ┌ Suitability score (relative, 0–1) ┐ ┌ Observed ┐            │ ┌ Validation ┐┌Reliability┐│
│ │█▓▒░ 0 ── 0.5 ── 1  ⓘ what is this?│ │● train   │            │ │CBI 0.71±0.09││Moderate   ││
│ │▨ outside training conditions      │ │▲ test    │            │ │OR10 12% /10%││2 caveats ▸││
│ └───────────────────────────────────┘ └──────────┘            │ │k=5 spatial  ││(rule-based)││
│  Layer: (•)Suitability ( )Uncertainty ( )Binary  [⇆ swipe]    │ └─────────────┘└───────────┘│
│                                                              │ Threshold                 │
│                                                              │ Rule [10th pct presence ▾]│
│                                                              │ 0 ──────●──────── 1  0.31 │
│                                                              │ ▸ Inspect point (click map)│
│                                                              │───────────────────────────│
│                                                              │[Duplicate & edit][■Export]│
└──────────────────────────────────────────────────────────────┴──────────────────────────┘
```

Click on map → popover: `Suitability 0.62 (score) · Uncertainty SD 0.08 · Inside training range ·
Nearest observation 1.2 km` + "Show in table".

Validation card: continuous Boyce index (CBI) and omission rate at the 10th-percentile threshold
(OR10, observed / expected) are primary; AUC (presence-background) appears only in the
Validation tab, labelled as such (per [07-scientific-validation §2.3](../research/07-scientific-validation.md)).

## W8 Batch (S9)

```
│ Atlas Occitanie 2026 · 307 species · Atlas standard config            [■ Retry 4 failed]│
│ ✓ 281 completed  ⚠ 16 with warnings  ✕ 6 failed  ◐ 4 running  ○ 0 queued  − 41 excluded│
│ Filter: [All][Failed][Warnings][Excluded]   Search [        ]          [Export ▾]      │
├──────────────────────────────┬───────┬────────────┬─────────┬──────────┬──────────────┤
│ Species                      │ n occ │ Status     │ CBI±SD  │ Area km² │ Stage        │
├──────────────────────────────┼───────┼────────────┼─────────┼──────────┼──────────────┤
│ Anthus campestris            │   412 │ ✓          │0.74±0.08│ 2 900    │ —            │
│ Bufo spinosus                │ 1 980 │ ⚠ 1        │0.61±0.11│ 9 400    │ —            │
│ Coronella girondica          │    61 │ ✕ timeout  │ —       │ —        │ Prep rasters │
│ Emys orbicularis             │   133 │ ◐ 3/5      │ —       │ —        │ Train        │
│ …(virtualised)               │       │            │         │          │              │
├──────────────────────────────┴───────┴────────────┴─────────┴──────────┴──────────────┤
│ Failure causes: 4 × raster fetch timeout (retryable) · 2 × no occurrences after        │
│ thinning (not retryable — change config or exclude)                                    │
```
No AUC column in batch: AUC is not comparable across species/extents (07 §2.3).
Row click → right side panel shows that species' Results summary (W7 panel) without leaving the grid.

## W9 Comparison (S11)

```
│ Run #1 (Standard)            │ Run #2 (Expert, β=2)          │ Config diff          │
│ ░▒▓██ map (synced)           │ ░▒▓██ map (synced)            │ β        1.0 → 2.0   │
│ legend 0–1 (shared)          │ legend 0–1 (shared)           │ features LQ → LQH    │
│                              │                               │ bias     none → TGB  │
│ Area 3 150 (2 400–3 900)     │ Area 2 700 (2 100–3 300)      │ Metrics              │
│ CBI 0.71±0.09 OR10 12%       │ CBI 0.78±0.06 OR10 9%         │ (no automatic winner)│
│                              │                               │ [■ Export comparison]│
```

## W10 Narrow / mobile (< 768 px, read-only review)

```
┌──────────────────────┐
│ ← L. bilineata · #2 ⚠│
├──────────────────────┤
│                      │
│   MAP (full width)   │
│   ░▒▓██  ●           │
│ [Layers] [Legend]    │
├══════════════════════┤ ← bottom sheet (drag handle + "Expand" button)
│ ⚠ 2 warnings ▸       │
│ Suitable area        │
│ 3 150 km² (2 400–3 900)│
│ rule 10th pct        │
│ CBI 0.71±0.09 (k=5)  │
│ Reliability Moderate │
│ [Summary table]      │
│ Setup & Expert mode  │
│ need a wider screen. │
└──────────────────────┘
```

## W11 Failure state (Results route of a failed run)

```
│ MAP: area + observations (still useful) │ ✕ Run #3 failed at "Prepare rasters"      │
│                                         │ The CHELSA service did not respond (3     │
│                                         │ attempts). No model was trained; your     │
│                                         │ configuration and logs are kept.          │
│                                         │ [■ Retry]  [ Use cached layers (30 Sep) ] │
│                                         │ [ Change variables ]                      │
│                                         │ ▸ Technical details (request id, log)     │
```
