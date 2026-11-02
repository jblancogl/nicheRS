//! XYZ (Web Mercator, 256 px) PNG tile rendering from lon/lat rasters, nearest-neighbour.
//! Rendering is display-only; analysis always uses the native grid values.

use crate::grid::Raster;

pub const TILE: u32 = 256;

/// Longitude/latitude of the centre of pixel (px, py) of tile (z, x, y).
pub fn pixel_lnglat(z: u8, x: u32, y: u32, px: u32, py: u32) -> (f64, f64) {
    let n = f64::from(1u32 << z) * f64::from(TILE);
    let gx = f64::from(x) * f64::from(TILE) + f64::from(px) + 0.5;
    let gy = f64::from(y) * f64::from(TILE) + f64::from(py) + 0.5;
    let lng = gx / n * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * gy / n))
        .sinh()
        .atan()
        .to_degrees();
    (lng, lat)
}

fn edge_lnglat(z: u8, gx: f64, gy: f64) -> (f64, f64) {
    let n = f64::from(1u32 << z);
    let lng = gx / n * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * gy / n))
        .sinh()
        .atan()
        .to_degrees();
    (lng, lat)
}

/// Tile bounds (west, south, east, north).
pub fn tile_bounds(z: u8, x: u32, y: u32) -> (f64, f64, f64, f64) {
    let (w, n) = edge_lnglat(z, f64::from(x), f64::from(y));
    let (e, s) = edge_lnglat(z, f64::from(x) + 1.0, f64::from(y) + 1.0);
    (w, s, e, n)
}

/// Render a tile; `style` maps a finite value to RGBA (or `None` = transparent).
pub fn render<F>(raster: &Raster, z: u8, x: u32, y: u32, style: F) -> Vec<u8>
where
    F: Fn(f32, u32, u32) -> Option<[u8; 4]>,
{
    let mut rgba = vec![0u8; (TILE * TILE * 4) as usize];
    let s = raster.spec;
    let (w, so, e, n) = tile_bounds(z, x, y);
    if e < s.west || w > s.east() || n < s.south() || so > s.north {
        return encode(&rgba);
    }
    for py in 0..TILE {
        for px in 0..TILE {
            let (lng, lat) = pixel_lnglat(z, x, y, px, py);
            if let Some(v) = raster.sample(lng, lat)
                && let Some(c) = style(v, px, py)
            {
                let o = ((py * TILE + px) * 4) as usize;
                rgba[o..o + 4].copy_from_slice(&c);
            }
        }
    }
    encode(&rgba)
}

pub fn encode(rgba: &[u8]) -> Vec<u8> {
    encode_rgba(TILE, TILE, rgba, png::Compression::Fast)
}

/// PNG overview of a whole raster, nearest-neighbour, at most `max_px` on its longest side
/// (thumbnails). Cells for which `style` returns `None` (and NoData) are transparent.
pub fn overview<F>(raster: &Raster, max_px: usize, style: F) -> Vec<u8>
where
    F: Fn(f32) -> Option<[u8; 4]>,
{
    let s = raster.spec;
    let scale = s.width.max(s.height) as f64 / max_px.max(1) as f64;
    let w = ((s.width as f64 / scale).floor() as usize).max(1);
    let h = ((s.height as f64 / scale).floor() as usize).max(1);
    let mut rgba = vec![0u8; w * h * 4];
    for py in 0..h {
        let r = (((py as f64 + 0.5) * scale) as usize).min(s.height - 1);
        for px in 0..w {
            let c = (((px as f64 + 0.5) * scale) as usize).min(s.width - 1);
            let v = raster.get(r, c);
            if v.is_nan() {
                continue;
            }
            if let Some(col) = style(v) {
                let o = (py * w + px) * 4;
                rgba[o..o + 4].copy_from_slice(&col);
            }
        }
    }
    encode_image(w as u32, h as u32, &rgba)
}

/// PNG of a `width` × `height` RGBA image (e.g. a report figure). Empty on invalid input.
pub fn encode_image(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    encode_rgba(width, height, rgba, png::Compression::Balanced)
}

fn encode_rgba(width: u32, height: u32, rgba: &[u8], compression: png::Compression) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(compression);
        // Writing to a Vec cannot fail except on invariant violations; return an empty tile then.
        let Ok(mut w) = enc.write_header() else {
            return Vec::new();
        };
        if w.write_image_data(rgba).is_err() {
            return Vec::new();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_zero_covers_world() {
        let (w, s, e, n) = tile_bounds(0, 0, 0);
        assert!(w < -179.0 && e > 179.0 && n > 85.0 && s < -85.0);
    }
}
