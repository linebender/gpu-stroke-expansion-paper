// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! Math for flattening of Euler spiral parallel curve

use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_4};

use kurbo::{BezPath, Point};

use crate::euler::EulerSeg;

/// A robustness strategy for the ESPC integral
enum EspcRobust {
    /// Both k1 and dist are large enough to divide by robustly.
    Normal,
    /// k1 is low, so model curve as a circular arc.
    LowK1,
    /// dist is low, so model curve as just an Euler spiral.
    LowDist,
}

/// Flatten an Euler spiral parallel curve to lines.
///
/// Invoke the callback for each point in the flattening. This includes the
/// end point but not the start point.
pub fn flatten_offset(es: &EulerSeg, offset: f64, tol: f64, mut f: impl FnMut(Point)) {
    let scale = (es.p0 - es.p1).hypot();
    let (k0, k1) = (es.params.k0 - 0.5 * es.params.k1, es.params.k1);
    // Note: scaling by ch is missing from earlier implementations. The math
    // should be validated carefully.
    let dist_scaled = offset * es.params.ch / scale;
    // The number of subdivisions for curvature = 1
    let scale_multiplier = 0.5 * FRAC_1_SQRT_2 * (scale / (es.params.ch * tol)).sqrt();
    // TODO: tune these thresholds
    const K1_THRESH: f64 = 1e-3;
    const DIST_THRESH: f64 = 1e-3;
    let mut a = 0.0;
    let mut b = 0.0;
    let mut integral = 0.0;
    let mut int0 = 0.0;
    let (n_frac, robust) = if k1.abs() < K1_THRESH {
        let k = k0 + 0.5 * k1;
        let n_frac = (k * (k * dist_scaled + 1.0)).abs().sqrt();
        (n_frac, EspcRobust::LowK1)
    } else if dist_scaled.abs() < DIST_THRESH {
        let f = |x: f64| x * x.abs().sqrt();
        a = k1;
        b = k0;
        int0 = f(b);
        let int1 = f(a + b);
        integral = int1 - int0;
        //println!("int0={int0}, int1={int1} a={a} b={b}");
        let n_frac = (2. / 3.) * integral / a;
        (n_frac, EspcRobust::LowDist)
    } else {
        a = -2.0 * dist_scaled * k1;
        b = -1.0 - 2.0 * dist_scaled * k0;
        int0 = espc_int_approx(b);
        let int1 = espc_int_approx(a + b);
        integral = int1 - int0;
        let k_peak = k0 - k1 * b / a;
        let integrand_peak = (k_peak * (k_peak * dist_scaled + 1.0)).abs().sqrt();
        let scaled_int = integral * integrand_peak / a;
        let n_frac = scaled_int;
        (n_frac, EspcRobust::Normal)
    };
    let n = (n_frac * scale_multiplier).ceil().max(1.0);
    for i in 0..n as usize - 1 {
        let t = (i + 1) as f64 / n;
        let s = match robust {
            EspcRobust::LowK1 => t,
            // Note opportunities to minimize divergence
            EspcRobust::LowDist => {
                let c = (integral * t + int0).cbrt();
                let inv = c * c.abs();
                (inv - b) / a
            }
            EspcRobust::Normal => {
                let inv = espc_int_inv_approx(integral * t + int0);
                (inv - b) / a
            }
        };
        f(es.eval_with_offset(s, offset));
    }
    f(es.eval_with_offset(1.0, offset));
}

pub fn flatten_offset_iter(iter: impl Iterator<Item = EulerSeg>, offset: f64) -> BezPath {
    let mut result = BezPath::new();
    let tol = 1.0;
    let mut first = true;
    for es in iter {
        if first {
            result.move_to(es.eval_with_offset(0.0, offset));
            first = false;
        }
        flatten_offset(&es, offset, tol, |p| result.line_to(p));
    }
    result
}

/// Integral of sqrt(|x^2 - 1|), analytical.
fn espc_integral(x: f64) -> f64 {
    let d = 1.0 - x * x;
    let ds = d.abs().sqrt();
    let a = if d >= 0.0 {
        x.asin()
    } else {
        FRAC_PI_2.copysign(x) - (ds + x.abs()).ln().copysign(x)
    };
    0.5 * (ds * x + a)
}

/// Compute the inverse of the ESPC integral.
///
/// Use straightforward Newton iteration to refine the inverse of the integral,
/// generally to within floating point roundoff error.
pub fn espc_integral_inv(x: f64) -> f64 {
    let mut y = espc_int_inv_approx(x);
    for i in 0..5 {
        let err = espc_integral(y) - x;
        println!("{x}: iter {i}: y={y} err={err:e}");
        const EPS: f64 = 1e-15;
        if err.abs() < EPS {
            break;
        }
        let dydx = (y * y - 1.0).abs().sqrt();
        y -= err / dydx;
    }
    y
}

// Compute number of subdivisions needed for given tolerance, analytical.
//
// Note: scale is factor to scale from unit arc-length ES, not unit chord. I'm pretty sure
// the original prototype leaves out the ES chord factor.
pub fn n_subdiv_analytic(k0: f64, k1: f64, scale: f64, dist: f64, tol: f64) -> f64 {
    let dist_scaled = dist / scale;
    // TODO: replace by epsilon, but in the mean time this is useful for calibration.
    if k1 == 0.0 {
        let k = k0 * (k0 * dist_scaled + 1.0);
        return 0.5 * FRAC_1_SQRT_2 * (scale * k.abs() / tol).sqrt();
    }
    let a = -2.0 * dist_scaled * k1;
    let b = -1.0 - 2.0 * dist_scaled * k0;
    let integral = espc_integral(a + b) - espc_integral(b);
    let k_peak = k0 - k1 * b / a;
    let integrand_peak = (k_peak * (k_peak * dist_scaled + 1.0)).abs().sqrt();
    let scaled_int = integral * integrand_peak / a;
    // Note: work to do here. Error is 1/8 curvature * len^2, which doesn't match blog
    0.5 * FRAC_1_SQRT_2 * (scale / tol).sqrt() * scaled_int
}

pub fn n_subdiv_approx(k0: f64, k1: f64, scale: f64, dist: f64, tol: f64) -> f64 {
    // TODO: handle numerical stability when k1 and/or dist are near 0
    let dist_scaled = dist / scale;
    let a = -2.0 * dist_scaled * k1;
    let b = -1.0 - 2.0 * dist_scaled * k0;
    let integral = espc_int_approx(a + b) - espc_int_approx(b);
    let k_peak = k0 - k1 * b / a;
    let integrand_peak = (k_peak * (k_peak * dist_scaled + 1.0)).abs().sqrt();
    let scaled_int = integral * integrand_peak / a;
    0.5 * FRAC_1_SQRT_2 * (scale / tol).sqrt() * scaled_int
}

const BREAK1: f64 = 0.8;
const BREAK2: f64 = 1.25;
const BREAK3: f64 = 2.1;
const SIN_SCALE: f64 = 1.0976991822760038;
const QUAD_A1: f64 = 0.6406;
const QUAD_B1: f64 = -0.81;
const QUAD_C1: f64 = 0.9148117935952064;
const QUAD_A2: f64 = 0.5;
const QUAD_B2: f64 = -0.156;
const QUAD_C2: f64 = 0.16145779359520596;

fn espc_int_approx(x: f64) -> f64 {
    let y = x.abs();
    let a = if y < BREAK1 {
        (SIN_SCALE * y).sin() * (1.0 / SIN_SCALE)
    } else if y < BREAK2 {
        (8.0f64.sqrt() / 3.0) * (y - 1.0) * (y - 1.0).abs().sqrt() + FRAC_PI_4
    } else {
        let (a, b, c) = if y < BREAK3 {
            (QUAD_A1, QUAD_B1, QUAD_C1)
        } else {
            (QUAD_A2, QUAD_B2, QUAD_C2)
        };
        a * y * y + b * y + c
    };
    a.copysign(x)
}

fn espc_int_inv_approx(x: f64) -> f64 {
    let y = x.abs();
    let a = if y < 0.7010707591262915 {
        (x * SIN_SCALE).asin() * (1.0 / SIN_SCALE)
    } else if y < 0.903249293595206 {
        let b = y - FRAC_PI_4;
        let u = b.abs().powf(2. / 3.).copysign(b);
        u * (9.0f64 / 8.).cbrt() + 1.0
    } else {
        let (u, v, w) = if y < 2.038857793595206 {
            const B: f64 = 0.5 * QUAD_B1 / QUAD_A1;
            (B * B - QUAD_C1 / QUAD_A1, 1.0 / QUAD_A1, B)
        } else {
            const B: f64 = 0.5 * QUAD_B2 / QUAD_A2;
            (B * B - QUAD_C2 / QUAD_A2, 1.0 / QUAD_A2, B)
        };
        (u + v * y).sqrt() - w
    };
    a.copysign(x)
}
