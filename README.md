# braille-graph

High-resolution terminal plotting (2×4 dots per character) with zero-alloc CSV ingest and diff-rendering.

## Quickstart
```bash
cargo run --release csv sample_data/industrial_production.csv --title "US Industrial Production, 1929–1941"
```

## CLI Reference
* `csv` - Plot CSV with 2–3 numeric columns
* `demo` - Animated Brownian motion
* `colors` - List colour names / hex syntax
* `examples` - Show common invocations

## CSV Schema
`time,<min>[,<max>]`
