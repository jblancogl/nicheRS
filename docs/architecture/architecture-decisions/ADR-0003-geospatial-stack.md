# ADR-0003 — Pure-Rust geospatial core with a restricted format/CRS allow-list; GDAL optional

- Status: Accepted (spikes pending: COG writer, proj4rs accuracy, Int16 reading)
- Evidence: `docs/research/05-geospatial-engineering.md`, `docs/research/13-single-binary-feasibility.md`

## Decision
Default build uses pure-Rust crates: `tiff` (GeoTIFF/COG reading), a tiled COG writer
(`geotiff-writer` spike, else in-house), `proj4rs` + `crs-definitions` restricted to an allow-list
(EPSG:4326, 3035, 2154, 3857 initially), `geo`/`geojson`/`rstar`, `geographiclib-rs` for geodesic
area, `rusqlite` (bundled) for GeoPackage. A `gdal` cargo feature (static GDAL + PROJ) is offered
for wide format support and used as a CI reference for reprojection/resampling accuracy.

## Rationale
A genuinely self-contained binary without GDAL/PROJ is practical **only** with restricted inputs.
Restrictions are explicit and checked at ingestion (unsupported CRS ⇒ clear error, never silent
assumption). Resampling is chosen by declared layer type (continuous → bilinear/average;
categorical → nearest/mode; counts → sum), never by a global default.

## Risks
proj4rs accuracy vs PROJ untested; datum-shift grids unsupported (acceptable for the listed CRSs
on ETRS89/WGS84 at SDM resolutions, to be quantified). Reversible: switch default feature to GDAL.

## Implementation note (2026-10-15)

The MVP needed no third-party geo crate: NicheRS ships its own minimal GeoTIFF reader/writer (`niche-geo`), works on the native CHELSA geographic grid and computes exact ellipsoidal areas (ADR-0010). `proj4rs`/GDAL remain the path for reprojecting user rasters (not yet supported; non-EPSG:4326 inputs are rejected explicitly).

## Implementation note (2026-10-20, Science v1 phase 1d)

`niche-geo::crs` implements, without PROJ or `proj4rs`, forward and inverse transforms between
WGS84 and EPSG:2154, 3035, 3857, 326xx/327xx and 25828–25838 with the IOGP Guidance Note 7-2
formulas (LCC 2SP, ellipsoidal LAEA, JHS Transverse Mercator, Pseudo-Mercator). It reproduces the
GN 7-2 worked examples to 0.01 m and PROJ 9.8.1 to < 1 mm on 16 points in France and Spain;
round trips are < 1 mm. RGF93/ETRS89 are treated as WGS84 (sub-metre). Raster reprojection is
still out of scope: new predictors (Copernicus DEM, ESA WorldCover) are already EPSG:4326 and are
aggregated onto the CHELSA lattice by pixel centre (`docs/architecture/predictors-v1.md`).
