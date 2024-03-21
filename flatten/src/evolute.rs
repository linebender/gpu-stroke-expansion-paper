// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! Math for flattening of Euler evolute

use kurbo::{BezPath, Point};

use crate::euler::{EulerParams, EulerSeg};

fn eval_es_params_evolute(params: &EulerParams, t: f64) -> Point {
    let offset = -1.0 / params.curvature(t);
    params.eval_with_offset(t, offset)
}

fn eval_es_evolute(es: &EulerSeg, t: f64) -> Point {
    let chord = es.p1 - es.p0;
    let Point { x, y } = eval_es_params_evolute(&es.params, t);
    Point::new(
        es.p0.x + chord.x * x - chord.y * y,
        es.p0.y + chord.x * y + chord.y * x,
    )
}

/// Flatten the evolute of an Euler spiral segment.
pub fn flatten_es_evolute(es: &EulerSeg, tol: f64) -> BezPath {
    let mut path = BezPath::new();
    // TODO: smart flatten logic
    const N: usize = 10;
    for i in 0..=N {
        let t = i as f64 / N as f64;
        let p = eval_es_evolute(es, t);
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
pub fn flatten_es(es: &EulerSeg, tol: f64) -> BezPath {
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
