//! Distance (km) from each cell centre of a geographic grid to the nearest target cell.
//!
//! Exact Euclidean distance transform (Felzenszwalb & Huttenlocher 2012, "Distance transforms of
//! sampled functions", Theory of Computing 8:415–428) in a *locally metric* approximation of the
//! lon/lat grid:
//!
//! - north–south positions are the meridian arc lengths of the row centres on WGS84 (ρ(φ)·Δφ,
//!   summed row by row), so they are exact up to the row-midpoint quadrature;
//! - east–west offsets are measured with the parallel scale ν(φ′)·cosφ′·Δλ of the **target**
//!   row φ′.
//!
//! The distance between cell (r, c) and target (r′, c′) is therefore
//! `sqrt((dx(r′)·(c − c′))² + (y(r) − y(r′))²)`, which is what the separable transform minimises
//! exactly (the first pass is per row with that row's scale, the second per column over arbitrary
//! increasing positions). Against the geodesic distance the error comes only from using the
//! target row's parallel scale for the whole east–west leg: for a north–south separation Δφ the
//! relative error is about `tanφ · Δφ` (≈ 0.4 % at 45° N for 25 km, i.e. ≈ 100 m), and below
//! 1 % in the tests up to 50 km. This is negligible at 30″ (~1 km) cells.
//!
//! **Edge effect.** Only targets inside the grid are seen. A caller computing distances for a
//! study window must pad the window by at least the largest distance it wants to be exact for
//! (and then crop): cells whose true nearest target lies outside the padded grid get an
//! overestimate. [`distance_to_mask_km`] returns `f32::INFINITY` where the grid has no target.

use crate::crs::metres_per_degree;
use crate::grid::GridSpec;

/// Distance in km from each cell centre to the nearest cell where `mask` is true (row-major,
/// same layout as `spec`). `f32::INFINITY` everywhere if the mask has no true cell.
///
/// # Panics
/// Never; a mask of the wrong length is treated as all-false beyond its end.
pub fn distance_to_mask_km(spec: &GridSpec, mask: &[bool]) -> Vec<f32> {
    let (w, h) = (spec.width, spec.height);
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let lat_of = |r: usize| spec.north - (r as f64 + 0.5) * spec.cell;
    // East–west cell size (km) of each row.
    let dx: Vec<f64> = (0..h)
        .map(|r| metres_per_degree(lat_of(r)).0 * spec.cell / 1000.0)
        .collect();
    // North–south position (km) of each row centre, increasing southwards.
    let mut ypos = vec![0.0f64; h];
    for r in 1..h {
        let mid = spec.north - r as f64 * spec.cell;
        ypos[r] = ypos[r - 1] + metres_per_degree(mid).1 * spec.cell / 1000.0;
    }
    // Pass 1: squared horizontal distance to the nearest target in the same row.
    let mut g = vec![f64::INFINITY; w * h];
    let xs: Vec<f64> = (0..w).map(|c| c as f64).collect();
    let mut f = vec![0.0f64; w.max(h)];
    let mut out_row = vec![0.0f64; w.max(h)];
    for r in 0..h {
        let mut any = false;
        for (c, fc) in f.iter_mut().take(w).enumerate() {
            let t = mask.get(r * w + c).copied().unwrap_or(false);
            any |= t;
            *fc = if t { 0.0 } else { f64::INFINITY };
        }
        if !any {
            continue;
        }
        let pos: Vec<f64> = xs.iter().map(|x| x * dx[r]).collect();
        lower_envelope(&f[..w], &pos, &mut out_row[..w]);
        g[r * w..(r + 1) * w].copy_from_slice(&out_row[..w]);
    }
    // Pass 2: along each column over the row positions.
    let mut out = vec![f32::INFINITY; w * h];
    for c in 0..w {
        let mut any = false;
        for r in 0..h {
            f[r] = g[r * w + c];
            any |= f[r].is_finite();
        }
        if !any {
            continue;
        }
        lower_envelope(&f[..h], &ypos, &mut out_row[..h]);
        for r in 0..h {
            out[r * w + c] = out_row[r].sqrt() as f32;
        }
    }
    out
}

/// 1-D squared distance transform: `d[i] = min_j f[j] + (p[i] − p[j])²` for strictly increasing
/// positions `p` (lower envelope of parabolas). `f[j] = ∞` marks non-sites.
fn lower_envelope(f: &[f64], p: &[f64], d: &mut [f64]) {
    let n = f.len();
    let mut v: Vec<usize> = Vec::with_capacity(n);
    let mut z: Vec<f64> = Vec::with_capacity(n + 1);
    for q in 0..n {
        if !f[q].is_finite() {
            continue;
        }
        loop {
            let Some(&last) = v.last() else {
                v.push(q);
                z.clear();
                z.push(f64::NEG_INFINITY);
                break;
            };
            let s =
                ((f[q] + p[q] * p[q]) - (f[last] + p[last] * p[last])) / (2.0 * (p[q] - p[last]));
            if let Some(&zl) = z.last()
                && s <= zl
                && v.len() > 1
            {
                v.pop();
                z.pop();
                continue;
            }
            v.push(q);
            z.push(s);
            break;
        }
    }
    if v.is_empty() {
        d.iter_mut().for_each(|x| *x = f64::INFINITY);
        return;
    }
    // z[k] is the left boundary of parabola v[k]'s interval; z[0] = −∞.
    let mut k = 0;
    for i in 0..n {
        while k + 1 < v.len() && z[k + 1] < p[i] {
            k += 1;
        }
        let j = v[k];
        d[i] = (p[i] - p[j]).powi(2) + f[j];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute(spec: &GridSpec, mask: &[bool]) -> Vec<f64> {
        let (w, h) = (spec.width, spec.height);
        let lat_of = |r: usize| spec.north - (r as f64 + 0.5) * spec.cell;
        let dx: Vec<f64> = (0..h)
            .map(|r| metres_per_degree(lat_of(r)).0 * spec.cell / 1000.0)
            .collect();
        let mut ypos = vec![0.0f64; h];
        for r in 1..h {
            let mid = spec.north - r as f64 * spec.cell;
            ypos[r] = ypos[r - 1] + metres_per_degree(mid).1 * spec.cell / 1000.0;
        }
        let mut out = vec![f64::INFINITY; w * h];
        for r in 0..h {
            for c in 0..w {
                for r2 in 0..h {
                    for c2 in 0..w {
                        if mask[r2 * w + c2] {
                            let ex = dx[r2] * (c as f64 - c2 as f64);
                            let ny = ypos[r] - ypos[r2];
                            let d = (ex * ex + ny * ny).sqrt();
                            if d < out[r * w + c] {
                                out[r * w + c] = d;
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// Deterministic pseudo-random mask (xorshift), density ~p.
    fn mask(n: usize, seed: u64, p: f64) -> Vec<bool> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                (s % 10_000) as f64 / 10_000.0 < p
            })
            .collect()
    }

    #[test]
    fn exact_against_brute_force() {
        for (seed, p, w, h, north) in [
            (1u64, 0.02, 37, 29, 51.0),
            (7, 0.10, 23, 41, 43.5),
            (99, 0.005, 40, 40, 62.0),
            (5, 0.3, 11, 13, -30.0),
        ] {
            let spec = GridSpec {
                west: 2.0,
                north,
                cell: 1.0 / 120.0,
                width: w,
                height: h,
            };
            let m = mask(w * h, seed, p);
            let d = distance_to_mask_km(&spec, &m);
            let b = brute(&spec, &m);
            for (x, y) in d.iter().zip(&b) {
                if y.is_infinite() {
                    assert!(x.is_infinite());
                } else {
                    assert!((f64::from(*x) - y).abs() < 1e-4, "{x} vs {y}");
                }
            }
        }
    }

    #[test]
    fn empty_mask_is_infinite() {
        let spec = GridSpec {
            west: 0.0,
            north: 45.0,
            cell: 0.01,
            width: 5,
            height: 4,
        };
        assert!(
            distance_to_mask_km(&spec, &[false; 20])
                .iter()
                .all(|x| x.is_infinite())
        );
    }

    /// Against the great-circle distance (mean radius 6371.0088 km): below 1 % up to ~50 km.
    #[test]
    fn close_to_geodesic() {
        let spec = GridSpec {
            west: 4.0,
            north: 46.0,
            cell: 1.0 / 120.0,
            width: 121,
            height: 121,
        };
        let mut m = vec![false; spec.len()];
        m[spec.index(60, 60)] = true;
        let d = distance_to_mask_km(&spec, &m);
        let (lng0, lat0) = spec.center(60, 60);
        let hav = |lng: f64, lat: f64| {
            let (p1, p2) = (lat0.to_radians(), lat.to_radians());
            let dp = p2 - p1;
            let dl = (lng - lng0).to_radians();
            let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
            2.0 * 6371.0088 * a.sqrt().asin()
        };
        for r in (0..121).step_by(10) {
            for c in (0..121).step_by(10) {
                let (lng, lat) = spec.center(r, c);
                let g = hav(lng, lat);
                if g < 1.0 {
                    continue;
                }
                let rel = (f64::from(d[spec.index(r, c)]) - g).abs() / g;
                assert!(rel < 0.01, "({r},{c}) {} vs {g}", d[spec.index(r, c)]);
            }
        }
    }
}
