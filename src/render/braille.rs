//! Numeric series to UTF-8 braille grid, zero intermediate buffers.
//!
//! ### Workflow
//! 1. `preprocess_to_braille` maps raw `DataTimeStep` into pixel-space
//!     extrema (`GraphTimeStep`), one entry per *half* column.
//! 2. `encode_braille` fills a caller-supplied buffer laid out row-major
//!    with exactly three bytes per character cell.  Every braille scalar
//!    U+2800..U+28FF encodes to the fixed pattern
//!    `E2 A0/.. A0+((mask>>6)&3)  80|mask&0x3F`, so we can write bytes
//!    directly without `char::encode_utf8` or any temporary “mask” array.
//!
//! The implementation relies on an invariant: the intersection of a
//! contiguous vertical range with a 4-pixel braille cell is itself one of
//! 11 canonical patterns --- triangle number (representing full, top and bottom contiguous triplet, top, bottom and middle contiguous pair and individual dots) plus one for the empty state
//! We pre-compute the bit-mask for each pattern for both the left and
//! right half-columns and index into those tables at run-time.

use crate::core::{
    config::Config, constants::BRAILLE_VERTICAL_RESOLUTION, data::DataTimeStep, error::GraphError,
};

/// Pixel-space min/max inside one half-column.
#[derive(Clone)]
pub struct GraphTimeStep {
    pub min: usize,
    pub max: usize,
}

pub struct BraillePlot {
    pub steps: Vec<GraphTimeStep>,
}

// --- Pre-Computed Masks ---

/// Pattern enumeration (11 entries):
///
/// 0 empty (⠀), 1 full (⡇), 2 top-three (⠇), 3 bottom-three(⡆), 4 top-two (⠃), 5 middle-two (⠆), 6 bottom-two (⡄),
/// 7 dot-zero (⠁), 8 dot-one (⠂), 9 dot-two (⠄), 10 dot-three (⡀)
const LEFT_MASKS: [u8; 11] = [
    0x00, 0x47, 0x07, 0x46, 0x03, 0x06, 0x44, 0x01, 0x02, 0x04, 0x40,
];
/// Pattern enumeration (11 entries):
///
/// 0 empty (⠀), 1 full (⢸), 2 top-three (⠸), 3 bottom-three(⢰), 4 top-two (⠘), 5 middle-two (⠰), 6 bottom-two (⢠),
/// 7 dot-zero (⠈), 8 dot-one (⠐), 9 dot-two (⠠), 10 dot-three (⢀)
const RIGHT_MASKS: [u8; 11] = [
    0x00, 0xB8, 0x38, 0xB0, 0x18, 0x30, 0xA0, 0x08, 0x10, 0x20, 0x80,
];

/// Map `(low, high)` --- pixel offsets inside a 4-row cell --- to the pattern id.
#[inline]
const fn pattern_id(low: usize, high: usize) -> usize {
    match (low, high) {
        (0, 3) => 1,  // full
        (0, 2) => 2,  // top-3
        (1, 3) => 3,  // bottom-3
        (0, 1) => 4,  // top-2
        (1, 2) => 5,  // middle-2
        (2, 3) => 6,  // bottom-2
        (0, 0) => 7,  // single-0
        (1, 1) => 8,  // single-1
        (2, 2) => 9,  // single-2
        (3, 3) => 10, // single-3
        _ => 0,       // empty / no overlap
    }
}

pub fn preprocess_to_braille(
    v: &[DataTimeStep],
    config: &Config,
    bridge: bool,
) -> Result<BraillePlot, GraphError> {
    if v.is_empty() {
        return Err(GraphError::EmptyData);
    }

    let vert_px = config.y_chars * BRAILLE_VERTICAL_RESOLUTION;
    let y_span = config.y_max - config.y_min; // > 0 by construction

    // λ : ℝ → [0,vert_px-1]
    let map = |y: f64| -> usize {
        let r = ((y - config.y_min) / y_span).clamp(0.0, 1.0) * (vert_px - 1) as f64;
        (vert_px - 1) - r.round() as usize
    };

    let mut steps: Vec<GraphTimeStep> = v
        .iter()
        .map(|p| {
            let (mut lo, mut hi) = (map(p.min), map(p.max));
            if lo > hi {
                std::mem::swap(&mut lo, &mut hi);
            }
            GraphTimeStep { min: lo, max: hi }
        })
        .collect();

    if bridge {
        let mut bridged = Vec::with_capacity(steps.len());
        bridged.push(steps[0].clone());
        for i in 1..steps.len() {
            let prev = &steps[i - 1];
            let curr = &steps[i];
            bridged.push(GraphTimeStep {
                min: prev.min.min(curr.min + 1),
                max: prev.max.max(curr.max.saturating_sub(1)),
            });
        }
        steps = bridged;
    }

    Ok(BraillePlot { steps })
}

/// Encode `plot` straight into `buf`, which is the full frame buffer.
///
/// * `offset` -- byte index of the first braille cell (row 0, col 0)
/// * `row_stride` -- bytes between successive graph rows in `buf`
pub fn encode_braille_into_frame(
    buf: &mut [u8],
    offset: usize,
    row_stride: usize,
    plot: &BraillePlot,
    x_chars: usize,
    y_chars: usize,
) {
    debug_assert!(
        buf.len() >= offset + row_stride * y_chars,
        "frame buffer too small"
    );

    // Iterate row-major for straightforward pointer math.
    for row in 0..y_chars {
        let row_top = row * BRAILLE_VERTICAL_RESOLUTION;
        let row_bottom = row_top + 3;
        let row_base = offset + row * row_stride;

        for col in 0..x_chars {
            // Left half-column
            let left_index = col * 2;

            let left_pattern = plot
                .steps
                .get(left_index)
                .and_then(|s| {
                    if s.max < row_top || s.min > row_bottom {
                        None
                    } else {
                        Some(pattern_id(
                            s.min.max(row_top) - row_top,
                            s.max.min(row_bottom) - row_top,
                        ))
                    }
                })
                .unwrap_or(0);

            // Right half-column
            let right_pattern = plot
                .steps
                .get(left_index + 1)
                .and_then(|s| {
                    if s.max < row_top || s.min > row_bottom {
                        None
                    } else {
                        Some(pattern_id(
                            s.min.max(row_top) - row_top,
                            s.max.min(row_bottom) - row_top,
                        ))
                    }
                })
                .unwrap_or(0);

            // Combine masks and write three UTF-8 bytes directly.
            // https://en.wikipedia.org/wiki/Braille_Patterns
            let mask = LEFT_MASKS[left_pattern] | RIGHT_MASKS[right_pattern];
            let cell = row_base + col * 3;
            buf[cell] = 0xE2;
            // Bitwise or the second byte with the most significant two bits
            // Represents the nonstandard bottom left and right dots
            buf[cell + 1] = 0xA0 | ((mask >> 6) & 0x03);
            // Bitwise or the third byte with the least significant six bits
            // Represents the normal six dots
            buf[cell + 2] = 0x80 | (mask & 0x3F);
        }
    }
}
