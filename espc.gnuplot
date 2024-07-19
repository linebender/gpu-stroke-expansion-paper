set term eps lw 1.5
set out "espc.eps"
set size square
set grid
plot 'espc_int' using 1:2 with lines title "exact integral", 'espc_int' using 1:3 with lines title "approximation"
