#!/usr/bin/python3
# Copyright 2024 the Vello Authors
# SPDX-License-Identifier: Apache-2.0

import pandas as pd
import matplotlib.pyplot as plt
import matplotlib
import sys

matplotlib.style.use('seaborn-v0_8-colorblind')

do_test_scenes = len(sys.argv) > 1 and sys.argv[1] == 'tests' or \
                 len(sys.argv) > 2 and sys.argv[2] == 'tests'
save_to_file = len(sys.argv) > 1 and sys.argv[1] == 'save' or \
               len(sys.argv) > 2 and sys.argv[2] == 'save'

if do_test_scenes:
	arcs = pd.DataFrame({
		"long path (arcs)": [8.11, 2.90, 0.559, 0.08245],
		"mmark-35k (arcs)": [7.50, 0.943, 1.67, 0.13434],
		"mmark-60k (arcs)": [13.17, 1.54, 2.38, 0.19562]
		}, index=["Mali-G78", "M1 Max", "GTX 980Ti", "RTX 4090"]
	)
	lines = pd.DataFrame({
		"long path (lines)": [13.06, 3.11, 1.35, 0.31733],
		"mmark-35k (lines)": [12.45, 1.73, 2.95, 0.22960],
		"mmark-60k (lines)": [22.52, 2.87, 4.34, 0.38593],
		}, index=["Mali-G78", "M1 Max", "GTX 980Ti", "RTX 4090"]
	)
else:
	arcs = pd.DataFrame({
		"spirograph (arcs)":[350.59, 88.01, 37.37, 13.48],
		"lorenz (arcs)":[361.09, 65.84, 50.61, 18.48],
		"spiral (arcs)":[324.73, 104.48, 35.97, 13.23],
		"blender (arcs)":[384.37, 86.25, 51.05, 20.73],
		"waves (arcs)":[1300, 135.61, 85.06, 33.75],
		"roads (arcs)":[748.49, 102.37, 87.63, 31.39]
		}, index=["Mali-G78", "M1 Max", "GTX 980Ti", "RTX 4090"]
	)
	lines = pd.DataFrame({
		"spirograph (lines)":[404.16, 99.03, 47.92, 18.35],
		"lorenz (lines)":[616.51, 99.88, 93.75, 31.98],
		"spiral (lines)":[456.60, 106.41, 63.08, 25.71],
		"blender (lines)":[517.80, 129.34, 68.76, 28.10],
		"waves (lines)":[3120, 208.40, 214.03, 74.02],
		"roads (lines)":[947.03, 113.48, 98.41, 34.63]
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
fig.set_size_inches(5, 4, forward=True)

if save_to_file:
    filename = "test_scenes_gpu_timings.eps" if do_test_scenes else "nehab_gpu_timings.eps"
    ylabel = 'Time (ms)' if do_test_scenes else 'Time ($\\mu$s)'
    ax.set_ylabel(ylabel)
    plt.savefig(filename)
else:
    plt.show()
