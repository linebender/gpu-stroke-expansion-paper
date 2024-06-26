// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! CPU-side stroke to fill expansion.
//!
//! Much of the contents of this file is copy-pasted from kurbo, but adapted for lowering
//! to lines or arcs.

use std::fmt::Write;
use std::{f64::consts::PI, ops::Range};

use kurbo::{
    common::solve_quadratic, Affine, Arc, BezPath, Cap, CubicBez, Join, Line, PathEl, PathSeg,
    Point, QuadBez, Stroke, Vec2,
};

use crate::{
    arc_segment::ArcSegment,
    euler::{CubicToEulerIter, EulerSeg},
    flatten::flatten_offset,
};

pub trait Lowering: Sized + Copy + Clone + core::fmt::Debug {
    // Possibly useful for debugging but not currently used.
    fn to_bez(&self, path: &mut BezPath);

    fn to_svg_string(&self, svg: &mut String, last_pt: Option<Point>);

    fn line(line: Line) -> Self;

    fn lower_arc(arc: &ArcSegment, segs: &mut Vec<Self>, tol: f64);

    // Note: assumes last_pt in the path is set, and is continuous with it.
    fn lower_espc(
        es: &EulerSeg,
        path: &mut LoweredPath<Self>,
        range: Range<f64>,
        dist: f64,
        tol: f64,
    );

    fn lower_es_evolute(es: &EulerSeg, path: &mut LoweredPath<Self>, range: Range<f64>, tol: f64) {
        let range_size = range.end - range.start;
        let arc_len = (es.p1 - es.p0).length() / es.params.ch;
        let ratio = es.params.k0 / es.params.k1;
        let rho_int_0 = (0.5 * (ratio + range.start - 0.5)).abs().sqrt();
        let rho_int_1 = (0.5 * (ratio + range.end - 0.5)).abs().sqrt();
        let rho_int = rho_int_1 - rho_int_0;
        let n_subdiv = (range_size * rho_int.abs() * (arc_len / tol).sqrt())
            .ceil()
            .max(1.0);
        let n = n_subdiv as usize;
        let sign2 = 2.0f64.copysign(ratio);
        for i in 1..=n {
            let t = i as f64 / n_subdiv;
            let u = rho_int_0 + t * rho_int;
            let s = range.start + range_size * (sign2 * u * u + 0.5 - ratio);
            path.line_to(es.eval_evolute(s));
        }
    }

    fn reverse(self) -> Self;

    fn end_point(self) -> Point;
}

#[derive(Debug)]
pub struct LoweredPath<L: Lowering> {
    pub path: Vec<L>,
    last_pt: Point,
}

struct StrokeContour<L: Lowering> {
    main_path: LoweredPath<L>,
    // the next two items are in play past the cusp
    rev_parallel: Option<LoweredPath<L>>,
    evolute: Option<LoweredPath<L>>,
}

impl<L: Lowering> Default for LoweredPath<L> {
    fn default() -> Self {
        LoweredPath {
            path: vec![],
            last_pt: Point::default(),
        }
    }
}

impl<L: Lowering> Default for StrokeContour<L> {
    fn default() -> Self {
        StrokeContour {
            main_path: Default::default(),
            rev_parallel: None,
            evolute: None,
        }
    }
}

impl<L: Lowering> LoweredPath<L> {
    fn move_to(&mut self, p: Point) {
        self.last_pt = p;
    }

    fn line_to(&mut self, p: Point) {
        self.path.push(L::line(Line::new(self.last_pt, p)));
        self.last_pt = p;
    }

    #[allow(unused)]
    fn to_bez(&self) -> BezPath {
        let mut result = BezPath::new();
        for seg in &self.path {
            seg.to_bez(&mut result);
        }
        result
    }

    pub fn to_svg(&self) -> String {
        let mut svg = String::new();
        let mut last_pt = None;
        for seg in &self.path {
            seg.to_svg_string(&mut svg, last_pt);
            last_pt = Some(seg.end_point());
        }
        svg
    }

    fn extend(&mut self, other: &Self) {
        if !other.path.is_empty() {
            self.path.extend(&other.path);
            self.last_pt = self.path.last().unwrap().end_point();
        }
    }
    fn extend_rev(&mut self, other: &Self) {
        if !other.path.is_empty() {
            for seg in other.path.iter().rev() {
                self.path.push(seg.reverse());
            }
            self.last_pt = self.path.last().unwrap().end_point();
        }
    }
}

impl Lowering for Line {
    fn to_bez(&self, path: &mut BezPath) {
        let need_move_to = match path.elements().last() {
            None => true,
            Some(el) => el.end_point() != Some(self.p0),
        };
        if need_move_to {
            path.move_to(self.p0);
        }
        path.line_to(self.p1);
    }

    fn to_svg_string(&self, svg: &mut String, last_pt: Option<Point>) {
        if Some(self.p0) != last_pt {
            _ = write!(svg, "M{:.3} {:.3}", self.p0.x, self.p0.y);
        }
        _ = write!(svg, "L{:.3} {:.3}", self.p1.x, self.p1.y);
    }

    fn line(line: Line) -> Self {
        line
    }

    fn lower_arc(arc: &ArcSegment, segs: &mut Vec<Self>, tol: f64) {
        let chord = arc.p1 - arc.p0;
        let chord_len = chord.length();
        let n_frac = arc.k0.abs() * (0.125 * chord_len / tol).sqrt();
        if n_frac <= 1.0 {
            segs.push(Line::new(arc.p0, arc.p1));
            return;
        }
        let n_ceil = n_frac.ceil();
        let n = n_ceil as usize;
        let mut p0 = arc.p0;
        let (s, c) = (0.5 * arc.k0).sin_cos();
        let scale = 0.5 / s;
        for i in 1..n {
            // The code in flatten.wgsl cleverly uses matrices to rotate
            let th = ((i as f64) / n_ceil - 0.5) * arc.k0;
            let u = th.sin() * scale + 0.5;
            let v = (th.cos() - c) * scale;
            let p1 = arc.p0 + Vec2::new(u * chord.x - v * chord.y, u * chord.y + v * chord.x);
            segs.push(Line::new(p0, p1));
            p0 = p1;
        }
        segs.push(Line::new(p0, arc.p1));
    }

    fn lower_espc(
        es: &EulerSeg,
        path: &mut LoweredPath<Self>,
        range: Range<f64>,
        dist: f64,
        tol: f64,
    ) {
        flatten_offset(es, range, dist, tol, |p| path.line_to(p));
    }

    fn reverse(self) -> Self {
        Line::new(self.p1, self.p0)
    }

    fn end_point(self) -> Point {
        self.p1
    }
}

impl Lowering for ArcSegment {
    #[allow(unused)]
    fn to_bez(&self, path: &mut BezPath) {
        let need_move_to = match path.elements().last() {
            None => true,
            Some(el) => el.end_point() != Some(self.p0),
        };
        if need_move_to {
            path.move_to(self.p0);
        }
        if self.k0.abs() < 1e-12 {
            path.line_to(self.p1);
        } else {
            if let Some(arc) = Arc::from_svg_arc(&self.to_svg_arc()) {
                path.extend(arc.append_iter(0.1));
            }
        }
    }

    fn to_svg_string(&self, svg: &mut String, last_pt: Option<Point>) {
        if Some(self.p0) != last_pt {
            _ = write!(svg, "M{:.3} {:.3}", self.p0.x, self.p0.y);
        }
        if self.k0.abs() < 1e-12 {
            _ = write!(svg, "L{:.3} {:.3}", self.p1.x, self.p1.y);
        } else {
            let svg_arc = self.to_svg_arc();
            _ = write!(
                svg,
                "A{:.3} {:.3} 0 {} {} {:.3} {:.3}",
                svg_arc.radii.x,
                svg_arc.radii.y,
                svg_arc.large_arc as usize,
                svg_arc.sweep as usize,
                svg_arc.to.x,
                svg_arc.to.y
            );
        }
    }

    fn line(line: Line) -> Self {
        ArcSegment::new(line.p0, line.p1, 0.0)
    }

    fn lower_arc(arc: &ArcSegment, segs: &mut Vec<Self>, _tol: f64) {
        segs.push(*arc);
    }

    fn lower_espc(
        es: &EulerSeg,
        path: &mut LoweredPath<Self>,
        range: Range<f64>,
        dist: f64,
        tol: f64,
    ) {
        let range_size = range.end - range.start;
        let arclen = es.p0.distance(es.p1) / es.params.ch;
        let est_err =
            (1. / 120.) / tol * es.params.k1.abs() * (arclen + 0.4 * (es.params.k1 * dist).abs());
        let n_subdiv = est_err.cbrt() * range_size;
        let n = (n_subdiv.ceil() as usize).max(1);
        let dt = range_size / n as f64;
        let mut p0 = path.last_pt;
        for i in 0..n {
            let t0 = range.start + i as f64 * dt;
            let t1 = t0 + dt;
            let p1 = es.eval_with_offset(t1, dist);
            let t = t0 + 0.5 * dt - 0.5;
            let k = es.params.k0 + t * es.params.k1;
            path.path.push(ArcSegment::new(p0, p1, k * dt));
            p0 = p1;
        }
        path.last_pt = p0;
    }

    fn lower_es_evolute(es: &EulerSeg, path: &mut LoweredPath<Self>, range: Range<f64>, tol: f64) {
        let range_size = range.end - range.start;
        let arc_len = (es.p1 - es.p0).length() / es.params.ch;
        let k0 = es.params.k0;
        let k1 = es.params.k1;
        let rho_int_0 = (1. / (k0 + (range.start - 0.5) * k1)).cbrt();
        let rho_int_1 = (1. / (k0 + (range.end - 0.5) * k1)).cbrt();
        let rho_int = rho_int_1 - rho_int_0;
        let est_err = rho_int.abs() * arc_len / tol;
        // TODO: apply scaling factor to est_err to tune actual error.
        // Math suggests 27. / 40. but eyeball error seems too large.
        let n_subdiv = (est_err.cbrt() * range_size).ceil().max(1.0);
        let n = n_subdiv as usize;
        let mut p0 = path.last_pt;
        let mut last_k = k0 + (range.start - 0.5) * k1;
        let mut last_r = last_k.abs().sqrt();
        let sign4 = 4.0f64.copysign(-k0);
        for i in 1..=n {
            let t = i as f64 / n_subdiv;
            let u = rho_int_0 + t * rho_int;
            let s_0_1 = (1. / u.powi(3) - k0) / k1 + 0.5;
            let s = range.start + range_size * s_0_1;
            let p1 = es.eval_evolute(s);
            let k = k0 + (s - 0.5) * k1;
            let r = k.abs().sqrt();
            let es_k = sign4 * (r - last_r).powi(2) / (k1 / k - k1 / last_k);
            path.path.push(ArcSegment::new(p0, p1, es_k));
            last_k = k;
            last_r = r;
            p0 = p1;
        }
        path.last_pt = p0;
    }

    fn reverse(self) -> Self {
        ArcSegment::new(self.p1, self.p0, -self.k0)
    }

    fn end_point(self) -> Point {
        self.p1
    }
}

// Copied from kurbo, the method is private
fn pathseg_tangents(seg: &PathSeg) -> (Vec2, Vec2) {
    const EPS: f64 = 1e-12;
    match seg {
        PathSeg::Line(l) => {
            let d = l.p1 - l.p0;
            (d, d)
        }
        PathSeg::Quad(q) => {
            let d01 = q.p1 - q.p0;
            let d0 = if d01.hypot2() > EPS { d01 } else { q.p2 - q.p0 };
            let d12 = q.p2 - q.p1;
            let d1 = if d12.hypot2() > EPS { d12 } else { q.p2 - q.p0 };
            (d0, d1)
        }
        PathSeg::Cubic(c) => {
            let d01 = c.p1 - c.p0;
            let d0 = if d01.hypot2() > EPS {
                d01
            } else {
                let d02 = c.p2 - c.p0;
                if d02.hypot2() > EPS {
                    d02
                } else {
                    c.p3 - c.p0
                }
            };
            let d23 = c.p3 - c.p2;
            let d1 = if d23.hypot2() > EPS {
                d23
            } else {
                let d13 = c.p3 - c.p1;
                if d13.hypot2() > EPS {
                    d13
                } else {
                    c.p3 - c.p0
                }
            };
            (d0, d1)
        }
    }
}

/// Internal structure used for creating strokes.
struct StrokeCtx<L: Lowering> {
    // As a possible future optimization, we might not need separate storage
    // for forward and backward paths, we can add forward to the output in-place.
    // However, this structure is clearer and the cost fairly modest.
    output: LoweredPath<L>,
    forward_path: StrokeContour<L>,
    backward_path: StrokeContour<L>,
    start_pt: Point,
    start_norm: Vec2,
    start_tan: Vec2,
    last_pt: Point,
    last_tan: Vec2,
    tolerance: f64,
    // Precomputation of the join threshold, to optimize per-join logic.
    // If hypot < (hypot + dot) * join_thresh, omit join altogether.
    join_thresh: f64,
}

/// Version of stroke expansion for styles with no dashes.
pub fn stroke_undashed<L: Lowering>(
    path: impl IntoIterator<Item = PathEl>,
    style: &Stroke,
    tolerance: f64,
) -> LoweredPath<L> {
    let mut ctx = StrokeCtx {
        join_thresh: 2.0 * tolerance / style.width,
        tolerance,
        output: Default::default(),
        forward_path: Default::default(),
        backward_path: Default::default(),
        start_pt: Default::default(),
        start_norm: Default::default(),
        start_tan: Default::default(),
        last_pt: Default::default(),
        last_tan: Default::default(),
    };
    for el in path {
        let p0 = ctx.last_pt;
        match el {
            PathEl::MoveTo(p) => {
                ctx.finish(style);
                ctx.start_pt = p;
                ctx.last_pt = p;
            }
            PathEl::LineTo(p1) => {
                if p1 != p0 {
                    let tangent = p1 - p0;
                    ctx.do_join(style, tangent);
                    ctx.last_tan = tangent;
                    ctx.do_line(style, tangent, p1);
                }
            }
            PathEl::QuadTo(p1, p2) => {
                if p1 != p0 || p2 != p0 {
                    let q = QuadBez::new(p0, p1, p2);
                    let (tan0, tan1) = pathseg_tangents(&PathSeg::Quad(q));
                    ctx.do_join(style, tan0);
                    ctx.do_cubic(style, q.raise());
                    ctx.last_tan = tan1;
                }
            }
            PathEl::CurveTo(p1, p2, p3) => {
                if p1 != p0 || p2 != p0 || p3 != p0 {
                    let c = CubicBez::new(p0, p1, p2, p3);
                    let (tan0, tan1) = pathseg_tangents(&PathSeg::Cubic(c));
                    ctx.do_join(style, tan0);
                    ctx.do_cubic(style, c);
                    ctx.last_tan = tan1;
                }
            }
            PathEl::ClosePath => {
                if p0 != ctx.start_pt {
                    let tangent = ctx.start_pt - p0;
                    ctx.do_join(style, tangent);
                    ctx.last_tan = tangent;
                    ctx.do_line(style, tangent, ctx.start_pt);
                }
                ctx.finish_closed(style);
            }
        }
    }
    ctx.finish(style);
    ctx.output
}

fn round_cap<L: Lowering>(out: &mut LoweredPath<L>, tolerance: f64, center: Point, norm: Vec2) {
    round_join(out, tolerance, center, norm, PI);
}

fn round_join<L: Lowering>(
    out: &mut LoweredPath<L>,
    tolerance: f64,
    center: Point,
    norm: Vec2,
    angle: f64,
) {
    let p1 = center - norm;
    let arc = ArcSegment::new(out.last_pt, p1, -angle);
    L::lower_arc(&arc, &mut out.path, tolerance);
    out.last_pt = p1;
}

fn square_cap<L: Lowering>(out: &mut LoweredPath<L>, center: Point, norm: Vec2) {
    let a = Affine::new([norm.x, norm.y, -norm.y, norm.x, center.x, center.y]);
    out.line_to(a * Point::new(1.0, 1.0));
    out.line_to(a * Point::new(-1.0, 1.0));
    out.line_to(center - norm);
}

impl<L: Lowering> StrokeCtx<L> {
    /// Append forward and backward paths to output.
    fn finish(&mut self, style: &Stroke) {
        if self.forward_path.main_path.path.is_empty() {
            return;
        }
        self.forward_path.finalize();
        self.output.extend(&self.forward_path.main_path);
        let return_p = if let Some(revp) = &self.backward_path.rev_parallel {
            revp.last_pt
        } else {
            self.backward_path.main_path.last_pt
        };
        let d = self.last_pt - return_p;
        match style.end_cap {
            Cap::Butt => self.output.line_to(return_p),
            Cap::Round => round_cap(&mut self.output, self.tolerance, self.last_pt, d),
            Cap::Square => square_cap(&mut self.output, self.last_pt, d),
        }
        self.backward_path.finalize();
        self.output.extend_rev(&self.backward_path.main_path);
        match style.start_cap {
            Cap::Butt => self.output.line_to(self.start_pt - self.start_norm),
            Cap::Round => round_cap(
                &mut self.output,
                self.tolerance,
                self.start_pt,
                self.start_norm,
            ),
            Cap::Square => square_cap(&mut self.output, self.start_pt, self.start_norm),
        }

        self.forward_path.main_path.path.clear();
        self.backward_path.main_path.path.clear();
    }

    /// Finish a closed path
    fn finish_closed(&mut self, style: &Stroke) {
        if self.forward_path.main_path.path.is_empty() {
            return;
        }
        self.do_join(style, self.start_tan);
        self.forward_path.finalize();
        self.output.extend(&self.forward_path.main_path);
        self.backward_path.finalize();
        self.output.extend_rev(&self.backward_path.main_path);
        self.forward_path.main_path.path.clear();
        self.backward_path.main_path.path.clear();
    }

    fn do_join(&mut self, style: &Stroke, tan0: Vec2) {
        let scale = 0.5 * style.width / tan0.hypot();
        let norm = scale * Vec2::new(-tan0.y, tan0.x);
        let p0 = self.last_pt;
        if self.forward_path.is_empty() {
            self.forward_path.move_to(p0 - norm);
            self.backward_path.move_to(p0 + norm);
            self.start_tan = tan0;
            self.start_norm = norm;
        } else {
            let ab = self.last_tan;
            let cd = tan0;
            let cross = ab.cross(cd);
            let dot = ab.dot(cd);
            let hypot = cross.hypot(dot);
            // possible TODO: a minor speedup could be squaring both sides
            if dot <= 0.0 || cross.abs() >= hypot * self.join_thresh {
                match style.join {
                    Join::Bevel => {
                        self.forward_path.line_to(p0 - norm);
                        self.backward_path.line_to(p0 + norm);
                    }
                    Join::Miter => {
                        if 2.0 * hypot < (hypot + dot) * style.miter_limit.powi(2) {
                            // TODO: maybe better to store last_norm or derive from path?
                            let last_scale = 0.5 * style.width / ab.hypot();
                            let last_norm = last_scale * Vec2::new(-ab.y, ab.x);
                            if cross > 0.0 {
                                let fp_last = p0 - last_norm;
                                let fp_this = p0 - norm;
                                let h = ab.cross(fp_this - fp_last) / cross;
                                let miter_pt = fp_this - cd * h;
                                self.forward_path.line_to(miter_pt);
                            } else if cross < 0.0 {
                                let fp_last = p0 + last_norm;
                                let fp_this = p0 + norm;
                                let h = ab.cross(fp_this - fp_last) / cross;
                                let miter_pt = fp_this - cd * h;
                                self.backward_path.line_to(miter_pt);
                            }
                        }
                        self.forward_path.line_to(p0 - norm);
                        self.backward_path.line_to(p0 + norm);
                    }
                    Join::Round => {
                        let angle = cross.atan2(dot);
                        if angle > 0.0 {
                            self.backward_path.line_to(p0 + norm);
                            // ok to touch main path, line_to caused finalize
                            round_join(
                                &mut self.forward_path.main_path,
                                self.tolerance,
                                p0,
                                norm,
                                angle,
                            );
                        } else {
                            self.forward_path.line_to(p0 - norm);
                            round_join(
                                &mut self.backward_path.main_path,
                                self.tolerance,
                                p0,
                                -norm,
                                angle,
                            );
                        }
                    }
                }
            }
        }
    }

    fn do_line(&mut self, style: &Stroke, tangent: Vec2, p1: Point) {
        let scale = 0.5 * style.width / tangent.hypot();
        let norm = scale * Vec2::new(-tangent.y, tangent.x);
        self.forward_path.line_to(p1 - norm);
        self.backward_path.line_to(p1 + norm);
        self.last_pt = p1;
    }

    fn do_cubic(&mut self, style: &Stroke, c: CubicBez) {
        // Discussion: do we need to detect linear case?
        // First, detect degenerate linear case

        // Ordinarily, this is the direction of the chord, but if the chord is very
        // short, we take the longer control arm.
        let chord = c.p3 - c.p0;
        let mut chord_ref = chord;
        let mut chord_ref_hypot2 = chord_ref.hypot2();
        let d01 = c.p1 - c.p0;
        if d01.hypot2() > chord_ref_hypot2 {
            chord_ref = d01;
            chord_ref_hypot2 = chord_ref.hypot2();
        }
        let d23 = c.p3 - c.p2;
        if d23.hypot2() > chord_ref_hypot2 {
            chord_ref = d23;
            chord_ref_hypot2 = chord_ref.hypot2();
        }
        // Project Bézier onto chord
        let p0 = c.p0.to_vec2().dot(chord_ref);
        let p1 = c.p1.to_vec2().dot(chord_ref);
        let p2 = c.p2.to_vec2().dot(chord_ref);
        let p3 = c.p3.to_vec2().dot(chord_ref);
        const ENDPOINT_D: f64 = 0.01;
        if p3 <= p0
            || p1 > p2
            || p1 < p0 + ENDPOINT_D * (p3 - p0)
            || p2 > p3 - ENDPOINT_D * (p3 - p0)
        {
            // potentially a cusp inside
            let x01 = d01.cross(chord_ref);
            let x23 = d23.cross(chord_ref);
            let x03 = chord.cross(chord_ref);
            let thresh = self.tolerance.powi(2) * chord_ref_hypot2;
            if x01 * x01 < thresh && x23 * x23 < thresh && x03 * x03 < thresh {
                // control points are nearly co-linear
                let midpoint = c.p0.midpoint(c.p3);
                // Mapping back from projection of reference chord
                let ref_vec = chord_ref / chord_ref_hypot2;
                let ref_pt = midpoint - 0.5 * (p0 + p3) * ref_vec;
                self.do_linear(style, c, [p0, p1, p2, p3], ref_pt, ref_vec);
                return;
            }
        }

        // TODO: fine-tune relative tolerance for cubic to Euler and lowering
        let es_tol = self.tolerance;
        let lower_tol = self.tolerance;
        for es in CubicToEulerIter::new(c, es_tol) {
            self.forward_path
                .do_euler_seg(&es, -0.5 * style.width, lower_tol);
            self.backward_path
                .do_euler_seg(&es, 0.5 * style.width, lower_tol);
        }
        self.last_pt = c.p3;
    }

    /// Do a cubic which is actually linear.
    ///
    /// The `p` argument is the control points projected to the reference chord.
    /// The ref arguments are the inverse map of a projection back to the client
    /// coordinate space.
    fn do_linear(
        &mut self,
        style: &Stroke,
        c: CubicBez,
        p: [f64; 4],
        ref_pt: Point,
        ref_vec: Vec2,
    ) {
        // Always do round join, to model cusp as limit of finite curvature (see Nehab).
        let style = Stroke::new(style.width).with_join(Join::Round);
        // Tangents of endpoints (for connecting to joins)
        let (tan0, tan1) = pathseg_tangents(&PathSeg::Cubic(c));
        self.last_tan = tan0;
        // find cusps
        let c0 = p[1] - p[0];
        let c1 = 2.0 * p[2] - 4.0 * p[1] + 2.0 * p[0];
        let c2 = p[3] - 3.0 * p[2] + 3.0 * p[1] - p[0];
        let roots = solve_quadratic(c0, c1, c2);
        // discard cusps right at endpoints
        const EPSILON: f64 = 1e-6;
        for t in roots {
            if t > EPSILON && t < 1.0 - EPSILON {
                let mt = 1.0 - t;
                let z = mt * (mt * mt * p[0] + 3.0 * t * (mt * p[1] + t * p[2])) + t * t * t * p[3];
                let p = ref_pt + z * ref_vec;
                let tan = p - self.last_pt;
                self.do_join(&style, tan);
                self.do_line(&style, tan, p);
                self.last_tan = tan;
            }
        }
        let tan = c.p3 - self.last_pt;
        self.do_join(&style, tan);
        self.do_line(&style, tan, c.p3);
        self.last_tan = tan;
        self.do_join(&style, tan1);
    }
}

impl<L: Lowering + core::fmt::Debug> StrokeContour<L> {
    fn do_euler_seg(&mut self, es: &EulerSeg, h: f64, tolerance: f64) {
        // TODO: make evolutes optional
        let chord_len = (es.p1 - es.p0).length();
        let cusp0 = es.params.curvature(0.) * h + chord_len;
        let cusp1 = es.params.curvature(1.) * h + chord_len;
        let t = if cusp0 * cusp1 >= 0.0 {
            // no cusp in this segment
            1.0
        } else {
            // location of cusp
            cusp0 / (cusp0 - cusp1)
        };
        if cusp0 >= 0.0 {
            self.finalize();
            L::lower_espc(es, &mut self.main_path, 0.0..t, h, tolerance);
            if t < 1.0 {
                let (evolute, rev_parallel) = self.start();
                L::lower_es_evolute(es, evolute, t..1.0, tolerance);
                L::lower_espc(es, rev_parallel, t..1.0, h, tolerance);
            }
        } else {
            let (evolute, rev_parallel) = self.start();
            evolute.line_to(es.eval_evolute(0.));
            L::lower_es_evolute(es, evolute, 0.0..t, tolerance);
            L::lower_espc(es, rev_parallel, 0.0..t, h, tolerance);
            if t < 1.0 {
                self.finalize();
                L::lower_espc(es, &mut self.main_path, t..1.0, h, tolerance);
            }
        }
    }

    fn start(&mut self) -> (&mut LoweredPath<L>, &mut LoweredPath<L>) {
        let evolute = self.evolute.get_or_insert_with(|| {
            let mut evolute = LoweredPath::default();
            evolute.move_to(self.main_path.last_pt);
            evolute
        });
        let rev_parallel = self.rev_parallel.get_or_insert_with(|| {
            let mut rev_parallel = LoweredPath::default();
            rev_parallel.move_to(self.main_path.last_pt);
            rev_parallel
        });
        (evolute, rev_parallel)
    }

    fn finalize(&mut self) {
        if let Some(evolute) = &mut self.evolute {
            let last_pt = self.rev_parallel.as_ref().unwrap().last_pt;
            evolute.line_to(last_pt);
            self.main_path.extend(evolute);
            self.main_path
                .extend_rev(self.rev_parallel.as_ref().unwrap());
            self.main_path.extend(evolute);
            self.evolute = None;
            self.rev_parallel = None;
        }
    }

    fn move_to(&mut self, p: Point) {
        self.finalize();
        self.main_path.move_to(p);
    }

    fn line_to(&mut self, p: Point) {
        self.finalize();
        self.main_path.line_to(p);
    }

    fn is_empty(&self) -> bool {
        self.main_path.path.is_empty()
    }
}

pub fn stroke_main() {
    let path = BezPath::from_svg("M 10 10 Q 50 10 100 20 Q 100 100 50 100").unwrap();
    //let path = BezPath::from_svg("M 10 10 L 100 20 L 50 100").unwrap();
    let style = Stroke::new(50.0).with_caps(Cap::Round);
    let stroked: LoweredPath<ArcSegment> = stroke_undashed(path, &style, 1.0);
    println!("{}", stroked.to_svg());
    //println!("{} segments", stroked.path.len());
}
