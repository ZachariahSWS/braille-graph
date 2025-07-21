//! Geometry helpers: axis ranges + terminal size plumbing.

use terminal_size::{Height, Width, terminal_size};

use crate::core::{
    constants::{BORDER_WIDTH, BRAILLE_HORIZONTAL_RESOLUTION, LABEL_GUTTER, MIN_GRAPH_HEIGHT},
    data::DataTimeStep,
};

/// Which axis we’re measuring.
pub enum Axis {
    X,
    Y,
}

impl Axis {
    /// Inclusive bounds with ±5 % padding; handles degenerate one-point sets.
    #[must_use]
    pub fn bounds(self, steps: &[DataTimeStep]) -> (f64, f64) {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for s in steps {
            match self {
                Self::X => {
                    lo = lo.min(s.time);
                    hi = hi.max(s.time);
                }
                Self::Y => {
                    lo = lo.min(s.min);
                    hi = hi.max(s.max);
                }
            }
        }
        if !lo.is_finite() || !hi.is_finite() {
            return (0.0, 1.0);
        }
        if (hi - lo).abs() < f64::EPSILON {
            return (lo - 0.5, hi + 0.5);
        }
        let pad = (hi - lo) * 0.05;
        (lo - pad, hi + pad)
    }
}

/// Current terminal geometry (80×30 fallback).
#[inline]
#[must_use]
pub fn terminal_geometry() -> (Width, Height) {
    terminal_size().unwrap_or((Width(80), Height(30)))
}

/// Convert terminal dimensions + sample count to graph char grid.
/// Leaves space for borders + labels.
#[inline]
#[must_use]
pub fn graph_dims((w, h): (Width, Height), samples: usize) -> (usize, usize) {
    let x_chars = std::cmp::min(
        samples / BRAILLE_HORIZONTAL_RESOLUTION,
        (w.0 as usize).saturating_sub(BORDER_WIDTH + LABEL_GUTTER + 1),
    );
    let y_chars = std::cmp::max(MIN_GRAPH_HEIGHT, (h.0 as usize).saturating_sub(4));
    (x_chars, y_chars)
}

/// How wide will the y-axis labels be for *current* min/max?
#[inline]
pub fn y_label_width(y_min: f64, y_max: f64, decimals: usize) -> usize {
    use std::fmt::Write;
    let mut s = String::new();
    write!(&mut s, "{y_min:.decimals$}").unwrap();
    let lo = s.len();
    s.clear();
    write!(&mut s, "{y_max:.decimals$}").unwrap();
    lo.max(s.len())
}
