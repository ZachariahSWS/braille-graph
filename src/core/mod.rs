//! Aggregates the “business logic” layer.

pub mod bounds;
pub mod color;
pub mod config;
pub mod data;
pub mod error;
pub mod rng;

// re-export frequently-used items for convenience
pub use bounds::Axis;
pub use color::{AnsiCode, ColorError, colorize};
pub use config::{Config, ConfigBuilder};
pub use data::{DECIMAL_PRECISION, DataTimeStep};
pub use error::{ConfigError, GraphError};
