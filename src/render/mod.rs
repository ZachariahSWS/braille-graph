pub mod braille;
pub mod frame;

pub use braille::{BraillePlot, filter_and_bin, preprocess_to_braille};
pub use frame::{BORDER_WIDTH, LABEL_GUTTER, MIN_GRAPH_HEIGHT, MIN_GRAPH_WIDTH, Renderer};
