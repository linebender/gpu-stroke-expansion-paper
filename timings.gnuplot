set term eps
set out "timings.eps"
set key autotitle columnheader
set style fill solid border 1
set style data histogram
set style histogram rowstacked
set ylabel "Time in milliseconds"
set key left
# generated with: cargo run --release --features skia-safe prim-count-graph paths timing > timings
plot 'timings' using ($2 * 1000):xtic(1), '' u ($3 * 1000), '' u ($4 * 1000), '' u ($5 * 1000), '' u ($7 * 1000), '' u ($8 * 1000)
