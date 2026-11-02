# Predictors v1 — terrain, land cover and distance to water

Status: implemented in `niche-geo` and `niche-data` (Science v1, phase 1d, 2026-10-20); offered
in the application catalogue since phase 4 (ADR-0015, *Proposed*): `env::layer_catalogue`,
`env::build_stack` (same native window and aggregation as CHELSA), reports and manifest. None of
these layers is selected by default; making any of them a default requires an ADR amending
ADR-0009 after review by Natural Solutions ecologists.

## Entry point

```rust
niche_data::predictors::read_window(&http, &cache, layer_id, (row0, col0, rows, cols)) -> Result<Raster, DataError>
niche_data::predictors::LAYERS         // &[chelsa::LayerDef], same shape as chelsa::LAYERS
niche_data::predictors::provenance(id) // source, vintage, licence (+URL), citation, attribution, method
```

The window is in pixel indices of the **native CHELSA v2.1 grid** (`chelsa::native_grid`), exactly
as for `chelsa::read_window`, and the returned raster has the same `GridSpec` as the CHELSA window:
layers stack without resampling (ADR-0010).

| id | unit | source | value per 30″ cell |
|---|---|---|---|
| `dem_elevation` | m | Copernicus DEM GLO-90 | mean of 3″ heights |
| `dem_slope` | ° | Copernicus DEM GLO-90 | mean of 3″ slopes (Horn 1981) |
| `dem_roughness` | m | Copernicus DEM GLO-90 | standard deviation of 3″ heights |
| `lc_tree`, `lc_shrub`, `lc_grass`, `lc_crop`, `lc_built`, `lc_bare`, `lc_water`, `lc_wetland` | fraction 0–1 | ESA WorldCover 2021 v200 | share of valid 10 m pixels in the class group |
| `dist_water` | km | ESA WorldCover 2021 v200 | distance to the nearest cell with water fraction ≥ 0.5, truncated at 25 km |

## Sources and licences (verified 2026-10-20)

### Copernicus DEM GLO-90

- Data: `s3://copernicus-dem-90m` (public, no authentication), one COG per 1°×1° tile, e.g.
  `https://copernicus-dem-90m.s3.amazonaws.com/Copernicus_DSM_COG_30_N43_00_E005_00_DEM/Copernicus_DSM_COG_30_N43_00_E005_00_DEM.tif`.
  Registry entry: <https://registry.opendata.aws/copernicus-dem/>.
- Licence: "Licence for COP-DEM-GLO-90-F Global 90m Full, Free & Open" (Copernicus WorldDEM-90),
  in the ESA licence bundle
  <https://dataspace.copernicus.eu/sites/default/files/media/files/2025-06/copernicus_contributing_mission_data_access_v2_cop_dem_licenses.pdf>
  (linked from <https://dataspace.copernicus.eu/explore-data/data-collections/copernicus-contributing-missions/collections-description/COP-DEM>).
  Article 4 grants any natural or legal person, free of charge (Art. 5), worldwide and without
  time limit (Art. 3): reproduction, distribution, communication to the public, adaptation,
  modification and combination with other data. The text does not mention commercial use
  explicitly; it grants the rights without restricting them. (Note: the same PDF also holds the
  general Copernicus Contributing Missions licence and the restricted COP-DEM-GLO-30-R /
  EEA-10-R licence, which *do* contain non-commercial clauses (e.g. "non-commercial public
  services"); they do not apply to the public GLO-90-F instance used here. A lawyer's
  confirmation is still advisable, as already noted in the design notes §2.)
- Required notices (Art. 6), recorded in `dem::ATTRIBUTION`:
  - adapted data: "produced using Copernicus WorldDEM-90 © DLR e.V. 2010-2014 and © Airbus Defence
    and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights
    reserved";
  - liability sentence: "The organisations in charge of the Copernicus programme by law or by
    delegation do not incur any liability for any use of the Copernicus WorldDEM-90";
  - do not suggest endorsement by ESA/EU/Airbus (Art. 6(d)).
- Citation: doi:10.5270/ESA-c5d3d65.

### ESA WorldCover 2021 v200

- Data: `s3://esa-worldcover/v200/2021/map/` (public), 3°×3° COGs, e.g.
  `https://esa-worldcover.s3.eu-central-1.amazonaws.com/v200/2021/map/ESA_WorldCover_10m_2021_v200_N42E003_Map.tif`.
- Licence: CC BY 4.0 (<https://creativecommons.org/licenses/by/4.0/>), stated on
  <https://esa-worldcover.org/en/data-access> ("provided free of charge, without restriction of
  use"). Commercial use allowed with attribution.
- Attribution (prescribed on the data-access page; also in the GeoTIFF `copyright` metadata):
  "© ESA WorldCover project 2021 / Contains modified Copernicus Sentinel data (2021) processed by
  ESA WorldCover consortium".
- Citation: Zanaga D. et al. (2022) ESA WorldCover 10 m 2021 v200. doi:10.5281/zenodo.7254221
  (full author list in `landcover::CITATION`).
- Note: v100 (2020) and v200 (2021) use different algorithms and must not be compared.

## Grid alignment and pixel conventions

The CHELSA lattice is west −180.000138889°, north 83.999860415°, cell 0.0083333333°
(43 200 × 20 880). Its edges sit **0.5″ (1/7200°) west/north of round degrees**. Neither source is
resampled: every source pixel is assigned to the CHELSA cell containing its **centre**.

- **Copernicus DEM** tiles declare `RasterPixelIsPoint`: the tie point (λ0, φ0+1) is the centre of
  the north-west pixel. Pixel centres are at φ0+1 − j·3″ and λ0 + i·Δλ; a tile contains its
  northern edge row but not its southern one. GDAL (and the `niche-geo` reader) place the outer
  edge half a pixel further out. Longitude spacing grows poleward of 50° (4.5″ for 50–60°, then
  6″, 9″, 15″, 30″): those tiles have non-square pixels, read with
  `GeoTiffInfo::read_allow_non_square` and georeferenced with `pixel_x`/`pixel_y`.
- **WorldCover** declares `RasterPixelIsArea` with 1/12 000° pixels: centres at λ0 + (i+½)/12 000.
- Because of the 0.5″ CHELSA offset, no centre of either source falls on a CHELSA edge (3″ and
  0.3″ grids vs 30″ cells offset by 0.5″), so the assignment is unambiguous and exact. A
  CHELSA-aligned 1° block takes its southern row partly from the next source degree; per-degree
  aggregates therefore cover up to 121 × 121 CHELSA cells and are **summed** (counts and sums are
  additive), never averaged, when a window is assembled.

## Aggregation methods

- **Elevation**: mean of valid 3″ heights whose centre is in the cell (~100 pixels below 50° N).
- **Slope**: per 3″ pixel, Horn's (1981) 3×3 finite differences with metric pixel sizes from the
  pixel row's latitude on WGS84 (ν·cosφ·Δλ east–west, ρ·Δφ north–south), then the **mean of the
  slopes** in the cell. Slope is computed before aggregation, never from aggregated elevation
  (which flattens relief). Implementation: dz/dx is the 1-2-1 weighted mean of the central
  differences of the three rows (identical to Horn's formula when the 8 neighbours exist; tested
  against the textbook formula). At tile edges and next to voids, a missing neighbour switches that
  row/column to a one-sided difference (exact for planar terrain; tested), because adjacent tiles
  are processed independently. This affects 1 of 1200 pixel rows/columns per tile edge.
- **Roughness**: standard deviation of the 3″ heights in the cell.
- **Land-cover fractions**: for each cell, (pixels of the group) / (valid pixels), counting every
  10 m pixel whose centre is in the cell (~8 000–10 000 per cell in France). NoData (0) is excluded
  from the denominator. The eight groups partition the valid classes, so fractions sum to 1
  (measured: max |Σ − 1| = 1.2·10⁻⁷ over 13 890 cells).

  | group | classes |
  |---|---|
  | tree | 10 |
  | shrub | 20 |
  | grass | 30 |
  | crop | 40 |
  | built | 50 |
  | bare | 60 bare/sparse, 70 snow/ice, 100 moss/lichen |
  | water | 80 permanent water bodies |
  | wetland | 90 herbaceous wetland, 95 mangroves |

- **Validity rule (all layers)**: a cell is NoData when fewer than 50 % of the source pixel
  centres it contains are valid. Absent tiles (sea; not in the bucket's tile list) count as
  invalid pixels.
- **Why WorldCover is read at full resolution**: the COGs have six internal overviews
  (18 000² … 562² per 3° tile), but an overview of a class map is itself a class map (nearest or
  mode sampling). A 1 km overview pixel holds one class, so "fractions" would be 0 or 1 and
  linear or small features (rivers, hedgerows, villages) would disappear or be exaggerated depending
  on the sampling phase. Exact fractions need all 10 m pixels.
- **Streaming**: `GeoTiffInfo::visit_window` decodes one internal 1024² tile at a time; byte
  ranges of consecutive tiles (GDAL COGs separate them by an 8-byte leader/trailer) are merged into
  one request of ≤ 8 MiB. A 1° block never holds more than one batch of compressed bytes plus one
  decoded tile; a whole 3° tile (1.3 G pixels) is never in memory.

## Distance to water

`niche_geo::distance::distance_to_mask_km` is an exact Euclidean distance transform
(Felzenszwalb & Huttenlocher 2012, two separable passes, O(n)) in a locally metric approximation
of the lon/lat grid: north–south positions are the WGS84 meridian arcs between row centres; the
east–west offset uses the parallel scale of the *target's* row. It equals a brute-force search
under the same metric exactly (tested on four random grids) and is within 1 % of the great-circle
distance up to ~50 km (tested); the approximation error is ≈ tanφ·Δφ relative (≈ 0.4 % at 45° N
for 25 km).

`dist_water` = distance from each cell centre to the nearest cell whose `lc_water` fraction is
≥ 0.5, **truncated at 25 km**. `predictors::read_window` pads the window by ≥ 25 km on every side
(rows: 25 km at the smallest meridian scale; columns: at the highest latitude of the padded area,
plus one cell), computes the transform on the padded window and crops. With this padding every
value ≤ 25 km is exact and independent of the window, and every truncated value is a correct
"≥ 25 km". Without truncation, values beyond the padding would depend on the window extent
(water outside the window is invisible) and results would not be reproducible across study
areas. Cells with unknown land cover are NoData.

Limits: water means ≥ 50 % of a ~1 km² cell, i.e. lakes, reservoirs, wide rivers, lagoons and the
**sea** (WorldCover maps the sea inside its tiles as class 80). Rivers narrower than ~500 m are not
"water cells"; a river network source (e.g. BD TOPAGE, France only; EU-Hydro, licence to check) is
needed for riparian species.

## Caches

Under the instance cache directory (same as CHELSA):

| path | content | size (measured) |
|---|---|---|
| `dem/tileList.txt` | list of existing GLO-90 tiles (26 475) | 1.11 MB, once |
| `dem/agg-v1/<tile>.bin` | per-tile counts/sums on ≤ 121×121 CHELSA cells (zlib) | ~233 KB per land tile (N43E005) |
| `worldcover/v200-2021-grid.geojson` | list of existing 3° tiles (2 651) | 544 KB, once |
| `worldcover/agg-v1/<N43E005>.bin` | per-degree class-group counts on ≤ 121×121 cells (zlib) | ~121 KB per degree (N43E005) |

Files carry a magic, a format version (`AGG_VERSION`, bump when a method changes) and the CHELSA
grid they refer to (bit-exact); a mismatch triggers recomputation. Writes are atomic (temp file
then rename), so concurrent processes cannot read a partial block. Each degree is downloaded and
computed once per instance. Estimated cache for France + Spain (~120 degree blocks): ~28 MB DEM,
~15 MB WorldCover [estimate from the two measured blocks; not measured].

## Measurements (2026-10-20, this workstation, home connection to AWS eu-central-1 / us-east-1)

From `cargo test --release -p niche-data --test live_predictors -- --ignored --nocapture`, fresh
caches:

| operation | result |
|---|---|
| DEM, CHELSA-aligned 1° window at 43–44° N 5–6° E (tile N43E005 4.28 MB file, ~3.7 MB read; N42E005 absent = sea) | 1.0–1.3 s first read; other layers from cache 11–31 ms |
| DEM values in that window | elevation −2.5 … 1215.6 m (mean 255.2); slope 0.0 … 31.0° (mean 5.33); roughness 0 … 174 m; 14 400/14 400 valid; sea cell (5.25° E 43.2° N) = 0 m; max raw 3″ pixel 1252.1 m at 43.98° N 5.485° E (Monts de Vaucluse, Signal de Saint-Pierre 1256 m) |
| WorldCover 1° block N43E005 (144 M pixels) | **8.31 MB in 13 range requests, 3.3–4.4 s** over two runs (download + decode + aggregate); before merging ranges across the 8-byte gaps it was 169 requests, 11.0 s |
| same block from cache | 0.9–2.9 ms |
| fractions in that degree | tree 0.474, water 0.233 (sea south of Marseille), grass 0.086, shrub 0.068, crop 0.068, built 0.066, bare 0.004, wetland 0.001; Σ = 1 ± 1.2·10⁻⁷ |
| `dist_water`, 30×30 window at the Étang de Berre, padded to 88×108 (two WorldCover blocks) | 5.0 s fresh; 0 … 13.2 km, mean 2.7 km |

No speed claim beyond these numbers. The cost of a first study over a new region is dominated by
WorldCover (≈ 4–5 s and ≈ 8 MB per degree on this connection, four blocks in parallel).

## Known limits

- **Temporal mismatch**: WorldCover is 2021; the climate baseline is 1981–2010 and GBIF records
  start in 1981. Land use may have changed at older records. Every land-cover layer and
  `dist_water` carry this caution; a year filter is proposed in the design notes
  §3.5 (not implemented).
- **DSM, not terrain**: Copernicus DEM is a surface model (TanDEM-X 2011–2015) — heights include
  forest canopy and buildings, so wooded and urban cells look higher and rougher than the ground.
- **Built-up and sampling bias**: `lc_built` correlates with observer effort in opportunistic
  records; its caution says so.
- **Compositional data**: the eight fractions sum to 1; selecting all of them creates an exact
  linear dependency that the pairwise Pearson filter does not detect. Leave at least one group out.
- **Edges**: distance is exact only up to the 25 km truncation; DEM slopes at 1° tile edges use
  one-sided differences; there is no handling of the ±180° meridian (not relevant for Europe).
- **Coastal cells**: CHELSA has values over the sea; WorldCover marks the sea as water and the
  DEM gives 0 m there. The land mask (ADR-0010) remains necessary.
- **Datums**: all sources are WGS84/ITRF-based; no datum shift is applied (sub-metre).

## CRS transforms (`niche_geo::crs`)

Not used by the predictors above (all EPSG:4326) but needed for user data and exports:
`Crs::{Wgs84, Lambert93, Laea3035, WebMercator, Utm{zone, north}}`, `Crs::from_epsg`,
`to_wgs84(x, y)`, `from_wgs84(lng, lat)`. Formulas: IOGP Guidance Note 7-2 (LCC 2SP §3.1.1.1,
LAEA §3.1.4.2, JHS Transverse Mercator §3.1.3.1, Pseudo-Mercator §1.3.3.2). Verified against the
GN 7-2 worked examples (Texas South Central LCC, British National Grid TM, LAEA Europe,
Pseudo-Mercator) to 0.01 m, against PROJ 9.8.1 on 16 points in France, Corsica, Spain and the
Canaries to < 1 mm, and by round trips < 1 mm over 27–52° N, 18° W–19° E. RGF93 v1 and ETRS89
are treated as WGS84 (difference < 1 m, irrelevant at 1 km cells); ETRS89 UTM codes map to WGS84
UTM (0.1 mm difference from the GRS80/WGS84 flattening).
