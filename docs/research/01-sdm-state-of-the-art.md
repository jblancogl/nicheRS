# 01 — SDM / ENM / habitat-suitability modelling: state of the art

Status: research note · Date: 2026-10-15 · Access date for all sources: 2026-10-15 unless stated.
Tags: [FACT] verified in a source read for this note · [ASSUMPTION] · [OPEN] · [REC] engineering recommendation · [UNVERIFIED – prior knowledge].

## 1. Scope

Which correlative modelling engines NicheRS should implement first, and in what order, for
consultancy use (France/Europe, 1..hundreds of species, mostly presence-only records from
GBIF/INPN-type sources plus environmental rasters). Covered: MaxEnt and maxnet, GLM, GAM,
Random Forest (default and down-sampled/balanced), BRT, XGBoost/LightGBM, presence-background
(PB) vs presence-absence (PA) data, ensembles, spatially explicit models, multi-species and joint
SDMs (Hmsc, sjSDM), deep learning (MALPOLON, GeoLifeCLEF, SatBird). Validation metrics are only
touched here; they are the subject of `07-scientific-validation.md`. Mechanistic/process models,
occupancy models with detection, and dynamic range models are out of scope.

## 2. Findings

### 2.1 The data type decides what can be estimated (most important finding)

- [FACT] Presence-only (PO) data allow estimation of *relative*, not absolute, intensity of
  occurrence; the IPP intensity is a more natural object of inference than occurrence probability,
  which is only defined relative to a quadrat (cell) size [5].
- [FACT] Probability of presence is not identifiable from presence-background data alone except in
  special cases: an estimate of prevalence is "necessary" [12]. Methods relying on the strong
  parametric assumption (Lele & Keim type) are fragile [12].
- [FACT] Predictions from PO models "suffice for some applications" but others require occurrence
  probabilities, "unattainable without reliable absence records"; binarising continuous output is
  "seldom clearly justified" and "usually degrades inference" [13].
- [REC] Consequence for the product: with PB data, NicheRS must label outputs "relative habitat
  suitability" (an index), never "probability of presence", and must not present suitable-area
  figures as estimates of occupied area. A PA pathway (when survey data exist) is the only route
  to calibrated probabilities.

### 2.2 MaxEnt is a penalised Poisson point-process model

| Aspect | Content | Tag/ref |
|---|---|---|
| Original formulation | Gibbs distribution over background cells, π(x) ∝ exp(λ·f(x)); maximise L1-regularised log-likelihood of presences | [FACT] [8], [9] |
| Features | linear, quadratic, product, hinge, threshold (categorical) transforms of covariates; complexity controlled by feature classes + regularisation multiplier | [FACT] [6], [9] |
| Point-process equivalence | MaxEnt reduces to a Poisson regression / IPP; differs only in the intercept, which MaxEnt treats as scale-dependent | [FACT] [4] |
| Finite-sample equivalence | IPP and MaxEnt give *exactly* the same density estimate; logistic regression generally does not; "infinitely weighted logistic regression" (IWLR) is exactly equivalent to the IPP | [FACT] [5] |
| Output | raw output ~ relative intensity; v3.4 default changed from logistic to cloglog, justified by the IPP interpretation; points treated as points, not cells | [FACT] [7] |
| cloglog formula | cloglog = 1 − exp(−exp(H)·raw), H = entropy of the fitted distribution; interpretable as probability only under an assumed relationship between intensity and cell prevalence | [UNVERIFIED – prior knowledge] |
| Software | maxent.jar v3.4.4 (tag 2020-11-24), Java, MIT licence, repo last push 2024-01-11; maxnet R 0.1.4 (2021-07-09, MIT) fits MaxEnt via glmnet | [FACT] [40], [41], [7] |
| maxnet algorithm | infinitely-weighted logistic regression with glmnet L1 path | [FACT] [2] (Valavi 2022 methods), [7] |
| Python | elapid 1.0.4 (MIT), MaxEnt-style models | [FACT] [42] |

When MaxEnt is appropriate [REC unless tagged]:
- PO data with a background that represents the environments available in the study area
  (≥ ~10 000 points; Valavi 2022 found AUC converging near 50 000 points for their data and
  explicitly calls the exact number "somewhat arbitrary") [FACT] [2]; background size matters
  because the point-process likelihood is approximated by quadrature [FACT] [10], [11].
- Ranking cells by relative suitability, mapping potential habitat, prioritising surveys.
- Small to moderate n (tens–hundreds of records) when complexity is tuned (feature classes,
  regularisation) with spatially blocked CV [FACT] [37], [38].

When MaxEnt is *not* appropriate, or needs a different framing:
- Absolute probability of occurrence, prevalence, abundance → requires PA/survey data or a
  prevalence estimate [FACT] [12], [13].
- Strongly biased sampling without a bias model: MaxEnt assumes presences are a random sample
  from the species' distribution; bias must be handled with a target-group background or a bias
  covariate [FACT] [33], [34], [6].
- Community-level questions (co-occurrence, richness, composition) → multi-species / joint models
  [FACT] [3], [25].
- Default settings without tuning: defaults "are apparently chosen, even though alternative
  settings are often more appropriate" [FACT] [6]; [37] documents over-complex defaults
  [UNVERIFIED – prior knowledge for the exact finding; paper exists, DOI verified].
- Temporal/spatial transfer far outside calibration conditions (see `07`, MESS/AOA).
- [OPEN] Debate on interpretation of presence-only inference remains [14].

### 2.3 Method-by-method summary

| Method | Mathematical core | Ecological / statistical assumptions | Inputs | Interpretability |
|---|---|---|---|---|
| MaxEnt / maxnet | penalised IPP (log-linear intensity on feature-expanded covariates), L1 | random sample of distribution (or bias modelled); background represents availability; stationarity of response | PO + background | High: response curves, coefficients, permutation importance |
| GLM | logit/log link, linear predictor with polynomials; with PB use IWLR (≡ IPP) or down-weighting | parametric response shape; independence | PA, or PB with weights | Very high |
| GAM | penalised regression splines, smoothness by REML [20] | smooth additive responses | PA or PB with weights | High (smooth terms) |
| RF (default) | bagged deep CART, random feature subsets [19] | none parametric; class balance implicitly assumed | PA (PB only with balancing) | Medium (partial dependence, importance) |
| RF down-sampled ("balanced") | each tree fitted to bootstrap with equal presence/background counts | same; manages class imbalance and class overlap | PB | Medium |
| BRT | stage-wise additive trees, learning rate × tree complexity × bag fraction [17] | none parametric; interactions via tree depth | PA or PB (weights) | Medium |
| XGBoost / LightGBM | regularised second-order gradient boosting [18] | as BRT; many hyperparameters | PA or PB | Medium–low |
| Ensemble | (weighted) mean of tuned members | members individually sound, errors not perfectly correlated | as members | Low–medium |
| Spatial models (spatial random field, smooth of coordinates) | GLMM/GAM with spatial term (e.g. sdmTMB, mgcv, INLA) | residual spatial dependence absorbed by latent field | PA (PO via LGCP) | Medium; random field can absorb bias *and* signal |
| JSDM (Hmsc, sjSDM) | multivariate probit/latent factor models; Bayesian MCMC (Hmsc) or MC-approximated likelihood (sjSDM) | community data with shared sampling units; residual co-occurrence ≠ interaction | PA/abundance site × species matrix | Medium; rich but complex |
| Deep learning (CNN on patches, multi-label) | neural networks on raster patches/remote sensing, multi-label loss | very large data; spatial context learned | millions of PO + PA surveys | Low |

| Method | Predictive performance (evidence) | Compute / scalability | Sampling-bias sensitivity | Validation needs | Production suitability |
|---|---|---|---|---|---|
| MaxEnt / maxnet | High-performance group in Valavi 2022 (225 spp., independent PA test) [FACT] [2]; 2nd in Valavi 2023 under 75 km spatial blocks [FACT] [16]; MaxEnt among "novel methods" outperforming established ones in Elith 2006 [FACT] [1] | Fast, deterministic [FACT] [2]; regularisation path is O(iterations × n_bg × p) [ASSUMPTION]; embarrassingly parallel across species | High; standard remedies exist (target-group background) [33] | Tune feature classes + β with block CV [37], [38] | **High**: deterministic, well-documented, reference implementations to test against |
| GLM | Moderate group, smooth models upper part [FACT] [2]; GLM variants among best in Norberg 2019 for PA communities with many rare species [FACT] [3] | Very fast | High (PB) | Standard | High; robust for small n [FACT] [2] (conclusion) |
| GAM | Moderate [FACT] [2]; GLM.1 ≈ GAM.1 in Norberg [FACT] [3] | Fast–moderate | High (PB) | Standard | High |
| RF default | **Low** group for PB; AUC 0.648 (lowest) in Valavi 2022 [FACT] [2] | Fast, parallel | High; poor on PB due to class imbalance + class overlap [FACT] [15] | Must not be used as-is with PB [REC] | Low for PB |
| RF down-sampled | High group; AUC 0.730, COR 0.216 vs default RF 0.648 [FACT] [2]; "excellent predictive performance" when imbalance managed [FACT] [15]; variant among top under spatial blocks [FACT] [16] | Fast (small balanced bootstraps), trivially parallel | High | Few hyperparameters; not very sensitive to tuning [FACT] [2] | **High** |
| BRT | High group, still among top-ranking since 2006 [FACT] [1], [2] | Moderate (sequential trees; CV for n.trees) | High | Needs learning-rate / n.trees tuning [17] | Medium–high |
| XGBoost | Mid-level on PB; most GLMs outperformed it; authors could not tune fully for 225 spp. [FACT] [2]; may do better on PA (cited study) [FACT] [2] | Fast per model but many hyperparameters; tuning dominates cost | High | Intensive tuning [FACT] [2] | Medium (tuning burden at hundreds of species) |
| LightGBM | No SDM benchmark read for this note [OPEN]; CRAN 4.7.0 (MIT) [FACT] [44b] | Fast | High | As XGBoost | Medium; not first choice |
| Ensemble (tuned members) | **Top** method in Valavi 2022 (mean of Lasso, GAM, MaxEnt, BRT, RF down-sampled) [FACT] [2]; best under random and spatial partitioning [FACT] [16]. biomod with defaults "no better than" average single models [FACT] [2], [22]. In Norberg, ENS.BEST5 worse than best single (HMSC.3) but better than its other members [FACT] [3] | Sum of members [FACT] [2] (Fig. 12) | Inherits members' | CV for each member + ensemble | Medium–high (roadmap) |
| Spatial random-field models | Not in the PB benchmarks read [OPEN] | Moderate–high | Can absorb spatial bias, but confounds bias with real spatial signal [ASSUMPTION] | Spatial CV mandatory | Low initially |
| JSDM Hmsc | HMSC.3 best average performance across 12 measures in Norberg 2019 (PA community data, 5 taxa) [FACT] [3]; sharing information helps rare species, but GLM better on common species [FACT] [3] | MCMC heavy; Hmsc-HPC GPU port gives >1000× for largest datasets [FACT] [27]; sjSDM scalable alternative [FACT] [28]; Hmsc 3.3-7 (2025-08-28, GPL-3) [FACT] [43] | Needs PA from common sampling units | Convergence diagnostics + CV | Low for PO-based product; candidate only for PA community data |
| Deep learning | CNNs exploiting spatial structure improved SDM performance in Deneu 2021 [FACT] [29]; MALPOLON PyTorch framework tested on GeoLifeCLEF 2024 [FACT] [30]; GeoLifeCLEF 2024: >5 M PO + ~90 k PA surveys across Europe [FACT] [31]; SatBird: eBird PA encounter rates + satellite imagery, US + Kenya [FACT] [32] | GPU, large data | Varies; learns observer bias if not modelled [ASSUMPTION] | Strong spatial CV, AOA | Low for v1 (data volume, opacity, auditability) |

### 2.4 Presence-background vs presence-absence

- [FACT] All Valavi 2022 methods use background samples; weighting matters: IWLR (≡ IPP)
  achieved better discrimination than NCEAS-2006 down-weighting for regression models [2], [5].
- [FACT] Background sampled "irrespective of the location of species records", consistent with
  point-process recommendations [2], [11]; pseudo-absence strategies are different objects and
  their choice affects results [39].
- [FACT] PB is imbalanced; some ML methods need weighting or down-sampling [2], [15].
- [FACT] PA data permit calibration, prevalence and absolute probability; Norberg 2019 used PA
  and evaluated calibration and precision, finding most-precise models poorly calibrated
  ("overconfident") [3].
- [FACT] Data integration (PO + PA) in a shared point-process framework is an active route to
  bias correction [34], [35]. [OPEN] for NicheRS: availability of standardised French PA data per
  taxon.

### 2.5 Multi-species: stacked vs joint

- [FACT] Norberg 2019: 33 variants of 15 approaches; no model performed well for all tasks;
  recommend "a small set of models showing complementary performance" chosen by CV for the goal
  of the study [3]. At least one of HMSC.3, GLM.5, SAM.1, GLM.12 performed well in 76 % of
  evaluation tasks [3]. Models with covariate interactions performed worse than without, for the
  data considered [3].
- [REC] For hundreds of species from PO data, NicheRS's unit is a *stacked* SDM (one model per
  species, independently reproducible, parallel). JSDMs need site × species PA matrices, which
  GBIF-type PO data are not.

### 2.6 What "performance" numbers mean

- [FACT] Valavi 2022 AUC range across all methods/species 0.139–0.996, mean 0.709; 9 % of models
  below 0.5 [2]: performance is species- and region-dependent more than method-dependent.
- [FACT] Differences among top-group methods were not statistically significant (Friedman aligned
  rank, Shaffer post hoc) [2].
- [REC] Never surface "MaxEnt is X % accurate" claims in the UI; method rankings are population
  averages, not guarantees for a given species.

## 3. Evidence and references

Primary benchmarks read for this note: Valavi et al. 2022 full text [2]; Norberg et al. 2019 full
text [3]; abstracts of [1], [4], [5], [6], [7], [12], [13], [15], [16], [27]. Software facts from
CRAN (crandb.r-pkg.org), PyPI and GitHub API, all queried 2026-10-15. DOIs resolved via Crossref.

## 4. Decision matrix

Scores are qualitative (H/M/L) syntheses of §2.3; weights follow the brief's priority order. They
are judgement, not measurements [REC].

| Criterion (weight) | MaxEnt/maxnet (IPP) | GLM/GAM (IWLR) | RF down-sampled | BRT | XGBoost/LGBM | Tuned ensemble | Hmsc/JSDM | Deep learning |
|---|---|---|---|---|---|---|---|---|
| Scientific grounding for PB (5) | H (IPP theory) | H | M (empirical) | M | L–M | M | L for PB | L |
| Benchmarked PB performance (4) | H | M | H | H | M | H (best) | n/a (PA) | n/a in PB benchmarks read |
| Reproducibility / determinism (4) | H (deterministic) | H | M (seeded RNG) | M (seeded) | M | M | M (MCMC, seeded) | L (GPU nondeterminism) |
| Interpretability for ecologists (3) | H | H | M | M | L–M | L–M | M | L |
| Tuning burden at 100s species (3) | M (2 hyperparameters) | L | L | M | H | H | H | H |
| Compute per species (2) | L | L | L | M | M | M–H | H | H |
| Reference implementation to validate against (4) | H (maxent.jar, maxnet) | H (R glm/glmnet/mgcv) | H (R ranger/randomForest per [15]) | H (gbm/dismo) | H | M | H (Hmsc) | M |
| Implementation effort in a compiled core (2) | M (coordinate-descent L1 GLM + feature expansion) | L–M | M | M | H (or bind library) | sum | H | H |
| **Verdict** | **v1 engine** | baseline / small n | **v2 engine** | v3 | defer | v3 (after 2 engines) | defer, PA only | defer, research |

## 5. Alternatives

1. Wrap R (maxnet, ranger, biomod2 4.3-4-7 [44]) as a sidecar instead of a native engine:
   fastest to scientific parity, heavier deployment, licence mix (biomod2 GPL-3) [FACT] [44].
2. Call maxent.jar (MIT) via a JVM: exact parity with the reference, but adds a Java runtime and
   file-based I/O [FACT] [40].
3. Native IPP/IWLR engine (maxnet-equivalent) in the core language, validated against R oracles.
4. Ensemble-first (biomod-style): not recommended as v1; defaults underperform [FACT] [2], [22].

## 6. Trade-offs

- Native implementation buys deployment simplicity and performance but carries the burden of
  proving numerical equivalence; equivalence must be demonstrated, not assumed [REC].
- maxnet (glmnet) and maxent.jar (sequential-update solver) do not produce identical
  coefficients; they optimise the same penalised objective up to defaults (regularisation
  scaling, feature construction) [ASSUMPTION – to verify in benchmark doc]. NicheRS must pick
  one as the oracle and document tolerance.
- Down-sampled RF is stochastic → requires seeded RNG and storing seeds for reproducibility.
- Ensembles improve average ranking but reduce interpretability and multiply compute and
  validation effort.

## 7. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Users read cloglog output as probability of presence | Wrong decisions, over-claimed areas | Label "relative suitability"; explain in UI; no "probability" wording for PB [REC] |
| Default MaxEnt settings applied to hundreds of species | Overfit / over-complex models [6], [37] | Automatic tuning grid with spatial block CV, per species [REC] |
| Sampling bias in GBIF-type data | Maps of observer effort | Target-group background, bias covariate, thinning; bias diagnostics (`07`) [33], [34] |
| Native engine silently diverges from reference | Scientific credibility loss | Golden tests vs maxnet / maxent.jar on the Elith et al. disdat benchmark data [2] |
| Default RF used with PB | Poor models [2], [15] | Only expose balanced RF for PB |
| Collinear predictors | Unstable coefficients, poor transfer | Collinearity screening (Valavi used \|r\| > 0.8 exclusion [FACT] [2]; review [45]) |

## 8. Recommendations

1. [REC] **Initial engine: MaxEnt as a penalised IPP (maxnet-equivalent formulation)** — feature
   classes L, Q, H, P (T optional), L1 regularisation path, background ≥ 10 000 points (configurable,
   default chosen by a convergence check à la Valavi 2022 Fig. 1), outputs: raw (relative
   intensity) and cloglog (labelled "relative suitability"). Tuning: grid over feature classes ×
   regularisation multiplier, selected by spatially blocked CV (not AICc alone; ENMeval 2.0 review
   found heavy reliance on AICc and low use of spatial CV [FACT] [38]).
2. [REC] Acceptance test: reproduce maxnet 0.1.4 predictions (rank correlation and max abs
   difference thresholds to be set in the benchmark doc) on the 225-species NCEAS/disdat data
   used by [1], [2]; secondary check vs maxent.jar 3.4.4.
3. [REC] **Second engine (roadmap): down-sampled (balanced) Random Forest** for PB [2], [15]:
   top-group performance, few hyperparameters, fast and parallel, structurally different from
   MaxEnt (useful for ensemble disagreement as an uncertainty signal). Then a **tuned ensemble**
   (MaxEnt + RF-ds + BRT, optionally GAM) as a v3 option [2], [16].
4. [REC] Offer IWLR-GLM/GAM as a "simple model" option for very small n and as a sanity check
   [2] (conclusions), [36].
5. [REC] PA pathway: when standardised survey data exist, fit PA-capable models (GLM/GAM/BRT) and
   only then report calibrated probabilities. JSDMs (Hmsc via Hmsc-HPC, sjSDM) and deep learning
   stay on the research shelf until NicheRS has PA community data and a clear use-case.
6. [REC] Every model run records: algorithm + version, feature classes, regularisation, background
   size and sampling seed, CV design, and fold assignment — feeding ODMAP output (`07`).

## 9. Open questions

- [OPEN] Which oracle (maxnet vs maxent.jar) defines "correct" MaxEnt for NicheRS, and what
  numerical tolerance is acceptable?
- [OPEN] Availability of French/European PA survey data per taxon group (would unlock calibrated
  probabilities and PA validation).
- [OPEN] Bias-correction default: target-group background requires a defensible target group per
  species; who defines it (user vs platform)?
- [OPEN] LightGBM/XGBoost on PB with automated tuning at scale: no benchmark read here supports
  prioritising them.
- [OPEN] Whether spatial random-field models (sdmTMB 1.1.0 [FACT] CRAN) add value over
  bias-corrected MaxEnt for consultancy-scale studies.

## References

[1] Elith J. et al. 2006. Novel methods improve prediction of species' distributions from occurrence data. Ecography 29:129–151. doi:10.1111/j.2006.0906-7590.04596.x (accessed 2026-10-15)
[2] Valavi R., Guillera-Arroita G., Lahoz-Monfort J.J., Elith J. 2022. Predictive performance of presence-only species distribution models: a benchmark study with reproducible code. Ecological Monographs 92(1):e01486. doi:10.1002/ecm.1486 (full text, accessed 2026-10-15)
[3] Norberg A. et al. 2019. A comprehensive evaluation of predictive performance of 33 species distribution models at species and community levels. Ecological Monographs 89(3):e01370. doi:10.1002/ecm.1370 (full text, accessed 2026-10-15)
[4] Renner I.W., Warton D.I. 2013. Equivalence of MAXENT and Poisson point process models for species distribution modeling in ecology. Biometrics 69(1):274–281. doi:10.1111/j.1541-0420.2012.01824.x (abstract, accessed 2026-10-15)
[5] Fithian W., Hastie T. 2013. Finite-sample equivalence in statistical models for presence-only data. Annals of Applied Statistics 7:1917–1939. doi:10.1214/13-AOAS667 (accessed 2026-10-15)
[6] Merow C., Smith M.J., Silander J.A. 2013. A practical guide to MaxEnt for modeling species' distributions: what it does, and why inputs and settings matter. Ecography 36:1058–1069. doi:10.1111/j.1600-0587.2013.07872.x (accessed 2026-10-15)
[7] Phillips S.J., Anderson R.P., Dudík M., Schapire R.E., Blair M.E. 2017. Opening the black box: an open-source release of Maxent. Ecography 40:887–893. doi:10.1111/ecog.03049 (accessed 2026-10-15)
[8] Phillips S.J., Anderson R.P., Schapire R.E. 2006. Maximum entropy modeling of species geographic distributions. Ecological Modelling 190:231–259. doi:10.1016/j.ecolmodel.2005.03.026 (accessed 2026-10-15)
[9] Elith J. et al. 2011. A statistical explanation of MaxEnt for ecologists. Diversity and Distributions 17:43–57. doi:10.1111/j.1472-4642.2010.00725.x (accessed 2026-10-15)
[10] Warton D.I., Shepherd L.C. 2010. Poisson point process models solve the "pseudo-absence problem" for presence-only data in ecology. Annals of Applied Statistics 4:1383–1402. doi:10.1214/10-AOAS331 (accessed 2026-10-15)
[11] Renner I.W. et al. 2015. Point process models for presence-only analysis. Methods in Ecology and Evolution 6:366–379. doi:10.1111/2041-210X.12352 (accessed 2026-10-15)
[12] Phillips S.J., Elith J. 2013. On estimating probability of presence from use–availability or presence–background data. Ecology 94:1409–1419. doi:10.1890/12-1520.1 (accessed 2026-10-15)
[13] Guillera-Arroita G. et al. 2015. Is my species distribution model fit for purpose? Matching data and models to applications. Global Ecology and Biogeography 24:276–292. doi:10.1111/geb.12268 (accessed 2026-10-15)
[14] Hastie T., Fithian W. 2013. Inference from presence-only data; the ongoing controversy. Ecography 36:864–867. doi:10.1111/j.1600-0587.2013.00321.x (accessed 2026-10-15)
[15] Valavi R., Elith J., Lahoz-Monfort J.J., Guillera-Arroita G. 2021. Modelling species presence-only data with random forests. Ecography 44:1731–1742. doi:10.1111/ecog.05615 (accessed 2026-10-15)
[16] Valavi R., Elith J., Lahoz-Monfort J.J., Guillera-Arroita G. 2023. Flexible species distribution modelling methods perform well on spatially separated testing data. Global Ecology and Biogeography 32:369–383. doi:10.1111/geb.13639 (accessed 2026-10-15)
[17] Elith J., Leathwick J.R., Hastie T. 2008. A working guide to boosted regression trees. Journal of Animal Ecology 77:802–813. doi:10.1111/j.1365-2656.2008.01390.x (accessed 2026-10-15)
[18] Chen T., Guestrin C. 2016. XGBoost: a scalable tree boosting system. Proc. 22nd ACM SIGKDD, 785–794. doi:10.1145/2939672.2939785 (accessed 2026-10-15)
[19] Breiman L. 2001. Random forests. Machine Learning 45:5–32. doi:10.1023/A:1010933404324 (accessed 2026-10-15)
[20] Wood S.N. 2011. Fast stable restricted maximum likelihood and marginal likelihood estimation of semiparametric generalized linear models. JRSS-B 73:3–36. doi:10.1111/j.1467-9868.2010.00749.x (accessed 2026-10-15)
[21] Thuiller W. et al. 2009. BIOMOD – a platform for ensemble forecasting of species distributions. Ecography 32:369–373. doi:10.1111/j.1600-0587.2008.05742.x (accessed 2026-10-15)
[22] Hao T. et al. 2019. A review of evidence about use and performance of species distribution modelling ensembles like BIOMOD. Diversity and Distributions 25:839–852. doi:10.1111/ddi.12892 (accessed 2026-10-15)
[23] Ovaskainen O. et al. 2017. How to make more out of community data? A conceptual framework and its implementation as models and software. Ecology Letters 20:561–576. doi:10.1111/ele.12757 (accessed 2026-10-15)
[24] Tikhonov G. et al. 2020. Joint species distribution modelling with the R-package Hmsc. Methods in Ecology and Evolution 11:442–447. doi:10.1111/2041-210X.13345 (accessed 2026-10-15)
[25] Warton D.I. et al. 2015. So many variables: joint modeling in community ecology. Trends in Ecology & Evolution 30:766–779. doi:10.1016/j.tree.2015.09.007 (accessed 2026-10-15)
[26] Pollock L.J. et al. 2014. Understanding co-occurrence by modelling species simultaneously with a Joint Species Distribution Model (JSDM). Methods in Ecology and Evolution 5:397–406. doi:10.1111/2041-210X.12180 (accessed 2026-10-15)
[27] Rahman A.U., Tikhonov G., Oksanen J., Rossi T., Ovaskainen O. 2024. Accelerating joint species distribution modelling with Hmsc-HPC by GPU porting. PLoS Computational Biology 20(9). doi:10.1371/journal.pcbi.1011914 (accessed 2026-10-15)
[28] Pichler M., Hartig F. 2021. A new joint species distribution model for faster and more accurate inference of species associations from big community data. Methods in Ecology and Evolution 12:2159–2173. doi:10.1111/2041-210X.13687 (accessed 2026-10-15)
[29] Deneu B. et al. 2021. Convolutional neural networks improve species distribution modelling by capturing the spatial structure of the environment. PLoS Computational Biology 17:e1008856. doi:10.1371/journal.pcbi.1008856 (accessed 2026-10-15)
[30] Larcher T., Picek L., Deneu B., Lorieul T., Servajean M., Joly A. 2024. MALPOLON: A framework for deep species distribution modeling. arXiv:2409.18102 (BMVC 2024 workshops); PyPI malpolon 2.1.2 (2024-11-22). https://arxiv.org/abs/2409.18102 (accessed 2026-10-15)
[31] Picek L., Botella C. et al. 2024. Overview of GeoLifeCLEF 2024: species composition prediction with high spatial resolution at continental scale using remote sensing. CLEF 2024 Working Notes, CEUR-WS Vol-3740, 1966–1977. https://ceur-ws.org/Vol-3740/paper-186.pdf (accessed 2026-10-15)
[32] Teng M. et al. 2023. SatBird: a dataset for bird species distribution modeling using remote sensing and citizen science data. NeurIPS 2023 Datasets and Benchmarks. arXiv:2311.00936 (accessed 2026-10-15)
[33] Phillips S.J. et al. 2009. Sample selection bias and presence-only distribution models: implications for background and pseudo-absence data. Ecological Applications 19:181–197. doi:10.1890/07-2153.1 (accessed 2026-10-15)
[34] Fithian W., Elith J., Hastie T., Keith D.A. 2015. Bias correction in species distribution models: pooling survey and collection data for multiple species. Methods in Ecology and Evolution 6:424–438. doi:10.1111/2041-210X.12242 (accessed 2026-10-15)
[35] Isaac N.J.B. et al. 2020. Data integration for large-scale models of species distributions. Trends in Ecology & Evolution 35:56–67. doi:10.1016/j.tree.2019.08.006 (accessed 2026-10-15)
[36] Merow C. et al. 2014. What do we gain from simplicity versus complexity in species distribution models? Ecography 37:1267–1281. doi:10.1111/ecog.00845 (accessed 2026-10-15)
[37] Radosavljevic A., Anderson R.P. 2014. Making better Maxent models of species distributions: complexity, overfitting and evaluation. Journal of Biogeography 41:629–643. doi:10.1111/jbi.12227 (accessed 2026-10-15)
[38] Kass J.M. et al. 2021. ENMeval 2.0: redesigned for customizable and reproducible modeling of species' niches and distributions. Methods in Ecology and Evolution 12:1602–1608. doi:10.1111/2041-210X.13628; CRAN ENMeval 2.0.6 (2026-09-11, GPL-3) (accessed 2026-10-15)
[39] Barbet-Massin M. et al. 2012. Selecting pseudo-absences for species distribution models: how, where and how many? Methods in Ecology and Evolution 3:327–338. doi:10.1111/j.2041-210X.2011.00172.x (accessed 2026-10-15)
[40] Maxent software, github.com/mrmaxent/Maxent, release v3.4.4 (2020-11-24), MIT licence, Java (GitHub API, accessed 2026-10-15)
[41] maxnet R package 0.1.4 (CRAN, 2021-07-09, MIT). https://cran.r-project.org/package=maxnet (accessed 2026-10-15)
[42] Anderson C.B. 2023. elapid: species distribution modeling tools for Python. JOSS 8:4930. doi:10.21105/joss.04930; PyPI elapid 1.0.4 (MIT) (accessed 2026-10-15)
[43] Hmsc R package 3.3-7 (CRAN, 2025-08-28, GPL-3). https://cran.r-project.org/package=Hmsc (accessed 2026-10-15)
[44] biomod2 R package 4.3-4-7 (CRAN, 2026-10-08, GPL-3). https://cran.r-project.org/package=biomod2 (accessed 2026-10-15)
[44b] lightgbm R package 4.7.0 (CRAN, 2026-07-18, MIT); xgboost R 3.2.1.1 (CRAN, 2026-03-18, Apache-2.0); ranger 0.18.0 (CRAN, 2026-01-16, GPL-3); sdmTMB 1.1.0 (CRAN, 2026-07-03, GPL-3); mgcv 1.9-4 (CRAN, 2025-11-07, GPL ≥2) (accessed 2026-10-15)
[45] Dormann C.F. et al. 2013. Collinearity: a review of methods to deal with it and a simulation study evaluating their performance. Ecography 36:27–46. doi:10.1111/j.1600-0587.2012.07348.x (accessed 2026-10-15)
[46] Elith J., Leathwick J.R. 2009. Species distribution models: ecological explanation and prediction across space and time. Annual Review of Ecology, Evolution, and Systematics 40:677–697. doi:10.1146/annurev.ecolsys.110308.120159 (accessed 2026-10-15)
[47] Qiao H., Soberón J., Peterson A.T. 2015. No silver bullets in correlative ecological niche modelling: insights from testing among many potential algorithms for niche estimation. Methods in Ecology and Evolution 6:1126–1136. doi:10.1111/2041-210X.12397 (accessed 2026-10-15)
[48] Guisan A., Thuiller W. 2005. Predicting species distribution: offering more than simple habitat models. Ecology Letters 8:993–1009. doi:10.1111/j.1461-0248.2005.00792.x (accessed 2026-10-15)
