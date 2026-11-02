//! Geographic evaluation helpers: great-circle distances, distance thinning of presences,
//! square spatial blocks for cross-validation and the spatial autocorrelation range used to size
//! them.
//!
//! Coordinates are `(longitude, latitude)` in decimal degrees (WGS84). Distances use a sphere of
//! the WGS84 mean radius (IUGG R1 = 6371.0088 km); the error against the ellipsoid is < 0.5 %,
//! negligible for thinning and blocking distances of kilometres to hundreds of kilometres.
//! Method notes: `docs/architecture/eval-methods-v1.md`.

use std::collections::{BTreeMap, HashMap};

use crate::Folds;
use crate::rng::Rng;

/// WGS84 mean Earth radius R1 (IUGG), km.
pub const EARTH_RADIUS_KM: f64 = 6371.0088;

/// Length of one degree of latitude (and of longitude at the equator) on that sphere, km.
pub const KM_PER_DEGREE: f64 = EARTH_RADIUS_KM * std::f64::consts::PI / 180.0;

/// Great-circle distance in km between two `(lng, lat)` points (haversine formula; numerically
/// stable for small distances, clamped for antipodal points).
pub fn haversine_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (lat1, lat2) = (a.1.to_radians(), b.1.to_radians());
    let dlat = lat2 - lat1;
    let dlon = (b.0 - a.0).to_radians();
    let s1 = (dlat / 2.0).sin();
    let s2 = (dlon / 2.0).sin();
    let h = s1 * s1 + lat1.cos() * lat2.cos() * s2 * s2;
    2.0 * EARTH_RADIUS_KM * h.sqrt().min(1.0).asin()
}

/// Point on the sphere of radius [`EARTH_RADIUS_KM`] (Earth-centred Cartesian, km).
fn cartesian(p: (f64, f64)) -> [f64; 3] {
    let (lon, lat) = (p.0.to_radians(), p.1.to_radians());
    [
        EARTH_RADIUS_KM * lat.cos() * lon.cos(),
        EARTH_RADIUS_KM * lat.cos() * lon.sin(),
        EARTH_RADIUS_KM * lat.sin(),
    ]
}

fn finite(p: (f64, f64)) -> bool {
    p.0.is_finite() && p.1.is_finite()
}

/// Distance thinning of presences (spThin, Aiello-Lammens et al. 2015, Ecography 38:541–545):
/// returns the indices (ascending) of a subset of `points` in which no two kept points are closer
/// than `min_km` (great-circle distance).
///
/// Greedy randomized algorithm: points are visited in a random order drawn from `rng`, and a
/// point is kept when no already kept point lies strictly within `min_km` (spThin removes pairs
/// with distance `< thin.par`). Deterministic for a given seed. spThin's `thin()` repeats the
/// randomization and keeps the largest set; use [`thin_by_distance_reps`] for that.
///
/// Neighbour search uses a hash grid over Earth-centred 3-D coordinates with cells the size of the
/// chord that corresponds to `min_km`, so it has no singularity at the poles or the antimeridian;
/// the final test is the exact haversine distance. Expected cost O(n) for well-spread points
/// (≈ 27 cell lookups per point); worst case O(n·kept) when everything falls in a few cells.
///
/// Points with non-finite coordinates are never kept. `min_km <= 0` (or NaN) keeps every finite
/// point. A `min_km` of half the Earth's circumference or more keeps a single point.
pub fn thin_by_distance(points: &[(f64, f64)], min_km: f64, rng: &mut Rng) -> Vec<usize> {
    let mut order: Vec<usize> = (0..points.len()).filter(|&i| finite(points[i])).collect();
    if min_km.is_nan() || min_km <= 0.0 {
        return order;
    }
    rng.shuffle(&mut order);
    // Great-circle distance d < min_km  ⇔  chord < 2R·sin(min_km / 2R) (monotone on [0, πR]).
    let half_angle = (min_km / (2.0 * EARTH_RADIUS_KM)).min(std::f64::consts::FRAC_PI_2);
    // Slightly enlarged cells so rounding never hides a neighbour in a non-adjacent cell.
    let cell = 2.0 * EARTH_RADIUS_KM * half_angle.sin() * (1.0 + 1e-9);
    let key = |c: [f64; 3]| -> [i64; 3] {
        [
            (c[0] / cell).floor() as i64,
            (c[1] / cell).floor() as i64,
            (c[2] / cell).floor() as i64,
        ]
    };
    let mut grid: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
    let mut kept = Vec::new();
    for i in order {
        let p = points[i];
        let k = key(cartesian(p));
        let mut clash = false;
        'search: for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let nk = [k[0] + dx, k[1] + dy, k[2] + dz];
                    if let Some(cell_pts) = grid.get(&nk) {
                        for &j in cell_pts {
                            if haversine_km(p, points[j]) < min_km {
                                clash = true;
                                break 'search;
                            }
                        }
                    }
                }
            }
        }
        if !clash {
            grid.entry(k).or_default().push(i);
            kept.push(i);
        }
    }
    kept.sort_unstable();
    kept
}

/// [`thin_by_distance`] repeated `reps` times (at least once) with successive draws from `rng`,
/// returning the largest kept set (the first one on ties), as spThin's `thin(reps = …)` with
/// `max.files = 1`. Deterministic for a given seed. Cost: `reps` × one thinning.
pub fn thin_by_distance_reps(
    points: &[(f64, f64)],
    min_km: f64,
    reps: usize,
    rng: &mut Rng,
) -> Vec<usize> {
    let mut best = thin_by_distance(points, min_km, rng);
    for _ in 1..reps {
        let cand = thin_by_distance(points, min_km, rng);
        if cand.len() > best.len() {
            best = cand;
        }
    }
    best
}

/// Square spatial blocks of about `block_km` × `block_km` assigned to `k` folds (blockCV
/// `cv_spatial`, Valavi et al. 2019, Methods Ecol. Evol. 10:225–232; Roberts et al. 2017).
///
/// Projection: a local equirectangular approximation centred on the mean latitude φ₀ of all finite
/// points (presences and background), x = (λ − λmin)·cos φ₀·111.195 km, y = (φ − φmin)·111.195 km;
/// the grid is anchored at the south-west corner of the points. Blocks are therefore exactly
/// `block_km` tall, and `block_km · cos φ / cos φ₀` wide at latitude φ: adequate for regional
/// extents (a few tens of degrees of latitude), not for global ones. Longitudes are not wrapped:
/// study areas crossing the antimeridian must be shifted to a continuous range (e.g. 0..360) by
/// the caller. cos φ₀ is clamped at 0.01 near the poles.
///
/// Fold assignment (deterministic for a given seed): blocks containing presences are shuffled with
/// `rng`, then stably sorted by decreasing presence count, and each is given to the fold with the
/// fewest presences so far (lowest fold index on ties) — the longest-processing-time greedy rule,
/// which balances presence counts. Background points follow the fold of their block; blocks with
/// background only are shuffled, sorted by decreasing count and given to the fold with the fewest
/// background points, the same way.
///
/// Every fold receives ≥ 1 presence whenever there are at least `k` occupied blocks. With fewer
/// occupied blocks the number of folds is reduced: the returned [`Folds::k`] is
/// `min(k, occupied blocks)` (at least 1), so callers must read `k` from the result. Points with
/// non-finite coordinates are grouped in one extra block; clean coordinates beforehand.
pub fn spatial_grid_blocks(
    pres: &[(f64, f64)],
    bg: &[(f64, f64)],
    block_km: f64,
    k: usize,
    rng: &mut Rng,
) -> Folds {
    let all = || pres.iter().chain(bg).copied().filter(|&p| finite(p));
    let n_all = all().count();
    let (mut lon_min, mut lat_min, mut lat_sum) = (f64::INFINITY, f64::INFINITY, 0.0);
    for (lon, lat) in all() {
        lon_min = lon_min.min(lon);
        lat_min = lat_min.min(lat);
        lat_sum += lat;
    }
    let lat0 = if n_all > 0 {
        lat_sum / n_all as f64
    } else {
        0.0
    };
    let coslat = lat0.to_radians().cos().max(0.01);
    let size = if block_km.is_finite() && block_km > 0.0 {
        block_km
    } else {
        f64::MAX
    };
    let block = |p: (f64, f64)| -> (i64, i64) {
        if !finite(p) {
            return (i64::MIN, i64::MIN);
        }
        let x = (p.0 - lon_min) * coslat * KM_PER_DEGREE;
        let y = (p.1 - lat_min) * KM_PER_DEGREE;
        ((x / size).floor() as i64, (y / size).floor() as i64)
    };
    let pres_blocks: Vec<(i64, i64)> = pres.iter().map(|&p| block(p)).collect();
    let bg_blocks: Vec<(i64, i64)> = bg.iter().map(|&p| block(p)).collect();

    let mut pres_count: BTreeMap<(i64, i64), usize> = BTreeMap::new();
    for &b in &pres_blocks {
        *pres_count.entry(b).or_default() += 1;
    }
    let k = k.min(pres_count.len()).max(1);
    let mut fold_of: HashMap<(i64, i64), usize> = HashMap::new();
    let mut fold_pres = vec![0usize; k];
    for (b, c) in shuffled_by_count(pres_count, rng) {
        let f = argmin(&fold_pres);
        fold_pres[f] += c;
        fold_of.insert(b, f);
    }

    let mut fold_bg = vec![0usize; k];
    let mut bg_only: BTreeMap<(i64, i64), usize> = BTreeMap::new();
    for b in &bg_blocks {
        match fold_of.get(b) {
            Some(&f) => fold_bg[f] += 1,
            None => *bg_only.entry(*b).or_default() += 1,
        }
    }
    for (b, c) in shuffled_by_count(bg_only, rng) {
        let f = argmin(&fold_bg);
        fold_bg[f] += c;
        fold_of.insert(b, f);
    }
    let lookup = |b: &(i64, i64)| fold_of.get(b).copied().unwrap_or(0);
    Folds {
        k,
        presence: pres_blocks.iter().map(lookup).collect(),
        background: bg_blocks.iter().map(lookup).collect(),
    }
}

/// Blocks in key order, shuffled, then stably sorted by decreasing count.
fn shuffled_by_count(m: BTreeMap<(i64, i64), usize>, rng: &mut Rng) -> Vec<((i64, i64), usize)> {
    let mut v: Vec<((i64, i64), usize)> = m.into_iter().collect();
    rng.shuffle(&mut v);
    v.sort_by_key(|b| std::cmp::Reverse(b.1));
    v
}

fn argmin(v: &[usize]) -> usize {
    let mut best = 0;
    for (i, &x) in v.iter().enumerate() {
        if x < v[best] {
            best = i;
        }
    }
    best
}

/// Variogram model families, each written with its practical range r (the lag at which the
/// model reaches the sill, or 95 % of it for the asymptotic models).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariogramModel {
    /// γ(h) = c0 + c1·(1 − e^(−3h/r)).
    Exponential,
    /// γ(h) = c0 + c1·(1.5·h/r − 0.5·(h/r)³) for h < r, c0 + c1 beyond.
    Spherical,
    /// γ(h) = c0 + c1·(1 − e^(−3h²/r²)).
    Gaussian,
}

impl VariogramModel {
    pub const ALL: [Self; 3] = [Self::Exponential, Self::Spherical, Self::Gaussian];

    /// Normalized structure g(h) ∈ [0, 1] for practical range `r`.
    pub fn shape(self, h: f64, r: f64) -> f64 {
        let x = h / r;
        match self {
            Self::Exponential => 1.0 - (-3.0 * x).exp(),
            Self::Spherical => {
                if x < 1.0 {
                    1.5 * x - 0.5 * x * x * x
                } else {
                    1.0
                }
            }
            Self::Gaussian => 1.0 - (-3.0 * x * x).exp(),
        }
    }
}

/// Variogram fitted to one variable: γ(h) = nugget + partial_sill·g(h; range_km).
#[derive(Debug, Clone, PartialEq)]
pub struct VariogramFit {
    pub model: VariogramModel,
    /// Practical range, km (exponential: 3a; Gaussian: √3·a; spherical: a, in gstat's `a`).
    pub range_km: f64,
    pub nugget: f64,
    pub partial_sill: f64,
    /// The best range is the largest one searched (3 × `cutoff_km`): the true range is probably
    /// larger than can be resolved from these data, so `range_km` is a lower bound.
    pub at_upper_bound: bool,
}

/// Spatial autocorrelation ranges of a set of variables (see [`autocorrelation_range_km`]).
#[derive(Debug, Clone, PartialEq)]
pub struct RangeEstimate {
    /// One entry per input variable; `None` when it could not be fitted (constant variable,
    /// too few pairs, or no spatial structure: pure-nugget fit).
    pub per_variable: Vec<Option<VariogramFit>>,
    /// Median of the fitted practical ranges, km (the block size blockCV suggests).
    pub median_km: f64,
    /// Points actually used (after dropping non-finite coordinates and subsampling).
    pub n_points: usize,
    /// Maximum lag of the empirical variogram, km.
    pub cutoff_km: f64,
}

/// Maximum number of points in the variogram subsample (2000 points ⇒ ≈ 2·10⁶ pairs).
pub const VARIOGRAM_MAX_POINTS: usize = 2000;
const VARIOGRAM_BINS: usize = 15;
const RANGE_GRID: usize = 240;

/// Spatial autocorrelation range of predictors, used to size spatial blocks (blockCV
/// `cv_spatial_autocor`, Valavi et al. 2019, which fits variograms with automap/gstat).
///
/// `coords[i]` is the location of sample `i`; `values[v][i]` the value of variable `v` there
/// (typically predictors sampled at random background points). At most
/// [`VARIOGRAM_MAX_POINTS`] points with finite coordinates are drawn with `rng`.
///
/// Empirical semivariogram: γ̂(h) = ½·mean (zᵢ − zⱼ)² over pairs in 15 equal-width distance bins
/// up to a cutoff of one third of the largest pairwise distance (gstat's default cutoff is one
/// third of the bounding-box diagonal), lag = mean pair distance of the bin; pairs with a
/// non-finite value are skipped. Models: exponential, spherical and Gaussian with nugget (as
/// automap's `autofitVariogram`, used by blockCV, tries several families and keeps the best),
/// fitted by weighted least squares with gstat's default weights Nⱼ/hⱼ² (`fit.method = 7`): for
/// each practical range r on a 240-point log-spaced grid from 3/20 of a bin width to 3 × cutoff,
/// nugget and partial sill (both ≥ 0) are solved in closed form; the (model, r) with the smallest
/// weighted SSE wins. An exponential-only fit was tried first and rejected: smooth climate-like
/// fields have sigmoidal variograms near the origin, and the exponential model then overstated
/// the range ≈ 2.5× on synthetic fields of known scale (see the unit tests).
///
/// Returns `None` when fewer than 10 usable points exist or no variable can be fitted.
/// Cost: O(m²) distances for m ≤ 2000 points plus O(m²) per variable.
pub fn autocorrelation_range_km(
    coords: &[(f64, f64)],
    values: &[Vec<f64>],
    rng: &mut Rng,
) -> Option<RangeEstimate> {
    let valid: Vec<usize> = (0..coords.len()).filter(|&i| finite(coords[i])).collect();
    let mut sample: Vec<usize> = if valid.len() > VARIOGRAM_MAX_POINTS {
        rng.sample_indices(valid.len(), VARIOGRAM_MAX_POINTS)
            .into_iter()
            .map(|j| valid[j])
            .collect()
    } else {
        valid
    };
    sample.sort_unstable();
    let m = sample.len();
    if m < 10 {
        return None;
    }
    let pts: Vec<(f64, f64)> = sample.iter().map(|&i| coords[i]).collect();
    let mut max_d: f64 = 0.0;
    for i in 0..m {
        for j in (i + 1)..m {
            max_d = max_d.max(haversine_km(pts[i], pts[j]));
        }
    }
    let cutoff = max_d / 3.0;
    if cutoff <= 0.0 || !cutoff.is_finite() {
        return None;
    }
    let width = cutoff / VARIOGRAM_BINS as f64;
    // Bin of every pair (u8::MAX = beyond the cutoff), in (i, j > i) order.
    let mut bins = Vec::with_capacity(m * (m - 1) / 2);
    let mut lag_sum = [0.0f64; VARIOGRAM_BINS];
    let mut lag_n = [0usize; VARIOGRAM_BINS];
    for i in 0..m {
        for j in (i + 1)..m {
            let d = haversine_km(pts[i], pts[j]);
            let b = (d / width).floor();
            if b < VARIOGRAM_BINS as f64 {
                let b = b as usize;
                lag_sum[b] += d;
                lag_n[b] += 1;
                bins.push(b as u8);
            } else {
                bins.push(u8::MAX);
            }
        }
    }
    let per_variable: Vec<Option<VariogramFit>> = values
        .iter()
        .map(|col| {
            let z: Vec<f64> = sample
                .iter()
                .map(|&i| col.get(i).copied().unwrap_or(f64::NAN))
                .collect();
            let mut g_sum = [0.0f64; VARIOGRAM_BINS];
            let mut g_n = [0usize; VARIOGRAM_BINS];
            let mut p = 0;
            for i in 0..m {
                for j in (i + 1)..m {
                    let b = bins[p];
                    p += 1;
                    if b == u8::MAX || !z[i].is_finite() || !z[j].is_finite() {
                        continue;
                    }
                    let b = b as usize;
                    let dz = z[i] - z[j];
                    g_sum[b] += 0.5 * dz * dz;
                    g_n[b] += 1;
                }
            }
            let mut emp = Vec::new();
            for b in 0..VARIOGRAM_BINS {
                if g_n[b] > 0 && lag_n[b] > 0 {
                    emp.push((
                        lag_sum[b] / lag_n[b] as f64,
                        g_sum[b] / g_n[b] as f64,
                        g_n[b],
                    ));
                }
            }
            fit_variogram(&emp, width, cutoff)
        })
        .collect();
    let mut ranges: Vec<f64> = per_variable.iter().flatten().map(|f| f.range_km).collect();
    if ranges.is_empty() {
        return None;
    }
    ranges.sort_by(|a, b| a.total_cmp(b));
    let n = ranges.len();
    let median_km = if n % 2 == 1 {
        ranges[n / 2]
    } else {
        0.5 * (ranges[n / 2 - 1] + ranges[n / 2])
    };
    Some(RangeEstimate {
        per_variable,
        median_km,
        n_points: m,
        cutoff_km: cutoff,
    })
}

/// Weighted least-squares fit of every [`VariogramModel`] to `(lag, gamma, n_pairs)` bins.
fn fit_variogram(emp: &[(f64, f64, usize)], width: f64, cutoff: f64) -> Option<VariogramFit> {
    if emp.len() < 3 || emp.iter().all(|e| e.1 <= 0.0) {
        return None;
    }
    let w: Vec<f64> = emp
        .iter()
        .map(|&(h, _, n)| n as f64 / (h.max(width * 1e-3) * h.max(width * 1e-3)))
        .collect();
    let (lo, hi) = ((3.0 * width / 20.0).ln(), (3.0 * cutoff).ln());
    // (sse, model, range, c0, c1, grid index)
    let mut best: Option<(f64, VariogramModel, f64, f64, f64, usize)> = None;
    for model in VariogramModel::ALL {
        for gi in 0..RANGE_GRID {
            let r = (lo + (hi - lo) * gi as f64 / (RANGE_GRID - 1) as f64).exp();
            let g: Vec<f64> = emp.iter().map(|e| model.shape(e.0, r)).collect();
            let Some((c0, c1, sse)) = nonneg_ls(emp, &g, &w) else {
                continue;
            };
            if best.is_none_or(|b| sse < b.0) {
                best = Some((sse, model, r, c0, c1, gi));
            }
        }
    }
    let (_, model, r, c0, c1, gi) = best?;
    if c1 <= 0.0 {
        return None;
    }
    Some(VariogramFit {
        model,
        range_km: r,
        nugget: c0,
        partial_sill: c1,
        at_upper_bound: gi == RANGE_GRID - 1,
    })
}

/// min Σ wⱼ (γⱼ − c0 − c1·gⱼ)² subject to c0, c1 ≥ 0 (closed form over the active sets).
fn nonneg_ls(emp: &[(f64, f64, usize)], g: &[f64], w: &[f64]) -> Option<(f64, f64, f64)> {
    let (mut s_w, mut s_g, mut s_gg, mut s_y, mut s_gy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for ((e, &gj), &wj) in emp.iter().zip(g).zip(w) {
        s_w += wj;
        s_g += wj * gj;
        s_gg += wj * gj * gj;
        s_y += wj * e.1;
        s_gy += wj * gj * e.1;
    }
    let sse = |c0: f64, c1: f64| -> f64 {
        emp.iter()
            .zip(g)
            .zip(w)
            .map(|((e, &gj), &wj)| {
                let r = e.1 - c0 - c1 * gj;
                wj * r * r
            })
            .sum()
    };
    let mut cands: Vec<(f64, f64)> = Vec::with_capacity(3);
    let det = s_w * s_gg - s_g * s_g;
    if det.abs() > 1e-12 * s_w * s_gg {
        let c1 = (s_w * s_gy - s_g * s_y) / det;
        let c0 = (s_y - c1 * s_g) / s_w;
        if c0 >= 0.0 && c1 >= 0.0 {
            cands.push((c0, c1));
        }
    }
    if s_gg > 0.0 {
        cands.push((0.0, (s_gy / s_gg).max(0.0)));
    }
    if s_w > 0.0 {
        cands.push(((s_y / s_w).max(0.0), 0.0));
    }
    cands
        .into_iter()
        .map(|(c0, c1)| (c0, c1, sse(c0, c1)))
        .min_by(|a, b| a.2.total_cmp(&b.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_points(n: usize, lon: (f64, f64), lat: (f64, f64), seed: u64) -> Vec<(f64, f64)> {
        let mut r = Rng::new(seed);
        (0..n)
            .map(|_| {
                (
                    lon.0 + (lon.1 - lon.0) * r.uniform(),
                    lat.0 + (lat.1 - lat.0) * r.uniform(),
                )
            })
            .collect()
    }

    #[test]
    fn haversine_known_distances() {
        // One degree of latitude.
        assert!((haversine_km((0.0, 0.0), (0.0, 1.0)) - KM_PER_DEGREE).abs() < 1e-9);
        // Across the antimeridian: 179.5E to 179.5W is 1 degree of longitude on the equator.
        assert!((haversine_km((179.5, 0.0), (-179.5, 0.0)) - KM_PER_DEGREE).abs() < 1e-6);
        // Antipodes: half the circumference.
        let half = std::f64::consts::PI * EARTH_RADIUS_KM;
        assert!((haversine_km((0.0, 0.0), (180.0, 0.0)) - half).abs() < 1e-6);
        // Paris–Madrid ≈ 1053 km.
        let d = haversine_km((2.3522, 48.8566), (-3.7038, 40.4168));
        assert!((1040.0..1065.0).contains(&d), "{d}");
    }

    fn brute_force_ok(points: &[(f64, f64)], kept: &[usize], min_km: f64) -> bool {
        for (a, &i) in kept.iter().enumerate() {
            for &j in &kept[a + 1..] {
                if haversine_km(points[i], points[j]) < min_km {
                    return false;
                }
            }
        }
        true
    }

    /// Maximality: every removed point has a kept point within `min_km`.
    fn maximal(points: &[(f64, f64)], kept: &[usize], min_km: f64) -> bool {
        (0..points.len())
            .filter(|i| kept.binary_search(i).is_err())
            .all(|i| {
                kept.iter()
                    .any(|&j| haversine_km(points[i], points[j]) < min_km)
            })
    }

    #[test]
    fn thinning_respects_distance_and_is_maximal() {
        let pts = random_points(3000, (-5.0, 5.0), (40.0, 48.0), 1);
        let kept = thin_by_distance(&pts, 20.0, &mut Rng::new(9));
        assert!(kept.len() > 100 && kept.len() < 3000, "{}", kept.len());
        assert!(brute_force_ok(&pts, &kept, 20.0));
        assert!(maximal(&pts, &kept, 20.0));
        assert!(kept.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn thinning_deterministic_for_seed() {
        let pts = random_points(2000, (0.0, 10.0), (0.0, 10.0), 2);
        let a = thin_by_distance(&pts, 50.0, &mut Rng::new(5));
        let b = thin_by_distance(&pts, 50.0, &mut Rng::new(5));
        let c = thin_by_distance(&pts, 50.0, &mut Rng::new(6));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn thinning_poles_and_antimeridian() {
        // Points spread across the antimeridian and around the north pole.
        let mut pts = random_points(800, (178.0, 180.0), (-2.0, 2.0), 3);
        pts.extend(random_points(800, (-180.0, -178.0), (-2.0, 2.0), 4));
        pts.extend(random_points(800, (-180.0, 180.0), (88.0, 90.0), 5));
        let kept = thin_by_distance(&pts, 30.0, &mut Rng::new(1));
        assert!(brute_force_ok(&pts, &kept, 30.0));
        assert!(maximal(&pts, &kept, 30.0));
    }

    #[test]
    fn thinning_edge_cases() {
        let pts = vec![(0.0, 0.0), (0.0, 0.0), (f64::NAN, 1.0), (10.0, 10.0)];
        assert_eq!(thin_by_distance(&pts, 0.0, &mut Rng::new(1)), vec![0, 1, 3]);
        let k = thin_by_distance(&pts, 1.0, &mut Rng::new(1));
        assert_eq!(k.len(), 2);
        assert!(k.contains(&3));
        assert_eq!(thin_by_distance(&pts, 30_000.0, &mut Rng::new(1)).len(), 1);
        assert!(thin_by_distance(&[], 10.0, &mut Rng::new(1)).is_empty());
    }

    #[test]
    fn thinning_reps_keeps_largest() {
        let pts = random_points(1500, (0.0, 5.0), (40.0, 45.0), 7);
        let single = thin_by_distance(&pts, 25.0, &mut Rng::new(3));
        let best = thin_by_distance_reps(&pts, 25.0, 10, &mut Rng::new(3));
        assert!(best.len() >= single.len());
        assert!(brute_force_ok(&pts, &best, 25.0));
        assert_eq!(
            best,
            thin_by_distance_reps(&pts, 25.0, 10, &mut Rng::new(3))
        );
    }

    #[test]
    fn thinning_50k_points_is_fast() {
        let pts = random_points(50_000, (-10.0, 10.0), (35.0, 55.0), 11);
        let t = std::time::Instant::now();
        let kept = thin_by_distance(&pts, 10.0, &mut Rng::new(1));
        assert!(t.elapsed().as_secs_f64() < 5.0, "{:?}", t.elapsed());
        assert!(!kept.is_empty());
    }

    #[test]
    fn grid_blocks_balanced_and_spatially_coherent() {
        let pres = random_points(500, (-5.0, 5.0), (40.0, 48.0), 21);
        let bg = random_points(5000, (-5.0, 5.0), (40.0, 48.0), 22);
        let f = spatial_grid_blocks(&pres, &bg, 100.0, 5, &mut Rng::new(1));
        assert_eq!(f.k, 5);
        let mut counts = [0usize; 5];
        for &g in &f.presence {
            counts[g] += 1;
        }
        assert!(
            counts.iter().all(|&c| (80..=120).contains(&c)),
            "{counts:?}"
        );
        let mut bgc = [0usize; 5];
        for &g in &f.background {
            bgc[g] += 1;
        }
        assert!(bgc.iter().all(|&c| c > 500), "{bgc:?}");
        // Points in the same block share a fold: two points 1 km apart near a block centre.
        let f2 = spatial_grid_blocks(
            &[(0.0, 40.0), (0.001, 40.001)],
            &[(0.0005, 40.0005)],
            100.0,
            2,
            &mut Rng::new(1),
        );
        assert_eq!(f2.k, 1);
        assert_eq!(f2.presence, vec![0, 0]);
        assert_eq!(f2.background, vec![0]);
    }

    #[test]
    fn grid_blocks_deterministic_and_reduce_k() {
        let pres = random_points(300, (0.0, 3.0), (45.0, 47.0), 31);
        let bg = random_points(1000, (0.0, 3.0), (45.0, 47.0), 32);
        let a = spatial_grid_blocks(&pres, &bg, 50.0, 4, &mut Rng::new(2));
        let b = spatial_grid_blocks(&pres, &bg, 50.0, 4, &mut Rng::new(2));
        assert_eq!(a, b);
        // Two presence clusters far apart and huge blocks: only as many folds as occupied blocks.
        let pres = vec![(0.0, 0.0), (0.01, 0.0), (20.0, 20.0)];
        let f = spatial_grid_blocks(&pres, &[(10.0, 10.0)], 500.0, 5, &mut Rng::new(1));
        assert_eq!(f.k, 2);
        assert_eq!(f.presence[0], f.presence[1]);
        assert_ne!(f.presence[0], f.presence[2]);
        assert!(f.background[0] < 2);
    }

    #[test]
    fn grid_blocks_every_fold_has_presences() {
        for seed in 0..20 {
            let pres = random_points(40, (0.0, 10.0), (40.0, 50.0), 100 + seed);
            let f = spatial_grid_blocks(&pres, &[], 150.0, 5, &mut Rng::new(seed));
            for fold in 0..f.k {
                assert!(f.presence.contains(&fold), "seed {seed} fold {fold}");
            }
        }
    }

    /// Smooth random field: sum of ±1 Gaussian bumps of scale `s_km` at random centres.
    fn gaussian_field(pts: &[(f64, f64)], s_km: f64, seed: u64) -> Vec<f64> {
        let centres = random_points(400, (-2.0, 22.0), (38.0, 57.0), seed);
        let mut r = Rng::new(seed + 1);
        let signs: Vec<f64> = (0..centres.len())
            .map(|_| if r.uniform() < 0.5 { -1.0 } else { 1.0 })
            .collect();
        pts.iter()
            .map(|&p| {
                centres
                    .iter()
                    .zip(&signs)
                    .map(|(&c, &s)| {
                        let d = haversine_km(p, c);
                        s * (-d * d / (2.0 * s_km * s_km)).exp()
                    })
                    .sum()
            })
            .collect()
    }

    #[test]
    fn autocorrelation_range_recovers_known_scale() {
        let pts = random_points(1500, (0.0, 20.0), (40.0, 55.0), 41);
        // Covariance of the field ≈ exp(−d²/4s²): 95 % decorrelation at d ≈ 3.46·s.
        let short = gaussian_field(&pts, 50.0, 1);
        let long = gaussian_field(&pts, 150.0, 2);
        let est = autocorrelation_range_km(&pts, &[short, long], &mut Rng::new(1)).expect("fit");
        let r_short = est.per_variable[0].as_ref().expect("short").range_km;
        let r_long = est.per_variable[1].as_ref().expect("long").range_km;
        // Expected ≈ 173 km and ≈ 519 km; observed 177 and 471 (Gaussian model selected).
        assert!((120.0..=230.0).contains(&r_short), "short {r_short}");
        assert!((360.0..=680.0).contains(&r_long), "long {r_long}");
        assert!(r_long > 1.8 * r_short, "{r_short} {r_long}");
        assert!(est.median_km >= r_short && est.median_km <= r_long);
        assert_eq!(est.n_points, 1500);
    }

    #[test]
    fn variogram_fit_recovers_exact_models() {
        let lags: Vec<f64> = (0..15).map(|i| 25.0 + 50.0 * i as f64).collect();
        for model in VariogramModel::ALL {
            let emp: Vec<(f64, f64, usize)> = lags
                .iter()
                .map(|&h| (h, 0.2 + 1.5 * model.shape(h, 300.0), 1000))
                .collect();
            let fit = fit_variogram(&emp, 50.0, 750.0).expect("fit");
            assert_eq!(fit.model, model);
            assert!((fit.range_km - 300.0).abs() < 10.0, "{model:?} {fit:?}");
            assert!((fit.nugget - 0.2).abs() < 0.05 && (fit.partial_sill - 1.5).abs() < 0.05);
            assert!(!fit.at_upper_bound);
        }
        // Pure nugget (no spatial structure) is not a fit.
        let flat: Vec<(f64, f64, usize)> = lags.iter().map(|&h| (h, 1.0, 1000)).collect();
        assert!(fit_variogram(&flat, 50.0, 750.0).is_none());
    }

    #[test]
    fn autocorrelation_constant_and_small_inputs() {
        let pts = random_points(200, (0.0, 5.0), (40.0, 45.0), 51);
        let constant = vec![1.0; 200];
        assert!(autocorrelation_range_km(&pts, &[constant], &mut Rng::new(1)).is_none());
        assert!(autocorrelation_range_km(&pts[..5], &[vec![0.0; 5]], &mut Rng::new(1)).is_none());
    }

    #[test]
    fn autocorrelation_subsamples_deterministically() {
        let pts = random_points(3000, (0.0, 10.0), (40.0, 50.0), 61);
        let v = gaussian_field(&pts, 80.0, 3);
        let a = autocorrelation_range_km(&pts, std::slice::from_ref(&v), &mut Rng::new(4));
        let b = autocorrelation_range_km(&pts, &[v], &mut Rng::new(4));
        assert_eq!(a, b);
        assert_eq!(a.expect("fit").n_points, VARIOGRAM_MAX_POINTS);
    }
}
