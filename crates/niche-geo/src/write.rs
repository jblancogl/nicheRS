//! Float32 GeoTIFF writer (EPSG:4326), tiled 256×256, Deflate, IFD before data.
//!
//! The layout (header → IFD → tag data → tiles, tiles in row-major order) follows the Cloud
//! Optimized GeoTIFF requirements except overviews, which are omitted; outputs open in QGIS/GDAL
//! and stream efficiently. NoData is NaN (GDAL_NODATA = "nan").

use std::io::Write;

use crate::grid::Raster;

const TILE: usize = 256;

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("compression failed: {0}")]
    Compress(String),
    #[error("raster too large for classic TIFF")]
    TooLarge,
}

struct Tag {
    tag: u16,
    typ: u16,
    count: u32,
    data: Vec<u8>,
}

fn short(tag: u16, v: u16) -> Tag {
    Tag {
        tag,
        typ: 3,
        count: 1,
        data: v.to_le_bytes().to_vec(),
    }
}
fn long(tag: u16, v: u32) -> Tag {
    Tag {
        tag,
        typ: 4,
        count: 1,
        data: v.to_le_bytes().to_vec(),
    }
}
fn longs(tag: u16, v: &[u32]) -> Tag {
    Tag {
        tag,
        typ: 4,
        count: v.len() as u32,
        data: v.iter().flat_map(|x| x.to_le_bytes()).collect(),
    }
}
fn doubles(tag: u16, v: &[f64]) -> Tag {
    Tag {
        tag,
        typ: 12,
        count: v.len() as u32,
        data: v.iter().flat_map(|x| x.to_le_bytes()).collect(),
    }
}
fn shorts(tag: u16, v: &[u16]) -> Tag {
    Tag {
        tag,
        typ: 3,
        count: v.len() as u32,
        data: v.iter().flat_map(|x| x.to_le_bytes()).collect(),
    }
}
fn ascii(tag: u16, s: &str) -> Tag {
    let mut d = s.as_bytes().to_vec();
    d.push(0);
    Tag {
        tag,
        typ: 2,
        count: d.len() as u32,
        data: d,
    }
}

/// Encode a raster as a GeoTIFF in memory.
pub fn write_geotiff_f32(r: &Raster) -> Result<Vec<u8>, WriteError> {
    let s = r.spec;
    let tx = s.width.div_ceil(TILE);
    let ty = s.height.div_ceil(TILE);
    let mut tiles: Vec<Vec<u8>> = Vec::with_capacity(tx * ty);
    let mut raw = vec![0u8; TILE * TILE * 4];
    for tr in 0..ty {
        for tc in 0..tx {
            for y in 0..TILE {
                for x in 0..TILE {
                    let (row, col) = (tr * TILE + y, tc * TILE + x);
                    let v = if row < s.height && col < s.width {
                        r.get(row, col)
                    } else {
                        f32::NAN
                    };
                    raw[(y * TILE + x) * 4..(y * TILE + x) * 4 + 4]
                        .copy_from_slice(&v.to_le_bytes());
                }
            }
            let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
            enc.write_all(&raw)
                .map_err(|e| WriteError::Compress(e.to_string()))?;
            tiles.push(
                enc.finish()
                    .map_err(|e| WriteError::Compress(e.to_string()))?,
            );
        }
    }
    let n = tiles.len();
    let mut tags = vec![
        long(256, s.width as u32),
        long(257, s.height as u32),
        short(258, 32),
        short(259, 8),
        short(262, 1),
        short(277, 1),
        short(284, 1),
        short(322, TILE as u16),
        short(323, TILE as u16),
        longs(324, &vec![0; n]),
        longs(
            325,
            &tiles.iter().map(|t| t.len() as u32).collect::<Vec<_>>(),
        ),
        short(339, 3),
        doubles(33550, &[s.cell, s.cell, 0.0]),
        doubles(33922, &[0.0, 0.0, 0.0, s.west, s.north, 0.0]),
        // GeoKeyDirectory: version 1.1.0, 3 keys: geographic model, pixel-is-area, EPSG:4326.
        shorts(
            34735,
            &[1, 1, 0, 3, 1024, 0, 1, 2, 1025, 0, 1, 1, 2048, 0, 1, 4326],
        ),
        ascii(42113, "nan"),
    ];
    tags.sort_by_key(|t| t.tag);
    let ifd_len = 2 + tags.len() * 12 + 4;
    let mut extra_off = 8 + ifd_len;
    // First pass: compute where each out-of-line tag value goes.
    let mut extra_offsets = Vec::new();
    for t in &tags {
        if t.data.len() > 4 {
            extra_offsets.push(Some(extra_off));
            extra_off += t.data.len() + (t.data.len() & 1);
        } else {
            extra_offsets.push(None);
        }
    }
    let data_start = extra_off;
    let mut tile_offsets = Vec::with_capacity(n);
    let mut pos = data_start;
    for t in &tiles {
        tile_offsets.push(u32::try_from(pos).map_err(|_| WriteError::TooLarge)?);
        pos += t.len();
    }
    u32::try_from(pos).map_err(|_| WriteError::TooLarge)?;
    for t in &mut tags {
        if t.tag == 324 {
            t.data = tile_offsets.iter().flat_map(|x| x.to_le_bytes()).collect();
        }
    }
    let mut out = Vec::with_capacity(pos);
    out.extend_from_slice(b"II");
    out.extend_from_slice(&42u16.to_le_bytes());
    out.extend_from_slice(&8u32.to_le_bytes());
    out.extend_from_slice(&(tags.len() as u16).to_le_bytes());
    for (t, eo) in tags.iter().zip(&extra_offsets) {
        out.extend_from_slice(&t.tag.to_le_bytes());
        out.extend_from_slice(&t.typ.to_le_bytes());
        out.extend_from_slice(&t.count.to_le_bytes());
        match eo {
            Some(o) => out.extend_from_slice(&(*o as u32).to_le_bytes()),
            None => {
                let mut v = t.data.clone();
                v.resize(4, 0);
                out.extend_from_slice(&v);
            }
        }
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    for t in &tags {
        if t.data.len() > 4 {
            out.extend_from_slice(&t.data);
            if t.data.len() & 1 == 1 {
                out.push(0);
            }
        }
    }
    debug_assert_eq!(out.len(), data_start);
    for t in &tiles {
        out.extend_from_slice(t);
    }
    Ok(out)
}
