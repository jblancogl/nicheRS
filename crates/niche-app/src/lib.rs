//! NicheRS application layer: use cases (preview data, run jobs, read results), job lifecycle,
//! persistence and manifests. Used by both the HTTP server and the CLI (same behaviour in every
//! deployment mode).

pub mod accounts;
pub mod analyses;
pub mod artifacts;
pub mod audit;
pub mod bias;
pub mod checkpoint;
pub mod demo;
pub mod dto;
pub mod engine;
pub mod env;
pub mod feedback;
pub mod fit;
pub mod gbif_accounts;
pub mod independent;
pub mod lease;
pub mod owndata;
pub mod projection;
pub mod projects;
pub mod report_html;
pub mod secrets;
pub mod store;
pub mod system;

pub use engine::{Config, Engine, Limits, MirrorConfig};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("data source problem: {0}")]
    Data(#[from] niche_data::DataError),
    #[error("model fitting failed: {0}")]
    Model(String),
    #[error("storage error: {0}")]
    Storage(String),
    /// The request conflicts with the current state (e.g. deleting a running job).
    #[error("{0}")]
    Conflict(String),
    /// Signed in, but not allowed (administration pages for non-administrators).
    #[error("{0}")]
    Forbidden(String),
    /// The organisation's monthly quota of modelled species would be exceeded.
    #[error("{0}")]
    QuotaExceeded(String),
    #[error("cancelled")]
    Cancelled,
    #[error("internal error: {0}")]
    Internal(String),
}
