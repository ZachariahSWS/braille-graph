//! Build a full-screen braille frame and flush to the terminal,
//! with cached top / bottom chrome to avoid per-frame work.

use std::io::{Write, stdout};

use crate::{
    core::{
        bounds::y_label_width,
        color::{AnsiCode, colorize},
        config::Config,
        constants::{
            BORDER_WIDTH, DECIMAL_PRECISION, LABEL_GUTTER, MIN_GRAPH_HEIGHT, MIN_GRAPH_WIDTH,
        },
        error::GraphError,
    },
    render::braille::{BraillePlot, encode_braille_into_frame},
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

const V_B: &[u8] = V.as_bytes();

const RESET_SEQ: &[u8] = b"\x1b[0m";

#[inline]
fn hash64(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;
    let mut h = FNV_OFFSET;
    for &b in s.as_bytes() {
        h = h.wrapping_mul(FNV_PRIME) ^ u64::from(b);
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

// --- Cached-Chrome Helpers ---

fn refresh_chrome(out_top: &mut String, out_bot: &mut String, cfg: &Config, label_w: usize) {
    let line_len = cfg.x_chars + label_w + LABEL_GUTTER + BORDER_WIDTH;

    // --- Rebuild top bar + blank line ---
    out_top.clear();
    out_top.push_str(TL);
    push_centered(out_top, &cfg.title, line_len - BORDER_WIDTH, &cfg.color);
    out_top.push_str(TR);
    out_top.push('\n');

    out_top.push_str(V);
    out_top.push_str(&" ".repeat(line_len - BORDER_WIDTH));
    out_top.push_str(V);
    out_top.push('\n');

    // --- Rebuild blank line + footer ---
    out_bot.clear();
    out_bot.push_str(V);
    out_bot.push_str(&" ".repeat(line_len - BORDER_WIDTH));
    out_bot.push_str(V);
    out_bot.push('\n');

    out_bot.push_str(BL);
    if let Some(sub) = &cfg.subtitle {
        push_centered(out_bot, sub, line_len - BORDER_WIDTH, &cfg.color);
    } else {
        out_bot.push_str(&H.repeat(line_len - BORDER_WIDTH));
    }
    out_bot.push_str(BR);
    out_bot.push('\n');
}

/// Build only the graph rows (with y-labels) into `dst`.
fn build_graph_rows(
    config: &Config,
    plot: &BraillePlot,
    out: &mut String,
) -> Result<(), GraphError> {
    out.clear();
    // Sanity-check geometry in columns
    if config.x_chars < MIN_GRAPH_WIDTH || config.y_chars < MIN_GRAPH_HEIGHT {
        return Err(GraphError::GraphTooSmall {
            want_w: MIN_GRAPH_WIDTH,
            want_h: MIN_GRAPH_HEIGHT,
            got_w: config.x_chars,
            got_h: config.y_chars,
        });
    }

    let high_label = format!("{:.*}", DECIMAL_PRECISION, config.y_max);
    let low_label = format!("{:.*}", DECIMAL_PRECISION, config.y_min);
    let label_w = high_label.len().max(low_label.len());

    // Per-row sizes in bytes
    let color_seq = config.color.as_str();
    let braille_bytes = config.x_chars * 3; // 3 bytes per glyph

    let fixed_prefix_bytes = V_B.len() // leading border
        + label_w                      // y-label field
        + LABEL_GUTTER                 // single space
        + color_seq.len(); // color escape

    let fixed_suffix_bytes = RESET_SEQ.len() + V_B.len(); // reset seq + trailing border

    let row_bytes = fixed_prefix_bytes + braille_bytes + fixed_suffix_bytes; // *no* newline

    // Allocate graph rows
    let mut graph = vec![0u8; (row_bytes + 1) * config.y_chars]; // + newline

    for r in 0..config.y_chars {
        let base = r * (row_bytes + 1);

        // Left border
        graph[base..base + V_B.len()].copy_from_slice(V_B);

        // fill label field with spaces
        for i in 0..label_w {
            graph[base + V_B.len() + i] = b' ';
        }

        // gutter spaces
        for g in 0..LABEL_GUTTER {
            graph[base + V_B.len() + label_w + g] = b' ';
        }

        // Color escape right after gutter
        let col_start = base + fixed_prefix_bytes - color_seq.len();
        graph[col_start..col_start + color_seq.len()].copy_from_slice(color_seq.as_bytes());

        // Reset + right border
        let reset_start = base + fixed_prefix_bytes + braille_bytes;
        graph[reset_start..reset_start + RESET_SEQ.len()].copy_from_slice(RESET_SEQ);
        graph[reset_start + RESET_SEQ.len()..reset_start + RESET_SEQ.len() + V_B.len()]
            .copy_from_slice(V_B);

        graph[base + row_bytes] = b'\n';
    }

    // Y-axis labels
    let top_off = V_B.len() + label_w - high_label.len(); // bytes from row start
    graph[top_off..top_off + high_label.len()].copy_from_slice(high_label.as_bytes());

    let last_row_base = (config.y_chars - 1) * (row_bytes + 1);
    let bot_off = last_row_base + V_B.len() + label_w - low_label.len();
    graph[bot_off..bot_off + low_label.len()].copy_from_slice(low_label.as_bytes());

    // Braille glyphs
    encode_braille_into_frame(
        &mut graph,
        fixed_prefix_bytes,
        row_bytes + 1,
        plot,
        config.x_chars,
        config.y_chars,
    );

    debug_assert!(std::str::from_utf8(&graph).is_ok());

    // SAFETY: all UTF-8 bytes are either Braille based on constants,
    // terminal box glyphs translated from strings or ascii spaces, digits and decimals.
    //
    // PERFORMANCE: average render for 200k frame demo went from 53µs to 66µs when checked with unwrap().
    // This represents roughly 4% of total user time and is significant enough to warrant unsafe code.
    out.push_str(unsafe { std::str::from_utf8_unchecked(&graph) });
    Ok(())
}

/// Hides the cursor on construction and shows it again on Drop
struct CursorGuard;

impl CursorGuard {
    fn new() -> Self {
        // hide is ESC[?25l
        let _ = write!(stdout(), "\x1b[?25l");
        CursorGuard
    }
}

impl Drop for CursorGuard {
    fn drop(&mut self) {
        // show is ESC[?25h
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
    scratch: String, // graph rows
    chrome_top: String,
    chrome_bot: String,
    cached_label_width: usize,
    cached_x: usize,
    cached_y: usize,
}

impl Renderer {
    #[inline]
    #[must_use]
    pub fn full() -> Self {
        Self {
            strat: Strategy::Full,
            first_frame: true,
            scratch: String::new(),
            chrome_top: String::new(),
            chrome_bot: String::new(),
            cached_label_width: 0,
            cached_x: 0,
            cached_y: 0,
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
            scratch: String::new(),
            chrome_top: String::new(),
            chrome_bot: String::new(),
            cached_label_width: 0,
            cached_x: 0,
            cached_y: 0,
        }
    }

    /// Cache top and bottom 2 lines. Everything else is determined by `build_graph_rows`
    /// and either renders it in full or only the lines that changed with delta.
    ///
    /// If using `Renderer::delta`, hash collision leads to an
    /// unnecessary redraw but no corruption.
    pub fn render(&mut self, config: &Config, plot: &BraillePlot) -> Result<(), GraphError> {
        let label_width = y_label_width(config.y_min, config.y_max, DECIMAL_PRECISION);

        // Refresh chrome if needed
        let chrome_stale = self.chrome_top.is_empty()
            || self.cached_label_width != label_width
            || self.cached_x != config.x_chars
            || self.cached_y != config.y_chars;

        if chrome_stale {
            refresh_chrome(
                &mut self.chrome_top,
                &mut self.chrome_bot,
                config,
                label_width,
            );
            self.cached_label_width = label_width;
            self.cached_x = config.x_chars;
            self.cached_y = config.y_chars;
        }
        build_graph_rows(config, plot, &mut self.scratch)?;
        let mut term = stdout().lock();
        let _cursor = CursorGuard::new();

        if self.first_frame {
            write!(term, "\x1b[2J")?;
            self.first_frame = false;
        }

        // Always re-print chrome if stale or first frame
        if chrome_stale {
            write!(term, "\x1b[1;1H{}", self.chrome_top)?;
        }

        // Graph rows start at line 3 (1-based)
        let graph_start_row = 3usize;
        match &mut self.strat {
            Strategy::Full => {
                write!(term, "\x1b[{};1H{}", graph_start_row, self.scratch)?;
            }
            Strategy::Delta { prev_hash } => {
                let mut row_index = graph_start_row;
                for line in self.scratch.lines() {
                    let h = hash64(line);
                    if prev_hash
                        .get(row_index - graph_start_row)
                        .is_none_or(|&p| p != h)
                    {
                        write!(term, "\x1b[{row_index};1H{line}")?;
                        if row_index - graph_start_row >= prev_hash.len() {
                            prev_hash.push(h);
                        } else {
                            prev_hash[row_index - graph_start_row] = h;
                        }
                    }
                    row_index += 1;
                }
                // Erase any leftover old lines
                for r in row_index..(graph_start_row + prev_hash.len()) {
                    write!(term, "\x1b[{r};1H\x1b[2K")?;
                }
                prev_hash.truncate(row_index - graph_start_row);
            }
        }

        let footer_row_start = config.y_chars + 3; // blank-under-graph line
        if chrome_stale {
            write!(term, "\x1b[{footer_row_start};1H{}", self.chrome_bot)?;
        }

        // park cursor just below the footer
        let after_footer = footer_row_start + 2;
        write!(term, "\x1b[{after_footer};1H")?;

        term.flush()?;
        Ok(())
    }
}
