//! ESA WorldCover 2021 v200 (10 m) as fractional cover on the native CHELSA 30″ grid.
//!
//! **Source.** Public COGs on AWS (`s3://esa-worldcover/v200/2021/map/`, no authentication),
//! 3°×3° tiles of 36 000 × 36 000 pixels (1/12 000° ≈ 9.3 m), EPSG:4326, `RasterPixelIsArea`,
//! uint8 class codes with a palette, Deflate, 1024×1024 internal tiles, NoData 0. Tiles that are
//! entirely sea do not exist; `esa_worldcover_grid.geojson` in the bucket lists the 2 651 existing
//! tiles (property `ll_tile`) and is cached instead of probing 404s.
//!
//! **Method.** For every CHELSA cell, the fraction of each class group among the *valid* 10 m
//! pixels whose centre falls in the cell (≈ 100 × 60–100 pixels per cell in Europe). NoData
//! pixels are excluded from the denominator; the cell is NoData when fewer than half of its
//! pixel centres are valid (also over absent sea tiles). Groups:
//!
//! | group | WorldCover classes |
//! |---|---|
//! | tree | 10 Tree cover |
//! | shrub | 20 Shrubland |
//! | grass | 30 Grassland |
//! | crop | 40 Cropland |
//! | built | 50 Built-up |
//! | bare | 60 Bare / sparse vegetation, 70 Snow and ice, 100 Moss and lichen |
//! | water | 80 Permanent water bodies |
//! | wetland | 90 Herbaceous wetland, 95 Mangroves |
//!
//! The groups partition the valid pixels, so the eight fractions sum to 1 in every valid cell.
//!
//! **Why full resolution.** The COGs carry six internal overviews (18 000² … 562²), but the
//! overview of a class map is a *resampled class map* (nearest / mode sampling), not a fraction:
//! a 1 km cell of an overview holds one class code, so fractions computed from it would be 0 or
//! 1, and small or linear features (rivers, hedgerows, villages) would vanish or be exaggerated
//! depending on the sampling phase. Exact fractions need every 10 m pixel. They are read
//! internal tile by internal tile with HTTP range requests (one request per row of internal
//! tiles), aggregated on the fly — never holding more than one batch of compressed tiles and
//! one decoded 1024² tile — and cached per 1°×1° block, so each degree is computed once per
//! instance.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use niche_geo::geotiff::{ByteSource, Chunk, GeoTiffInfo};
use niche_geo::{GridSpec, Raster};

use crate::DataError;
use crate::cellagg::{Agg, Footprint, WindowAcc, chelsa_col, chelsa_row, parallel_map};
use crate::http::{Http, HttpSource};

pub const SOURCE: &str = "ESA WorldCover 10 m 2021 v200";
pub const LICENCE: &str = "CC BY 4.0";
pub const LICENCE_URL: &str = "https://creativecommons.org/licenses/by/4.0/";
/// Attribution prescribed on https://esa-worldcover.org/en/data-access (checked 2026-10-20).
pub const ATTRIBUTION: &str = "© ESA WorldCover project 2021 / Contains modified Copernicus Sentinel data (2021) processed by ESA WorldCover consortium";
pub const CITATION: &str = "Zanaga D., Van De Kerchove R., Daems D., De Keersmaecker W., Brockmann C., Kirches G., Wevers J., Cartus O., Santoro M., Fritz S., Lesiv M., Herold M., Tsendbazar N.E., Xu P., Ramoino F., Arino O. (2022). ESA WorldCover 10 m 2021 v200. doi:10.5281/zenodo.7254221";

const BASE: &str = "https://esa-worldcover.s3.eu-central-1.amazonaws.com/v200/2021";
const AGG_VERSION: u32 = 1;
const MAGIC: &[u8; 4] = b"NLCV";
/// Pixels per degree of the 10 m map.
const PER_DEGREE: usize = 12_000;

/// Class groups, in the order of the cached count bands.
pub const GROUPS: [&str; 8] = [
    "tree", "shrub", "grass", "crop", "built", "bare", "water", "wetland",
];
const C_TOTAL: usize = 8;
const C_VALID: usize = 9;
const NCOUNTS: usize = 10;

/// Group index of a WorldCover class code (`None`: NoData or unknown code).
pub fn group_of(code: u8) -> Option<usize> {
    Some(match code {
        10 => 0,
        20 => 1,
        30 => 2,
        40 => 3,
        50 => 4,
        60 | 70 | 100 => 5,
        80 => 6,
        90 | 95 => 7,
        _ => return None,
    })
}

/// Group index by name (`"water"` → 6).
pub fn group_index(name: &str) -> Option<usize> {
    GROUPS.iter().position(|g| *g == name)
}

/// `ll_tile` name of the 3° tile containing the degree block (lat0, lon0).
pub fn tile_ll(lat0: i32, lon0: i32) -> (i32, i32, String) {
    let la = lat0.div_euclid(3) * 3;
    let lo = lon0.div_euclid(3) * 3;
    let ns = if la >= 0 { 'N' } else { 'S' };
    let ew = if lo >= 0 { 'E' } else { 'W' };
    (la, lo, format!("{ns}{:02}{ew}{:03}", la.abs(), lo.abs()))
}

pub fn tile_url(ll: &str) -> String {
    format!("{BASE}/map/ESA_WorldCover_10m_2021_v200_{ll}_Map.tif")
}

/// Existing 3° tiles (`ll_tile` names), from the cached grid GeoJSON.
pub fn tile_list(http: &Http, cache: &Path) -> Result<HashSet<String>, DataError> {
    let f = cache.join("worldcover").join("v200-2021-grid.geojson");
    let url = format!("{BASE}/esa_worldcover_grid.geojson");
    if !f.exists() {
        if let Some(d) = f.parent() {
            std::fs::create_dir_all(d).map_err(|e| DataError::Cache(e.to_string()))?;
        }
        http.download_to_file(&url, &f, 16 << 20)?;
    }
    let text = std::fs::read_to_string(&f).map_err(|e| DataError::Cache(e.to_string()))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| DataError::Format {
        url: url.clone(),
        msg: e.to_string(),
    })?;
    let set: HashSet<String> = v["features"]
        .as_array()
        .map(|fs| {
            fs.iter()
                .filter_map(|f| f["properties"]["ll_tile"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if set.len() < 1000 {
        let _ = std::fs::remove_file(&f);
        return Err(DataError::Format {
            url,
            msg: format!("only {} tiles listed", set.len()),
        });
    }
    Ok(set)
}

/// Centres of the 10 m pixels of degree block (lat0, lon0) span
/// [lat0 + h, lat0 + 1 − h] × [lon0 + h, lon0 + 1 − h], h = half a pixel.
fn block_footprint(grid: &GridSpec, lat0: i32, lon0: i32) -> Footprint {
    let h = 0.5 / PER_DEGREE as f64;
    let (la, lo) = (f64::from(lat0), f64::from(lon0));
    Footprint::of_centres(grid, la + h, la + 1.0 - h, lo + h, lo + 1.0 - h)
}

/// Block over sea (no tile): pixel centres counted, none valid.
fn absent_block(grid: &GridSpec, lat0: i32, lon0: i32) -> Agg {
    let fp = block_footprint(grid, lat0, lon0);
    let mut agg = Agg::new(fp, NCOUNTS, 0);
    let px = 1.0 / PER_DEGREE as f64;
    let mut rows = vec![0u32; fp.rows];
    let mut cols = vec![0u32; fp.cols];
    for j in 0..PER_DEGREE {
        let lat = f64::from(lat0) + 1.0 - (j as f64 + 0.5) * px;
        if let Some(k) = fp.index(chelsa_row(grid, lat), fp.col0) {
            rows[k / fp.cols] += 1;
        }
        let lng = f64::from(lon0) + (j as f64 + 0.5) * px;
        if let Some(k) = fp.index(fp.row0, chelsa_col(grid, lng)) {
            cols[k] += 1;
        }
    }
    for (r, nr) in rows.iter().enumerate() {
        for (c, nc) in cols.iter().enumerate() {
            agg.counts[C_TOTAL][r * fp.cols + c] = nr * nc;
        }
    }
    agg
}

/// Statistics of one streamed block (for reporting).
#[derive(Debug, Clone, Copy, Default)]
pub struct BlockStats {
    pub bytes_read: u64,
    pub requests: u64,
    pub elapsed: std::time::Duration,
}

/// Counting wrapper around a byte source.
struct Counted<'a> {
    inner: &'a dyn ByteSource,
    bytes: std::sync::atomic::AtomicU64,
    requests: std::sync::atomic::AtomicU64,
}

impl ByteSource for Counted<'_> {
    fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, niche_geo::geotiff::TiffError> {
        use std::sync::atomic::Ordering::Relaxed;
        self.bytes.fetch_add(len as u64, Relaxed);
        self.requests.fetch_add(1, Relaxed);
        self.inner.read_at(offset, len)
    }
}

/// Opaque per-degree aggregate (class-group counts on the CHELSA lattice).
#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedBlock(pub(crate) Agg);

/// Stream-aggregate the degree block (lat0, lon0) of a WorldCover-like GeoTIFF onto the CHELSA
/// grid. The image must be EPSG:4326 with a pixel size dividing 1° evenly and cover the block.
pub fn aggregate_block(
    src: &dyn ByteSource,
    info: &GeoTiffInfo,
    lat0: i32,
    lon0: i32,
    grid: &GridSpec,
) -> Result<(AggregatedBlock, BlockStats), DataError> {
    let t = std::time::Instant::now();
    let fmt = |msg: String| DataError::Format {
        url: "worldcover".into(),
        msg,
    };
    let (px, py) = (info.pixel_x, info.pixel_y);
    let n = (1.0 / py).round() as usize;
    if (n as f64 * py - 1.0).abs() > 1e-9 || (px - py).abs() > 1e-12 {
        return Err(fmt(format!("pixel size {px}×{py} does not divide 1°")));
    }
    let c0 = (f64::from(lon0) - info.spec.west) / px;
    let r0 = (info.spec.north - (f64::from(lat0) + 1.0)) / py;
    if (c0 - c0.round()).abs() > 1e-6 || (r0 - r0.round()).abs() > 1e-6 || c0 < -0.5 || r0 < -0.5 {
        return Err(fmt(format!(
            "block ({lat0}, {lon0}) not aligned with the image"
        )));
    }
    let (c0, r0) = (c0.round() as usize, r0.round() as usize);
    if c0 + n > info.width || r0 + n > info.height {
        return Err(fmt(format!("block ({lat0}, {lon0}) outside the image")));
    }
    let fp = Footprint::of_centres(
        grid,
        f64::from(lat0) + py / 2.0,
        f64::from(lat0) + 1.0 - py / 2.0,
        f64::from(lon0) + px / 2.0,
        f64::from(lon0) + 1.0 - px / 2.0,
    );
    // Local CHELSA row/col of every image row/col of the block.
    let row_map: Vec<usize> = (0..n)
        .map(|j| {
            let lat = info.spec.north - ((r0 + j) as f64 + 0.5) * py;
            (chelsa_row(grid, lat) - fp.row0) as usize
        })
        .collect();
    let col_map: Vec<usize> = (0..n)
        .map(|i| {
            let lng = info.spec.west + ((c0 + i) as f64 + 0.5) * px;
            (chelsa_col(grid, lng) - fp.col0) as usize
        })
        .collect();
    let mut lut = [u8::MAX; 256];
    for (code, slot) in lut.iter_mut().enumerate() {
        if let Some(g) = group_of(code as u8) {
            *slot = g as u8;
        }
    }
    let mut agg = Agg::new(fp, NCOUNTS, 0);
    let counted = Counted {
        inner: src,
        bytes: 0.into(),
        requests: 0.into(),
    };
    info.visit_window(&counted, r0, c0, n, n, &mut |ch: &Chunk| {
        let rr0 = ch.row0.max(r0);
        let rr1 = (ch.row0 + ch.height).min(r0 + n);
        let cc0 = ch.col0.max(c0);
        let cc1 = (ch.col0 + ch.width).min(c0 + n);
        for r in rr0..rr1 {
            let base = row_map[r - r0] * fp.cols;
            let vals = &ch.values[(r - ch.row0) * ch.stride..];
            for c in cc0..cc1 {
                let k = base + col_map[c - c0];
                agg.counts[C_TOTAL][k] += 1;
                let v = vals[c - ch.col0];
                if v.is_finite() && (0.0..=255.0).contains(&v) {
                    let g = lut[v as usize];
                    if g != u8::MAX {
                        agg.counts[C_VALID][k] += 1;
                        agg.counts[g as usize][k] += 1;
                    }
                }
            }
        }
        Ok(())
    })?;
    use std::sync::atomic::Ordering::Relaxed;
    let stats = BlockStats {
        bytes_read: counted.bytes.load(Relaxed),
        requests: counted.requests.load(Relaxed),
        elapsed: t.elapsed(),
    };
    Ok((AggregatedBlock(agg), stats))
}

fn agg_path(cache: &Path, lat0: i32, lon0: i32) -> PathBuf {
    let ns = if lat0 >= 0 { 'N' } else { 'S' };
    let ew = if lon0 >= 0 { 'E' } else { 'W' };
    cache
        .join("worldcover")
        .join(format!("agg-v{AGG_VERSION}"))
        .join(format!("{ns}{:02}{ew}{:03}.bin", lat0.abs(), lon0.abs()))
}

/// Aggregate of one degree block: cached, else streamed from the remote COG and cached.
/// Returns the statistics of the download when it was computed.
pub fn block(
    http: &Http,
    cache: &Path,
    grid: &GridSpec,
    list: &HashSet<String>,
    lat0: i32,
    lon0: i32,
) -> Result<(AggregatedBlock, Option<BlockStats>), DataError> {
    let (_, _, ll) = tile_ll(lat0, lon0);
    if !list.contains(&ll) {
        return Ok((AggregatedBlock(absent_block(grid, lat0, lon0)), None));
    }
    let path = agg_path(cache, lat0, lon0);
    if let Some(a) = Agg::load(&path, MAGIC, AGG_VERSION, grid) {
        return Ok((AggregatedBlock(a), None));
    }
    let src = HttpSource::new(http.clone(), tile_url(&ll));
    let info = GeoTiffInfo::read(&src)?;
    let (b, stats) = aggregate_block(&src, &info, lat0, lon0, grid)?;
    tracing::debug!(block = %path.display(), ?stats, "WorldCover block aggregated");
    b.0.save(&path, MAGIC, AGG_VERSION, grid)?;
    Ok((b, Some(stats)))
}

pub type Window = (usize, usize, usize, usize);

/// Degree blocks whose pixel centres fall in the window.
fn blocks_for(grid: &GridSpec, window: Window) -> Vec<(i32, i32)> {
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
    for lat0 in (south.floor() as i32)..=(north.ceil() as i32 - 1) {
        if !(-90..90).contains(&lat0) {
            continue;
        }
        for lon0 in (west.floor() as i32)..=(east.ceil() as i32 - 1) {
            let lon0 = (lon0 + 180).rem_euclid(360) - 180;
            if block_footprint(grid, lat0, lon0).intersects(&wfp) && !out.contains(&(lat0, lon0)) {
                out.push((lat0, lon0));
            }
        }
    }
    out
}

/// Fractions (0–1) of every group on a native CHELSA window, in [`GROUPS`] order.
pub fn read_fractions(
    http: &Http,
    cache: &Path,
    grid: &GridSpec,
    window: Window,
) -> Result<Vec<Raster>, DataError> {
    let list = tile_list(http, cache)?;
    let blocks = blocks_for(grid, window);
    let aggs = parallel_map(&blocks, 4, |&(la, lo)| {
        block(http, cache, grid, &list, la, lo)
    });
    let mut acc = WindowAcc::new(window, NCOUNTS, 0);
    for a in aggs {
        acc.add(&a?.0.0);
    }
    Ok(fractions(&acc, grid))
}

/// Fraction of one group (`"water"`, …) on a native CHELSA window.
pub fn read_window(
    http: &Http,
    cache: &Path,
    grid: &GridSpec,
    window: Window,
    group: &str,
) -> Result<Raster, DataError> {
    let g = group_index(group).ok_or_else(|| DataError::Format {
        url: group.into(),
        msg: "unknown land-cover group".into(),
    })?;
    let mut all = read_fractions(http, cache, grid, window)?;
    Ok(all.swap_remove(g))
}

fn fractions(acc: &WindowAcc, grid: &GridSpec) -> Vec<Raster> {
    let fp = acc.footprint();
    let spec = GridSpec {
        west: grid.west + fp.col0 as f64 * grid.cell,
        north: grid.north - fp.row0 as f64 * grid.cell,
        cell: grid.cell,
        width: fp.cols,
        height: fp.rows,
    };
    let a = &acc.agg;
    (0..GROUPS.len())
        .map(|g| {
            let data = (0..fp.len())
                .map(|k| {
                    let (total, valid) = (a.counts[C_TOTAL][k], a.counts[C_VALID][k]);
                    if total == 0 || 2 * valid < total {
                        f32::NAN
                    } else {
                        (f64::from(a.counts[g][k]) / f64::from(valid)) as f32
                    }
                })
                .collect();
            Raster { spec, data }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use niche_geo::write::write_geotiff_f32;

    fn chelsa_like() -> GridSpec {
        GridSpec {
            west: -180.000_138_889,
            north: 83.999_860_415,
            cell: 1.0 / 120.0,
            width: 43_200,
            height: 20_880,
        }
    }

    #[test]
    fn names() {
        assert_eq!(tile_ll(43, 5), (42, 3, "N42E003".into()));
        assert_eq!(tile_ll(-1, -1), (-3, -3, "S03W003".into()));
        assert!(tile_url("N42E003").ends_with("ESA_WorldCover_10m_2021_v200_N42E003_Map.tif"));
        assert_eq!(group_of(100), Some(5));
        assert_eq!(group_of(95), Some(7));
        assert_eq!(group_of(0), None);
        assert_eq!(group_index("water"), Some(6));
    }

    /// A synthetic 1° "WorldCover" at a coarser 1/1200° pixel (to keep the test light), written
    /// with the project's GeoTIFF writer: classes follow a deterministic pattern with NoData
    /// patches. The streamed aggregate equals a brute-force count over pixel centres, fractions
    /// sum to 1, and NoData-majority cells are NaN.
    #[test]
    fn streamed_fractions_match_brute_force() {
        let g = chelsa_like();
        let n = 1200;
        let cell = 1.0 / n as f64;
        let spec = GridSpec {
            west: 3.0,
            north: 45.0,
            cell,
            width: n,
            height: 2 * n,
        };
        let codes = [10u8, 20, 30, 40, 50, 60, 70, 80, 90, 95, 100];
        let mut r = Raster::new(spec, 0.0);
        for j in 0..2 * n {
            for i in 0..n {
                let v = if (i / 37 + j / 53) % 9 == 0 {
                    f32::NAN // NoData patches
                } else {
                    f32::from(codes[(i * 7 + j * 3 + (i * j) % 5) % codes.len()])
                };
                r.data[j * n + i] = v;
            }
        }
        let bytes = write_geotiff_f32(&r).expect("write");
        let info = GeoTiffInfo::read(&bytes).expect("info");
        // Block (43, 3) is the southern degree of the 2° image.
        let (AggregatedBlock(agg), stats) = aggregate_block(&bytes, &info, 43, 3, &g).expect("agg");
        assert!(stats.bytes_read > 0);
        let mut brute = Agg::new(agg.fp, NCOUNTS, 0);
        for j in n..2 * n {
            for i in 0..n {
                let lat = 45.0 - (j as f64 + 0.5) * cell;
                let lng = 3.0 + (i as f64 + 0.5) * cell;
                let k = agg
                    .fp
                    .index(chelsa_row(&g, lat), chelsa_col(&g, lng))
                    .expect("in");
                brute.counts[C_TOTAL][k] += 1;
                let v = r.data[j * n + i];
                if let Some(gr) = (!v.is_nan()).then(|| group_of(v as u8)).flatten() {
                    brute.counts[C_VALID][k] += 1;
                    brute.counts[gr][k] += 1;
                }
            }
        }
        assert_eq!(agg, brute);
        let total: u32 = agg.counts[C_TOTAL].iter().sum();
        assert_eq!(total as usize, n * n);
        let mut acc = WindowAcc::new(
            (agg.fp.row0 as usize + 1, agg.fp.col0 as usize + 1, 119, 119),
            NCOUNTS,
            0,
        );
        acc.add(&agg);
        let fr = fractions(&acc, &g);
        let mut nan = 0;
        for k in 0..119 * 119 {
            let s: f32 = fr.iter().map(|f| f.data[k]).sum();
            if s.is_nan() {
                nan += 1;
                let (t, v) = (acc.agg.counts[C_TOTAL][k], acc.agg.counts[C_VALID][k]);
                assert!(2 * v < t);
            } else {
                assert!((s - 1.0).abs() < 1e-5, "{s}");
            }
        }
        assert!(nan > 0, "some cells are NoData-majority");
    }

    #[test]
    fn absent_block_counts_every_pixel_once() {
        let g = chelsa_like();
        let a = absent_block(&g, 43, 5);
        let total: u64 = a.counts[C_TOTAL].iter().map(|&v| u64::from(v)).sum();
        assert_eq!(total, (PER_DEGREE * PER_DEGREE) as u64);
        assert_eq!((a.fp.rows, a.fp.cols), (121, 121));
        assert!(a.counts[C_VALID].iter().all(|&v| v == 0));
    }

    #[test]
    fn blocks_for_window() {
        let g = chelsa_like();
        // A 1° CHELSA-aligned block starting just west/north of 5°E 44°N touches 4 degrees
        // (0.5″ slivers of the neighbours).
        let r0 = chelsa_row(&g, 44.0 - 1e-6) as usize;
        let c0 = chelsa_col(&g, 5.0 + 1e-6) as usize;
        let b = blocks_for(&g, (r0, c0, 120, 120));
        assert!(b.contains(&(43, 5)));
        assert!(b.len() <= 4, "{b:?}");
        let b1 = blocks_for(&g, (r0 + 1, c0 + 1, 100, 100));
        assert_eq!(b1, vec![(43, 5)]);
    }
}
