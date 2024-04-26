set term eps
set out "cubic_err_measured.eps"
set size square
set xlabel "Left control distance"
set ylabel "Right control distance"
set cbrange [-5:-1]
# generate with: cargo run cubic-err > cubic_err_measured
plot 'cubic_err_measured' using 1:2:3 with image title ''
