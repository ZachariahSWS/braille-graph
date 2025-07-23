//! Public-facing crate root – re-exports + one-shot helper.

pub mod cli;
pub mod core;
pub mod render;

pub use core::{
    color::{AnsiCode, ColorError, colorize},
    config::{Config, ConfigBuilder},
    constants::{DECIMAL_PRECISION, MIN_GRAPH_HEIGHT, MIN_GRAPH_WIDTH},
    data::DataTimeStep,
    error::{ConfigError, GraphError},
};

pub use render::{Binner, Renderer, Strategy, preprocess_to_braille};
