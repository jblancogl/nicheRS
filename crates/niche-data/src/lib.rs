//! Remote data adapters: HTTP range reads, CHELSA v2.1 predictors, terrain (Copernicus DEM) and
//! land-cover (ESA WorldCover) predictors, GBIF occurrences/taxonomy, and GitLab issues for user
//! feedback.
//!
//! Every function returns provenance alongside data (URLs, counts, licences) so the execution
//! manifest can record exactly what was used (ADR-0203).

mod cellagg;
pub mod chelsa;
pub mod dem;
pub mod gbif;
pub mod gbif_download;
pub mod gbif_mirror;
mod gbif_mirror_cells;
pub mod gitlab;
pub mod http;
pub mod landcover;
pub mod landmask;
pub mod predictors;

pub use http::Http;

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("network error contacting {url}: {msg}")]
    Network { url: String, msg: String },
    #[error("unexpected response from {url}: {msg}")]
    Format { url: String, msg: String },
    #[error("raster error: {0}")]
    Raster(#[from] niche_geo::geotiff::TiffError),
    #[error("cache error: {0}")]
    Cache(String),
    #[error("cancelled")]
    Cancelled,
}
