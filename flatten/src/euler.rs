// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! Calculations and utilities for Euler spirals

use arrayvec::ArrayVec;
use kurbo::{CubicBez, ParamCurve, ParamCurveArclen, Point, Vec2};

use crate::flatten::n_subdiv_analytic;

#[derive(Debug)]
pub struct CubicParams {
    pub th0: f64,
    pub th1: f64,
    pub d0: f64,
    pub d1: f64,
}

#[derive(Debug)]
pub struct EulerParams {
    pub th0: f64,
    #[allow(unused)]
    pub th1: f64,
    pub k0: f64,
    pub k1: f64,
    pub ch: f64,
}

#[derive(Debug)]
pub struct EulerSeg {
    pub p0: Point,
    pub p1: Point,
    pub params: EulerParams,
}

pub struct CubicToEulerIter {
    c: CubicBez,
    tolerance: f64,
    // [t0 * dt .. (t0 + 1) * dt] is the range we're currently considering
    t0: u64,
    dt: f64,
    last_p: Point,
    last_q: Vec2,
    last_t: f64,
}

impl CubicParams {
    pub fn from_cubic(c: CubicBez) -> Self {
        let chord = c.p3 - c.p0;
        // TODO: if chord is 0, we have a problem
        let d01 = c.p1 - c.p0;
        let h0 = Vec2::new(
            d01.x * chord.x + d01.y * chord.y,
            d01.y * chord.x - d01.x * chord.y,
        );
        let th0 = h0.atan2();
        let d0 = h0.hypot() / chord.hypot2();
        let d23 = c.p3 - c.p2;
        let h1 = Vec2::new(
            d23.x * chord.x + d23.y * chord.y,
            d23.x * chord.y - d23.y * chord.x,
        );
        let th1 = h1.atan2();
        let d1 = h1.hypot() / chord.hypot2();
        CubicParams { th0, th1, d0, d1 }
    }

    pub fn from_points_derivs(p0: Point, p1: Point, q0: Vec2, q1: Vec2, dt: f64) -> Self {
        let chord = p1 - p0;
        let scale = dt / chord.hypot2();
        let h0 = Vec2::new(
            q0.x * chord.x + q0.y * chord.y,
            q0.y * chord.x - q0.x * chord.y,
        );
        let th0 = h0.atan2();
        let d0 = h0.hypot() * scale;
        let h1 = Vec2::new(
            q1.x * chord.x + q1.y * chord.y,
            q1.x * chord.y - q1.y * chord.x,
        );
        let th1 = h1.atan2();
        let d1 = h1.hypot() * scale;
        CubicParams { th0, th1, d0, d1 }
    }

    // Estimated error of GH to Euler spiral
    //
    // Return value is normalized to chord - to get actual error, multiply
    // by chord.
    pub fn est_euler_err(&self) -> f64 {
        // Potential optimization: work with unit vector rather than angle
        let e0 = (2. / 3.) / (1.0 + self.th0.cos());
        let e1 = (2. / 3.) / (1.0 + self.th1.cos());
        let s0 = self.th0.sin();
        let s1 = self.th1.sin();
        let s01 = (s0 + s1).sin();
        let amin = 0.15 * (2. * e0 * s0 + 2. * e1 * s1 - e0 * e1 * s01);
        let a = 0.15 * (2. * self.d0 * s0 + 2. * self.d1 * s1 - self.d0 * self.d1 * s01);
        let aerr = (a - amin).abs();
        let symm = (self.th0 + self.th1).abs();
        let asymm = (self.th0 - self.th1).abs();
        let dist = (self.d0 - e0).hypot(self.d1 - e1);
        let ctr = 3.7e-6 * symm.powi(5) + 6e-3 * asymm * symm.powi(2);
        let halo_symm = 5e-3 * symm * dist;
        let halo_asymm = 7e-2 * asymm * dist;
        1.25 * ctr + 1.55 * aerr + halo_symm + halo_asymm
    }
}

impl EulerParams {
    pub fn from_angles(th0: f64, th1: f64) -> EulerParams {
        let mut k1_old = 0.0;
        let dth = th1 - th0;
        let k0 = th0 + th1;
        let mut k1 = (6.0 - (1. / 70.) * dth * dth - 0.1 * k0 * k0) * dth;
        let mut error_old = dth;
        for _ in 0..10 {
            let (u, v) = integ_euler(k0, k1, 1e-12);
            let chth = v.atan2(u);
            let error = dth - (0.25 * k1 - 2.0 * chth);
            if error.abs() < 1e-9 {
                let ch = u.hypot(v);
                return EulerParams {
                    th0,
                    th1,
                    k0,
                    k1,
                    ch,
                };
            }
            let new_k1 = k1 + (k1_old - k1) * error / (error - error_old);
            k1_old = k1;
            error_old = error;
            k1 = new_k1;
        }
        panic!("fit_euler diverged on {}, {}", th0, th1);
    }

    /// Evaluate theta at given parameter.
    ///
    /// Note: this returns -th0 at t=0 and th1 at t=1. A case can be made that
    /// negating the sign would be more intuitive, but this is the way most of
    /// my Euler spiral code works.
    pub fn eval_th(&self, t: f64) -> f64 {
        (self.k0 + 0.5 * self.k1 * (t - 1.0)) * t - self.th0
    }

    /// Curvature scaled to a unit chord.
    ///
    /// Curvature for an arbitrary scaling can be determined by dividing by
    /// the chord length.
    pub fn curvature(&self, t: f64) -> f64 {
        let raw_k = self.k0 + self.k1 * (t - 0.5);
        raw_k * self.ch
    }

    /// Find parameters to solve for given theta.
    ///
    /// Note that this returns all possible parameters, not just [0, 1].
    pub fn inv_th(&self, th: f64) -> ArrayVec<f64, 2> {
        let c0 = -th - self.th0;
        let c1 = self.k0 - 0.5 * self.k1;
        let c2 = 0.5 * self.k1;
        kurbo::common::solve_quadratic(c0, c1, c2)
    }

    /// Evaluate the curve at the given parameter.
    ///
    /// The parameter is in the range 0..1, and the result goes from (0, 0) to (1, 0).
    pub fn eval(&self, t: f64) -> Point {
        let thm = self.eval_th(t * 0.5);
        let k0 = self.k0;
        let k1 = self.k1;
        // This value is for experimentation, so is tuned for accuracy rather than performance.
        // It is hardcoded here to keep things easy, but would be plumbed for production code.
        const ACCURACY: f64 = 1e-12;
        let (u, v) = integ_euler((k0 + k1 * (0.5 * t - 0.5)) * t, k1 * t * t, ACCURACY);
        let s = t / self.ch * thm.sin();
        let c = t / self.ch * thm.cos();
        let x = u * c - v * s;
        let y = -v * c - u * s;
        Point::new(x, y)
    }

    pub fn eval_with_offset(&self, t: f64, offset: f64) -> Point {
        let th = self.eval_th(t);
        let v = Vec2::new(offset * th.sin(), offset * th.cos());
        self.eval(t) + v
    }

    /// Compute estimated flattening error based on sampling density integral,
    pub fn est_flatten_err(&self, offset: f64) -> f64 {
        let scale = 1.0 / self.ch;
        let n = n_subdiv_analytic(self.k0 - 0.5 * self.k1, self.k1, scale, offset, 1.0);
        n * n
    }

    /// Compute exact flattening error based on distance to chord.
    pub fn exact_flatten_err(&self, offset: f64) -> f64 {
        let n0 = offset * Vec2::new(-self.th0.sin(), self.th0.cos());
        let n1 = offset * Vec2::new(self.th1.sin(), self.th1.cos());
        let d01 = Vec2::new(1.0, 0.0) + n1 - n0;
        let u = d01.x / d01.hypot();
        let v = d01.y / d01.hypot();
        let chord_th = d01.atan2();
        // println!("pt0 = {n0:?}");
        // println!("pt1 = {:?}", Vec2::new(1.0, 0.0) + n1);
        // println!("chord_th = {chord_th}, ch = {}", d01.hypot());
        let mut err = 0.0;
        for t in self.inv_th(-chord_th) {
            if t > 0.0 && t < 1.0 {
                // println!("solved t = {t}");
                // rotate point to horizontal axis
                let p = self.eval_with_offset(t, offset) - n0;
                // let x = p.x * u + p.y * v;
                let y = p.y * u - p.x * v;
                // println!("{t}: {x}, {y}");
                err = y.abs().max(err);
            }
        }
        err
    }

    /// Compute exact flattening error based on distance to chord, for a segment.
    pub fn exact_flatten_err_seg(&self, offset: f64, t0: f64, t1: f64) -> f64 {
        let p0 = self.eval_with_offset(t0, offset);
        let p1 = self.eval_with_offset(t1, offset);
        let d01 = p1 - p0;
        let u = d01.x / d01.hypot();
        let v = d01.y / d01.hypot();
        let chord_th = d01.atan2();
        let mut err = 0.0;
        for t in self.inv_th(-chord_th) {
            if t > t0 && t < t1 {
                let p = self.eval_with_offset(t, offset) - p0;
                let y = p.y * u - p.x * v;
                err = y.abs().max(err);
            }
        }
        err
    }

    /// Compute the ESPC flattening integral using crude numerical integration.
    ///
    /// Primarily used to validate the more sophisticated analytical techniques.
    pub fn numeric_flatten_err(&self, offset: f64) -> f64 {
        const N: usize = 1000;
        let mut sum = 0.0;
        for i in 0..N {
            let t = (i as f64 + 0.5) * (1.0 / N as f64);
            let k = self.curvature(t);
            let integrand = k * (1.0 + k * offset);
            sum += integrand.abs().sqrt();
        }
        let n = sum * (1.0 / N as f64) * 0.125f64.sqrt() / self.ch;
        n * n
    }

    fn eval_evolute(&self, t: f64) -> Point {
        let offset = -1.0 / self.curvature(t);
        self.eval_with_offset(t, offset)
    }

    // Determine whether a render as a single cubic will be adequate
    pub fn cubic_ok(&self) -> bool {
        self.th0.abs() < 1.0 && self.th1.abs() < 1.0
    }
}

impl EulerSeg {
    pub fn from_params(p0: Point, p1: Point, params: EulerParams) -> Self {
        EulerSeg { p0, p1, params }
    }

    /// Create an Euler segment from a cubic Bézier.
    ///
    /// The curve is fit according to G1 geometric Hermite interpolation, in
    /// other words the endpoints and tangents match the given curve.
    pub fn from_cubic(c: CubicBez) -> EulerSeg {
        let d01 = c.p1 - c.p0;
        let d23 = c.p3 - c.p2;
        let d03 = c.p3 - c.p0;
        let th0 = d03.cross(d01).atan2(d03.dot(d01));
        let th1 = d23.cross(d03).atan2(d23.dot(d03));
        let params = EulerParams::from_angles(th0, th1);
        EulerSeg::from_params(c.p0, c.p3, params)
    }

    /// Use two-parabola approximation.
    pub fn to_cubic(&self) -> CubicBez {
        let (s0, c0) = self.params.th0.sin_cos();
        let (s1, c1) = self.params.th1.sin_cos();
        let d0 = (2. / 3.) / (1.0 + c0);
        let d1 = (2. / 3.) / (1.0 + c1);
        let chord = self.p1 - self.p0;
        let p1 = self.p0 + d0 * Vec2::new(chord.x * c0 - chord.y * s0, chord.y * c0 + chord.x * s0);
        let p2 = self.p1 - d1 * Vec2::new(chord.x * c1 + chord.y * s1, chord.y * c1 - chord.x * s1);
        CubicBez::new(self.p0, p1, p2, self.p1)
    }

    #[allow(unused)]
    pub fn eval(&self, t: f64) -> Point {
        let Point { x, y } = self.params.eval(t);
        let chord = self.p1 - self.p0;
        Point::new(
            self.p0.x + chord.x * x - chord.y * y,
            self.p0.y + chord.x * y + chord.y * x,
        )
    }

    pub fn eval_with_offset(&self, t: f64, offset: f64) -> Point {
        let chord = self.p1 - self.p0;
        let scaled = offset / chord.hypot();
        let Point { x, y } = self.params.eval_with_offset(t, scaled);
        Point::new(
            self.p0.x + chord.x * x - chord.y * y,
            self.p0.y + chord.x * y + chord.y * x,
        )
    }

    pub fn eval_evolute(&self, t: f64) -> Point {
        let chord = self.p1 - self.p0;
        let Point { x, y } = self.params.eval_evolute(t);
        Point::new(
            self.p0.x + chord.x * x - chord.y * y,
            self.p0.y + chord.x * y + chord.y * x,
        )
    }

    /// Calculate error from cubic bezier.
    ///
    /// This is a fairly brute-force technique, sampling the bezier and
    /// reporting RMS distance error.
    ///
    /// Note: experimentation suggests that n = 4 is enough to estimate the
    /// error fairly accurately. A future evolution may get rid of that
    /// parameter, and possibly also a tolerance parameter.
    pub fn cubic_euler_err(&self, cubic: CubicBez, n: usize) -> f64 {
        // One way to improve this would be compute arclengths for each segment
        // and cumulative sum them, rather than from 0 each time.
        //
        // We can also consider Legendre-Gauss quadrature.
        //
        // It's likely a rough approximation to arclength will be effective
        // here, for example LGQ with a low order.
        let cubic_len = cubic.arclen(1e-9);
        let mut err = 0.0;
        for i in 0..n {
            let t = (i + 1) as f64 / ((n + 1) as f64);
            let norm_len = cubic.subsegment(0.0..t).arclen(1e-9) / cubic_len;
            let cubic_xy = cubic.eval(t);
            let euler_xy = self.eval(norm_len);
            err += (cubic_xy - euler_xy).hypot2();
        }
        (err / (n as f64)).sqrt()
    }
}

/// Evaluate point and derivative.
///
/// Note that the second value returned is 1/3 the actual derivative,
/// to reduce multiplication.
pub fn eval_and_deriv(c: &CubicBez, t: f64) -> (Point, Vec2) {
    let p0 = c.p0.to_vec2();
    let p1 = c.p1.to_vec2();
    let p2 = c.p2.to_vec2();
    let p3 = c.p3.to_vec2();
    let m = 1.0 - t;
    let mm = m * m;
    let mt = m * t;
    let tt = t * t;
    let p = p0 * (mm * m) + (p1 * (3.0 * mm) + p2 * (3.0 * mt) + p3 * tt) * t;
    let q = (p1 - p0) * mm + (p2 - p1) * (2.0 * mt) + (p3 - p2) * tt;
    (p.to_point(), q)
}

/// Threshold below which a derivative is considered too small.
const DERIV_THRESH: f64 = 1e-6;
/// Amount to nudge t when derivative is near-zero.
const DERIV_EPS: f64 = 1e-6;

impl Iterator for CubicToEulerIter {
    type Item = EulerSeg;

    fn next(&mut self) -> Option<EulerSeg> {
        let t0 = (self.t0 as f64) * self.dt;
        if t0 == 1.0 {
            return None;
        }
        loop {
            let mut t1 = t0 + self.dt;
            let p0 = self.last_p;
            let q0 = self.last_q;
            let (mut p1, mut q1) = eval_and_deriv(&self.c, t1);
            if q1.hypot2() < DERIV_THRESH.powi(2) {
                let (p1_, q1_) = eval_and_deriv(&self.c, t1 - DERIV_EPS);
                q1 = q1_;
                if t1 < 1.0 {
                    p1 = p1_;
                    t1 = t1 - DERIV_EPS;
                }
            }
            let actual_dt = t1 - self.last_t;
            let cubic_params = CubicParams::from_points_derivs(p0, p1, q0, q1, actual_dt);
            let est_err = cubic_params.est_euler_err();
            let err = est_err * (p1 - p0).hypot();
            if err <= self.tolerance {
                self.t0 += 1;
                let shift = self.t0.trailing_zeros();
                self.t0 >>= shift;
                self.dt *= (1 << shift) as f64;
                let euler_params = EulerParams::from_angles(cubic_params.th0, cubic_params.th1);
                let es = EulerSeg::from_params(p0, p1, euler_params);
                self.last_p = p1;
                self.last_q = q1;
                self.last_t = t1;
                return Some(es);
            }
            self.t0 *= 2;
            self.dt *= 0.5;
        }
    }
}

impl CubicToEulerIter {
    pub fn new(c: CubicBez, tolerance: f64) -> Self {
        let last_p = c.p0;
        let mut last_q = c.p1 - c.p0;
        if last_q.hypot2() < DERIV_THRESH.powi(2) {
            last_q = eval_and_deriv(&c, DERIV_EPS).1;
        }
        CubicToEulerIter {
            c,
            tolerance,
            t0: 0,
            dt: 1.0,
            last_p,
            last_q,
            last_t: 0.0,
        }
    }
}

/// Integrate Euler spiral.
fn integ_euler_12(k0: f64, k1: f64) -> (f64, f64) {
    let t1_1 = k0;
    let t1_2 = 0.5 * k1;
    let t2_2 = t1_1 * t1_1;
    let t2_3 = 2. * (t1_1 * t1_2);
    let t2_4 = t1_2 * t1_2;
    let t3_4 = t2_2 * t1_2 + t2_3 * t1_1;
    let t3_6 = t2_4 * t1_2;
    let t4_4 = t2_2 * t2_2;
    let t4_5 = 2. * (t2_2 * t2_3);
    let t4_6 = 2. * (t2_2 * t2_4) + t2_3 * t2_3;
    let t4_7 = 2. * (t2_3 * t2_4);
    let t4_8 = t2_4 * t2_4;
    let t5_6 = t4_4 * t1_2 + t4_5 * t1_1;
    let t5_8 = t4_6 * t1_2 + t4_7 * t1_1;
    let t5_10 = t4_8 * t1_2;
    let t6_6 = t4_4 * t2_2;
    let t6_7 = t4_4 * t2_3 + t4_5 * t2_2;
    let t6_8 = t4_4 * t2_4 + t4_5 * t2_3 + t4_6 * t2_2;
    let t6_9 = t4_5 * t2_4 + t4_6 * t2_3 + t4_7 * t2_2;
    let t6_10 = t4_6 * t2_4 + t4_7 * t2_3 + t4_8 * t2_2;
    let t7_8 = t6_6 * t1_2 + t6_7 * t1_1;
    let t7_10 = t6_8 * t1_2 + t6_9 * t1_1;
    let t8_8 = t6_6 * t2_2;
    let t8_9 = t6_6 * t2_3 + t6_7 * t2_2;
    let t8_10 = t6_6 * t2_4 + t6_7 * t2_3 + t6_8 * t2_2;
    let t9_10 = t8_8 * t1_2 + t8_9 * t1_1;
    let t10_10 = t8_8 * t2_2;
    let mut u = 1.;
    u -= (1. / 24.) * t2_2 + (1. / 160.) * t2_4;
    u += (1. / 1920.) * t4_4 + (1. / 10752.) * t4_6 + (1. / 55296.) * t4_8;
    u -= (1. / 322560.) * t6_6 + (1. / 1658880.) * t6_8 + (1. / 8110080.) * t6_10;
    u += (1. / 92897280.) * t8_8 + (1. / 454164480.) * t8_10;
    u -= 2.4464949595157930e-11 * t10_10;
    let mut v = (1. / 12.) * t1_2;
    v -= (1. / 480.) * t3_4 + (1. / 2688.) * t3_6;
    v += (1. / 53760.) * t5_6 + (1. / 276480.) * t5_8 + (1. / 1351680.) * t5_10;
    v -= (1. / 11612160.) * t7_8 + (1. / 56770560.) * t7_10;
    v += 2.4464949595157932e-10 * t9_10;
    (u, v)
}

/// Computation of the Euler spiral integral using subdivision.
fn integ_euler_12n(mut k0: f64, mut k1: f64, n: usize) -> (f64, f64) {
    let th1 = k0;
    let th2 = 0.5 * k1;
    let ds = (n as f64).recip();

    k0 *= ds;
    k1 *= ds;

    let mut x = 0.0;
    let mut y = 0.0;
    let s0 = 0.5 * ds - 0.5;

    for i in 0..n {
        let s = s0 + ds * (i as f64);
        let km0 = k1 * s + k0;
        let km1 = k1 * ds;

        let (u, v) = integ_euler_12(km0, km1);

        let th = (th2 * s + th1) * s;
        let cth = th.cos();
        let sth = th.sin();

        x += cth * u - sth * v;
        y += cth * v + sth * u;
    }
    (x * ds, y * ds)
}

/// Evaluate the Euler spiral integral.
///
/// Compute the following integral to the desired accuracy.
///
/// $$
/// \int_{-0.5}^{0.5} \exp(i(k_0 s + 1/2 k_1 s^2)) ds
/// $$
///
/// This is discussed in section 8.1 of [Raph's thesis], and the error bounds
/// are validated in the notebook attached to the parallel curve blog post.
///
/// [Raph's thesis]: https://www.levien.com/phd/thesis.pdf
pub fn integ_euler(k0: f64, k1: f64, accuracy: f64) -> (f64, f64) {
    let c1 = k1.abs();
    let c0 = k0.abs() + 0.5 * c1;
    let est_err_raw = 0.006 * c0 * c0 + 0.029 * c1;
    // Fun performance note: if the accuracy were always known at compile time,
    // it would be theoretically cheaper to compare against accuracy^(1/6), which
    // is computed anyway in the subdivision case. But the cost of the powi(6) is
    // basically not measurable, and the cost of the ^(1/6) is ballpark double
    // the integration itself.
    if est_err_raw.powi(6) < accuracy {
        integ_euler_12(k0, k1)
    } else {
        let n = (est_err_raw / accuracy.powf(1.0 / 6.0)).ceil() as usize;
        integ_euler_12n(k0, k1, n)
    }
}
