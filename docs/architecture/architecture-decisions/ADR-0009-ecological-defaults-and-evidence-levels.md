# ADR-0009 — Ecological defaults, model selection and evidence levels

- Status: Accepted (2026-10-15). Decided by the NicheRS lead acting as the project ecologist, on the
  evidence below; to be reviewed by Natural Solutions ecologists after the first client projects.
- Supersedes the "open questions" 1–2 of `docs/executive-summary.md`.
- Implementation: `crates/niche-app/src/{env.rs,fit.rs}`, `crates/niche-data/src/{gbif.rs,chelsa.rs}`.
- Opt-in extensions (defaults unchanged): ADR-0014 (null models, spatial grid blocks, area of
  applicability), proposed; ADR-0013 (target-group background, effort layer, bias diagnostic,
  distance thinning), proposed.
- Amended by ADR-0011 (proposed, implemented 2026-10-20): cleaning rule for coordinates
  generalised by the publisher (§1).

Each default below states the decision, the evidence, and the known cost. Users can change the
model and validation defaults in Expert mode; they cannot switch off the cleaning steps (they
remain visible as exclusion counts).

## 1. Occurrence cleaning (applied in this order; first failing rule is reported)

| Rule | Default | Evidence / rationale |
|---|---|---|
| Fossil and living (captive/cultivated) specimens | excluded | Not wild populations in their environmental niche (Zizka et al. 2019, CoordinateCleaner). |
| Licence | CC0 + CC BY only | CC BY-NC records are not usable in commercial consultancy deliverables without a legal reading (ADR-0006). |
| Undated records | excluded | Cannot be matched to the climate baseline. |
| Year | ≥ 1981 | CHELSA v2.1 climatology covers 1981–2010; older records risk niche–climate mismatch, especially for range-shifting species. |
| Coordinates generalised by the publisher (ADR-0011) | excluded when the uncertainty is missing or > the limit below; kept and counted otherwise | Blurred points of sensitive species (Darwin Core `dataGeneralizations` / `informationWithheld`) are only usable when their stated precision is within the limit (Chapman 2020). Only the GBIF search API reports the flag; downloads and the mirror cannot, and a warning says so. |
| Coordinate uncertainty | ≤ 1000 m (missing values kept) | Uncertainty larger than the ~1 km cell degrades models (Graham et al. 2008; Moudrý & Šímová 2012). Records without the field are kept because excluding them would remove a large share of valid data; their count is visible. |
| Outside study area, at sea / in large lakes | excluded | Terrestrial niche; CHELSA has values over the ocean (ADR-0010). |
| Duplicates | one record per grid cell | Equivalent to MaxEnt `removeduplicates` on grids; reduces pseudo-replication and part of the sampling-density bias. |
| Distance thinning (opt-in, ADR-0013) | off | When set (0–100 km), presence cells closer than the distance to a retained one are removed (spThin). |

The organisation's own records (ADR-0108, opt-in) pass the same rules in the same order, with two
additions that GBIF records never reach: an *absence* rule first (an absence is never a presence)
and the licence rule always accepting the organisation's own data.

GBIF retrieval: when more records exist than the per-species cap (default 20 000), a spatially
stratified subsample is taken (quadtree of count queries, proportional allocation), instead of the
first N records of the API order, which are not spatially random. The UI and report say so and
recommend a GBIF download with a DOI for final reports.

Taxon rank: models are fitted per species (or subspecies/variety/form). If the selected GBIF key
is a higher rank (genus, family…), records of several species with different niches are pooled,
violating the single-niche assumption; the data review and results show a warning and the evidence
level is capped at *Exploratory* (added 2026-10-16 after a genus-level run in production).

## 2. Predictors

- Default CHELSA v2.1 set: BIO1, BIO4, BIO5, BIO6, BIO12, BIO14, BIO15 — energy, water, their
  seasonality and extremes, the axes most often limiting terrestrial distributions in Europe.
- BIO8, BIO9, BIO18, BIO19 are flagged and not selected by default: they combine temperature and
  precipitation of specific quarters and show spatial artefacts (Booth 2022).
- Collinearity: greedy filter keeping variables in the user's priority order, dropping any with
  |Pearson r| > 0.7 with a kept variable, computed on the background sample (Dormann et al. 2013).
  Dropped variables and their correlation partner are reported.

## 3. Background

10 000 random land cells within the study area, seeded (Phillips & Dudík 2008; Barbet-Massin et
al. 2012). No bias correction by default: target-group background requires a defensible target
group per species, which is a project decision. Since 2026-10-20 it is available as an opt-in
(ADR-0013, proposed): background drawn among cells with records of a GBIF class in the regional
mirror's effort grids, with a fallback to this random background when unavailable.

## 4. Model tuning and selection

- Grid: feature classes {L, LQ, H, LQH, LQHP} × regularization multiplier {0.5, 1, 2, 3, 4}
  (25 settings), MaxEnt-compatible engine, `autofeature` off (classes are explicit).
- Partition: spatial blocks (4 groups split at median latitude then median longitude of the
  presences; background assigned by the same lines; Muscarella et al. 2014). Random k-fold is
  offered but flagged as optimistic (Roberts et al. 2017).
- Selection (sequential criteria): lowest mean 10th-percentile omission rate on held-out blocks,
  then highest mean continuous Boyce index, then fewest coefficients (Radosavljevic & Anderson
  2014; Kass et al. 2021). AUC is reported but not used for selection: with presence-background
  data and geographically restricted test background it mostly reflects the background extent
  (Lobo et al. 2008; Jiménez-Valverde 2012).
- Final model refitted on all presences and background with the selected setting.

Known cost: OR10-first selection favours smoother, more regularized models; in the Brittany test
run (*Lacerta bilineata*) it picked H/β=3 (OR10 0.12, CBI 0.40) over LQH/β=2 (OR10 0.19, CBI 0.62).
We accept this bias towards transferability over apparent discrimination; Expert mode shows the
full tuning table so analysts can override with a fixed setting.

## 5. Outputs and interpretation

- Output: cloglog relative suitability (Phillips et al. 2017), labelled "relative habitat
  suitability (0–1) — not a probability of presence" (ADR-0007).
- Uncertainty map: per-cell standard deviation of suitability across the cross-validation models
  of the selected setting (data-resampling uncertainty). Not a confidence interval.
- Extrapolation: MESS against background + presence environments (Elith et al. 2010); share of
  the study area with MESS < 0 is reported and the hatched overlay is on by default in the UI.
- Thresholds: three rules reported as a band — minimum training presence (MTP), 10th-percentile
  training presence (P10, **default for headline area**), maximum sensitivity + specificity
  (Liu et al. 2013). P10 is the default because it tolerates the ~10 % of records that are
  erroneous or in marginal habitat in opportunistic data while remaining conservative; MTP is
  maximally inclusive and sensitive to single outliers.
- Areas: sum of exact WGS84 ellipsoidal cell areas ≥ threshold; worded "area of suitable
  conditions", never "range" or "occupied area".

## 6. Evidence levels (rule-based, no percentages)

| Level | All criteria |
|---|---|
| Insufficient | < 10 usable presence cells → no model is fitted (van Proosdij et al. 2016: 14–25 minimum). |
| Exploratory | model fitted but at least one *Indicative* criterion fails. |
| Indicative | ≥ 25 presence cells; mean CBI ≥ 0.40; mean OR10 ≤ 0.25; extrapolated share ≤ 25 %. |
| Supported | ≥ 50 presence cells; mean CBI ≥ 0.70; mean OR10 ≤ 0.15; extrapolated share ≤ 10 %; spatially blocked validation. |

Rationale: 25 and 50 presences bracket the sample sizes at which MaxEnt performance stabilizes
(Wisz et al. 2008; van Proosdij et al. 2016). OR10 is expected to be ≈ 0.10 for a model that is
not overfit; values well above indicate overfitting or poor transfer (Radosavljevic & Anderson
2014). CBI > 0 means better than random; values near 1 indicate predictions consistent with the
presence distribution (Hirzel et al. 2006). Cut-offs 0.40/0.70 are judgement calls, recorded here
so they can be changed by ADR, not silently.

Every result also states: "No independent validation data: evidence comes from cross-validation
of the same, possibly biased, records." No level is higher than *Supported*; a "Validated" level is
reserved for future independent presence-absence evaluation.

Amended 2026-10-20 (pointer only): the *Validated* level and its criteria are proposed in ADR-0108
(the organisation's presence–absence data held out from training, ≥ 30 presence and ≥ 30 absence
sites after a leakage rule, PA AUC ≥ 0.75, TSS ≥ 0.40, and *Supported*). Without such data the
levels and the sentence above are unchanged.

## Consequences

- Results are conservative by construction; many opportunistic-data species will be
  *Indicative* or *Exploratory*. That is the honest outcome, not a defect.
- Defaults are versioned in the execution manifest (`methods`, `spec`), so any later change is
  traceable per run.
