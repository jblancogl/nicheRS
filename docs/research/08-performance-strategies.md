# 08 — Performance strategies for hundreds of species

Status: research draft · Date: 2026-10-15 · Author: research agent (NicheRS)
Tags: [FACT] · [ASSUMPTION] · [OPEN] · [REC] · [UNVERIFIED – prior knowledge] · **[EST]** = back-of-envelope estimate (not a measurement)

## 1. Scope

How to make the pipeline (download → preprocessing → feature extraction → training → raster
inference → encoding → network → visualisation) scale to hundreds of species on a
workstation-class machine (reference: i7-9700K, 8 cores, no SMT, AVX2+FMA, 31 GB RAM — doc 04 §3),
without trading away correctness or reproducibility. Identifies the likely bottleneck per stage
and how it moves with resolution, territory, #species, #background and model complexity.

No measurements were taken. All numbers tagged **[EST]** are order-of-magnitude reasoning whose
assumptions are stated inline and must be replaced by benchmark results
(`docs/benchmarks/methodology.md`).

## 2. Findings

### 2.1 Size model **[EST]**

Assumptions: metropolitan France ≈ 550 000 km² of land [ASSUMPTION]; a rectangular grid over
France contains extra nodata cells (sea, neighbours), ignored here; 19 bioclimatic variables as
in WorldClim 2.1, whose finest native resolution is 30 arc-seconds (~1 km²) [FACT] [1]; float32
storage.

| Quantity | 1 km | 100 m | Scaling |
|---|---|---|---|
| Land cells N | ≈ 0.55 M | ≈ 55 M | ∝ area / res² |
| Env stack, 19 × f32 | ≈ 42 MB | ≈ 4.2 GB | ∝ N × V |
| Same in f64 | ≈ 84 MB | ≈ 8.4 GB | ×2 |
| One suitability map, f32 | ≈ 2.2 MB | ≈ 220 MB | ∝ N |
| 500 species maps, f32 raw | ≈ 1.1 GB | ≈ 110 GB | ∝ N × S |
| Materialised feature matrix for inference, 300 features × f32 | ≈ 0.66 GB | ≈ 66 GB | ∝ N × F — **do not materialise** |
| Training matrix: (10 000 bg + ≤ few 1 000 occ) × 1 000 candidate features × f64 | ≈ 0.1 GB | same | ∝ n × F, independent of N |

Note: 100 m is finer than WorldClim's native grid; 100 m stacks imply resampled climate or other
sources (land cover, DEM). [ASSUMPTION]

### 2.2 Compute model per stage **[EST]**

MaxEnt background default is 10 000 points and max 500 iterations (maxent.jar `parameters.csv`)
[FACT] [2]; each iteration evaluates `exp` over all background points [FACT] [2]. Feature counts
below are assumptions: candidates 10²–10⁴ (19 linear + 19 quadratic + 171 products + hinges/
thresholds), active after L1 regularisation 10¹–10³.

| Stage | Cost driver | 1 km, 1 species | 100 m, 1 species | 500 species | Likely bottleneck when… |
|---|---|---|---|---|---|
| Occurrence download | remote API latency, async export jobs | dominated by latency (s–min) | same | parallel requests, API rate limits | always for small S; network bound [ASSUMPTION] |
| Env raster download | bytes; one-time | ~tens–hundreds MB | GBs | **shared** across species | first run only, then cached |
| Preprocessing (reproject/crop/mask/align) | N × V | ~10⁷ cell-ops, < 1 s | ~10⁹ cell-ops, I/O-bound | **once per study area**, not per species | first run at 100 m |
| Env extraction at points | (n_occ + n_bg) × V | trivial | trivial | shared background extracted once | never |
| Feature generation (training) | n × F_cand | ~10⁷ | same | per species × feature-class set | rarely |
| Training, one fit | iters × n × F_touched; e.g. 500 × 10⁴ × 10³ = 5·10⁹ ops | seconds/core | same (independent of N) | × S | — |
| Training with tuning | fits = feature-class sets × β grid × (k folds + 1); e.g. 6 × 8 × 6 = 288 | ~10³ s/core/species | same | 500 × 288 × 5 s ≈ 7·10⁵ core-s ≈ 25 h on 8 cores | **dominant at 1 km if tuning is enabled** |
| Raster inference | N × F_active (+1 exp per cell) | 0.55 M × 300 ≈ 1.7·10⁸ → < 1 s | 55 M × 300 ≈ 1.7·10¹⁰ → ~17 s/core (~2 s on 8 cores), plus reading 4.2 GB | × S (×k if replicate maps) | **dominant at 100 m without tuning** |
| Encoding (COG, compression, overviews) | N × S bytes | ms | ~1 s-scale per map, compression-bound | 110 GB raw at 100 m | at 100 m × hundreds of species (disk + CPU) |
| Network / visualisation | tiles requested, not N | small | overviews essential | per-species layer switching | if maps are served un-tiled |

Assumed throughput for conversion to time: ~10⁹ simple feature evaluations/s/core
[ASSUMPTION — to be measured, B3/B4 in methodology].

### 2.3 How each driver moves the bottleneck **[EST]**

| Driver ↑ | Affected stages | Effect |
|---|---|---|
| Resolution (1 km→100 m, ×100 cells) | preprocessing, inference, encoding, storage | linear in N; training unaffected unless background grows |
| Territory size | same as resolution | linear in area |
| #species S | training, inference, encoding | linear; shared stages amortise |
| #background n | training (linear), extraction | inference unaffected |
| Model complexity (feature classes, products/hinges) | training (F_cand), inference (F_active) | products are V² |
| Tuning grid / replicates | training (multiplicative), inference if each replicate is mapped | the largest multiplier |

### 2.4 Strategies

| Strategy | Applies to | Assessment | Tag |
|---|---|---|---|
| **Shared env stack per study area** (one aligned, masked stack; species reference it) | preprocessing, inference | removes S× redundant work; keyed by content hash of (layers+versions, extent, CRS, resolution, mask) | [REC] |
| **Shared background matrix** (same background for all species when the sampling design allows) | extraction, feature gen | env values at 10 000 points extracted once; per-species only occurrences | [REC]; validity depends on bias design [OPEN] |
| **Precomputed feature matrix per (species, feature-class set)** reused across β grid and CV folds | training | folds = row masks over one matrix; β changes only the penalty; avoids regenerating hinges/products | [REC] |
| Never materialise full-raster feature matrix | inference | compute features on the fly per tile (§2.1: 66 GB at 100 m) | [REC] |
| Raster tiling 512×512 | preprocessing, inference, encoding | COG default `BLOCKSIZE=512` [FACT] [3]; 512² × 19 × f32 ≈ 20 MB/tile **[EST]**; 8 workers × 2 buffers ≈ 320 MB | [REC] |
| **Multi-species batching in inference** (read tile once, apply K models) | inference | memory traffic ÷K; deterministic (cells independent) | [REC] |
| Species/fit-level parallelism (one fit per worker) vs within-model | training | hundreds of independent fits saturate 8 cores; single-threaded fit keeps reduction order fixed → reproducible; within-model parallelism only for interactive single-species latency | [REC] |
| Tile-level parallelism | inference, encoding | per-cell independence → output identical regardless of thread count | [FACT] model form; [REC] |
| rayon global pool, no nested pools; any BLAS forced single-threaded | all CPU stages | avoid oversubscription; rayon float `sum` is non-deterministic [FACT] [4] → do not reduce floats through it | [REC] |
| mmap (`memmap2` 0.9.11, MIT OR Apache-2.0) of a raw tiled cache of the stack | inference | lets the OS page cache share a 4.2 GB stack across processes; COG (compressed) cannot be mmapped directly | [FACT] crate [5]; [ASSUMPTION] benefit |
| Columnar storage (Arrow/Parquet 60.0.0, Apache-2.0) | occurrences, background, extracted values, evaluation metrics, run manifests | good fit for tables; **not** for rasters (use COG/Zarr) | [FACT] crates [5]; [REC] |
| Sparse vs dense | training | threshold/categorical features are 0/1 and sparse; at n ≈ 10⁴ dense is simpler and likely sufficient | [ASSUMPTION]; measure |
| SIMD | inference inner loop (hinge = max(0,·), linear, exp) | auto-vectorisation first; `pulp`/`wide` if profiles warrant; `std::simd` unstable [6] | [REC] |
| Native BLAS vs faer | training | MaxEnt coordinate updates are not GEMM-shaped; neither may be needed; avoid OpenBLAS (C dep, own thread pool) | [ASSUMPTION]/[REC] |
| Caching / incremental compute | all | content-addressed artefacts: env stack, background, feature matrix, fitted model, map; adding a species reuses everything shared; changing β reuses feature matrix; changing extent invalidates stack and maps | [REC] |
| Model reuse | inference | projection to new extent/scenario reuses fitted model + clamping ranges | [REC] |
| Output quantisation | encoding, storage | f32 → u16/u8 reduces 110 GB **[EST]** by 2–4×; changes values → only for display products, keep f32 for analysis | [REC] |
| COG compression | encoding | ZSTD (level default 9) / DEFLATE with floating-point predictor; LERC lossless default [FACT] [3] | measure ratio/time |
| Out-of-core | 100 m and above | tile streaming makes peak memory ∝ workers × tile, not N | [REC] |
| Thread affinity | inference | single-socket 8-core, no SMT on reference machine → low expected gain; NUMA irrelevant here | [ASSUMPTION] |
| Memory allocator (mimalloc 0.1.52, tikv-jemallocator 0.7.0) | many small allocations | only if profiles show allocator contention; reuse per-worker buffers instead | [FACT] crates [5]; [REC] |
| GPU | inference | per-cell independence suits GPU, but CUDA/wgpu adds deployment complexity contrary to single binary; consider only if CPU inference measured above targets | [REC] |

### 2.5 Memory budget (31 GB machine) **[EST]**

| Item | 1 km | 100 m |
|---|---|---|
| Env stack (f32) resident | 42 MB | 4.2 GB (or mmapped/tiled: ~0.3 GB) |
| Per-worker training matrices (8 × 0.1 GB) | 0.8 GB | 0.8 GB |
| Inference tile buffers (8 × 2 × 20 MB) + K-species output tiles | < 0.5 GB | < 0.5 GB |
| OS page cache / app server / UI | remainder | remainder |
Conclusion **[EST]**: France at 100 m fits in RAM if streamed; Europe at 100 m (~10× area
[ASSUMPTION]) requires out-of-core tiling.

## 3. Evidence and references

Facts used: maxent.jar defaults and hot loop [2]; COG options [3]; rayon reduction semantics
[4]; crate versions/licences [5]; SIMD status [6]; WorldClim resolution [1]; Go/JVM runtime facts
in doc 04. Everything else is labelled [EST]/[ASSUMPTION].

## 4. Alternatives

- **Per-species pipeline (maxent.jar style)**: each run re-reads layers and writes its own
  outputs — simplest, S× redundant I/O.
- **Shared-stack batch pipeline** (recommended): one stack, one background, many fits, batched
  inference.
- **Distributed (cluster/k8s jobs per species)**: scales beyond one machine; heavier ops;
  unnecessary until single-node benchmarks show it [ASSUMPTION].
- **GPU inference**: see table; deferred.

## 5. Trade-offs

- Shared background across species saves extraction but encodes a sampling assumption; per-species
  or bias-corrected background is scientifically different → must be an explicit, recorded choice.
- Fit-level parallelism maximises throughput and reproducibility but does not reduce latency of a
  single interactive species; within-model parallelism does, at the cost of reduction-order control.
- Quantised outputs save storage but must never feed area/threshold computations.
- Caching speeds iteration but requires strict content hashing to avoid stale results.

## 6. Risks

| Risk | Mitigation |
|---|---|
| Estimates wrong by an order of magnitude | Replace with B1–B6 measurements before design freeze |
| Cache returns stale artefact after a source raster update | Hash content + version, not file names |
| Oversubscription (rayon + BLAS/GDAL threads) | Single pool; GDAL/BLAS threads = 1 inside workers |
| Non-reproducible sums under parallelism | Fixed chunking + ordered reduction; record thread count |
| Disk fills at 100 m × hundreds of species | Per-run storage estimate shown before launch; compression; quotas |

## 7. Recommendations

1. [REC] Architect around **study-area-scoped shared artefacts** (env stack, background,
   extracted values) with content-addressed caching.
2. [REC] Parallelise **across fits and tiles**, not inside the optimiser; keep each fit
   single-threaded for deterministic reductions.
3. [REC] Stream inference by 512² tiles with **multi-species batching**; never materialise
   N × F feature matrices.
4. [REC] Arrow/Parquet for tabular artefacts; COG (f32, ZSTD/DEFLATE + predictor) for maps,
   with overviews for visualisation.
5. [REC] No GPU, no native BLAS, no custom allocator in v1; reconsider only with profiles.
6. [REC] First benchmark targets: tuning-enabled training at 1 km and inference at 100 m, the two
   predicted bottlenecks.

## 8. Open questions

- [OPEN] Real per-iteration cost and active-feature counts for typical French species (drives
  both training and inference estimates).
- [OPEN] Default tuning grid the product will offer (multiplier on training time).
- [OPEN] Is a shared background scientifically acceptable for the default workflow, or must
  background be per species / bias-corrected?
- [OPEN] Required output precision for suitable-area estimates (determines whether quantised
  products are ever acceptable).
- [OPEN] Expected territories beyond France (Europe at 100 m changes the memory regime).

## References

[1] WorldClim 2.1 data page; Fick S.E., Hijmans R.J. (2017) WorldClim 2. Int. J. Climatol. 37:4302–4315. doi:10.1002/joc.5086. https://worldclim.org/data/worldclim21.html (accessed 2026-10-15)
[2] mrmaxent/Maxent v3.4.4 source, `density/parameters.csv`, `Sequential.java`, `FeaturedSpace.java`. https://github.com/mrmaxent/Maxent (accessed 2026-10-15)
[3] GDAL COG driver documentation. https://gdal.org/en/stable/drivers/raster/cog.html (accessed 2026-10-15)
[4] rayon 1.12.0 ParallelIterator docs (sum/reduce determinism). https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html (accessed 2026-10-15)
[5] crates.io API: memmap2 0.9.11, arrow 60.0.0, parquet 60.0.0, mimalloc 0.1.52, tikv-jemallocator 0.7.0, pulp 0.22.3, wide 1.7.1, faer 0.24.4, wgpu 30.0.1, cudarc 0.19.10. https://crates.io/ (accessed 2026-10-15)
[6] rust-lang/rust#86656 portable SIMD tracking issue. https://github.com/rust-lang/rust/issues/86656 (accessed 2026-10-15)
