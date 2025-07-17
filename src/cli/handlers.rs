use std::time::Instant;

use crate::{
    DECIMAL_PRECISION, MIN_GRAPH_HEIGHT, MIN_GRAPH_WIDTH,
    core::{
        bounds::{Axis, graph_dims, terminal_geometry},
        config::Config,
        data::{DataTimeStep, read_csv_from_path},
        error::GraphError,
        rng::Lcg,
    },
    render::{BORDER_WIDTH, LABEL_GUTTER, Renderer, filter_and_bin, preprocess_to_braille},
};

use super::parse::{CsvArgs, DemoArgs};

pub fn csv(a: CsvArgs) -> Result<(), GraphError> {
    let t_ingest = Instant::now();
    let mut data = read_csv_from_path(&a.file)?;
    if a.sort {
        data.sort_by(|l, r| l.time.partial_cmp(&r.time).unwrap());
    }
    let dur_ingest = t_ingest.elapsed().as_micros();

    // config
    let (y_lo, y_hi) = Axis::Y.bounds(&data);
    let term = terminal_geometry();
    let (x_chars, y_chars) = graph_dims(term, data.len());

    let mut b = Config::builder(x_chars, y_chars)
        .title(a.title)
        .subtitle_opt(&a.subtitle)
        .color(&a.color)
        .y_min(a.y_min.unwrap_or(y_lo))
        .y_max(a.y_max.unwrap_or(y_hi));

    if let (Some(lo), Some(hi)) = (a.x_min, a.x_max) {
        b = b.x_range(lo, hi);
    }
    let cfg = b.build()?;

    // transform + render
    data = filter_and_bin(data, &cfg);
    let plot = preprocess_to_braille(data, &cfg, a.cumulative)?;
    if a.debug {
        eprintln!("CSV ingest: {dur_ingest} µs   ({} rows)", plot.steps.len());
    }
    Renderer::full().render(&cfg, &plot)
}

pub fn demo(a: DemoArgs) -> Result<(), GraphError> {
    use crate::core::bounds::{self, Axis};
    use crate::core::data::BRAILLE_HORIZONTAL_RESOLUTION;

    // quit flag (q/Q)
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    fn quit_flag() -> Arc<AtomicBool> {
        #[cfg(unix)]
        crate::cli::tty_raw::enter_raw_mode().ok();
        let f = Arc::new(AtomicBool::new(false));
        let f2 = f.clone();
        std::thread::spawn(move || {
            use std::io::Read;
            for b in std::io::stdin().bytes().flatten() {
                if matches!(b, b'q' | b'Q') {
                    f2.store(true, Ordering::Relaxed);
                    break;
                }
            }
        });
        f
    }

    // RNG + first samples
    let mut rng = Lcg::seed_from_time();
    let mut data = Vec::<DataTimeStep>::with_capacity(a.steps);
    let mut x = 0.0_f64;

    // Seed with enough points to fill current terminal width
    let term = bounds::terminal_geometry();
    let label_w = 4; // safe lower bound until we know y-range
    let cols_available = term.0.0 as usize - BORDER_WIDTH - LABEL_GUTTER - label_w - 1; // safety margin
    let char_cols = cols_available.max(MIN_GRAPH_WIDTH);
    let points_needed = char_cols * BRAILLE_HORIZONTAL_RESOLUTION;

    for i in 0..points_needed.min(a.steps) {
        if i > 0 {
            x += a.mu * a.dt + a.sigma * rng.randn() * a.dt.sqrt();
        }
        data.push(DataTimeStep {
            time: i as f64 * a.dt,
            min: x,
            max: x,
        });
    }

    // Render loop
    let mut renderer = Renderer::delta();
    let stop = quit_flag();
    let frame_pause = std::time::Duration::from_millis(1_000 / a.fps.max(1));
    let mut i = data.len();

    while i < a.steps && !stop.load(Ordering::Relaxed) {
        // Append the next point
        let dw = rng.randn() * a.dt.sqrt();
        x += a.mu * a.dt + a.sigma * dw;
        data.push(DataTimeStep {
            time: i as f64 * a.dt,
            min: x,
            max: x,
        });
        i += 1;

        // Axis limits
        let (y_lo, y_hi) = Axis::Y.bounds(&data);

        // Determine label width **now** (exact, not guessed)
        let lbl_w = bounds::y_label_width(y_lo, y_hi, DECIMAL_PRECISION);

        // Terminal geometry – recalc every frame (handles resizes)
        let term = bounds::terminal_geometry();
        let cols_av = term.0.0 as usize - BORDER_WIDTH - LABEL_GUTTER - lbl_w - 1;
        let x_chars = cols_av.max(MIN_GRAPH_WIDTH);
        let y_chars = (term.1.0 as usize).saturating_sub(4).max(MIN_GRAPH_HEIGHT);
        let max_pts = x_chars * BRAILLE_HORIZONTAL_RESOLUTION;

        if a.scroll {
            if data.len() > max_pts {
                data.drain(..data.len() - max_pts);
            }
        }

        let cfg = Config::builder(x_chars, y_chars)
            .title("Itô Process Demo")
            .subtitle(format!("μ = {},  σ = {}", a.mu, a.sigma))
            .color(&a.color)
            .y_range(y_lo..=y_hi)
            .x_range(data.first().unwrap().time, data.last().unwrap().time)
            .build()?;

        // Apply optional binning
        let vis = if a.scroll {
            data.clone()
        } else {
            filter_and_bin(data.clone(), &cfg)
        };
        let plot = preprocess_to_braille(vis, &cfg, true)?;
        renderer.render(&cfg, &plot)?;

        std::thread::sleep(frame_pause);
    }
    Ok(())
}

/// Pretty-print available colour names + an example hex code.
pub fn colors() {
    use crate::core::color::{AnsiCode, colorize};

    println!("\nPossible colours:");
    println!("{}", colorize(&AnsiCode::black(), "black"));
    println!("{}", colorize(&AnsiCode::red(), "red"));
    println!("{}", colorize(&AnsiCode::green(), "green"));
    println!("{}", colorize(&AnsiCode::yellow(), "yellow"));
    println!("{}", colorize(&AnsiCode::blue(), "blue"));
    println!("{}", colorize(&AnsiCode::magenta(), "magenta"));
    println!("{}", colorize(&AnsiCode::cyan(), "cyan"));
    println!("{}", colorize(&AnsiCode::white(), "white"));
    println!(
        "{}",
        colorize(&AnsiCode::industrial_orange(), "orange | industrial")
    );
    println!(
        "{}  (#505050 or any other #RRGGBB)\n",
        colorize(&AnsiCode::rgb(0x50, 0x50, 0x50), "#505050")
    );
}

/// Print handy invocations for new users.
pub fn examples() {
    let bin = "cargo run"; // adjust if you rename the binary
    println!(
        "
Example invocations
-------------------
• Basic CSV        : {bin} csv sample_data/industrial_production.csv
• Cumulative plot  : {bin} csv sample_data/industrial_production.csv --cumulative
• Named colour     : {bin} csv sample_data/industrial_production.csv --color blue
• Hex colour       : {bin} csv sample_data/industrial_production.csv --color #6048c1
• Custom title     : {bin} csv sample_data/industrial_production.csv \\
                      --title \"American Industrial Production, Aug 1929 = 100\"
• Debug mode       : {bin} csv sample_data/industrial_production.csv --debug
• Brownian “video” : {bin} demo --steps 3000 --sigma 0.7 --fps 25
"
    );
}
