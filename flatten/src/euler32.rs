// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! Euler spirals with 32 bit coordinates.
//! For now, convert from kurbo-compatible. We'll build up robust
//! 32-bit conversion from Beziers.

use crate::cubic32::{Cubic, Point};

#[derive(Debug)]
pub struct CubicParams {
    pub th0: f32,
    pub th1: f32,
    pub d0: f32,
    pub d1: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct EulerParams {
    pub th0: f32,
    pub th1: f32,
    pub k0: f32,
    pub k1: f32,
    pub ch: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct EulerSeg {
    pub p0: Point,
    pub p1: Point,
    pub params: EulerParams,
}

pub struct CubicToEulerIter {
    c: Cubic,
    tolerance: f32,
    // [t0 * dt .. (t0 + 1) * dt] is the range we're currently considering
    t0: u32,
    dt: f32,
    last_p: Point,
    last_q: Point,
    last_t: f32,
}

/// Threshold below which a derivative is considered too small.
const DERIV_THRESH: f32 = 1e-6;
/// Amount to nudge t when derivative is near-zero.
const DERIV_EPS: f32 = 1e-6;

impl CubicParams {
    pub fn from_points_derivs(p0: Point, p1: Point, q0: Point, q1: Point, dt: f32) -> Self {
        println!("p0={p0:?} p1={p1:?} q0={q0:?} q1={q1:?}");
        let chord = p1 - p0;
        let scale = dt / chord.hypot2();
        let h0 = Point::new(
            q0.x * chord.x + q0.y * chord.y,
            q0.y * chord.x - q0.x * chord.y,
        );
        let th0 = h0.atan2();
        let d0 = h0.hypot() * scale;
        let h1 = Point::new(
            q1.x * chord.x + q1.y * chord.y,
            q1.x * chord.y - q1.y * chord.x,
        );
        println!("h0 = {h0:?} h1 = {h1:?}");
        let th1 = h1.atan2();
        let d1 = h1.hypot() * scale;
        CubicParams { th0, th1, d0, d1 }
    }

    // Estimated error of GH to Euler spiral
    //
    // Return value is normalized to chord - to get actual error, multiply
    // by chord.
    pub fn est_euler_err(&self) -> f32 {
        // Potential optimization: work with unit vector rather than angle
        let cth0 = self.th0.cos();
        let cth1 = self.th1.cos();
        if cth0 * cth1 < 0.0 {
            // Rationale: this happens when fitting a cusp or near-cusp with
            // a near 180 degree u-turn. The actual ES is bounded in that case.
            // Further subdivision won't reduce the angles if actually a cusp.
            return 2.0;
        }
        let e0 = (2. / 3.) / (1.0 + cth0);
        let e1 = (2. / 3.) / (1.0 + cth1);
        let s0 = self.th0.sin();
        let s1 = self.th1.sin();
        // Note: some other versions take sin of s0 + s1 instead. Those are incorrect.
        // Strangely, calibration is the same, but more work could be done.
        //let s01 = (self.th0 + self.th1).sin();
        let s01 = cth0 * s1 + cth1 * s0;
        let amin = 0.15 * (2. * e0 * s0 + 2. * e1 * s1 - e0 * e1 * s01);
        let a = 0.15 * (2. * self.d0 * s0 + 2. * self.d1 * s1 - self.d0 * self.d1 * s01);
        let aerr = (a - amin).abs();
        let symm = (self.th0 + self.th1).abs();
        let asymm = (self.th0 - self.th1).abs();
        let dist = (self.d0 - e0).hypot(self.d1 - e1);
        let ctr = 4.625e-6 * symm.powi(5) + 7.5e-3 * asymm * symm.powi(2);
        let halo_symm = 5e-3 * symm * dist;
        let halo_asymm = 7e-2 * asymm * dist;
        println!("ctr={ctr}, aerr={aerr}, halo_symm={halo_symm}, halo_asymm={halo_asymm}");
        ctr + 1.55 * aerr + halo_symm + halo_asymm
    }
}

impl EulerParams {
    pub fn from_f64(ep: &crate::EulerParams) -> Self {
        EulerParams {
            th0: ep.th0 as f32,
            th1: ep.th1 as f32,
            k0: ep.k0 as f32,
            k1: ep.k1 as f32,
            ch: ep.ch as f32,
        }
    }

    pub fn from_angles(th0: f32, th1: f32) -> EulerParams {
        let k0 = th0 + th1;
        let dth = th1 - th0;
        let d2 = dth * dth;
        let k2 = k0 * k0;
        let mut a = 6.0;
        a -= d2 * (1. / 70.);
        a -= (d2 * d2) * (1. / 10780.);
        a += (d2 * d2 * d2) * 2.769178184818219e-07;
        let b = -0.1 + d2 * (1. / 4200.) + d2 * d2 * 1.6959677820260655e-05;
        let c = -1. / 1400. + d2 * 6.84915970574303e-05 - k2 * 7.936475029053326e-06;
        a += (b + c * k2) * k2;
        let k1 = dth * a;

        // calculation of chord
        let mut ch = 1.0;
        ch -= d2 * (1. / 40.);
        ch += (d2 * d2) * 0.00034226190482569864;
        ch -= (d2 * d2 * d2) * 1.9349474568904524e-06;
        let b = -1. / 24. + d2 * 0.0024702380951963226 - d2 * d2 * 3.7297408997537985e-05;
        let c = 1. / 1920. - d2 * 4.87350869747975e-05 - k2 * 3.1001936068463107e-06;
        ch += (b + c * k2) * k2;
        EulerParams {
            th0,
            th1,
            k0,
            k1,
            ch,
        }
    }

    /// Evaluate theta at given parameter.
    ///
    /// Note: this returns -th0 at t=0 and th1 at t=1. A case can be made that
    /// negating the sign would be more intuitive, but this is the way most of
    /// my Euler spiral code works.
    pub fn eval_th(&self, t: f32) -> f32 {
        (self.k0 + 0.5 * self.k1 * (t - 1.0)) * t - self.th0
    }

    /// Evaluate the curve at the given parameter.
    ///
    /// The parameter is in the range 0..1, and the result goes from (0, 0) to (1, 0).
    pub fn eval(&self, t: f32) -> Point {
        let thm = self.eval_th(t * 0.5);
        let k0 = self.k0;
        let k1 = self.k1;
        let (u, v) = integ_euler_10((k0 + k1 * (0.5 * t - 0.5)) * t, k1 * t * t);
        let s = t / self.ch * thm.sin();
        let c = t / self.ch * thm.cos();
        let x = u * c - v * s;
        let y = -v * c - u * s;
        Point::new(x, y)
    }

    pub fn eval_with_offset(&self, t: f32, offset: f32) -> Point {
        let th = self.eval_th(t);
        let p = self.eval(t);
        Point::new(p.x + offset * th.sin(), p.y + offset * th.cos())
    }
}

impl EulerSeg {
    fn from_f64(es: &crate::euler::EulerSeg) -> Self {
        EulerSeg {
            p0: Point::from_kurbo(es.p0),
            p1: Point::from_kurbo(es.p1),
            params: EulerParams::from_f64(&es.params),
        }
    }

    fn from_params(p0: Point, p1: Point, params: EulerParams) -> Self {
        EulerSeg { p0, p1, params }
    }

    pub fn eval_with_offset(&self, t: f32, offset: f32) -> Point {
        let chord = self.p1 - self.p0;
        let scaled = offset / chord.hypot();
        let Point { x, y } = self.params.eval_with_offset(t, scaled);
        Point::new(
            self.p0.x + chord.x * x - chord.y * y,
            self.p0.y + chord.x * y + chord.y * x,
        )
    }
}

impl Iterator for CubicToEulerIter {
    type Item = EulerSeg;

    fn next(&mut self) -> Option<EulerSeg> {
        let t0 = (self.t0 as f32) * self.dt;
        if t0 == 1.0 {
            return None;
        }
        loop {
            let mut t1 = t0 + self.dt;
            let p0 = self.last_p;
            let q0 = self.last_q;
            let (mut p1, mut q1) = self.c.eval_and_deriv(t1);
            if q1.hypot2() < DERIV_THRESH.powi(2) {
                let (p1_, q1_) = self.c.eval_and_deriv(t1 - DERIV_EPS);
                q1 = q1_;
                if t1 < 1.0 {
                    p1 = p1_;
                    t1 = t1 - DERIV_EPS;
                }
            }
            println!(
                "t0={t0}, last_t = {}, dt={} tweaked t1={t1}",
                self.last_t, self.dt
            );
            let actual_dt = t1 - self.last_t;
            let cubic_params = CubicParams::from_points_derivs(p0, p1, q0, q1, actual_dt);
            println!("{cubic_params:?}");
            let est_err = cubic_params.est_euler_err();
            let err = est_err * (p1 - p0).hypot();
            println!(
                "{t0}..{t1}: err={err}, raw {est_err} scale {}",
                (p1 - p0).hypot()
            );
            if err <= self.tolerance {
                self.t0 += 1;
                let shift = self.t0.trailing_zeros();
                self.t0 >>= shift;
                self.dt *= (1 << shift) as f32;
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
    pub fn new(c: Cubic, tolerance: f32) -> Self {
        let last_p = c.p0;
        let mut last_q = c.p1 - c.p0;
        if last_q.hypot2() < DERIV_THRESH.powi(2) {
            last_q = c.eval_and_deriv(DERIV_EPS).1;
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
///
/// TODO: investigate needed accuracy. We might be able to get away
/// with 8th order.
fn integ_euler_10(k0: f32, k1: f32) -> (f32, f32) {
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
    let t6_6 = t4_4 * t2_2;
    let t6_7 = t4_4 * t2_3 + t4_5 * t2_2;
    let t6_8 = t4_4 * t2_4 + t4_5 * t2_3 + t4_6 * t2_2;
    let t7_8 = t6_6 * t1_2 + t6_7 * t1_1;
    let t8_8 = t6_6 * t2_2;
    let mut u = 1.;
    u -= (1. / 24.) * t2_2 + (1. / 160.) * t2_4;
    u += (1. / 1920.) * t4_4 + (1. / 10752.) * t4_6 + (1. / 55296.) * t4_8;
    u -= (1. / 322560.) * t6_6 + (1. / 1658880.) * t6_8;
    u += (1. / 92897280.) * t8_8;
    let mut v = (1. / 12.) * t1_2;
    v -= (1. / 480.) * t3_4 + (1. / 2688.) * t3_6;
    v += (1. / 53760.) * t5_6 + (1. / 276480.) * t5_8;
    v -= (1. / 11612160.) * t7_8;
    (u, v)
}
