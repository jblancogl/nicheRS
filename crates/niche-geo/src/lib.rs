//! Geospatial primitives for NicheRS without GDAL/PROJ (ADR-0003).
//!
//! Scope is deliberately narrow: geographic (EPSG:4326) regular grids — the native grid of the
//! default predictors (CHELSA v2.1, 30 arc-seconds) — so that no reprojection or resampling of
//! predictors is needed. Areas are computed exactly on the WGS84 ellipsoid instead of on a
//! projected grid (ADR-0010). [`crs`] adds PROJ-free transforms between WGS84 and the projected
//! CRSs used in France and Spain (Lambert-93, LAEA Europe, Web Mercator, UTM 28–33N);
//! [`distance`] computes metric distance transforms on geographic grids; [`overlay`] the exact
//! ellipsoidal area of polygons and of their overlap with grid cells.

pub mod area;
pub mod boundaries;
pub mod colormap;
pub mod crs;
pub mod distance;
pub mod geotiff;
pub mod grid;
pub mod mask;
pub mod overlay;
pub mod tiles;
pub mod write;

pub use grid::{GridSpec, Raster};
