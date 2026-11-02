# ADR-0010 — Model on the native CHELSA geographic grid with exact areas and a land mask

- Status: Accepted (2026-10-15). Refines ADR-0003 (which assumed an EPSG:3035 modelling grid).

## Context

The default predictors (CHELSA v2.1) are global 30 arc-second (1/120°) geographic grids. Projecting
them to an equal-area grid (EPSG:3035) would require resampling every predictor — an avoidable
source of interpolation error — and a PROJ-quality transformation in pure Rust (ADR-0003 spike
still open). Two facts found while implementing:

1. CHELSA v2.1 GeoTIFFs store one row per strip (Deflate + horizontal predictor, uint16 with
   GDAL scale/offset), so a study-area window is a few MB of HTTP range requests.
2. **CHELSA v2.1 has values over the ocean** (e.g. BIO1 = 13.65 °C at 6° W, 46.5° N, Bay of
   Biscay; verified 2026-10-15). Without a land mask, background points for terrestrial species
   would fall at sea, inflating AUC/CBI and distorting response curves.

## Decision

- The modelling grid is the native CHELSA lattice (optionally aggregated by an integer factor with
  majority-valid mean aggregation). No reprojection or resampling of predictors.
- Areas are computed per cell with the exact ellipsoidal quadrangle formula on WGS84
  (Snyder 1987), tested against the WGS84 total surface (510 065 621.7 km²).
- Spatial blocks are defined in geographic coordinates (median latitude/longitude splits).
- Land mask: Natural Earth 1:10m land ∪ minor islands minus lakes (public domain), rasterized at
  cell centres; records at sea or in large lakes are excluded with an explicit reason.
- Map display uses server-rendered Web-Mercator tiles sampled from the native grid (display only).
- User-supplied rasters must be EPSG:4326 for now; other CRSs are rejected with an explicit message.

## Consequences

- No resampling error in predictors; exact areas; no PROJ dependency in the default build.
- Cells are not equal-area (≈ 0.93 × 0.6 km at 50° N); background sampling is by cell, which
  slightly over-weights northern cells within large north–south extents. For study areas within
  France (≈ 9° of latitude) the cell-area ratio is ≤ 1.2; documented, acceptable.
- Natural Earth 1:10m coastline has ~0.5–1 km positional uncertainty: coastal cells may be
  misclassified; coastal records are therefore listed with their exclusion reason for review.
- Marine and coastal-intertidal species are out of scope until a marine predictor set exists.
