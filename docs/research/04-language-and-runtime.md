# 04 — Language and runtime feasibility for the NicheRS compute core

Status: research draft · Date: 2026-10-15 · Author: research agent (NicheRS)
Tags: [FACT] verified in a source read for this document · [ASSUMPTION] · [OPEN] · [REC] · [UNVERIFIED – prior knowledge]

## 1. Scope

Which language/runtime should implement (a) the MaxEnt training core, (b) raster inference, and
(c) the orchestration/API layer, for a platform that must run hundreds of species with
scientific correctness and reproducibility ranked above performance (see `_BRIEF.md`).
Candidates: native Rust, native Go, existing MaxEnt Java (`maxent.jar`), Python (elapid /
scikit-learn), R (maxnet / glmnet), hybrids, and a native library behind a standalone service.

Out of scope: frontend stack, geospatial I/O details (other docs), algorithm choice beyond MaxEnt.

No benchmarks were run for this document. Every performance statement below is either a
cited fact, a complexity argument, or an explicitly labelled assumption. Measurements are
deferred to `docs/benchmarks/methodology.md`.

## 2. Findings

### 2.1 What the workload actually is

| Aspect | Observation | Tag / ref |
|---|---|---|
| Maxent Java training algorithm | Sequential (coordinate-wise) update of feature weights, with an optional periodic "parallel update" step every `parallelUpdateFrequency=10` iterations; this is algorithmic, not multithreading (`density/Sequential.java`). | [FACT] [1] |
| Default limits | `maximumiterations=500`, `convergenceThreshold=0.00001`, `maximumbackground=10000`, `betaMultiplier=1`, output `cloglog` (`density/parameters.csv`). | [FACT] [1] |
| Threading in maxent.jar | `threads` parameter (default 1, tagged EXPERIMENTAL): "speeds up some operations, especially variable jackknifing". Code uses a fixed thread pool for replicates/jackknife/response curves/pictures (`Runner.java`, `ParallelRun.java`). The core optimiser loop is single-threaded. | [FACT] [1] |
| Hot loop | Each iteration evaluates `exp(linearPredictor - normalizer)` over all background points (`FeaturedSpace.java` line ~693). Cost per iteration ~ O(n_background × features touched). | [FACT] code read [1]; complexity [ASSUMPTION] |
| Matrix shapes | ≤10 000 background + occurrences rows × (tens to low thousands of derived features). Small, column-oriented, not dense-decomposition-bound. | [ASSUMPTION] derived from defaults |
| maxnet | Same model structure as Maxent 3.4.0, fitted as an L1-penalised GLM via `glmnet`; 481 lines of R in total. | [FACT] [2][3] |
| maxnet vs maxent.jar | ENMeval documentation: differences "may lead to slight numeric differences in the results, especially when hinge feature classes are used". | [FACT] [4] |
| Inference | Per raster cell: evaluate active features + one `exp`; embarrassingly parallel over cells, independent of other cells. | [FACT] model form [5][6]; parallelism [ASSUMPTION] |

Consequence [ASSUMPTION]: the training core is latency- and transcendental-function-bound on
small matrices; BLAS-class linear algebra (faer vs OpenBLAS) is unlikely to be the decisive
factor. Language choice matters more for (i) raster inference over tens of millions of cells,
(ii) multi-species/multi-config orchestration, (iii) memory footprint and deployment, (iv)
reproducibility control. This must be confirmed by profiling maxent.jar (benchmark B1/B2 in the
methodology).

### 2.2 Reference implementations (oracles)

| Implementation | Version / date | Licence | Notes | Ref |
|---|---|---|---|---|
| maxent.jar (Java) | 3.4.4, GitHub release 2020-11-24; repo last push 2024-01-11 | MIT | 16 752 Java lines in `density/` (incl. GUI); ~2 800 lines in optimiser + feature classes | [FACT] [1] |
| maxnet (R) | 0.1.4, CRAN 2021-07-09 | MIT + file LICENSE | Imports `glmnet` | [FACT] [3] |
| glmnet (R) | 5.1, CRAN 2026-09-24 | **GPL-2** | Coordinate descent, Friedman et al. 2010 | [FACT] [3][7] |
| elapid (Python) | 1.0.4; repo push 2026-06-11 | MIT | Uses `glmnet.LogitNet` if importable, else sklearn `LogisticRegression(solver="liblinear", l1_ratio=1.0)` | [FACT] [8] |
| python-glmnet | 2.2.1, last upload 2020-06-30 | **GPL-2** | Fortran build (elapid comments on Windows gfortran issues) | [FACT] [8][9] |
| scikit-learn | 1.9.1 | BSD-3-Clause | liblinear L1 logistic | [FACT] [9] |
| ENMeval (R) | 2.0.6, 2026-09-11 | GPL-3 | Tuning harness; maxnet default since 0.3.0 | [FACT] [3][4] |
| Native Rust MaxEnt (SDM) crate | none found on crates.io (search "maxent" returns only NLP text classifiers) | — | Rewrite would be greenfield | [FACT] [10] |

Licence implication [FACT + REC]: maxnet's MIT licence does not cover its GPL-2 `glmnet`
dependency; embedding R/maxnet or python-glmnet into a proprietary single binary raises GPL
obligations. maxent.jar (MIT) and a clean-room Rust port derived from the MIT Java source do not.

### 2.3 Language/runtime capabilities (verified)

**Rust crates (crates.io API, queried 2026-10-15)** [FACT] [10]

| Crate | Latest stable | Licence | Last update | Relevance |
|---|---|---|---|---|
| ndarray | 0.17.2 | MIT OR Apache-2.0 | 2026-01-10 | n-d arrays, views over raster tiles |
| faer | 0.24.4 | MIT | 2026-06-24 | pure-Rust dense LA; repo moved to Codeberg [11] |
| nalgebra | 0.35.0 | Apache-2.0 | 2026-05-24 | small fixed-size LA (less relevant) |
| argmin | 0.11.0 | MIT OR Apache-2.0 | 2025-09-28 | generic optimisers (L-BFGS etc.) |
| linfa / linfa-logistic | 0.8.1 | MIT OR Apache-2.0 | 2025-12-23 | logistic regression; no MaxEnt features |
| polars | 0.55.2 | MIT | 2026-08-06 | tabular occurrences/eval tables |
| pulp | 0.22.3 | MIT | 2026-06-20 | runtime-dispatched SIMD (used by faer) |
| wide | 1.7.1 | Zlib OR Apache-2.0 OR MIT | 2026-09-14 | stable-Rust SIMD types |
| rayon | 1.12.0 | MIT OR Apache-2.0 | 2026-04-14 | data parallelism |
| smartcore | 0.6.16 | Apache-2.0 | 2026-10-12 | alternative ML crate |
| gdal / gdal-sys | 0.19.0 / 0.12.0 | MIT | 2025-12-23 | GDAL bindings (C++ dep) |
| tiff | 0.11.3 | MIT | 2026-02-10 | pure-Rust TIFF decode |
| ndarray-linalg | 0.18.1 | MIT OR Apache-2.0 | 2026-01-12 | LAPACK bindings (needs system BLAS) |
| openblas-src / blas-src | 0.10.16 / 0.14.0 | Apache-2.0/MIT | 2026 / 2025 | native BLAS linkage |
| cargo-zigbuild / cross | 0.23.4 / 0.2.5 | MIT / MIT OR Apache-2.0 | 2026-09 / **2023-02** | cross-compilation |

- `std::simd` (portable_simd) is **still unstable** (tracking issue rust-lang/rust#86656 open;
  last maintainer status 2025-03-08: "still mostly blocked"). Stable alternatives: `pulp`, `wide`,
  `std::arch` intrinsics, or relying on auto-vectorisation. [FACT] [12]
- Rust float semantics (RFC 3514): IEEE 754-2008, round-to-nearest-even, no flush-to-zero; the
  compiler does not contract `a*b+c` into FMA; NaN bit patterns are non-deterministic; `exp`/`ln`
  accuracy is **not** specified by the RFC (platform libm). [FACT] [13]
- Rayon: float `sum`/`reduce` order is unspecified, results "not fully deterministic". [FACT] [14]

**Go** [FACT]
- gonum v0.17.0, BSD-3-Clause, active (push 2026-10-08); `optimize` provides BFGS, LBFGS, CG,
  Newton. [15]
- GC: concurrent mark, takes 25 % of CPU during mark; `GOGC` trades CPU vs heap ("doubling GOGC
  will double heap memory overheads and roughly halve GC CPU cost"); `GOMEMLIMIT` soft limit
  since 1.19. [16]
- Go 1.26: Green Tea GC enabled by default, release notes expect "10–40% reduction in garbage
  collection overhead in real-world programs that heavily use the garbage collector";
  `simd/archsimd` experimental, amd64 only, `GOEXPERIMENT=simd`, API not stable. [17]
- Large `[]float64` buffers contain no pointers; GC mark cost is proportional to live heap scanned
  [16] — so long-lived numeric arrays mostly cost memory headroom (GOGC), not scan time.
  [ASSUMPTION: no-pointer spans are not scanned — UNVERIFIED – prior knowledge]

**JVM (Java 25 locally: OpenJDK 25.0.4.1)** [FACT]
- `Math.exp` "must be within 1 ulp of the exact result" and `Math` methods are explicitly "not
  defined to return the bit-for-bit same results" across implementations; JIT may substitute
  intrinsics. `StrictMath` is the reproducible alternative. maxent.jar uses `Math.exp`. [18][1]
- Vector API is still incubating (JEP 508, tenth incubator, JDK 25). [19]
- JIT warm-up is not guaranteed to converge: Barrett et al. found at most 43.5 % of
  〈VM, benchmark〉 pairs consistently reached a peak steady state. [20]

**Python** [FACT]: free-threaded CPython is officially supported but optional in 3.14 (PEP 779,
phase II), single-thread overhead "around 10%". [21] NumPy 2.5.4 (BSD-3 etc.), rasterio 1.5.2,
xarray 2026.9.0, pyarrow 26.0.0. [9]

**R** [FACT]: terra 1.9-50 (GPL ≥3), dismo 1.3-16 (GPL ≥3), glmnet 5.1 (GPL-2). [3]

### 2.4 Comparative assessment

Ratings are qualitative engineering judgements [ASSUMPTION], not measurements.

| Criterion | Rust native | Go native | maxent.jar (JVM) | Python (elapid) | R (maxnet) |
|---|---|---|---|---|---|
| Algorithmic fidelity to Maxent 3.4.x | must be earned (port + validation) | must be earned | **reference** | glmnet path ≈ maxnet, not maxent.jar [4][8] | ≈ maxent.jar, hinge diffs [4] |
| Runtime perf evidence | none for SDM; no crate exists | none | no published benchmark found [4] | none found | anecdote: faster than jar via ENMeval [4] |
| Allocation / GC | no GC; explicit buffers | GC tunable (GOGC, GOMEMLIMIT) [16] | GC + JIT warm-up variance [20] | NumPy buffers, interpreter overhead | copy-on-modify semantics |
| Multicore | rayon (stable) | goroutines (stable) | thread pool; core loop single-threaded [1] | process pool / GIL (free-threading optional) [21] | `parallel`/future, process-based |
| SIMD | `pulp`/`wide` stable; `std::simd` unstable [12] | `simd/archsimd` experimental [17] | Vector API incubating [19] | via NumPy internals | via BLAS internals |
| Floating-point control | strict IEEE, no auto-FMA [13]; libm `exp` unspecified | [UNVERIFIED] | `Math.exp` ≤1 ulp, not bit-reproducible [18] | NumPy/BLAS dependent | BLAS dependent |
| Single binary / deploy | yes (static possible, GDAL complicates) | yes (cgo for GDAL) | needs JRE (jlink possible) | needs interpreter + wheels | needs R + compiled pkgs |
| Licence for embedding | MIT/Apache crates | BSD gonum | MIT | GPL-2 if glmnet [9] | GPL-2 glmnet [3] |
| Ecosystem maturity (numeric/geo) | moderate; LA strong, ML thin (linfa) | thin ML | n/a (self-contained) | strongest | strongest for SDM |
| Team productivity | slower initially [ASSUMPTION] | fast | n/a | fastest prototyping | fastest for ecologists |

### 2.5 Is a full native rewrite justified?

- **Performance alone does not justify it**: no evidence was found that maxent.jar's training is
  slow relative to the end-to-end pipeline, and the training kernel is small-matrix and
  `exp`-bound where JIT-compiled Java and native code may be comparable [ASSUMPTION]. "Rust is
  faster" is not assumed.
- **Arguments that do favour a native port** [REC]:
  1. Single executable with no JRE/R/Python runtime (brief's deployment goal).
  2. Control over determinism: fixed-order reductions, chosen `exp` implementation, pinned
     thread chunking — not available inside an unmodified maxent.jar using `Math.exp` [18].
  3. Shared in-memory feature matrices across hundreds of species/configurations (doc 08) — the
     jar re-reads layers per run and is driven by CLI/files.
  4. Inference fused with raster tiling and multi-species batching.
  5. Licence: avoids GPL-2 glmnet.
- **Scope is bounded**: core optimiser + feature classes ≈ 2 800 Java lines (maxnet: 481 R lines)
  [FACT] [1][2]. The risk is fidelity, not size.

## 3. Evidence and references

See §2 tables; numbered list in References. Local toolchain recorded: Intel i7-9700K (8 cores,
1 thread/core, AVX2+FMA, no AVX-512), 31 GB RAM, Fedora 44, kernel 7.2.9, OpenJDK 25.0.4.1, rustc
1.96.0, go1.26.5, uv 0.11.31 [FACT, local `lscpu`/`--version` 2026-10-15].

## 4. Alternatives

| ID | Option | Summary |
|---|---|---|
| A | Rust everything (port MaxEnt) | single binary; fidelity must be proven |
| B | Rust host + maxent.jar subprocess | fastest path to correct results; JRE dependency; file-based I/O |
| C | Rust host + JVM service (long-lived, warm) | amortises JIT; still JRE; IPC |
| D | Python service (elapid) behind Rust/TS API | rich ecosystem; maxnet-like not jar-like; GPL glmnet; 2 runtimes |
| E | R service (maxnet/ENMeval) | closest to ecologist practice; GPL; heavy runtime |
| F | Go everything | good concurrency/deploy; weaker numeric ecosystem; no SIMD stable |
| G | Native lib (Rust) exposed as standalone service + Python/R bindings | reusable by data scientists; extra API surface |

## 5. Trade-offs

- A vs B: A buys determinism, memory sharing and deployment at the cost of a validation programme;
  B buys immediate fidelity but inherits jar non-reproducibility (`Math.exp`) and per-run JVM
  start-up/JIT warm-up [20].
- F vs A: Go's GC cost is controllable and dominated by live heap scanning [16]; for large
  pointer-free float buffers the main cost is memory headroom. Go's lack of stable SIMD [17] and
  thinner numeric stack are the real gaps, not GC.
- D/E: best scientific ecosystem, worst single-binary story, and reference behaviour is maxnet,
  not maxent.jar.

## 6. Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Port diverges from maxent.jar (hinge/threshold generation, regularisation, clamping, cloglog) | High initially | Oracle tests vs jar 3.4.4 on fixed datasets, tolerances set before running (methodology §7) |
| Rayon/threads change float sums | Certain if naive [14] | Fixed chunk sizes + ordered tree reduction; or compensated (Kahan/pairwise) sums [22] |
| `exp` differs across platforms/libm | Medium [13] | Ship a pinned pure-Rust `exp` (e.g. `libm` 0.2.16, MIT, "libm in pure Rust" [10]) or accept ULP-level tolerance |
| faer API churn (0.x; moved to Codeberg) | Medium [10][11] | Keep LA behind an internal trait; core may not need faer at all |
| `cross` stale (last release 2023-02) | Medium [10] | Prefer `cargo-zigbuild` or native CI runners |
| GDAL C dependency breaks "single static binary" | High | See geospatial doc; pure-Rust `tiff` for COG read where possible |
| Rust productivity for a small team | Medium | Keep MaxEnt core small, well-tested; Python notebooks for validation |

## 7. Recommendations

1. [REC] **Hybrid A+B, staged**: Rust host (API, orchestration, raster I/O, inference). MaxEnt
   training ported to Rust **behind a validation gate**; maxent.jar 3.4.4 kept as the
   **oracle** (CI/validation harness, and optional fallback engine via subprocess) until the port
   passes the pre-registered tolerances on all benchmark datasets.
2. [REC] Do not adopt R/Python glmnet in the shipped product (GPL-2, multi-runtime); use maxnet/
   elapid only as secondary oracles to characterise expected inter-implementation spread [4].
3. [REC] Determinism contract in the Rust core: no float reductions through rayon's default
   `sum`; fixed partitioning; single pinned `exp`; record seed, thread count, crate versions,
   target CPU features in every run manifest.
4. [REC] Do not build on `std::simd` (unstable [12]). Start scalar + auto-vectorisation; add `pulp`
   or `wide` only where profiles show benefit.
5. [REC] Defer faer/BLAS: the training kernel likely needs no dense decompositions; revisit if a
   Newton/IRLS-type solver is introduced.
6. [REC] Before committing, run benchmarks B1–B4 (methodology) on maxent.jar to locate the real
   bottleneck.

**Conditions that would reverse the recommendation**
- Port cannot meet tolerances vs maxent.jar after a time-boxed effort → ship B/C (JVM engine).
- Profiling shows end-to-end time dominated by I/O/download/encoding, and jar training is a small
  fraction → native port loses its performance rationale; keep jar unless determinism/deploy
  still require it.
- Stakeholders require bit-for-bit maxent.jar outputs (regulatory reproducibility) → jar engine.
- Users prefer maxnet/ENMeval semantics (common in R-based consultancies) → R/Python service.
- Team lacks Rust capacity → Go host + jar, or Python service.

## 8. Open questions

- [OPEN] Actual time split in maxent.jar runs (feature generation vs iterations vs projection)
  for France 1 km/100 m with 10 000 background.
- [OPEN] Which `exp` implementation to pin in Rust, and whether ULP-level differences propagate to
  thresholded suitable-area estimates.
- [OPEN] Does maxent.jar's result depend on the order of background sampling/feature generation
  in ways that require replicating its RNG?
- [OPEN] Is maxent.jar's MIT licence (2016 copyright) applicable to all bundled third-party
  components (`license-3rd-party.txt` not reviewed here)?
- [OPEN] Expected scale of hyperparameter tuning (feature classes × regularisation × folds)
  per species — determines whether training dominates (doc 08).

## References

[1] mrmaxent/Maxent source, GitHub, release v3.4.4 (2020-11-24), files `density/Sequential.java`, `Runner.java`, `ParallelRun.java`, `parameters.csv`, `LICENSE.md`. https://github.com/mrmaxent/Maxent (accessed 2026-10-15)
[2] mrmaxent/maxnet source, `DESCRIPTION`, `R/*.R`. https://github.com/mrmaxent/maxnet (accessed 2026-10-15)
[3] CRAN metadata via crandb: maxnet 0.1.4, glmnet 5.1, dismo 1.3-16, ENMeval 2.0.6, terra 1.9-50. https://crandb.r-pkg.org/ (accessed 2026-10-15)
[4] ENMeval documentation / NEWS (maxnet vs maxent.jar differences, default algorithm). https://www.rdocumentation.org/packages/ENMeval/versions/0.3.1/topics/ENMevaluate ; https://cran.nics.utk.edu/cran/web/packages/ENMeval/NEWS ; Maxent Google Group https://groups.google.com/g/MaxEnt/c/XtAlMw_J-e4 (accessed 2026-10-15)
[5] Phillips S.J., Anderson R.P., Schapire R.E. (2006) Maximum entropy modeling of species geographic distributions. Ecol. Model. 190:231–259. doi:10.1016/j.ecolmodel.2005.03.026
[6] Phillips S.J., Dudík M. (2008) Modeling of species distributions with Maxent: new extensions and a comprehensive evaluation. Ecography 31:161–175. doi:10.1111/j.0906-7590.2007.5203.x
[7] Friedman J., Hastie T., Tibshirani R. (2010) Regularization paths for GLMs via coordinate descent. J. Stat. Softw. 33(1). doi:10.18637/jss.v033.i01
[8] earth-chris/elapid source `elapid/models.py` and docs https://earth-chris.github.io/elapid/sdm/maxent/ (accessed 2026-10-15)
[9] PyPI JSON API: elapid 1.0.4, scikit-learn 1.9.1, numpy 2.5.4, rasterio 1.5.2, xarray 2026.9.0, pyarrow 26.0.0, glum 3.4.1, glmnet 2.2.1 (GPL-2). https://pypi.org/ (accessed 2026-10-15)
[10] crates.io API, `/api/v1/crates/<name>` and search `q=maxent`. https://crates.io/ (accessed 2026-10-15)
[11] faer README (pure Rust; migrated to Codeberg). https://github.com/sarah-ek/faer-rs ; benchmarks https://faer.veganb.tw/benchmarks/ (accessed 2026-10-15)
[12] Tracking issue for portable SIMD, rust-lang/rust#86656. https://github.com/rust-lang/rust/issues/86656 (accessed 2026-10-15)
[13] RFC 3514 Float semantics. https://rust-lang.github.io/rfcs/3514-float-semantics.html (accessed 2026-10-15)
[14] rayon ParallelIterator::sum/reduce docs, rayon 1.12.0. https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html (accessed 2026-10-15)
[15] gonum v0.17.0 (BSD-3-Clause), optimize package. https://github.com/gonum/gonum ; https://pkg.go.dev/gonum.org/v1/gonum/optimize (accessed 2026-10-15)
[16] A Guide to the Go Garbage Collector. https://tip.golang.org/doc/gc-guide (accessed 2026-10-15)
[17] Go 1.26 Release Notes (Green Tea GC default; simd/archsimd). https://go.dev/doc/go1.26 (accessed 2026-10-15)
[18] Java SE 25 API, java.lang.Math. https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/lang/Math.html (accessed 2026-10-15)
[19] JEP 508: Vector API (Tenth Incubator), JDK 25. https://openjdk.org/jeps/508 (accessed 2026-10-15)
[20] Barrett E. et al. (2017) Virtual machine warmup blows hot and cold. Proc. ACM Program. Lang. 1(OOPSLA). doi:10.1145/3133876 ; arXiv:1602.00602
[21] PEP 779 – Criteria for supported status for free-threaded Python. https://peps.python.org/pep-0779/ (accessed 2026-10-15)
[22] Demmel J., Nguyen H.D. (2013) Fast reproducible floating-point summation. IEEE ARITH 21. doi:10.1109/ARITH.2013.9
