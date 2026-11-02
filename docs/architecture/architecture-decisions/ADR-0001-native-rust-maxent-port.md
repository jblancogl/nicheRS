# ADR-0001 — Native Rust MaxEnt engine, validated against maxent.jar 3.4.4

- Status: Accepted (feasibility gate G1 passed, 2026-10-15)
- Deciders: NicheRS engineering
- Evidence: `docs/research/02-maxent-internals.md`, `docs/research/04-language-and-runtime.md`, `docs/benchmarks/phase3-results.md`

## Context
NicheRS needs a MaxEnt engine that runs inside one executable without Java, R or Python, is
reproducible, and gives results ecologists already trust. maxent.jar 3.4.4 (MIT, frozen since
2020-11) is the de-facto reference; maxnet/elapid use glmnet (GPL-2) and a different
regularization path and hinge-knot scheme, so they are not numerically equivalent.

## Decision
Port the MaxEnt 3.4.4 training algorithm (feature construction, regularization, sequential
coordinate descent with its bookkeeping, `.lambdas` format, projection transforms) to Rust as
`niche-maxent`, *path-faithful* to the Java implementation. maxent.jar is a test oracle only and is
not shipped.

## Alternatives
| Option | Why not (now) |
|---|---|
| Ship maxent.jar + bundled JRE | Violates single-binary/no-Java runtime; JRE adds ~40–60 MB; JVM start-up per job. Kept as offline oracle. |
| Port maxnet (glmnet path) | glmnet is GPL-2; not equivalent to the jar users know. |
| Generic convex solver on the MaxEnt objective | Lands on a different (earlier-stopped vs fully converged) solution; jar stops after damped steps. |
| Go | No evidence of a numerical or ecosystem advantage; GC adds variance; team preference is Rust. |

## Evidence
- 11/11 controlled cases (synthetic, 3 600–50 000 background points, 12–500 presences, categorical,
  missing data, β ∈ {0.5,1,2}, l/q/p/t/h subsets) reproduce the jar's feature set and order,
  iteration count and λ to ≤ 2·10⁻¹¹ (tolerance 10⁻⁶); cloglog background predictions to
  ≤ 5·10⁻¹⁰ (jar prints 6 significant digits).
- Rust training (feature construction + optimizer, single thread) took 0.45–0.66× the time of the
  jar's optimizer alone with a warm JIT (i.e. 1.5–2.2× faster) across the 10 non-trivial cases;
  process peak RSS 7–35× lower (JVM heap untuned, `-Xmx4g`). Measured on one 8-core Linux machine
  with synthetic data only — not a general claim.

## Trade-offs / risks
- Path-faithfulness binds us to quirks (stale expectation cache, unstable quicksort ties,
  `precision()` threshold rule). Tiny `exp`/`ln` ulp differences could flip a tie on some
  dataset → mitigated by golden tests on more datasets (real species, raster backgrounds).
- Not yet covered: raster-based background sampling (RNG), bias files, replicates/CV, clamping
  grids, jackknife, permutation importance, response curves.

## Reversibility
High. The kernel sits behind a small API (`train`, `MaxentModel::compile/predict`); the jar can be
wrapped as an alternative engine in a subprocess if a blocker appears.
