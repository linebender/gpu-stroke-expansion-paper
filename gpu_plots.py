#!/usr/bin/python3
# Copyright 2024 the Vello Authors
# SPDX-License-Identifier: Apache-2.0

import pandas as pd
import matplotlib.pyplot as plt
import matplotlib
import matplotlib.ticker as tkr
import sys

matplotlib.style.use('seaborn-v0_8-colorblind')

do_test_scenes = len(sys.argv) > 1 and sys.argv[1] == 'tests' or \
                 len(sys.argv) > 2 and sys.argv[2] == 'tests'
save_to_file = len(sys.argv) > 1 and sys.argv[1] == 'save' or \
               len(sys.argv) > 2 and sys.argv[2] == 'save'

if do_test_scenes:
	arcs = pd.DataFrame({
		"long dash (arcs)": [16.01, 3.24, 0.559, 0.08245],
		"mmark-70k (arcs)": [9.93, 1.83, 1.67, 0.13434],
		"mmark-120k (arcs)": [25.32, 2.97, 2.38, 0.19562]
		}, index=["Mali-G78", "M1 Max", "GTX 980Ti", "RTX 4090"]
	)
	lines = pd.DataFrame({
		"long dash (lines)": [24.42, 3.38, 1.35, 0.31733],
		"mmark-70k (lines)": [20.30, 3.42, 2.95, 0.22960],
		"mmark-120k (lines)": [39.41, 5.63, 4.34, 0.38593],
		}, index=["Mali-G78", "M1 Max", "GTX 980Ti", "RTX 4090"]
	)
else:
	arcs = pd.DataFrame({
		"spirograph (arcs)":[565.77, 146.03, 37.37, 13.48],
		"lorenz (arcs)":[412.13, 82.14, 50.61, 18.48],
		"spiral (arcs)":[529.88, 118.93, 35.97, 13.23],
		"blender (arcs)":[599.37, 133.49, 51.05, 20.73],
		"waves (arcs)":[1480, 158.11, 85.06, 33.75],
		"roads (arcs)":[1400, 277.05, 87.63, 31.39]
		}, index=["Mali-G78", "M1 Max", "GTX 980Ti", "RTX 4090"]
	)
	lines = pd.DataFrame({
		"spirograph (lines)":[638.10, 183.05, 47.92, 18.35],
		"lorenz (lines)":[682.04, 133.18, 93.75, 31.98],
		"spiral (lines)":[1200, 257.67, 63.08, 25.71],
		"blender (lines)":[761.69, 172.18, 68.76, 28.10],
		"waves (lines)":[3460, 270.72, 214.03, 74.02],
		"roads (lines)":[1460, 366.60, 98.41, 34.63]
		}, index=["Mali-G78", "M1 Max", "GTX 980Ti", "RTX 4090"]
	)

stacked_data = arcs
stacked_data2 = lines

fig, ax = plt.subplots()

stacked_data.plot(kind="bar", stacked=True, width=0.3, 
                  ax=ax, position=0, rot=0)
stacked_data2.plot(kind="bar", stacked=True, width=0.3, 
                   ax=ax, position=1, hatch='/', rot=0)
ax.set_xlim(right=len(stacked_data)-0.5)

def numfmt(x, pos):
    s = f'{x/1000:,.0f}'
    return s

yfmt = tkr.FuncFormatter(numfmt)

if do_test_scenes:
    ylabel = 'Time (ms)'
    filename = "test_scenes_gpu_timings.eps"
    xinches = 5
else:
    ylabel = 'Time (ms)'
    ax.yaxis.set_major_formatter(yfmt)
    filename = "nehab_gpu_timings.eps"
    xinches = 5.3

ax.set_ylabel(ylabel)
fig.set_size_inches(xinches, 4, forward=True)

if save_to_file:
    plt.savefig(filename)
else:
    plt.show()
