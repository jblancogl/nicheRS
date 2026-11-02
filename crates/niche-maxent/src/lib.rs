//! NicheRS modelling kernel.
//!
//! Pure domain code: no file system, network, raster or async dependencies. Inputs arrive as
//! in-memory matrices; outputs are models and numbers. Adapters live in other crates.
//!
//! The MaxEnt engine is a port of MaxEnt 3.4.4 (Phillips, Dudík & Schapire; MIT licence)
//! and is validated against `maxent.jar` by the golden tests in `tests/`.

pub mod explain;
pub mod javafmt;
pub mod javarandom;
pub mod javasort;
pub mod model;
pub mod settings;
pub mod train;

pub use explain::{ResponseCurve, permutation_importance, response_curves};
pub use model::{CompiledModel, FeatureKind, MaxentModel, ModelFeature, OutputFormat};
pub use settings::{FeatureClasses, MaxentSettings};
pub use train::{CancelCheck, SwdData, TrainError, TrainReport, VarKind, Variable, train};
