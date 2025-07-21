//! Aggregates the “business logic” layer.

pub mod bounds;
pub mod color;
pub mod config;
pub mod constants;
pub mod data;
pub mod error;
pub mod rng;

// re-export frequently-used items for convenience
pub use bounds::Axis;
pub use color::{AnsiCode, ColorError, colorize};
pub use config::{Config, ConfigBuilder};
pub use constants::{
    BORDER_WIDTH, BRAILLE_HORIZONTAL_RESOLUTION, DECIMAL_PRECISION, LABEL_GUTTER, MIN_GRAPH_HEIGHT,
    MIN_GRAPH_WIDTH,
};
pub use data::DataTimeStep;
pub use error::{ConfigError, GraphError};
