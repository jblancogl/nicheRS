# 02 — MaxEnt (Java) internals, for a behaviour-equivalent Rust engine

Status: research note · Date: 2026-10-15 · Access date for all sources: 2026-10-15.
Tags: [FACT] verified in a source read for this note (Java source, jar execution, paper) ·
[ASSUMPTION] · [OPEN] · [REC] engineering recommendation · [UNVERIFIED – prior knowledge].

Companion document: `docs/architecture/mathematical-specification.md` (testable spec, SPEC-xxx items).
Source tree read: `reference/vendor/Maxent` (git clone of github.com/mrmaxent/Maxent, HEAD
`963d0114`, 2024-01-11; `density/Utils.java:35` → `version = "3.4.4"`). All `file:line` references
below are relative to `reference/vendor/Maxent/density/` unless stated.

---

## 1. Scope

Exactly what `maxent.jar` 3.4.4 computes when fitting and projecting a model, at the level needed
to re-implement it in Rust and compare outputs numerically (coefficients, normalisers, predictions,
iteration path) rather than "the same objective". Covers: formulation and its statistical
interpretations, feature construction, regularization, background/sample handling, the
sequential-update optimizer, output transforms, clamping/extrapolation, variable importance,
`.lambdas` serialization, headless CLI, performance, and the differences with maxnet (R) and elapid
(Python). Out of scope: GUI, HTML report layout, bias-file/FactorBiasOut internals beyond what
touches the core math, NCEAS format, polyhedral features (hidden flag).

## 2. Findings

### 2.0 Provenance, licence, version, and an empirical cross-check

| Item | Finding | Tag | Ref |
|---|---|---|---|
| Latest release | v3.4.4, GitHub release 2020-11-24; jar in `ArchivedReleases/3.4.4/maxent.jar` (sha256 `4c856e55…fee5e`); `java -cp maxent.jar density.MaxEnt printversion` → `MaxEnt version 3.4.4` | [FACT] | [1][2] |
| Licence | MIT (`LICENSE.md`, © 2016 Phillips, Dudík, Schapire). Release readmes: 3.3.3k (2011) has the old "educational and research" terms; 3.4.0/3.4.1 readme says "distributed under the MIT License". So **MIT since 3.4.0 (Dec 2016)**. Third-party: GNU getopt (LGPL-2), PtPlot (MIT variant), public-domain helpers (`license-summary.txt`) | [FACT] | [1][3] |
| AMNH page | Behind a Cloudflare CAPTCHA for automated fetch on 2026-10-15; not used as evidence | [FACT] | [4] |
| Source = jar? | A ~450-line Python port written from this source alone (`reference/vendor/oracle/pymaxent.py`, gitignored) reproduced the jar's `.lambdas` **bit-for-bit** on one SWD dataset (58 samples, 3000 bg, L+Q+H, 220 iterations) and to ≤1e-11 relative on five others (6–120 samples, products, added samples, betamultiplier 0.7/1/2.5): same selected features, same knots, same order, same iteration count, gains equal to 1e-14. | [FACT] (own experiment) | §2.12 |

The last row matters: it shows the algorithm described below is complete enough to reproduce the
jar's optimization *path*, not only its optimum.

### 2.1 Formulation

**Model.** Let P be the set of N "points" (background points plus, by default, presence samples not
already in the background — §2.4) and f_j(·) ∈ [0,1] the features. MaxEnt fits a Gibbs
distribution over P:

  q_λ(i) = exp(λ·f(i)) / Σ_{k∈P} exp(λ·f(k))   [FACT, `FeaturedSpace.java:32-34`, `setDensity` :688]

and minimizes the ℓ1-regularized negative mean log-likelihood of the m presence samples

  ℒ(λ) = −λ·π̃ + log Σ_{k∈P} exp(λ·f(k)) + Σ_j β_j |λ_j|   [FACT, `getLoss` :185, `getN1`, `getL1reg`; `Sequential.getLoss` :62]

where π̃_j is the (interval-midpoint) sample mean of f_j and β_j the per-feature regularization
width (§2.3). This is the dual of the box-constrained maximum-entropy problem
|E_q[f_j] − π̃_j| ≤ β_j (Dudík, Phillips & Schapire 2004/2007; Phillips et al. 2004, 2006) [5][6][7][8].
Numerically the jar stores `linearPredictor[i] = λ·f(i)`, `linearPredictorNormalizer` L = max_i
λ·f(i), `density[i] = exp(lp_i − L)`, `densityNormalizer` Z̃ = Σ density[i], so
log Σ exp(λ·f) = L + log Z̃ (a max-shift log-sum-exp) [FACT, `FeaturedSpace.java:656-664,688-702`].
"Gain" = log N − ℒ (regularized); unregularized gain = gain + Σβ|λ| [FACT, `Runner.java:1714`, `writeSummary`].

**Statistical interpretations** (all agree on the *exponent*, differ on the intercept/normalization):

| View | Statement | Consequence for NicheRS | Ref |
|---|---|---|---|
| Inhomogeneous Poisson point process (IPP) | MaxEnt ≡ IPP with log-intensity log μ(x) = α + λ·f(x); the ℓ1-penalized MaxEnt likelihood equals the penalized IPP likelihood with α profiled out; Maxent's raw output ∝ intensity | Engine can be written as a PPM, but the jar's optimizer and penalties must still be reproduced for equality | Renner & Warton 2013 [9]; Warton & Shepherd 2010 [10]; Fithian & Hastie 2013 [11] |
| Infinitely weighted logistic regression (IWLR) | Logistic regression of presence vs background with background weights → ∞ converges to the MaxEnt/IPP slope estimates | maxnet/elapid approximate with weight 100 and glmnet ⇒ *not* numerically identical | Fithian & Hastie 2013 [11]; maxnet source [12] |
| Density-ratio view | q_λ estimates f1(z)/f(z) (covariate density at presences over background); Maxent minimizes relative entropy to background | Explains "raw" output and entropy-based calibration | Elith et al. 2011 [13] |
| cloglog (3.4.0+) | Under an IPP with intensity μ(x), P(≥1 presence in a site) = 1 − exp(−μ(x)·c); Maxent sets c via e^H, giving 1 − exp(−e^H · raw) | Default output since 3.4.0 | Phillips et al. 2017 [14] |

[FACT for rows citing papers whose abstracts/bibliographic data were verified via Crossref; the
detailed derivations are [UNVERIFIED – prior knowledge] for this note, but the code-level formulas in
§2.7 are [FACT].]

### 2.2 Feature construction (`Runner.makeFeatures` :2145)

Base variables are sorted by name (`ParamsPre.getSelected` → `Arrays.sort`, :188; Java `String`
order, uppercase before lowercase) [FACT]. Construction order — which fixes tie-breaking and the
`.lambdas` line order — is: bias layers; for each categorical layer all its binary features; for each
continuous layer a linear feature; then quadratics; then products (i<j); then threshold generators;
then for each layer a hinge generator and a *reverse* hinge generator [FACT, :2156-2196].

| Class | Raw function | Scaling | Notes | Code |
|---|---|---|---|---|
| Linear `v` | x | `(x − min)/(max − min)`, min/max over **all N points** (background + added samples), not samples; range 0 → scale 1 | Always constructed (needed for clamping), deactivated if `linear=false` | `ScaledFeature(Feature)` ctor; `CachedScaledFeature` |
| Quadratic `v^2` | **x² of the raw variable**, then scaled by min/max of x² over points | not (scaled x)² | `SquareFeature.java`, :2178 |
| Product `a*b` | **x_a·x_b raw**, then scaled by min/max of the product over points | skipped if either layer is constant ("mask") | `ProductFeature.java`, :2184-2188 |
| Hinge (forward) `'v` | h_t(x) = (x − t)/(M − t) if x > t else 0; M = max of x over points ∪ samples | inherently in [0,1]; *not* wrapped in ScaledFeature | `HingeFeatureGenerator.featureEval` :104 |
| Hinge (reverse) `` `v `` | same on y = −x: (−x − t)/(max(−x) − t) | equivalently (k − x)/(k − min x) for x < k, k = −t | `Runner.revFeature` :2220 |
| Threshold `(t<v)` | 1 if x > t else 0 | binary | `ThrFeatureGenerator.featureEval` :106 |
| Categorical `(v=c)` | 1 if x == (float)c | one per distinct value over **points** | `BinaryFeature.makeAll` |

[FACT for all rows.] When `doClamp=true` (default) each continuous layer and each derived
linear/quadratic/product feature is additionally wrapped in `ClampedFeature(min,max over points)`
before scaling (`naturallyClamped` :2228). At training time this is a no-op for points; it only
changes **samples that lie outside the point range**, which can happen only when
`addsamplestobackground=false` [FACT].

**Hinge/threshold knots (`SortedFeatureGenerator` ctor :99).** All point values and all sample
values of the raw variable are sorted together (stable `Arrays.sort` on `Integer[]`, points first).
A *precision* `prec = min over all values of precision(v)` is computed (`precision` :78: inspects
the first six significant decimal digits; returns 50·10^k when it finds a run "00/01/98/99", else
5·10^(e−6)); `precision(0)=0`, so a single zero value makes `prec=0` [FACT]. Walking the sorted
values, a knot is emitted whenever `val − lastVal > prec`, at `t = (val + lastVal)/2`, and
`lastVal` is updated only at emitted knots. **There is no fixed number of hinges**: every midpoint
between consecutive distinct values (≈ number of distinct values − 1, e.g. 2560–2954 per variable on
a 3000-point background) is a candidate; the optimizer selects among them lazily (§2.5) [FACT].
Hinges use all knots (`thrFirst=0, thrLast=numThr`); threshold features only knots between the
first and last *sample* value [FACT, `ThrFeatureGenerator` ctor].

**Auto-features (`autofeature=true`, `autoSetActive` :2312, defaults from `parameters.csv`).** With m
= number of training samples after duplicate/missing-data removal:

| Class | Active when | Parameter |
|---|---|---|
| linear | always | — |
| quadratic | m ≥ 10 | `l2lqThreshold=10` |
| hinge | m ≥ 15 | `hingeThreshold=15` |
| product | m ≥ 80 | `lq2lqptThreshold=80` |
| threshold | m ≥ 80 **and** `threshold=true` (default **false** in 3.4.x) | `lq2lqptThreshold=80` |

[FACT; confirmed by run logs: 58 samples → L,Q,H; 6 → L; 120 → L,Q,P,H.] Feature-class flags
`linear/quadratic/product/threshold/hinge` must also be true for the class to exist. Binary
features with zero sample expectation are deactivated (`maxentRun` :1704-1708) [FACT].

### 2.3 Regularization

**Class betas** (`autoSetBeta` :2253; `interpolate` :2240 = piecewise-linear, constant beyond ends,
first breakpoint with m ≤ x_i):

| Class | Breakpoints (m) | β values | Condition |
|---|---|---|---|
| linear/quadratic/product ("lqp"), product table | 0, 10, 17, 30, 100 | 2.6, 1.6, 0.9, 0.55, 0.05 | `product` on and (not autofeature or m ≥ 80) |
| lqp, quadratic table | 0, 10, 17, 30, 100 | 1.3, 0.8, 0.5, 0.25, 0.05 | else `quadratic` on and (not autofeature or m ≥ 10) |
| lqp, linear table | 10, 30, 100 | 1.0, 0.2, 0.05 | otherwise |
| threshold | 0, 100 | 2.0, 1.0 | — |
| hinge | — | 0.5 (constant) | — |
| categorical | 0, 10, 17 | 0.65, 0.5, 0.25 | (`doSqrtCat` hidden alternative) |

Overrides `beta_lqp`, `beta_threshold`, `beta_hinge`, `beta_categorical` (≥0 replaces the table).
Every feature gets `beta = classBeta × betamultiplier` [FACT]. Logged example: m=58 →
"linear/quadratic/product: 0.170 … threshold: 1.420, hinge: 0.500" = 0.25 − 0.2·28/70 ✓, 2 − 0.58 ✓.

**Per-feature width β_j (the "sampleDeviation")** (`setSampleExpectations` :469,
`getDividedSampleInfo` :429, `Interval` :397). Without a bias file:

- μ̂ = mean of f_j over samples with data; σ̂ = sqrt((Σf² − m μ̂²)/(m−1)) (0 if the radicand is
  negative from rounding); **σ̂ capped at ½(max_P f − min_P f)** (= ½ for scaled features); m=1 ⇒
  σ̂ = ½ range.
- low = μ̂ − β/√m · σ̂, high = μ̂ + β/√m · σ̂; π̃_j = ½(low+high); β_j = ½(high−low), i.e.
  **β_j = β · σ̂ / √m** — the "σ/√m" form of Phillips & Dudík 2008 [7].
- Floor: if β_j < `minDeviation` = 0.001 × betamultiplier (a mutable *static*,
  `FeaturedSpace.java:310-312`), β_j := 1/(2m) for a binary feature with π̃=1, else minDeviation.
- Hinge generators (`HingeFeatureGenerator.setSampleExpectations` :42): σ̂ of the hinge values,
  **clamped to [1/√m, ½]** (m>1), m=1 ⇒ ½; same interval and floor. Threshold generators: σ̂ capped
  at ½, no lower clamp.
- With a bias file (`biasFile`/FactorBiasOut) values are divided by the bias grid and intervals are
  divided (`Interval(Interval,Interval)`); out of scope for NicheRS v1 [FACT/OPEN].

maxnet reproduces these tables (`maxnet.default.regularization.R`) but computes the width as
`max(0.001·range, hinge floor 0.5·max(sd,1/√m)/√m, threshold all-0/1 → 1, sd·β/√m)` on
*unscaled* features and passes it as glmnet `penalty.factor` — so the effective penalty differs [FACT, 12].

### 2.4 Background, samples, missing data, duplicates

| Aspect | Behaviour | Code | Tag |
|---|---|---|---|
| Background, grids | Reservoir sampling (Algorithm R) over cells with data in **all** layers, row-major scan, `maximumbackground=10000`; `Utils.generator = new Random(0)` unless `randomseed=true` (then `currentTimeMillis`) | `Extractor.readSamples` :229; `Runner.java:303` | [FACT] |
| Background, SWD | **all** rows of the SWD background file; a row containing the NODATA value (`nodata=-9999`) in any selected variable is dropped; `maximumbackground` not applied | `GridSetFromFile` ctor :35 | [FACT] |
| SWD layout | col 0 species, col 1 x/lon, col 2 y/lat, env vars from col 3 by header name | `SampleSet.java:36` | [FACT] |
| Sample missing data | sample with NODATA (or empty) in any layer is skipped unless `allowpartialdata` | `SampleSet2` read; `Runner.withAllData` | [FACT] |
| Duplicates (`removeduplicates=true`) | grids: same cell (row·ncols+col); SWD: same `lat+""+lon` **string** (Java `Double.toString` concatenation, no separator) | `SampleSet.removeDuplicates` :137 | [FACT] |
| Add samples to background (`addsamplestobackground=true`) | for each sample, hash r = Σ_j u_j x_j with u_j ~ `Utils.generator.nextDouble()`; added unless r equals (binary search) some background hash, i.e. unless **its environmental vector already exists** in the background; `addallsamplestobackground` adds all. Missing sample values imputed from a random other added sample | `Runner.featuresWithSamples` :1220 | [FACT] |
| Effect | N = n_bg + n_added; all min/max (scaling, clamping), knots, Z̃, entropy and `numBackgroundPoints` use the extended set | — | [FACT] (run: 120 perturbed samples → N = 2500+110) |
| Random test split | `randomtestpoints>0` and not randomseed ⇒ generator reset to `Random(11111)` | `Runner.java:365` | [FACT] |

Because java.util.Random is a specified 48-bit LCG, bit-exact background selection is
reproducible in Rust if the same RNG and call order are used [ASSUMPTION – JDK spec, not tested
here]. [REC] For equivalence tests use SWD background (no RNG involved).

### 2.5 Optimization: the sequential-update algorithm (`Sequential.java`)

The algorithm is the "sequential-update" coordinate-descent of Dudík et al. [6][8] with
engineering additions. Key properties [all FACT, line numbers in `Sequential.java`]:

1. **Initial state**: all λ=0, L=0, density=1, Z̃=N, loss = log N (`run` :497).
2. **Iteration loop** (`run`): for `iteration = 0 … maximumiterations−1` (default 500): if
   `iteration>0 && iteration % parallelUpdateFrequency == 0` (**default 30**, `parameters.csv`;
   the field initialiser 10 is overwritten in `Runner.maxentRun`) do a *parallel update*, else a
   *sequential update* of the single best feature.
3. **Feature selection** (`getBestFeature` :66): scans all active non-generated features and **every
   knot of every generator**; picks the minimum of the *loss-reduction bound*
   `deltaLossBound` :325: with W1 = E_q[f], N1 = π̃, α = `goodAlpha` (closed-form optimal step for a
   binary feature with the ℓ1 term, :294),
   bound = −N1·α + log(1 − W1 + W1·e^α) + β(|λ+α| − |λ|). Valid as an upper bound for f∈[0,1] by
   convexity — **this is why every feature must be in [0,1]**. Strict `<` against an initial
   bestLb = 1.0 ⇒ ties go to the first feature in list order, generators after plain features. A
   chosen generator knot is materialized ("exported") as a `HingeFeature`/`ThresholdFeature` and
   **appended** to the feature list.
4. **Sequential step** (`doSequentialUpdate` :419): refresh h's expectation if stale; compute
   `toUpdate` (below); binary ⇒ α = goodAlpha; otherwise α = **one Newton step**
   `−deriv/Var_q[f]` (:223; deriv = W1−N1 ± β, subgradient at λ=0) truncated so λ does not change
   sign. Then **damping** `reduceAlpha`: α/50 for iteration<10, α/10 for <20, α/3 for <50
   (:467). Apply; if the realized loss change exceeds the bound, undo and run `searchAlpha`
   (start at goodAlpha, multiply by 4 while loss decreases, then try ×2; :104), damp again, apply.
5. **Parallel step** (`doParallelUpdate` :245): direction u_j = λ_j − λ_j^{prev} (last parallel
   step) for non-binary non-zero features; Newton step size along u with exact expectations of the
   involved features (:175), truncated at the first sign change, coefficients within 1e-6 of zero
   snapped to zero; reverted (u → −u applied) if loss increased.
6. **Stale expectations — behaviour-defining.** After each λ change the density is recomputed, but
   expectations E_q[f_j] are refreshed only for `featuresToUpdate()` (:397): features changed in
   the last 10 iterations, features with `index % 20 == iteration % 20`, and the 5 features with the
   lowest bound (sorted with the custom quicksort `DoubleIndexSort`, Bentley–McIlroy, *not stable*).
   Generator knots are always fresh (O(N+m) sweep per generator, `updateFeatureExpectations` :87).
   Selection therefore uses **stale W1** for some plain features; reproducing the path requires
   reproducing this cache policy exactly, including the sort.
7. **Convergence** (`terminationTest` :476): only every 20 iterations; stop when
   (loss 20 iterations ago − current loss) < `convergencethreshold` (default 1e-5), or gain > 10000.
   The reported iteration count is the index at which it stopped (multiple of 20) or 500.
8. **Numerical stability**: max-shift (L) in all exponentials; Z̃ ≥ 1; generator expectations via
   cumulative sums of `density·x` and `density` in sorted order; NaN/Inf guarded in searchAlpha and
   bound; no log-sum-exp beyond the max shift. [FACT]
9. **Contributions** (`h.contribution`): loss decrease credited to the updated feature (subtracted
   when the step moves λ toward zero); parallel steps distribute by previous contribution deltas.

### 2.6 Loss/gain bookkeeping

`getLoss = getN1 + log Z̃ + reg`, `getN1 = −Σλ_j π̃_j + L` (`FeaturedSpace.java:185-214`). The
running `reg` is updated incrementally in sequential steps (`increaseLambda` :387) and recomputed
fully after parallel steps (`setReg`) — floating differences are path-relevant only at the 1e-15
level [FACT].

### 2.7 Output transforms (`Project.java`)

At projection, for a site with variables x: s = Σ λ_j f_j(x) (only λ≠0), then

- raw = min(1, exp(s − L) / Z̃) (`pred` :271) — sums to 1 over the training points;
- **cloglog** (default since 3.4.0) = 1 − exp(−e^H · raw) (`cloglog` :331);
- **logistic** = τ e^H raw / ((1−τ) + τ e^H raw), τ = `defaultprevalence` = 0.5 (`logistic` :327);
- **cumulative** = 100 × (sum of raw over background points with raw ≤ value); in projection it is
  *linearly interpolated* from the subsampled `_omission.csv` table (`interpolateCumulative` :552),
  so it is approximate;
- NaN/Inf ⇒ 1 (or 100) if s > L else 0.

H = −Σ_i p_i ln p_i over the N training points, p_i = density_i/Z̃ (`getEntropy` :87). [FACT]
Logistic(τ=0.5) = raw·e^H/(1+raw·e^H) equals maxnet's 1/(1+exp(−H−link)) [FACT, 12]. Prevalence
reported in `maxentResults.csv` = mean over points of the chosen probability transform.

### 2.8 Clamping, extrapolation, missing values at prediction

| Flag (default) | Effect | Code |
|---|---|---|
| `doclamp` (true) | linear base variables clamped to the [min,max] written on their linear line; quadratic/product computed from **clamped** variables, then scaled feature clamped to [0,1]; hinge = 1 when x > knot max (so hinge saturates) | `ClampedGrid` :467, `ScaledGrid` :487, `HingeGrid` :450 |
| `extrapolate` (true) | if false: prediction 0 wherever any variable is outside its linear [min,max] | `wouldExtrapolate` in `projectGrid` |
| `fadebyclamping` (false) | pred −= |pred_clamped − pred_unclamped|, floored at 0 | `eval` :293 |
| `writeclampgrid` (true) | |pred_clamped − pred_unclamped| grid | `clampGrid` |
| Missing data | cell is NODATA if **any** grid referenced by the lambdas file lacks data (all linear variables are always referenced) | `hasData` in `projectGrid` |
| Precision | projection reads every variable as **float32** (`Grid.eval` returns float; SWD `(float) allvals`) and writes float32; training uses float64 for SWD | `GridSetFromFile.getGrid`, `Project` | 
| Threshold feature | training `x > t`, projection `x >= t` (inconsistency) | `ThresholdFeature`, `ThresholdGrid` |

[FACT all rows.]

### 2.9 Variable importance, jackknife, response curves

- **Percent contribution**: per-feature `contribution` (§2.5.9) summed per variable (product credit
  split ½/½), normalized to 100 (`determineContributions` :1737). Path-dependent; [FACT].
- **Permutation importance**: AUC (training presences vs background, background *without* added
  samples) with one variable permuted across the pooled presence+background rows; drop in AUC,
  negatives → 0, normalized to 100. Uses `java.util.Random(11111)` and permutation via sorting
  uniform doubles with `DoubleIndexSort`; evaluation through `Evaluate` (cloglog, clamped, float32
  inputs) (`PermutationImportance.importance` :94, :124) [FACT].
- **Jackknife** (`jackknife=true`): for each variable a full re-fit without it and with only it
  (betas recomputed with the same m); reports training gain, test gain, AUC (`jackknifeGain`
  :1548) [FACT].
- **Response curves**: (a) marginal — vary one variable over [min − 10% range, max + 10%] with the
  others at their sample mean (categorical: modal class) and evaluate via `Project` (clamped,
  output format); (b) one-variable models re-fitted with only that variable
  (`createProfiles` :1273/:1390) [FACT].

### 2.10 `.lambdas` file format (`FeaturedSpace.writeWeights` :750; readers: `Project.projectGrid` :137, `FeaturedSpace(GridSet,…)` :521)

One feature per line, `name, lambda, min, max`, written with Java string concatenation (`", "`
separators, `Double.toString` numbers, e.g. `1.0E-4`, `214.89999999999998`), followed by four
footer lines. Real example (3.4.4, this study):

```
elev, -0.23189638900754544, 0.0, 1999.4
prec, 0.0, -30.04, 1011.88
temp, 0.0, 4.413, 17.375
`prec, -3.948371134077774, -30.04, 422.37
'temp, -2.4150369826952884, 12.3385, 17.375
linearPredictorNormalizer, 0.36851692509237716
densityNormalizer, 760.6097256920323
numBackgroundPoints, 3000
entropy, 7.593218189142168
```

Rules [FACT]: lines follow the internal feature order (§2.2) with exported hinge/threshold features
appended **in order of first selection**; zero-λ features are omitted except linear ones (needed for
clamping) and the first category of each categorical variable; at least one feature is always
written. min/max are the scaling bounds (linear/quadratic/product), knot and M (forward hinge),
min(x) and knot (reverse hinge, already converted back to x units), `0.0, 1.0` (threshold,
categorical). Name grammar is decided by substring search in this order: `*`, `^2`, `^p`, `=`,
`<`, `'`, `` ` ``, footer keywords, else linear — **variable names containing these characters or
commas are unsafe**. Full grammar in the spec.

### 2.11 Headless command line

Arguments are `name=value` or bare booleans (`nowarnings`, `noaskoverwrite`); single-letter flags
`-a`, `-z`, `-A`… **toggle** the current value (`Parameters.readFromArgs` :96) [FACT]. With
`visible=false` (`-z`) `MaxEnt.main` runs `Runner.start()` directly without Swing; `autorun` only
matters for the GUI [FACT, `MaxEnt.java`]. Verified working command (3.4.4, OpenJDK 25, this study):

```
java -Djava.awt.headless=true -Duser.language=en -Duser.country=US -Xmx2g -jar maxent.jar \
  samplesfile=sp_swd.csv environmentallayers=bg_swd.csv outputdirectory=out \
  visible=false autorun=true warnings=false askoverwrite=false skipifexists=false \
  pictures=false plots=false responsecurves=false jackknife=false \
  outputformat=cloglog randomseed=false betamultiplier=1 \
  linear=true quadratic=true product=true hinge=true threshold=false autofeature=true \
  addsamplestobackground=true removeduplicates=true maximumiterations=500 convergencethreshold=0.00001 \
  writebackgroundpredictions=true
```

Useful extras: `verbose` (per-iteration trace in `maxent.log`: selected feature, α, bound, W1, N1,
loss — **formatted with the JVM default locale**, hence `-Duser.language=en`), `togglelayertype=prefix`
(`-t`, mark categorical), `togglelayerselected=prefix` (`-N`), `togglespeciesselected` (`-E`),
`replicates`, `replicatetype`, `randomtestpoints`, `projectionlayers`, `nodata`. Outputs:
`<sp>.lambdas`, `maxentResults.csv`, `<sp>_samplePredictions.csv` (raw/cloglog in full double
precision), `<sp>_backgroundPredictions.csv` (raw printed with 6 significant digits!),
`<sp>.csv` (SWD projection, float32), `<sp>_omission.csv`, `maxent.log`. Runtime of the test above:
0.6 s wall.

### 2.12 Empirical validation of this reading (own experiment, reproducible)

`reference/vendor/oracle/` (gitignored): `gen.py`/`gen2.py` (synthetic SWD), `pymaxent.py` (port),
`project.py` (projection), `cmp.sh` (driver). Results [FACT, 2026-10-15]:

| Case (m samples / bg / added / betamult) | Classes | Iter. jar = port | Max |Δλ| | Gain jar vs port |
|---|---|---|---|---|
| 58 / 3000 / 0 / 1 | L,Q,H | 220 = 220 | 0 (bit-identical file) | identical |
| 110 / 2500 / 110 / 1 | L,Q,P,H | 500 = 500 | ~5e-12 | Δ 2e-15 |
| 112 / 2500 / 0 / 2.5 | L,Q,P,H | 480 = 480 | ~3e-13 | Δ 2e-15 |
| 12 / 2500 / 12 / 1 | L,Q | 60 = 60 | ~6e-15 | Δ 9e-16 |
| 6 / 2500 / 0 / 1 | L | 60 = 60 | 0 (identical) | identical |
| 30 / 2500 / 30 / 0.7 | L,Q,H | 300 = 300 | ~3e-12 | Δ 2e-15 |

Projection: cloglog recomputed from the jar's `.lambdas` with float32 inputs matches the jar's SWD
projection to 5.8e-8 (float32 print precision); double-precision recomputation matches
`backgroundPredictions.csv` cloglog to 5e-10. Residual Δλ come from `exp`/summation-order ulps
(numpy vs Java), i.e. Rust will not be bit-identical by default either.

### 2.13 maxnet and elapid vs maxent.jar

| Aspect | maxent.jar 3.4.4 | maxnet 0.1.4 (R, CRAN 2021-07-09) | elapid (Python, PyPI 1.0.4) |
|---|---|---|---|
| Fitting | sequential-update coordinate descent, damped, lazy hinge generation, ≤500 iters, conv. 1e-5/20 iters | glmnet binomial (IWLR, background weight 100) along a 200-λ path ending at λ_min = mean(reg)·Σp/Σw; uses last path point | `glmnet` python `LogitNet` (lasso, `scoring=roc_auc`, internal CV picks `lambda_max_inx_`, "best") or sklearn; background weight 100; 100-λ path |
| Background | + samples not present (default) | + samples not present (`dplyr::setdiff`) | caller supplies presence+background arrays; no automatic add-samples step found in `models.py` |
| Linear/quadratic/product | scaled to [0,1] over points; Q = scaled raw x² | **unscaled** model.matrix terms | MinMax-scaled (`LinearTransformer`), Q/P scaled |
| Hinges | every distinct-value midpoint, both directions; knot max/min = data max/min | 50 knots (`nknots=50`) evenly spaced in [min,max] → 2×49 hinges | `n_hinge_features=10` linspace → 2×9 |
| Thresholds | midpoints within sample range, `x>t` | 50 evenly spaced, `x>=t` | 10 linspace |
| Regularization | β·σ̂/√m on scaled features, σ̂ caps/floors as §2.3, minDev 0.001·bm | same tables; `max(0.001·range, sd·β/√m, hinge floor, threshold 1)` on unscaled features as glmnet `penalty.factor` | maxnet tables (`RegularizationConfig`) ×per-class multipliers (1.0) × **beta_multiplier default 1.5**; `compute_regularization`'s `beta_categorical` default points to `beta_hinge` (both 1.0, harmless) |
| Auto classes | L; +Q ≥10; +H ≥15; +P ≥80 (T off) | identical thresholds ("l","lq","lqh","lqph") | default `["linear","hinge","product"]` regardless of m |
| Output | raw/cloglog/logistic(τ)/cumulative, entropy over points | link/exponential/cloglog/logistic (no τ), entropy over background rows | raw/logistic(τ)/cloglog |
| Clamping | variables & features, hinge saturation, extrapolate flag | clamp vars to [varmin,varmax] and features to [featuremin,max] | MinMaxScaler(clip=True) for L/Q/P |
| Objective at optimum | penalized MaxEnt | approximation of it (finite weight, different penalty scaling, path end) | idem + different defaults |

[FACT for all cells read in source: maxnet `R/maxnet.R`, `R/maxnet.default.regularization.R`,
`R/hinge.R`, `R/thresholds.R`, `R/maxnet.formula.R`, `R/predict.maxnet.R` (commit `271b85e`,
2022-12-13); elapid `elapid/features.py`, `elapid/config.py`, `elapid/models.py` (commit
`359355a`, 2026-06-11; pyproject says 1.0.3, PyPI reports 1.0.4).] Consequence: neither maxnet nor
elapid can serve as the numerical reference for "equivalent to maxent.jar"; they are useful as
*sanity* references (similar response shapes, AUC) only [REC].

### 2.14 Performance characteristics

| Hotspot | Cost | Notes |
|---|---|---|
| `getBestFeature` | O(Σ knots) bound evaluations per iteration (~2·V·N) | cheap closed form, but scalar Java loop |
| `setDensity` | O(N·(1+|toUpdate|)) per λ change + O(N+m) per generator | dominant inner loop; memory-bound |
| `newLossChange` in `searchAlpha` | O(N) exp per trial | only after bound violation |
| Product features | V(V−1)/2 cached arrays of N doubles | memory O(N·V²) |
| Projection | per cell, all non-zero features, float32 grids, single-threaded, ASCII/MXE I/O | dominant for large rasters |
| Jackknife / response curves / replicates | (2V+1)× / (V+1)× / k× full fits | embarrassingly parallel (`threads`) |
| Permutation importance | V × (m+N) evaluations through `Evaluate` with a HashMap per value | slow, O(V·(m+N)·features) |

[FACT from code structure; timings: 0.35–0.6 s per fit at N≈3000, V=3 on this machine.] No
published benchmark was used; none is claimed.

## 3. Evidence and references

Primary evidence is the vendored source (file:line throughout), the 3.4.4 jar executed headless,
and the oracle comparison in §2.12. Literature: [5]–[11], [13]–[16]. Software: [1]–[3], [12], [17].

## 4. Alternatives

| Option | Description | Equivalence achievable |
|---|---|---|
| A. Faithful port of `Sequential` (+ generators, stale-expectation policy, DoubleIndexSort, damping) | what §2 describes | path-equivalent to ~1e-11 (demonstrated in Python) |
| B. Generic solver (coordinate descent/L-BFGS-OWL-QN/FISTA) on the same objective, same features incl. all hinge knots | converges to (near) the same optimum if run to tight tolerance | **not** equal: jar stops after ≤500 damped iterations at 1e-5/20-iter criterion, i.e. far from the exact optimum; sparsity pattern differs |
| C. glmnet-style IWLR (maxnet) | well-understood, fast | different estimator (finite weights, penalty scaling, knots) |
| D. Call maxent.jar as a subprocess | exact by construction | requires JVM; violates single-binary goal; useful as test oracle only |

## 5. Trade-offs

- A reproduces the jar's *early-stopped, damped* solution — scientifically it is the de-facto
  standard (and what reviewers compare against) but it is not the exact penalized MLE. B gives a
  "better" optimum that will *differ* from published Maxent results by more than numerical noise.
- A is inherently sequential per fit (each iteration depends on the last); parallelism must come
  from inner loops (deterministic chunked reductions) and from running species/folds concurrently.
- Bit-exactness vs speed: SIMD/parallel summation reorders floating additions; tie-breaks
  (strict `<`, unstable quicksort) and the 1e-5 convergence test can flip on 1-ulp differences.

## 6. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Path divergence from ulp differences (exp, summation order) on knife-edge decisions | low per fit (0/6 observed), non-zero over thousands of fits | different λ set; predictions usually close | two-tier test (path-exact vs prediction-equivalent), deterministic sequential sums in "compat" mode |
| Mis-porting stale-expectation policy | medium | silent divergence after ~10 iterations | per-iteration trace comparison with `verbose` log |
| Float32 vs float64 inputs at projection | certain | 1e-7 diffs | spec defines both; compare against jar float output with float tolerance |
| Java `Double.toString` differs across JDK versions (shortest-repr fix in JDK 19) | medium | byte-level `.lambdas` diffs | compare numerically, not textually |
| Variable names with `* ^ = < ' \` ,` | medium | corrupt `.lambdas` | validate names |
| SWD duplicate key `lat+""+lon` collisions | very low | different m | emulate exactly in compat mode |
| Locale-dependent log formatting | certain on non-US locales | trace parser breaks | force `-Duser.language=en` |

## 7. Recommendations

1. [REC] Implement Alternative A as the NicheRS "maxent-compat" engine; treat any other solver as a
   separate, explicitly named engine.
2. [REC] Port, in this order: SWD input + dedup + add-samples; linear/quadratic/hinge (both
   directions) features with exact scaling and knot rule (incl. `precision`); class-beta tables and
   σ̂/√m widths with caps/floors; `Sequential` (bound, goodAlpha, Newton, damping, search, parallel
   update every 30, stale-expectation policy, DoubleIndexSort, termination every 20); entropy,
   cloglog; `.lambdas` writer/reader; clamped projection. Then product, threshold, categorical,
   grids + java.util.Random reservoir sampling, cumulative, importance.
3. [REC] Keep a "compat" numeric mode: sequential left-to-right f64 sums, no FMA, scalar `exp`;
   allow a "fast" mode later only behind the two-tier tests.
4. [REC] Use the jar (3.4.4, pinned sha256) plus the Python oracle as CI oracles on a corpus of
   synthetic and real SWD datasets; compare `.lambdas` numerically, iteration counts, and per-
   iteration traces (`verbose`).
5. [REC] Store in NicheRS model metadata: engine version, compat mode, feature list, betas,
   iteration count, convergence reason, N, m, added samples.

## 8. Open questions

- [OPEN] Rust `f64::exp` (libm/system) vs Java `StrictMath`/`Math.exp` intrinsics: frequency of
  path divergence on real datasets (needs a corpus run).
- [OPEN] Whether to reproduce java.util.Random reservoir sampling for grid backgrounds or to require
  explicit SWD backgrounds for compat mode.
- [OPEN] Bias-file (sampling bias) support: needed for v1? Its interval-division math is in
  `FeaturedSpace.Interval(Interval,Interval)`.
- [OPEN] Whether to fix the threshold `>`/`>=` inconsistency or reproduce it.
- [OPEN] Category handling of float equality (`(float)` cast) for categorical rasters stored as
  float.

## References

[1] mrmaxent/Maxent GitHub repository, commit 963d0114 (2024-01-11), release v3.4.4 (2020-11-24). https://github.com/mrmaxent/Maxent (accessed 2026-10-15)
[2] maxent.jar 3.4.4, `ArchivedReleases/3.4.4/maxent.jar`, sha256 4c856e55412f70c5597b03cf9aaaf27e0782e0921f937262273b68bdcb8fee5e (executed 2026-10-15)
[3] Maxent LICENSE.md (MIT), license-summary.txt, ArchivedReleases/{3.3.3k,3.4.1,3.4.3,3.4.4}/readme.txt (accessed 2026-10-15)
[4] AMNH Maxent page https://biodiversityinformatics.amnh.org/open_source/maxent/ (Cloudflare challenge, not readable, 2026-10-15)
[5] Phillips S.J., Dudík M., Schapire R.E. 2004. A maximum entropy approach to species distribution modeling. Proc. ICML 2004, p.83. doi:10.1145/1015330.1015412 (accessed 2026-10-15)
[6] Dudík M., Phillips S.J., Schapire R.E. 2004. Performance guarantees for regularized maximum entropy density estimation. COLT 2004, LNCS 3120:472–486. doi:10.1007/978-3-540-27819-1_33 (accessed 2026-10-15)
[7] Phillips S.J., Dudík M. 2008. Modeling of species distributions with Maxent: new extensions and a comprehensive evaluation. Ecography 31:161–175. doi:10.1111/j.0906-7590.2008.5203.x (accessed 2026-10-15)
[8] Dudík M., Phillips S.J., Schapire R.E. 2007. Maximum entropy density estimation with generalized regularization and an application to species distribution modeling. JMLR 8:1217–1260. https://jmlr.org/papers/v8/dudik07a.html (accessed 2026-10-15)
[9] Renner I.W., Warton D.I. 2013. Equivalence of MAXENT and Poisson point process models for species distribution modeling in ecology. Biometrics 69:274–281. doi:10.1111/j.1541-0420.2012.01824.x (accessed 2026-10-15)
[10] Warton D.I., Shepherd L.C. 2010. Poisson point process models solve the "pseudo-absence problem" for presence-only data in ecology. Ann. Appl. Stat. 4:1383–1402. doi:10.1214/10-AOAS331 (accessed 2026-10-15)
[11] Fithian W., Hastie T. 2013. Finite-sample equivalence in statistical models for presence-only data. Ann. Appl. Stat. 7:1917–1939. doi:10.1214/13-AOAS667 (accessed 2026-10-15)
[12] mrmaxent/maxnet GitHub repository, commit 271b85e (2022-12-13); CRAN maxnet 0.1.4 (published 2021-07-09, MIT). https://github.com/mrmaxent/maxnet ; https://CRAN.R-project.org/package=maxnet (accessed 2026-10-15)
[13] Elith J., Phillips S.J., Hastie T., Dudík M., Chee Y.E., Yates C.J. 2011. A statistical explanation of MaxEnt for ecologists. Diversity and Distributions 17:43–57. doi:10.1111/j.1472-4642.2010.00725.x (accessed 2026-10-15)
[14] Phillips S.J., Anderson R.P., Dudík M., Schapire R.E., Blair M.E. 2017. Opening the black box: an open-source release of Maxent. Ecography 40:887–893. doi:10.1111/ecog.03049 (accessed 2026-10-15)
[15] Phillips S.J., Anderson R.P., Schapire R.E. 2006. Maximum entropy modeling of species geographic distributions. Ecological Modelling 190:231–259. doi:10.1016/j.ecolmodel.2005.03.026 (accessed 2026-10-15)
[16] Merow C., Smith M.J., Silander J.A. 2013. A practical guide to MaxEnt for modeling species' distributions. Ecography 36:1058–1069. doi:10.1111/j.1600-0587.2013.07872.x (accessed 2026-10-15)
[17] elapid, commit 359355a (2026-06-11), PyPI 1.0.4 (MIT); Anderson C.B. 2023. elapid: Species distribution modeling tools for Python. JOSS 8(84):4930. doi:10.21105/joss.04930. https://github.com/earth-chris/elapid (accessed 2026-10-15)
[18] Friedman J., Hastie T., Tibshirani R. 2010. Regularization paths for generalized linear models via coordinate descent. J. Stat. Softw. 33(1). doi:10.18637/jss.v033.i01 (accessed 2026-10-15)
[19] Elith J., Kearney M., Phillips S. 2010. The art of modelling range-shifting species (MESS). Methods Ecol. Evol. 1:330–342. doi:10.1111/j.2041-210X.2010.00036.x (accessed 2026-10-15)
