//! Run-time configuration object + fluent builder.

use crate::core::{color::AnsiCode, error::ConfigError};

/// Immutable parameters handed to the renderer.
#[derive(Debug, Clone)]
pub struct Config {
    pub title: String,
    pub subtitle: Option<String>,
    pub x_chars: usize,
    pub y_chars: usize,
    pub color: AnsiCode,
    pub y_range: (f64, f64),
    pub x_range: Option<(f64, f64)>,
}

impl Config {
    #[inline]
    #[must_use]
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
    y_range: Option<(f64, f64)>,
    x_range: Option<(f64, f64)>,
    color: Option<AnsiCode>,
}

impl ConfigBuilder {
    #[must_use]
    pub(crate) fn new(x_chars: usize, y_chars: usize) -> Self {
        Self {
            x_chars,
            y_chars,
            title: None,
            subtitle: None,
            y_range: None,
            x_range: None,
            color: None,
        }
    }

    #[inline]
    #[must_use]
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    #[inline]
    #[must_use]
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = Some(s.into());
        self
    }
    #[inline]
    #[must_use]
    pub fn subtitle_opt(mut self, s: &Option<String>) -> Self {
        if let Some(t) = s {
            self.subtitle = Some(t.clone());
        }
        self
    }

    #[inline]
    #[must_use]
    pub fn y_range(mut self, r: std::ops::RangeInclusive<f64>) -> Self {
        self.y_range = Some((*r.start(), *r.end()));
        self
    }
    #[inline]
    #[must_use]
    pub fn x_range(mut self, r: std::ops::RangeInclusive<f64>) -> Self {
        self.x_range = Some((*r.start(), *r.end()));
        self
    }
    #[inline]
    #[must_use]
    pub fn color(mut self, c: AnsiCode) -> Self {
        self.color = Some(c);
        self
    }

    pub fn build(self) -> Result<Config, ConfigError> {
        let y_range = self.y_range.ok_or(ConfigError::MissingField("y_range"))?;
        if y_range.0 >= y_range.1 {
            return Err(ConfigError::InvalidRange {
                low: y_range.0,
                high: y_range.1,
            });
        }
        Ok(Config {
            title: self.title.unwrap_or_default(),
            subtitle: self.subtitle,
            x_chars: self.x_chars,
            y_chars: self.y_chars,
            color: self.color.unwrap_or_else(AnsiCode::industrial_orange),
            y_range,
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
