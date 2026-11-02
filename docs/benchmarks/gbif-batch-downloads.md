# GBIF batch downloads — throughput for many species

Date: 2026-10-16. Follow-up of `g6-france-batch.md`, whose bottleneck was occurrence retrieval
through the rate-limited GBIF search API (~100 s per species, capped at 20 000 records).

## Change

One authenticated GBIF occurrence download per job (all species of species rank or below, study
area bbox, `HAS_COORDINATE`, `HAS_GEOSPATIAL_ISSUE=false`, `OCCURRENCE_STATUS=PRESENT`). GBIF
prepares the archive asynchronously and mints a DOI. NicheRS streams the SIMPLE_CSV archive into
one file per species (cached 30 days) and loads each species only when it is modelled. Cleaning
filters (licence, year, uncertainty, land, duplicates) stay local so exclusion counts are exact.

## Measurements (10 species, metropolitan France, 1 km, 25 settings × 4 spatial folds)

Laptop: 8 logical CPUs, 31 GB RAM.

| Run | GBIF data | Modelling 10 species | Wall time | Peak RSS |
|---|---|---|---|---|
| Search API (previous) | ~100 s × species, ≤ 20 000 records each (200 000 total) | ≈ 35 s | 22 min 33 s | 477 MB |
| Batch download, cold | 23 min in the GBIF queue (1 937 582 records, 162 MB zip, DOI 10.15468/dl.u4us9b) | 81 s | 24 min 33 s | 3.8 GB (first version) |
| Batch download, streamed split, cached | 0 s (archive cached) | ≈ 85 s | **1 min 31 s** | **0.87 GB** |

Per-species modelling with all records: 5–12 s (largest: *Fagus sylvatica*, 616 601 records).

## Server test ( `NICHE_MAX_WORKERS=6`, 2026-10-16)

Same 10-species spec submitted through the public API. GBIF returned the existing download for the
identical request (no queue), so this is the "data already prepared at GBIF" case:

| Stage | Time |
|---|---|
| Whole job (start → finish) | **3 min 10 s** |
| Per-species modelling (all records) | 10.8–20.0 s (mean ≈ 15 s) |
| Pod memory after the job | 617 MiB |

With 6 of the server’s 16 cores the modelling is ≈ 2× slower than on the 8-core laptop; raising
`NICHE_MAX_WORKERS` would reduce it roughly proportionally, at the expense of the other services
sharing the node.

## Interpretation

- Cold time is now dominated by GBIF's download queue (asynchronous, not under our control,
  observed 23 min), but it is paid **once per job regardless of the number of species**. For 100
  species the cold wait is expected to be of the same order (GBIF time scales mainly with record
  count), plus ≈ 15 min of modelling — versus ≈ 3 h with the search API (extrapolated, not measured).
- Warm re-runs (same species and area within 30 days) take seconds per species.
- Using every record instead of a 20 000-record subsample changes results (e.g. *Erinaceus
  europaeus* Boyce 0.27 → 0.80; *Lacerta bilineata* 0.17 → 0.70). More records also carry more
  observer-effort bias; the spatial-block validation and the evidence rules still apply, and the
  report keeps the sampling-bias warning.
- Memory is bounded by the largest species, not by the batch.

## Provenance

Each result cites the download DOI; the manifest records the download key, URL, creation date and
the GBIF citation string ("GBIF.org (date) GBIF Occurrence Download DOI").
