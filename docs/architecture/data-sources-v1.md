# Data sources — Science v1 additions (`niche-data`)

Date: 2026-10-20. Phase 1c of `docs/science-v1-plan.md`. This page records what the data layer
provides. Later phases use it through their own ADRs (ADR-0011 generalised records, ADR-0013
target-group background, ADR-0016 future climate; see "Use by later phases" below). Defaults of
ADR-0009 are unchanged.

## 1. Generalised-coordinate flags

`gbif::Occurrence::generalised: Option<bool>`:

| Source | Value | Why |
|---|---|---|
| Search API (`gbif::fetch_occurrences[_cached]`) | `Some(true)` if `dataGeneralizations` or `informationWithheld` is a non-blank string, else `Some(false)` | Both Darwin Core fields are in the JSON response. The text is free and multilingual: only its presence is used (the design notes §2). |
| Batch download (`gbif_download`, SIMPLE_CSV) | `None` (unknown) | SIMPLE_CSV has no such columns (checked on a real archive: 50 columns, `class`/`kingdom` present, neither generalisation field). |
| Regional mirror (`gbif_mirror`) | `None` (unknown) | Built from a SIMPLE_CSV download. |

- `gbif::generalisation_flags_available(RecordSource)` says which sources report the flag
  (`SearchApi` only), for the `generalisation_flags_unavailable` warning planned in
  donnees-sensibles-rgpd.md §6.1.
- Search-API cache: entries are now `<taxon>_<bbox>_<cap>.v2.json`
  (`gbif::SEARCH_CACHE_VERSION = 2`). Entries written before (no version in the name, no field)
  are never read again, so a record is never treated as "not generalised" because its cache entry
  predates the flag; they are refetched. Old files stay on disk until the cache is cleared.
- `None` is not serialised (`skip_serializing_if`), so JSON written by the download and mirror
  paths is unchanged. Split files (`<taxon>.tsv`) keep their 8-column format: they only ever
  hold SIMPLE_CSV-derived records.
- Live check (2026-10-20): 310 search-API records of *Testudo hermanni* in Provence
  (5.5–7.0 E, 42.9–43.8 N), 205 flagged generalised, none unknown.

## 2. Sampling-effort grids (regional mirror, format 2)

Definition, file layout and compatibility: ADR-0114, section "Format 2". In short: for every
record of the region archive with coordinates and a class, identified to species or not, the
number of records per (class, 30-arc-second cell) of a global grid aligned to (−180°, +90°).
`Snapshot::effort_classes()` lists classes; `Snapshot::effort(class, (w, s, e, n))` returns
`(lng, lat of the cell centre, count)` for cells with records whose centre lies in the box
(inclusive), or `None` if the snapshot has no grid for that class (format 1, or no record).
Cells without records are absent (count 0). Counts are records, not visits or observers: a
dataset that posts many records at one point weighs as much as many observers.

**Class names** are those of the GBIF backbone as published in the download. They follow the
current backbone, e.g. *Emys orbicularis* is in class `Testudines` (not `Reptilia`), both in the
download and in `gbif::taxon_class` (checked 2026-10-20), so a species' class finds its grid.

### Measurements

| What | Data | Result |
|---|---|---|
| Exact counts | synthetic archive, 9 rows (`effort_grids_count_records_by_class_and_cell`) | counts per cell and class equal the hand-computed values; genus-level and class-level records counted; records without class or coordinates not counted |
| Real archive | GBIF download 0027314-260928105237408 (*Emys orbicularis*, 59,408 records) | 1 class, 18,122 cells, `effort.bin` 144,976 bytes; mirror records still identical to the per-study download in 12 taxa × 4 boxes |
| Build cost | synthetic 2 M records over France, release build | 1.03 s with effort vs 0.89 s without a `class` column (+15 %) |
| Size | same | 842,969 (class, cell) pairs → `effort.bin` 6,743,752 bytes (8 bytes per pair) + 136-byte index |
| Query, binary search vs whole-class scan | same, class of 542,182 cells | 1°×1° box: 0.30 ms vs 3.7 ms; Paris region 0.5°×0.3°: 0.13 ms vs 3.6 ms; France: 2.9 ms vs 9.5 ms |

The query reads two binary searches' worth of 4-byte values (≈ 2 × 20 reads) and then only the
runs of the box's rows; a box narrow in longitude still reads its rows' full width of the
region (bounded, a few MB at most).

### Estimates for the `fr-es` region (not measured: no real region archive at hand)

- **Disk**: `effort.bin` = 8 bytes × (class, cell) pairs with records. The region has 649 cells
  of 0.5° = 2.34 M cells of 30″; the real *Emys* download gives 3.3 records per occupied cell
  for one class. Assuming the ~10 dominant classes (Aves, Magnoliopsida, Insecta, Liliopsida,
  Mammalia, …) occupy between a quarter and all of the cells and the long tail of classes adds
  a few million pairs, **estimate 50–250 MB** (hard upper bound: 8 bytes × 327 M records =
  2.6 GB). Small next to the ~14 GB snapshot.
- **Temporary disk during the build**: 6 bytes per record with a class, ≈ 2 GB for 327 M records
  (estimate), freed during the build.
- **Memory**: the third pass holds one band (20 rows = 1/6° of latitude across the region) at a
  time, ≈ 14 bytes per record of the band at the peak. The region spans ~150 bands, so an
  average band is ≈ 2.2 M records (≈ 30 MB); the densest band (Paris, ~48.8° N) could be 5–10×
  that, **estimate ≤ 300 MB**. The process maximum resident size of the 2 M-record measurement
  test was 316 MB, dominated by the test's in-memory archive, so it is not a measurement of
  the build.

## 3. Taxon class lookup

`gbif::taxon_class(http, key) -> Result<Option<String>, DataError>` reads `class` from the GBIF
species API (`/v1/species/{key}`). `None` for taxa above class or without a class.
`gbif::taxon_info(http, key)` returns rank, scientific name, kingdom and class from one request;
`taxon_rank`, `taxon_class` and `species_name` all go through it, and answers are memoised in the
process for 24 h (at most 50,000 taxa; errors are not memoised). Before, `taxon_rank` was not
cached at all (one request per call); now the rank and class of a species cost one request.
Live check: *Columba palumbus* → `Aves`, *Quercus robur* → `Magnoliopsida`, Animalia → none.

## 4. CHELSA v2.1 CMIP6 future climatologies

Catalogue (`chelsa::FUTURE_PERIODS`, `FUTURE_GCMS`, `FUTURE_SSPS`): periods 2011–2040,
2041–2070, 2071–2100; SSPs ssp126, ssp370, ssp585; the five ISIMIP3b models GFDL-ESM4,
IPSL-CM6A-LR, MPI-ESM1-2-HR, MRI-ESM2-0, UKESM1-0-LL; BIO1–BIO19.

```
https://os.zhdk.cloud.switch.ch/chelsav2/GLOBAL/climatologies/{period}/{GCM}/{ssp}/bio/CHELSA_bio{n}_{period}_{gcm lower case}_{ssp}_V.2.1.tif
```

`chelsa::future_url(id, period, gcm, ssp)` (None outside the catalogue) and
`chelsa::read_window_future(http, cache, id, period, gcm, ssp, window)`, which reuses the
1981–2010 window reader, native grid and on-disk cache (under
`chelsa-future/<period>/<GCM>/<ssp>/<id>/`), and refuses a file whose grid differs from the
baseline grid.

### Verification (2026-10-20)

- **URLs**: HEAD requests on all 855 combinations (19 variables × 3 periods × 5 models × 3
  SSPs): 855 × HTTP 200. Directory names use the CMIP6 casing, file names the lower case.
- **Grid, scale, offset**: GeoTIFF headers read with the existing reader for BIO1–BIO19 of
  2041–2070 MPI-ESM1-2-HR ssp370 and for BIO1 of all 45 period/model/SSP combinations (64
  files, live test `chelsa_future_layers_share_the_baseline_grid`): width, height and grid
  (origin, cell size) identical to the 1981–2010 files; scale 0.1 and offset (−273.15 for
  temperatures, 0 otherwise) identical.
- **No-data differs** (found by that check): future files declare no-data 0 for BIO1, 2, 4–7
  and 12–14, where the 1981–2010 files declare −99999 (a value their integer data cannot hold,
  so no baseline cell is masked). A raw 0 is real data: BIO14 (precipitation of the driest
  month) is 0 mm in the Atacama in both files. Applying the declared 0 would turn the driest
  cells into missing values in future layers only. `read_window_future` therefore ignores a
  declared no-data of 0 (for temperatures a raw 0 would be 0 K and never occurs); other
  no-data values (65535, f32::MAX) are the same in both and kept. Live test
  `chelsa_future_zero_values`: Atacama BIO14 is 0 in both periods through the public readers;
  sea cells hold values in both (e.g. Bay of Biscay BIO1 14.35 → 15.25 °C).
- **Sanity**: BIO1 mean over a Brittany window, 11.86 °C (1981–2010) and 14.36 °C (2071–2100,
  GFDL-ESM4, ssp585). A unit check, not a result.

### Licence and citation

- **CHELSA**: the CHELSA climatologies dataset (EnviDat, doi:10.16904/envidat.228, resource
  "CHELSA V2.1 (current)", described as "based on ERA5 forcings and CMIP6") and the CHELSA site's
  climatologies page both state **Creative Commons Zero (CC0 1.0)**; no separate licence is
  shown for the future climatologies, so the dataset-level CC0 is taken to cover them
  (checked 2026-10-20: https://envidat.ch/dataset/chelsa-climatologies,
  https://www.chelsa-climate.org/datasets/chelsa_climatologies). The opendata.swiss mirror of
  the record shows the terms "Commercial Allowed / Reference Not Required", consistent with CC0.
- **CMIP6 inputs**: the CMIP6 terms of use (https://pcmdi.llnl.gov/CMIP6/TermsOfUse/TermsOfUse6-2.html)
  put model output under **CC BY 4.0** (since October 2022) and ask publications to cite the
  model output, acknowledge WCRP and list the models. They do not address derived products
  explicitly. NicheRS therefore treats commercial use as allowed and **always attributes**:
  `chelsa::FUTURE_LICENCE` and `chelsa::FUTURE_CITATION` (Karger et al. 2017 and 2021; Eyring
  et al. 2016 for CMIP6; Lange 2019 for the ISIMIP3b bias adjustment; WCRP acknowledgement and the
  five models) are to go in every report and manifest that uses a future layer (phase 5).
- **Open point**: confirm with CHELSA (Dirk Karger, WSL) that CC0 applies to the CMIP6-based
  future files as stated at dataset level. Not blocking: attribution is given either way.

## Use by later phases

- Phase 2 (2026-10-20): the generalisation flag drives the cleaning rule of ADR-0011 (search-API
  records) and the `generalisation_flags_unavailable` warning (downloads, mirror); the effort
  grids and `taxon_class` drive the opt-in target-group background, effort layer and bias
  diagnostic of ADR-0013.
- Phase 5 (2026-10-20): the CMIP6 layers drive the opt-in future-climate projections of
  ADR-0016 (`niche_app::projection`). `Http::range_bytes_received()` now counts the bytes of
  range requests, used to measure their transfer cost: a 1° window (121 native rows) of one
  CMIP6 layer is about 3.2 MB.

## Not done in this phase

- Generalisation flags for downloads: needs the DWCA or SQL download formats
  (donnees-sensibles-rgpd.md §A2), not in scope.
- No use of any of this in cleaning, background selection, the API or the UI (phases 2, 5; phase 2 is now done, see above).
- The `fr-es` effort sizes above are estimates; they are measured at the first format-2 refresh
  (`meta.json`: `effort_records`, `bytes`).
