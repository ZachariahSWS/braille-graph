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
    cfg: &Config,
    bridge: bool,
) -> Result<BraillePlot, GraphError> {
    if v.is_empty() {
        return Err(GraphError::EmptyData);
    }
    let vert_px = cfg.y_chars * BRAILLE_VERTICAL_RESOLUTION;

    // map one point
    let map = |p: &DataTimeStep| -> GraphTimeStep {
        let inv = |y: f64| -> usize {
            let r =
                ((y - cfg.y_min) / (cfg.y_max - cfg.y_min) * (vert_px - 1) as f64).round() as usize;
            (vert_px - 1) - r
        };
        let mut lo = inv(p.min);
        let mut hi = inv(p.max);
        if lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }
        GraphTimeStep { min: lo, max: hi }
    };

    let mut steps: Vec<_> = v.iter().map(map).collect();
    if bridge {
        // Build a *new* vector so each segment only spans (i-1 … i).
        let mut bridged = Vec::with_capacity(steps.len());
        bridged.push(steps[0].clone()); // first point untouched
        for i in 1..steps.len() {
            let prev = &steps[i - 1];
            let cur = &steps[i];
            bridged.push(GraphTimeStep {
                min: min(prev.min, cur.min + 1), // connect prev ↔ cur
                max: max(prev.max, cur.max - 1),
            });
        }
        steps = bridged; // replace
    }
    Ok(BraillePlot { steps })
}
