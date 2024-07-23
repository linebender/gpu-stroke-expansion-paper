// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! Plot of cubic to Euler spiral error metric.

use clap::Parser;
use kurbo::{CubicBez, Point};

use crate::euler::{CubicParams, EulerSeg};

#[derive(Parser)]
pub struct CubicErrPlot {
    #[arg(short, long)]
    approx: bool,
}

// This function currently calls est_euler_err, but the code exists to compute from
// scratch, which might be useful for experimentation.
#[allow(unused)]
pub fn cubic_err_plot(plot: CubicErrPlot) {
    const N: usize = 600;
    let th0 = 0.1f64;
    let th1 = 0.2f64;
    for y in 0..N {
        for x in 0..N {
            let param0 = x as f64 * (0.666 / N as f64) + 0.0001;
            let param1 = y as f64 * (0.666 / N as f64) + 0.0001;
            let d0 = param0;
            let d1 = param1;
            let p2 = Point::new(1.0 - d1 * th1.cos(), d1 * th1.sin());
            let p1 = Point::new(d0 * th0.cos(), d0 * th0.sin());
            let c = CubicBez::new(Point::ORIGIN, p1, p2, Point::new(1.0, 0.0));
            let e = EulerSeg::from_cubic(c);
            let err = if plot.approx {
                let e0 = 2. / (3. * (1.0 + th0.cos()));
                let e1 = 2. / (3. * (1.0 + th1.cos()));
                let s0 = th0.sin();
                let s1 = th1.sin();
                let s01 = (th0 + th1).sin();
                let amin = 0.15 * (2. * e0 * s0 + 2. * e1 * s1 - e0 * e1 * s01);
                let a = 0.15 * (2. * d0 * s0 + 2. * d1 * s1 - d0 * d1 * s01);
                let aerr = (a - amin).abs();
                let symm = (th0 + th1).abs();
                let asymm = (th0 - th1).abs();
                let dist = (d0 - e0).hypot(d1 - e1);
                let amb3 = 1e-3 * symm.powi(3) + 1e-3 * asymm.powi(3);
                //aerr.hypot(0.02 * symm * dist).hypot(2.0 * asymm * dist).hypot(amb3)
                // Accurate estimate of center error for th0, th1 in [0..2/3]
                let ctr = 3e-6 * symm.powi(5) + 6e-3 * asymm * symm.powi(2);
                //ctr + aerr + 2e-3 * symm * dist.powf(0.5) + 7e-2 * asymm * dist.powf(0.5)
                let ctr = 3.7e-6 * symm.powi(5) + 6e-3 * asymm * symm.powi(2);
                let halo_symm = 5e-3 * symm * dist;
                let halo_asymm = 7e-2 * asymm * dist;
                //1.25 * ctr + 1.55 * aerr + halo_symm + halo_asymm
                let cparams = CubicParams::from_cubic(c);
                cparams.est_euler_err()
            } else {
                e.cubic_euler_err(c, 10)
            };
            println!("{param0} {param1} {}", err.log10().max(-8.));
        }
        println!("");
    }
}
