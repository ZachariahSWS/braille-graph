use std::time::{Duration, Instant};

use crate::{
    core::{
        bounds::{Axis, graph_dims, terminal_geometry},
        config::Config,
        constants::{
            BORDER_WIDTH, BRAILLE_HORIZONTAL_RESOLUTION, DECIMAL_PRECISION, LABEL_GUTTER,
            MIN_GRAPH_HEIGHT, MIN_GRAPH_WIDTH,
        },
        data::{DataTimeStep, read_csv_from_path},
        error::GraphError,
        rng::Lcg,
    },
    render::{Binner, Renderer, Strategy, preprocess_to_braille},
};

use super::parse::{CsvArgs, DemoArgs};

pub fn csv(a: CsvArgs) -> Result<(), GraphError> {
    let t_ingest = Instant::now();
    let mut data = read_csv_from_path(&a.file)?;
    if !data.windows(2).all(|w| w[0].time <= w[1].time) {
        data.sort_by(|l, r| {
            l.time
                .partial_cmp(&r.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    let dur_ingest = t_ingest.elapsed().as_micros();

    // config
    let (y_low, y_high) = Axis::Y.bounds(&data);
    let term = terminal_geometry();
    let (x_chars, y_chars) = graph_dims(term, data.len());

    let mut b = Config::builder(x_chars, y_chars)
        .title(a.title)
        .subtitle_opt(&a.subtitle)
        .color(a.color)
        .y_range(a.y_min.unwrap_or(y_low)..=a.y_max.unwrap_or(y_high));

    if let (Some(lo), Some(hi)) = (a.x_min, a.x_max) {
        b = b.x_range(lo..=hi);
    }
    let cfg = b.build()?;

    // transform + render
    let mut binner = Binner::new(a.bin_type);
    let binned = binner.bin(&data, &cfg);
    let plot = preprocess_to_braille(&binned, &cfg, a.bridge)?;
    if a.debug {
        eprintln!("CSV ingest: {dur_ingest} µs   ({} rows)", plot.steps.len());
    }
    Renderer::full().render(&cfg, &plot)
}

pub fn demo(a: &DemoArgs) -> Result<(), GraphError> {
    use crate::core::bounds::{self, Axis};

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
    let dt = 1.0 / a.fps.max(1) as f64;

    for i in 0..points_needed.min(a.steps) {
        if i > 0 {
            x += a.mu.mul_add(dt, a.sigma * rng.randn() * dt.sqrt());
        }
        data.push(DataTimeStep {
            time: i as f64 * dt,
            min: x,
            max: x,
        });
    }

    // Render loop
    let mut binner = Binner::new(Strategy::Time);
    let mut renderer = Renderer::delta();
    let demo_start = Instant::now();
    let mut total_render_us: u128 = 0;
    let mut total_setup_us: u128 = 0;
    let mut total_processing_us: u128 = 0;
    let mut frame_number: usize = 0;
    let mut i = data.len();

    let frame_dt = Duration::from_secs_f64(dt);
    let mut next_frame_deadline = Instant::now() + frame_dt;

    while i < a.steps {
        let t = Instant::now();
        // Append the next point
        let dw = rng.randn() * dt.sqrt();
        x += a.mu.mul_add(dt, a.sigma * dw);
        data.push(DataTimeStep {
            time: i as f64 * dt,
            min: x,
            max: x,
        });
        i += 1;

        // Axis limits
        let y_bounds = Axis::Y.bounds(&data);

        // Determine label width **now** (exact, not guessed)
        let label_width = bounds::y_label_width(y_bounds, DECIMAL_PRECISION);

        // Terminal geometry – recalc every frame (handles resizes)
        let term = bounds::terminal_geometry();
        let cols_av = term.0.0 as usize - BORDER_WIDTH - LABEL_GUTTER - label_width - 1;
        let x_chars = cols_av.max(MIN_GRAPH_WIDTH);
        let y_chars = (term.1.0 as usize).saturating_sub(5).max(MIN_GRAPH_HEIGHT);
        let max_points = x_chars * BRAILLE_HORIZONTAL_RESOLUTION;

        if a.scroll && data.len() > max_points {
            data.drain(..data.len() - max_points);
        }

        let config = Config::builder(x_chars, y_chars)
            .title("Itô Process Demo")
            .subtitle(format!("μ = {},  σ = {}", a.mu, a.sigma))
            .color(a.color)
            .y_range(y_bounds.0..=y_bounds.1)
            .x_range(data.first().unwrap().time..=data.last().unwrap().time)
            .build()?;

        let setup_us = t.elapsed().as_micros();

        // Apply optional binning
        let binned = binner.bin(&data, &config);
        let plot = preprocess_to_braille(&binned, &config, false)?;
        let processing_us = t.elapsed().as_micros() - setup_us;

        renderer.render(&config, &plot)?;

        let now = Instant::now();
        let render_us = (now - t).as_micros() - setup_us - processing_us;

        if now < next_frame_deadline {
            std::thread::sleep(next_frame_deadline - now);
        } else {
            next_frame_deadline = now;
        }
        next_frame_deadline += frame_dt;

        frame_number += 1;
        total_render_us += render_us;
        total_setup_us += setup_us;
        total_processing_us += processing_us;
    }

    if a.debug && frame_number > 0 {
        let total_us = demo_start.elapsed().as_micros();
        let inv_frame = 1.0 / frame_number as f64;
        eprintln!(
            "demo complete: {frame_number} frames   total {total_us} µs\n   avg render {:.1} µs   avg setup {:.1}µs   avg processing {:.1}µs",
            total_render_us as f64 * inv_frame,
            total_setup_us as f64 * inv_frame,
            total_processing_us as f64 * inv_frame,
        );
    }
    Ok(())
}

/// Pretty-print available color names + an example hex code.
pub fn colors() {
    use crate::core::color::{AnsiCode, colorize};

    println!("\nPossible colors:");
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
• Connected plot  : {bin} csv sample_data/industrial_production.csv --bridge
• Named color     : {bin} csv sample_data/industrial_production.csv --color blue
• Hex color       : {bin} csv sample_data/industrial_production.csv --color #6048c1
• Custom title     : {bin} csv sample_data/industrial_production.csv \\
                      --title \"American Industrial Production, Aug 1929 = 100\"
• Debug mode       : {bin} csv sample_data/industrial_production.csv --debug
• Brownian “video” : {bin} demo --steps 3000 --sigma 0.7 --fps 25
"
    );
}
