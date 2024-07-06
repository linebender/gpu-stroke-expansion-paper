# Test crate

## mmark

A simple utility to generate a static instance of the mmark test scene.

Usage: `cargo run mmark {n} > mmark.svg`

## prim-count-graph

For timing graphs:

```
cargo run --release --features skia-safe prim-count-graph {directory} timing
```

Result is in gnuplot format, sent to stdout.

For primitive count graphs:

```
cargo run --release --features skia-safe prim-count-graph {directory} count
```

Result is in gnuplot format, sent to stdout.

Currently this runs arcs, lines and the Skia stroker.

## TODO: other subcommands
