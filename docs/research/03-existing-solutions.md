# 03 — Existing solutions: SDM tools, platforms and commercial offerings

Status: research draft · Access date for all sources: 2026-10-15 unless stated · Author: research agent (NicheRS)

## 1. Scope

What this covers: open-source and commercial software that fits, evaluates and projects species distribution models (SDM) or ecological niche models (ENM), with Maxent-family tools first, because NicheRS will ship MaxEnt first. For each tool: algorithms, stack, licence, performance, automation/API, batch processing, raster support, scientific validation, UX, operational limitations and maintenance status.

What this does not cover: detailed algorithm mathematics (see the MaxEnt/maxnet algorithm document), occurrence/environmental data sources, and Rust crate choices.

Method: CRAN package pages, the GitHub API (licence and `pushed_at`), PyPI JSON, the QGIS plugin repository, vendor documentation and peer-reviewed papers. A licence is marked [FACT] only when it was read from CRAN DESCRIPTION, PyPI metadata or the GitHub licence API. Note that the GitHub API reports `license: null` when no LICENSE file is detected, even if the DESCRIPTION declares GPL.

## 2. Findings

### 2.1 Maxent-family engines

**Maxent (Java, `maxent.jar`)**
- Algorithm: maximum-entropy density estimation with L1 regularisation and linear, quadratic, product, threshold and hinge features. Output formats are raw, logistic and cloglog. [UNVERIFIED – prior knowledge: cloglog has been the default output since 3.4.0]
- Stack and licence: Java desktop GUI plus CLI. The GitHub repo `mrmaxent/Maxent` reports an MIT licence, with separate third-party licence files [FACT][1]. It was open-sourced in 2017 (Phillips et al. 2017, Ecography) [FACT][2].
- Maintenance: the last GitHub release is v3.4.4 (2020-11-24) and the last commit is 2024-01-11, a README link fix [FACT][1][3]. Treat it as **frozen**: still the reference implementation, but not evolving.
- Automation: CLI flags and batch files. R access goes through `dismo::maxent()` or `predicts` via rJava. Runs are file-based (ASCII/MXE grids and CSV samples). [UNVERIFIED – prior knowledge]
- Performance: single JVM process with a `threads` option for replicates. Every R wrapper adds JVM and rJava friction. Wallace documents rJava bitness failures and a manual `maxent.jar` placement step [FACT][14]. No benchmark is cited here.
- Scientific standing: the most-cited SDM software and the de-facto reference for "MaxEnt". Its outputs are the baseline that NicheRS must reproduce. [ASSUMPTION – widely accepted, not quantified here]

**maxnet (R)**
- Algorithm: Maxent fitted as an inhomogeneous Poisson process through `glmnet` (Phillips et al. 2017) [FACT][2]. It is not bit-identical to maxent.jar. For example, default feature handling differs and there is no sequential-update optimiser [UNVERIFIED – prior knowledge].
- Licence: MIT. The only import is glmnet [FACT][4].
- Maintenance: CRAN 0.1.4 was published 2021-07-09 and the last GitHub commit is 2022-12-13 [FACT][4][5]. **Stable but dormant.**
- Role: the default Maxent in Wallace, ENMeval 2 and biomod2 (`MAXNET`) [FACT][13][14][9]. It is the de-facto "open Maxent" for R users.

**elapid (Python)**
- Algorithm: a Maxent reimplementation (scikit-learn API, feature transformers, regularisation) plus NicheEnvelopeModel [FACT][6].
- Stack and licence: Python ≥3.11 with numpy 2, scikit-learn ≥1.8, rasterio, geopandas and pyproj. MIT licence. Version 1.0.4 was released 2026-06-12 [FACT][7].
- Geospatial tooling: point annotation from rasters, `apply_model_to_rasters` (windowed raster prediction), background sampling from rasters/polygons/bias surfaces, checkerboard and geographic k-fold CV, and cloud URL reads [FACT][6]. The docs note that the pip install of the optional glmnet dependency is broken [FACT][6].
- Paper: Anderson 2023, JOSS, doi:10.21105/joss.04930 [FACT][6].
- Relevance: the closest architectural analogue to NicheRS's engine layer (a library rather than a GUI, raster-native, MIT). QMaxent (QGIS) is built on it [FACT][24].

### 2.2 Tuning, evaluation and workflow packages (R)

| Tool | What it adds | Licence [FACT] | Latest release [FACT] | Notes |
|---|---|---|---|---|
| **ENMeval 2.x** | Grid tuning of Maxent settings (feature classes × regularisation multiplier) via maxnet, maxent.jar or BIOCLIM, extensible through `ENMdetails`. Random, leave-one-out, spatial and user partitions. Metrics include AUC, Continuous Boyce Index and omission rates. Also null models and `rangeModelMetadata` [8][13] | GPL-3 | 2.0.6, 2026-09-11 [8] | Kass et al. 2021, MEE, doi:10.1111/2041-210X.13628 [13]. Partition names (block, checkerboard, jackknife) and AICc are [UNVERIFIED – prior knowledge]. Uses terra and foreach/parallel. **Actively maintained.** |
| **kuenm (v1)** | Maxent (jar) calibration across many candidate models, with selection by significance, omission and AICc | none detected in repo [22] | last push 2023-12-04 [22] | Superseded by kuenm2. |
| **kuenm2** | Multi-algorithm ENM calibration (imports glmnet and mgcv, so maxnet-style plus GLM/GAM), partial ROC (`fpROC`), MOP extrapolation risk (`mop`) and transfer variability [10] | GPL (≥3) | 0.1.3, 2026-04-20 [10] | terra-native and parallel through doSNOW. Exact algorithm set is [UNVERIFIED]. Young package. |
| **ENMTML** | End-to-end ENM pipeline: collinearity, bias control, accessible area (M), pseudo-absences, partitioning, thresholds, MSDM overprediction correction, ensembles, MOP [21] | README badge GPL (≥3). No licence file detected via API [21][23] | not on CRAN. Last push 2023-11-09 [23] | Banner reads "UNDER UPDATE!" **Effectively unmaintained.** |
| **modleR (ModelR)** | dismo-based workflow covering cleaning, pseudo-absences, CV/bootstrap and ensembling. Algorithms: bioclim, maxent, maxnet, mahal, domain, BRT, SVM, GLM, RF [25] | GPL (≥3) [26] | 1.0.0 (GitHub only) | README: "**no longer being actively maintained**". Depends on retired `rgeos`, `raster` and `sp` [25][26]. |

### 2.3 Multi-algorithm / ensemble frameworks (R)

**biomod2**
- Algorithms: 14 model codes (ANN, CTA, DNN, FDA, GAM, GBM, GLM, MARS, MAXENT, MAXNET, RF, RFd, SRE, XGBOOST) [FACT][9]. MAXENT means the Java jar [UNVERIFIED – prior knowledge].
- Cross-validation strategies: random, kfold, block, strat, env and user.defined [FACT][9].
- Metrics: ROC AUC, PR-AUC, TSS, Kappa, Boyce, MPA and others [FACT][9].
- Ensembles: committee averaging, weighted mean and others. The current version adds `EMmode`/`EMfreq` for qualitative data [FACT][12].
- Licence and release: GPL-3. Version 4.3-4-7 was published 2026-10-08. Maintained at Univ. Grenoble Alpes (LECA, a French lab) [FACT][11].
- Parallelism: `nb.cpu` parallelises single-model fitting [FACT][9]. The docs say tuning "is still a quite long running process" [FACT][12].
- Assessment: the European reference for ensemble SDM, widely used in French academic ecology. It has a heavy dependency graph, is not a service, and its object model is R-specific (S4 objects saved to disk).

**sdm (Naimi)**
- Algorithms: many through a pluggable method registry (GLM, GAM, BRT, RF, SVM, MARS, Maxent, Maxlike, bioclim, domain and others), plus ensembles and a GUI [UNVERIFIED – prior knowledge; Naimi & Araújo 2016 Ecography doi:10.1111/ecog.01881].
- Licence and release: GPL (≥3). Version 1.2-59 was published 2025-07-13. **Imports sp + raster**, the legacy stack [FACT][27].
- Risk: lagging migration to terra/sf.

**SSDM**
- Purpose: stacked SDMs for richness and endemism. Supports single or ensemble models per species, and stacking by summed probabilities, thresholding or Bernoulli draws. Has a Shiny GUI (`gui()`) [FACT][28][29].
- Licence and release: GPL (≥3). Version 0.2.11 was published 2025-05-20. Imports `raster`, `dismo` and `sdm` [FACT][28].
- Relevance: the closest prior art to the NicheRS requirement of N species leading to a richness/stack product.

**flexsdm**
- Scope: pre-modelling (bias, partitioning, pseudo-absence), modelling and post-modelling tools.
- Algorithms: DOM, GAM, GAU, GBM, GLM, MAX, NET, RAF and SVM. Supports `tune_*` for GBM, MAX, NET, RAF and SVM, ensembles, and Ensemble of Small Models (`esm_*`) [FACT][30].
- Licence and maintenance: GPL-3. The last commit was 2026-10-14 [FACT][30][31].
- Paper: Velazco et al. 2022, MEE [FACT – per README][30].

**dismo → predicts (Hijmans)**
- dismo 1.3-16 (2024-11-25) is GPL (≥3) and now imports terra [FACT][32]. It is the historical reference for `maxent()`, bioclim, domain, mahal and `evaluate()`.
- predicts 0.2-2 (2026-04-29) is GPL (≥3) and is the terra-native successor. rJava is only suggested, for maxent.jar [FACT][33].
- Low-level building blocks rather than workflows.

### 2.4 Specialised statistical SDMs (not direct competitors)

| Tool | Model class | Licence [FACT] | Release [FACT] | Why it matters |
|---|---|---|---|---|
| **sdmTMB** | Spatial and spatio-temporal GLMMs (TMB + SPDE/fmesher) | GPL-3 | 1.1.0, 2026-07-03 [34] | Needs abundance or presence/absence survey data, not presence-only. Used mostly for fisheries. Out of NicheRS scope. |
| **Hmsc** | Bayesian joint SDM (communities, traits, phylogeny), Gibbs MCMC | GPL-3 | 3.3-7, 2025-08-28 [35] | Hmsc-HPC GPU port (TensorFlow) reports >1000× speed-ups on the largest data sets (Rahman et al. 2024, PLoS Comput Biol 20:e1011914) [FACT][36]. Needs community survey matrices. Out of scope for a MaxEnt-first product, but relevant to the "hundreds of species" claim: **joint models are a different product**. |

### 2.5 GUIs and desktop/GIS integrations

**Wallace (R Shiny)**
- Scope: a modular, reproducible pipeline covering occurrences, environment, processing, partitioning, modelling and visualisation. maxnet is the default and maxent.jar is optional (manual jar step). It can download session code [FACT][14].
- Licence and release: GPL-3. Version 2.2.1 was published 2026-05-07. AMNH maintainer [FACT][15].
- Assessment: closest in **intent** to NicheRS (guided UX plus reproducibility), but it runs on the desktop as a single species per session, is not a multi-user service and is not built for batch work. [ASSUMPTION – based on design; batch features not verified]

**QGIS plugins**

| Plugin | Approach | Version / date [FACT] | Licence |
|---|---|---|---|
| **QMaxent** | Full Maxent workflow on elapid: CV, jackknife importance, response curves, survey planning. Private venv of about 590 MB. QGIS 3.44 to 3.99 | 0.1.7, 2026-05-17 [24] | MIT (GitHub API) [37] |
| **SDM** (jp-payopay) | Wizard covering thinning, background, VIF, CV, LR/GAM/RF/GBM/XGBoost/SVM/MLP/MaxEnt/ENFA, ensembles and an HTML report. QGIS ≥4.0 | 1.2.1, 2026-09-21 [38] | not stated [38] |
| **Maxent Model** | Only exports inputs (CSV/ASCII) for maxent.jar | 1.0, 2022 [39] | not verified |
| **SatSDM** | Satellite-derived predictors plus Maxent | 1.0.1, Oct 2025 [39] | not verified |
| **Processing R provider** | Runs R scripts (for example biomod2) as Processing algorithms | 4.1.0, 2024-01-15 [40] | not stated on page |

- Assessment: QGIS is where Natural Solutions-type users already work. Plugins are single-user, single-machine tools with fragile Python/R dependency management. None offers multi-species batch runs, job queues or audit trails. [ASSUMPTION – based on plugin descriptions]

**SDMtoolbox 2.0 (ArcGIS)**
- Scope: 79 Python scripts for ArcGIS covering SDM pre/post-processing, Maxent interfacing, corridors and biodiversity metrics [FACT][41]. Brown et al. 2017, PeerJ, doi:10.7717/peerj.4095.
- Requirements: ArcGIS (proprietary) with Spatial Analyst [UNVERIFIED – prior knowledge].
- Status: current maintenance and ArcGIS Pro compatibility could not be verified [OPEN].

**openModeller (C++)**
- Scope: a multi-algorithm niche-maxentling framework with a desktop GUI and web services. Licence is GPL [FACT – secondary source][42].
- Status: the last library release on SourceForge is 1.5.0 (2015-01-08) and Desktop was last updated 2010 [FACT][43]. The GitHub org repo URL returns 404. **Dead.** Historically useful as proof that a compiled-language SDM server (C++, SOAP web services) was built before. Lifemapper used it [FACT – secondary][44].

### 2.6 Hosted platforms

**EcoCommons (Australia; BCCVL successor)**
- History: BCCVL was shut down 2022-07-08. EcoCommons launched 2022-11-29 with a "BCCVL Modelling Wizard" covering single- and multi-species SDMs, climate projections and trait models. It also offers notebooks (BinderHub/Colab) [FACT – secondary][45].
- Platform claims: the site claims "over 17 algorithms" and cloud HPC. Hosted by QCIF with ARDC/NCRIS funding [FACT][46].
- Assessment: the only mature hosted SDM *service* found, and the strongest prior art for NicheRS as a server product. Constraints:
  - Access conditions for non-Australian users are not verified [OPEN].
  - No 2025–2026 development news was found [FACT – absence of evidence][46].
  - Engine is R-based (biomod2 family) [UNVERIFIED – prior knowledge].

**ShinyBIOMOD**
- Scope: a biomod2 Shiny GUI that won the 2020 GBIF Ebbe Nielsen Challenge [FACT – secondary][47].
- Status: current maintenance not verified.

**GBIF**
- GBIF provides occurrence data and download APIs only. No GBIF-operated SDM service was found [FACT – absence of evidence][47].

**Map of Life (MOL, Yale)**
- What it offers: expert range maps, point data and high-resolution refined distribution maps (with the Half-Earth Project). Biodiversity indicators are computed on Earth Engine [FACT – secondary][48][49].
- What it is not: a curated map product, not a user-run modelling tool. Public API status was not verified [OPEN].

**SpeciesDistributionToolkit.jl (Julia)**
- Status: active, last push 2026-09-26. Licence SPDX `NOASSERTION` [FACT][50].
- Relevance: evidence that non-R compiled-language SDM stacks are viable, but the community is small.

### 2.7 Commercial

**ArcGIS Pro — Presence-only Prediction (MaxEnt)** (Spatial Statistics) [FACT][51]
- Licensing: available at Basic, Standard and Advanced. Raster inputs/outputs require **Spatial Analyst**.
- Basis expansions: linear, product, hinge, threshold and quadratic. Hinge and threshold are mutually exclusive. Categorical variables are linear-only.
- Study areas: convex hull (default), raster extent or polygon. Background points are taken from cell centroids.
- Options:
  - Spatial thinning.
  - Presence/background relative weight (1–100).
  - Link function: cloglog (default) or logistic.
  - Probability cutoff.
  - Random k-fold CV (2–10 groups).
  - "Allow predictions outside data ranges", **on by default**.
- Outputs: response curves, a sensitivity table and a reusable model file. Parallel processing is used **only for prediction**. RNG is Mersenne Twister.
- Gaps (from the doc):
  - No spatial/block CV.
  - No tuning grid over regularisation.
  - No MOP or extrapolation map. There is a range-check flag, but it is permissive by default.
  - No multi-species batch beyond ModelBuilder/arcpy scripting [ASSUMPTION].
- Implication: a commercial vendor shipped a simplified MaxEnt with sane defaults. It is evidence that a reduced, opinionated option set is acceptable to practitioners, but its defaults (random CV, extrapolation allowed) fall below current methodological standards [REC: do not copy those defaults].

No other commercial SDM product with a verifiable feature set was found [OPEN]. Consultancies typically run R scripts in-house [ASSUMPTION].

### 2.8 Ecosystem-level evidence

Kass et al. 2024 (Ecography, doi:10.1111/ecog.07346; 25 authors including the maintainers of biomod2, ENMeval, sdm, SSDM, flexsdm and kuenm) make three points [FACT][52]:
- The SDM software ecosystem is dominated by R packages that users must combine across workflow steps.
- Researchers tend to stick to one or two packages.
- The authors provide `sdmverse` metadata and identify remaining gaps.

This is the strongest single argument that the field's problem is **integration and standards compliance, not missing algorithms**.

## 3. Comparative table

Legend: ✔ yes · ◐ partial · ✘ no · ? not verified. "Batch" means native multi-species runs.

| Tool | Algorithms | Stack | Licence | Last release | API / automation | Batch N spp | Raster I/O | Spatial CV / tuning | Extrapolation (MOP/clamping) | UX | Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Maxent.jar | Maxent | Java | MIT | 3.4.4 (2020-11) | CLI | ◐ scripts | ASCII/MXE | ✘ / ✘ | clamping, MESS ? | Desktop GUI | frozen |
| maxnet | Maxent (IPP/glmnet) | R | MIT | 0.1.4 (2021-07) | R fn | ✘ | via terra | ✘ / ✘ | clamp ✔ ? | none | dormant |
| elapid | Maxent, envelope | Python | MIT | 1.0.4 (2026-06) | Python lib | ◐ | rasterio, COG/cloud | ✔ / ◐ | ? | none | active |
| ENMeval 2 | maxnet, jar, bioclim, + | R | GPL-3 | 2.0.6 (2026-09) | R fn | ✘ | terra | ✔ / ✔ | ◐ ? | none | active |
| kuenm2 | maxnet-like, GLM ? | R | GPL≥3 | 0.1.3 (2026-04) | R fn | ✘ | terra | ✔ / ✔ | ✔ MOP | none | active, young |
| ENMTML | many | R | GPL≥3 (badge) | GitHub, 2023 | R fn | ✔ | raster | ✔ / ◐ | ✔ MOP | none | stale |
| modleR | dismo set | R | GPL≥3 | 1.0.0 | R fn | ◐ | raster | ◐ | ? | none | unmaintained |
| biomod2 | 14 | R | GPL-3 | 4.3-4-7 (2026-10) | R fn | ◐ loops | terra | ✔ / ◐ | ◐ ? | none (ShinyBIOMOD) | active |
| sdm | many, pluggable | R | GPL≥3 | 1.2-59 (2025-07) | R fn + GUI | ◐ | raster (legacy) | ◐ | ? | GUI | maintained, legacy deps |
| SSDM | ~9 + stacking | R | GPL≥3 | 0.2.11 (2025-05) | R fn + Shiny | ✔ (stack) | raster | ◐ | ? | Shiny | slow |
| flexsdm | 9 + ESM, ensemble | R | GPL-3 | GitHub (2026-10 commit) | R fn | ◐ | terra | ✔ / ✔ | ✔ ? | none | active |
| predicts / dismo | maxent(jar), envelopes | R | GPL≥3 | 0.2-2 (2026-04) / 1.3-16 | R fn | ✘ | terra | ◐ | ◐ | none | active / legacy |
| sdmTMB | spatial GLMM | R/TMB | GPL-3 | 1.1.0 (2026-07) | R fn | ✘ | ◐ | n/a | n/a | none | active |
| Hmsc (+HPC) | Bayesian JSDM | R (+Py/TF GPU) | GPL-3 | 3.3-7 (2025-08) | R fn | ✔ joint | ◐ | ◐ | n/a | none | active |
| Wallace | maxnet, jar, (bioclim?) | R Shiny | GPL-3 | 2.2.1 (2026-05) | session code | ✘ | terra | ✔ (ENMeval) | ◐ | guided GUI | active |
| QMaxent | Maxent (elapid) | QGIS/Py | MIT | 0.1.7 (2026-05) | ✘ | ✘ | QGIS | ✔ | ? | QGIS dialogs | active, young |
| QGIS "SDM" | 9 incl. MaxEnt | QGIS/Py | ? | 1.2.1 (2026-09) | ✘ | ✘ | QGIS | ✔ | ? | wizard + HTML | active, young |
| SDMtoolbox 2 | Maxent wrapper + GIS | ArcGIS/Py | ? | 2017 paper | ArcGIS | ◐ | ArcGIS | ◐ | ? | toolbox | ? |
| openModeller | many | C++ | GPL | 1.5.0 (2015-01) | SOAP WS | ◐ | GDAL | ✘ | ✘ | desktop | dead |
| EcoCommons | >17 (R) | hosted | n/a (service) | launched 2022-11 | notebooks | ✔ | hosted | ? | ? | web wizard | ? (no 2025–26 news) |
| ArcGIS Presence-only Prediction | Maxent-like | ArcGIS Pro | proprietary (+Spatial Analyst) | current Pro | arcpy | ◐ scripts | ArcGIS | ✘ random only / ✘ | ◐ flag, on by default | toolbox | active |

## 4. Evidence and references

See §9 References. All [FACT] items were read from the cited URL on 2026-10-15. Release dates come from CRAN `Published:`, PyPI `upload_time` or GitHub API `pushed_at`/releases.

## 5. Alternatives (positioning options for NicheRS)

| Option | Description | Comment |
|---|---|---|
| A. Orchestrate R | NicheRS as a Rust/web shell that calls ENMeval/maxnet/biomod2 in R workers | Highest scientific credibility from day 1 and reuses validated code. Cost: R runtime in deployment, so "single executable" is lost. Weak reproducibility of R package versions unless pinned (renv/containers). [ASSUMPTION] |
| B. Native engine (Rust) | Reimplement Maxent (maxnet IPP/glmnet-style or jar-style), validated against maxent.jar/maxnet | Matches the single-binary goal and performance. Burden: proving numerical equivalence. High risk of subtle divergence. |
| C. Hybrid | Native Rust engine for production, plus a CI "oracle" harness that runs maxnet/maxent.jar/ENMeval on fixture data sets and asserts agreement | [REC] Preferred. Uses mature tools as **test oracles** rather than runtime dependencies. |
| D. Python core (elapid) | Wrap elapid behind the API | Viable (MIT), but brings the Python packaging pain (QMaxent's 590 MB venv) and contradicts single-binary deployment. |

## 6. Trade-offs

- **Credibility vs. deployment**: reviewers and clients recognise "biomod2" and "ENMeval/maxnet" in method statements. "NicheRS native Maxent" will need a validation dossier before reports citing it are accepted. [ASSUMPTION]
- **Breadth vs. depth**: biomod2 has 14 algorithms and flexsdm has 9 plus ESM. Competing on algorithm count is unwinnable. Depth on one well-validated Maxent pipeline (tuning + spatial CV + MOP + uncertainty) is achievable.
- **Licence**: most of the R ecosystem is GPL-3. Porting code (not just ideas) from ENMeval, biomod2, kuenm2 or flexsdm would make NicheRS a derivative work. maxnet, maxent.jar and elapid are MIT, so code-level reuse with attribution is permitted. [FACT on licences; legal interpretation is ASSUMPTION — confirm with counsel]

## 7. Risks

1. **Numerical non-equivalence** with maxent.jar/maxnet: users comparing outputs lose trust. Mitigation: oracle tests (Alternative C).
2. **Methodological defaults**: copying ArcGIS-style defaults (random CV, extrapolation allowed) would produce over-optimistic maps. Mitigation: spatial CV and MOP on by default.
3. **Reinventing ENMeval/kuenm2** poorly: tuning and selection rules (omission-based, AICc, CBI) have published semantics. Diverging silently is a scientific-correctness bug.
4. **Licence contamination**: transliterating GPL R code into Rust.
5. **Scope creep** into JSDM (Hmsc), spatio-temporal GLMM (sdmTMB) or abundance models. These are different products.
6. **Upstream dormancy**: maxnet (2021) and maxent.jar (2020) are frozen references. Bugs found there will not be fixed upstream, so NicheRS must document any deliberate deviation.

## 8. Recommendations

**Reusable concepts and patterns** (ideas, not code)
- [REC] A settings grid × partition scheme × metrics table, as ENMeval `ENMevaluate` does, as the core "calibration run" artefact. Pick the selection rule explicitly: omission rate then AUC, AICc, or a kuenm2-style partial ROC + omission + AICc chain.
- [REC] Metadata in the `rangeModelMetadata` style: emit a machine-readable model card per run (data provenance, settings, partitions, metrics, software versions) [13].
- [REC] Partition schemes: block, checkerboard, random k-fold, user-defined and environmental (biomod2 `env`) [9]. Default to spatial.
- [REC] Extrapolation: MOP (kuenm2/`mop`) [10] and clamping maps shipped alongside every suitability map.
- [REC] Model-file reuse, as in ArcGIS's "trained model file" [51]. A serialised model enables re-projection without retraining.
- [REC] Windowed raster prediction (elapid `apply_model_to_rasters`) [6], tiled and parallel. This is where Rust/Rayon gives real gains. ArcGIS only parallelises prediction too [51].
- [REC] Stacking outputs (SSDM): summed probabilities, thresholded sums and a between-model variance map [28]. Treat these as the multi-species deliverable.
- [REC] Ensemble of Small Models (flexsdm `esm_*`) [30] as a future option for rare species with few records, a common case in consultancy work [ASSUMPTION].
- [REC] Wallace's "download session code" reproducibility idea [14]. NicheRS equivalent: an exportable, re-runnable job spec (JSON) plus an optional generated R script that reproduces the run with maxnet/ENMeval. This gives auditors a path back to the reference tools.

**Gaps NicheRS could address** (supported by the evidence above)
1. **Multi-species batch as a first-class service.** Tools fall into two groups: R libraries that need user-written loops (biomod2, ENMeval), or single-species GUIs (Wallace, QGIS plugins, ArcGIS). Only EcoCommons is a hosted multi-species service, and its availability and activity outside Australia are unclear.
2. **Deployable without R/Java/Python.** rJava bitness, manual `maxent.jar` placement and a 590 MB venv are documented pain points [14][24].
3. **Methodologically safe defaults for non-specialists.** Spatial CV, a tuning grid, MOP/extrapolation masks and null-model significance on by default. Most GUIs expose these as options, and ArcGIS omits them.
4. **Audit trail and reproducibility for consultancy deliverables.** Pinned engine version, seeds, input hashes and a model card per species. A regulatory and report context is not served by research-oriented packages [ASSUMPTION].
5. **Web map + comparison UX.** Side-by-side runs, area-of-suitable-habitat under several thresholds, and exports (COG/GeoPackage). These are scattered across packages today.

**Honest differentiation statement**
> NicheRS is not a new SDM method and should not claim to be more scientifically advanced than biomod2, ENMeval, kuenm2 or flexsdm. Its defensible differentiation is **operational**: a single deployable service that runs a validated, opinionated MaxEnt workflow (calibration, spatial validation, extrapolation diagnostics, uncertainty) for hundreds of species, reproducibly, with a map-centric UI for ecology professionals. Its scientific credibility must be *borrowed* through demonstrated agreement with maxnet/maxent.jar and ENMeval-style evaluation, not asserted.

**Where NicheRS should NOT compete** [REC]
- Algorithm breadth and ensembles across 10+ methods (biomod2, flexsdm, sdm). Add algorithms only with a validation story.
- Joint/community models (Hmsc), spatio-temporal mixed models (sdmTMB) and abundance models.
- Novel evaluation statistics: implement published ones (AUC, CBI, omission, AICc, partial ROC, TSS) with references.
- Being a full GIS: QGIS and ArcGIS already are. Export cleanly to them, and consider a thin QGIS plugin that calls the NicheRS API later.
- Research-grade flexibility (arbitrary formulas, custom partitions in code). Researchers will and should keep using R.

## 9. Open questions

- [OPEN] Exact numerical differences between maxent.jar 3.4.4 and maxnet 0.1.4 on identical inputs. Which one is NicheRS's reference oracle? (Feeds the algorithm document.)
- [OPEN] EcoCommons access for European/French organisations, current engine and licence terms of its workflows.
- [OPEN] SDMtoolbox maintenance and ArcGIS Pro compatibility.
- [OPEN] Licences of the QGIS "SDM" plugin and the Processing R provider. Licence of SpeciesDistributionToolkit.jl (`NOASSERTION`).
- [OPEN] Does any French or European consultancy tool or commercial SaaS exist for SDM? None found. A targeted French-language search and enquiries with peers are needed.
- [OPEN] Map of Life public API and whether its refined range products are usable as validation layers.
- [OPEN] Would clients and regulators accept a non-R engine in method statements, given a published validation note?

## References

[1] mrmaxent/Maxent GitHub repository (MIT licence, README). https://github.com/mrmaxent/Maxent — accessed 2026-10-15
[2] Phillips S.J. et al. 2017. Opening the black box: an open-source release of Maxent. Ecography 40:887–893. doi:10.1111/ecog.03049 (title/pages: UNVERIFIED – prior knowledge; DOI resolves) — accessed 2026-10-15
[3] GitHub API, mrmaxent/Maxent releases (v3.4.4, 2020-11-24) and commits (last 2024-01-11). https://api.github.com/repos/mrmaxent/Maxent/releases — accessed 2026-10-15
[4] CRAN maxnet 0.1.4 (Published 2021-07-09, MIT). https://cran.r-project.org/package=maxnet — accessed 2026-10-15
[5] GitHub API, mrmaxent/maxnet commits (last 2022-12-13). https://api.github.com/repos/mrmaxent/maxnet/commits — accessed 2026-10-15
[6] elapid documentation; Anderson C.B. 2023, JOSS, doi:10.21105/joss.04930. https://earth-chris.github.io/elapid/ — accessed 2026-10-15
[7] PyPI elapid 1.0.4 (2026-06-12, MIT, Python ≥3.11). https://pypi.org/project/elapid/ — accessed 2026-10-15
[8] CRAN ENMeval 2.0.6 (Published 2026-09-11, GPL-3). https://cran.r-project.org/package=ENMeval — accessed 2026-10-15
[9] biomod2 reference, BIOMOD_Modeling (models, CV.strategy, metric.eval, nb.cpu). https://biomodhub.github.io/biomod2/reference/BIOMOD_Modeling.html — accessed 2026-10-15
[10] CRAN kuenm2 0.1.3 (Published 2026-04-20, GPL≥3). https://cran.r-project.org/package=kuenm2 — accessed 2026-10-15
[11] CRAN biomod2 4.3-4-7 (Published 2026-10-08, GPL-3). https://cran.r-project.org/package=biomod2 — accessed 2026-10-15
[12] biomod2 website / news. https://biomodhub.github.io/biomod2/ — accessed 2026-10-15
[13] ENMeval website; Kass J.M. et al. 2021. ENMeval 2.0. Methods Ecol Evol 12:1602–1608. doi:10.1111/2041-210X.13628. https://jamiemkass.github.io/ENMeval/ — accessed 2026-10-15
[14] Wallace website (v2, maxnet default, maxent.jar/rJava notes). https://wallaceecomod.github.io/wallace/ — accessed 2026-10-15
[15] CRAN wallace 2.2.1 (Published 2026-05-07, GPL-3). https://cran.r-project.org/package=wallace — accessed 2026-10-15
[21] ENMTML GitHub README. https://github.com/andrefaa/ENMTML — accessed 2026-10-15
[22] GitHub API marlonecobos/kuenm (license null, pushed 2023-12-04). https://api.github.com/repos/marlonecobos/kuenm — accessed 2026-10-15
[23] GitHub API andrefaa/ENMTML (license null, pushed 2023-11-09). https://api.github.com/repos/andrefaa/ENMTML — accessed 2026-10-15
[24] QGIS plugin QMaxent 0.1.7 (2026-05-17). https://plugins.qgis.org/plugins/qmaxent/ — accessed 2026-10-15
[25] modleR GitHub README ("no longer being actively maintained"). https://github.com/Model-R/modleR — accessed 2026-10-15
[26] modleR DESCRIPTION (1.0.0, GPL≥3). https://raw.githubusercontent.com/Model-R/modleR/master/DESCRIPTION — accessed 2026-10-15
[27] CRAN sdm 1.2-59 (Published 2025-07-13, GPL≥3). https://cran.r-project.org/package=sdm — accessed 2026-10-15
[28] CRAN SSDM 0.2.11 (Published 2025-05-20, GPL≥3). https://cran.r-project.org/package=SSDM — accessed 2026-10-15
[29] SSDM GitHub README. https://github.com/sylvainschmitt/SSDM — accessed 2026-10-15
[30] flexsdm GitHub README (GPL-3, algorithms). https://github.com/sjevelazco/flexsdm — accessed 2026-10-15
[31] GitHub API flexsdm commits (last 2026-10-14). https://api.github.com/repos/sjevelazco/flexsdm/commits — accessed 2026-10-15
[32] CRAN dismo 1.3-16 (Published 2024-11-25, GPL≥3). https://cran.r-project.org/package=dismo — accessed 2026-10-15
[33] CRAN predicts 0.2-2 (Published 2026-04-29, GPL≥3). https://cran.r-project.org/package=predicts — accessed 2026-10-15
[34] CRAN sdmTMB 1.1.0 (Published 2026-07-03, GPL-3). https://cran.r-project.org/package=sdmTMB — accessed 2026-10-15
[35] CRAN Hmsc 3.3-7 (Published 2025-08-28, GPL-3). https://cran.r-project.org/package=Hmsc — accessed 2026-10-15
[36] Rahman A.U. et al. 2024. Accelerating joint species distribution modelling with Hmsc-HPC by GPU porting. PLoS Comput Biol 20:e1011914. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC11398642/ — accessed 2026-10-15 (via search summary)
[37] GitHub API osgeokr/qmaxent (MIT, pushed 2026-05-17). https://api.github.com/repos/osgeokr/qmaxent — accessed 2026-10-15
[38] QGIS plugin "SDM" 1.2.1 (2026-09-21). https://plugins.qgis.org/plugins/sdm_plugin/ — accessed 2026-10-15
[39] QGIS plugins Maxent Model / SatSDM (via repository search). https://plugins.qgis.org/plugins/maxent_model/ ; https://plugins.qgis.org/plugins/satsdm/ — accessed 2026-10-15
[40] QGIS plugin Processing R provider 4.1.0 (2024-01-15). https://plugins.qgis.org/plugins/processing_r/ — accessed 2026-10-15
[41] Brown J.L., Bennett J.R., French C.M. 2017. SDMtoolbox 2.0. PeerJ 5:e4095. doi:10.7717/peerj.4095. https://pmc.ncbi.nlm.nih.gov/articles/PMC5721907 — accessed 2026-10-15 (via search summary)
[42] CERN SciencePAD entry, openModeller (GPL-3.0). https://sciencepad.web.cern.ch/node/125.html — accessed 2026-10-15 (via search summary)
[43] SourceForge openModeller files (libopenmodeller 1.5.0, 2015-01-08). https://sourceforge.net/projects/openmodeller/files/ — accessed 2026-10-15
[44] Lifemapper / openModeller TDWG 2011 abstract. https://mbgocs.mobot.org/index.php/tdwg/2011/paper/view/207 — accessed 2026-10-15 (via search summary)
[45] ARDC / QCIF news on EcoCommons and BCCVL shutdown. https://ardc.edu.au/project/ecocommons ; https://www.qcif.edu.au/news/ecocommons-australia%3A-a-powerful-new-tool-for-ecological-modelling — accessed 2026-10-15 (via search summary)
[46] EcoCommons homepage. https://www.ecocommons.org.au/ — accessed 2026-10-15
[47] Web search: GBIF SDM services, ShinyBIOMOD (GBIF Ebbe Nielsen Challenge 2020). https://data.gbif.org/news/AcT155L4KYZ5RxsfDnGGt — accessed 2026-10-15 (via search summary)
[48] Jetz W., McPherson J.M., Guralnick R.P. 2012. Integrating biodiversity distribution knowledge: toward a global map of life. TREE. https://jetzlab.yale.edu/publications/integrating-biodiversity-distribution-knowledge-toward-global-map-life — accessed 2026-10-15 (via search summary)
[49] Map of Life high-resolution maps / richness documentation. https://mol.org/patterns/high-res/background ; https://mol.org/documentation/global-richness-rarity — accessed 2026-10-15 (via search summary)
[50] GitHub API PoisotLab/SpeciesDistributionToolkit.jl (NOASSERTION, pushed 2026-09-26). https://api.github.com/repos/PoisotLab/SpeciesDistributionToolkit.jl — accessed 2026-10-15
[51] Esri, ArcGIS Pro tool reference: Presence-only Prediction (MaxEnt). https://doc.esri.com/en/arcgis-pro/latest/tool-reference/spatial-statistics/presence-only-prediction.html — accessed 2026-10-15
[52] Kass J.M. et al. 2024. Achieving higher standards in species distribution modeling by leveraging the diversity of available software. Ecography. doi:10.1111/ecog.07346. https://cogs.sdsu.edu/publication/kass-2024-achieving-higher/ — accessed 2026-10-15
