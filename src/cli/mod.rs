mod handlers;
pub mod parse;
#[cfg(unix)]
mod tty_raw;

use clap::Parser;
pub use parse::Cli;

use crate::core::error::GraphError;

pub fn run() -> Result<(), GraphError> {
    let cli = parse::Cli::parse();
    match cli.cmd {
        parse::Command::Csv(a) => handlers::csv(a),
        parse::Command::Colors => {
            handlers::colors();
            Ok(())
        }
        parse::Command::Demo(a) => handlers::demo(a),
        parse::Command::Examples => {
            handlers::examples();
            Ok(())
        }
    }
}
