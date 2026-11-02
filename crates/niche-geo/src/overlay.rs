//! Exact ellipsoidal areas of polygons and of their overlap with grid cells.
//!
//! Polygons are GeoJSON-style lon/lat rings whose edges are straight in longitude/latitude
//! (RFC 7946 §3.1.1). Their area on the WGS84 ellipsoid follows from Green's theorem applied to
//! the authalic area element `dA = b²/2 · q′(φ) dλ dφ` used by [`crate::area`]:
//!
//! `A = b²/2 · |∮ q(φ) dλ|`
//!
//! Along a parallel (φ constant) the integral is `q(φ)·Δλ` exactly, along a meridian it is 0, so
//! grid cells and polygons clipped to cells get the same area as [`crate::area::quad_area_km2`].
//! Sloping edges are integrated with 8-point Gauss–Legendre quadrature; `q` is smooth, so for
//! edges of a few km the quadrature error is far below the f64 rounding of the result.
//!
//! The overlap of a polygon with each cell is obtained by clipping every ring to the cell
//! (Sutherland–Hodgman against the cell's four sides). For a simple ring clipped by a convex
//! window the clipped ring may contain zero-width "bridges" along the window border, which add no
//! signed area, so its signed area is the area of the intersection. Holes are subtracted; rings
//! of different polygons are assumed not to overlap (valid MultiPolygon).

use crate::grid::GridSpec;
use crate::mask::{Rings, bbox};

const A: f64 = 6_378_137.0;
const F: f64 = 1.0 / 298.257_223_563;

fn q(phi: f64) -> f64 {
    let e2 = F * (2.0 - F);
    let e = e2.sqrt();
    let s = phi.sin();
    s / (1.0 - e2 * s * s) + 1.0 / (2.0 * e) * ((1.0 + e * s) / (1.0 - e * s)).ln()
}

/// Gauss–Legendre nodes and weights on [−1, 1] (8 points).
const GL8: [(f64, f64); 8] = [
    (-0.960_289_856_497_536_3, 0.101_228_536_290_376_26),
    (-0.796_666_477_413_626_7, 0.222_381_034_453_374_47),
    (-0.525_532_409_916_329, 0.313_706_645_877_887_3),
    (-0.183_434_642_495_649_8, 0.362_683_783_378_362),
    (0.183_434_642_495_649_8, 0.362_683_783_378_362),
    (0.525_532_409_916_329, 0.313_706_645_877_887_3),
    (0.796_666_477_413_626_7, 0.222_381_034_453_374_47),
    (0.960_289_856_497_536_3, 0.101_228_536_290_376_26),
];

/// ∫ q(φ) dλ along the straight lon/lat edge (x1, y1) → (x2, y2), degrees in, radians·q out.
fn edge_integral(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dl = (x2 - x1).to_radians();
    if dl == 0.0 {
        return 0.0;
    }
    if y1 == y2 {
        return q(y1.to_radians()) * dl;
    }
    let (p1, p2) = (y1.to_radians(), y2.to_radians());
    let mut s = 0.0;
    for (t, w) in GL8 {
        let phi = 0.5 * (p1 + p2) + 0.5 * (p2 - p1) * t;
        s += w * q(phi);
    }
    0.5 * s * dl
}

/// Signed ellipsoidal area (km²) of one ring; the sign depends on its orientation.
pub fn ring_signed_area_km2(ring: &[(f64, f64)]) -> f64 {
    let n = ring.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..n {
        let (x1, y1) = ring[i];
        let (x2, y2) = ring[(i + 1) % n];
        s += edge_integral(x1, y1, x2, y2);
    }
    let b = A * (1.0 - F);
    // Counter-clockwise in (λ, φ) gives a negative ∮ q dλ: return it positive.
    -s * b * b / 2.0 / 1e6
}

/// Area (km²) of one polygon (exterior minus holes), on the WGS84 ellipsoid.
pub fn polygon_area_km2(rings: &Rings) -> f64 {
    let mut it = rings.iter();
    let Some(outer) = it.next() else {
        return 0.0;
    };
    let holes: f64 = it.map(|r| ring_signed_area_km2(r).abs()).sum();
    (ring_signed_area_km2(outer).abs() - holes).max(0.0)
}

/// Total area (km²) of several non-overlapping polygons.
pub fn polygons_area_km2(polys: &[Rings]) -> f64 {
    polys.iter().map(polygon_area_km2).sum()
}

/// Keep the part of `ring` on the inner side of an axis-parallel line.
fn clip(
    ring: &[(f64, f64)],
    inside: impl Fn((f64, f64)) -> bool,
    cut: impl Fn((f64, f64), (f64, f64)) -> (f64, f64),
) -> Vec<(f64, f64)> {
    let n = ring.len();
    let mut out = Vec::with_capacity(n + 4);
    if n == 0 {
        return out;
    }
    for i in 0..n {
        let cur = ring[i];
        let prev = ring[(i + n - 1) % n];
        let (ci, pi) = (inside(cur), inside(prev));
        if ci {
            if !pi {
                out.push(cut(prev, cur));
            }
            out.push(cur);
        } else if pi {
            out.push(cut(prev, cur));
        }
    }
    out
}

fn at_x(a: (f64, f64), b: (f64, f64), x: f64) -> (f64, f64) {
    let t = (x - a.0) / (b.0 - a.0);
    (x, a.1 + t * (b.1 - a.1))
}

fn at_y(a: (f64, f64), b: (f64, f64), y: f64) -> (f64, f64) {
    let t = (y - a.1) / (b.1 - a.1);
    (a.0 + t * (b.0 - a.0), y)
}

fn clip_band_y(ring: &[(f64, f64)], south: f64, north: f64) -> Vec<(f64, f64)> {
    let r = clip(ring, |p| p.1 <= north, |a, b| at_y(a, b, north));
    clip(&r, |p| p.1 >= south, |a, b| at_y(a, b, south))
}

fn clip_band_x(ring: &[(f64, f64)], west: f64, east: f64) -> Vec<(f64, f64)> {
    let r = clip(ring, |p| p.0 >= west, |a, b| at_x(a, b, west));
    clip(&r, |p| p.0 <= east, |a, b| at_x(a, b, east))
}

/// Area (km²) of the overlap of the polygons with each grid cell they touch, as
/// `(cell index, km²)` in row-major order; cells with no overlap are left out.
pub fn cell_overlap_km2(polys: &[Rings], spec: &GridSpec) -> Vec<(usize, f64)> {
    let mut acc: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
    for poly in polys {
        let Some((w, s, e, n)) = bbox(poly) else {
            continue;
        };
        if e <= spec.west || w >= spec.east() || n <= spec.south() || s >= spec.north {
            continue;
        }
        let r0 = (((spec.north - n) / spec.cell).floor().max(0.0)) as usize;
        let r1 = ((((spec.north - s) / spec.cell).ceil()).max(0.0) as usize).min(spec.height);
        for row in r0..r1 {
            let top = spec.north - row as f64 * spec.cell;
            let bottom = top - spec.cell;
            let band: Vec<Vec<(f64, f64)>> = poly
                .iter()
                .map(|ring| clip_band_y(ring, bottom, top))
                .collect();
            let Some((bw, _, be, _)) = bbox(&band) else {
                continue;
            };
            let c0 = (((bw - spec.west) / spec.cell).floor().max(0.0)) as usize;
            let c1 = ((((be - spec.west) / spec.cell).ceil()).max(0.0) as usize).min(spec.width);
            for col in c0..c1 {
                let left = spec.west + col as f64 * spec.cell;
                let right = left + spec.cell;
                let cell: Rings = band.iter().map(|r| clip_band_x(r, left, right)).collect();
                let a = polygon_area_km2(&cell);
                if a > 0.0 {
                    *acc.entry(spec.index(row, col)).or_insert(0.0) += a;
                }
            }
        }
    }
    acc.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::area::quad_area_km2;

    fn rect(w: f64, s: f64, e: f64, n: f64) -> Rings {
        vec![vec![(w, s), (e, s), (e, n), (w, n), (w, s)]]
    }

    #[test]
    fn rectangles_match_the_quadrangle_formula() {
        let a = polygon_area_km2(&rect(3.0, 44.0, 3.5, 44.25));
        let b = quad_area_km2(44.0, 44.25, 0.5);
        assert!((a - b).abs() < 1e-9 * b, "{a} {b}");
        // Orientation does not matter.
        let mut r = rect(3.0, 44.0, 3.5, 44.25);
        r[0].reverse();
        assert!((polygon_area_km2(&r) - b).abs() < 1e-9 * b);
    }

    #[test]
    fn holes_are_subtracted() {
        let mut p = rect(0.0, 0.0, 1.0, 1.0);
        p.push(rect(0.25, 0.25, 0.75, 0.75).remove(0));
        let want = quad_area_km2(0.0, 1.0, 1.0) - quad_area_km2(0.25, 0.75, 0.5);
        assert!((polygon_area_km2(&p) - want).abs() < 1e-9 * want);
    }

    #[test]
    fn triangle_is_half_of_a_small_square() {
        // On a small cell the latitude dependence is tiny but not zero: the diagonal triangle
        // is close to, not exactly, half the square.
        let t: Rings = vec![vec![(5.0, 45.0), (5.01, 45.0), (5.01, 45.01), (5.0, 45.0)]];
        let sq = quad_area_km2(45.0, 45.01, 0.01);
        let a = polygon_area_km2(&t);
        assert!((a / sq - 0.5).abs() < 1e-3, "{}", a / sq);
        // Upper-left triangle: the two halves add up to the square exactly.
        let u: Rings = vec![vec![(5.0, 45.0), (5.01, 45.01), (5.0, 45.01), (5.0, 45.0)]];
        assert!((a + polygon_area_km2(&u) - sq).abs() < 1e-9 * sq);
    }

    #[test]
    fn overlap_with_cells_is_exact_and_sums_to_the_polygon() {
        let spec = GridSpec {
            west: 3.0,
            north: 45.0,
            cell: 0.1,
            width: 10,
            height: 10,
        };
        // Exactly cells (row 2..4, col 1..3).
        let p = rect(3.1, 44.6, 3.3, 44.8);
        let ov = cell_overlap_km2(std::slice::from_ref(&p), &spec);
        assert_eq!(ov.len(), 4);
        for (i, a) in &ov {
            let row = i / spec.width;
            let top = spec.north - row as f64 * spec.cell;
            let want = quad_area_km2(top - spec.cell, top, spec.cell);
            assert!((a - want).abs() < 1e-9 * want, "{i} {a} {want}");
        }
        // Half a cell (western half of cell row 0, col 0).
        let h = rect(3.0, 44.9, 3.05, 45.0);
        let ov = cell_overlap_km2(&[h], &spec);
        assert_eq!(ov.len(), 1);
        let want = quad_area_km2(44.9, 45.0, 0.05);
        assert!((ov[0].1 - want).abs() < 1e-9 * want);
        // An irregular polygon: overlaps sum to its area.
        let irr: Rings = vec![vec![
            (3.05, 44.15),
            (3.72, 44.31),
            (3.93, 44.88),
            (3.41, 44.62),
            (3.22, 44.95),
            (3.05, 44.15),
        ]];
        let total: f64 = cell_overlap_km2(std::slice::from_ref(&irr), &spec)
            .iter()
            .map(|c| c.1)
            .sum();
        let a = polygon_area_km2(&irr);
        assert!((total - a).abs() < 1e-9 * a, "{total} {a}");
    }

    #[test]
    fn parts_outside_the_grid_are_ignored() {
        let spec = GridSpec {
            west: 0.0,
            north: 1.0,
            cell: 0.5,
            width: 2,
            height: 2,
        };
        let p = rect(-1.0, 0.5, 0.5, 2.0);
        let ov = cell_overlap_km2(&[p], &spec);
        assert_eq!(ov.len(), 1);
        assert_eq!(ov[0].0, 0);
        let want = quad_area_km2(0.5, 1.0, 0.5);
        assert!((ov[0].1 - want).abs() < 1e-9 * want);
    }
}
