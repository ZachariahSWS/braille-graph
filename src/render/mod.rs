pub mod binner;
pub mod braille;
pub mod frame;

pub use binner::{Binner, Strategy};
pub use braille::{BraillePlot, preprocess_to_braille};
pub use frame::Renderer;
