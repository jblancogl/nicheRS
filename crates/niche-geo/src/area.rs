//! Exact cell areas of a lon/lat grid on the WGS84 ellipsoid.
//!
//! The area of the quadrangle between latitudes φ1 < φ2 and a longitude span Δλ (radians) is
//! A = Δλ · b²/2 · [q(φ2) − q(φ1)], q(φ) = sinφ/(1 − e² sin²φ) + 1/(2e) · ln((1 + e sinφ)/(1 − e sinφ))
//! (Snyder 1987, "Map Projections — A Working Manual", USGS PP 1395, eq. 3-12 for the authalic
//! latitude). Using exact areas avoids reprojecting predictors to an equal-area grid (ADR-0010).

const A: f64 = 6_378_137.0;
const F: f64 = 1.0 / 298.257_223_563;

fn q(phi: f64) -> f64 {
    let e2 = F * (2.0 - F);
    let e = e2.sqrt();
    let s = phi.sin();
    s / (1.0 - e2 * s * s) + 1.0 / (2.0 * e) * ((1.0 + e * s) / (1.0 - e * s)).ln()
}

/// Area in km² of a cell spanning `dlon_deg` between latitudes `lat1_deg` and `lat2_deg`.
pub fn quad_area_km2(lat1_deg: f64, lat2_deg: f64, dlon_deg: f64) -> f64 {
    let b = A * (1.0 - F);
    let (p1, p2) = (lat1_deg.to_radians(), lat2_deg.to_radians());
    (dlon_deg.to_radians() * b * b / 2.0 * (q(p2) - q(p1))).abs() / 1e6
}

/// Cell area (km²) for each row of a grid.
pub fn row_areas_km2(spec: &crate::grid::GridSpec) -> Vec<f64> {
    (0..spec.height)
        .map(|r| {
            let top = spec.north - r as f64 * spec.cell;
            quad_area_km2(top - spec.cell, top, spec.cell)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_ellipsoid() {
        // WGS84 surface area: 510 065 621.7 km² (e.g. Karney 2013, GeographicLib).
        let total = quad_area_km2(-90.0, 90.0, 360.0);
        assert!((total - 510_065_621.7).abs() < 1.0, "{total}");
    }

    #[test]
    fn one_degree_cell_at_equator() {
        // ≈ 12 308.8 km² (geodesic reference value for a 1°×1° cell at the equator).
        let a = quad_area_km2(0.0, 1.0, 1.0);
        assert!((a - 12_308.8).abs() < 1.0, "{a}");
    }
}
