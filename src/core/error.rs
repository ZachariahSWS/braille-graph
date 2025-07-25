//! Centralised error types used across the crate.

use std::{error::Error, fmt, io};

use crate::core::{color::ColorError, data::ParseCsvError};

/// Precise configuration faults.
#[derive(Debug)]
pub enum ConfigError {
    MissingField(&'static str),
    InvalidRange { low: f64, high: f64 },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingField(x) => write!(f, "configuration missing field `{x}`"),
            ConfigError::InvalidRange { low, high } => {
                write!(f, "y_min {low} must be < y_max {high}")
            }
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
        match self {
            GraphError::Io(e) => write!(f, "{e}"),
            GraphError::Csv(e) => write!(f, "{e}"),
            GraphError::Color(e) => write!(f, "{e}"),
            GraphError::Config(e) => write!(f, "{e}"),
            GraphError::GraphTooSmall {
                want_w,
                want_h,
                got_w,
                got_h,
            } => write!(
                f,
                "terminal too small: need ≥{want_w}×{want_h}, got {got_w}×{got_h}"
            ),
            GraphError::EmptyData => write!(f, "data set is empty"),
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
