# ADR-0017 — Down-sampled random forest as the second modelling engine

- Status: **Proposed — pending review by Natural Solutions ecologists**
- Refines: ADR-0002 (MaxEnt first; down-sampled RF second). Related: ADR-0007 (output
  semantics), ADR-0009 (defaults), ADR-0201 (no abstraction without two implementations),
  ADR-0103 (heavy work off the async runtime), ADR-0203 (execution manifest).
- Evidence: `docs/research/01-sdm-state-of-the-art.md` §2.3 and refs [2], [15], [16].
- Implementation: `crates/niche-rf` (Phase 1b of the Science v1 programme); app integration in
  `crates/niche-app/src/fit.rs` (Phase 6, see [Integration](#integration-phase-6-2026-10-20)).

## Context

ADR-0002 chose a down-sampled (balanced) random forest as the second engine. The evidence:

- Valavi et al. 2022 (*Ecol. Monogr.* 92:e01486; 225 species, independent presence–absence
  test data) placed down-sampled RF in the top performance group for presence–background (PB)
  data (mean AUC 0.730), while the **default** random forest was in the lowest group (AUC 0.648,
  the lowest of all methods). Differences within the top group were not significant.
- Valavi et al. 2021 (*Ecography* 44:1731–1742) explain why: PB data are strongly imbalanced
  (few presences, many background points) and the classes overlap (the background contains
  presences). Balancing each tree's sample ("down-sampling", Chen, Liaw & Breiman 2004) restores
  "excellent predictive performance"; default RF must **not** be used with PB data.
- Valavi et al. 2023 (*Glob. Ecol. Biogeogr.* 32:369–383) found the down-sampled variant among
  the top methods also under spatially separated testing.
- The ensemble of tuned members (including MaxEnt and down-sampled RF) was the top method in
  Valavi 2022; a second engine with a different inductive bias also exposes algorithm
  disagreement, which is useful evidence for users.

Unlike MaxEnt (ADR-0001) there is no reference implementation we can match bit for bit: R's
`randomForest` uses R's RNG and breaks vote ties at random.

## Decision

A new pure crate `niche-rf` (no I/O, threads, async or third-party numerics; CART implemented
in-house; depends only on `niche-eval` for the frozen PRNG and AUC, `serde`, `thiserror`).

Algorithm (each choice documented with its source in the crate's doc comments):

| Item | Choice |
|---|---|
| Input | column-major presence and background matrices (`RfData`, like `SwdData`); all values finite |
| Per-tree sample | bootstrap of the `n_p` presences (with replacement) + `n_p` background points drawn with replacement — `randomForest(sampsize = c(n_p, n_p), replace = TRUE)` as in Valavi 2021 |
| Tree | CART classification, Gini, midpoints between consecutive distinct sorted values, `x <= t` goes left |
| `mtry` | `floor(sqrt(p))` (min 1), drawn without replacement per node; if none of the drawn variables splits, the node is a leaf (as `randomForest`) |
| Size | fully grown (`min_node_size = 1`), optional `max_depth` |
| Trees | `ntree = 1000` |
| Categorical predictors | treated as ordered numeric codes (simplification; never wrong, may need more splits) |
| Prediction | fraction of trees voting presence (`predict(type = "prob")`); a leaf with exactly ½ presences casts ½ vote (deterministic expectation of R's random tie-break) |
| Output label | **relative suitability index in [0, 1], not a calibrated probability** (the 1:1 balance fixes the apparent prevalence; ADR-0007) |
| Determinism | tree `i` uses an RNG stream derived only from `(seed, i)` (`niche_eval::rng::Rng`), so trees can be fitted in parallel by the caller with bit-identical results |
| OOB | optional out-of-bag scores and AUC, replaying each tree's bootstrap from its seed; diagnostic only (random, not spatial) |
| Importance | permutation importance as mean decrease in presence–background AUC, pooled presence + background shuffles, seeded; also normalised to percent like MaxEnt's |
| Response | partial dependence (Friedman 2001) over an even grid between the training min and max, averaged over a seeded subsample of the pooled rows |
| Storage | serde (flat node arrays per tree, format version 1); deserialisation validates structure; JSON must be parsed with exact float round-tripping |
| Cancellation | `&dyn Fn() -> bool` polled between trees (no new trait, ADR-0201) |

These defaults become part of ADR-0009's scope when the engine is exposed; changing them later
needs a new ADR.

## Consequences

- **Opt-in only.** MaxEnt remains the default engine; RF and the ensemble are offered as
  explicit algorithm choices (Phase 6). Valavi's results are population averages, not
  guarantees for a given species (doc 01 §2.6).
- **Validation without golden reference.** Correctness is tested by synthetic truth recovery
  and invariants (`crates/niche-rf/tests/forest.rs`): deterministic forests for a seed and
  independence from tree fitting order; held-out AUC on a known Gaussian niche (2 informative +
  5 noise variables) close to the AUC of the true suitability (0.905 vs 0.915 in the test
  run); informative variables ranked first by permutation importance; partial-dependence peaks
  within 0.15 of the true optima; constant columns, 10 presences, input errors, cancellation
  and serialisation round trip (bit-identical predictions; corrupted trees rejected).
- **Measured cost** (release, one thread, 8-core laptop, 2026-10-20): 1000 trees on 500
  presences + 10 000 background points × 7 variables fit in 0.50 s (314 k nodes, 6.6 MB JSON);
  predicting 10 000 rows takes 0.57 s (≈ 57 µs per row per 1000 trees) — prediction, not fitting,
  dominates for raster maps (≈ 1 min per million cells single-threaded), so the app must
  parallelise prediction by tile. Partial dependence at 50 grid points × 200 rows × 7 variables
  takes 4.7 s.
- **Stochastic.** Results depend on the seed; the seed and resolved settings (stored in the
  serialised forest) must go into the execution manifest (ADR-0203).
- **Model size.** Fully grown forests are large (several MB); store compressed or as binary
  if checkpoint size becomes a problem.
- Output is labelled "relative suitability" everywhere; never "probability".

## Proposal for app integration (Phase 6, to be decided then)

- **Tuning:** none by default — Valavi 2022 found down-sampled RF "not very sensitive to
  tuning". Optionally a small grid `mtry ∈ {1, floor(sqrt(p)), floor(p/2)}` evaluated with the
  same spatial block CV and selection rule as MaxEnt (ADR-0009), if ecologists want it.
- **CV evaluation:** the same folds, Boyce index, omission at P10 and null-model comparison as
  MaxEnt, so both engines are judged identically; OOB AUC is shown only as a diagnostic.
- **Ensemble (proposal):** unweighted mean of **rank-normalised** predictions of MaxEnt
  (cloglog) and RF, since the two scales are not comparable (rank normalisation over the study
  area's background cells), with the per-cell disagreement between members shown as a separate
  uncertainty layer. Weighting by CV performance is deferred.
- **Execution:** fit trees in parallel with `Trainer::fit_tree` on the blocking pool / Rayon
  in `niche-app` (ADR-0103), predict tiles in parallel, poll cancellation between batches.
- **Explanations:** permutation importance and partial-dependence curves feed the same UI
  components as MaxEnt's importance and response curves, labelled with the method used
  (partial dependence averages over the other variables; MaxEnt curves hold them at the mean).

## Integration (Phase 6, 2026-10-20)

Decisions taken when the engine was exposed (`model.algorithm`), still *Proposed*. MaxEnt stays
the default and MaxEnt studies are bit-identical to the builds before this phase (`niche demo`
with the build of `9a88072` and with this phase: identical rasters, lambdas, curves,
occurrences, `result.json`, `report.html`; same `spec_sha256`).

| Item | Decision | Why |
|---|---|---|
| API | `model.algorithm` = `maxent` (default, omitted from the JSON) \| `rf` \| `ensemble`; `rf_trees` (100–2000, absent = 500); `rf_tune_mtry` (bool, absent = off); `rf_*` refused with `maxent` | old specs keep their hash; invalid combinations fail at submission |
| Trees | **500** by default (Valavi et al. used 1000) | measured below: prediction, not fitting, dominates, and it is linear in trees (France-sized grid 4.7 s vs 10.0 s per forest, × 5 forests per species with 4 folds). The Monte-Carlo SD of a vote share over 500 trees is ≤ √(0.25/500) = 0.022, small next to the fold SD (0.08 on the demo) |
| `mtry` | `floor(sqrt(p))`; optional second candidate `floor(p/2)` (`rf_tune_mtry`), skipped when equal | Valavi 2022 found down-sampled RF insensitive to tuning; one optional alternative doubles the forest's CV cost at most and is selected by the same sequential rule as MaxEnt (lowest mean OR10, then highest mean Boyce) under the same folds. `mtry = 1` was not added: with the few predictors kept by the collinearity filter it often equals √p |
| CV | same partitions, metrics, selection rule, null models (refitting the selected setting, forests included) and evidence rules as MaxEnt | both engines are judged identically |
| Thresholds of RF | P10, MTP and max SSS use the forest's **out-of-bag** scores at the training presences (`Forest::oob`; R's `predict()` without new data); map and background values are ordinary predictions | first integration used in-bag predictions: fully grown trees score their own presences ≈ 1, so the P10 threshold was ≈ 1 and held-out OR10 was 0.99 on the demo; with OOB scores it is 0.02–0.04 |
| Ensemble | members tuned and selected separately; each member's value replaced by its **mid-rank percentile among its own predictions at the background points**, equal-weight mean of the two ranks | Valavi 2022 averaged tuned members after rescaling to 0–1 (checked in the paper's Methods). Min–max rescaling depends on extremes and leaves the members on different shapes (cloglog vs vote share); ranks are invariant to any monotone transform of either member. Consequence: the ensemble value is a relative index ≈ "share of the study area's background the two models rank lower", not comparable between studies |
| Ensemble evaluation | fold by fold, the ensemble of the two members' fold models (rank references = the fold's training background) is tested on the held-out data; its metrics are the study's metrics and drive the evidence level; members' own means in `validation.members`; thresholds on the ensemble values with the same rules; uncertainty = SD across the fold ensembles | the reported metrics describe the delivered map |
| Explanations | RF: permutation importance (3 repeats, presences + ≤ 2000 background points, normalised to 100) as `contributions[].rf_importance` (and `percent` for `rf`); partial dependence (51 points, 300 rows, variables in parallel through the new `niche_rf::partial_dependence_of`) as `partial_dependence`, labelled "partial dependence", never "response curve". Ensemble: both members' | |
| AOA weights | RF permutation importance; ensemble: mean of the two members' normalised importances | ADR-0014 uses the model's permutation importance |
| Seeds | one forest seed per species: (study seed XOR species key × 0x9E3779B9 XOR an RF stream) mod 2^53 (exact in JSON doubles); importance and partial dependence use further fixed streams; recorded as `selected.rf_seed` | reproducibility (ADR-0203) |
| Parallelism | trees with `Trainer::fit_tree` in a Rayon `par_iter`, prediction by cell in parallel (as MaxEnt), variables of partial dependence in parallel; `niche-rf` stays sequential and pure | ADR-0103 |
| Checkpoints (ADR-0104) | RF fold checkpoints store the **metrics only** (`cv-rf-{combo}-{fold}.json`, a few hundred bytes); after a resume, the selected setting's fold forests are **refitted** | a forest is a deterministic function of the inputs and seed, and fitting is cheap (0.04 s for 500 trees on 500 + 10 000 points, 8 threads) while a stored forest costs ≈ 3 MB per fold at that size (6 MB at 1000 trees). The resume test interrupts an ensemble study with null models and checks every output byte-identical |
| Outputs | `model.rf.json` (serde JSON of `Forest`, format version 1; 1.8 MB on the demo at 500 trees, ≈ 3 MB for 500 presences); `partial_dependence.csv`; `model.lambdas` only when a MaxEnt model exists (the ensemble's member is labelled as such); file descriptions, reports (md/html: method text, members table, references Breiman 2001, Chen, Liaw & Breiman 2004, Valavi et al. 2021, Valavi et al. 2022, Friedman 2001) and manifest (`methods.random_forest`, `methods.ensemble`) say that the output is a relative suitability index, not a probability | ADR-0007, ADR-0203 |

**Measured cost** (release, 8 threads, 2026-10-20; `cargo test --release -p niche-app --test
rf_engine -- --ignored --nocapture`, synthetic 500 presences + 10 000 background × 7 predictors):

| Trees | Fit (parallel) | Nodes | JSON | Predict 1 M cells |
|---|---|---|---|---|
| 500 | 0.04 s | 143 814 | 3.0 MB | 4.65 s |
| 1000 | 0.08 s | 288 040 | 6.1 MB | 9.96 s |

A species over metropolitan France at 30″ (≈ 1 M cells) with 4 spatial blocks predicts 5
forests: ≈ 23 s at 500 trees, ≈ 50 s at 1000; the ensemble adds the MaxEnt cost.

Synthetic demo (`niche run` of the demo spec, 79 344 cells, 152 presences, 10 000 background,
auto MaxEnt grid of 25 settings, 4 spatial blocks; wall time incl. outputs): MaxEnt 5.0 s, RF
2.7 s, ensemble 7.9 s. Held-out metrics (spatial blocks): MaxEnt Boyce 0.53, OR10 0.00, AUC
0.67; RF 0.47, 0.04, 0.69; ensemble 0.52, 0.02, 0.68 (all *Indicative*). On the smaller test
setting (2 000 background, factor 2) RF is clearly weaker than MaxEnt under spatial blocks (Boyce
0.22 vs 0.57) and comparable under random folds: trees cannot extrapolate into the conditions of
a held-out block. The RF Boyce index changed from 0.51 to 0.47 between two forest seeds on the
demo: seed sensitivity of the metrics is not negligible at 500 trees and is not yet reported.

Not done: a per-cell member-disagreement layer for the ensemble; weighting members by CV
performance; categorical subset splits; reporting seed sensitivity; comparison on real
species and the ecologists' review.

## Open questions for reviewers

1. Is the 1:1 balance appropriate for all taxa, or should the background:presence ratio be
   configurable (Valavi 2021 tested only equal sizes as the recommended setting)?
2. Should categorical predictors (e.g. land-cover class) get true subset splits before
   release?
3. Ensemble method and whether it should become the default once both engines are validated
   on Natural Solutions case studies.
4. Default forest size (500 trees chosen on cost) and whether the optional `mtry` grid is
   useful.
5. Out-of-bag scores for the thresholds of the forest (a choice made at integration).
