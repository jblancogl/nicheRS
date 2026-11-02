# Gate G6 — 10-species batch over metropolitan France at ~1 km

Date: 2026-10-15. Machine: 8 logical CPUs, 31 GB RAM, Fedora (Linux 7.2), home broadband.
Binary: `target/dist/niche` (release, stripped). Spec: `tests/real-specs/france_batch10.json`
(CHELSA BIO1/4/5/6/12/14/15, native 30″ grid, 25-setting tuning × 4 spatial blocks, 10 000
background points, GBIF cap 20 000 records per species).

## Results

| Measure | Value |
|---|---|
| Grid | 1721 × 1167 cells; 929 475 land cells in the study area (549 618 km², ellipsoidal) |
| Environmental stack | 6.2 s cold (7 CHELSA windows over HTTP + land mask), 0.4 s from cache |
| Wall time, 10 species | 22 min 33 s (≈ 2.3 min / species) |
| Mean CPU use | 24 % of one core-equivalent per process (network-bound) |
| Peak RSS | 477 MB |
| Per-species breakdown (Lucanus cervus, measured separately) | GBIF retrieval 22–99 s (rate-limit dependent); cleaning + 100 CV fits + final fit + prediction/uncertainty/MESS over 929 k cells: **3.5 s**; outputs 0.5 s |
| Re-run with cached occurrences | retrieval 0.007 s; identical outputs (area P10 378 774 km², Boyce 0.73) |

| Species | Evidence | Boyce (CV) | OR10 (CV) | Area at P10 (km²) |
|---|---|---|---|---|
| Lacerta bilineata | exploratory | 0.17 | 0.05 | 431 716 |
| Quercus ilex | supported | 0.74 | 0.12 | 110 166 |
| Salamandra salamandra | exploratory | 0.32 | 0.09 | 428 067 |
| Erinaceus europaeus | exploratory | 0.27 | 0.06 | 408 759 |
| Rana dalmatina | indicative | 0.60 | 0.11 | 290 522 |
| Vipera aspis | exploratory | 0.23 | 0.10 | 469 518 |
| Bufo spinosus | indicative | 0.62 | 0.06 | 289 513 |
| Fagus sylvatica | indicative | 0.90 | 0.19 | 330 204 |
| Lucanus cervus | supported | 0.73 | 0.14 | 378 774 |
| Sciurus vulgaris | supported | 0.84 | 0.11 | 372 868 |

## Interpretation (ecology)

- Species with a strong climatic niche at national scale (Mediterranean *Quercus ilex*; *Fagus
  sylvatica* limited by summer drought and heat) get high Boyce indices and sensible extents.
- Generalists that occupy most of France (*Erinaceus*, *Lacerta bilineata* outside the north-east,
  *Vipera aspis*) cannot be discriminated by climate alone: Boyce 0.17–0.27 → *exploratory*. This
  is the intended behaviour — the tool does not dress up an uninformative model.
- *Fagus* shows OR10 0.19 (> 0.15): held-out blocks are partly outside the training climate; the
  evidence rule correctly stops it at *indicative*.

## Interpretation (performance)

The bottleneck is occurrence retrieval from the public GBIF search API (rate-limited, HTTP 429),
not modelling: compute is ≈ 3.5 s per species at national scale on 8 cores. Hundreds of species
are therefore feasible in hours with the search API and in minutes for re-runs (cache). For
production batches the GBIF download API (asynchronous, one DOI per batch, requires user
credentials) is the right path; it is on the roadmap. No GPU or further CPU optimization is
justified by these measurements (master prompt Phase 6).

**G6: passed** for 10 species (memory bounded at < 0.5 GB; time dominated by an external,
cacheable service). The 100-species target is extrapolated, not measured: ≈ 4 h cold, minutes warm.
