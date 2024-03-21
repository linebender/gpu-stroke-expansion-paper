set term eps
set out "subdiv_density.eps"
set size ratio -1
set grid
plot 'espc_int' using 1:4 with lines title "subdivision density"
