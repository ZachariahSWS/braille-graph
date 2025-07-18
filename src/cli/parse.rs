use crate::AnsiCode;
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
    /// CSV path (use `-` for stdin)
    #[arg(value_name = "FILE", default_value = "-")]
    pub file: String,

    /// Graph title
    #[arg(short, long, default_value = "CSV Data")]
    pub title: String,

    /// Optional subtitle
    #[arg(short, long)]
    pub subtitle: Option<String>,

    /// Y-axis lower bound (auto if omitted)
    #[arg(long)]
    pub y_min: Option<f64>,
    /// Y-axis upper bound (auto if omitted)
    #[arg(long)]
    pub y_max: Option<f64>,

    /// X-axis lower bound
    #[arg(long)]
    pub x_min: Option<f64>,
    /// X-axis upper bound
    #[arg(long)]
    pub x_max: Option<f64>,

    /// Color (name or `#RRGGBB`)
    #[arg(long, default_value = "industrial", value_parser = parse_ansi)]
    pub color: AnsiCode,

    /// Bridge min/max envelopes
    #[arg(long)]
    pub bridge: bool,

    /// Emit timing diagnostics
    #[arg(long)]
    pub debug: bool,

    /// Sort by timestamp before plotting
    #[arg(long)]
    pub sort: bool,
}

/// `braille-graph demo …`
#[derive(Parser, Debug)]
pub struct DemoArgs {
    #[arg(long, default_value_t = 2000)]
    pub steps: usize,
    #[arg(long, default_value_t = 0.05)]
    pub dt: f64,
    #[arg(long, default_value_t = 0.0)]
    pub mu: f64,
    #[arg(long, default_value_t = 1.0)]
    pub sigma: f64,
    #[arg(long, default_value_t = 60)]
    pub fps: u64,
    #[arg(long, default_value = "industrial", value_parser = parse_ansi)]
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
