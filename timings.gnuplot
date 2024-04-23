set term eps
set out "timings.eps"
set key autotitle columnheader
set style fill solid border 1
set style data histogram
set style histogram rowstacked
set ylabel "Time in seconds"
# generated with: cargo run --release --features skia-safe prim-count-graph paths timing > timings
plot 'timings' using 2:xtic(1), '' u 3, '' u 4, '' u 5, '' u 6, '' u 7
