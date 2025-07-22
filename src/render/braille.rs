//! Turns raw data into character and subcharacter positioning with braille.

use crate::core::{
    config::Config, constants::BRAILLE_VERTICAL_RESOLUTION, data::DataTimeStep, error::GraphError,
};

/// After `filter_and_bin` the *logical* per-timestep min/max are mapped to these
/// vertical pixel coordinates.
#[derive(Clone)]
pub struct GraphTimeStep {
    pub min: usize,
    pub max: usize,
}

pub struct BraillePlot {
    pub steps: Vec<GraphTimeStep>,
}

/// Convert `DataTimeStep` list into pixel coordinates (+ optional bridging).
///
/// Safety-critical invariants:
/// * `GraphTimeStep::min  <= GraphTimeStep::max`
/// * both are in `[0 , vert_px-1]` inclusive
pub fn preprocess_to_braille(
    v: &[DataTimeStep],
    config: &Config,
    bridge: bool,
) -> Result<BraillePlot, GraphError> {
    if v.is_empty() {
        return Err(GraphError::EmptyData);
    }

    let vert_px = config.y_chars * BRAILLE_VERTICAL_RESOLUTION;
    let y_span = config.y_max - config.y_min; // cfg validated: > 0

    // Robust mapping λ(y): ℝ → [0 , vert_px-1]
    let inv = |y: f64| -> usize {
        // Normalise into [0,1]  (values slightly outside due to float error
        // or user-supplied y_min/y_max are gracefully clamped).
        let ratio = ((y - config.y_min) / y_span).max(0.0).min(1.0);

        // Scale to pixel grid, round to nearest integer, then invert so
        // logical “top” (y_max) maps to row 0.
        let r = (ratio * (vert_px - 1) as f64).round() as usize;
        (vert_px - 1) - r
    };

    // Initial point-wise mapping
    let mut steps: Vec<GraphTimeStep> = v
        .iter()
        .map(|p| {
            let mut low = inv(p.min);
            let mut high = inv(p.max);
            if low > high {
                std::mem::swap(&mut low, &mut high);
            }
            GraphTimeStep {
                min: low,
                max: high,
            }
        })
        .collect();

    // Optional min/max “bridging” pass
    if bridge {
        let mut bridged = Vec::with_capacity(steps.len());
        bridged.push(steps[0].clone()); // first point unchanged
        for i in 1..steps.len() {
            let prev = &steps[i - 1];
            let curr = &steps[i];
            bridged.push(GraphTimeStep {
                min: prev.min.min(curr.min + 1), // +1 so lines touch
                max: prev.max.max(curr.max.saturating_sub(1)),
            });
        }
        steps = bridged;
    }

    Ok(BraillePlot { steps })
}
