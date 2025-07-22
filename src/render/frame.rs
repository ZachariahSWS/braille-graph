//! Build a full-screen braille frame and flush to the terminal.

use std::io::{Write, stdout};

use crate::{
    core::{
        color::{AnsiCode, colorize},
        config::Config,
        constants::{
            BORDER_WIDTH, DECIMAL_PRECISION, LABEL_GUTTER, MIN_GRAPH_HEIGHT, MIN_GRAPH_WIDTH,
        },
        error::GraphError,
    },
    render::braille::{BraillePlot, GraphTimeStep},
};

// Public constants (re-exported by braille.rs)
pub(crate) const BRAILLE_DOT_POSITIONS: [[u8; 4]; 2] = [
    [0, 1, 2, 6], // left  column: dots 1,2,3,7
    [3, 4, 5, 7], // right column: dots 4,5,6,8
];
pub(crate) const BRAILLE_UNICODE_BASE: u32 = 0x2800;

// Layout constants

/// Two spaces in front, one space behind
const TITLE_PADDING: usize = 3;

// Box-drawing glyphs
const TL: &str = "┌";
const TR: &str = "┐";
const BL: &str = "└";
const BR: &str = "┘";
const H: &str = "─";
const V: &str = "│";

// Utilities
#[inline]
fn hash64(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;
    let mut h = FNV_OFFSET;
    for &b in s.as_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Write centred colored text between horizontal rules.
fn push_centered(buf: &mut String, text: &str, width: usize, color: &AnsiCode) {
    let inner = width.saturating_sub(TITLE_PADDING);
    let len = text.chars().count();
    if len == 0 || len > inner {
        buf.push_str(&H.repeat(width));
        return;
    }
    let pad_left = (inner - len) / 2;
    let pad_right = inner - len - pad_left;

    buf.push_str(&H.repeat(pad_left));
    buf.push_str("  "); // 2-char left padding
    buf.push_str(&colorize(color, text));
    buf.push(' '); // 1-char right padding
    buf.push_str(&H.repeat(pad_right));
}

/// Map two half-columns at (`char_idx`,`row`) to a single Unicode braille scalar.
#[inline]
fn braille_char(char_idx: usize, row: usize, plot: &BraillePlot) -> char {
    let left = char_idx * 2;
    let right = left + 1;
    let base_y = row * 4;
    let mut mask = 0u8;

    let mut stamp = |step: &GraphTimeStep, col: usize| {
        for y in 0..4 {
            let g = base_y + y;
            if g >= step.min && g <= step.max {
                mask |= 1 << BRAILLE_DOT_POSITIONS[col][y];
            }
        }
    };

    if let Some(s) = plot.steps.get(left) {
        stamp(s, 0);
    }
    if let Some(s) = plot.steps.get(right) {
        stamp(s, 1);
    }

    char::from_u32(BRAILLE_UNICODE_BASE + u32::from(mask)).unwrap()
}

/// Render a complete frame into a single `String`.
pub fn build_frame(cfg: &Config, plot: &BraillePlot) -> Result<String, GraphError> {
    if cfg.x_chars < MIN_GRAPH_WIDTH || cfg.y_chars < MIN_GRAPH_HEIGHT {
        return Err(GraphError::GraphTooSmall {
            want_w: MIN_GRAPH_WIDTH,
            want_h: MIN_GRAPH_HEIGHT,
            got_w: cfg.x_chars,
            got_h: cfg.y_chars,
        });
    }

    let hi_lbl = format!("{:.*}", DECIMAL_PRECISION, cfg.y_max);
    let lo_lbl = format!("{:.*}", DECIMAL_PRECISION, cfg.y_min);
    let lbl_w = hi_lbl.len().max(lo_lbl.len());
    let line_len = cfg.x_chars + lbl_w + LABEL_GUTTER + BORDER_WIDTH;

    let mut out = String::with_capacity(line_len * (cfg.y_chars + 4));

    // Title bar
    out.push_str(TL);
    push_centered(&mut out, &cfg.title, line_len - BORDER_WIDTH, &cfg.color);
    out.push_str(TR);
    out.push('\n');

    // Top Padding
    out.push_str(V);
    out.push_str(&" ".repeat(line_len - BORDER_WIDTH));
    out.push_str(V);
    out.push_str("\n");

    // Graph rows
    for row in 0..cfg.y_chars {
        out.push_str(V);
        if row == 0 {
            out.push_str(&format!("{hi_lbl:>lbl_w$}"));
        } else if row + 1 == cfg.y_chars {
            out.push_str(&format!("{lo_lbl:>lbl_w$}"));
        } else {
            out.push_str(&" ".repeat(lbl_w));
        }
        out.push(' ');
        out.push_str(cfg.color.as_str());
        for col in 0..cfg.x_chars {
            out.push(braille_char(col, row, plot));
        }
        out.push_str(AnsiCode::reset().as_str());
        out.push_str(V);
        out.push('\n');
    }

    // Bottom Padding
    out.push_str(V);
    out.push_str(&" ".repeat(line_len - BORDER_WIDTH));
    out.push_str(V);
    out.push_str("\n");

    // Bottom bar
    out.push_str(BL);
    if let Some(sub) = &cfg.subtitle {
        push_centered(&mut out, sub, line_len - BORDER_WIDTH, &cfg.color);
    } else {
        out.push_str(&H.repeat(line_len - BORDER_WIDTH));
    }
    out.push_str(BR);
    out.push_str("\n");
    Ok(out)
}

/// Hides the cursor on construction and shows it again on Drop
struct CursorGuard;

impl CursorGuard {
    fn new() -> Self {
        // hide → ESC[?25l
        let _ = write!(stdout(), "\x1b[?25l");
        CursorGuard
    }
}

impl Drop for CursorGuard {
    fn drop(&mut self) {
        // show → ESC[?25h
        let _ = write!(stdout(), "\x1b[?25h");
        let _ = stdout().flush();
    }
}

enum Strategy {
    /// Replace every character in the graph
    Full,
    /// Replace only the lines that changed.
    Delta { prev_hash: Vec<u64> },
}

pub struct Renderer {
    strat: Strategy,
    first_frame: bool,
}

impl Renderer {
    #[inline]
    #[must_use]
    pub fn full() -> Self {
        Self {
            strat: Strategy::Full,
            first_frame: true,
        }
    }
    #[inline]
    #[must_use]
    pub fn delta() -> Self {
        Self {
            strat: Strategy::Delta {
                prev_hash: Vec::new(),
            },
            first_frame: true,
        }
    }

    /// Calls `build_frame` and either renders it in full or only
    /// the lines that changed with delta.
    ///
    /// If using `Renderer::delta`, hash collision leads to an
    /// unnecessary redraw but no corruption.
    pub fn render(&mut self, cfg: &Config, plot: &BraillePlot) -> Result<(), GraphError> {
        let frame = build_frame(cfg, plot)?;
        let mut term = stdout().lock();
        let _cursor = CursorGuard::new();
        if self.first_frame {
            write!(term, "\x1b[2J")?;
            self.first_frame = false;
        }
        match &mut self.strat {
            Strategy::Full => {
                write!(term, "\x1b[H")?;
                term.write_all(frame.as_bytes())?;
            }
            Strategy::Delta { prev_hash } => {
                let mut row = 1usize;
                for line in frame.lines() {
                    let h = hash64(line);
                    if prev_hash.get(row - 1).is_none_or(|&p| p != h) {
                        write!(term, "\x1b[{row};1H{line}")?;
                        if row > prev_hash.len() {
                            prev_hash.push(h);
                        } else {
                            prev_hash[row - 1] = h;
                        }
                    }
                    row += 1;
                }
                for r in row..=prev_hash.len() {
                    write!(term, "\x1b[{r};1H\x1b[2K")?;
                }
                prev_hash.truncate(row - 1);

                // Park cursor *below* the frame
                // If this is the last frame, nothing gets cut off
                write!(term, "\x1b[{row};1H")?;
            }
        }
        term.flush()?;
        Ok(())
    }
}
