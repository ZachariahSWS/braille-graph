//! Public-facing crate root – re-exports + one-shot helper.

pub mod cli;
pub mod core;
pub mod render;

pub use core::{
    color::{AnsiCode, ColorError, colorize},
    config::{Config, ConfigBuilder},
    data::{DECIMAL_PRECISION, DataTimeStep},
    error::{ConfigError, GraphError},
};

pub use render::{
    MIN_GRAPH_HEIGHT, MIN_GRAPH_WIDTH, Renderer, filter_and_bin, preprocess_to_braille,
};

/// Convenience function kept for backwards compatibility.  Plots a **static**
/// in-memory data set with automatic axis scaling.
pub fn plot_data(
    mut data: Vec<DataTimeStep>,
    title: &str,
    color: &str,
    cumulative: bool,
) -> Result<(), GraphError> {
    use core::bounds::{Axis, graph_dims, terminal_geometry};
    use render::{filter_and_bin, preprocess_to_braille};

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

    data = filter_and_bin(data, &cfg);
    let plot = preprocess_to_braille(&data, &cfg, cumulative)?;
    Renderer::full().render(&cfg, &plot)
}
