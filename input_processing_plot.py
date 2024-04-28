#!/usr/bin/python3
# Copyright 2024 the Vello Authors
# SPDX-License-Identifier: Apache-2.0

import matplotlib.pyplot as plt
import matplotlib
import numpy as np

xlabels = ['spiral', 'lorenz', 'spirograph', 'blender', 'roads', 'waves', 'mmark-35k', 'mmark-60k', 'longpathdash']
segments = np.array([561, 2111, 2783, 2971, 7842, 13308, 105022, 179929, 503304])
mali_ip = np.array([635.59, 545.17, 588.20, 603.49, 593.95, 556.22, 2270, 4100, 5230])
mali_flatten = np.array([313.15, 342.61, 335.75, 375.86, 736.16, 1250, 7480, 13080, 8120])
m1_ip = np.array([69.47, 86.11, 120.83, 135.72, 177.21, 105.26, 89.63, 201.19, 307.82])
m1_flatten = np.array([99.74, 63.77, 87.28, 99.03, 177.21, 136.48, 950.78, 1540, 2910])
rtx4090_ip = np.array([13.95, 13.78, 13.94, 13.92, 13.96, 14.30, 15.91, 32.42, 34.41])
rtx4090_flatten = np.array([13.35, 18.38, 13.64, 20.89, 31.72, 30.97, 131.03, 196.31, 279.69])


plt.figure(figsize=(5, 4), layout='constrained')
#plt.plot(segments, mali_ip, label='Mali-G78 (tag monoid)')
#plt.plot(segments, mali_flatten, label='Mali-G78 (flatten)')
plt.plot(segments, m1_ip, label='M1 Max (tag monoid)', linewidth=2)
plt.plot(segments, m1_flatten, label='M1 Max (flatten)', linewidth=2)
plt.plot(segments, rtx4090_ip, label='RTX 4090 (tag monoid)', linewidth=2)
plt.plot(segments, rtx4090_flatten, label='RTX 4090 (tag flatten)', linewidth=2)
plt.xlabel('Input Segments')
plt.ylabel('Time ($\\mu$s)')
plt.legend()

plt.savefig('tag_monoid_gpu_timings.eps')
plt.show()
