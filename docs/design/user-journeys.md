# Personas and user journeys

Date: 2026-10-15. Personas are [ASSUMPTION]-based proto-personas built from the brief (ecology
professionals at French/European consultancies, not data scientists) and comparable-tool
research [09](../research/09-ux-research.md); validate with ≥ 5 interviews at Natural Solutions
and clients ([OPEN]).

## 1. Personas

### Claire — consultant ecologist (primary)
- Context: impact studies (wind farms, infrastructure), Natura 2000 assessments. Writes reports
  for clients and authorities. Uses QGIS weekly, no R.
- Goal: a defensible suitability map + suitable-area estimate + stated limitations for 1–5
  species, within a day.
- Pain: SDM tools assume statistics fluency; unclear which defaults are safe; fear of being
  challenged on method.
- Needs: recommended defaults with rationale, plain-language caveats, report-ready exports with
  method text, French UI.
- Success: report section produced without asking a specialist; reviewer accepts it.

### Thomas — SDM specialist (expert)
- Context: biostatistician; uses ENMeval/maxnet; reviews colleagues' models.
- Goal: tune features, regularisation, background and bias correction; compare runs; reproduce.
- Pain: black-box GUIs, unexportable configs, inability to see what defaults were used.
- Needs: Expert mode, config diff, provenance, JSON/script export, response curves.

### Sophie — project lead / reviewer
- Context: signs off deliverables; low SDM knowledge; reads on laptop/tablet.
- Goal: understand quickly whether results are reliable enough; compare alternatives; export.
- Needs: summary with explicit reliability statement, warnings first, shareable links.

### Malik — GIS / data lead (batch)
- Context: regional biodiversity atlas, 50–500 species, one territory.
- Goal: run all species with one config; track progress; recover failures; export all.
- Needs: CSV import with name resolution, pre-flight filtering, status grid, retry-failed,
  comparison table, batch export with manifest.

## 2. Journeys

Notation: step · screen · user action · system response · emotional risk → design answer.

### J0 — Quick study, no modelling knowledge (any visitor; added 2026-10-19)

Territorial actors (local authorities, natural-area managers) who want to know where a species
could find suitable conditions, without choosing data filters, variables or model settings.

| # | Screen | Action | System | Risk → answer |
|---|---|---|---|---|
| 1 | New study | Chooses **Quick** (remembered for the next studies) | 3-step path; switch to Expert always visible | Fear of a "lighter" analysis → both paths run the same validated model |
| 2 | Species | Types a common or scientific name, picks one | One species; choosing another replaces it | Wants several → pointed to Expert |
| 3 | Study area | Picks a municipality, department, region or protected area | Area km²; finest resolution chosen automatically | Area too large → explicit message |
| 4 | Check & launch | Reads "N usable locations from M GBIF records" and every data warning; launches | Recommended settings stated; "relative suitability, not probability of presence" | Warnings hidden for simplicity → never: same warnings and acknowledgement as Expert |

### J1 — First single-species model (Claire)

| # | Screen | Action | System | Risk → answer |
|---|---|---|---|---|
| 1 | Project home | "New study" | Opens Setup · Species with map | — |
| 2 | S1 Species | Types "Lacerta bil…", picks *Lacerta bilineata* | Shows accepted name, 1 204 occurrences (source, date), points on map | Wrong taxon → synonym/ambiguity states |
| 3 | S2 Study area | Accepts suggested area or picks a département | Area km², in/out occurrence counts, excluded points dashed | Silent exclusions → explicit counts |
| 4 | S3 Environment | Accepts recommended 8 variables | Rationale: collinearity-filtered, resolution, sources | Doesn't know bioclim codes → labels + units |
| 5 | S4 Model | Keeps "Standard" preset | Summary sentence of what the model does | Fear of wrong settings → preset explanation, defaults cited |
| 6 | S5 Validation | Keeps spatial blocks k = 5 | Folds preview on map | — |
| 7 | S6 Review | Reads 1 warning ("sampling concentrated near roads"), acknowledges, launches | Pre-flight list; ETA range | Warning ignored → acknowledgement |
| 8 | S7 Execution | Pans map, opens another study | Background run; status in context bar; toast on completion | Anxiety → stage + heartbeat |
| 9 | S8 Results | Reads cards; moves threshold; toggles uncertainty | Area updates with range; legend explains score | Over-reads area → rule + range always |
| 10 | Export | Chooses "Report bundle" | PNG map with legend, CSV metrics, method text (fr), GeoTIFF, config JSON | Missing caveats in export → caveats embedded |

### J2 — Expert tuning and comparison (Thomas)

1. Open Claire's Run #1 → "Duplicate & edit" → Setup opens on Review step with config copied.
2. Expert tab: feature classes LQH, β multiplier 2, bias file = target-group background.
   "Modified (3)" badge.
3. Launch → Run #2. While running, opens Compare (Run #1 vs #2 placeholder).
4. Compare: synced maps, same legend domain; config diff table (3 rows); metrics table with
   mean ± SD (CBI, omission; AUC presence-background is valid here because species/extent/background
   are identical); no automatic winner.
5. Marks Run #2 as "Preferred for report" (explicit user action, with note) → Export config JSON.

### J3 — Review and sign-off (Sophie)

1. Opens shared link `…/runs/r2?tab=summary` on laptop → identical view.
2. Reads warnings banner first, then Reliability card ("Moderate: 2 caveats").
3. Opens Validation tab: understands spatial CV in one sentence.
4. Comments (phase 2 [OPEN]) or exports PDF summary.
5. On tablet/narrow: map + bottom sheet; same caveats.

### J4 — Batch atlas (Malik)

| # | Screen | Action | System |
|---|---|---|---|
| 1 | Batch new · Species | Pastes/uploads CSV of 312 names | Resolution table: 298 accepted, 9 synonyms (auto-mapped, reviewable), 5 unresolved (must fix or drop) |
| 2 | Batch new · Area | Picks region | Per-species in-area occurrence counts computed |
| 3 | Batch new · Environment & model | Reuses saved config template "Atlas standard" | Same config panel as single study, "applies to 307 species" |
| 4 | Batch new · Pre-flight | Reviews | 41 species < 10 occurrences → default "Exclude (keep listed)"; alternative "Run anyway, flag" |
| 5 | Launch | — | Batch grid; concurrency and queue position shown |
| 6 | Monitor | Filters "Failed" | 6 failed: aggregated causes ("4 × raster fetch timeout — retryable", "2 × no occurrences after thinning — not retryable") |
| 7 | Recover | "Retry failed (4 retryable)" | Only those rows re-queued; config unchanged; attempts counted |
| 8 | Compare | Comparison table: species, n, CBI ± SD, omission vs expected, suitable area (range), reliability level, warnings (no AUC across species) | Sort/filter; select rows → map small multiples (max 4) |
| 9 | Export | Batch export | Zip/COG folder per species + manifest CSV + per-species status + global methods text; partial batches labelled |

### J5 — Failure recovery (any persona)

Run fails at "Prepare rasters" (source unavailable) → Results route shows failure panel:
cause in operational terms ("Le service WorldClim n'a pas répondu"), consequence ("no model was
trained; nothing was lost"), actions ("Retry", "Use cached layers from 2026-09-30", "Change
variables"), technical detail collapsed (request id, log excerpt). Config and logs retained.

## 3. Journey-level acceptance (summary)

- J1 completed by a Claire-profile tester in ≤ 15 min of active time, without help, with correct
  explanation of what the suitability score is (usability test, [OPEN] schedule).
- J4 retry affects only failed rows; batch export works with partial completion.
- J3: shared link reproduces view state exactly.
