//! Build a full-screen braille frame and flush to the terminal.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::{Write, stdout},
};

use crate::{
    core::{
        color::{AnsiCode, colorize},
        config::Config,
        data::DECIMAL_PRECISION,
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
pub const MIN_GRAPH_HEIGHT: usize = 7;
pub const MIN_GRAPH_WIDTH: usize = 14;

pub const BORDER_WIDTH: usize = 2;
pub const LABEL_GUTTER: usize = 1;
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
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
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

/// Map two half-columns at (char_idx,row) to a single Unicode braille scalar.
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
        stamp(s, 0)
    }
    if let Some(s) = plot.steps.get(right) {
        stamp(s, 1)
    }

    char::from_u32(BRAILLE_UNICODE_BASE + mask as u32).unwrap()
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

    // Graph rows
    for row in 0..cfg.y_chars {
        out.push_str(V);
        if row == 0 {
            out.push_str(&format!("{:>lbl_w$}", hi_lbl));
        } else if row + 1 == cfg.y_chars {
            out.push_str(&format!("{:>lbl_w$}", lo_lbl));
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

    // Bottom bar
    out.push_str(BL);
    if let Some(sub) = &cfg.subtitle {
        push_centered(&mut out, sub, line_len - BORDER_WIDTH, &cfg.color);
    } else {
        out.push_str(&H.repeat(line_len - BORDER_WIDTH));
    }
    out.push_str(BR);
    out.push_str("\n\n");
    Ok(out)
}

enum Strategy {
    /// Replace every character in the graph
    Full,
    /// Replace only the lines that changed.
    Delta {
        prev_hash: Vec<u64>,
        cursor_hidden: bool,
    },
}

pub struct Renderer {
    strat: Strategy,
}

impl Renderer {
    #[inline]
    pub fn full() -> Self {
        Self {
            strat: Strategy::Full,
        }
    }
    #[inline]
    pub fn delta() -> Self {
        Self {
            strat: Strategy::Delta {
                prev_hash: Vec::new(),
                cursor_hidden: false,
            },
        }
    }

    pub fn render(&mut self, cfg: &Config, plot: &BraillePlot) -> Result<(), GraphError> {
        let frame = build_frame(cfg, plot)?;
        match &mut self.strat {
            Strategy::Full => {
                stdout().lock().write_all(frame.as_bytes())?;
            }
            Strategy::Delta {
                prev_hash,
                cursor_hidden,
            } => {
                let mut term = stdout().lock();
                if !*cursor_hidden {
                    write!(term, "\x1b[?25l\x1b[2J\x1b[H")?; // hide cursor + clear
                    *cursor_hidden = true;
                }

                let mut row = 1usize;
                for line in frame.lines() {
                    let h = hash64(line);
                    if prev_hash.get(row - 1).map_or(true, |&p| p != h) {
                        write!(term, "\x1b[{row};1H{line}")?;
                        if row > prev_hash.len() {
                            prev_hash.push(h)
                        } else {
                            prev_hash[row - 1] = h
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
                term.flush()?;
            }
        }
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        if let Strategy::Delta {
            cursor_hidden: true,
            ..
        } = self.strat
        {
            let _ = write!(stdout(), "\x1b[?25h");
        }
    }
}
