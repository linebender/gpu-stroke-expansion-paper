set term eps
set out "cubic_err_approx.eps"
set size square
set xlabel "Left control distance"
set ylabel "Right control distance"
set cbrange [-5:-1]
# generate with: cargo run cubic-err -a > cubic_err_approx
plot 'cubic_err_approx' using 1:2:3 with image title ''
