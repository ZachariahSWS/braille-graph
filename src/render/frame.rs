//! Full-screen braille frame renderer with:
//! - persistent double buffering (`graph_buf` / `prev_buf`)
//! - row-diff via XOR (bitfield; ≤ 64 rows fast-path, else fallback to full)
//! - batched writes using `write_vectored`
//! - cached chrome (top/bottom) buffers

use std::io::{IoSlice, Write, stdout};

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

// --- Helpers ---

/// Hides the cursor on construction and shows it again on Drop
struct CursorGuard;
impl CursorGuard {
    fn new() -> Self {
        let _ = write!(stdout(), "\x1b[?25l");
        CursorGuard
    }
}
impl Drop for CursorGuard {
    fn drop(&mut self) {
        let _ = write!(stdout(), "\x1b[?25h");
        let _ = stdout().flush();
    }
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

#[inline]
fn push_usize_dec(buf: &mut Vec<u8>, mut n: usize) {
    // enough for 64-bit usize (20 digits max)
    let mut tmp = [0u8; 20];
    let mut i = tmp.len();

    // write digits in reverse
    loop {
        i -= 1;
        tmp[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }

    buf.extend_from_slice(&tmp[i..]);
}

enum Strategy {
    /// Replace every character in the graph
    Full,
    /// Replace only the lines that changed.
    Delta,
}

pub struct Renderer {
    strat: Strategy,
    first_frame: bool,

    // cached chrome (top and bottom)
    chrome_top: Vec<u8>,
    chrome_bot: Vec<u8>,

    // persistent double buffer for graph rows
    graph_buf: Vec<u8>,
    prev_buf: Vec<u8>, // same size as graph_buf once primed

    row_bytes: usize, // cached per-row byte width (excluding '\n')

    cached_label_width: usize,
    cached_x: usize,
    cached_y: usize,
}

impl Renderer {
    #[inline]
    #[must_use]
    pub fn full() -> Self {
        Self::new(Strategy::Full)
    }
    #[inline]
    #[must_use]
    pub fn delta() -> Self {
        Self::new(Strategy::Delta)
    }

    fn new(strat: Strategy) -> Self {
        Self {
            strat,
            first_frame: true,
            chrome_top: Vec::new(),
            chrome_bot: Vec::new(),
            graph_buf: Vec::new(),
            prev_buf: Vec::new(),
            row_bytes: 0,
            cached_label_width: 0,
            cached_x: 0,
            cached_y: 0,
        }
    }

    fn refresh_chrome(&mut self, cfg: &Config, label_width: usize) {
        let line_len = cfg.x_chars + label_width + LABEL_GUTTER + BORDER_WIDTH;

        // --- top ---
        let mut top = String::new();
        top.push_str(TL);
        push_centered(&mut top, &cfg.title, line_len - BORDER_WIDTH, &cfg.color);
        top.push_str(TR);
        top.push('\n');

        top.push_str(V);
        top.push_str(&" ".repeat(line_len - BORDER_WIDTH));
        top.push_str(V);
        top.push('\n');

        self.chrome_top.clear();
        self.chrome_top.extend_from_slice(top.as_bytes());

        // --- bottom ---
        let mut bot = String::new();
        bot.push_str(V);
        bot.push_str(&" ".repeat(line_len - BORDER_WIDTH));
        bot.push_str(V);
        bot.push('\n');

        bot.push_str(BL);
        if let Some(sub) = &cfg.subtitle {
            push_centered(&mut bot, sub, line_len - BORDER_WIDTH, &cfg.color);
        } else {
            bot.push_str(&H.repeat(line_len - BORDER_WIDTH));
        }
        bot.push_str(BR);
        bot.push('\n');

        self.chrome_bot.clear();
        self.chrome_bot.extend_from_slice(bot.as_bytes());
    }

    /// Build the whole graph area (all rows) directly into `self.graph_buf`.
    fn fill_graph_rows(&mut self, cfg: &Config, plot: &BraillePlot) -> Result<(), GraphError> {
        if cfg.x_chars < MIN_GRAPH_WIDTH || cfg.y_chars < MIN_GRAPH_HEIGHT {
            return Err(GraphError::GraphTooSmall {
                want_w: MIN_GRAPH_WIDTH,
                want_h: MIN_GRAPH_HEIGHT,
                got_w: cfg.x_chars,
                got_h: cfg.y_chars,
            });
        }

        let high_label = format!("{:.*}", DECIMAL_PRECISION, cfg.y_range.1);
        let low_label = format!("{:.*}", DECIMAL_PRECISION, cfg.y_range.0);
        let label_width = high_label.len().max(low_label.len());

        // Per-row byte layout
        let color_seq = cfg.color.as_str();
        let braille_bytes = cfg.x_chars * 3; // 3 bytes per glyph

        let fixed_prefix_bytes = V_B.len()        // left border
            + label_width
            + LABEL_GUTTER
            + color_seq.len();

        let fixed_suffix_bytes = RESET_SEQ.len() + V_B.len(); // reset + right border

        let row_bytes = fixed_prefix_bytes + braille_bytes + fixed_suffix_bytes; // (no '\n')
        self.row_bytes = row_bytes;

        // allocate / resize buffers
        let stride = row_bytes + 1; // include '\n'
        let total = stride * cfg.y_chars;
        if self.graph_buf.len() != total {
            self.graph_buf.resize(total, 0);
        }
        if self.prev_buf.len() != total {
            self.prev_buf.resize(total, 0);
        }

        // Build every row
        for r in 0..cfg.y_chars {
            let base = r * stride;

            // Left border
            self.graph_buf[base..base + V_B.len()].copy_from_slice(V_B);

            // Label field -> spaces (we’ll overwrite first/last row with numbers below)
            let label_off = base + V_B.len();
            for i in 0..label_width {
                self.graph_buf[label_off + i] = b' ';
            }

            // Gutter
            let gutter_off = label_off + label_width;
            for g in 0..LABEL_GUTTER {
                self.graph_buf[gutter_off + g] = b' ';
            }

            // Color
            let col_start = base + fixed_prefix_bytes - color_seq.len();
            self.graph_buf[col_start..col_start + color_seq.len()]
                .copy_from_slice(color_seq.as_bytes());

            // Reset + right border
            let reset_start = base + fixed_prefix_bytes + braille_bytes;
            self.graph_buf[reset_start..reset_start + RESET_SEQ.len()].copy_from_slice(RESET_SEQ);
            self.graph_buf
                [reset_start + RESET_SEQ.len()..reset_start + RESET_SEQ.len() + V_B.len()]
                .copy_from_slice(V_B);

            // Newline
            self.graph_buf[base + row_bytes] = b'\n';
        }

        // Y labels (top / bottom rows only)
        let top_off = V_B.len() + label_width - high_label.len();
        self.graph_buf[top_off..top_off + high_label.len()].copy_from_slice(high_label.as_bytes());

        let last_row_base = (cfg.y_chars - 1) * stride;
        let bot_off = last_row_base + V_B.len() + label_width - low_label.len();
        self.graph_buf[bot_off..bot_off + low_label.len()].copy_from_slice(low_label.as_bytes());

        // Braille payload
        let offset = fixed_prefix_bytes;
        let row_stride = stride; // include newline
        encode_braille_into_frame(
            &mut self.graph_buf,
            offset,
            row_stride,
            plot,
            cfg.x_chars,
            cfg.y_chars,
        );

        Ok(())
    }

    /// XOR per-row vs `prev_buf` (fast path if `y_chars` ≤ 64, else full redraw).
    fn diff_rows_xor(&self, cfg: &Config) -> u64 {
        let rows = cfg.y_chars;
        if rows > 64 {
            return u64::MAX;
        }
        let mut mask: u64 = 0;
        let stride = self.row_bytes + 1;

        for i in 0..rows {
            let start = i * stride;
            let end = start + self.row_bytes; // exclude '\n'

            let mut diff = 0u64;
            let mut pos = 0;
            let chunk_len = end - start;

            while pos + 8 <= chunk_len {
                let a = u64::from_ne_bytes(
                    self.graph_buf[start + pos..start + pos + 8]
                        .try_into()
                        .unwrap(),
                );
                let b = u64::from_ne_bytes(
                    self.prev_buf[start + pos..start + pos + 8]
                        .try_into()
                        .unwrap(),
                );
                diff |= a ^ b;
                pos += 8;
            }
            while pos < chunk_len {
                diff |= u64::from(self.graph_buf[start + pos] ^ self.prev_buf[start + pos]);
                pos += 1;
            }
            if diff != 0 {
                mask |= 1u64 << i;
            }
        }
        mask
    }

    /// Main render entry.
    pub fn render(&mut self, config: &Config, plot: &BraillePlot) -> Result<(), GraphError> {
        let label_width = y_label_width(config.y_range, DECIMAL_PRECISION);

        // Refresh chrome if needed
        let chrome_stale = self.chrome_top.is_empty()
            || self.cached_label_width != label_width
            || self.cached_x != config.x_chars
            || self.cached_y != config.y_chars;

        if chrome_stale {
            self.refresh_chrome(config, label_width);
            self.cached_label_width = label_width;
            self.cached_x = config.x_chars;
            self.cached_y = config.y_chars;
        }

        self.fill_graph_rows(config, plot)?;
        let mut term = stdout().lock();
        let _cursor = CursorGuard::new();

        if self.first_frame {
            write!(term, "\x1b[2J")?;
            self.first_frame = false;
        }

        // Always re-print chrome if stale
        if chrome_stale {
            write!(term, "\x1b[1;1H")?;
            term.write_all(&self.chrome_top)?;
        }

        // Graph rows start at line 3 (1-based)
        let graph_start_row = 3usize;

        match self.strat {
            Strategy::Full => {
                write!(term, "\x1b[{graph_start_row};1H")?;
                term.write_all(&self.graph_buf)?;
                self.prev_buf.copy_from_slice(&self.graph_buf);
            }
            Strategy::Delta => {
                let dirty_mask = self.diff_rows_xor(config);
                let rows = config.y_chars;
                let dirty_count = if dirty_mask == u64::MAX && rows > 64 {
                    rows // force full
                } else {
                    dirty_mask.count_ones() as usize
                };

                let too_many = rows > 64 || dirty_count * 2 > rows; // >50% dirty → full redraw
                if too_many {
                    write!(term, "\x1b[{graph_start_row};1H")?;
                    term.write_all(&self.graph_buf)?;
                    self.prev_buf.copy_from_slice(&self.graph_buf);
                } else {
                    // --- SAFE VECTORED WRITE PATH ---
                    // Pre-build all cursor sequences in one grow (no reallocation afterwards)
                    let mut cursor_buf = Vec::<u8>::with_capacity(dirty_count * 16); // plenty
                    let mut cursor_spans: Vec<(usize, usize, usize, usize)> =
                        Vec::with_capacity(dirty_count);
                    // (cur_start, cur_end, row_start, row_len)

                    let stride = self.row_bytes + 1;

                    for i in 0..rows {
                        if dirty_mask & (1u64 << i) == 0 {
                            continue;
                        }
                        let row_1based = graph_start_row + i;

                        // cursor
                        let cur_start = cursor_buf.len();
                        cursor_buf.extend_from_slice(b"\x1b[");
                        push_usize_dec(&mut cursor_buf, row_1based);
                        cursor_buf.extend_from_slice(b";1H");
                        let cur_end = cursor_buf.len();

                        // row slice
                        let start = i * stride;
                        cursor_spans.push((cur_start, cur_end, start, self.row_bytes));
                    }

                    // Build IoSlice<'_>s that borrow from our now-stable buffers
                    let mut ios: Vec<IoSlice<'_>> = Vec::with_capacity(dirty_count * 2);
                    for (cur_start, cur_end, row_start, row_len) in &cursor_spans {
                        ios.push(IoSlice::new(&cursor_buf[*cur_start..*cur_end]));
                        ios.push(IoSlice::new(
                            &self.graph_buf[*row_start..row_start + row_len],
                        ));
                    }

                    if !ios.is_empty() {
                        let _ = term.write_vectored(&ios)?;
                    }

                    // Sync only dirty rows
                    for i in 0..rows {
                        if dirty_mask & (1u64 << i) != 0 {
                            let start = i * stride;
                            let end = start + self.row_bytes;
                            self.prev_buf[start..end].copy_from_slice(&self.graph_buf[start..end]);
                        }
                    }
                }
            }
        }

        let footer_row_start = config.y_chars + 3;
        if chrome_stale {
            write!(term, "\x1b[{footer_row_start};1H")?;
            term.write_all(&self.chrome_bot)?;
        }

        let after_footer = footer_row_start + 2;
        write!(term, "\x1b[{after_footer};1H")?;
        term.flush()?;
        Ok(())
    }
}
