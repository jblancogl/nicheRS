# ADR-0114 — Regional GBIF mirror (France and Spain)

- Status: Accepted (2026-10-19). Product decision (session of 2026-10-19):
  "try option 1, make it work in every case and in the best possible way".
- Extends ADR-0105 §9 (data preparation off the compute worker) and ADR-0110 (GBIF accounts).
- Implementation: `crates/niche-data/src/gbif_mirror.rs` (+ generated `gbif_mirror_cells.rs`,
  `scripts/gen_mirror_cells.py`), `crates/niche-data/src/http.rs` (`download_resumable`),
  `crates/niche-app/src/engine/mirror.rs`, `crates/niche-cli` (`NICHE_GBIF_MIRROR*`,
  `niche mirror status|import`), admin console (System), setup review note.

## Context

Most of a study's wall time is GBIF's download queue, not modelling: 23 min for 10 species in
France against 1 min 31 s once the download is cached (`docs/benchmarks/gbif-batch-downloads.md`).
National portals are no faster alternative (measured 2026-10-19): GBIF.ES's biocache answers in
15.5 s per 1,000 records, holds fewer records than GBIF for the same species in Spain (2,303 vs
3,595 for *Lacerta bilineata*) and most are CC BY-NC; OpenObs (MNHN) blocks programmatic access
behind an anti-bot firewall, caps exports at 2 M records and largely republishes INPN data that
is already in GBIF. GBIF's cloud snapshots (Parquet) need no queue but filtering a species means
scanning terabytes.

## Decision

1. **One GBIF download of the region**, refreshed every `NICHE_GBIF_MIRROR_REFRESH_DAYS` (7) by
   a background thread with the shared GBIF account: the per-study predicate (coordinates, no
   geospatial issue, present) without taxon filter, over the region. Region `fr-es`: 0.5° cells
   intersecting metropolitan France (with Corsica), Spain (with the Balearic and Canary Islands,
   Ceuta and Melilla), Andorra and Monaco, buffered by ~10 km; ~327 M records (counted
   2026-10-19), sent to GBIF as a MULTIPOLYGON of counter-clockwise rectangles.
2. **Stored indexed by taxon**: records under their `speciesKey` and, when different, their
   `taxonKey` (subspecies, synonyms) — exactly how a per-study download assigns them — in 256
   sorted bucket files of 42-byte records (lossless: f64 coordinates and uncertainty) and an
   index. Records not identified to species are not kept. Building is memory-bounded (bucket by
   bucket) and the archive is deleted once read; ≈ 14 GB on disk.
3. **When a study uses it** (otherwise it downloads from GBIF exactly as before; no study can fail
   because of the mirror):
   - the snapshot is at most `NICHE_GBIF_MIRROR_MAX_AGE_DAYS` (30) old;
   - not the synthetic demo;
   - the whole study area lies in region cells or open sea (records at sea are excluded by
     cleaning, so none a model can use is missing);
   - per species: present in the index, or a species-or-lower taxon per GBIF (then zero
     records is the true answer); higher taxa keep the search API.
   The study's records are those of the mirror inside its grid box: identical, record by record,
   to a per-study download of the same archive (tested on synthetic data and on a real 59,408-record
   GBIF download, 12 taxa × 4 boxes), except records of the box outside the region, which lie
   outside the study area and are used by no model.
   The data preview and species left out of the batch read the mirror too.
4. **Robustness**: every step resumes after a restart — the pending GBIF request (`pending`, 3
   days), the archive downloaded in 256 MiB range requests (`incoming/*.partial`), a build in a
   `.building` directory renamed at the end; `current` names the snapshot in use and the previous
   one is kept for studies still reading it. Refresh errors are retried hourly; a refresh needs
   about 4× the current snapshot in free disk (60 GB the first time) and waits otherwise. While
   the mirror's download waits at GBIF it holds one of the shared account's three slots.
   `niche mirror import --zip … --key …` builds it from a download made by hand.
5. **Traceability**: each study stores `jobs/<id>/gbif-mirror.json` (snapshot, records used per
   GBIF dataset); the manifest's `occurrence_source` records the source "GBIF regional mirror", the
   region download (key, DOI, date), `gbif_account: "mirror"` and that record count per dataset.
   The citation is the region download's DOI. **Open:** registering a GBIF *derived dataset* per
   study (the citation GBIF recommends for subsets) needs a public URL of the extracted data;
   the per-dataset counts are kept so it can be done later.

## Consequences

- Studies in France and Spain start modelling within seconds (measured: a Gers study, 2.7 s in
  all with the mirror built from a real download).
- Data are up to a week old (30 days at worst), stated with the download date in every manifest
  and on the review step.
- Disk: ~14 GB per snapshot, ~40 GB at the peak of a refresh (archive + buckets); the hosted
  volume goes from 50 to 100 GiB.
- Overseas France, Portugal and other countries keep the per-study download. Adding a region is a
  new entry in `REGIONS` generated by the same script.
- If GBIF moves downloads to alphanumeric (COL XR) taxon keys, records whose keys do not fit
  are not indexed and those species fall back to the per-study download.

## Format 2: sampling-effort grids (2026-10-20)

Science v1, phase 1c (`docs/science-v1-plan.md`). Data layer; since phase 2 the grids drive the
opt-in target-group background of ADR-0013. A deployed mirror has them only after a refresh (or
`niche mirror import`) with this version; until then target-group studies fall back to the
random background with a warning.

- **What**: while building a snapshot, every record of the region archive with coordinates and
  a taxonomic class (SIMPLE_CSV `class` column) is counted per (class, 30-arc-second cell),
  *including records not identified to species* (they are sampling effort too, Phillips et al.
  2009, Ecological Applications 19:181). Grid: global, aligned to (−180°, +90°), cell = 1/120°;
  `col = ⌊(lng + 180)·120⌋`, `row = ⌊(90 − lat)·120⌋`, cell id `row·43200 + col` (u32).
  Class names are the GBIF backbone's (e.g. turtles are `Testudines`, not `Reptilia`), the same
  names `gbif::taxon_class` returns for a species.
- **Files**: `effort.bin` holds, per class, a contiguous run of 8-byte records (cell u32,
  count u32, little-endian) sorted by cell; `effort-index.json` lists each class with its first
  run, cells and records. `meta.json` gains `effort_records`. `FORMAT` is 2.
- **Memory stays bounded**: the first pass also appends (class u16, cell u32) pairs to one
  temporary file per band of 20 grid rows (1/6° of latitude; files opened on first use, ~150 for
  `fr-es`); a third pass sorts and run-length counts one band at a time and gathers each class's
  runs in band (= cell) order. No map of all cells is ever held. Peak memory ≈ 14 bytes × the
  records of the densest band; temporary disk ≈ 6 bytes per record with a class (≈ 2 GB for
  327 M records, an estimate), freed before the snapshot is renamed into place.
- **Reading**: `Snapshot::effort(class, bbox)` binary-searches the class's runs for the box's
  rows and reads only that slice (measured on 0.54 M cells: 0.13–0.3 ms for a department or the
  Paris region against 3.6–9 ms reading the whole class; `docs/architecture/data-sources-v1.md`).
- **Compatibility**: format-1 snapshots still open and serve studies exactly as before; they
  have no effort grids (`effort_classes()` is empty, `effort()` returns `None`) until the next
  weekly refresh builds a format-2 snapshot. `niche mirror import` builds them too (same
  `build`). An archive without a `class` column builds a format-2 snapshot with no classes.
- **Cost**: build time +15 % on a 2 M-record synthetic archive (1.03 s vs 0.89 s, release);
  `effort.bin` is 8 bytes per (class, cell) with records — 145 KB for a real 59,408-record
  download (18,122 cells); for `fr-es` an *estimated* 50–250 MB (see data-sources-v1.md).
