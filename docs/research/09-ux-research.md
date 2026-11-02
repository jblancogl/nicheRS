# 09 — UX research: interface for an SDM platform used by ecology professionals

Date: 2026-10-15. Status: research input for the UX specification
([ux-specification](../architecture/ux-specification.md)) and the `docs/design/` set.
Tags: [FACT] verified from a source read on 2026-10-15; [ASSUMPTION]; [OPEN]; [REC];
[UNVERIFIED – prior knowledge].

## 1. Scope

How should NicheRS present species distribution modelling (SDM) to ecology professionals who are
not data scientists, so that (a) a defensible suitability map, area estimate and uncertainty
statement can be produced without statistical expertise, (b) experts can still reach every
parameter, (c) hundreds of species can be processed and compared, and (d) the interface never
overstates certainty. Covered: comparable tools (QGIS Processing, ArcGIS Pro geoprocessing,
Wallace, BCCVL/EcoCommons, web GIS dashboards, batch UIs), literature on uncertainty
visualisation and scientific colour, the mandated stack (shadcn/ui, MapLibre), personas,
journeys, and map-first vs workflow-first architecture. Not covered: backend design, algorithm
choice (see other research docs), visual identity beyond tokens.

## 2. Findings

### 2.1 Comparable tools

| Tool | Pattern observed | Lesson for NicheRS |
|---|---|---|
| **Wallace v2** (R Shiny; CRAN 2.2.1, May 2026) [6][7][27] | [FACT] Ten ordered components (Occ data → Env data → Process Occs → Process Envs → Env space → Partition → Model → Visualize → Transfer → Reproduce), each with alternative *modules*; left toolbar with module controls and "?" guidance; right visualisation space with map, occurrence table, results, log window, species dropdown; downloads an annotated R Markdown script reproducing the session. [FACT] Closing the browser loses the session unless saved; batch across species is a checkbox per module; later code depends on earlier steps. | Ordered, ecology-named steps work for SDM; guidance next to each step; reproducibility export is a first-class step. Avoid: session loss (NicheRS must persist every step server-side), per-module batch checkboxes (make batch a first-class object), deep component tabs that hide the map. |
| **EcoCommons / BCCVL Modelling Wizard** [8] | [FACT] Point-and-click wizard for single- and multi-species SDMs, traits, climate projections; experiments are typed (primary / secondary — outputs of primary feed secondary); BCCVL offered 17 algorithms across 4 categories; BCCVL shut down 8 Jul 2022 and was folded into EcoCommons. | Wizard + "experiment" object is a proven mental model; but 17 algorithms at step one is a Reduce violation for NicheRS' audience — start with MaxEnt, expose alternatives only in Expert mode. Chaining (model → projection) as typed objects is worth copying. |
| **ArcGIS Pro geoprocessing** [9] | [FACT] Tool pane with required parameters marked; per-run Environments tab; background "geoprocessing thread" lets the user keep working on the map; status banner with progress %, cancel; success / warning / error icon after the run with View Details (messages, run time, parameters); project-level History of every run; tools started while another runs are queued. | Copy: non-blocking execution with a persistent status surface, warning as a distinct terminal state ("completed with warnings"), run history including parameters (provenance). |
| **QGIS 3.44 Processing** [10] | [FACT] Algorithm dialog auto-generated from parameter definitions; Log tab with clickable outputs; per-run "Advanced" settings; copy configuration as Python / `qgis_process` / JSON; "Run as Batch Process" turns the same dialog into a parameter table, one row per run. | Copy: batch = same configuration in a table with per-row overrides; export configuration as code/JSON for reproducibility. Avoid: forms generated mechanically from parameter schemas (no grouping, no ecological wording). |
| Web GIS dashboards (e.g. ArcGIS Dashboards, Kepler-style explorers) | [UNVERIFIED – prior knowledge] Persistent full-bleed map, floating layer list, KPI cards in a side rail, linked charts. | Map as persistent canvas; metrics in a side rail; but dashboards favour headline numbers without uncertainty — NicheRS metric cards must carry spread and caveats. |
| Scientific exploration tools (Jupyter, Observable, napari) | [UNVERIFIED – prior knowledge] Linked views, explicit parameter provenance, everything reproducible from a script. | Every results view must link back to the exact run configuration and be exportable as a reproducible bundle. |
| Batch / CI interfaces (GitLab pipelines, Airflow grid) | [UNVERIFIED – prior knowledge] Grid of jobs × stages with status glyphs, retry-failed, filter by status, logs per job. | Batch view = species × stage status grid with "retry failed only" and status filter. |

### 2.2 Uncertainty communication literature

- [FACT] MacEachren et al. 2012 ran two experiments on how intuitive visual variables and iconic
  symbols are for spatial/temporal/attribute uncertainty and derived initial representation
  guidelines [11]. [UNVERIFIED – prior knowledge] fuzziness, location, value/lightness,
  arrangement and transparency rated most intuitive; hue least.
- [FACT] Kinkeldey, MacEachren & Schiewe 2014 reviewed user studies of geospatial uncertainty
  visualisation: few techniques are empirically evaluated and the user **task** determines what
  works; they propose task-centred typologies [12]. ⇒ [REC] Design uncertainty displays per task
  (identify where the model is unreliable; decide whether a site is suitable; report an area range).
- [FACT] Correll, Moritz & Heer 2018 (CHI): VSUPs allocate more colour resolution to values with
  low uncertainty and merge values with high uncertainty; in a crowdsourced study people weighted
  uncertainty more heavily in decisions than with conventional bivariate maps [13]. A JS
  implementation `vsup` exists (npm 1.1.0, MIT, D3-based) [14]. [FACT] `pals` discourages a
  viridis-based VSUP because viridis' lightness change collides with the suppression axis [15].
- [FACT] Crameri, Shephard & Heron 2020 (Nat. Commun. 11:5444): rainbow-like and red–green maps
  distort data and exclude CVD readers; scientific colour maps should be perceptually uniform and
  CVD-readable [16]. [FACT] cividis is optimised for CVD and has linearly increasing lightness [17].
- SDM-specific: [FACT] Phillips et al. 2017 present cloglog output as an estimate of occurrence
  probability and raw output as relative abundance [19]; the AMNH Maxent page states this holds
  "only under particular assumptions about the input data and biological sampling efforts" and the
  Maxent tutorial states Maxent is not directly calculating probability of occurrence [19][20].
  [FACT] Yackulic et al. 2013 found 54 % of 108 reviewed MAXENT studies interpreted logistic output
  incorrectly as occurrence probability and most likely suffered from sampling bias [21].
  ⇒ [REC] The UI calls the output a **suitability score (0–1, relative)**, never a probability or
  percentage; the exact transform is shown in provenance; a "what this number means" disclosure is
  attached to every legend.
- [FACT] Elith, Kearney & Phillips 2010 address extrapolation errors when transferring models
  (source of MESS) [22]. ⇒ [REC] extrapolation is shown as a colour-independent hatch overlay,
  on by default where present.

### 2.3 Stack findings

- [FACT] shadcn/ui theming: oklch semantic variables, `.dark` overrides, Tailwind v4 `@theme
  inline`, `--radius` derivation [2]. [FACT] Changelog: "Base UI as the Default" (Jul 2026), "React
  Aria" (Jul 2026), "shadcn/cli v4" (Mar 2026), "Toast" (Jul 2026), "Questionnaire" multi-step
  component (Aug 2026, Base UI / React Aria / Radix variants), standalone `cn` package (Sep 2026)
  [3]. Component list (66) includes Field, Empty, Item, Kbd, Spinner, Button Group, Input Group,
  Resizable, Sidebar, Sheet, Drawer, Data Table, Chart, Slider, Toggle Group, Command [4]. Field
  provides FieldSet/FieldLegend/FieldError, accepts react-hook-form errors and Standard Schema
  issues (Zod) and expects `aria-invalid` on controls [5].
- [FACT] MapLibre GL JS KeyboardHandler: arrows pan 100 px, `+`/`-` zoom (Shift doubles),
  Shift+arrows rotate 15° / pitch 10° [25]; its docs say nothing about ARIA [25]. [ASSUMPTION]
  the canvas is not screen-reader accessible ⇒ tabular/textual alternatives are mandatory.
- [FACT] WCAG 2.2 (Rec. 12 Dec 2024) adds at AA: 2.4.11 Focus Not Obscured (Min), 2.5.7 Dragging
  Movements, 2.5.8 Target Size (Min, 24×24 CSS px with spacing exception), 3.3.8 Accessible
  Authentication (Min); at A: 3.2.6 Consistent Help, 3.3.7 Redundant Entry [23][24]. 2.5.7 is
  directly relevant to drawing a study area and to the threshold slider.

### 2.4 Personas (summary; full in [user-journeys](../design/user-journeys.md))

| Persona | Role | SDM expertise | Primary need | Mode |
|---|---|---|---|---|
| **Claire** (primary) | Consultant ecologist, impact assessments | Knows what an SDM is; not R | Defensible map + area + caveats for a client report, fast | Guided |
| **Thomas** | Biostatistician / SDM specialist | Expert (ENMeval, maxnet) | Control of features, regularisation, background, partitions; reproducibility | Expert |
| **Sophie** | Project lead / reviewer | Low | Read, compare, approve, export; trust signals | Read-only |
| **Malik** | GIS/data lead on regional atlases | Medium | Run 50–500 species with one config, recover failures, batch export | Batch |

[ASSUMPTION] Users are French-speaking professionals; UI must ship fr (default) and en; terms
follow French ecological practice (e.g. "zone d'étude", "occurrences", "aire favorable").

### 2.5 Journeys (summary)

1. **First model** (Claire): pick species → draw/upload study area → accept recommended
   environmental set → keep default MaxEnt config → accept default spatial-block validation →
   launch → keep working → open results → set/confirm threshold → export report bundle.
2. **Expert tuning** (Thomas): duplicate run → Expert mode → change features/regularisation/
   background → launch → compare runs side by side → pick → export with provenance.
3. **Review** (Sophie): open shared result link → read summary, validation, caveats → compare two
   species → approve/comment → export PDF.
4. **Batch atlas** (Malik): import species list (CSV) → resolve names → one territory, one config
   → pre-flight check (flags species with too few records) → launch → monitor grid → retry failed
   → comparison table → batch export.

### 2.6 Map-first vs workflow-first

| Criterion (weight) | Map-first workspace | Workflow-first wizard | Hybrid: map canvas + guided panel |
|---|---|---|---|
| Spatial tasks (study area, occurrence review) (high) | ++ direct manipulation | − map buried in steps | ++ |
| First-time success for non-experts (high) | − blank canvas, recall | ++ one step at a time | ++ stepper in panel |
| Context preservation (species/territory always visible) (high) | ++ | − | ++ |
| Expert speed (med) | + | − linear | + steps are jumpable; Expert tab |
| Batch (med) | − single-object focus | + config reuse | + batch is a separate view sharing the config panel |
| Results interpretation (high) | ++ large map | − | ++ |
| Implementation cost (low) | med | low | med |

[REC] **Hybrid**: persistent MapLibre canvas as the workspace; a contextual right-hand side panel
that hosts the guided setup stepper (one primary action per step), execution status and results;
minimal left rail for global navigation. Batch and comparison are table-first views because their
primary object is a list, not a place. This follows Wallace's ordered steps [7] and ArcGIS Pro's
non-blocking run model [9] while avoiding Wallace's session fragility and BCCVL's algorithm-first
complexity [8].

## 3. Evidence and references

See §References; every [FACT] above cites the source read on 2026-10-15.

## 4. Alternatives

| Alternative | Why considered | Verdict |
|---|---|---|
| Full-screen modal wizard (BCCVL-like) | Simplest for novices | Rejected as primary: hides the map during spatial steps, breaks context. Kept for **batch creation** where the object is a list. |
| Notebook-style UI (cells = steps) | Reproducible, expert-friendly | Rejected for primary users; offer "export as script/config" instead. |
| Dashboard with KPIs first | Fast scanning | Rejected for results: encourages headline numbers without uncertainty. Used for batch summary only, with spread shown. |
| shadcn Questionnaire component for setup [3] | Native multi-step flow | [OPEN] evaluate; likely too form-centric (designed for question flows) for map-coupled steps; a custom stepper built on Tabs/Item is safer. |
| VSUP as default results display [13] | Proven to make users weigh uncertainty | Deferred to phase 2: needs a non-viridis base ramp and user testing [15]. |

## 5. Trade-offs

- Guided simplicity vs expert control → progressive disclosure: Guided panel shows ≤ 5 decisions
  per step; "Expert" tab exposes all parameters with the same underlying config object (no mode
  divergence).
- Honest uncertainty vs readability → suitability map stays clean; uncertainty is one click away
  (toggle/swipe) and summarised as text on the results panel; extrapolation hatch is always on.
- Fixed `[0,1]` ramp domain vs visual contrast → comparability wins; stretch only in Expert mode
  with a visible legend flag.
- Dense instrument look vs touch use → desktop-first (≥ 1280 px); narrow layouts are read-only
  review layouts.

## 6. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Users read suitability as probability / "% chance" | Wrong decisions in impact studies | Wording rules, legend disclosure, no % on scores, area always with threshold rule ([REC], [21]) |
| Single area figure quoted without threshold | Overstated certainty | Area always shown with rule + sensitivity range |
| Defaults treated as validated | Poor models shipped | Pre-flight checks (n occurrences, spatial bias, collinearity) with ecological wording; "completed with warnings" state |
| Map canvas inaccessible | Exclusion, non-conformance | Tabular summaries, keyboard map control, text alternatives ([accessibility](../design/accessibility.md)) |
| shadcn primitive library switch (Base UI default since Jul 2026) [3] | Inconsistent behaviour across copied components | Pin one primitive base in `components.json`; [OPEN] confirm Base UI variant maturity for Slider/Combobox |
| Batch of 500 overwhelms UI | Slow, unreadable | Virtualised table, status aggregation, server-side filtering |

## 7. Recommendations

1. [REC] Hybrid map-centred workspace + guided side-panel stepper (§2.6).
2. [REC] Six setup steps: Species → Study area → Environmental data → Model → Validation → Review
   & launch; each step has one primary action and a computed default.
3. [REC] Terminology: "suitability score (relative, 0–1)"; "observations/occurrences" vs
   "predictions"; "predicted suitable area (threshold rule)"; never "probability", "confidence %".
4. [REC] Results: large map with fixed-domain viridis, separate uncertainty ramp, extrapolation
   hatch, threshold control with live area range, validation panel with mean ± SD over folds,
   provenance drawer, export bundle (GeoTIFF + COG, PNG with legend, CSV metrics, JSON config,
   PDF summary).
5. [REC] Runs are immutable; edits create a new run ("Duplicate & edit"); all operations
   reversible except delete (which is soft for 30 days [ASSUMPTION]).
6. [REC] Batch as a first-class object with a species × stage grid, retry-failed, compare table.
7. [REC] Adopt the Laws of Simplicity audit ([laws-of-simplicity-audit](../design/laws-of-simplicity-audit.md))
   as a review gate for every screen.

## 8. Open questions

- [OPEN] Which uncertainty metric(s) are produced by the modelling engine (replicate SD, CV-fold
  range, MESS, bootstrap)? UI depends on it.
- [OPEN] Default threshold rule for consultancy reports (10th percentile training presence vs
  max TSS/SSS) — needs domain decision.
- [OPEN] Darker-is-more-uncertain vs washed-out-is-more-uncertain convention: test with 5 users.
- [OPEN] Is narrow/mobile in scope beyond read-only review?
- [OPEN] Regulatory report templates (French EIA) that exports must match.
- [OPEN] Base UI vs Radix variant of shadcn for this project.

## References

[1] J. Maeda, *The Laws of Simplicity*, https://lawsofsimplicity.com/ (accessed 2026-10-15).
[2] shadcn/ui — Theming, https://ui.shadcn.com/docs/theming (accessed 2026-10-15).
[3] shadcn/ui — Changelog, https://ui.shadcn.com/docs/changelog (accessed 2026-10-15).
[4] shadcn/ui — Components, https://ui.shadcn.com/docs/components (accessed 2026-10-15).
[5] shadcn/ui — Field, https://ui.shadcn.com/docs/components/base/field (accessed 2026-10-15).
[6] Kass J.M. et al. 2018, WALLACE: a flexible platform for reproducible modeling of species niches and distributions built for community expansion, *Methods Ecol. Evol.* 9(4):1151–1156, doi:10.1111/2041-210X.12945 [DOI UNVERIFIED – prior knowledge] (accessed 2026-10-15 via https://pure.au.dk/portal/da/publications/wallace-a-flexible-platform-for-reproducible-modeling-of-species-/).
[7] Wallace v2 tutorial, https://wallaceecomod.github.io/wallace/articles/tutorial-v2.html (accessed 2026-10-15).
[8] EcoCommons support — BCCVL Modelling Wizard guide, https://support.ecocommons.org.au/support/solutions/folders/6000240550 and ARDC case study https://ardc.edu.au/case-study/finding-fast-answers-to-urgent-ecological-questions/ (accessed 2026-10-15).
[9] Esri, ArcGIS Pro — Run geoprocessing tools, https://doc.esri.com/en/arcgis-pro/latest/help/analysis/geoprocessing/basics/run-geoprocessing-tools.html (accessed 2026-10-15).
[10] QGIS 3.44 User Manual — The Toolbox, https://docs.qgis.org/latest/en/docs/user_manual/processing/toolbox.html (accessed 2026-10-15).
[11] MacEachren A.M. et al. 2012, Visual Semiotics & Uncertainty Visualization: An Empirical Study, *IEEE TVCG* 18(12):2496–2505, doi:10.1109/TVCG.2012.279 [DOI UNVERIFIED – prior knowledge]; PDF https://geography.wisc.edu/cartography/projects/publications/MacEachrenEtAl_2012_TVCG.pdf (accessed 2026-10-15).
[12] Kinkeldey C., MacEachren A.M., Schiewe J. 2014, How to assess visual communication of uncertainty? A systematic review of geospatial uncertainty visualisation user studies, *Cartographic J.* 51(4):372–386, doi:10.1179/1743277414Y.0000000099 (accessed 2026-10-15).
[13] Correll M., Moritz D., Heer J. 2018, Value-Suppressing Uncertainty Palettes, CHI 2018, doi:10.1145/3173574.3174216 [DOI UNVERIFIED – prior knowledge]; https://idl.uw.edu/vsup/ (accessed 2026-10-15).
[14] `vsup` npm package v1.1.0, MIT, https://www.npmjs.com/package/vsup (accessed 2026-10-15).
[15] `pals` R package — bivariate palettes (vsup.viridis discouraged), https://search.r-project.org/CRAN/refmans/pals/html/bivariate.html (accessed 2026-10-15).
[16] Crameri F., Shephard G.E., Heron P.J. 2020, The misuse of colour in science communication, *Nat. Commun.* 11:5444, doi:10.1038/s41467-020-19160-7; colour maps doi:10.5281/zenodo.1243862 (accessed 2026-10-15).
[17] Nuñez J.R., Anderton C.R., Renslow R.S. 2018, Optimizing colormaps with consideration for color vision deficiency…, *PLOS ONE* 13(7):e0199239, doi:10.1371/journal.pone.0199239, https://pmc.ncbi.nlm.nih.gov/articles/PMC6070163 (accessed 2026-10-15).
[18] matplotlib 3.10.8 `viridis`, `cividis` colormaps, stops sampled locally at i/8 (2026-10-15).
[19] Phillips S.J. et al. 2017, Opening the black box: an open-source release of Maxent, *Ecography* 40:887–893, doi:10.1111/ecog.03049 [DOI/pages UNVERIFIED – prior knowledge]; AMNH Maxent page https://biodiversityinformatics.amnh.org/open_source/maxent (accessed 2026-10-15).
[20] Phillips S.J. 2017, A brief tutorial on Maxent, https://biodiversityinformatics.amnh.org/open_source/maxent/Maxent_tutorial2017.pdf (accessed 2026-10-15).
[21] Yackulic C.B. et al. 2013, Presence-only modelling using MAXENT: when can we trust the inferences?, *Methods Ecol. Evol.* 4:236–243, doi:10.1111/2041-210x.12004 [pages UNVERIFIED – prior knowledge]; https://pubs.usgs.gov/publication/70045720 (accessed 2026-10-15).
[22] Elith J., Kearney M., Phillips S. 2010, The art of modelling range-shifting species, *Methods Ecol. Evol.* 1:330–342 (accessed 2026-10-15 via https://findanexpert.unimelb.edu.au/scholarlywork/329654-the-art-of-modelling-range-shifting-species).
[23] W3C, Web Content Accessibility Guidelines (WCAG) 2.2, Recommendation 12 Dec 2024, https://www.w3.org/TR/WCAG22/ (accessed 2026-10-15).
[24] W3C, Understanding SC 2.5.8 Target Size (Minimum), https://w3c.github.io/wcag/understanding/target-size-minimum.html (accessed 2026-10-15).
[25] MapLibre GL JS — KeyboardHandler API, https://maplibre.org/maplibre-gl-js/docs/API/classes/KeyboardHandler/ (accessed 2026-10-15).
[26] B. Ottosson 2020, A perceptual color space for image processing (OKLab), https://bottosson.github.io/posts/oklab/ [UNVERIFIED – prior knowledge; matrices used for conversion].
[27] CRAN — wallace 2.2.1 (May 2026), https://cran.r-project.org/package=wallace (accessed 2026-10-15).
