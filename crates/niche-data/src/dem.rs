//! Copernicus DEM GLO-90 (3″ ≈ 90 m) terrain aggregated to the native CHELSA 30″ grid.
//!
//! **Source.** Public Cloud Optimized GeoTIFFs on AWS (`s3://copernicus-dem-90m`, no
//! authentication), one 1°×1° tile per object, EPSG:4326 horizontally, EGM2008 heights in metres,
//! Float32, Deflate + floating-point predictor (3), one 2048×2048 internal tile holding the
//! 1200-row image. Tiles that are entirely sea do not exist (HTTP 404); the bucket publishes the
//! list of existing tiles in `tileList.txt` (26 475 names, checked 2026-10-20), which is cached
//! and used instead of probing 404s.
//!
//! **Pixel convention.** The tiles declare `RasterPixelIsPoint`: the tie point (λ0, φ0 + 1) is
//! the *centre* of the north-west pixel, so pixel centres are at φ0 + 1 − j·Δφ (j = 0…1199) and
//! λ0 + i·Δλ. A tile therefore holds its northern edge row but not its southern one (that row
//! is row 0 of the tile below). GDAL places the outer edge half a pixel further out; the
//! `niche-geo` reader does the same. Longitude spacing is 3″ below 50° latitude and coarser
//! poleward (4.5″ for 50–60°, 6″, 9″, 15″, 30″), so tiles north of 50° have non-square pixels.
//!
//! **Aggregation** (per CHELSA 30″ cell, ~100 DEM pixels at 3″):
//! - `elevation`: mean of the pixel heights (m);
//! - `slope`: mean of per-pixel slopes in degrees, each computed at 90 m with Horn's (1981)
//!   3×3 weighted finite differences using metric pixel sizes from the latitude of each row
//!   (ellipsoidal ν·cosφ·Δλ and ρ·Δφ on WGS84). Slope is computed **before** aggregation —
//!   never from the aggregated elevation, which would flatten relief;
//! - `roughness`: standard deviation of the pixel heights within the cell (m).
//!
//! A cell is NoData when fewer than half of the pixel centres it contains are valid (absent sea
//! tiles count as invalid). At tile edges, the 3×3 window lacks the neighbouring tile: the
//! derivative falls back to one-sided differences for that row/column (1 of 1200 rows; in an
//! affected CHELSA cell, 10 of ~100 pixels), which keeps an unbiased gradient estimate but with
//! less smoothing.
//!
//! **Caveat.** Copernicus DEM is a *surface* model (DSM, TanDEM-X 2011–2015): heights include
//! canopy and buildings, so forests and cities appear higher and rougher than the ground.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use niche_geo::geotiff::{ByteSource, Chunk, GeoTiffInfo};
use niche_geo::{GridSpec, Raster, crs::metres_per_degree};

use crate::DataError;
use crate::cellagg::{Agg, Footprint, WindowAcc, chelsa_col, chelsa_row, parallel_map};
use crate::http::{Http, HttpSource};

pub const SOURCE: &str =
    "Copernicus DEM GLO-90 (COP-DEM-GLO-90-F), 3 arc-second surface model, AWS Open Data copy";
pub const LICENCE: &str = "Copernicus WorldDEM-90 licence (free of charge: reproduction, distribution, communication to the public, adaptation; worldwide, no time limit)";
pub const LICENCE_URL: &str = "https://dataspace.copernicus.eu/sites/default/files/media/files/2025-06/copernicus_contributing_mission_data_access_v2_cop_dem_licenses.pdf";
/// Notice required by Article 6(b) of the licence for adapted/modified data (our aggregates).
pub const ATTRIBUTION: &str = "produced using Copernicus WorldDEM-90 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved. The organisations in charge of the Copernicus programme by law or by delegation do not incur any liability for any use of the Copernicus WorldDEM-90.";
pub const CITATION: &str = "European Space Agency. Copernicus DEM (COP-DEM GLO-90). doi:10.5270/ESA-c5d3d65; accessed from https://registry.opendata.aws/copernicus-dem (s3://copernicus-dem-90m)";

const BASE: &str = "https://copernicus-dem-90m.s3.amazonaws.com";
/// Cache format version of the per-tile aggregates; bump when the method changes.
const AGG_VERSION: u32 = 1;
const MAGIC: &[u8; 4] = b"NDEM";
// Counts: pixel centres, valid heights, valid slopes. Sums: z, z², slope.
const C_TOTAL: usize = 0;
const C_VALID: usize = 1;
const C_SLOPE: usize = 2;
const S_Z: usize = 0;
const S_Z2: usize = 1;
const S_SLOPE: usize = 2;

/// Terrain layer derived from the DEM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemLayer {
    Elevation,
    Slope,
    Roughness,
}

/// Object name of the tile whose south-west corner is (lat0, lon0).
pub fn tile_name(lat0: i32, lon0: i32) -> String {
    let ns = if lat0 >= 0 { 'N' } else { 'S' };
    let ew = if lon0 >= 0 { 'E' } else { 'W' };
    format!(
        "Copernicus_DSM_COG_30_{ns}{:02}_00_{ew}{:03}_00_DEM",
        lat0.abs(),
        lon0.abs()
    )
}

pub fn tile_url(lat0: i32, lon0: i32) -> String {
    let n = tile_name(lat0, lon0);
    format!("{BASE}/{n}/{n}.tif")
}

/// GLO-90 longitude spacing (arc-seconds) for the tile starting at `lat0` (Copernicus DEM
/// Product Handbook, latitude-dependent spacing; verified on N43E005 = 3″, N50E002 = 4.5″).
pub fn spacing_x_arcsec(lat0: i32) -> f64 {
    let m = lat0.abs().min((lat0 + 1).abs());
    match m {
        0..=49 => 3.0,
        50..=59 => 4.5,
        60..=69 => 6.0,
        70..=79 => 9.0,
        80..=84 => 15.0,
        _ => 30.0,
    }
}

/// Names of the existing tiles (cached `tileList.txt`).
pub fn tile_list(http: &Http, cache: &Path) -> Result<HashSet<String>, DataError> {
    let f = cache.join("dem").join("tileList.txt");
    if !f.exists() {
        if let Some(d) = f.parent() {
            std::fs::create_dir_all(d).map_err(|e| DataError::Cache(e.to_string()))?;
        }
        http.download_to_file(&format!("{BASE}/tileList.txt"), &f, 16 << 20)?;
    }
    let text = std::fs::read_to_string(&f).map_err(|e| DataError::Cache(e.to_string()))?;
    let set: HashSet<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    if set.len() < 1000 {
        let _ = std::fs::remove_file(&f);
        return Err(DataError::Format {
            url: format!("{BASE}/tileList.txt"),
            msg: format!("only {} tile names", set.len()),
        });
    }
    Ok(set)
}

/// CHELSA cells covered by the pixel centres of the tile at (lat0, lon0).
fn tile_footprint(grid: &GridSpec, lat0: i32, lon0: i32) -> Footprint {
    let (py, px): (f64, f64) = (3.0 / 3600.0, spacing_x_arcsec(lat0) / 3600.0);
    let (la, lo) = (f64::from(lat0), f64::from(lon0));
    Footprint::of_centres(grid, la + py, la + 1.0, lo, lo + 1.0 - px)
}

/// Aggregates of a tile that does not exist (sea): pixel centres counted, none valid.
fn absent_tile(grid: &GridSpec, lat0: i32, lon0: i32) -> Agg {
    let fp = tile_footprint(grid, lat0, lon0);
    let mut agg = Agg::new(fp, 3, 3);
    let (py, px): (f64, f64) = (3.0 / 3600.0, spacing_x_arcsec(lat0) / 3600.0);
    let nrows = (1.0 / py).round() as usize;
    let ncols = (1.0 / px).round() as usize;
    let top = f64::from(lat0) + 1.0;
    let rows: Vec<i64> = (0..nrows)
        .map(|j| chelsa_row(grid, top - j as f64 * py))
        .collect();
    let cols: Vec<i64> = (0..ncols)
        .map(|i| chelsa_col(grid, f64::from(lon0) + i as f64 * px))
        .collect();
    for &r in &rows {
        for &c in &cols {
            if let Some(k) = fp.index(r, c) {
                agg.counts[C_TOTAL][k] += 1;
            }
        }
    }
    agg
}

/// Geometry of a DEM image: pixel (j, i) has its centre at
/// `lng = west_centre + i·px`, `lat = north_centre − j·py` (degrees).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelCentres {
    pub width: usize,
    pub height: usize,
    pub west_centre: f64,
    pub north_centre: f64,
    pub px: f64,
    pub py: f64,
}

/// Aggregate a DEM image (row-major heights, NaN = void) onto the CHELSA grid.
pub fn aggregate_heights(
    z: &[f32],
    geo: PixelCentres,
    grid: &GridSpec,
) -> Result<AggregatedTile, DataError> {
    let PixelCentres {
        width,
        height,
        west_centre,
        north_centre,
        px,
        py,
    } = geo;
    if z.len() != width * height || width < 2 || height < 2 {
        return Err(DataError::Format {
            url: "dem".into(),
            msg: "image size".into(),
        });
    }
    let lat = |j: usize| north_centre - j as f64 * py;
    let lng = |i: usize| west_centre + i as f64 * px;
    let fp = Footprint::of_centres(grid, lat(height - 1), lat(0), lng(0), lng(width - 1));
    let rows: Vec<usize> = (0..height)
        .map(|j| (chelsa_row(grid, lat(j)) - fp.row0) as usize)
        .collect();
    let cols: Vec<usize> = (0..width)
        .map(|i| (chelsa_col(grid, lng(i)) - fp.col0) as usize)
        .collect();
    // Metric pixel sizes (m) per row.
    let dx: Vec<f64> = (0..height)
        .map(|j| metres_per_degree(lat(j)).0 * px)
        .collect();
    let dy: Vec<f64> = (0..height)
        .map(|j| metres_per_degree(lat(j)).1 * py)
        .collect();
    let mut agg = Agg::new(fp, 3, 3);
    for j in 0..height {
        for i in 0..width {
            let k = rows[j] * fp.cols + cols[i];
            agg.counts[C_TOTAL][k] += 1;
            let v = z[j * width + i];
            if !v.is_finite() {
                continue;
            }
            let v = f64::from(v);
            agg.counts[C_VALID][k] += 1;
            agg.sums[S_Z][k] += v;
            agg.sums[S_Z2][k] += v * v;
            if let Some(s) = horn_slope_deg(z, width, height, j, i, &dx, dy[j]) {
                agg.counts[C_SLOPE][k] += 1;
                agg.sums[S_SLOPE][k] += s;
            }
        }
    }
    Ok(AggregatedTile(agg))
}

/// Opaque per-tile aggregate (counts and sums on the CHELSA lattice).
#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedTile(pub(crate) Agg);

/// Horn (1981) slope in degrees at pixel (j, i): dz/dx is the 1-2-1 weighted mean of the central
/// differences of rows j−1, j, j+1 (Horn's formula exactly when all 8 neighbours exist); missing
/// neighbours (image edge, voids) fall back to one-sided differences, or drop that row. Same for
/// dz/dy over columns. Row-specific east–west pixel sizes `dx[j]`.
fn horn_slope_deg(
    z: &[f32],
    w: usize,
    h: usize,
    j: usize,
    i: usize,
    dx: &[f64],
    dy: f64,
) -> Option<f64> {
    let at = |r: usize, c: usize| -> Option<f64> {
        let v = z[r * w + c];
        v.is_finite().then_some(f64::from(v))
    };
    let deriv = |prev: Option<f64>, centre: Option<f64>, next: Option<f64>, step: f64| match (
        prev, centre, next,
    ) {
        (Some(a), _, Some(b)) => Some((b - a) / (2.0 * step)),
        (None, Some(c), Some(b)) => Some((b - c) / step),
        (Some(a), Some(c), None) => Some((c - a) / step),
        _ => None,
    };
    let (mut gx, mut wx) = (0.0, 0.0);
    let (mut gy, mut wy) = (0.0, 0.0);
    for d in -1i64..=1 {
        let weight = if d == 0 { 2.0 } else { 1.0 };
        let r = j as i64 + d;
        if r >= 0 && (r as usize) < h {
            let r = r as usize;
            let prev = (i > 0).then(|| at(r, i - 1)).flatten();
            let next = (i + 1 < w).then(|| at(r, i + 1)).flatten();
            if let Some(g) = deriv(prev, at(r, i), next, dx[r]) {
                gx += weight * g;
                wx += weight;
            }
        }
        let c = i as i64 + d;
        if c >= 0 && (c as usize) < w {
            let c = c as usize;
            // Rows run southwards: "next" is the southern neighbour; the sign is irrelevant.
            let prev = (j > 0).then(|| at(j - 1, c)).flatten();
            let next = (j + 1 < h).then(|| at(j + 1, c)).flatten();
            if let Some(g) = deriv(prev, at(j, c), next, dy) {
                gy += weight * g;
                wy += weight;
            }
        }
    }
    if wx == 0.0 || wy == 0.0 {
        return None;
    }
    let (gx, gy) = (gx / wx, gy / wy);
    Some((gx * gx + gy * gy).sqrt().atan().to_degrees())
}

/// Aggregate a whole DEM GeoTIFF (any [`ByteSource`]) onto the CHELSA grid.
pub fn aggregate_geotiff(
    src: &dyn ByteSource,
    grid: &GridSpec,
) -> Result<AggregatedTile, DataError> {
    let info = GeoTiffInfo::read_allow_non_square(src)?;
    let (w, h) = (info.width, info.height);
    let mut z = vec![f32::NAN; w * h];
    info.visit_window(src, 0, 0, h, w, &mut |ch: &Chunk| {
        for r in 0..ch.height {
            let dst = (ch.row0 + r) * w + ch.col0;
            z[dst..dst + ch.width]
                .copy_from_slice(&ch.values[r * ch.stride..r * ch.stride + ch.width]);
        }
        Ok(())
    })?;
    let (px, py) = (info.pixel_x, info.pixel_y);
    // `spec.west/north` are outer edges; centres are half a pixel inside.
    let geo = PixelCentres {
        width: w,
        height: h,
        west_centre: info.spec.west + px / 2.0,
        north_centre: info.spec.north - py / 2.0,
        px,
        py,
    };
    aggregate_heights(&z, geo, grid)
}

fn agg_path(cache: &Path, lat0: i32, lon0: i32) -> PathBuf {
    cache
        .join("dem")
        .join(format!("agg-v{AGG_VERSION}"))
        .join(format!("{}.bin", tile_name(lat0, lon0)))
}

/// Aggregates of one tile: from the cache, else downloaded and computed (then cached).
fn tile_agg(
    http: &Http,
    cache: &Path,
    grid: &GridSpec,
    list: &HashSet<String>,
    lat0: i32,
    lon0: i32,
) -> Result<Agg, DataError> {
    if !list.contains(&tile_name(lat0, lon0)) {
        return Ok(absent_tile(grid, lat0, lon0));
    }
    let path = agg_path(cache, lat0, lon0);
    if let Some(a) = Agg::load(&path, MAGIC, AGG_VERSION, grid) {
        return Ok(a);
    }
    let url = tile_url(lat0, lon0);
    let src = HttpSource::new(http.clone(), url.clone());
    let t = std::time::Instant::now();
    let AggregatedTile(agg) = aggregate_geotiff(&src, grid)?;
    let expected = tile_footprint(grid, lat0, lon0);
    if agg.fp != expected {
        return Err(DataError::Format {
            url,
            msg: format!(
                "tile geometry {:?} differs from the GLO-90 grid {expected:?}",
                agg.fp
            ),
        });
    }
    tracing::debug!(tile = %tile_name(lat0, lon0), elapsed = ?t.elapsed(), "DEM tile aggregated");
    agg.save(&path, MAGIC, AGG_VERSION, grid)?;
    Ok(agg)
}

/// Window in CHELSA pixel indices (row0, col0, rows, cols), as in [`crate::chelsa::read_window`].
pub type Window = (usize, usize, usize, usize);

/// Tiles (lat0, lon0) whose pixel centres fall in the window.
fn tiles_for(grid: &GridSpec, window: Window) -> Vec<(i32, i32)> {
    let (r0, c0, rows, cols) = window;
    let north = grid.north - r0 as f64 * grid.cell;
    let south = north - rows as f64 * grid.cell;
    let west = grid.west + c0 as f64 * grid.cell;
    let east = west + cols as f64 * grid.cell;
    let wfp = Footprint {
        row0: r0 as i64,
        col0: c0 as i64,
        rows,
        cols,
    };
    let mut out = Vec::new();
    for lat0 in (south.floor() as i32 - 1)..=(north.ceil() as i32) {
        if !(-90..90).contains(&lat0) {
            continue;
        }
        for lon0 in (west.floor() as i32 - 1)..=(east.ceil() as i32) {
            let lon0 = (lon0 + 180).rem_euclid(360) - 180;
            if tile_footprint(grid, lat0, lon0).intersects(&wfp) && !out.contains(&(lat0, lon0)) {
                out.push((lat0, lon0));
            }
        }
    }
    out
}

/// Accumulated counts/sums of every DEM tile over a CHELSA window.
fn window_acc(
    http: &Http,
    cache: &Path,
    grid: &GridSpec,
    window: Window,
) -> Result<WindowAcc, DataError> {
    let list = tile_list(http, cache)?;
    let tiles = tiles_for(grid, window);
    let aggs = parallel_map(&tiles, 4, |&(la, lo)| {
        tile_agg(http, cache, grid, &list, la, lo)
    });
    let mut acc = WindowAcc::new(window, 3, 3);
    for a in aggs {
        acc.add(&a?);
    }
    Ok(acc)
}

/// Final layer values from accumulated counts/sums (majority-valid rule).
fn finish(acc: &WindowAcc, grid: &GridSpec, layer: DemLayer) -> Raster {
    let fp = acc.footprint();
    let spec = GridSpec {
        west: grid.west + fp.col0 as f64 * grid.cell,
        north: grid.north - fp.row0 as f64 * grid.cell,
        cell: grid.cell,
        width: fp.cols,
        height: fp.rows,
    };
    let a = &acc.agg;
    let data = (0..fp.len())
        .map(|k| {
            let total = a.counts[C_TOTAL][k];
            let n = a.counts[C_VALID][k];
            if total == 0 || 2 * n < total {
                return f32::NAN;
            }
            let nf = f64::from(n);
            match layer {
                DemLayer::Elevation => (a.sums[S_Z][k] / nf) as f32,
                DemLayer::Roughness => {
                    let m = a.sums[S_Z][k] / nf;
                    ((a.sums[S_Z2][k] / nf - m * m).max(0.0)).sqrt() as f32
                }
                DemLayer::Slope => {
                    let ns = a.counts[C_SLOPE][k];
                    if 2 * ns < total {
                        f32::NAN
                    } else {
                        (a.sums[S_SLOPE][k] / f64::from(ns)) as f32
                    }
                }
            }
        })
        .collect();
    Raster { spec, data }
}

/// Read a DEM-derived layer on a native CHELSA window (`grid` = [`crate::chelsa::native_grid`]).
pub fn read_window(
    http: &Http,
    cache: &Path,
    grid: &GridSpec,
    window: Window,
    layer: DemLayer,
) -> Result<Raster, DataError> {
    let acc = window_acc(http, cache, grid, window)?;
    Ok(finish(&acc, grid, layer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use niche_geo::write::write_geotiff_f32;

    pub(crate) fn chelsa_like() -> GridSpec {
        GridSpec {
            west: -180.000_138_889,
            north: 83.999_860_415,
            cell: 1.0 / 120.0,
            width: 43_200,
            height: 20_880,
        }
    }

    #[test]
    fn names_and_spacing() {
        assert_eq!(tile_name(43, 5), "Copernicus_DSM_COG_30_N43_00_E005_00_DEM");
        assert_eq!(
            tile_name(-1, -78),
            "Copernicus_DSM_COG_30_S01_00_W078_00_DEM"
        );
        assert_eq!(spacing_x_arcsec(43), 3.0);
        assert_eq!(spacing_x_arcsec(49), 3.0);
        assert_eq!(spacing_x_arcsec(50), 4.5);
        assert_eq!(spacing_x_arcsec(-51), 4.5);
        assert_eq!(spacing_x_arcsec(-50), 3.0);
    }

    /// The tile footprints partition the CHELSA lattice: each CHELSA cell's pixel centres come
    /// from at most two tiles in latitude (the 0.5″ offset) and one in longitude.
    #[test]
    fn footprint_of_a_tile() {
        let g = chelsa_like();
        let fp = tile_footprint(&g, 43, 5);
        assert_eq!((fp.rows, fp.cols), (121, 120));
        // Northern edge row (lat 44) belongs to the CHELSA row just above the 1° block.
        assert_eq!(fp.row0, chelsa_row(&g, 44.0));
        assert_eq!(fp.col0, chelsa_col(&g, 5.0));
        let a = absent_tile(&g, 43, 5);
        let total: u32 = a.counts[C_TOTAL].iter().sum();
        assert_eq!(total, 1200 * 1200);
        // Interior cell: 10×10 pixel centres; top row of the footprint: one pixel row only.
        assert_eq!(a.counts[C_TOTAL][fp.cols * 60 + 60], 100);
        assert_eq!(a.counts[C_TOTAL][60], 10);
        assert!(a.counts[C_VALID].iter().all(|&v| v == 0));
    }

    /// A synthetic tilted plane, written with the project's GeoTIFF writer: z rises 0.1 m per
    /// metre northwards (slope atan(0.1) = 5.7106°) plus a constant 250 m. Mean elevation of each
    /// cell equals the plane at the mean pixel position; slope equals 5.7106° everywhere
    /// (including tile edges, where one-sided differences are exact for a plane).
    #[test]
    fn plane_slope_and_mean_elevation() {
        let g = chelsa_like();
        let n = 240; // 0.2° at 3″
        let cell = 1.0 / 1200.0;
        let spec = GridSpec {
            west: 5.0,
            north: 44.0,
            cell,
            width: n,
            height: n,
        };
        let mut r = Raster::new(spec, 0.0);
        let lat_c = |j: usize| 44.0 - (j as f64 + 0.5) * cell;
        // Northing (m) relative to the south edge, by the meridian arc of each row.
        for j in 0..n {
            let mut y = 0.0;
            for jj in (j + 1..n).rev() {
                y += metres_per_degree((lat_c(jj) + lat_c(jj - 1)) / 2.0).1 * cell;
            }
            for i in 0..n {
                r.data[j * n + i] = (250.0 + 0.1 * y) as f32;
            }
        }
        let bytes = write_geotiff_f32(&r).expect("write");
        let AggregatedTile(agg) = aggregate_geotiff(&bytes, &g).expect("agg");
        let mut acc = WindowAcc::new(
            (
                agg.fp.row0 as usize,
                agg.fp.col0 as usize,
                agg.fp.rows,
                agg.fp.cols,
            ),
            3,
            3,
        );
        acc.add(&agg);
        let slope = finish(&acc, &g, DemLayer::Slope);
        let elev = finish(&acc, &g, DemLayer::Elevation);
        let want = 0.1f64.atan().to_degrees();
        let mut checked = 0;
        for k in 0..slope.data.len() {
            let s = slope.data[k];
            if s.is_nan() {
                continue;
            }
            // f32 storage of heights limits precision to ~1e-3 degrees.
            assert!((f64::from(s) - want).abs() < 2e-3, "slope {s} vs {want}");
            checked += 1;
        }
        assert!(checked >= 20 * 20, "{checked}");
        // Brute-force mean elevation of one interior cell.
        let (row, col) = (5usize, 7usize);
        let k = row * agg.fp.cols + col;
        let (mut s, mut cnt) = (0.0f64, 0);
        for j in 0..n {
            for i in 0..n {
                let (lng, lat) = (5.0 + (i as f64 + 0.5) * cell, lat_c(j));
                if chelsa_row(&g, lat) == agg.fp.row0 + row as i64
                    && chelsa_col(&g, lng) == agg.fp.col0 + col as i64
                {
                    s += f64::from(r.data[j * n + i]);
                    cnt += 1;
                }
            }
        }
        assert_eq!(cnt, 100);
        assert!((f64::from(elev.data[k]) - s / 100.0).abs() < 1e-3);
        let rough = finish(&acc, &g, DemLayer::Roughness);
        // Within a cell the plane spans 9 pixel rows of ~92.6 m at 0.1: SD of a uniform
        // 10-level ramp = 92.6·0.1·sqrt((10²−1)/12) ≈ 26.6 m.
        assert!(
            (f64::from(rough.data[k]) - 26.6).abs() < 0.5,
            "{}",
            rough.data[k]
        );
    }

    #[test]
    fn horn_matches_textbook_formula_inside() {
        // 3×3 example: Horn's dz/dx = ((c + 2f + i) − (a + 2d + g)) / (8 Δx).
        let z: [f32; 9] = [50.0, 45.0, 50.0, 30.0, 30.0, 30.0, 8.0, 10.0, 10.0];
        let dx = [10.0; 3];
        let s = horn_slope_deg(&z, 3, 3, 1, 1, &dx, 10.0).expect("slope");
        let (a, b, c, d, f, g, h, i) = (50.0, 45.0, 50.0, 30.0, 30.0, 8.0, 10.0, 10.0);
        let gx: f64 = ((c + 2.0 * f + i) - (a + 2.0 * d + g)) / 80.0;
        let gy: f64 = ((g + 2.0 * h + i) - (a + 2.0 * b + c)) / 80.0;
        let want = (gx * gx + gy * gy).sqrt().atan().to_degrees();
        assert!((s - want).abs() < 1e-12, "{s} vs {want}");
    }

    #[test]
    fn sea_majority_cells_are_nodata() {
        let g = chelsa_like();
        let mut acc = WindowAcc::new((4800, 22200, 2, 2), 3, 3);
        let k = 0;
        acc.agg.counts[C_TOTAL][k] = 100;
        acc.agg.counts[C_VALID][k] = 49;
        acc.agg.counts[C_TOTAL][1] = 100;
        acc.agg.counts[C_VALID][1] = 50;
        acc.agg.counts[C_SLOPE][1] = 50;
        acc.agg.sums[S_Z][1] = 500.0;
        let e = finish(&acc, &g, DemLayer::Elevation);
        assert!(e.data[0].is_nan());
        assert_eq!(e.data[1], 10.0);
        assert!(e.data[2].is_nan());
    }
}
