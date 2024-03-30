// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! Logic for lowering Euler spirals to arcs

use std::f64::consts::FRAC_PI_2;

use kurbo::{common::GAUSS_LEGENDRE_COEFFS_32, Vec2};

use crate::euler::{EulerParams, EulerSeg};

/// Area of a circle segment.
///
/// The `th` argument is the total angle for the arc.
pub fn arc_area(th: f64) -> f64 {
    // The trig has numerical robustness problems even for reasonable
    // small inputs; use Taylor series instead
    if th.abs() < 0.022 {
        let t2 = th * th;
        (2. / 3. + (4. / 45.) * t2 + (4. / 315.) * t2 * t2) * th
    } else {
        let sth = th.sin();
        (th - sth * th.cos()) / (sth * sth)
    }
}

pub fn inv_arc_area(a: f64) -> f64 {
    const SCALE: f64 = 0.949;
    let mut th = (1.5 / SCALE) * (SCALE * a).atan();
    let thabs = th.abs();
    let n_iter = if thabs < 1e-5 {
        0
    } else if thabs < 0.15 {
        1
    } else if thabs < 0.1 {
        2
    } else if thabs < 0.5 {
        3
    } else if thabs < 1.4 {
        4
    } else if thabs < 3.2 {
        5
    } else {
        // TODO: more - and possibly fit the curve
        6
    };
    for _ in 0..n_iter {
        let err = arc_area(th) - a;
        let sth = th.sin();
        let deriv = (2. - 2. * th * th.cos() / sth) / (sth * sth);
        th -= err / deriv;
    }
    th
}

// Possibly should be method of EulerParams
fn euler_area(params: &EulerParams) -> f64 {
    let mut sum = 0.0;
    for (wi, xi) in GAUSS_LEGENDRE_COEFFS_32 {
        let t = 0.5 + 0.5 * xi;
        let p = params.eval(t);
        sum += wi * p.y * params.eval_th(t).cos();
    }
    0.5 * sum / params.ch
}

fn fit_euler_to_arc(params: &EulerParams) -> f64 {
    let a = euler_area(params);
    println!("area = {a}");
    inv_arc_area(4.0 * a)
}

fn euler_arc_error(params: &EulerParams, th: f64) -> f64 {
    let arc_params = EulerParams::from_angles(th, th);
    const N: usize = 100;
    let mut max_err2 = 0.0;
    for i in 1..N {
        let t = i as f64 / N as f64;
        let d = params.eval(t) - arc_params.eval(t);
        max_err2 = d.length_squared().max(max_err2);
    }
    max_err2.sqrt()
}

fn espc_arc_error(params: &EulerParams, d: f64, k: f64) -> f64 {
    let arc_params = EulerParams::from_angles(k * 0.5, k * 0.5);
    let p0 = params.eval_with_offset(0.0, d);
    let p1 = params.eval_with_offset(1.0, d);
    let arc_seg = EulerSeg::from_params(p0, p1, arc_params);
    const N: usize = 100;
    let mut max_err2 = 0.0;
    let s0 = params.eval_th(0.0) * d * params.ch;
    let s1 = 1. + params.eval_th(1.0) * d * params.ch;
    for i in 1..N {
        let t = i as f64 / N as f64;
        let s = (t + params.eval_th(t) * d * params.ch - s0) / (s1 - s0);
        let d = params.eval_with_offset(t, d) - arc_seg.eval(s);
        max_err2 = d.length_squared().max(max_err2);
    }
    max_err2.sqrt()
}

// The area calculation of the offset strip is careful, but the mapping between
// area and k0 is the first order approximation (and the estimate of area of an
// ES is just the mean of the endpoint angles).
//
// We could wire up the precise area/k0 mapping, but it's probably more productive
// to measure the error and tweak error metrics accordingly.
fn espc_to_arc_k(params: &EulerParams, d: f64) -> f64 {
    let chord = 1.0;
    let s = chord / params.ch;
    let k0 = params.k0;
    let th0 = -params.eval_th(0.0);
    let th1 = params.eval_th(1.0);
    let aq = 0.5 * d * (chord * (th0.cos() + th1.cos()) + d * k0.sin());
    ((s * s + 6.0 * d * d) * k0 + 12. * s * d - 12. * aq) / (s + d * k0).powi(2)
}

#[allow(unused)]
fn arc_main_err() {
    let kmean = 1.0;
    let dk = 2.0;
    let ep = EulerParams::from_angles(kmean + 0.5 * dk, kmean - 0.5 * dk);
    for i in 0..11 {
        let t = i as f64 * 0.1;
        println!("{:?} {}", ep.eval(t), ep.eval_th(t));
    }
    println!("{ep:?}");
    let th = fit_euler_to_arc(&ep);
    let th = kmean;
    println!("th = {th}");
    let err = euler_arc_error(&ep, th);
    let est_err = 0.05 * dk;
    let est_err_k1 = -ep.k1 * 1. / 120.;
    println!("err = {err:.3e}, scaled = {:.3e}, est = {est_err:.3e}, est k1 = {est_err_k1:.e}", err * ep.ch);
}

pub fn arc_main() {
    let ep = EulerParams::from_angles(-0.05, 0.05);
    for i in 0..11 {
        let d = i as f64 * 0.1;
        let prediction = (ep.k1 * 1. / 120.).abs() * (1.0 + 0.4 * (ep.k1 * d).abs());
        println!("{d:.1}: {} {prediction}", espc_arc_error(&ep, d, ep.k0));
    }
}