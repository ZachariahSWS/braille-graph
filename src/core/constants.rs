//! A collection of constants.

/// The left and right border characters
pub const BORDER_WIDTH: usize = 2;
/// One character of space between x axis labels and the plotted data
pub const LABEL_GUTTER: usize = 1;

/// Graph must be at least 7 characters tall
pub const MIN_GRAPH_HEIGHT: usize = 7;
/// Graph must be at least 14 characters wide
pub const MIN_GRAPH_WIDTH: usize = 14;

/// Braille has 2 horizontal dots and four vertical dots that can be either off or on
pub const BRAILLE_HORIZONTAL_RESOLUTION: usize = 2;
/// Braille has 2 horizontal dots and four vertical dots that can be either off or on
pub const BRAILLE_VERTICAL_RESOLUTION: usize = 4;

/// Numbers are rounded to the first decimal place.
///
/// 14.832 becomes 14.8
pub const DECIMAL_PRECISION: usize = 1;
