//! Memory-efficient CSV loader with zero-allocation float parsing.

use std::{
    error::Error,
    fmt::{self, Display},
    io::{BufRead, BufReader, Read},
};

// --- Public Row Structs ---
#[derive(Clone, Copy)]
pub struct DataTimeStep {
    pub time: f64,
    pub min: f64,
    pub max: f64,
}

// --- Error Handling ---
#[derive(Debug)]
pub struct ParseCsvError {
    pub line: usize,
    pub kind: ParseErrorKind,
}

#[derive(Debug)]
pub enum ParseErrorKind {
    Io(std::io::Error),
    BadColumnCount(usize),
    BadFloat { field: &'static str, text: String },
}

impl Display for ParseCsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ParseErrorKind::Io(e) => write!(f, "I/O error on line {}: {}", self.line, e),
            ParseErrorKind::BadColumnCount(n) => {
                write!(f, "line {}: expected 2–3 columns, got {}", self.line, n)
            }
            ParseErrorKind::BadFloat { field, text } => {
                write!(f, "line {}: invalid {} value '{}'", self.line, field, text)
            }
        }
    }
}
impl Error for ParseCsvError {}

// --- Helpers ---
#[inline]
fn trim(mut b: &[u8]) -> &[u8] {
    while !b.is_empty() && b[0].is_ascii_whitespace() {
        b = &b[1..];
    }
    while !b.is_empty() && b[b.len() - 1].is_ascii_whitespace() {
        b = &b[..b.len() - 1];
    }
    b
}

#[inline]
pub fn normalize_unicode_minus(buf: &mut Vec<u8>) {
    let (mut r, mut w) = (0, 0);
    while r < buf.len() {
        if r + 2 < buf.len() && buf[r] == 0xE2 && buf[r + 1] == 0x88 && buf[r + 2] == 0x92 {
            buf[w] = b'-';
            r += 3;
            w += 1;
        } else {
            if r != w {
                buf[w] = buf[r];
            }
            r += 1;
            w += 1;
        }
    }
    buf.truncate(w);
}

#[inline]
fn parse_f64(bytes: &[u8], line: usize, field: &'static str) -> Result<f64, ParseCsvError> {
    let val = lexical_core::parse::<f64>(bytes).map_err(|_| ParseCsvError {
        line,
        kind: ParseErrorKind::BadFloat {
            field,
            text: String::from_utf8_lossy(bytes).into_owned(),
        },
    })?;
    if val.is_finite() {
        Ok(val)
    } else {
        Err(ParseCsvError {
            line,
            kind: ParseErrorKind::BadFloat {
                field,
                text: "NaN".into(),
            },
        })
    }
}

// --- Fast CSV ingest ---
const BUF_CAP: usize = 1 << 20; // 1 MiB

pub fn read_csv_fast<R: Read>(src: R) -> Result<Vec<DataTimeStep>, ParseCsvError> {
    let mut rdr = BufReader::with_capacity(BUF_CAP, src);
    let mut buf = Vec::<u8>::with_capacity(256);
    let mut data = Vec::<DataTimeStep>::new();
    let mut saw_first = false;
    let mut line_no = 0usize;

    loop {
        buf.clear();
        let n = rdr.read_until(b'\n', &mut buf).map_err(|e| ParseCsvError {
            line: line_no,
            kind: ParseErrorKind::Io(e),
        })?;
        if n == 0 {
            break;
        }
        line_no += 1;

        if buf.ends_with(b"\n") {
            buf.pop();
        }
        if buf.ends_with(b"\r") {
            buf.pop();
        }

        normalize_unicode_minus(&mut buf);
        if buf.is_empty() || buf[0] == b'#' {
            continue;
        }

        // simple header detection (non-numeric first field)
        if !saw_first {
            saw_first = true;
            let first = buf.iter().position(|&b| b == b',').unwrap_or(buf.len());
            if lexical_core::parse::<f64>(trim(&buf[..first])).is_err() {
                continue;
            }
        }

        // split – max 3 cols
        let mut cols = [None::<&[u8]>; 3];
        let mut idx = 0;
        let mut start = 0;
        loop {
            let end = buf[start..]
                .iter()
                .position(|&b| b == b',')
                .map_or(buf.len(), |p| start + p);
            if idx < 3 {
                cols[idx] = Some(trim(&buf[start..end]));
                idx += 1;
            } else {
                return Err(ParseCsvError {
                    line: line_no,
                    kind: ParseErrorKind::BadColumnCount(idx + 1),
                });
            }
            if end == buf.len() {
                break;
            }
            start = end + 1;
        }
        if idx < 2 {
            return Err(ParseCsvError {
                line: line_no,
                kind: ParseErrorKind::BadColumnCount(idx),
            });
        }

        let t = parse_f64(cols[0].unwrap(), line_no, "time")?;
        let min = parse_f64(cols[1].unwrap(), line_no, "min")?;
        let max = match cols[2] {
            Some(c) if !c.is_empty() => parse_f64(c, line_no, "max")?,
            _ => min,
        };
        data.push(DataTimeStep { time: t, min, max });
    }
    if data.is_empty() {
        return Err(ParseCsvError {
            line: 0,
            kind: ParseErrorKind::BadColumnCount(0),
        });
    }
    Ok(data)
}

pub fn read_csv_from_path(path: &str) -> Result<Vec<DataTimeStep>, ParseCsvError> {
    if path == "-" {
        read_csv_fast(std::io::stdin())
    } else {
        use std::fs::File;
        read_csv_fast(File::open(path).map_err(|e| ParseCsvError {
            line: 0,
            kind: ParseErrorKind::Io(e),
        })?)
    }
}
