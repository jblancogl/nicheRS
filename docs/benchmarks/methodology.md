# Benchmark methodology (NicheRS)

Status: draft v0 · Date: 2026-10-15
Purpose: produce reproducible, comparable measurements of performance **and numerical
agreement** between NicheRS engines (Rust port) and reference engines (maxent.jar 3.4.4,
maxnet 0.1.4, elapid 1.0.4), stage by stage. Feeds decisions in research docs 04 and 08.

Tags: [FACT] verified source · [REC] · [ASSUMPTION] · [OPEN]

## 1. Principles

1. **Tolerances and regression thresholds are fixed and committed before the first run** (§7).
   Changing them later requires a dated, justified commit.
2. **Correctness before speed**: a performance number is reported only for a configuration that
   passed the numerical-agreement checks.
3. **Stage isolation**: each stage is timed separately with its inputs materialised on local disk
   beforehand; end-to-end runs are reported in addition, never instead.
4. **Everything recorded**: hardware, OS, toolchain, flags, input hashes, seeds, thread counts.
5. **No cross-machine comparisons** unless both machines are recorded and the comparison is labelled.

## 2. Stages and benchmark IDs

| ID | Stage | Unit of work | Engines |
|---|---|---|---|
| B0 | Occurrence/raster download | one species / one layer set | (network; reported, not compared) |
| B1 | Preprocessing (crop, reproject, align, mask) | one study area | GDAL CLI, Rust |
| B2 | Env extraction at points + background sampling | one species | jar, maxnet, elapid, Rust |
| B3 | Feature generation + single fit (fixed β, fixed features) | one fit | jar, maxnet, elapid, Rust |
| B4 | Raster inference (projection) of a fitted model | one model × study area | jar, maxnet (`predict`), elapid, Rust |
| B5 | Tuning grid (feature classes × β × k folds) | one species | ENMeval/maxnet, Rust |
| B6 | Multi-species batch (S = 10, 100, 500) | whole run | jar (scripted), Rust |
| B7 | Encoding (COG write, compression, overviews) | one map / S maps | GDAL, Rust |

For maxent.jar, B3/B4 isolation uses separate invocations (training only with
`outputgrids=false`-type options vs `density.Project` for projection) [ASSUMPTION — exact
flag set to be confirmed against `parameters.csv` in the jar source].

## 3. Environment recording

Captured automatically into `run-manifest.json` per benchmark session:

| Field | Command / source |
|---|---|
| CPU model, cores, threads/core, flags (avx2, fma, avx512*) | `lscpu`, `/proc/cpuinfo` |
| RAM, swap | `free -b`, `/proc/meminfo` |
| OS, kernel | `/etc/os-release`, `uname -r` |
| CPU governor / turbo | `/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor`, `/sys/devices/system/cpu/intel_pstate/no_turbo` |
| Toolchains | `java -version`, `rustc -vV`, `cargo -V`, `go version`, `uv --version`, `python -V`, `R --version` |
| Build flags | `RUSTFLAGS`, profile (`release`, `lto`, `codegen-units`), `target-cpu` (default generic **unless** stated) |
| Libraries | `Cargo.lock` hash, `uv.lock` hash, R `sessionInfo()`, jar SHA-256 |
| Input data | SHA-256 of every input file; dataset generator version + seed |
| Thread settings | `RAYON_NUM_THREADS`, `OMP_NUM_THREADS`, `OPENBLAS_NUM_THREADS`, `GDAL_NUM_THREADS`, jar `threads=` |
| JVM | `-Xmx`, GC selected, `-XX:+PrintFlagsFinal` subset |
| Background load | `uptime` load average before/after |

Reference machine at time of writing [FACT, local inspection 2026-10-15]: Intel Core i7-9700K
@ 3.60 GHz, 8 cores, 1 thread/core, AVX2 + FMA, no AVX-512; 31 GB RAM; Fedora 44, kernel
7.2.9-201.fc44; OpenJDK 25.0.4.1; rustc 1.96.0 (`~/.cargo/bin`); go 1.26.5; uv 0.11.31;
cgroup v2 controllers available: cpuset, cpu, io, memory. `hyperfine` and `perf` are **not
installed** yet.

Host hygiene [REC]: AC power, `performance` governor if permitted (record either way), no
other heavy processes, same file system for all inputs, drop or pre-warm page cache explicitly
(state which).

## 4. Datasets

All synthetic datasets are produced by a versioned generator with a fixed seed; outputs are
written as GeoTIFF + CSV so every engine reads identical bytes.

| ID | Description | Purpose |
|---|---|---|
| D1 | Synthetic gradients: 2–19 continuous layers (linear, radial, sinusoidal), known true response; 1 km-like grid of ~0.5 M cells | correctness against known truth; speed scaling in V |
| D2 | D1 + 1–3 categorical layers (5–50 classes) | categorical feature handling |
| D3 | D1 with missing data: NoData holes, mismatched masks, occurrences on NoData | masking/edge behaviour (agreement on *which* cells are excluded) |
| D4 | Background sizes 1 000 / 10 000 / 50 000 | training scaling in n |
| D5 | Occurrence counts 10 / 100 / 1 000 / 10 000 | small-sample feature-class rules, scaling |
| D6 | Regularisation grid β ∈ {0.5, 1, 1.5, 2, 3, 4, 6, 8} × feature-class sets {L, LQ, H, LQH, LQHP, LQHPT} | tuning workloads, B5 |
| D7 | Large grid: France-sized ~55 M cells × 19 layers (synthetic values) | inference / memory / I/O at 100 m |
| D8 | Real species × real covariates (e.g. WorldClim 2.1 [2] + GBIF occurrences) — **only with verified licences** (record DOI of GBIF download, layer licences) | realism; not used for synthetic-truth metrics |

Background points are sampled once per dataset by the harness and **passed to all engines**
(where the engine allows) so differences reflect the algorithm, not sampling. Where an engine
cannot accept external background, record that and compare distributions instead.

## 5. Measurement protocol

### 5.1 Timing

- **Process-level** (B1, B3–B7 via CLI): `hyperfine` with `--warmup`, `--runs`, `--prepare`
  (e.g. cache dropping for cold runs), `--export-json`; it flags statistical outliers [FACT] [3].
  Report cold and warm cache separately.
- **In-process micro/mesobenchmarks (Rust)**: `criterion` 0.8.2 (Apache-2.0 OR MIT, released
  2026-02-04) as default; `divan` 0.1.21 (MIT OR Apache-2.0) as an option, it offers
  `AllocProfiler` for allocation counts but its last release is 2025-04-10 [FACT] [4][5].
- **Java in-process**: JMH 1.37 (latest on Maven Central) for kernel-level comparisons [FACT] [6].
- **Python**: `time.perf_counter_ns` around the stage, interpreter start excluded; report
  imports separately.
- **R**: `bench::mark` or `system.time` around the stage [ASSUMPTION — confirm package].

### 5.2 Warm-up (JIT)

- JVM warm-up does not reliably reach a steady state: Barrett et al. found at most 43.5 % of
  〈VM, benchmark〉 pairs consistently reached peak steady state [FACT] [7].
- Therefore report **both** (a) cold single-invocation time (what a user experiences when the jar
  is launched per species) and (b) in-process time after N warm-up iterations, with N chosen
  per benchmark and the per-iteration time series stored to show whether steady state was reached.
- Apply the same two views to Rust/Go/Python for symmetry (cold process vs warm in-process).

### 5.3 Repetitions and statistics

- ≥ 10 measured runs per configuration for runs > 1 s; ≥ 30 for shorter; repeat whole sessions
  on ≥ 2 different days/boots for headline numbers (inter-invocation variance) [REC] following
  Kalibera & Jones [8] and Hoefler & Belli [9].
- Report **median and IQR** (plus min, max, n). Do not report mean ± SD as the headline.
- Comparisons: ratio of medians with a bootstrap 95 % CI (≥ 10 000 resamples, fixed seed);
  a difference is "real" only if the CI excludes 1.0 and exceeds the regression threshold (§7).
- Keep raw per-run data (JSON) in the repo under `docs/benchmarks/results/<date>-<machine>/`.

### 5.4 Memory

- Peak RSS: `/usr/bin/time -v` → "Maximum resident set size" per process.
- Whole process tree incl. children (JVM, R, GDAL subprocesses): run inside a transient cgroup,
  e.g. `systemd-run --user --scope -p MemoryAccounting=yes <cmd>`, then read `memory.peak` of the
  cgroup ("max memory usage recorded for the cgroup and its descendants"; writable to reset)
  [FACT] [10].
- Memory caps for out-of-core tests: `memory.max` (hard limit; OOM inside the cgroup) [FACT] [10].
- JVM: record `-Xmx` and report RSS, not heap, as the comparable number.
- Rust allocation counts: `divan::AllocProfiler` or `dhat` 0.3.3 (last release 2024-02) [FACT] [5].

### 5.5 CPU utilisation

- `/usr/bin/time -v` user + system time and "Percent of CPU this job got".
- cgroup `cpu.stat` (`usage_usec`, `user_usec`, `system_usec`) for process trees [FACT] [10].
- Parallel efficiency = T(1) / (p × T(p)) for p ∈ {1, 2, 4, 8} (hyperfine `-P threads 1 8`) [FACT] [3].
- Optional: `perf stat` (cycles, instructions, cache misses) once installed.

## 6. Numerical comparison metrics

Computed between candidate engine C and reference R on identical inputs:

| Metric | Level | Definition |
|---|---|---|
| Max abs diff of predictions | raster | max over valid cells of \|p_C − p_R\| on cloglog output (also raw) |
| Mean abs diff, 99th-percentile abs diff | raster | robust spread |
| Pearson r, Spearman ρ | raster | over valid cells (Spearman checks ranking, which drives thresholds) |
| Mask agreement | raster | count of cells valid in one and not the other — must be 0 |
| Thresholded agreement | raster | Cohen's κ / fraction of matching cells at standard thresholds (e.g. 10th-percentile training presence) |
| Suitable area diff | scalar | relative diff of area above threshold |
| Coefficient diff | model | per-feature \|λ_C − λ_R\| for matched features; set difference of active features |
| Loss / gain diff | model | final regularised log-loss (or training gain) difference |
| AUC / CBI diff | evaluation | on the same test folds |
| Run-to-run determinism | self | same engine, same inputs, different thread counts and repeated runs: bit-identical? if not, max abs diff |

## 7. Tolerances and regression thresholds (pre-registered, v0)

These are **initial proposals [REC]** to be reviewed and committed before the first run. They
are not derived from measurements.

| Comparison | Criterion (pass) |
|---|---|
| Rust vs Rust, same build, any thread count | **bit-identical** outputs (rasters, coefficients) |
| Rust vs Rust, different CPU/OS | max abs diff ≤ 1e-12 on raw; otherwise document cause (libm) |
| Rust vs maxent.jar 3.4.4, same features + β + background | max abs pred diff ≤ 1e-6 (cloglog); Spearman ≥ 0.99999; active-feature sets identical; Δloss ≤ 1e-8 relative |
| Rust vs maxent.jar, default auto-features | Spearman ≥ 0.999; suitable-area relative diff ≤ 0.5 %; κ ≥ 0.99 |
| Rust vs maxnet | characterise only (published differences, esp. hinge [11]); no pass/fail |
| Mask agreement (any pair) | 0 differing cells |

Performance regression thresholds [REC]: CI fails if median time of a tracked benchmark rises
> 10 % (bootstrap CI excluding ≤ 10 %) or peak RSS rises > 10 % vs the stored baseline on the
same machine; alerts (non-blocking) at > 5 %.

## 8. Reporting template

Each result file contains: manifest (§3), dataset ID + hashes, engine + version, stage ID,
thread count, cold/warm, n runs, median, IQR, min, max, peak RSS, CPU%, numerical metrics (§6)
with pass/fail against §7, and free-text anomalies. Summary tables in
`docs/benchmarks/results/README` are generated, not hand-edited.

## 9. Known pitfalls

- Page cache: second run reads from RAM; state cold/warm explicitly.
- Turbo/thermal throttling on long runs (i7-9700K, 8 cores): interleave engines (A B A B) rather
  than running all A then all B.
- Thread oversubscription (rayon + BLAS/GDAL/JVM threads): set every thread env var explicitly.
- maxent.jar `threads` option parallelises jackknife/replicates, not the core optimiser
  (`parameters.csv`) [FACT] [12] — compare single-fit timings at threads=1.
- maxent.jar uses `Math.exp`, which may differ by ≤ 1 ulp across JVMs [FACT] [13]: record JVM build.
- Rayon float reductions are not deterministic [FACT] [14]: determinism test must vary
  `RAYON_NUM_THREADS`.

## 10. Open questions

- [OPEN] Exact maxent.jar CLI flags to isolate training vs projection and to inject a fixed
  background.
- [OPEN] Are the §7 tolerances meaningful for ecologists (impact on thresholded maps)? Review with
  domain experts before freezing.
- [OPEN] Which real species/datasets have licences that allow committing derived results.

## References

[1] This document's local environment: `lscpu`, `free`, `uname`, toolchain `--version` outputs, 2026-10-15.
[2] Fick S.E., Hijmans R.J. (2017) WorldClim 2. Int. J. Climatol. 37:4302–4315. doi:10.1002/joc.5086 ; https://worldclim.org/data/worldclim21.html (accessed 2026-10-15)
[3] hyperfine README (warmup, runs, prepare, export-json, outlier detection, parameter scan). https://github.com/sharkdp/hyperfine ; crate 2.0.0 (MIT OR Apache-2.0) (accessed 2026-10-15)
[4] divan README and docs (AllocProfiler). https://github.com/nvzqz/divan ; https://docs.rs/divan/latest/divan/ (accessed 2026-10-15)
[5] crates.io API: criterion 0.8.2, divan 0.1.21, dhat 0.3.3, hyperfine 2.0.0. https://crates.io/ (accessed 2026-10-15)
[6] Maven Central search, org.openjdk.jmh:jmh-core latest 1.37. https://search.maven.org/ (accessed 2026-10-15)
[7] Barrett E. et al. (2017) Virtual machine warmup blows hot and cold. Proc. ACM Program. Lang. 1(OOPSLA). doi:10.1145/3133876
[8] Kalibera T., Jones R. (2013) Rigorous benchmarking in reasonable time. ISMM '13. doi:10.1145/2491894.2464160
[9] Hoefler T., Belli R. (2015) Scientific benchmarking of parallel computing systems. SC '15. doi:10.1145/2807591.2807644
[10] Linux kernel Control Group v2 documentation (memory.peak, memory.max, cpu.stat). https://docs.kernel.org/admin-guide/cgroup-v2.html (accessed 2026-10-15)
[11] ENMeval documentation (maxnet vs maxent.jar numeric differences). https://www.rdocumentation.org/packages/ENMeval/versions/0.3.1/topics/ENMevaluate (accessed 2026-10-15)
[12] mrmaxent/Maxent v3.4.4 `density/parameters.csv`. https://github.com/mrmaxent/Maxent (accessed 2026-10-15)
[13] Java SE 25 API, java.lang.Math. https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/lang/Math.html (accessed 2026-10-15)
[14] rayon 1.12.0 ParallelIterator docs. https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html (accessed 2026-10-15)
