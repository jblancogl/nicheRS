# 05 — Geospatial engineering for NicheRS

Status: research, 2026-10-15. Tags: [FACT] verified from a source read on 2026-10-15; [ASSUMPTION];
[OPEN]; [REC]; [UNVERIFIED – prior knowledge]. Crate data from the crates.io API (version, release
date of that version, licence, total downloads) on 2026-10-15 [05-1].

## 1. Scope

All geospatial I/O and processing NicheRS needs for SDM work (France/Europe, hundreds of species):

- Formats: GeoTIFF/COG (read + write), GeoJSON, GeoPackage (vector, maybe raster), PMTiles.
- CRS handling and coordinate transforms; raster alignment, resampling, clipping, masking, NoData,
  compression, block/tiled processing.
- Vector→raster (study-area masks, rasterised occurrences), raster→vector (binary suitable-habitat
  polygons), area estimation (equal-area vs geodesic), spatial indexing.
- Map delivery: COG rendering, dynamic XYZ tiling (frontend side in doc 10).
- The build-time question: can a genuinely self-contained executable avoid GDAL/PROJ (doc 13 handles
  packaging; this document handles capabilities and scientific risk).

Out of scope: data sources (06), validation statistics (07), UI (09/10).

## 2. Findings

### 2.1 What NicheRS actually needs (requirement-level)

| Need | Typical inputs | Minimum capability | Notes |
|---|---|---|---|
| Read env. layers | WorldClim/CHELSA (EPSG:4326, Float32/Int16, often DEFLATE/LZW, tiled or striped), Copernicus/CORINE land cover (EPSG:3035, categorical), BD ALTI/IGN (EPSG:2154) | GeoTIFF: strips+tiles, DEFLATE/LZW/ZSTD, horizontal + floating-point predictor, BigTIFF, GDAL_NODATA tag, GeoKeys | User-supplied files are uncontrolled → robust decoder matters most |
| Align stack | N layers, different CRS/res/extent | warp to one grid (CRS, origin, res, extent), per-layer resampling method | Scientific core; see §2.6 |
| Write outputs | suitability (Float32 0–1), binary maps (UInt8), uncertainty | tiled GeoTIFF/COG with overviews, DEFLATE/ZSTD + predictor, NoData | Must open cleanly in QGIS/ArcGIS |
| Occurrences | GBIF/CSV/GeoJSON points, EPSG:4326 | point → CRS transform, point-in-raster lookup | trivially parallel |
| Study area | GeoJSON/GeoPackage polygons, drawn polygons (4326) | reproject + rasterise to model grid (all-touched vs centre rule documented) | |
| Area estimates | binary raster | equal-area pixel counting or per-pixel geodesic area | see §2.7 |
| Export polygons | binary raster | polygonise (pixel-exact, 4/8-connectivity) | GeoPackage/GeoJSON |
| Display | outputs | XYZ PNG/WebP tiles (server) or COG in browser | doc 10 |

### 2.2 Rust geospatial ecosystem (verified 2026-10-15)

| Crate | Version (release date) | Licence | Downloads | What it really does | Verdict |
|---|---|---|---|---|---|
| `gdal` (georust) | 0.19.0 (2025-12-23) | MIT | 4.28 M | Safe bindings to GDAL; supports GDAL ≥3.8, prebuilt bindings 3.8–3.13, `bindgen` feature for others [05-2] [FACT] | Most capable option; C/C++ dependency |
| `gdal-sys` | 0.12.0 (2025-12-23) | MIT | 4.29 M | FFI; `bundled` feature pulls `gdal-src` [05-3] [FACT] | |
| `gdal-src` | 0.3.0+3.12.1 (2025-12-23) | GDAL licence file ("non-standard" on crates.io) | 85 k | CMake-builds **GDAL 3.12.1** statically: `BUILD_SHARED_LIBS=OFF`, `GDAL_USE_INTERNAL_LIBS=ON`, `GDAL_USE_EXTERNAL_LIBS=OFF`, optional drivers OFF and opt-in per feature (`driver_gtiff`, `driver_gpkg`, …), curl OFF unless `curl-sys`, GEOS optional (LGPL warning in Cargo.toml) [05-3] [FACT]. Upstream GDAL is 3.13.3 (2026-08-18) [05-4] → bundled version lags | Viable static route; heavy C++ build |
| `proj` (georust) | 0.31.0 (2025-08-29) | MIT/Apache-2.0 | 0.86 M | Bindings to PROJ; `geo` integrates via `Transform` (feature `proj`) [05-5] [FACT] | Reference-quality transforms |
| `proj-sys` | 0.27.0 (2025-08-29) | MIT/Apache-2.0 | 0.96 M | Needs PROJ ≥ 9.6.2; `bundled_proj` builds PROJ 9.6.2 from a vendored tarball with `BUILD_SHARED_LIBS=OFF`, `ENABLE_CURL=OFF`, TIFF optional; SQLite from system or from `libsqlite3-sys` bundled [05-6] [FACT]. Upstream PROJ is 9.9.0 (2026-09-15) [05-4] | |
| `proj4rs` (3liz) | 0.2.1 (2026-10-09) | MIT/Apache-2.0 | 1.20 M | Pure-Rust port of proj4js. Source tree has: aea, aeqd, cea, eqc, estmerc, etmerc, geocent, geos, krovak, **laea**, latlong, **lcc**, **merc**, mill, moll, somerc, stere, sterea, tmerc [05-7] [FACT]. `+towgs84`, NTv2 / GeoTIFF nadgrids; no WKT (use `proj4wkt`), no 3D/4D, "not a replacement of PROJ"; EPSG lookup only via optional `crs-definitions` [05-7] [FACT] | Covers EPSG:4326, 3857, 3035 (LAEA), 2154 (LCC) — see §2.4 |
| `crs-definitions` | 0.6.0 (2026-09-16) | CC0-1.0 | 0.26 M | Static EPSG → proj4/WKT strings | Use with proj4rs |
| `wbprojection` (Whitebox) | 0.3.3 (2026-08-04) | MIT/Apache-2.0 | 17 k | Pure-Rust projection library "inspired by PROJ", EPSG registry, NTv2 grid shifts [05-8] [FACT]; first release 2026-03-31 | Too young; watch |
| `geo` | 0.33.1 (2026-04-20) | MIT/Apache-2.0 | 24 M | `Area` (planar), `GeodesicArea` (Karney 2013, ellipsoid), `ChamberlainDuquetteArea` (sphere), `BooleanOps`, `Simplify`, `Contains`, `Validation`, `Transform` (proj feature); **no raster algorithms** [05-9] [FACT] | Use |
| `geo-types` | 0.7.20 (2026-08-02) | MIT/Apache-2.0 | 29 M | Shared geometry types | Use |
| `geojson` | 1.0.0 (2026-03-16) | MIT/Apache-2.0 | 11.8 M | GeoJSON (de)serialisation | Use |
| `geozero` | 0.15.1 (2025-12-11) | MIT/Apache-2.0 | 1.8 M | Zero-copy streaming conversion between formats (GeoJSON, WKB, GPKG via sqlx, MVT, …) [UNVERIFIED – prior knowledge for format list] | Optional |
| `geographiclib-rs` | 0.2.7 (2026-02-17) | MIT | 21 M | Port of a subset of GeographicLib: `Geodesic`, `PolygonArea` (accurate summation on by default) [05-10] [FACT] | Use for geodesic areas |
| `rstar` | 0.13.0 (2026-05-24) | MIT/Apache-2.0 | 49 M | R*-tree | Use for vector indexing |
| `tiff` (image-rs) | 0.11.3 (2026-02-10) | MIT | 134 M | See §2.3 | Usable reader, weak writer |
| `geotiff` (georust) | 0.1.0 (2025-06-10) | MIT | 25 k | "simply read GeoTIFFs", pixel lookup only, "works for very basic GeoTIFFs", breaking changes expected [05-11] [FACT] | Not sufficient |
| `georaster` (pka) | 0.2.0 (2025-01-11) | MIT/Apache-2.0 | 8 k | GeoTIFF/COG reader [05-12] [FACT]; little documentation | Not sufficient |
| `async-tiff` (Development Seed) | 0.4.0 (2026-09-18) | MIT/Apache-2.0 | 46 k | **Read-only**, async, tiled TIFF; Deflate, LERC, LZMA, LZW, JPEG, JPEG2000, WebP, ZSTD; `object_store` backends; GeoTIFF tag metadata [05-13] [FACT] | Good COG reader; no writing, no strips |
| `geotiff-reader` / `geotiff-writer` / `tiff-writer` (roteiro-gis) | 0.8.1 (2026-08-13), first 2026-03-20 | MIT/Apache-2.0 | 10–17 k | Pure-Rust: writer supports Deflate/LZW/JPEG/LERC/ZSTD, horizontal **and floating-point predictor**, BigTIFF, tiles, streaming tile writes, **COG with GDAL ghost-area metadata, overviews (nearest/average)**, NoData, GeoTIFF 1.1 keys; parity tests against GDAL when installed [05-14] [FACT, self-reported, 15 stars] | Promising; immature — candidate for the pure-Rust writer after a spike |
| `oxigdal` → `oxigeo` (cool-japan) | 0.1.7 (2026-07-20, final under that name); `oxigeo-core` 0.2.4 | Apache-2.0 | 1 k | Claims "production-ready" pure-Rust GDAL replacement, 16 drivers, 211 embedded EPSG, 18 k tests; repo shows 13 commits on master [05-15] [FACT that these are claims; not verified] | Do not depend on it |
| `geo-rasterize` | 0.1.2 (2022-01-03) | MIT/Apache-2.0 | 185 k | Pure-Rust rasteriser of geo-types | Unmaintained ~4 years; small enough to vendor or rewrite |
| `contour` | 0.13.1 (2024-04-30) | MIT/Apache-2.0 | 142 k | Marching-squares isolines/isobands [UNVERIFIED – prior knowledge] | Not pixel-exact polygonisation |
| `pmtiles` (Stadia) | 0.25.0 (2026-10-15) | MIT/Apache-2.0 | 344 k | Async read (mmap, http, S3, object_store); `write` feature with `PmTilesWriter` [05-16] [FACT] | Use if PMTiles needed |
| `rusqlite` | 0.40.2 (2026-08-08) | MIT | 126 M | SQLite; `bundled` feature | Base for GeoPackage |
| `geopackage` (urschrei) | 0.11.2 (2026-10-15), first 2026-07-24 | MIT/Apache-2.0 | 1.2 k | Read/write features, tile pyramids (bytes, not decoded), spatial index, validation, Arrow batches; built on rusqlite 0.40; `bundled` SQLite feature [05-17] [FACT] | Promising; 3 months old |
| `gpkg` | 0.1.0 (2022-06-04) | MIT/Apache-2.0 | 3 k | — | Abandoned |

Ecosystem observation [FACT from release dates]: several pure-Rust "GDAL replacement" crates
(`oxigdal/oxigeo`, `wbraster/wbvector/wbprojection`, `geotiff-writer`, `geopackage`) all first appeared in
2026 and have ≤20 k downloads. They widen the pure-Rust option but none has the field exposure of
GDAL (decades, used by QGIS/ArcGIS-adjacent toolchains). [ASSUMPTION] Code volume per commit in some
suggests heavy code generation; treat their self-reported test counts as unverified.

### 2.3 Can the `tiff` crate read/write real-world COGs? (source-level check, v0.11.3 tag)

| Capability | Decode | Encode | Evidence |
|---|---|---|---|
| Strips / tiles | both (`get_chunk_type`, `tile_count`, `read_chunk`, `chunk_dimensions`) | **strips only** (no tile API in `encoder/mod.rs` at v0.11.3) | [05-18] docs.rs Decoder; source grep [FACT] |
| BigTIFF | yes | yes (`TiffEncoder::new_big`) | README, encoder docs [FACT] |
| None/LZW/Deflate/PackBits | yes | yes | README table [FACT] |
| ZSTD | yes (feature `zstd`, since 0.10.0) | no | README, CHANGES [FACT] |
| JPEG / Fax4 | yes | no | README [FACT] |
| WebP | yes (feature `webp`, 0.11.3) | no | CHANGES [FACT] |
| Horizontal predictor | yes | yes (since 0.10.0) | CHANGES [FACT] |
| Floating-point predictor (3) | **yes for IEEEFP** (decoder `mod.rs` l.701; `image.rs` accepts `(FloatingPoint, IEEEFP)`) | **no** in 0.11.3 (`encoder/mod.rs`: "FloatingPoint is currently not supported"); supported on `main`, unreleased | source at tag v0.11.3 and main [FACT]. Note: docs.rs page for `tags::Predictor` still says "Not currently supported" — stale for the decoder |
| Float32/64, UInt8–64 gray | yes | yes | README [FACT] |
| Signed integer (Int16 DEMs) | [UNVERIFIED – README table lists only UINT and IEEEFP columns] | [UNVERIFIED] | must be tested — Int16 is common for DEMs/WorldClim-derived data |
| Overviews (sub-IFDs / multi-page) | multipage yes | write multipage yes, COG layout no | README [FACT]; COG layout [ASSUMPTION] |
| GeoKeys, ModelTiepoint/PixelScale/Transformation, GDAL_NODATA (42113), GDAL_METADATA (42112) | raw tag access only | raw tags | must be implemented on top [ASSUMPTION] |

Conclusion [FACT-derived]: `tiff` can read most tiled/compressed/BigTIFF/predictor COGs except
codecs like LERC/LZMA/JPEG2000; it cannot *write* a COG (no tiles, no float predictor, no ZSTD). The
`geotiff` crate adds only basic GeoKey reading. A pure-Rust COG writer requires `geotiff-writer`
(young) or ~1–2 k lines of in-house code on top of `tiff` primitives [ASSUMPTION – estimate].

### 2.4 CRS handling: PROJ vs proj4rs

| Aspect | PROJ (via `proj`) | `proj4rs` |
|---|---|---|
| EPSG database | `proj.db` (SQLite) with full EPSG registry + transformation pipelines | none; caller passes proj4 strings (or `crs-definitions`) |
| Datum transformation selection | Chooses best operation (area of use, accuracy), grids via PROJ-data/CDN | Only what is written in `+towgs84` / `+nadgrids` — user responsibility |
| WKT/PROJJSON input from GeoTIFF/GPKG | Yes | No (needs `proj4wkt`) |
| Static embedding | PROJ ≥9.6 `EMBED_RESOURCE_FILES` defaults ON for static builds and embeds proj.db/proj.ini; `USE_ONLY_EMBEDDED_RESOURCE_FILES` disables FS lookup [05-19] [FACT]. proj-sys bundled builds static PROJ 9.6.2 → proj.db embedded by default [inference from [05-6]+[05-19]] | Nothing to embed |
| Projections needed by NicheRS | all | 4326 (latlong), 3857 (merc), 3035 (laea), 2154 (lcc) present in source tree [05-7] [FACT]; accuracy vs PROJ not documented → **[OPEN] must be tested** |

For EPSG:2154 (RGF93 v1 / Lambert-93) and EPSG:3035 (ETRS89-LAEA), the datum (RGF93/ETRS89) is
treated as coincident with WGS84 at the metre level in proj4 strings (`+towgs84=0,0,0,...`)
[UNVERIFIED – prior knowledge]. At SDM resolutions (≥ 30 m, typically 100 m–1 km) a sub-metre
ETRS89↔WGS84 difference is irrelevant [ASSUMPTION – reasonable given ~0.5–1 m drift over decades].
The risk with proj4rs is not the projection maths but **identifying the CRS** of arbitrary input
files (WKT parsing, non-EPSG CRS, local grids, NTF-based legacy French data needing grid shifts).

### 2.5 Raster operations needed and where they come from

| Operation | GDAL route | Pure-Rust route | Effort pure Rust |
|---|---|---|---|
| Warp/reproject with nearest/bilinear/cubic/average/mode/sum | `gdalwarp` API (`GDALWarp`, `-r near/bilinear/average/mode/sum/rms`, NoData-aware average) [05-20] [FACT] | implement inverse-mapping warper with proj4rs + chunked kernels | Medium–high; mode/average with partial-pixel weights is the tricky part |
| Aligned grid snapping | `-tap`, `-te`, `-tr` [05-20] [FACT] | own grid model (`origin, res, shape, crs`) | Low |
| Clip/mask by polygon | `gdalwarp -cutline`, `GDALRasterize` | rasterise mask + multiply | Low–medium |
| Rasterisation | `GDALRasterizeLayers` (all-touched option) | `geo-rasterize` (stale) or scanline fill | Low–medium |
| Polygonisation | `GDALPolygonize` (4/8 connectivity) | own pixel-edge tracer (`contour` interpolates → not pixel-exact) | Medium |
| COG write | COG driver: COMPRESS incl. ZSTD/LERC/DEFLATE, PREDICTOR YES/FLOATING_POINT, BLOCKSIZE 512 default, `BIGTIFF=YES` must be explicit for >4 GB compressed [05-21] [FACT] | `geotiff-writer` or own | Medium |
| Overviews | COG driver; **default overview resampling is CUBIC for non-paletted data** [05-21] [FACT] | own (nearest/average/mode) | Low |
| Block processing | `RasterBand::read_block`/windowed `read_as` | `tiff::read_chunk` / `async-tiff` | Low |
| Zonal stats, area | manual | manual (`ndarray` + Rayon) | Low |

### 2.6 Scientific risks of geospatial errors (why this matters more than packaging)

| Error | Mechanism | Effect on SDM | Mitigation [REC] |
|---|---|---|---|
| Wrong/missing CRS | Layer assumed 4326 but is 2154; or axis order lat/lon swapped (EPSG:4326 authority order is lat,lon) [UNVERIFIED – prior knowledge] | Occurrences sampled from wrong cells → silently wrong environmental values, model still "trains" | Refuse layers without CRS; normalise axis order to x=lon,y=lat; bounding-box sanity check of occurrences vs layer extent; store CRS (WKT2/PROJJSON + EPSG) in run manifest |
| Wrong datum transformation | proj4 string without `+towgs84` for non-WGS84 datum (e.g. NTF, ED50) | 100–200 m shifts [UNVERIFIED – prior knowledge] | Restrict accepted CRSs or use PROJ; flag legacy datums |
| Mismatched resolution/extent/origin | Stacks compared cell-by-cell with half-pixel offsets | Mis-registration of predictors; biased response curves at edges | Single "model grid" object; every layer warped to it; assert equality of transform before stacking |
| Categorical data interpolated | Bilinear/cubic on land-cover classes creates non-existent classes (e.g. class 23.4) | Garbage factor levels / spurious "intermediate" classes | Layer metadata `kind: categorical` → nearest (same/finer res) or **mode** (coarsening). Note GDAL COG overview default is CUBIC [05-21] — must override |
| Continuous data with nearest when coarsening | Aliasing; one fine pixel represents a 1 km cell | Noise, loss of representativeness | Coarsening → **average** (NoData-aware); refining → bilinear |
| Coarsening counts/areas with average | e.g. population, % cover | Wrong magnitude | `sum` or fraction-cover aggregation |
| NoData mishandled | NoData value (-3.4e38, -9999) treated as data; NaN propagation | Extreme covariates, broken clamping, background points in sea | Read GDAL_NODATA tag; convert to explicit mask; background sampling only in valid mask |
| Area computed in degrees | Counting 4326 pixels × constant | Pixel area varies with cos(lat): ~18% larger at 42°N than at 51°N within mainland France [arithmetic: cos 42°/cos 51° ≈ 0.743/0.629 ≈ 1.18] | Equal-area grid (3035) or per-row geodesic pixel area |
| Reprojection of output suitability with cubic | Overshoot outside [0,1] | Invalid probabilities | Bilinear or nearest for display; clamp; keep native grid for statistics |
| Model grid in Web Mercator | 3857 is not equal-area; area scale ∝ 1/cos²(lat) | Area and density errors | Never model in 3857; 3857 only for display tiles |

### 2.7 Area estimation

- Planar area on an equal-area projection (EPSG:3035 for Europe; Lambert-93 2154 is conformal, not
  equal-area — area distortion is small over France but non-zero) [UNVERIFIED – prior knowledge on
  2154 distortion magnitude].
- Geodesic: `geo::GeodesicArea` or `geographiclib_rs::PolygonArea` (Karney 2013) on the ellipsoid
  [05-9][05-10] [FACT]. For a lat/lon raster, area of a cell depends only on its row; computing
  one geodesic area per row (n_rows calls) and multiplying by counts is exact and cheap [REC].
- [REC] Report suitable area in km² with the method stated (equal-area grid vs geodesic) and the
  threshold used; both methods should agree within numerical tolerance in tests.

### 2.8 Tiled processing and rendering

- Block processing: read by native blocks (COG 256/512 tiles), process with Rayon per block,
  write tiled output; never materialise full Europe-scale stacks unless they fit the memory budget
  (`NICHE_MAX_MEMORY_MB`). Grid size check: Europe at 100 m in 3035 ≈ 60 000 × 50 000 cells =
  3·10⁹ cells ≈ 12 GB per Float32 layer [arithmetic from assumed 6000 × 5000 km extent;
  ASSUMPTION on extent].
- Dynamic tiles: server renders XYZ tiles in 3857 from model-grid COGs (warp tile footprint with
  bilinear/nearest per layer kind, apply colour ramp, encode PNG/WebP), cached on disk. Alternative
  client-side COG decoding is evaluated in doc 10.
- PMTiles is useful for static vector overlays (study areas, admin boundaries) and for exporting
  pre-rendered raster pyramids for offline sharing; not required for MVP [REC].

### 2.9 Is a self-contained executable without GDAL/PROJ practical?

Four architectures:

| | A. Pure Rust | B. Static GDAL+PROJ (gdal-src, proj-sys bundled) | C. Dynamic GDAL/PROJ | D. Hybrid restricted |
|---|---|---|---|---|
| Formats | whatever we implement | all enabled drivers (GTiff, COG, GPKG, GeoJSON, …) | all installed | GeoTIFF/COG + GeoJSON (+GPKG via rusqlite) |
| CRS | proj4rs (subset, no WKT, no op selection) | full PROJ 9.6.2, proj.db embedded | full PROJ (system version) | fixed allow-list (4326, 3857, 3035, 2154, UTM 30–32N) via proj4rs; PROJ used in tests as oracle |
| Build | `cargo build`; trivially cross-compiles incl. musl/wasm | C/C++ toolchain + CMake; per-target cross builds; build time large [ASSUMPTION; to measure] | easy build, hard deploy (DLL hell on Windows, version drift on Linux) | `cargo build` |
| Single binary | yes | yes (statically linked; licences permissive unless GEOS) | **no** | yes |
| Reproducibility | pinned in Cargo.lock | pinned (gdal-src 3.12.1/PROJ 9.6.2; lag upstream) | depends on host GDAL/PROJ version | pinned |
| Correctness risk | highest: we own warping, resampling, COG writer, CRS parsing | lowest: battle-tested | low, but version-dependent results | medium, bounded by allow-list + oracle tests |
| User-input robustness | weak (odd GeoTIFF variants, LERC, JPEG2000, old datums) | strong | strong | weak but **fails loudly** with a clear "convert with QGIS/gdalwarp" message |

Assessment:
- A full pure-Rust GDAL replacement is not practical for NicheRS's correctness priority in 2026:
  the candidate crates are months old and self-reported [05-14][05-15].
- B is feasible on paper (static libs, embedded proj.db) [05-3][05-6][05-19], but the cost is a
  multi-minute C++ build per target, cross-compiling GDAL for Windows/macOS/ARM from Linux CI, and
  tracking a lagging bundled version. [OPEN] binary size and build time must be measured in a spike;
  no published numbers found.
- D matches the product: SDM inputs come from a curated catalogue (doc 06) whose layers NicheRS
  itself downloads and normalises; user uploads can be restricted to GeoTIFF/COG and GeoJSON/GPKG
  with a CRS from an allow-list.

## 3. Evidence and references

See References; every crate fact comes from crates.io API, docs.rs or the repository source at the
stated tag, accessed 2026-10-15.

## 4. Alternatives

1. Pure Rust (A) — `tiff`/`async-tiff` + `geotiff-writer` + `proj4rs` + `geo` + own warper.
2. Static GDAL/PROJ (B) — `gdal` + `gdal-sys/bundled` + `proj-sys/bundled_proj` + `libsqlite3-sys/bundled`.
3. Dynamic GDAL/PROJ (C) — system/OSGeo4W/Homebrew/conda GDAL.
4. Hybrid (D) — pure-Rust core with restricted formats and CRS allow-list, plus an **optional**
   `gdal` cargo feature (static, B) used for (i) ingestion of arbitrary user data, (ii) a
   `niche doctor --oracle` / test-suite oracle comparing our warps against `gdalwarp`.
5. External preprocessing: require users to deliver aligned rasters (as many SDM tools do) — shifts
   the scientific risk to the user; rejected as primary path.

## 5. Trade-offs

| Criterion (brief priority) | A | B | C | D (+optional B) |
|---|---|---|---|---|
| Scientific correctness | − (unproven code) | ++ | ++ | + (oracle-tested, restricted inputs) |
| Reproducibility | ++ | + | − | ++ |
| Performance | + (Rayon, no FFI copies) | + | + | + |
| Reliable geospatial for arbitrary inputs | − | ++ | ++ | − / + with gdal feature |
| Deployment simplicity | ++ | + | −− | ++ |
| Maintenance burden | −− | − (C++ toolchain) | + | − (own warper, own COG writer) |

Twelve-factor tension 8 (portability vs native libs): D keeps the default build free of native
geo libraries (factor II: "no undocumented native deps") while making the GDAL build an explicit,
documented variant rather than a hidden requirement.

## 6. Risks

- R1 Own warping/resampling bugs (half-pixel offsets, edge handling, NoData weighting) produce
  plausible but wrong rasters. Mitigation: golden tests vs `gdalwarp` for each method/layer kind,
  property tests (round-trip, conservation of sum under `sum`, class set preserved under `mode`).
- R2 proj4rs numerical divergence from PROJ for LAEA/LCC [OPEN]. Mitigation: grid of test points
  compared to PROJ 9.x with tolerance ≤ 1 mm.
- R3 Input files the decoder can't read (LERC, JPEG2000, Int16 if unsupported, odd GeoKeys).
  Mitigation: explicit error with conversion recipe; optional GDAL feature.
- R4 Young crates (`geotiff-writer`, `geopackage`) abandoned. Mitigation: wrap behind internal
  traits (`RasterReader`, `RasterWriter`, `CrsTransform`), keep ability to swap to GDAL.
- R5 Static GDAL build breaks on a target (Windows ARM, macOS). Mitigation: GDAL feature is
  optional; CI matrix builds it only where needed.
- R6 GDAL default COG overview resampling (CUBIC) silently applied to categorical outputs if B is
  used naively [05-21]. Mitigation: always pass OVERVIEW_RESAMPLING explicitly.
- R7 EPSG database version drift changes transformations between releases. Mitigation: record
  PROJ/proj4rs version + CRS definitions in run manifest.

## 7. Recommendations

- [REC] Adopt D: pure-Rust default build with an explicit internal geospatial kernel:
  - Grid model: `Grid { crs: CrsId, transform: affine, shape }`; one model grid per project, default
    EPSG:3035 for Europe-wide, EPSG:2154 allowed for France-only; never 3857 for analysis.
  - CRS: `proj4rs` 0.2.1 + `crs-definitions` 0.6.0 with an allow-list (4326, 3857, 3035, 2154,
    32630–32632, extendable via config) and PROJ-based golden tests in CI (the `proj` crate as a
    dev-dependency only).
  - Read: `tiff` 0.11.3 (features `zstd`, `deflate`) for local files; `async-tiff` 0.4.0 if remote
    COG range reads are needed.
  - Write: spike `geotiff-writer` 0.8.1; fall back to a minimal in-house tiled writer (DEFLATE +
    horizontal/float predictor, internal overviews) if the spike finds defects.
  - Vector: `geo` 0.33.1, `geo-types` 0.7.20, `geojson` 1.0.0, `rstar` 0.13.0,
    `geographiclib-rs` 0.2.7; GeoPackage via `rusqlite` 0.40.2 (`bundled`) — evaluate `geopackage`
    0.11.x but keep a thin own layer.
  - Rasterise / polygonise: own small implementations (scanline fill with documented
    centre-vs-all-touched rule; pixel-edge tracing polygoniser), tested against GDAL.
- [REC] Optional cargo feature `gdal` (`gdal` 0.19.0 + `gdal-sys` `bundled`, static GDAL 3.12.1,
  PROJ 9.6.2 with embedded proj.db) for arbitrary-input ingestion and as an oracle in CI. Do not
  ship it in the default binary until a spike measures size/build time on all targets.
- [REC] Layer metadata must declare `kind ∈ {continuous, categorical, count}`; resampling is derived
  from kind and direction (refine/coarsen), never left to a library default.
- [REC] Area: count cells on an equal-area model grid; when the model grid is geographic, use
  per-row geodesic cell area (geographiclib-rs). Report method in outputs.
- [REC] Persist CRS (WKT2/PROJJSON + EPSG), grid, resampling methods and library versions in
  every run manifest (doc 07 reproducibility).

## 8. Open questions

- [OPEN] proj4rs accuracy vs PROJ for LAEA (3035) and LCC (2154) — test needed.
- [OPEN] Does `tiff` 0.11.3 decode signed-integer (Int16) rasters? Test with a WorldClim/DEM file.
- [OPEN] Static GDAL build size/time per target; feasibility of gdal-src under cargo-zigbuild/xwin.
- [OPEN] Is the `geotiff-writer` COG output valid per GDAL `validate_cloud_optimized_geotiff.py`?
- [OPEN] Which user-supplied formats do Natural Solutions consultants actually need (Shapefile? ECW?)
  — determines whether the GDAL feature is a must.
- [OPEN] Default model grid for multi-country projects (3035 vs per-country).

## References

[05-1] crates.io API v1, crate and version metadata for all crates listed. https://crates.io/api/v1/crates/<name> (accessed 2026-10-15)
[05-2] georust/gdal README (supported GDAL versions, prebuilt bindings 3.8–3.13). https://github.com/georust/gdal (accessed 2026-10-15)
[05-3] georust/gdal `gdal-src/Cargo.toml` and `gdal-src/build.rs`, `gdal-sys/Cargo.toml` (`bundled` feature), master. https://github.com/georust/gdal/tree/master/gdal-src (accessed 2026-10-15)
[05-4] GitHub releases: OSGeo/gdal v3.13.3 (2026-08-18); OSGeo/PROJ 9.9.0 (2026-09-15). https://github.com/OSGeo/gdal/releases ; https://github.com/OSGeo/PROJ/releases (accessed 2026-10-15)
[05-5] proj crate 0.31.0, docs/README. https://github.com/georust/proj (accessed 2026-10-15)
[05-6] proj-sys 0.27.0 README and `build.rs` (MINIMUM_PROJ_VERSION 9.6.2, BUILD_SHARED_LIBS=OFF, ENABLE_CURL=OFF). https://github.com/georust/proj/tree/main/proj-sys (accessed 2026-10-15)
[05-7] 3liz/proj4rs README, `proj4rs/Cargo.toml`, `proj4rs/src/projections/` listing, v0.2.1. https://github.com/3liz/proj4rs (accessed 2026-10-15)
[05-8] wbprojection 0.3.3 docs. https://docs.rs/wbprojection/latest/wbprojection/ (accessed 2026-10-15)
[05-9] geo 0.33.1 docs (algorithm list). https://docs.rs/geo/0.33.1/geo/ (accessed 2026-10-15)
[05-10] geographiclib-rs 0.2.7 docs. https://docs.rs/geographiclib-rs/0.2.7/geographiclib_rs/ ; Karney C.F.F. (2013) Algorithms for geodesics. J. Geodesy 87:43–55. doi:10.1007/s00190-012-0578-z (accessed 2026-10-15)
[05-11] georust/geotiff README, v0.1.0. https://github.com/georust/geotiff (accessed 2026-10-15)
[05-12] georaster 0.2.0 docs. https://docs.rs/georaster/0.2.0/georaster/ (accessed 2026-10-15)
[05-13] developmentseed/async-tiff README, v0.4.0. https://github.com/developmentseed/async-tiff (accessed 2026-10-15)
[05-14] roteiro-gis/geotiff-rust README (tiff-reader/writer, geotiff-reader/writer 0.8.1). https://github.com/roteiro-gis/geotiff-rust (accessed 2026-10-15)
[05-15] cool-japan/oxigdal (now OxiGeo) README, v0.1.7. https://github.com/cool-japan/oxigdal (accessed 2026-10-15)
[05-16] stadiamaps/pmtiles-rs README, v0.25.0. https://github.com/stadiamaps/pmtiles-rs (accessed 2026-10-15)
[05-17] geopackage 0.11.2 docs. https://docs.rs/geopackage/latest/geopackage/ (accessed 2026-10-15)
[05-18] image-rs/image-tiff README, CHANGES.md, source at tag v0.11.3 (`src/decoder/mod.rs`, `src/decoder/image.rs`, `src/encoder/mod.rs`) and main; docs.rs tiff 0.11.3 `Decoder`, `tags::Predictor`. https://github.com/image-rs/image-tiff ; https://docs.rs/tiff/0.11.3/ (accessed 2026-10-15)
[05-19] PROJ installation docs, CMake options EMBED_RESOURCE_FILES / USE_ONLY_EMBEDDED_RESOURCE_FILES (PROJ 9.9.0 docs); PROJ RFC-8 (target PROJ 9.6). https://proj.org/en/stable/install.html ; https://proj.org/en/stable/community/rfc/rfc-8.html (accessed 2026-10-15)
[05-20] GDAL gdalwarp documentation (resampling methods, -tap, -te, -tr). https://gdal.org/en/stable/programs/gdalwarp.html (accessed 2026-10-15)
[05-21] GDAL COG driver documentation (COMPRESS, PREDICTOR, RESAMPLING default CUBIC for non-paletted, BLOCKSIZE, BIGTIFF). https://gdal.org/en/stable/drivers/raster/cog.html (accessed 2026-10-15)
