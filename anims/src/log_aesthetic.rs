// Copyright 2023 the Kurbo Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! An implementation of log-aesthetic curves.

// This file is copy-pasted from a draft PR for the linebender Wiki to draw general
// log-aesthetic curves, but we'll only be using it to draw Euler spirals.
#![allow(unused)]

use std::ops::Range;

use vello::kurbo::{
    common::GAUSS_LEGENDRE_COEFFS_32, CurveFitSample, ParamCurve, ParamCurveFit, Point, Vec2,
};

/// Parameters for a log-aesthetic curve segment.
///
/// These parameters determine a segment of unit arc length. The Cesàro equation
/// for the segment is κ(s) = (c1 * s + c0) ^ gamma, where s is in the range
/// [0, 1].
pub struct LogAestheticParams {
    /// The exponent relating arc length to curvature.
    gamma: f64,
    c0: f64,
    c1: f64,
}

pub struct LogAestheticCurve {
    params: LogAestheticParams,
    p0: Point,
    p1: Point,
    scale_rot: Vec2,
    integrals: Vec<Vec2>,
}

impl LogAestheticParams {
    pub fn new(gamma: f64, c0: f64, c1: f64) -> Self {
        LogAestheticParams { gamma, c0, c1 }
    }

    fn theta(&self, t: f64) -> f64 {
        let expt = self.gamma + 1.0;
        (self.c1 * t + self.c0).powf(expt) / (self.c1 * expt)
    }

    // TODO: improve, possibly by using technique in [Ziatdinov2012]
    //
    // [Ziatdinov2012]: https://www.sciencedirect.com/science/article/abs/pii/S0167839611001452
    pub fn integrate(&self, range: Range<f64>) -> Vec2 {
        let dt = range.end - range.start;
        let u1 = 0.5 * dt;
        let mut xy = Vec2::ZERO;
        let u0 = range.start + u1;
        // TODO: probably lower order integration here, increase number of pieces
        for (wi, xi) in GAUSS_LEGENDRE_COEFFS_32 {
            let u = u0 + u1 * xi;
            xy += *wi * Vec2::from_angle(self.theta(u));
        }
        u1 * xy
    }

    /// Curvature of the unit arc length segment
    fn curvature(&self, t: f64) -> f64 {
        (self.c1 * t + self.c0).powf(self.gamma)
    }
}

impl LogAestheticCurve {
    /// Create a new log-aesthetic curve with given parameters and end points.
    pub fn from_points_params(params: LogAestheticParams, p0: Point, p1: Point) -> Self {
        // TODO: estimate based on error metric
        let n = 4;
        let dt = 1.0 / n as f64;
        let mut integral = Vec2::ZERO;
        let integrals = (0..n)
            .map(|i| {
                let piece = params.integrate(i as f64 * dt..(i + 1) as f64 * dt);
                let exclusive = integral;
                integral += piece;
                exclusive
            })
            .collect();
        let uv = integral;
        let uv_scaled = uv / uv.length_squared();
        let d = p1 - p0;
        let scale_rot = Vec2::new(uv_scaled.dot(d), uv_scaled.cross(d));
        LogAestheticCurve {
            params,
            p0,
            p1,
            scale_rot,
            integrals,
        }
    }

    /// We could implement the ParamCurveCurvature trait, but that would involve
    /// building out the analytical derivatives. Possible though!
    pub fn curvature(&self, t: f64) -> f64 {
        self.params.curvature(t) / self.scale_rot.length()
    }

    pub fn eval_with_offset(&self, t: f64, offset: f64) -> Point {
        let (p, q) = self.sample_pt_deriv(t);
        let qscaled = offset * q.normalize();
        p + Vec2::new(-qscaled.y, qscaled.x)
    }

    pub fn evolute(&self) -> LogAestheticCurve {
        // TODO: this function is janky, is only correct for Euler spiral
        // (source gamma = 1).
        let p0 = self.eval_with_offset(0.0, 1.0 / self.curvature(0.0));
        let p1 = self.eval_with_offset(1.0, 1.0 / self.curvature(1.0));
        let gamma = (-1.0 - 2.0 * self.params.gamma) / self.params.gamma;
        let s0 = self.params.c0;
        let s1 = s0 + self.params.c1;
        let l0 = s0.powf(-self.params.gamma);
        let l1 = s1.powf(-self.params.gamma);
        let evolute_len = l0 - l1;
        // uniform scaling parameter
        let a = 1.0 / self.params.c1;
        // curvatures of evolute at endpoints
        let kbar0 = a * a * s0.powi(3) * evolute_len;
        let kbar1 = a * a * s1.powi(3) * evolute_len;
        // solve log-aesthetic parameters for these curvatures
        let sbar0 = (kbar0 / a).powf(1.0 / gamma);
        let sbar01 = (kbar1 / a).powf(1.0 / gamma);

        let c0 = sbar0;
        let c1 = sbar01 - sbar0;
        let params = LogAestheticParams::new(gamma, c0, c1);
        LogAestheticCurve::from_points_params(params, p0, p1)
    }
}

impl ParamCurve for LogAestheticCurve {
    fn eval(&self, t: f64) -> Point {
        if t == 0.0 {
            self.p0
        } else if t == 1.0 {
            self.p1
        } else {
            let n = self.integrals.len();
            let ti = ((n as f64) * t).floor();
            let t0 = ti / n as f64;
            let uv = self.integrals[ti as usize] + self.params.integrate(t0..t);
            let s = self.scale_rot;
            self.p0 + Vec2::new(s.x * uv.x - s.y * uv.y, s.x * uv.y + s.y * uv.x)
        }
    }

    fn start(&self) -> Point {
        self.p0
    }

    fn end(&self) -> Point {
        self.p1
    }

    fn subsegment(&self, range: std::ops::Range<f64>) -> Self {
        // This isn't going to be very hard, but it'll involve a bit of math
        todo!()
    }
}

impl ParamCurveFit for LogAestheticCurve {
    fn sample_pt_tangent(&self, t: f64, _sign: f64) -> CurveFitSample {
        let (p, tangent) = self.sample_pt_deriv(t);
        CurveFitSample { p, tangent }
    }

    fn sample_pt_deriv(&self, t: f64) -> (Point, Vec2) {
        let p = self.eval(t);
        let uv = Vec2::from_angle(self.params.theta(t));
        let s = self.scale_rot;
        let d = Vec2::new(s.x * uv.x - s.y * uv.y, s.x * uv.y + s.y * uv.x);
        (p, d)
    }

    fn break_cusp(&self, _: std::ops::Range<f64>) -> Option<f64> {
        None
    }
}
