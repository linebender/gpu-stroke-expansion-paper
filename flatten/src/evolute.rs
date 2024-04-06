// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! Math for flattening of Euler evolute

use kurbo::BezPath;

use crate::euler::{EulerParams, EulerSeg};

/// Flatten the evolute of an Euler spiral segment.
pub fn flatten_es_evolute(es: &EulerSeg, tol: f64) -> BezPath {
    let mut path = BezPath::new();
    let arc_len = (es.p1 - es.p0).length() / es.params.ch;
    let ratio = es.params.k0 / es.params.k1;
    let rho_int_0 = (0.5 * (ratio - 0.5)).abs().sqrt();
    let rho_int_1 = (0.5 * (ratio + 0.5)).abs().sqrt();
    let rho_int = rho_int_1 - rho_int_0;
    let n_subdiv = (rho_int.abs() * (arc_len / tol).sqrt()).ceil().max(1.0);
    let n = n_subdiv as usize;
    let sign2 = 2.0f64.copysign(ratio);
    for i in 0..=n {
        let t = i as f64 / n_subdiv;
        let u = rho_int_0 + t * rho_int;
        let s = sign2 * u * u + 0.5 - ratio;
        let p = es.eval_evolute(s);
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p)
        }
    }
    path
}

/// Flatten an Euler spiral segment.
///
/// This is duplicative of other work but maybe useful for exploration.
pub fn flatten_es(es: &EulerSeg, _tol: f64) -> BezPath {
    let mut path = BezPath::new();
    // TODO: smart flatten logic
    const N: usize = 10;
    for i in 0..=N {
        let t = i as f64 / N as f64;
        let p = es.eval(t);
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p)
        }
    }
    path
}

#[allow(unused)]
pub fn euler_evolute_scratch() {
    // This is a sketchpad for numerically verifying the subdivision
    // density of the evolute of an Euler spiral.
    let es_params = EulerParams::from_angles(0.7, 1.0);
    let es = EulerSeg::from_params(
        kurbo::Point::new(100., 100.),
        kurbo::Point::new(300., 100.),
        es_params,
    );
    let arc_len = (es.p1 - es.p0).length() / es.params.ch;
    println!("analytic arc length: {arc_len}");
    const N: usize = 10;
    let rho_int_0 = (0.5 * arc_len * (es.params.k0 / es.params.k1 - 0.5)).sqrt();
    for i in 0..=N {
        let t = i as f64 / N as f64;
        let dt = 1e-3;
        let s0 = es.eval_evolute(t - dt);
        let s1 = es.eval_evolute(t);
        let s2 = es.eval_evolute(t + dt);
        let dx_dt = (s2 - s0) / (2.0 * dt);
        let dx2 = ((s2 - s1) - (s1 - s0)) / (dt * dt);
        let num_k = dx2.cross(dx_dt) / dx_dt.length_squared().powf(1.5);
        // curvature of ES, scaled to unit arc length
        let k = es.params.k0 + es.params.k1 * (t - 0.5);
        // curvature of actual euler spiral
        let es_k = k / arc_len;
        // arc length of Euler spiral, shifted so inflection point is at 0
        // Note: scaled to unit length
        let s_inflection_rel = (t - 0.5) + es.params.k0 / es.params.k1;
        let s_inflection_rel = k / es.params.k1;
        // This calculation works scaled to unit arc length
        let dsprime_ds = 1.0 / (s_inflection_rel.powi(2) * es.params.k1);
        let dsprime_ds = es.params.k1 / (k * k);
        // est curvature of evolute
        //let evo_k = es_k.powi(3) * arc_len.powi(2) / es.params.k1;
        let evo_k = k.powi(3) / (arc_len * es.params.k1);
        let rho = (0.125 * evo_k.abs()).sqrt() * dsprime_ds * arc_len;
        let rho = (0.125 * k.powi(3) * arc_len / es.params.k1).sqrt() * dsprime_ds;
        let rho = (0.125 * arc_len / s_inflection_rel).sqrt();
        let rho_int = (0.5 * arc_len * s_inflection_rel).sqrt() - rho_int_0;

        //println!("{t}: {}, num_k {num_k}, analytic {evo_k}", dx_dt.length());
        //println!("{t}: dsprime numeric {}, dsprime analytic {dsprime_ds}", dx_dt.length() / arc_len);
        println!("{t}: {rho} {rho_int}");
    }
}

pub fn euler_evolute_main() {
    let es_params = EulerParams::from_angles(0.7, 1.0);
    let es = EulerSeg::from_params(
        kurbo::Point::new(100., 100.),
        kurbo::Point::new(300., 100.),
        es_params,
    );
    let path = flatten_es_evolute(&es, 0.1);
    println!("{}", path.to_svg());
    let path = flatten_es(&es, 1.0);
    println!("{}", path.to_svg());
}
