// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! Math for flattening of Euler evolute

use kurbo::BezPath;

use crate::euler::EulerSeg;

/// Flatten the evolute of an Euler spiral segment.
pub fn flatten_es_evolute(es: &EulerSeg, _tol: f64) -> BezPath {
    let mut path = BezPath::new();
    // TODO: smart flatten logic
    const N: usize = 10;
    for i in 0..=N {
        let t = i as f64 / N as f64;
        let p = es.eval_evolute(t);
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
