# 07 — Scientific validation, uncertainty and confidence communication

Status: research note · Date: 2026-10-15 · Access date for all sources: 2026-10-15 unless stated.
Tags: [FACT] verified in a source read for this note · [ASSUMPTION] · [OPEN] · [REC] engineering recommendation · [UNVERIFIED – prior knowledge].

## 1. Scope

How NicheRS should validate SDMs (mostly presence-background MaxEnt first, see `01`), which
metrics are defensible, how to separate the different kinds of "uncertainty", how to communicate
confidence to ecology professionals without inventing a single "confidence %", and how to compute
suitable-habitat area correctly. Includes cross-validation design (spatial, environmental,
temporal, independent data), metrics (AUC, partial ROC, Boyce, TSS, calibration), thresholds,
extrapolation (MESS, MOP, AOA), ensemble/fold/parameter uncertainty, sampling-bias diagnostics
and reporting standards (ODMAP, Araújo et al. 2019).

## 2. Findings

### 2.1 Six quantities that must never be merged

| Quantity | What it is | Obtainable from PB data? | How NicheRS shows it | Ref |
|---|---|---|---|---|
| **Habitat suitability score** | Relative index (MaxEnt raw/cloglog, RF vote share) ranking cells by environmental similarity to occupied conditions | Yes | Main map, continuous scale, labelled "relative" | [FACT] [35], [21] |
| **Probability of occurrence** | P(species present in a cell of given size) | **No**, without prevalence or PA data | Only on PA pathway, with calibration evidence | [FACT] [21], [22] |
| **Statistical (predictive) uncertainty** | Variability of the prediction caused by the data sample, hyperparameters, algorithm choice | Partially (bootstrap/CV/ensemble spread) | Per-cell spread maps (SD/IQR across replicates) | [FACT] [3], [27] |
| **Model reliability** | Evidence that the model predicts held-out data well, under a CV design matching the intended use | Partially (PB metrics are biased; see §2.3) | Indicator panel per model: Boyce, omission, null-model comparison, fold spread | [FACT] [1], [6], [7] |
| **Confidence in input data** | Quantity, spatial/temporal coverage, positional accuracy, taxonomic reliability, sampling bias of occurrences; quality/resolution of predictors | Yes (data diagnostics) | Data-adequacy panel, before modelling | [FACT] [9] (response/predictor criteria) |
| **Geographic transferability** | Whether predictions in a given cell rely on environmental conditions represented in training data | Yes (MESS, MOP, AOA) | Mask/hatching layer + % of area outside applicability | [FACT] [11], [12] |

[REC] Each of these has its own indicator(s) and its own UI element. None is derived from another
(in particular, AUC says nothing about per-cell extrapolation or data adequacy).

### 2.2 Cross-validation design

- [FACT] Random CV on structured data underestimates prediction error and can lead to
  inappropriate model selection; structured data also invite overfitting with non-causal
  predictors; block CV is "nearly universally more appropriate" when predicting to new data or
  predictor space [6].
- [FACT] But blocking in space/time can induce extrapolation by restricting predictor ranges in
  training folds, overestimating interpolation error; deliberate blocking in predictor space is
  appropriate when extrapolation is the goal [6].
- [FACT] blockCV (R) generates spatially or environmentally separated folds and measures spatial
  autocorrelation ranges of covariates [5]. Current CRAN version 4.0-1 (2026-09-28, GPL ≥3)
  exports `cv_spatial`, `cv_cluster` (environmental or spatial clustering), `cv_buffer`
  (buffered LOO), `cv_nndm`, `cv_knndm`, `cv_group`, `cv_spatial_autocor`, `cv_block_size`,
  `cv_similarity` (extrapolation in test folds), `cv_distance`, `cv_summary` [FACT] [5b].
- [FACT] NNDM LOO CV matches the nearest-neighbour distance distribution between test and
  training points to that between prediction locations and training points [13]; CAST 1.1.2
  implements NNDM, kNNDM and AOA [FACT] [12b].
- [FACT] Spatial validation revealed poor predictive performance of large-scale mapping models
  that random CV had overstated [14]. Counterpoint: spatial CV "is not the right way to evaluate
  map accuracy" when a probability sample of the map area is available [15]. [OPEN] For
  opportunistic occurrences (never a probability sample), [REC] spatial/NNDM-type CV is the default.
- [FACT] Valavi et al. 2023: method rankings unchanged between random and 75 km spatial-block
  testing; tuned flexible methods still best [16].
- [FACT] Spatial sorting bias inflates AUC under random CV; Hijmans proposes calibration with a
  null model / pairwise distance sampling [17].
- Temporal validation [REC]: when records span decades, fit on older records and test on recent
  (Roberts' "temporal blocking" [6]); also checks for land-use/climate drift. Requires the
  `eventDate` field to be kept through the pipeline.
- Independent data [FACT] [9]: the Araújo et al. evaluation questions include "How predictive is
  the model when tested against independent data?". [REC] When PA survey data exist (e.g.
  standardised inventories), use them as the evaluation set, as in [36], [37].

### 2.3 Metrics and their limits

| Metric | Definition | Known problems | Use in NicheRS | Ref |
|---|---|---|---|---|
| AUC-ROC | P(score at presence > score at absence/background) | Lobo 2008: ignores predicted values and goodness-of-fit; summarises regions of ROC never used; equal weight to omission/commission; no spatial info; strongly depends on the extent of the study area [FACT] [1]. With PB, maximum attainable AUC < 1 and depends on species prevalence [UNVERIFIED – prior knowledge, Phillips et al. 2006]. | Report as "AUC (presence-background)", only for comparing models of the same species/extent/background; never across species; never as "confidence" | [1], [18] |
| Partial ROC | AUC ratio over the region with omission ≤ E (user-set data error, default 5 % in implementations), against random expectation | Requires choice of E; still discrimination-only | Optional secondary metric | [FACT] [2], implementations (fpROC, enmpa) |
| Continuous Boyce index | Spearman correlation between suitability class (moving window) and predicted/expected (P/E) ratio of test presences; −1..+1, ~0 = random | Window width/resolution choices; insensitive to absolute calibration; presence-only so works for PB | **Primary** PB evaluation metric, with P/E curve displayed | [FACT] [3], [3b] |
| P/E curve | P/E ratio per suitability class | Visual; noisy with few test presences | Show as "does higher suitability mean more records?" | [FACT] [3] |
| TSS | sensitivity + specificity − 1 | Allouche: claimed prevalence-independent [FACT] [4]; Leroy 2018: TSS depends on prevalence even with ideal PA sampling, and with biased or pseudo-absence data **no** discrimination metric gives adequate estimates [FACT] [19] | Only on PA test data; never headline on PB | [4], [19] |
| Omission rate at threshold | Fraction of test presences below threshold (e.g. 10th-percentile training threshold) | Depends on threshold; compare with expected rate (e.g. 10 %) | **Primary** for PB, signals overfitting | [FACT] used in [20]; [UNVERIFIED – prior knowledge for OR10 as standard] |
| Null-model significance | Compare metric against models built on random localities | Compute cost | Recommended; ENMeval 2.0 implements null-model significance and effect sizes [FACT] [25] | [25] |
| Calibration (PA) | Agreement of predicted probability with observed frequency (calibration plot, Brier, log-loss) | Not possible on PB [FACT] [21], [22] | PA pathway only | [FACT] Norberg: most precise models poorly calibrated [26] |

### 2.4 Thresholds

| Threshold | Rule | Behaviour | Ref |
|---|---|---|---|
| MTP / LPT | lowest predicted value at a training presence | zero training omission; maximal area; driven by single (possibly erroneous) record | [Pearson 2007, 23] [UNVERIFIED – prior knowledge for details] |
| 10th percentile training presence (P10) | value excluding the 10 % lowest training presences | robust to a few outliers; assumes ~10 % records erroneous/marginal | [20] [UNVERIFIED – prior knowledge] |
| maxSSS | maximise sensitivity + specificity | not affected by pseudo-absences; produces same threshold from PO as from PA data; consistent across presence:random ratios | [FACT] [7], [8] |
| maxFpb | maximise F-measure for PB | varies with known-presence/random-point ratio | [FACT] [8] |
| Fixed (e.g. 0.5) | arbitrary | "the worst" among 12 approaches | [FACT] [24] |
| Prevalence-based | threshold = training prevalence | requires real prevalence; PB prevalence is arbitrary | [FACT] [24]; [21] |

- [FACT] Threshold-based prevalence estimates from PB were "almost always biased" [8].
- [FACT] Converting continuous output to presence/absence "usually degrades inference" [22].
- [REC] Binary maps and suitable-area figures are *derived products*: always show the threshold
  rule, its value, and the area under at least MTP, P10 and maxSSS side by side (a sensitivity
  band, not a single number).

### 2.5 Extrapolation, novelty, applicability

- [FACT] MESS (multivariate environmental similarity surface) maps where prediction-domain values
  fall outside training ranges; introduced in Elith, Kearney & Phillips 2010 [10]
  [UNVERIFIED – prior knowledge for formula: per-variable similarity based on percentile within
  the reference distribution, MESS = minimum over variables; negative = outside range].
- [FACT] ExDet ("here be dragons") quantifies novelty from both univariate range and
  covariance (correlation-change) novelty [11b]; MOP addresses limited environmental ranges in
  calibration areas [11c].
- [FACT] AOA (Meyer & Pebesma 2021): dissimilarity index DI = minimum distance to training data
  in importance-weighted predictor space; AOA threshold = outlier-removed maximum DI of training
  data under CV; inside the AOA prediction error ≈ CV error, outside the CV error does not apply;
  DI–performance relationship can map expected performance [12]. Implemented in CAST [12b];
  applied to global prediction models showing limited transferability [12c].
- [REC] MESS (or ExDet) for MaxEnt (range-based, model-agnostic) + AOA where variable importance
  is available. Cells with MESS < 0 or outside AOA are hatched, never blank-masked silently.

### 2.6 Other uncertainty components

- Ensemble/algorithm disagreement [UNVERIFIED – prior knowledge] [27]: algorithm choice is a major source of predictive
  variation; [FACT] Araújo gold-level model building "quantif[ies] the degree to which variability in
  the final predictions is introduced by the modeling methods themselves" [9].
- Parameter / hyperparameter uncertainty [REC]: bootstrap or subsample presences (n replicates,
  seeded), refit with the tuned settings, store per-cell SD/IQR; additionally report sensitivity
  to the regularisation multiplier (MaxEnt has no analytical standard errors under L1)
  [UNVERIFIED – prior knowledge].
- Fold stability [REC]: SD of each metric across CV folds; rank stability of variable
  importance; per-cell SD of fold-model predictions.
- [FACT] "Maps of ignorance": explicit spatial representation of uncertainty is needed
  alongside predictions [28]; uncertainty should be incorporated in predictions rather than
  reported as a single global figure [29].

### 2.7 Sampling bias diagnostics

- [FACT] Sample selection bias affects PO models; target-group background is a remedy when
  bias is shared with a group of related species [30].
- [FACT] Bias can be corrected by pooling survey and collection data in a shared model [31].
- [FACT] Predictors can "predict" distributions spuriously (paintings as predictors achieved
  good AUC), showing evaluation metrics cannot validate predictor choice [32].
- [REC] Diagnostics computed before modelling: (a) nearest-neighbour clustering of records vs
  random points in the study area; (b) environmental coverage: proportion of study-area
  environmental space (e.g. PCA bins) represented by records; (c) density of records vs proxies
  of access (roads, towns, protected areas) — correlation reported, not "corrected" silently;
  (d) record-date histogram; (e) coordinate-uncertainty distribution vs raster resolution
  (Araújo: whether spatial accuracy of occurrences matches predictor resolution [FACT] [9]).

### 2.8 Reporting standards

- [FACT] ODMAP (Overview, Data, Model, Assessment, Prediction) is a standard protocol for
  reporting SDMs, serving as checklist and structured documentation; an interactive web app
  exists [9b].
- [FACT] Araújo et al. 2019 define four levels — gold (aspirational), silver (cutting-edge),
  bronze (minimum acceptable), deficient (unacceptable for policy) — across response variable,
  predictor variables, model building and evaluation, for explanation/prediction/projection;
  flagging extrapolation outside each predictor's training extent is a bronze-level practice;
  400 studies scored showed generally low adequacy [9].
- [REC] NicheRS auto-generates an ODMAP record per run and a self-assessment against the
  Araújo criteria that the platform can check mechanically (e.g. "extrapolation flagged: yes",
  "independent evaluation data: no"), leaving non-checkable items for the user.

### 2.9 Suitable-habitat area

- [FACT] EPSG:3035 = ETRS89-extended / LAEA Europe: Lambert Azimuthal Equal Area, centre 52°N
  10°E, metres, pan-European area of use [33].
- [REC] Area = Σ over cells above threshold of (cell area × fraction of cell inside study area).
  Two correct options:
  1. Model on an equal-area grid (EPSG:3035 for Europe): every cell has the same area.
  2. Model on geographic (EPSG:4326) grid: compute **per-cell ellipsoidal area** (depends on
     latitude), never `n_cells × nominal resolution²`.
- [ASSUMPTION] Reprojecting a finished suitability raster to compute area introduces resampling
  error at range edges; prefer computing area on the modelling grid with exact cell areas.
- [UNVERIFIED – prior knowledge] Lambert-93 (EPSG:2154), the French national CRS, is conformal
  (not equal-area); its area distortion is small within France but nonzero, so it should not be
  the area-computation CRS for reported figures without a correction.
- [REC] Report area as a band (MTP / P10 / maxSSS, and across bootstrap/fold replicates), with
  the explicit sentence: "area of environmentally suitable conditions according to the model,
  not occupied area" [22].
- [REC] Cells outside the AOA / MESS < 0 are counted separately ("suitable but extrapolated").

## 3. Evidence and references

Read for this note: abstracts of [1], [4], [5], [6], [7], [8], [12], [16], [19], [22], [24], [25];
Araújo 2019 full text (Europe PMC PMC6357756) [9]; Norberg 2019 full text [26]; ecospat.boyce
documentation [3b]; blockCV and CAST CRAN reference manuals [5b], [12b]; epsg.io [33]. DOIs
resolved via Crossref API on 2026-10-15.

## 4. Proposed confidence communication method (no fabricated "confidence %")

Principle [REC]: present **evidence**, not a score. Each indicator is a measured value with a
plain-language rule; the overall statement is a categorical evidence level derived from explicit,
published rules, modelled on Araújo's tiers [9], never an AUC-derived percentage.

### 4.1 Indicator panel (per species × model run)

| Block | Indicator | Computation | Display |
|---|---|---|---|
| A. Input data | n records after cleaning/thinning; n distinct cells | counts | number + rule (e.g. "< 15 records: low", thresholds configurable, documented) [ASSUMPTION on cut-offs] |
| | Environmental coverage | % of study-area environmental bins containing ≥ 1 record | % + bivariate plot |
| | Spatial clustering | NN ratio records vs random | value + qualitative label |
| | Positional fit | % records with coordinate uncertainty ≤ cell size | % |
| | Temporal span | min/max/median year | histogram |
| B. Model reliability (spatial-block CV) | Continuous Boyce (median, IQR across folds) | ecospat-equivalent | value + P/E curve |
| | Test omission at P10 vs expected 10 % | per fold | value; flag if ≫ expected |
| | AUC-PB (labelled) | per fold | small, secondary |
| | Better than null? | null-model distribution (ENMeval-style) | yes/no + effect size |
| | Fold stability | SD of metrics across folds | value |
| C. Per-cell uncertainty | Replicate spread | SD or IQR of bootstrap/fold predictions | map layer |
| | Algorithm disagreement (when ≥ 2 engines) | SD across engines; % agreement of binaries | map layer |
| D. Transferability | MESS < 0 / outside AOA | per cell | hatched overlay + % of study area |
| E. Area | Suitable area under MTP / P10 / maxSSS × replicate range | equal-area computation | range bar, not a single value |
| F. Reporting | ODMAP record; Araújo self-check | auto | downloadable |

### 4.2 Map presentation

- [REC] Default map: suitability (sequential palette) with hatching for extrapolation (D); a
  toggle for a bivariate suitability × uncertainty map (C) following the "maps of ignorance"
  idea [28].
- [REC] Never show an uncertainty layer with the same colour scale as suitability.
- [REC] Legends state "relative suitability (not probability)" for PB models.

### 4.3 Evidence level (overall statement)

[REC] Three categorical levels, computed from explicit rules on blocks A, B, D, displayed with
the failing rules listed:
- **Exploratory**: any of — fewer records than minimum; Boyce median ≤ 0 or not better than null;
  > X % of study area outside AOA/MESS.
- **Indicative**: passes the above with spatial-block CV only (no independent data).
- **Supported**: additionally validated on independent (PA or temporally separated) data.
The cut-offs (X, minimum n) are [OPEN] and must be set with Natural Solutions ecologists and
documented in the method notes; they are policy, not statistics.

## 5. Alternatives

1. Single composite "confidence score" (weighted sum of indicators) — rejected: arbitrary
   weights, hides failing components, invites misuse [REC].
2. AUC-only reporting — rejected [1], [19].
3. Full Bayesian uncertainty (e.g. spatial point-process models with posterior intervals) —
   statistically richer, but conditional on model structure and costly; [OPEN] for roadmap.
4. Random k-fold CV as default — rejected for PO data [6], [14], [17]; kept as diagnostic only.

## 6. Trade-offs

- Spatial blocks vs NNDM/kNNDM: blocks are simple and explainable; NNDM matches the
  prediction task better but is harder to explain and compute [13]. [REC] spatial blocks
  (size from autocorrelation range) as default, kNNDM as advanced option.
- Environmental blocking tests extrapolation but pessimistically for interpolation tasks [6].
- More replicates → better uncertainty maps, linear compute cost per species.
- Equal-area modelling grid simplifies area but forces reprojection of inputs (resampling of
  predictors) upfront.

## 7. Risks

| Risk | Mitigation |
|---|---|
| Users treat AUC/Boyce as "accuracy %" | No % framing; labelled metrics; evidence levels with rules |
| Optimistic CV due to clustered records | Spatial/NNDM CV default; report random-CV gap as diagnostic |
| Thresholded area quoted as occupied range | Area band + mandatory wording [22] |
| Silent extrapolation in projections (e.g. climate scenarios) | MESS/AOA overlay, % extrapolated in summary |
| Area computed on lat-lon grid with nominal cell size | Per-cell ellipsoidal area or EPSG:3035; unit tests vs known areas |
| Bias-driven models pass CV (bias in both train and test) | Bias diagnostics (§2.7); target-group background option; independent data when available [30], [32] |

## 8. Recommendations

1. [REC] Default validation: spatial-block CV (block size from predictor autocorrelation, ≥ 5
   folds, folds balanced on presence counts), with fold assignments persisted; optional
   environmental (cluster) blocks for extrapolation tasks, temporal split if dates permit.
2. [REC] Headline PB metrics: continuous Boyce + P/E curve, test omission at P10, null-model
   comparison; AUC-PB secondary and labelled. TSS and calibration only on PA data.
3. [REC] Per-cell layers: replicate SD, MESS (+ AOA when importance available), algorithm
   disagreement once a 2nd engine exists.
4. [REC] Area: equal-area computation (EPSG:3035 grid or per-cell geodesic area), reported as a
   band across MTP/P10/maxSSS and replicates; wording "suitable conditions, not occupancy".
5. [REC] Evidence levels (Exploratory/Indicative/Supported) with rule lists; no confidence %.
6. [REC] ODMAP export and Araújo self-check per run.
7. [REC] Golden tests: reproduce ecospat Boyce, blockCV fold logic properties, and maxSSS on
   fixed datasets with documented tolerances.

## 9. Open questions

- [OPEN] Cut-offs for evidence levels (minimum records, max % extrapolated) — needs ecologist
  agreement and possibly a calibration study on known cases.
- [OPEN] Default block size when autocorrelation ranges exceed study-area dimensions (small
  consultancy study areas).
- [OPEN] Which independent PA datasets exist per taxon in France for "Supported" level.
- [OPEN] Whether AOA (developed for ML regression with variable importance) is meaningful for
  MaxEnt with permutation importance — needs testing against MESS.
- [OPEN] Boyce window width and class count defaults for small test sets.

## References

[1] Lobo J.M., Jiménez-Valverde A., Real R. 2008. AUC: a misleading measure of the performance of predictive distribution models. Global Ecology and Biogeography 17:145–151. doi:10.1111/j.1466-8238.2007.00358.x (accessed 2026-10-15)
[2] Peterson A.T., Papeş M., Soberón J. 2008. Rethinking receiver operating characteristic analysis applications in ecological niche modeling. Ecological Modelling 213:63–72. doi:10.1016/j.ecolmodel.2007.11.008 (accessed 2026-10-15)
[3] Hirzel A.H., Le Lay G., Helfer V., Randin C., Guisan A. 2006. Evaluating the ability of habitat suitability models to predict species presences. Ecological Modelling 199:142–152. doi:10.1016/j.ecolmodel.2006.05.017 (accessed 2026-10-15)
[3b] ecospat R package, ecospat.boyce documentation; CRAN ecospat 4.1.4 (2026-06-19, GPL ≥2). https://rdrr.io/pkg/ecospat/man/ecospat.boyce.html (accessed 2026-10-15)
[3c] Boyce M.S. et al. 2002. Evaluating resource selection functions. Ecological Modelling 157:281–300. doi:10.1016/S0304-3800(02)00200-4 (accessed 2026-10-15)
[4] Allouche O., Tsoar A., Kadmon R. 2006. Assessing the accuracy of species distribution models: prevalence, kappa and the true skill statistic (TSS). Journal of Applied Ecology 43:1223–1232. doi:10.1111/j.1365-2664.2006.01214.x (accessed 2026-10-15)
[5] Valavi R., Elith J., Lahoz-Monfort J.J., Guillera-Arroita G. 2019. blockCV: an R package for generating spatially or environmentally separated folds for k-fold cross-validation of species distribution models. Methods in Ecology and Evolution 10:225–232. doi:10.1111/2041-210X.13107 (accessed 2026-10-15)
[5b] blockCV R package 4.0-1 (CRAN, 2026-09-28, GPL ≥3), reference manual. https://cran.r-project.org/package=blockCV (accessed 2026-10-15)
[6] Roberts D.R. et al. 2017. Cross-validation strategies for data with temporal, spatial, hierarchical, or phylogenetic structure. Ecography 40:913–929. doi:10.1111/ecog.02881 (accessed 2026-10-15)
[7] Liu C., White M., Newell G. 2013. Selecting thresholds for the prediction of species occurrence with presence-only data. Journal of Biogeography 40:778–789. doi:10.1111/jbi.12058 (accessed 2026-10-15)
[8] Liu C., Newell G., White M. 2016. On the selection of thresholds for predicting species occurrence with presence-only data. Ecology and Evolution 6:337–348. doi:10.1002/ece3.1878 (accessed 2026-10-15)
[9] Araújo M.B. et al. 2019. Standards for distribution models in biodiversity assessments. Science Advances 5:eaat4858. doi:10.1126/sciadv.aat4858 (full text via Europe PMC PMC6357756, accessed 2026-10-15)
[9b] Zurell D. et al. 2020. A standard protocol for reporting species distribution models. Ecography 43:1261–1277. doi:10.1111/ecog.04960 (accessed 2026-10-15)
[10] Elith J., Kearney M., Phillips S. 2010. The art of modelling range-shifting species. Methods in Ecology and Evolution 1:330–342. doi:10.1111/j.2041-210X.2010.00036.x (accessed 2026-10-15)
[11b] Mesgaran M.B., Cousens R.D., Webber B.L. 2014. Here be dragons: a tool for quantifying novelty due to covariate range and correlation change when projecting species distribution models. Diversity and Distributions 20:1147–1159. doi:10.1111/ddi.12209 (accessed 2026-10-15)
[11c] Owens H.L. et al. 2013. Constraints on interpretation of ecological niche models by limited environmental ranges on calibration areas. Ecological Modelling 263:10–18. doi:10.1016/j.ecolmodel.2013.04.011 (accessed 2026-10-15)
[12] Meyer H., Pebesma E. 2021. Predicting into unknown space? Estimating the area of applicability of spatial prediction models. Methods in Ecology and Evolution 12:1620–1633. doi:10.1111/2041-210X.13650 (accessed 2026-10-15)
[12b] CAST R package 1.1.2 (CRAN, 2026-09-09, GPL ≥2), reference manual (aoa, trainDI, nndm, knndm). https://cran.r-project.org/package=CAST (accessed 2026-10-15)
[12c] Ludwig M. et al. 2023. Assessing and improving the transferability of current global spatial prediction models. Global Ecology and Biogeography 32:356–368. doi:10.1111/geb.13635 (accessed 2026-10-15)
[13] Milà C., Mateu J., Pebesma E., Meyer H. 2022. Nearest neighbour distance matching Leave-One-Out Cross-Validation for map validation. Methods in Ecology and Evolution 13:1304–1316. doi:10.1111/2041-210X.13851 (accessed 2026-10-15)
[14] Ploton P. et al. 2020. Spatial validation reveals poor predictive performance of large-scale ecological mapping models. Nature Communications 11:4540. doi:10.1038/s41467-020-18321-y (accessed 2026-10-15)
[15] Wadoux A.M.J.-C. et al. 2021. Spatial cross-validation is not the right way to evaluate map accuracy. Ecological Modelling 457:109692. doi:10.1016/j.ecolmodel.2021.109692 (accessed 2026-10-15)
[16] Valavi R., Elith J., Lahoz-Monfort J.J., Guillera-Arroita G. 2023. Flexible species distribution modelling methods perform well on spatially separated testing data. Global Ecology and Biogeography 32:369–383. doi:10.1111/geb.13639 (accessed 2026-10-15)
[17] Hijmans R.J. 2012. Cross-validation of species distribution models: removing spatial sorting bias and calibration with a null model. Ecology 93:679–688. doi:10.1890/11-0826.1 (accessed 2026-10-15)
[18] Jiménez-Valverde A. 2012. Insights into the area under the receiver operating characteristic curve (AUC) as a discrimination measure in species distribution modelling. Global Ecology and Biogeography 21:498–507. doi:10.1111/j.1466-8238.2011.00683.x (accessed 2026-10-15)
[19] Leroy B. et al. 2018. Without quality presence–absence data, discrimination metrics such as TSS can be misleading measures of model performance. Journal of Biogeography 45:1994–2002. doi:10.1111/jbi.13402 (accessed 2026-10-15)
[20] Radosavljevic A., Anderson R.P. 2014. Making better Maxent models of species distributions: complexity, overfitting and evaluation. Journal of Biogeography 41:629–643. doi:10.1111/jbi.12227 (accessed 2026-10-15)
[21] Phillips S.J., Elith J. 2013. On estimating probability of presence from use–availability or presence–background data. Ecology 94:1409–1419. doi:10.1890/12-1520.1 (accessed 2026-10-15)
[22] Guillera-Arroita G. et al. 2015. Is my species distribution model fit for purpose? Matching data and models to applications. Global Ecology and Biogeography 24:276–292. doi:10.1111/geb.12268 (accessed 2026-10-15)
[23] Pearson R.G. et al. 2007. Predicting species distributions from small numbers of occurrence records: a test case using cryptic geckos in Madagascar. Journal of Biogeography 34:102–117. doi:10.1111/j.1365-2699.2006.01594.x (accessed 2026-10-15)
[24] Liu C., Berry P.M., Dawson T.P., Pearson R.G. 2005. Selecting thresholds of occurrence in the prediction of species distributions. Ecography 28:385–393. doi:10.1111/j.0906-7590.2005.03957.x (accessed 2026-10-15)
[25] Kass J.M. et al. 2021. ENMeval 2.0: redesigned for customizable and reproducible modeling of species' niches and distributions. Methods in Ecology and Evolution 12:1602–1608. doi:10.1111/2041-210X.13628 (accessed 2026-10-15)
[26] Norberg A. et al. 2019. A comprehensive evaluation of predictive performance of 33 species distribution models at species and community levels. Ecological Monographs 89(3):e01370. doi:10.1002/ecm.1370 (accessed 2026-10-15)
[27] Hao T. et al. 2019. A review of evidence about use and performance of species distribution modelling ensembles like BIOMOD. Diversity and Distributions 25:839–852. doi:10.1111/ddi.12892 (accessed 2026-10-15) [UNVERIFIED – prior knowledge for the claim that algorithm choice is a major variance source; abstract not read]
[28] Rocchini D. et al. 2011. Accounting for uncertainty when mapping species distributions: the need for maps of ignorance. Progress in Physical Geography 35:211–226. doi:10.1177/0309133311399491 (accessed 2026-10-15)
[29] Beale C.M., Lennon J.J. 2012. Incorporating uncertainty in predictive species distribution modelling. Phil. Trans. R. Soc. B 367:247–258. doi:10.1098/rstb.2011.0178 (accessed 2026-10-15)
[30] Phillips S.J. et al. 2009. Sample selection bias and presence-only distribution models: implications for background and pseudo-absence data. Ecological Applications 19:181–197. doi:10.1890/07-2153.1 (accessed 2026-10-15)
[31] Fithian W., Elith J., Hastie T., Keith D.A. 2015. Bias correction in species distribution models: pooling survey and collection data for multiple species. Methods in Ecology and Evolution 6:424–438. doi:10.1111/2041-210X.12242 (accessed 2026-10-15)
[32] Fourcade Y., Besnard A.G., Secondi J. 2018. Paintings predict the distribution of species, or the challenge of selecting environmental predictors and evaluation statistics. Global Ecology and Biogeography 27:245–256. doi:10.1111/geb.12684 (accessed 2026-10-15)
[33] EPSG:3035 — ETRS89-extended / LAEA Europe. https://epsg.io/3035 (accessed 2026-10-15)
[34] Fielding A.H., Bell J.F. 1997. A review of methods for the assessment of prediction errors in conservation presence/absence models. Environmental Conservation 24:38–49. doi:10.1017/S0376892997000088 (accessed 2026-10-15)
[35] Fithian W., Hastie T. 2013. Finite-sample equivalence in statistical models for presence-only data. Annals of Applied Statistics 7:1917–1939. doi:10.1214/13-AOAS667 (accessed 2026-10-15)
[36] Elith J. et al. 2006. Novel methods improve prediction of species' distributions from occurrence data. Ecography 29:129–151. doi:10.1111/j.2006.0906-7590.04596.x (accessed 2026-10-15)
[37] Valavi R., Guillera-Arroita G., Lahoz-Monfort J.J., Elith J. 2022. Predictive performance of presence-only species distribution models: a benchmark study with reproducible code. Ecological Monographs 92(1):e01486. doi:10.1002/ecm.1486 (accessed 2026-10-15)
