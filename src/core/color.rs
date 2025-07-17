//! Zero-alloc ANSI colour wrapper.  No external deps.

use std::{fmt, str};

#[derive(Debug)]
pub enum ColorError {
    InvalidHexDigit,
    InvalidHexLength,
}

// --- AnsiCode ---
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnsiCode {
    Static(&'static str),
    Inline { buf: [u8; 20], len: u8 },
}

impl AnsiCode {
    pub const fn black() -> Self {
        Self::Static("\x1b[30m")
    }
    pub const fn red() -> Self {
        Self::Static("\x1b[31m")
    }
    pub const fn green() -> Self {
        Self::Static("\x1b[32m")
    }
    pub const fn yellow() -> Self {
        Self::Static("\x1b[33m")
    }
    pub const fn blue() -> Self {
        Self::Static("\x1b[34m")
    }
    pub const fn magenta() -> Self {
        Self::Static("\x1b[35m")
    }
    pub const fn cyan() -> Self {
        Self::Static("\x1b[36m")
    }
    pub const fn white() -> Self {
        Self::Static("\x1b[37m")
    }
    pub const fn industrial_orange() -> Self {
        Self::Static("\x1b[38;2;210;135;10m")
    }
    #[inline]
    pub const fn reset() -> Self {
        Self::Static("\x1b[0m")
    }

    /// True-colour escape `ESC[38;2;R;G;Bm`.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        let mut buf = [0u8; 20];
        buf[..7].copy_from_slice(b"\x1b[38;2;");
        let mut len = 7;

        for (i, v) in [r, g, b].into_iter().enumerate() {
            len += write_u8(&mut buf[len..], v);
            if i != 2 {
                buf[len] = b';';
                len += 1;
            }
        }
        buf[len] = b'm';
        len += 1;
        Self::Inline {
            buf,
            len: len as u8,
        }
    }

    /// Parse colour names or `#rrggbb`.  Falls back to hex parser on miss.
    pub fn from_name(s: &str) -> Result<Self, ColorError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "black" => Ok(Self::black()),
            "red" => Ok(Self::red()),
            "green" => Ok(Self::green()),
            "yellow" => Ok(Self::yellow()),
            "blue" => Ok(Self::blue()),
            "magenta" => Ok(Self::magenta()),
            "cyan" => Ok(Self::cyan()),
            "white" => Ok(Self::white()),
            "orange" | "industrial" => Ok(Self::industrial_orange()),
            _ => Self::from_hex(s),
        }
    }

    pub fn from_hex(hex: &str) -> Result<Self, ColorError> {
        let h = hex.trim_start_matches('#');
        if h.len() != 6 {
            return Err(ColorError::InvalidHexLength);
        }
        let byte = |s: &str| u8::from_str_radix(s, 16).map_err(|_| ColorError::InvalidHexDigit);
        Ok(Self::rgb(byte(&h[..2])?, byte(&h[2..4])?, byte(&h[4..])?))
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Static(s) => s,
            Self::Inline { buf, len } => str::from_utf8(&buf[..*len as usize]).unwrap(),
        }
    }
}

// --- convenience conversions ---
impl<'a> From<&'a str> for AnsiCode {
    #[inline]
    fn from(s: &'a str) -> Self {
        // default to industrial orange on parse failure (no panics)
        AnsiCode::from_name(s).unwrap_or_else(|_| AnsiCode::industrial_orange())
    }
}

impl From<&String> for AnsiCode {
    #[inline]
    fn from(s: &String) -> Self {
        AnsiCode::from(s.as_str())
    }
}

impl From<AnsiCode> for String {
    #[inline]
    fn from(c: AnsiCode) -> Self {
        c.as_str().to_owned()
    }
}

// --- Helpers ---
fn write_u8(dst: &mut [u8], mut n: u8) -> usize {
    let mut tmp = [0u8; 3];
    let mut i = 3;
    loop {
        i -= 1;
        tmp[i] = b'0' + n % 10;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    let len = 3 - i;
    dst[..len].copy_from_slice(&tmp[i..]);
    len
}

impl fmt::Display for AnsiCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Wrap `text` in colour + reset sequence.
#[inline]
pub fn colorize(c: &AnsiCode, text: &str) -> String {
    format!("{c}{text}{}", AnsiCode::reset())
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::InvalidHexDigit => f.write_str("invalid hex colour digit"),
            ColorError::InvalidHexLength => f.write_str("hex colour must be exactly 6 digits"),
        }
    }
}
