set term eps lw 1.5
set out "prim_count.eps"
set log x
set log y
set xrange rev
set xlabel "Tolerance"
set ylabel "Primitive count"
# generated with: cargo run --release --features skia-safe prim-count-graph paths count > prim_count
plot for [I=0:2] 'prim_count' i I u 1:2 w line title columnheader(1)
