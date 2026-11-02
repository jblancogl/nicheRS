//! Exact aggregation of fine source pixels onto the native CHELSA 30″ lattice.
//!
//! A source pixel belongs to the CHELSA cell that contains its **centre**. The CHELSA lattice is
//! offset by 1/7200° (0.5″) from round degrees (west −180.000138889°, north 83.999860415°), so
//! no 3″ DEM or 0.3″ WorldCover pixel centre ever falls on a CHELSA cell edge: the assignment is
//! unambiguous and every pixel is counted exactly once.
//!
//! Each 1°×1° source block yields an [`Agg`] — per-cell counts and sums over the CHELSA cells its
//! pixel centres fall in (up to 121×121 cells because the lattices are offset). Because counts
//! and sums are additive, blocks combine exactly into any window ([`WindowAcc`]), and each block
//! is computed once and cached on disk (zlib-compressed, versioned, tied to the CHELSA grid).

use std::io::{Read, Write};
use std::path::Path;

use niche_geo::GridSpec;

use crate::DataError;

/// CHELSA row containing latitude `lat` (may be outside the grid).
pub(crate) fn chelsa_row(grid: &GridSpec, lat: f64) -> i64 {
    ((grid.north - lat) / grid.cell).floor() as i64
}

/// CHELSA column containing longitude `lng` (may be outside the grid).
pub(crate) fn chelsa_col(grid: &GridSpec, lng: f64) -> i64 {
    ((lng - grid.west) / grid.cell).floor() as i64
}

/// A rectangle of CHELSA cells (global row/column indices).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Footprint {
    pub row0: i64,
    pub col0: i64,
    pub rows: usize,
    pub cols: usize,
}

impl Footprint {
    /// Cells covered by pixel centres spanning [lat_min, lat_max] × [lng_min, lng_max].
    pub fn of_centres(
        grid: &GridSpec,
        lat_min: f64,
        lat_max: f64,
        lng_min: f64,
        lng_max: f64,
    ) -> Self {
        let (r0, r1) = (chelsa_row(grid, lat_max), chelsa_row(grid, lat_min));
        let (c0, c1) = (chelsa_col(grid, lng_min), chelsa_col(grid, lng_max));
        Footprint {
            row0: r0,
            col0: c0,
            rows: (r1 - r0 + 1).max(0) as usize,
            cols: (c1 - c0 + 1).max(0) as usize,
        }
    }

    pub fn intersects(&self, o: &Footprint) -> bool {
        self.row0 < o.row0 + o.rows as i64
            && o.row0 < self.row0 + self.rows as i64
            && self.col0 < o.col0 + o.cols as i64
            && o.col0 < self.col0 + self.cols as i64
    }

    pub fn len(&self) -> usize {
        self.rows * self.cols
    }

    /// Local index of global cell (row, col), if inside.
    pub fn index(&self, row: i64, col: i64) -> Option<usize> {
        let (r, c) = (row - self.row0, col - self.col0);
        if r < 0 || c < 0 || r >= self.rows as i64 || c >= self.cols as i64 {
            return None;
        }
        Some(r as usize * self.cols + c as usize)
    }
}

/// Per-cell counts and sums of one source block.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Agg {
    pub fp: Footprint,
    pub counts: Vec<Vec<u32>>,
    pub sums: Vec<Vec<f64>>,
}

impl Agg {
    pub fn new(fp: Footprint, ncounts: usize, nsums: usize) -> Self {
        Agg {
            fp,
            counts: vec![vec![0; fp.len()]; ncounts],
            sums: vec![vec![0.0; fp.len()]; nsums],
        }
    }

    /// Serialise (zlib) with a magic, a format version and the CHELSA grid it refers to.
    pub fn save(
        &self,
        path: &Path,
        magic: &[u8; 4],
        version: u32,
        grid: &GridSpec,
    ) -> Result<(), DataError> {
        let mut raw = Vec::with_capacity(64 + self.fp.len() * 16);
        raw.extend_from_slice(magic);
        raw.extend_from_slice(&version.to_le_bytes());
        for v in [grid.west, grid.north, grid.cell] {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        raw.extend_from_slice(&self.fp.row0.to_le_bytes());
        raw.extend_from_slice(&self.fp.col0.to_le_bytes());
        raw.extend_from_slice(&(self.fp.rows as u32).to_le_bytes());
        raw.extend_from_slice(&(self.fp.cols as u32).to_le_bytes());
        raw.extend_from_slice(&(self.counts.len() as u32).to_le_bytes());
        raw.extend_from_slice(&(self.sums.len() as u32).to_le_bytes());
        for c in &self.counts {
            c.iter()
                .for_each(|v| raw.extend_from_slice(&v.to_le_bytes()));
        }
        for s in &self.sums {
            s.iter()
                .for_each(|v| raw.extend_from_slice(&v.to_le_bytes()));
        }
        let cache = |e: std::io::Error| DataError::Cache(format!("{}: {e}", path.display()));
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(cache)?;
        }
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
        enc.write_all(&raw).map_err(cache)?;
        let bytes = enc.finish().map_err(cache)?;
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        std::fs::write(&tmp, bytes)
            .and_then(|()| std::fs::rename(&tmp, path))
            .map_err(cache)
    }

    /// Load a cached block; `None` if absent, corrupt, of another version or another grid.
    pub fn load(path: &Path, magic: &[u8; 4], version: u32, grid: &GridSpec) -> Option<Agg> {
        let bytes = std::fs::read(path).ok()?;
        let mut raw = Vec::new();
        flate2::read::ZlibDecoder::new(&bytes[..])
            .take(256 << 20)
            .read_to_end(&mut raw)
            .ok()?;
        let mut p = 0usize;
        let mut take = |n: usize| -> Option<&[u8]> {
            let s = raw.get(p..p + n)?;
            p += n;
            Some(s)
        };
        if take(4)? != magic {
            return None;
        }
        let u32_at = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        let f64_at = |b: &[u8]| {
            let mut a = [0u8; 8];
            a.copy_from_slice(b);
            f64::from_le_bytes(a)
        };
        if u32_at(take(4)?) != version {
            return None;
        }
        for v in [grid.west, grid.north, grid.cell] {
            if f64_at(take(8)?).to_bits() != v.to_bits() {
                return None;
            }
        }
        let row0 = i64::from_le_bytes(take(8)?.try_into().ok()?);
        let col0 = i64::from_le_bytes(take(8)?.try_into().ok()?);
        let rows = u32_at(take(4)?) as usize;
        let cols = u32_at(take(4)?) as usize;
        let nc = u32_at(take(4)?) as usize;
        let ns = u32_at(take(4)?) as usize;
        if rows * cols > 1 << 20 || nc > 64 || ns > 64 {
            return None;
        }
        let fp = Footprint {
            row0,
            col0,
            rows,
            cols,
        };
        let n = fp.len();
        let mut agg = Agg::new(fp, nc, ns);
        for c in agg.counts.iter_mut() {
            let b = take(n * 4)?;
            for (i, v) in c.iter_mut().enumerate() {
                *v = u32_at(&b[i * 4..]);
            }
        }
        for s in agg.sums.iter_mut() {
            let b = take(n * 8)?;
            for (i, v) in s.iter_mut().enumerate() {
                *v = f64_at(&b[i * 8..i * 8 + 8]);
            }
        }
        Some(agg)
    }
}

/// Accumulator over a CHELSA window (row0, col0, rows, cols).
pub(crate) struct WindowAcc {
    pub agg: Agg,
}

impl WindowAcc {
    pub fn new(window: (usize, usize, usize, usize), ncounts: usize, nsums: usize) -> Self {
        let (r0, c0, rows, cols) = window;
        WindowAcc {
            agg: Agg::new(
                Footprint {
                    row0: r0 as i64,
                    col0: c0 as i64,
                    rows,
                    cols,
                },
                ncounts,
                nsums,
            ),
        }
    }

    pub fn footprint(&self) -> Footprint {
        self.agg.fp
    }

    /// Add the overlapping part of a block.
    pub fn add(&mut self, b: &Agg) {
        let w = self.agg.fp;
        let r_lo = w.row0.max(b.fp.row0);
        let r_hi = (w.row0 + w.rows as i64).min(b.fp.row0 + b.fp.rows as i64);
        let c_lo = w.col0.max(b.fp.col0);
        let c_hi = (w.col0 + w.cols as i64).min(b.fp.col0 + b.fp.cols as i64);
        for r in r_lo..r_hi {
            for c in c_lo..c_hi {
                let (Some(i), Some(j)) = (w.index(r, c), b.fp.index(r, c)) else {
                    continue;
                };
                for (dst, src) in self.agg.counts.iter_mut().zip(&b.counts) {
                    dst[i] += src[j];
                }
                for (dst, src) in self.agg.sums.iter_mut().zip(&b.sums) {
                    dst[i] += src[j];
                }
            }
        }
    }
}

/// Run `f` on each item with at most `threads` items in flight (std threads, no async runtime;
/// network-bound block downloads overlap). Results keep the input order.
pub(crate) fn parallel_map<T: Sync, R: Send>(
    items: &[T],
    threads: usize,
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    let mut out = Vec::with_capacity(items.len());
    for batch in items.chunks(threads.max(1)) {
        let res: Vec<R> = std::thread::scope(|s| {
            let hs: Vec<_> = batch.iter().map(|it| s.spawn(|| f(it))).collect();
            hs.into_iter()
                .map(|h| h.join())
                .collect::<Result<Vec<R>, _>>()
                .unwrap_or_else(|p| std::panic::resume_unwind(p))
        });
        out.extend(res);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_roundtrip_and_grid_check() {
        let grid = GridSpec {
            west: -180.000_138_889,
            north: 83.999_860_415,
            cell: 1.0 / 120.0,
            width: 43_200,
            height: 20_880,
        };
        let fp = Footprint {
            row0: 4800,
            col0: 22_200,
            rows: 3,
            cols: 4,
        };
        let mut a = Agg::new(fp, 2, 1);
        a.counts[1][5] = 7;
        a.sums[0][11] = -3.5;
        let dir = std::env::temp_dir().join(format!("niche-cellagg-{}", std::process::id()));
        let f = dir.join("x.bin");
        a.save(&f, b"TEST", 1, &grid).expect("save");
        assert_eq!(Agg::load(&f, b"TEST", 1, &grid), Some(a));
        assert_eq!(Agg::load(&f, b"TEST", 2, &grid), None);
        let other = GridSpec {
            west: -180.0,
            ..grid
        };
        assert_eq!(Agg::load(&f, b"TEST", 1, &other), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn window_accumulates_overlaps() {
        let mut w = WindowAcc::new((10, 10, 2, 2), 1, 0);
        let mut b = Agg::new(
            Footprint {
                row0: 9,
                col0: 11,
                rows: 2,
                cols: 2,
            },
            1,
            0,
        );
        b.counts[0] = vec![1, 2, 3, 4];
        w.add(&b);
        // Block cells (9,11) (9,12) (10,11) (10,12): only (10,11) overlaps → window index 1.
        assert_eq!(w.agg.counts[0], vec![0, 3, 0, 0]);
    }
}
