// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

use std::f64::consts::PI;

use kurbo::{Point, SvgArc, Vec2};

/// An arc segment.
///
/// This works the same way as an EulerSeg with symmetrical angles.
#[derive(Clone, Copy, Debug)]
pub struct ArcSegment {
    pub p0: Point,
    pub p1: Point,
    pub k0: f64,
}

impl ArcSegment {
    pub fn new(p0: Point, p1: Point, k0: f64) -> Self {
        ArcSegment { p0, p1, k0 }
    }

    pub fn to_svg_arc(&self) -> SvgArc {
        let radius = if self.k0.abs() < 1e-12 {
            0.0
        } else {
            0.5 * (self.p1 - self.p0).length() / (0.5 * self.k0).sin()
        };
        SvgArc {
            from: self.p0,
            to: self.p1,
            radii: Vec2::new(radius.abs(), radius.abs()),
            x_rotation: 0.0,
            large_arc: self.k0.abs() > PI,
            sweep: self.k0 <= 0.0,
        }
    }
}
