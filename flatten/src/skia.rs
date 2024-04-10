// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! Testbed for evaluating Skia's stroke expansion utility.

use std::time::{Duration, Instant};

use kurbo::{BezPath, PathEl, Point, Stroke};
use skia_safe::{path_utils::fill_path_with_paint, Color4f, Paint, Path};

fn skpt(p: Point) -> skia_safe::Point {
    (p.x as f32, p.y as f32).into()
}

fn kpt(p: skia_safe::Point) -> Point {
    Point::new(p.x as f64, p.y as f64)
}

fn kurbo_to_skia(path: &BezPath) -> Path {
    let mut sk_path = Path::new();
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => _ = sk_path.move_to(skpt(*p)),
            PathEl::LineTo(p) => _ = sk_path.line_to(skpt(*p)),
            PathEl::QuadTo(p1, p2) => _ = sk_path.quad_to(skpt(*p1), skpt(*p2)),
            PathEl::CurveTo(p1, p2, p3) => _ = sk_path.cubic_to(skpt(*p1), skpt(*p2), skpt(*p3)),
            PathEl::ClosePath => _ = sk_path.close(),
        }
    }
    sk_path
}

fn skia_to_kurbo(sk_path: &Path) -> BezPath {
    let mut bez_path = BezPath::new();
    let n_points = sk_path.count_points();
    let n_verbs = sk_path.count_verbs();
    let mut points = vec![skia_safe::Point::default(); n_points];
    let mut verbs = vec![0u8; n_verbs];
    sk_path.get_points(&mut points);
    sk_path.get_verbs(&mut verbs);
    let mut j = 0;
    for verb in &verbs {
        match verb {
            0 => {
                bez_path.move_to(kpt(points[j]));
                j += 1;
            }
            1 => {
                bez_path.line_to(kpt(points[j]));
                j += 1;
            }
            2 => {
                bez_path.quad_to(kpt(points[j]), kpt(points[j + 1]));
                j += 2;
            }
            4 => {
                bez_path.curve_to(kpt(points[j]), kpt(points[j + 1]), kpt(points[j + 2]));
                j += 3;
            }
            5 => bez_path.close_path(),
            _ => (),
        }
    }
    bez_path
}

pub fn stroke_expand(path: &BezPath, style: &Stroke) -> (BezPath, Duration) {
    let sk_path = kurbo_to_skia(path);
    let mut dst = Path::new();
    let mut paint = Paint::new(Color4f::new(0.0, 0.0, 0.0, 1.0), None);
    paint.set_style(skia_safe::PaintStyle::Stroke);
    paint.set_stroke_width(style.width as f32);
    // TODO: caps and joins
    let start = Instant::now();
    fill_path_with_paint(&sk_path, &paint, &mut dst, None, None);
    let elapsed = start.elapsed();
    let result = skia_to_kurbo(&dst);
    (result, elapsed)
}
