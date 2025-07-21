pub mod braille;
pub mod frame;

pub use braille::{BraillePlot, filter_and_bin, preprocess_to_braille};
pub use frame::Renderer;
