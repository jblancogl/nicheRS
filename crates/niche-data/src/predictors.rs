//! Non-climatic predictors on the native CHELSA 30″ grid: terrain (Copernicus DEM GLO-90), land
//! cover fractions (ESA WorldCover 2021) and distance to water.
//!
//! One entry point, [`read_window`], with the same window convention as
//! [`crate::chelsa::read_window`] (row0, col0, rows, cols on the native CHELSA grid), so the
//! application can stack these layers with the climate layers without resampling (ADR-0010).
//! Metadata mirror [`crate::chelsa::LAYERS`] ([`LayerDef`]) plus per-layer provenance
//! ([`provenance`]). None is selected by default: making any of them a default needs an ADR
//! amending ADR-0009 after ecologists' review.

use std::path::Path;

use niche_geo::distance::distance_to_mask_km;
use niche_geo::{GridSpec, Raster, crs::metres_per_degree};

use crate::chelsa::LayerDef;
use crate::http::Http;
use crate::{DataError, chelsa, dem, landcover};

/// Distance to water is truncated at this value (km). With the window padded by at least this
/// distance, every value is exact (up to the local metric approximation) and independent of the
/// window: a cell whose nearest water is farther gets exactly `DIST_WATER_MAX_KM`.
pub const DIST_WATER_MAX_KM: f64 = 25.0;
/// A cell counts as water when its WorldCover permanent-water fraction is at least this.
pub const WATER_FRACTION_THRESHOLD: f32 = 0.5;

const TEMPORAL: &str = "Land cover of 2021 (ESA WorldCover v200) combined with the 1981–2010 climate baseline and occurrences since 1981: temporal mismatch; land use may have changed at older records.";
const BUILT: &str = "Land cover of 2021 vs 1981–2010 climate baseline: temporal mismatch. Built-up area is strongly correlated with observer effort in opportunistic records (sampling bias): a positive response may reflect observers, not the species.";
const DSM: &str = "Copernicus DEM is a surface model (DSM): heights include canopy and buildings.";
const DIST: &str = "Distance (truncated at 25 km) to the nearest 30″ cell with ≥ 50 % permanent water in ESA WorldCover 2021 (lakes, reservoirs, large rivers and the sea; narrow rivers are not resolved at 1 km). Temporal mismatch with the 1981–2010 climate baseline.";

pub const LAYERS: &[LayerDef] = &[
    LayerDef {
        id: "dem_elevation",
        short: "ELEV",
        name: "Elevation (mean of 90 m DEM)",
        unit: "m",
        default_selected: false,
        caution: Some(DSM),
    },
    LayerDef {
        id: "dem_slope",
        short: "SLOPE",
        name: "Slope (mean of 90 m slopes, Horn)",
        unit: "°",
        default_selected: false,
        caution: Some(DSM),
    },
    LayerDef {
        id: "dem_roughness",
        short: "ROUGH",
        name: "Terrain roughness (SD of 90 m elevations)",
        unit: "m",
        default_selected: false,
        caution: Some(DSM),
    },
    LayerDef {
        id: "lc_tree",
        short: "TREE",
        name: "Tree cover fraction",
        unit: "fraction 0–1",
        default_selected: false,
        caution: Some(TEMPORAL),
    },
    LayerDef {
        id: "lc_shrub",
        short: "SHRUB",
        name: "Shrubland fraction",
        unit: "fraction 0–1",
        default_selected: false,
        caution: Some(TEMPORAL),
    },
    LayerDef {
        id: "lc_grass",
        short: "GRASS",
        name: "Grassland fraction",
        unit: "fraction 0–1",
        default_selected: false,
        caution: Some(TEMPORAL),
    },
    LayerDef {
        id: "lc_crop",
        short: "CROP",
        name: "Cropland fraction",
        unit: "fraction 0–1",
        default_selected: false,
        caution: Some(TEMPORAL),
    },
    LayerDef {
        id: "lc_built",
        short: "BUILT",
        name: "Built-up fraction",
        unit: "fraction 0–1",
        default_selected: false,
        caution: Some(BUILT),
    },
    LayerDef {
        id: "lc_bare",
        short: "BARE",
        name: "Bare, sparse, snow/ice and moss/lichen fraction",
        unit: "fraction 0–1",
        default_selected: false,
        caution: Some(TEMPORAL),
    },
    LayerDef {
        id: "lc_water",
        short: "WATER",
        name: "Permanent water fraction",
        unit: "fraction 0–1",
        default_selected: false,
        caution: Some(TEMPORAL),
    },
    LayerDef {
        id: "lc_wetland",
        short: "WETL",
        name: "Herbaceous wetland and mangrove fraction",
        unit: "fraction 0–1",
        default_selected: false,
        caution: Some(TEMPORAL),
    },
    LayerDef {
        id: "dist_water",
        short: "DWAT",
        name: "Distance to permanent water (truncated at 25 km)",
        unit: "km",
        default_selected: false,
        caution: Some(DIST),
    },
];

/// Where a layer comes from, for the execution manifest and report attributions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provenance {
    pub source: &'static str,
    /// Reference year(s) of the data.
    pub vintage: &'static str,
    pub licence: &'static str,
    pub licence_url: &'static str,
    pub citation: &'static str,
    /// Notice that must accompany any published output using the layer.
    pub attribution: &'static str,
    /// How the 30″ value is derived from the source.
    pub method: &'static str,
}

const DEM_PROV: Provenance = Provenance {
    source: dem::SOURCE,
    vintage: "2011–2015 (TanDEM-X acquisitions)",
    licence: dem::LICENCE,
    licence_url: dem::LICENCE_URL,
    citation: dem::CITATION,
    attribution: dem::ATTRIBUTION,
    method: "",
};

const LC_PROV: Provenance = Provenance {
    source: landcover::SOURCE,
    vintage: "2021",
    licence: landcover::LICENCE,
    licence_url: landcover::LICENCE_URL,
    citation: landcover::CITATION,
    attribution: landcover::ATTRIBUTION,
    method: "fraction of valid 10 m pixels (centre in cell) of the class group; NoData if < 50 % valid",
};

pub fn layer(id: &str) -> Option<&'static LayerDef> {
    LAYERS.iter().find(|l| l.id == id)
}

pub fn provenance(id: &str) -> Option<Provenance> {
    Some(match id {
        "dem_elevation" => Provenance {
            method: "mean of 3″ heights (centre in cell); NoData if < 50 % valid",
            ..DEM_PROV
        },
        "dem_slope" => Provenance {
            method: "mean of 3″ slopes (Horn 1981, metric pixel sizes per latitude), computed before aggregation",
            ..DEM_PROV
        },
        "dem_roughness" => Provenance {
            method: "standard deviation of 3″ heights in the cell",
            ..DEM_PROV
        },
        "dist_water" => Provenance {
            method: "exact Euclidean distance transform (locally metric) to cells with water fraction ≥ 0.5, window padded by 25 km, truncated at 25 km",
            ..LC_PROV
        },
        id if id.starts_with("lc_") && layer(id).is_some() => LC_PROV,
        _ => return None,
    })
}

/// Read a predictor on a native CHELSA window (row0, col0, rows, cols).
pub fn read_window(
    http: &Http,
    cache: &Path,
    id: &str,
    window: (usize, usize, usize, usize),
) -> Result<Raster, DataError> {
    let grid = chelsa::native_grid(http, cache)?;
    read_window_on_grid(http, cache, &grid, id, window)
}

/// [`read_window`] with an explicit native grid.
pub fn read_window_on_grid(
    http: &Http,
    cache: &Path,
    grid: &GridSpec,
    id: &str,
    window: (usize, usize, usize, usize),
) -> Result<Raster, DataError> {
    let (r0, c0, rows, cols) = window;
    if rows == 0 || cols == 0 || r0 + rows > grid.height || c0 + cols > grid.width {
        return Err(DataError::Format {
            url: id.into(),
            msg: "window outside the native grid".into(),
        });
    }
    match id {
        "dem_elevation" => dem::read_window(http, cache, grid, window, dem::DemLayer::Elevation),
        "dem_slope" => dem::read_window(http, cache, grid, window, dem::DemLayer::Slope),
        "dem_roughness" => dem::read_window(http, cache, grid, window, dem::DemLayer::Roughness),
        "dist_water" => {
            let padded = padded_window(grid, window, DIST_WATER_MAX_KM);
            let water = landcover::read_window(http, cache, grid, padded, "water")?;
            Ok(distance_to_water(&water, padded, window, DIST_WATER_MAX_KM))
        }
        _ => match id.strip_prefix("lc_") {
            Some(g) if landcover::group_index(g).is_some() => {
                landcover::read_window(http, cache, grid, window, g)
            }
            _ => Err(DataError::Format {
                url: id.into(),
                msg: "unknown predictor".into(),
            }),
        },
    }
}

/// The window grown by at least `km` on every side (clipped to the grid).
pub fn padded_window(
    grid: &GridSpec,
    window: (usize, usize, usize, usize),
    km: f64,
) -> (usize, usize, usize, usize) {
    let (r0, c0, rows, cols) = window;
    let north = grid.north - r0 as f64 * grid.cell;
    let south = north - rows as f64 * grid.cell;
    let pad_deg_lat = km * 1000.0 / metres_per_degree(0.0).1; // ρ is smallest at the equator
    let pr = (pad_deg_lat / grid.cell).ceil() as usize + 1;
    // Highest |latitude| reached by the padded window: smallest parallel scale.
    let max_lat = (north + pad_deg_lat)
        .abs()
        .max((south - pad_deg_lat).abs())
        .min(89.0);
    let pc = (km * 1000.0 / (metres_per_degree(max_lat).0 * grid.cell)).ceil() as usize + 1;
    let nr0 = r0.saturating_sub(pr);
    let nc0 = c0.saturating_sub(pc);
    let nr1 = (r0 + rows + pr).min(grid.height);
    let nc1 = (c0 + cols + pc).min(grid.width);
    (nr0, nc0, nr1 - nr0, nc1 - nc0)
}

/// Distance (km, truncated at `max_km`) to the nearest cell with water fraction ≥ 0.5, computed
/// on the padded water raster and cropped to `window`. NaN where the water fraction is unknown.
pub fn distance_to_water(
    water: &Raster,
    padded: (usize, usize, usize, usize),
    window: (usize, usize, usize, usize),
    max_km: f64,
) -> Raster {
    let mask: Vec<bool> = water
        .data
        .iter()
        .map(|&v| v >= WATER_FRACTION_THRESHOLD)
        .collect();
    let d = distance_to_mask_km(&water.spec, &mask);
    let (pr0, pc0, _, _) = padded;
    let (r0, c0, rows, cols) = window;
    let spec = GridSpec {
        west: water.spec.west + (c0 - pc0) as f64 * water.spec.cell,
        north: water.spec.north - (r0 - pr0) as f64 * water.spec.cell,
        cell: water.spec.cell,
        width: cols,
        height: rows,
    };
    let mut out = Raster::new(spec, f32::NAN);
    for r in 0..rows {
        for c in 0..cols {
            let k = water.spec.index(r + r0 - pr0, c + c0 - pc0);
            if !water.data[k].is_nan() {
                out.data[r * cols + c] = f64::from(d[k]).min(max_km) as f32;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_layer_has_provenance_and_is_not_default() {
        for l in LAYERS {
            assert!(!l.default_selected, "{}", l.id);
            let p = provenance(l.id).expect("provenance");
            assert!(
                !p.attribution.is_empty() && !p.method.is_empty(),
                "{}",
                l.id
            );
        }
        assert!(provenance("chelsa_bio1").is_none());
        assert!(layer("lc_water").is_some());
    }

    #[test]
    fn padding_covers_the_truncation_distance() {
        let grid = GridSpec {
            west: -180.000_138_889,
            north: 83.999_860_415,
            cell: 1.0 / 120.0,
            width: 43_200,
            height: 20_880,
        };
        let w = (4800, 22_200, 100, 100);
        let p = padded_window(&grid, w, 25.0);
        // ≥ 25 km north–south (~0.926 km per row) and east–west (~0.6 km per column at 44° N).
        assert!(w.0 - p.0 >= 27 && p.2 - w.2 - (w.0 - p.0) >= 27, "{p:?}");
        assert!(w.1 - p.1 >= 37, "{p:?}");
        assert_eq!(padded_window(&grid, (0, 0, 10, 10), 25.0).0, 0);
    }

    /// Water in the padding is seen; values are truncated; unknown cells stay NaN.
    #[test]
    fn distance_uses_padding_and_truncates() {
        let spec = GridSpec {
            west: 5.0,
            north: 44.0,
            cell: 1.0 / 120.0,
            width: 200,
            height: 120,
        };
        let mut water = Raster::new(spec, 0.0);
        water.data[spec.index(60, 20)] = 1.0; // only water: in the western padding
        water.data[spec.index(61, 150)] = f32::NAN;
        let padded = (1000, 2000, 120, 200);
        let window = (1010, 2050, 100, 120);
        let d = distance_to_water(&water, padded, window, 25.0);
        assert_eq!((d.spec.width, d.spec.height), (120, 100));
        // Window cell (50, 0) is 30 columns east of the water cell at the same row: 30 × ~0.67 km.
        let v = d.get(50, 0);
        let want = 30.0 * metres_per_degree(43.5).0 / 120.0 / 1000.0;
        assert!((f64::from(v) - want).abs() < 0.05, "{v} vs {want}");
        assert_eq!(d.get(0, 119), 25.0);
        assert!(d.get(51, 100).is_nan());
        assert!((d.spec.west - (5.0 + 50.0 / 120.0)).abs() < 1e-12);
    }
}
