use crate::{AnsiCode, Strategy};
use clap::{Parser, Subcommand};

/// Top-level CLI structure.
#[derive(Parser)]
#[command(
    name = "braille-graph",
    about = "High-resolution terminal plotting using braille"
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Plot data from a CSV file
    Csv(CsvArgs),
    /// Show available color names / hex syntax
    Colors,
    /// Animated Brownian motion demo
    Demo(DemoArgs),
    /// Print example invocations
    Examples,
}

/// `braille-graph csv …`
#[derive(Parser, Debug)]
pub struct CsvArgs {
    #[arg(
        value_name = "FILE",
        default_value = "-",
        help = "CSV path (use `-` for stdin)"
    )]
    pub file: String,

    #[arg(short, long, default_value = "CSV Data", help = "Graph title")]
    pub title: String,

    #[arg(short, long, help = "Optional subtitle")]
    pub subtitle: Option<String>,

    #[arg(long, help = "Y-axis lower bound (auto if omitted)")]
    pub y_min: Option<f64>,
    #[arg(long, help = "Y-axis upper bound (auto if omitted)")]
    pub y_max: Option<f64>,

    #[arg(long, help = "X-axis lower bound (auto if omitted)")]
    pub x_min: Option<f64>,
    #[arg(long, help = "X-axis upper bound (auto if omitted)")]
    pub x_max: Option<f64>,

    #[arg(long, default_value = "industrial", value_parser = parse_ansi, help = "Color (name or `#RRGGBB`")]
    pub color: AnsiCode,

    #[arg(long, help = "Bridge min/max envelopes")]
    pub bridge: bool,

    #[arg(long, help = "Emit timing diagnostics")]
    pub debug: bool,

    #[arg(long, default_value = "time", value_parser = parse_strategy, help = "Choose whether to bin the x_axis by index or time")]
    pub bin_type: Strategy,
}

/// `braille-graph demo …`
#[derive(Parser, Debug)]
pub struct DemoArgs {
    #[arg(
        long,
        default_value_t = 2000,
        help = "Number of steps before the process terminates"
    )]
    pub steps: usize,
    #[arg(
        long,
        default_value_t = 0.0,
        help = "The drift coefficient: positive means up, negative means down"
    )]
    pub mu: f64,
    #[arg(
        long,
        default_value_t = 1.0,
        help = "The diffusion coefficient: higher means wider outcomes"
    )]
    pub sigma: f64,
    #[arg(long, default_value_t = 60, help = "Updates per second")]
    pub fps: u64,
    #[arg(long, default_value = "industrial", value_parser = parse_ansi, help = "Use colors command for valid strings")]
    pub color: AnsiCode,
    #[arg(
        long,
        default_value_t = false,
        help = "Scroll instead of bin when the series exceeds screen width"
    )]
    pub scroll: bool,
    #[arg(long, default_value_t = false, help = "Emit timing diagnostics")]
    pub debug: bool,
}

fn parse_ansi(s: &str) -> Result<AnsiCode, String> {
    match s.to_ascii_lowercase().as_str() {
        // accepted names
        "black" => Ok(AnsiCode::black()),
        "red" => Ok(AnsiCode::red()),
        "green" => Ok(AnsiCode::green()),
        "yellow" => Ok(AnsiCode::yellow()),
        "blue" => Ok(AnsiCode::blue()),
        "magenta" => Ok(AnsiCode::magenta()),
        "cyan" => Ok(AnsiCode::cyan()),
        "white" => Ok(AnsiCode::white()),
        "orange" | "industrial" => Ok(AnsiCode::industrial_orange()),
        // hex literal
        _ if s.starts_with('#') => AnsiCode::from_hex(s).map_err(|e| e.to_string()),
        _ => Err(format!("unknown color '{s}' (try colors)")),
    }
}

fn parse_strategy(s: &str) -> Result<Strategy, String> {
    match s.to_ascii_lowercase().as_str() {
        "index" => Ok(Strategy::Index),
        "time" => Ok(Strategy::Time),
        _ => Err(format!("unknown bin type '{s}' (try index or time)")),
    }
}
