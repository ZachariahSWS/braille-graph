//! Geometry helpers: axis ranges + terminal size plumbing.

use terminal_size::{Height, Width, terminal_size};

use crate::core::{
    constants::{
        BORDER_WIDTH, BRAILLE_HORIZONTAL_RESOLUTION as HR, LABEL_GUTTER, MIN_GRAPH_HEIGHT,
    },
    data::DataTimeStep,
};

/// Which axis we’re measuring.
pub enum Axis {
    X,
    Y,
}

impl Axis {
    /// Inclusive bounds without any padding.
    ///
    /// * If the series is empty or contains only non-finite values the
    ///   fallback is `(0.0, 1.0)`.
    /// * If *all* finite points are identical we expand by +-0.5 so the graph
    ///   still has non-zero height/width.
    #[must_use]
    pub fn bounds(self, steps: &[DataTimeStep]) -> (f64, f64) {
        let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);

        for s in steps {
            match self {
                Self::X => {
                    low = low.min(s.time);
                    high = high.max(s.time);
                }
                Self::Y => {
                    low = low.min(s.min);
                    high = high.max(s.max);
                }
            }
        }

        // All points were non-finite or there were none at all.
        if !low.is_finite() || !high.is_finite() {
            return (0.0, 1.0);
        }

        // Degenerate (flat-line) series - give it some breathing room.
        if (high - low).abs() < f64::EPSILON {
            return (low - 0.5, high + 0.5);
        }

        // Normal case: exact extrema, no additional padding.
        (low, high)
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
        samples.div_ceil(HR),
        (w.0 as usize).saturating_sub(BORDER_WIDTH + LABEL_GUTTER + 1),
    );
    let y_chars = std::cmp::max(MIN_GRAPH_HEIGHT, usize::from(h.0).saturating_sub(5));
    (x_chars, y_chars)
}

/// How wide will the y-axis labels be for *current* min/max?
#[inline]
#[must_use]
pub fn y_label_width(y_range: (f64, f64), decimals: usize) -> usize {
    let (low, high) = y_range;
    use std::fmt::Write;
    let mut s = String::new();
    write!(&mut s, "{low:.decimals$}").unwrap();
    let lo = s.len();
    s.clear();
    write!(&mut s, "{high:.decimals$}").unwrap();
    lo.max(s.len())
}
