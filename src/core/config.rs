//! Run-time configuration object + fluent builder.

use crate::core::{color::AnsiCode, error::ConfigError};

/// Immutable parameters handed to the renderer.
#[derive(Debug, Clone)]
pub struct Config {
    pub title: String,
    pub subtitle: Option<String>,
    pub y_min: f64,
    pub y_max: f64,
    pub x_chars: usize,
    pub y_chars: usize,
    pub color: AnsiCode,
    pub x_range: Option<(f64, f64)>,
}

impl Config {
    #[inline]
    pub fn builder(x_chars: usize, y_chars: usize) -> ConfigBuilder {
        ConfigBuilder::new(x_chars, y_chars)
    }
}

/// Fluent builder with zero allocation until `build`.
#[derive(Debug)]
pub struct ConfigBuilder {
    x_chars: usize,
    y_chars: usize,
    title: Option<String>,
    subtitle: Option<String>,
    y_min: Option<f64>,
    y_max: Option<f64>,
    x_range: Option<(f64, f64)>,
    color: Option<AnsiCode>,
}

impl ConfigBuilder {
    pub(crate) fn new(x_chars: usize, y_chars: usize) -> Self {
        Self {
            x_chars,
            y_chars,
            title: None,
            subtitle: None,
            y_min: None,
            y_max: None,
            x_range: None,
            color: None,
        }
    }

    #[inline]
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    #[inline]
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = Some(s.into());
        self
    }
    #[inline]
    pub fn subtitle_opt(mut self, s: &Option<String>) -> Self {
        if let Some(t) = s {
            self.subtitle = Some(t.clone())
        }
        self
    }
    #[inline]
    pub fn y_min(mut self, v: f64) -> Self {
        self.y_min = Some(v);
        self
    }
    #[inline]
    pub fn y_max(mut self, v: f64) -> Self {
        self.y_max = Some(v);
        self
    }
    #[inline]
    pub fn y_range(mut self, r: std::ops::RangeInclusive<f64>) -> Self {
        self.y_min = Some(*r.start());
        self.y_max = Some(*r.end());
        self
    }
    #[inline]
    pub fn x_range(mut self, lo: f64, hi: f64) -> Self {
        self.x_range = Some((lo, hi));
        self
    }
    #[inline]
    pub fn color<C: Into<AnsiCode>>(mut self, c: C) -> Self {
        self.color = Some(c.into());
        self
    }

    pub fn build(self) -> Result<Config, ConfigError> {
        let y_min = self.y_min.ok_or(ConfigError::MissingField("y_min"))?;
        let y_max = self.y_max.ok_or(ConfigError::MissingField("y_max"))?;
        if y_min >= y_max {
            return Err(ConfigError::InvalidRange {
                lo: y_min,
                hi: y_max,
            });
        }
        Ok(Config {
            title: self.title.unwrap_or_default(),
            subtitle: self.subtitle,
            y_min,
            y_max,
            x_chars: self.x_chars,
            y_chars: self.y_chars,
            color: self.color.unwrap_or_else(AnsiCode::industrial_orange),
            x_range: self.x_range,
        })
    }
}

/// Ergonomic `?` on a builder chain.
impl From<ConfigBuilder> for Result<Config, ConfigError> {
    fn from(b: ConfigBuilder) -> Self {
        b.build()
    }
}
