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
    render::braille::BraillePlot,
};

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
        h = h.wrapping_mul(FNV_PRIME);
        h ^= u64::from(b);
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

/// Render the full frame (title, borders, labels, graph) into a `String`.
///
/// The heavy work of converting numeric data into UTF-8 braille bytes is
/// delegated to `encode_braille`, so this function is now almost entirely
/// string-glue.
pub fn build_frame(config: &Config, plot: &BraillePlot) -> Result<String, GraphError> {
    use crate::render::braille::encode_braille;

    if config.x_chars < MIN_GRAPH_WIDTH || config.y_chars < MIN_GRAPH_HEIGHT {
        return Err(GraphError::GraphTooSmall {
            want_w: MIN_GRAPH_WIDTH,
            want_h: MIN_GRAPH_HEIGHT,
            got_w: config.x_chars,
            got_h: config.y_chars,
        });
    }

    // --- Pre-Compute Layout ---

    let high_label = format!("{:.*}", DECIMAL_PRECISION, config.y_max);
    let low_label = format!("{:.*}", DECIMAL_PRECISION, config.y_min);
    let label_w = high_label.len().max(low_label.len());

    let line_length = config.x_chars + label_w + LABEL_GUTTER + BORDER_WIDTH;
    let mut out = String::with_capacity(line_length * (config.y_chars + 4));

    // Prepare braille canvas
    let mut canvas = vec![0u8; config.x_chars * config.y_chars * 3];
    encode_braille(&mut canvas, plot, config.x_chars, config.y_chars);

    // Title bar
    out.push_str(TL);
    push_centered(
        &mut out,
        &config.title,
        line_length - BORDER_WIDTH,
        &config.color,
    );
    out.push_str(TR);
    out.push('\n');

    // Top padding
    out.push_str(V);
    out.push_str(&" ".repeat(line_length - BORDER_WIDTH));
    out.push_str(V);
    out.push('\n');

    // Graph rows
    for row in 0..config.y_chars {
        out.push_str(V);

        // Y-axis labels
        if row == 0 {
            out.push_str(&format!("{high_label:>label_w$}"));
        } else if row + 1 == config.y_chars {
            out.push_str(&format!("{low_label:>label_w$}"));
        } else {
            out.push_str(&" ".repeat(label_w));
        }
        out.push(' '); // gutter

        // Braille slice for this row
        let offset = row * config.x_chars * 3;
        let slice = &canvas[offset..offset + config.x_chars * 3];
        // SAFETY: `encode_braille` wrote valid UTF-8.
        let glyphs = std::str::from_utf8(slice).unwrap();

        out.push_str(config.color.as_str());
        out.push_str(glyphs);
        out.push_str(AnsiCode::reset().as_str());

        out.push_str(V);
        out.push('\n');
    }

    // Bottom padding
    out.push_str(V);
    out.push_str(&" ".repeat(line_length - BORDER_WIDTH));
    out.push_str(V);
    out.push('\n');

    // Bottom bar
    out.push_str(BL);
    if let Some(sub) = &config.subtitle {
        push_centered(&mut out, sub, line_length - BORDER_WIDTH, &config.color);
    } else {
        out.push_str(&H.repeat(line_length - BORDER_WIDTH));
    }
    out.push_str(BR);
    out.push('\n');

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
