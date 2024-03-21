// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

// 32 bit versions of flattening integral, intended to match GPU-side
// computation.

use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_4};

use kurbo::BezPath;

use crate::cubic32::Point;
use crate::euler32::EulerSeg;

/// A robustness strategy for the ESPC integral
enum EspcRobust {
    /// Both k1 and dist are large enough to divide by robustly.
    Normal,
    /// k1 is low, so model curve as a circular arc.
    LowK1,
    /// dist is low, so model curve as just an Euler spiral.
    LowDist,
}

fn to_kurbo(p: Point) -> kurbo::Point {
    kurbo::Point::new(p.x as f64, p.y as f64)
}

// Note: generates a kurbo-compatible BezPath for debugging reasons, but
// actual output can be f32 LineSoup without much trouble.
pub fn flatten_offset(iter: impl Iterator<Item = EulerSeg>, offset: f32, tol: f32) -> BezPath {
    let mut result = BezPath::new();
    let mut first = true;
    for es in iter {
        if core::mem::take(&mut first) {
            result.move_to(to_kurbo(es.eval_with_offset(0.0, offset)));
        }
        let scale = (es.p0 - es.p1).hypot();
        let (k0, k1) = (es.params.k0 - 0.5 * es.params.k1, es.params.k1);
        // Note: scaling by ch is missing from earlier implementations. The math
        // should be validated carefully.
        let dist_scaled = offset * es.params.ch / scale;
        // The number of subdivisions for curvature = 1
        let scale_multiplier = 0.5 * FRAC_1_SQRT_2 * (scale / (es.params.ch * tol)).sqrt();
        // TODO: tune these thresholds
        const K1_THRESH: f32 = 1e-3;
        const DIST_THRESH: f32 = 1e-3;
        let mut a = 0.0;
        let mut b = 0.0;
        let mut integral = 0.0;
        let mut int0 = 0.0;
        let (n_frac, robust) = if k1.abs() < K1_THRESH {
            let k = k0 + 0.5 * k1;
            let n_frac = (k * (k * dist_scaled + 1.0)).abs().sqrt();
            (n_frac, EspcRobust::LowK1)
        } else if dist_scaled.abs() < DIST_THRESH {
            let f = |x: f32| x * x.abs().sqrt();
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
        for i in 0..n as usize {
            let t = (i + 1) as f32 / n;
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
            result.line_to(to_kurbo(es.eval_with_offset(s, offset)));
        }
    }
    result
}

const BREAK1: f32 = 0.8;
const BREAK2: f32 = 1.25;
const BREAK3: f32 = 2.1;
const SIN_SCALE: f32 = 1.0976991822760038;
const QUAD_A1: f32 = 0.6406;
const QUAD_B1: f32 = -0.81;
const QUAD_C1: f32 = 0.9148117935952064;
const QUAD_A2: f32 = 0.5;
const QUAD_B2: f32 = -0.156;
const QUAD_C2: f32 = 0.16145779359520596;

pub fn espc_int_approx(x: f32) -> f32 {
    let y = x.abs();
    let a = if y < BREAK1 {
        (SIN_SCALE * y).sin() * (1.0 / SIN_SCALE)
    } else if y < BREAK2 {
        (8.0f32.sqrt() / 3.0) * (y - 1.0) * (y - 1.0).abs().sqrt() + FRAC_PI_4
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

pub fn espc_int_inv_approx(x: f32) -> f32 {
    let y = x.abs();
    let a = if y < 0.7010707591262915 {
        (x * SIN_SCALE).asin() * (1.0 / SIN_SCALE)
    } else if y < 0.903249293595206 {
        let b = y - FRAC_PI_4;
        let u = b.abs().powf(2. / 3.).copysign(b);
        u * (9.0f32 / 8.).cbrt() + 1.0
    } else {
        let (u, v, w) = if y < 2.038857793595206 {
            const B: f32 = 0.5 * QUAD_B1 / QUAD_A1;
            (B * B - QUAD_C1 / QUAD_A1, 1.0 / QUAD_A1, B)
        } else {
            const B: f32 = 0.5 * QUAD_B2 / QUAD_A2;
            (B * B - QUAD_C2 / QUAD_A2, 1.0 / QUAD_A2, B)
        };
        (u + v * y).sqrt() - w
    };
    a.copysign(x)
}

// The following two functions are for experimentation and do not need to
// be ported.

pub fn n_subdiv_approx(k0: f32, k1: f32, scale: f32, dist: f32, tol: f32) -> f32 {
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

pub fn n_subdiv_robust(k0: f32, k1: f32, scale: f32, dist: f32, tol: f32) -> f32 {
    let dist_scaled = dist / scale;
    // The number of subdivisions for curvature = 1
    let scale_multiplier = 0.5 * FRAC_1_SQRT_2 * (scale / tol).sqrt();
    // TODO: tune these thresholds
    const K1_THRESH: f32 = 1e-3;
    const DIST_THRESH: f32 = 1e-3;
    if k1.abs() < K1_THRESH {
        //println!("below k1 thresh");
        let k = k0 + 0.5 * k1;
        return scale_multiplier * (k * (k * dist_scaled + 1.0)).abs().sqrt();
    }
    if dist.abs() < DIST_THRESH {
        // This is computed as a special case, which is easy to reason about,
        // but on GPU we might try to minimize divergence by unifying with the
        // main case.
        //println!("below dist thresh");
        let f = |x: f32| x * x.abs().sqrt();
        return (2. / 3.) * scale_multiplier * (f(k0 + k1) - f(k0)) / k1;
    }
    let a = -2.0 * dist_scaled * k1;
    let b = -1.0 - 2.0 * dist_scaled * k0;
    let integral = espc_int_approx(a + b) - espc_int_approx(b);
    let k_peak = k0 - k1 * b / a;
    let integrand_peak = (k_peak * (k_peak * dist_scaled + 1.0)).abs().sqrt();
    let scaled_int = integral * integrand_peak / a;
    scale_multiplier * scaled_int
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cubic32::Point;
    use crate::euler32::{EulerParams, EulerSeg};

    /// Maximum distance between coordinates of similar paths.
    ///
    /// Panic on non-similar or anything other than an open polyline.
    fn linepath_dist(path0: &BezPath, path1: &BezPath) -> f64 {
        use kurbo::PathEl;
        let els0 = path0.elements();
        let els1 = path1.elements();
        assert_eq!(els0.len(), els1.len());
        let mut err2 = 0.0;
        for (el0, el1) in els0.iter().zip(els1) {
            match (el0, el1) {
                (PathEl::MoveTo(p0), PathEl::MoveTo(p1)) => {
                    let dist2 = p0.distance_squared(*p1);
                    err2 = dist2.max(err2);
                }
                (PathEl::LineTo(p0), PathEl::LineTo(p1)) => {
                    let dist2 = p0.distance_squared(*p1);
                    err2 = dist2.max(err2);
                }
                _ => panic!("unexpected path element"),
            }
        }
        err2.sqrt()
    }

    #[test]
    fn test_low_dist() {
        let ep = crate::euler::EulerParams::from_angles(0.1, 0.2);
        let es = EulerSeg {
            p0: Point::new(0.0, 0.0),
            p1: Point::new(1.0, 0.0),
            params: EulerParams::from_f64(&ep),
        };
        let tol = 0.1;
        let ref_path = flatten_offset([es].into_iter(), 0.0, tol);
        for dist in [1e-5, 1e-4, 1e-3] {
            let path = flatten_offset([es].into_iter(), dist, tol);
            let err = linepath_dist(&ref_path, &path);
            // We expect the distance of points to be close to the offset.
            // Note: it isn't precise, as middle points can slide along the
            // tangent.
            assert!(err > dist as f64 * 0.99);
            assert!(err < dist as f64 * 1.1);
        }
    }

    #[test]
    fn test_low_k1() {
        let k0 = 0.2;
        let ep = crate::euler::EulerParams::from_angles(k0, k0);
        let es = EulerSeg {
            p0: Point::new(0.0, 0.0),
            p1: Point::new(1.0, 0.0),
            params: EulerParams::from_f64(&ep),
        };
        let tol = 0.1;
        let dist = 0.1;
        let ref_path = flatten_offset([es].into_iter(), dist, tol);
        for epsilon in [1e-5, 1e-4, 1e-3, 2e-3] {
            let ep = crate::euler::EulerParams::from_angles(0.2, 0.2 - epsilon);
            let es = EulerSeg {
                p0: Point::new(0.0, 0.0),
                p1: Point::new(1.0, 0.0),
                params: EulerParams::from_f64(&ep),
            };
            let path = flatten_offset([es].into_iter(), dist, tol);
            let err = linepath_dist(&ref_path, &path);
            let expected_err = epsilon * 0.5 * k0;
            assert!(err > expected_err as f64 * 0.9);
            assert!(err < expected_err as f64 * 1.2);
        }
    }
}
