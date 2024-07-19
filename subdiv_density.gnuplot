set term eps lw 1.5
set out "subdiv_density.eps"
set size ratio -1
set grid
set xlabel "arc length"
set ylabel "subdivision density"
plot 'espc_int' using 1:4 with lines notitle
