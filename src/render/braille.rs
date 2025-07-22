//! Turns raw data into character and subcharacter positioning with braille.

use std::cmp::{max, min};

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

/// Convert `DataTimeStep` list into pixel coordinates + optional bridging.
pub fn preprocess_to_braille(
    v: &[DataTimeStep],
    config: &Config,
    bridge: bool,
) -> Result<BraillePlot, GraphError> {
    if v.is_empty() {
        return Err(GraphError::EmptyData);
    }
    let vert_px = config.y_chars * BRAILLE_VERTICAL_RESOLUTION;

    // map one point
    let map = |p: &DataTimeStep| -> GraphTimeStep {
        let inv = |y: f64| -> usize {
            let r = ((y - config.y_min) / (config.y_max - config.y_min) * (vert_px - 1) as f64)
                .round() as usize;
            (vert_px - 1) - r
        };
        let mut low = inv(p.min);
        let mut high = inv(p.max);
        if low > high {
            std::mem::swap(&mut low, &mut high);
        }
        GraphTimeStep {
            min: low,
            max: high,
        }
    };

    let mut steps: Vec<_> = v.iter().map(map).collect();
    if bridge {
        // Build a new vector so each segment only spans (i-1 ... i).
        let mut bridged = Vec::with_capacity(steps.len());
        bridged.push(steps[0].clone()); // first point untouched
        for i in 1..steps.len() {
            let previous = &steps[i - 1];
            let current = &steps[i];
            bridged.push(GraphTimeStep {
                min: min(previous.min, current.min + 1), // connect prev to current
                max: max(previous.max, current.max - 1),
            });
        }
        steps = bridged;
    }

    Ok(BraillePlot { steps })
}
