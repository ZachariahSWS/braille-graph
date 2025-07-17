//! Centralised error types used across the crate.

use std::{error::Error, fmt, io};

use crate::core::{color::ColorError, data::ParseCsvError};

/// Precise configuration faults.
#[derive(Debug)]
pub enum ConfigError {
    MissingField(&'static str),
    InvalidRange { lo: f64, hi: f64 },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ConfigError::*;
        match self {
            MissingField(x) => write!(f, "configuration missing field `{x}`"),
            InvalidRange { lo, hi } => write!(f, "y_min {lo} must be < y_max {hi}"),
        }
    }
}
impl Error for ConfigError {}

/// Top-level error type bubbled up by public APIs.
#[derive(Debug)]
pub enum GraphError {
    Io(io::Error),
    Csv(ParseCsvError),
    Color(ColorError),
    Config(ConfigError),
    GraphTooSmall {
        want_w: usize,
        want_h: usize,
        got_w: usize,
        got_h: usize,
    },
    EmptyData,
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use GraphError::*;
        match self {
            Io(e) => write!(f, "{e}"),
            Csv(e) => write!(f, "{e}"),
            Color(e) => write!(f, "{e}"),
            Config(e) => write!(f, "{e}"),
            GraphTooSmall {
                want_w,
                want_h,
                got_w,
                got_h,
            } => write!(
                f,
                "terminal too small: need ≥{}×{}, got {}×{}",
                want_w, want_h, got_w, got_h
            ),
            EmptyData => write!(f, "data set is empty"),
        }
    }
}
impl Error for GraphError {}

// automatic conversions
impl From<io::Error> for GraphError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<ParseCsvError> for GraphError {
    fn from(e: ParseCsvError) -> Self {
        Self::Csv(e)
    }
}
impl From<ColorError> for GraphError {
    fn from(e: ColorError) -> Self {
        Self::Color(e)
    }
}
impl From<ConfigError> for GraphError {
    fn from(e: ConfigError) -> Self {
        Self::Config(e)
    }
}
