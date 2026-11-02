# NicheRS MaxEnt-compatible engine — mathematical specification (testable)

Status: draft v0.1 · Date: 2026-10-15 · Reference implementation: maxent.jar **3.4.4**
(`reference/vendor/Maxent`, commit 963d0114; jar sha256 `4c856e55…fee5e`).
Rationale, provenance and comparisons: `docs/research/02-maxent-internals.md`.
Executable oracle (Python port that reproduces the jar's `.lambdas` to ≤1e-11, bit-identical on 2/6
test cases): `reference/vendor/oracle/pymaxent.py` (gitignored, local).

Code references `File.java:line` are relative to `reference/vendor/Maxent/density/`.
"MUST" = required for compat mode ("maxent-compat"); "NOTE" = jar quirk to reproduce or decide on.

---

## 0. Scope of compat mode v1

Continuous variables; SWD (samples-with-data) inputs for training; no bias file; feature classes
linear (L), quadratic (Q), hinge (H), product (P), threshold (T), categorical (C); outputs raw,
cloglog, logistic, cumulative; clamped projection. Priority order for implementation:
L+Q+H → regularization → sequential optimizer → entropy/cloglog → `.lambdas` I/O → projection/
clamping → P → C → T → grid backgrounds (java.util.Random) → cumulative → importance.

## 1. Notation

| Symbol | Meaning |
|---|---|
| V | selected variables, sorted by Java `String.compareTo` (UTF-16 code-unit order) |
| B | background rows (n_B), SWD rows without NODATA in any selected variable |
| S | training samples after filtering and de-duplication, m = \|S\| |
| A ⊆ S | samples added to the background (§3.3) |
| P = B ∪ A | "points", N = n_B + \|A\|, ordered: B in file order, then A in sample order |
| x_v(i) | raw value (f64) of variable v at point/sample i |
| f_j | feature j, values in [0,1] on P |
| λ_j | coefficient |
| π̃_j, β_j | sample expectation (interval midpoint) and regularization width ("sampleDeviation") |
| β^cls | class regularization value × betamultiplier (`bm`) |
| lp_i | λ·f(i); L = max_{i∈P} lp_i ("linearPredictorNormalizer") |
| d_i | exp(lp_i − L) ("density"); Z = Σ_{i∈P} d_i ("densityNormalizer") |
| W_j | E_q[f_j] = Σ d_i f_j(i) / Z ("expectation", possibly stale, §5.6) |
| H | entropy −Σ_i (d_i/Z) ln(d_i/Z) |
| ε | 1e-6 (`Sequential.eps`) |

All arithmetic is IEEE-754 binary64 unless stated. Sums are left-to-right over the index order
given (compat mode).

## 2. Inputs

- SWD CSV: header row; col 0 species, col 1 x (longitude), col 2 y (latitude), variables matched by
  header name from col 3 (`SampleSet.java:36`). If col 1 header contains "lat" and col 2 does not,
  x/y are swapped (`SampleSet2`).
- Species names sanitized: trim, ' '→'_', and `? ! ~ ' \ / { } ( )` → '_', `"` removed.
- NODATA value = `nodata` parameter (default −9999), compared as `value == -9999.0`.

## 3. Data preparation

### 3.1 Background (SWD)
Every background row is kept unless any selected variable equals NODATA (`GridSetFromFile:35`).
`maximumbackground` does NOT apply to SWD.

### 3.2 Samples
A sample with NODATA/empty in any selected variable is dropped (unless `allowpartialdata`).
De-duplication (`removeduplicates=true`): keep the first sample per key; SWD key = Java string
`Double.toString(lat) + Double.toString(lon)` (no separator) (`SampleSet.java:137`).

### 3.3 Adding samples to the background (`addsamplestobackground=true`)
A sample is added to P iff its vector (x_v(s))_{v∈V} is not exactly equal to the vector of any
background row (the jar uses a random-projection hash with `Utils.generator`; equal vectors always
collide, unequal vectors collide with probability ≈0) (`Runner.featuresWithSamples:1220`).
`addallsamplestobackground=true` adds all samples.

### 3.4 Auto feature classes (`autofeature=true`, `Runner.autoSetActive:2312`)
L always; Q iff m ≥ 10; H iff m ≥ 15; P iff m ≥ 80; T iff m ≥ 80 and `threshold=true`
(default false). A class must also be enabled by its flag.

## 4. Features (exact formulas)

For a function g over P∪S define min_P g = min_{i∈P} g(i), max_P g likewise.

### 4.1 Scaled features (L, Q, P) — `ScaledFeature`, `Runner.makeFeatures:2145`
raw_L(v) = x_v; raw_Q(v) = x_v · x_v; raw_P(a,b) = x_a · x_b (a before b in V order; skipped if
either variable is constant on P).
f(i) = (raw(i) − lo) / sc, lo = min_P raw, hi = max_P raw, sc = hi − lo, and sc := 1 if sc = 0.
(Quadratic = square of the **raw** variable, then scaled; not the square of the scaled variable.)
Evaluated identically for samples (may fall outside [0,1] only if the sample is not in P).
NOTE (doClamp=true): before scaling, raw values are clamped to [min_P raw, max_P raw]; affects only
samples not in P.

### 4.2 Knot set for a variable (H, T) — `SortedFeatureGenerator:99`
Let z be the raw values of the generator's variable (forward: x_v; reverse: −x_v) over the
concatenation (P in order, then S in order). Stable-sort ascending → values u_0 ≤ … ≤ u_{K−1}
(K = N + m). Compute prec = min_k precision(u_k) with

```
precision(f):                       # SortedFeatureGenerator.java:78, maxDigits = 6
  f = |f|
  e = floor(ln(f)/ln(10))           # f=0 -> -inf
  p = 10^e                          # f=0 -> 0
  last2 = 0
  for i in 1..6:
     d = javaInt(f / p)             # NaN->0, trunc toward 0, saturating
     last2 = (last2*10 + d) mod 100
     if i >= 2 and last2 in {0, 1, 99, 98}: return p*50
     f = f - d*p ; p = p/10
  return p*5
```
Then: `last = u_0`; for k = 0..K−1: if u_k − last > prec then emit knot t = (u_k + last)/2, set
`last = u_k`, and record valToThr[k] = knot index; else valToThr[k] = −1.
M = u_{K−1} (max over P∪S), u_min = u_0.

### 4.3 Hinge features — `HingeFeatureGenerator`
For each variable v (in V order): a forward generator on z = x_v named `'v` and a reverse
generator on z = −x_v (internal name `v__rev`, serialized as `` `v ``). Every knot t of §4.2 is a
candidate:
h_t(i) = (z(i) − t) / (M − t) if z(i) > t, else 0.
Serialized bounds: forward `'v, λ, t, M`; reverse `` `v, λ, −M_rev, −t `` = `` `v, λ, min_x, k `` with
k = −t, i.e. h(x) = (k − x)/(k − min_x) for x < k.

### 4.4 Threshold features — `ThrFeatureGenerator`
Knots from §4.2 restricted to indices [thrFirst, thrLast): thrFirst = first knot with
x(firstSample) ≤ t, thrLast = one past the last knot with x(lastSample) > t, where first/last sample
are the lowest/highest samples in sorted order. f_t(i) = 1 if x(i) > t else 0. Serialized
`(t<v), λ, 0.0, 1.0` with t printed by `Double.toString`.

### 4.5 Categorical — `BinaryFeature`
For each distinct value c (as double, then cast to float32) of x_v over P, sorted ascending:
f(i) = 1 if (float)x == c else 0. Name `(v=c)` with c via `Float.toString`. The first category
is flagged (always serialized).

### 4.6 Feature list order (determines tie-breaks and `.lambdas` order)
X.features := [C features (per categorical variable in V order, categories ascending)] ++
[L for each continuous v] ++ [Q for each v] ++ [P for pairs i<j] — only features that are active
or linear (inactive linear features kept with λ=0). Generators (separate list): [T for each v] ++
[H forward v, H reverse v for each v]. Exported hinge/threshold features are **appended** to
X.features at first selection.

## 5. Regularization

### 5.1 Class values — `Runner.autoSetBeta:2253`
`interp(xs, ys, m)`: i = first index with m ≤ xs[i]; i=0 → ys[0]; i=len → ys[last];
else ys[i−1] + (ys[i]−ys[i−1])·(m − xs[i−1])/(xs[i] − xs[i−1]).

| Class | xs | ys | Selected when |
|---|---|---|---|
| lqp (product table) | 0,10,17,30,100 | 2.6,1.6,0.9,0.55,0.05 | product on ∧ (¬autofeature ∨ m ≥ 80) |
| lqp (quadratic table) | 0,10,17,30,100 | 1.3,0.8,0.5,0.25,0.05 | else quadratic on ∧ (¬autofeature ∨ m ≥ 10) |
| lqp (linear table) | 10,30,100 | 1.0,0.2,0.05 | otherwise |
| threshold | 0,100 | 2.0,1.0 | |
| hinge | — | 0.5 | |
| categorical | 0,10,17 | 0.65,0.5,0.25 | |

Overrides `beta_lqp|beta_threshold|beta_hinge|beta_categorical ≥ 0` replace the class value.
β^cls = class value × bm. minDev = 0.001 × bm.

### 5.2 Width for non-generator features — `FeaturedSpace.setSampleExpectations:469`
With S_j = samples with data (= S in compat mode), m_j = |S_j|:
```
r   = max_P f - min_P f                       # = 1 for scaled features
if m_j == 0: avg=(min+max)/2, sd=r/2, feature inactive
elif m_j == 1: avg = f(s), sd = r/2
else: avg = (Σ f(s))/m_j ; q = Σ f(s)^2
      sd = (q < m_j*avg*avg) ? 0 : sqrt((q - m_j*avg*avg)/(m_j-1)) ; sd = min(sd, r/2)
lo = avg - β^cls/sqrt(m_j)*sd ; hi = avg + β^cls/sqrt(m_j)*sd
π̃ = 0.5*(lo+hi) ; β_j = 0.5*(hi-lo)
if β_j < minDev: β_j = (binary ∧ π̃ == 1 ∧ m_j > 0) ? 1/(2 m_j) : minDev
```
(Sums left-to-right over samples in input order. The exact `lo/hi` form MUST be kept: π̃ may
differ from avg in the last bit.)

### 5.3 Width for hinge knots — `HingeFeatureGenerator.setSampleExpectations:42`
Sweep k = K−1 down to 0 over the sorted values; when k is a sample with value z: s1 += z,
s2 += z², w1 += 1, w2 += 1. When valToThr[k] = t_idx ≥ 0 with knot t, R = M − t:
```
c1 = (s1 - t*w1)/R ; c2 = (s2 - 2*s1*t + t*t*w2)/R/R
avg = c1/m ; sd = 0.5                       # m == 1 keeps sd = 0.5
if m > 1: sd = (c2 < m*avg*avg) ? 0 : sqrt((c2 - m*avg*avg)/(m-1))
          sd = min(sd, 0.5) ; sd = max(sd, 1/sqrt(m))
lo, hi, π̃, β as in 5.2 with β^cls = β_hinge ; β = max(β, minDev)
```

### 5.4 Width for threshold knots — `ThrFeatureGenerator.setSampleExpectations:50`
Same sweep with s1 += 1, s2 += 1 per sample; m==1: avg = s1, sd = 0.5; m>1: avg = s1/m,
sd = sample sd capped at 0.5 (no floor); interval and minDev floor as above.

### 5.5 Deactivation
Binary/categorical features with π̃ = 0 are deactivated after construction (`Runner.maxentRun`).

## 6. Objective

ℒ(λ) = −Σ_j λ_j π̃_j + L + ln Z + Σ_j β_j |λ_j|, over j ∈ X.features (exported knots included).
Regularized training gain G = ln N − ℒ(λ̂); unregularized gain = G + Σ β_j |λ_j|.

## 7. Optimizer — faithful pseudocode of `Sequential.java`

Parameters: maxIt = `maximumiterations` (500), conv = `convergencethreshold` (1e-5),
puf = `parallelUpdateFrequency` (30), updateCycle = 20, recentChange = 10, topSelect = 5,
convergenceTestFrequency = 20.

```
state per feature: λ=0, prevλ=0, contrib=0, prevContrib=0, W, lastChange=-1, lastExpUpd=-1
init: lp=0, L=0 (lp[0]), d_i=1, Z=N, W_j for active non-generated, generator W (7.9); reg = Σβ|λ| = 0

run():
  newLoss = loss()                                 # = ln N
  for it = 0 .. maxIt-1:
     oldLoss = newLoss
     if it > 0 and puf > 0 and it % puf == 0: newLoss = parallelUpdate()
     else:
        h = bestFeature(); if h == null: break
        newLoss = sequentialUpdate(h)
     if terminationTest(newLoss): break
  return loss()                                    # iterations reported = it at exit

loss() = -Σ λ_j π̃_j + L + ln Z + reg

goodAlpha(h):                                      # :294
  N1=π̃_h, W1=W_h, W0=1-W1, N0=1-N1, b=β_h, λ=λ_h
  if W0 < ε or W1 < ε: return 0
  if N1-b > ε and (α = ln((N1-b)W0/((N0+b)W1))) + λ > 0: return α
  if N0-b > ε and (α = ln((N1+b)W0/((N0-b)W1))) + λ < 0: return α
  return -λ                                        # NaN/Inf -> fatal in jar

bound(h):                                          # deltaLossBound :325
  if !active(h): return 0
  α = goodAlpha(h); if isInf(α): return 0
  v = -N1 α + ln(W0 + W1 e^α) + b(|λ+α| - |λ|)
  return isNaN(v) ? 0 : v

bestFeature():                                     # :66
  best = 1.0; choice = null
  for f in X.features (list order): if active ∧ ¬generated ∧ bound(f) < best: choice=f, best=bound
  for g in generators (order), for knot k in [first,last): if bound(g,k) < best: choice=(g,k), best
  if choice is a generator knot: f = exported feature of (g,k) or export it now:
       new feature (generated=true, π̃,β,W copied from generator, beta=g.beta), append to X.features
  return f

featuresToUpdate():                                # :397  (uses current it)
  U = [f in X.features (order) : active ∧ ¬generated ∧ (it < f.lastChange+10 ∨ idx(f)%20 == it%20)]
  D = [bound(f) for f in X.features]
  ord = DoubleIndexSort(D)                         # Bentley-McIlroy quicksort port, ascending
  for r in 0 .. min(|X.features|,5)-1: f = X.features[ord[r]];
       if active ∧ ¬generated ∧ f ∉ U: U.append(f)
  return U

deriv(h): u = W-π̃; λ>0: u+β; λ<0: u-β; λ=0: (u+β>0) ? u+β : (u-β<0) ? u-β : 0

newton1(h):                                        # :223
  var = Σ d_i h_i² / Z - W_h²  (Σ left-to-right) ; if var < ε²: return 0
  s = -deriv(h)/var ; if (s+λ)λ < 0: s = -λ ; return s

damp(α): it<10: α/50 ; it<20: α/10 ; it<50: α/3 ; else α

applyOne(h, α, U):                                 # Sequential.increaseLambda(h,..) :387
  reg += (|λ_h+α| - |λ_h|) β_h
  if α != 0:
     λ_h += α ; for i: lp_i += α h_i ; L = max_i lp_i
     d_i = exp(lp_i - L) ; Z = Σ d_i ; for f in U: W_f = Σ d_i f_i / Z ; update all generators (7.9)
  for f in U: f.lastExpUpd = it
  return loss()

lossChange(h, α) = -α π̃_h + ln(Σ_i d_i e^{α h_i}) + (|λ+α|-|λ|)β_h

search(h, α):                                      # :104
  cur = lossChange(h, α)
  loop: t = lossChange(h, 4α); if t >= cur or isNaN/Inf(t): break; cur = t; α = 4α
  t = lossChange(h, 2α); if t < cur and finite(t): α = 2α
  return α

sequentialUpdate(h):                               # :419
  λold = λ_h
  if ¬generated(h) and h.lastExpUpd != it-1: W_h = Σ d_i h_i / Z ; h.lastExpUpd = it
  h.lastChange = it
  U = featuresToUpdate() ; dlb = bound(h)
  if binary(h): α = damp(goodAlpha(h)); new = applyOne(h, α, U)
  else:
     α = damp(newton1(h)); new = applyOne(h, α, U)
     if new - oldLoss > dlb:
        applyOne(h, -α, [h]); α = damp(search(h, goodAlpha(h))); new = applyOne(h, α, U)
  if α·λold < 0: h.contrib = max(0, h.contrib + new - oldLoss) else h.contrib += oldLoss - new
  return new

parallelUpdate():                                  # :245
  for j (X.features order): u_j = (binary ∨ λ_j == 0) ? 0 : λ_j - prevλ_j ; prevλ_j = λ_j
                            cd_j = max(0, contrib_j - prevContrib_j) ; prevContrib_j = contrib_j
  Hs = {j : u_j != 0}
  for j∈Hs: W_j = Σ d_i f_j(i)/Z ; lastExpUpd_j = it          # exact refresh
  FTu_i = Σ_{j∈Hs} u_j f_j(i) ; m1 = Σ d_i FTu_i / Z ; var = Σ d_i FTu_i² / Z - m1²
  s = (var < ε²) ? 0 : -(Σ_j deriv_j u_j)/var
  if var >= ε²: for j: if (s u_j + λ_j) λ_j < 0: s = -λ_j/u_j
  for j: a_j = u_j s ; if a_j != -λ_j and |a_j + λ_j| < ε: a_j = -λ_j
  was = loss() ; U = featuresToUpdate()
  now = applyAll(a, U)
  if now > was: now = applyAll(-a, U)
  else: Δ = was - now ; cs = Σ_{a_j≠0} cd_j ; if cs > 0: contrib_j += Δ cd_j / cs for a_j≠0
  return now

applyAll(a, U):                                    # FeaturedSpace.increaseLambda(double[],..) :728
  for j: if a_j != 0: λ_j += a_j ; lp_i += a_j f_j(i)
  L = max_i lp_i ; recompute d, Z, W_f for f∈U, generators ; f.lastExpUpd = it for f∈U
  reg = Σ_j |λ_j| β_j ; return loss()

terminationTest(new):                              # :476
  if it == 0: prev = new; return false
  if it % 20 != 0: return false
  if prev - new < conv: return true
  if ln N - new > 10000: return true
  prev = new; return false
```

### 7.9 Generator expectations (`HingeFeatureGenerator.updateFeatureExpectations:87`)
Sweep k = K−1..0 over sorted values; if k is a point i: S1 += z_i d_i, S0 += d_i; if valToThr[k] =
t_idx: W(t) = (S1 − t·S0)/Z/(M − t). Threshold: S0 only, W(t) = S0/Z. Exported knots take W(t).

## 8. Post-fit quantities
- H = −Σ_{i∈P} p_i ln p_i, p_i = d_i/Z, skipping p_i = 0 (`FeaturedSpace.getEntropy:87`).
- numBackgroundPoints = N (includes added samples).
- Percent contribution: Σ contrib per variable (strip `( = ^2 ' \` < ) __rev`; product split
  equally), normalized to 100 (`Runner.determineContributions:1737`).

## 9. Prediction (`Project.projectGrid:137`)

Input values are cast to **float32** (jar behaviour). Let s(x) = Σ_{λ_j≠0} λ_j φ_j(x) with, under
`doclamp=true`:
- linear/quadratic/product: c_v = clamp(x_v, lo_v, hi_v) with (lo_v,hi_v) from the linear line of v
  (float32 result); φ = clamp((raw(c) − lo)/(hi − lo), 0, 1) where raw(c) = c_v, c_v², c_a·c_b
  (linear uses the unclamped x_v, then clamps φ to [0,1] — equivalent).
- hinge `'v, λ, a, b`: φ = 0 if x ≤ a; 1 if x > b; else (x − a)/(b − a).
- reverse hinge `` `v, λ, a, b ``: with y = −x, a' = −b, b' = −a: φ = 0 if y ≤ a'; 1 if y > b';
  else (y − a')/(b' − a').
- threshold `(t<v)`: φ = 1 if x ≥ t else 0 (NOTE: training used `>`).
- categorical `(v=c)`: φ = 1 if x == c.
Without clamping, the clamps are removed (hinges extrapolate linearly).

Outputs: raw = min(1, exp(s − L)/Z); cloglog = 1 − exp(−e^H·raw);
logistic = τ e^H raw / ((1−τ) + τ e^H raw), τ = 0.5; if NaN/Inf: (s > L) ? 1 : 0.
`extrapolate=false`: 0 if any linear variable is outside [lo_v, hi_v].
`fadebyclamping`: pred = max(0, pred − |pred_clamped − pred_unclamped|).
NODATA if any variable referenced in the lambdas file is NODATA.
Cumulative (training): for background weights w (sorted) with cumulative cw_k = 100·Σ_{l≤k} w_l /Σw,
value at r = linear interpolation (`Runner.interpolate`); projection interpolates from the
`_omission.csv` subsample (`Project.interpolateCumulative:552`).

## 10. `.lambdas` grammar (`FeaturedSpace.writeWeights:750`)

```
file      := featline* footer
featline  := name ", " num ", " num ", " num EOL
footer    := "linearPredictorNormalizer, " num EOL
             "densityNormalizer, " num EOL
             "numBackgroundPoints, " int EOL
             "entropy, " num EOL
num       := Java Double.toString (e.g. "0.0", "-3.948371134077774", "1.0E-4", "2.5E7")
name      := var                    (linear; min,max = lo,hi)
           | var "^2"               (quadratic; lo,hi of x²)
           | var "*" var            (product; lo,hi of xa·xb)
           | "'" var                (hinge; knot, M)
           | "`" var                (reverse hinge; min_x, knot)
           | "(" num "<" var ")"    (threshold; 0.0, 1.0)
           | "(" var "=" fnum ")"   (categorical; 0.0, 1.0; fnum = Float.toString)
```
Writer rules: iterate X.features in order; skip a feature if λ = 0 AND (an earlier line was written
OR it is not the last feature) AND it is not linear AND it is not a first-category binary.
Reader rules (jar): split on ','; parse with `Double.parseDouble` (tolerates surrounding blanks);
classify the name by first match of: contains `*` → product (split at first `*`); contains `^2`;
`^p`; `=`; `<`; `'`; `` ` ``; footer keyword; else linear. Missing min/max ⇒ 0.0, 1.0. Lines with
λ = 0 are ignored except linear (clamping). Variable names MUST NOT contain `, * ^ = < ' \` ( )`.

## 11. Testable specification items

Test harness (all items): datasets = synthetic SWD (`reference/vendor/oracle/gen*.py`) + a corpus
of real SWD exports; jar command:

```
java -Djava.awt.headless=true -Duser.language=en -Duser.country=US -Xmx2g -jar maxent.jar \
  samplesfile=SP.csv environmentallayers=BG.csv outputdirectory=OUT visible=false autorun=true \
  warnings=false askoverwrite=false pictures=false plots=false responsecurves=false jackknife=false \
  writebackgroundpredictions=true [verbose] [betamultiplier=..] [feature flags]
```
Compare against `OUT/<sp>.lambdas`, `maxentResults.csv`, `maxent.log` ("Regularization values",
"N samples", per-iteration trace with `verbose`), `<sp>_samplePredictions.csv` (double),
`<sp>.csv` (float32 SWD projection). Tolerances below are for compat mode; "Tier B" applies only if
a test is explicitly marked path-divergent.

| ID | Requirement | Tolerance | How to test vs jar |
|---|---|---|---|
| SPEC-001 | Variables sorted by Java String order | exact | `Layers:` line in log |
| SPEC-002 | SWD background rows with any NODATA dropped; no subsampling | exact count | `#Background points` (with addsamples off) |
| SPEC-003 | Sample filtering + de-dup key `lat+""+lon` | exact m | log "`m` samples", `#Training samples` |
| SPEC-004 | Add-samples rule (vector not in B) | exact N | `numBackgroundPoints` |
| SPEC-005 | Auto classes thresholds 10/15/80; T off by default | exact set | feature names present in `.lambdas` across m ∈ {5,9,10,14,15,79,80} |
| SPEC-006 | Class β tables + interp + overrides | ≤1e-12 abs | log "Regularization values" (3 decimals) + unit tests on interp |
| SPEC-007 | L/Q/P scaling bounds lo,hi over P (raw x², raw xa·xb) | exact (bit) | 3rd/4th fields of L/Q/P lines |
| SPEC-008 | `precision()` and knot rule (§4.2) | exact (bit) | knot values on hinge lines; unit tests incl. 0, 1000, 0.0015, negatives |
| SPEC-009 | Forward/reverse hinge evaluation and serialization | exact | hinge lines `'v,t,M` / `` `v,min,k `` |
| SPEC-010 | Sample expectation/width (5.2) incl. lo/hi midpoint form, caps, minDev | ≤1 ulp | oracle unit tests; jar via `verbose` (N1) to 6 dp |
| SPEC-011 | Hinge knot widths (5.3) clamp [1/√m, ½] | ≤1 ulp | oracle; indirectly via selected path |
| SPEC-012 | Threshold knot range & widths (5.4) | ≤1 ulp | `threshold=true` runs, `(t<v)` lines |
| SPEC-013 | Categorical binaries, float cast, deactivation π̃=0, first category written | exact | `togglelayertype` runs |
| SPEC-014 | Initial loss = ln N | ≤1e-15 rel | log "Initial loss" |
| SPEC-015 | goodAlpha / bound formulas | ≤1e-14 rel | per-iteration `alpha`, `lossBound`, `W1`, `N1` (verbose, 6 dp) |
| SPEC-016 | bestFeature order & strict `<` tie-break; lazy export & append | identical sequence | sequence of selected feature names (verbose) = `.lambdas` order of generated features |
| SPEC-017 | Newton step + sign truncation + damping 1/50,1/10,1/3 | ≤1e-12 rel on α | verbose `alpha` per iteration |
| SPEC-018 | Bound-violation fallback to search (×4 then ×2) | identical decisions | verbose trace; oracle |
| SPEC-019 | Stale-expectation policy (featuresToUpdate, recentChange 10, cycle 20, top 5, DoubleIndexSort) | identical decisions | oracle step-by-step; DoubleIndexSort unit tests vs Java on random arrays with ties |
| SPEC-020 | Parallel update every 30 iterations (u = Δλ, Newton, snap 1e-6, revert) | identical decisions | verbose "Updating k features", oracle |
| SPEC-021 | Termination every 20 its, prev−new < 1e-5, gain>10000, max 500 | exact iteration count | `Iterations` in `maxentResults.csv` |
| SPEC-022 | Max-shift density (L = max lp), Z, loss bookkeeping (incremental reg, full after parallel) | ≤1e-12 rel | `linearPredictorNormalizer`, `densityNormalizer` |
| SPEC-023 | Final coefficients | same feature list & order; \|Δλ\| ≤ 1e-9·max(1,\|λ\|) (observed ≤5e-12) | parse `.lambdas` |
| SPEC-024 | Regularized/unregularized training gain | ≤1e-10 abs | `maxentResults.csv` (4 dp) + log "Resulting gain" (full) |
| SPEC-025 | Entropy over P | ≤1e-10 abs | `entropy` footer |
| SPEC-026 | raw/cloglog/logistic formulas (§9) | f64 path: ≤1e-12 rel vs oracle; vs samplePredictions.csv ≤1e-9 abs | `_samplePredictions.csv` |
| SPEC-027 | Float32 input cast at projection | ≤1e-6 abs on cloglog | `<sp>.csv` (SWD projection, observed 5.8e-8) |
| SPEC-028 | Clamping (L/Q/P via clamped vars, hinge saturation) | ≤1e-6 abs | projection SWD file with out-of-range rows (`projectionlayers=`) |
| SPEC-029 | `extrapolate=false`, `fadebyclamping`, clamp grid | ≤1e-6 abs | same, flags toggled |
| SPEC-030 | Threshold projection uses `>=` | exact | crafted value equal to knot |
| SPEC-031 | `.lambdas` writer: order, skip rules, Double.toString format | numeric equality per SPEC-023; textual equality optional (JDK-dependent) | diff after numeric parse |
| SPEC-032 | `.lambdas` reader accepts jar files (all 3.4.x name forms, sci notation) | exact parse | round-trip jar files; predictions per SPEC-027 |
| SPEC-033 | Percent contribution | ≤1e-6 abs (percent) | `<var> contribution` columns |
| SPEC-034 | Permutation importance (Random(11111), DoubleIndexSort permutation, AUC) | ≤1e-6 abs if RNG ported; else Tier B ≤2 pp. **Implemented** (`niche-maxent/src/explain.rs`, `java.util.Random` ported): checked ≤1e-4 abs, the precision of the 4-dp reference | `permutation importance` columns |
| SPEC-035 | Training AUC (Mann–Whitney with ties, background incl. added samples) | ≤1e-12 | `Training AUC` (4 dp) |
| SPEC-036 | Cumulative output (training interpolation; projection via omission table) | ≤1e-6 abs | `backgroundPredictions.csv` cumulative column (9 dp) |
| SPEC-037 | betamultiplier scales β^cls AND minDev | as SPEC-023 | runs with bm ∈ {0.5,1,2.5} |
| SPEC-038 | `addsamplestobackground=false` ⇒ sample clamping at training | as SPEC-023 | flag toggled with out-of-range samples |
| SPEC-039 | Determinism: identical inputs ⇒ bit-identical outputs across runs/threads | bit-exact | run twice, hash outputs |
| SPEC-040 | Tier B (path-divergent datasets only): objective equivalence | \|ΔG\| ≤ 1e-4; cloglog max abs ≤ 0.01; Spearman(raw) ≥ 0.999 | flagged in test manifest with reason |
| SPEC-041 | Grid background: java.util.Random(0) reservoir sampling, row-major, all-layer data | exact cell set | `backgroundPredictions.csv` x,y list (`randomseed=false`) |
| SPEC-043 | Marginal response curves (`createProfiles`): other variables at presence mean (mode if categorical), range of training background ± 10 %, 501 points, clamped, output format | x ≤1e-9 rel, y ≤1e-5 abs (cloglog, as G1). **Implemented** (`explain.rs`) | `plots/<sp>_<var>.dat` (`responsecurves=true writeplotdata=true`; `scripts/gen_response_refs.py`) |
| SPEC-042 | Logs/metadata record engine version, compat mode, m, N, betas, iterations, stop reason | present | NicheRS-internal check |

## 12. Known jar quirks (decide: reproduce in compat mode, document elsewhere)

1. Threshold `>` (train) vs `>=` (project). 2. Float32 projection inputs. 3. SWD dedup key without
separator. 4. `minDeviation` is a static mutated per run (multi-threaded jar runs with different
bm could interfere). 5. Cumulative projection interpolated from a subsampled table.
6. `backgroundPredictions.csv` raw printed with 6 significant digits — never use it for tight tests.
7. Verbose log numbers use the JVM default locale.
