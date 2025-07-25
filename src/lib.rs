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

/// Convenience function kept for backwards compatibility.  Plots a **static**
/// in-memory data set with automatic axis scaling.
pub fn plot_data(
    data: Vec<DataTimeStep>,
    title: &str,
    color: AnsiCode,
    cumulative: bool,
) -> Result<(), GraphError> {
    use core::bounds::{Axis, graph_dims, terminal_geometry};

    if data.is_empty() {
        return Err(GraphError::EmptyData);
    }

    let (y_min, y_max) = Axis::Y.bounds(&data);
    let (x_min, x_max) = Axis::X.bounds(&data);

    let term = terminal_geometry();
    let (x_chars, y_chars) = graph_dims(term, data.len());

    let cfg = Config::builder(x_chars, y_chars)
        .title(title)
        .color(color)
        .y_range(y_min..=y_max)
        .x_range(x_min, x_max)
        .build()?;

    let mut binner = Binner::new(Strategy::Index);
    let binned = binner.bin(&data, &cfg);
    let plot = preprocess_to_braille(&binned, &cfg, cumulative)?;
    Renderer::full().render(&cfg, &plot)
}
