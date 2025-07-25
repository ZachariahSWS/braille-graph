# braille-graph

High-resolution terminal plotting (2×4 dots per character) with zero-alloc CSV ingest and diff-rendering.

## Quickstart
```bash
cargo run --release csv sample_data/industrial_production.csv --title "US Industrial Production, 1929–1941"
cargo run --release demo --steps 20000 --fps 1000
```

## CLI Reference
* `csv` - Plot CSV with 2–3 numeric columns
* `demo` - Animated Brownian motion
* `colors` - List colour names / hex syntax
* `examples` - Show common invocations

## CSV Schema
`time,<min>[,<max>]`

## Performance
On an M2, 200k frame demo at 5 kHz: 41 µs render, 68µs setup, 121 µs processing or 230 µs excluding sleep.
