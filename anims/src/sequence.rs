use vello::{
    kurbo::{Affine, Line, Point, Rect, Stroke, Vec2},
    peniko::Color,
    Scene,
};

fn timed(t: &mut f64, duration: f64) -> bool {
    if *t < duration {
        true
    } else {
        *t -= duration;
        false
    }
}

pub fn render(scene: &mut Scene, mut t: f64) {
    if timed(&mut t, 2.0) {
        clocky(scene, t);
    } else {
        end_card(scene);
    }
}

pub fn clocky(scene: &mut Scene, t: f64) {
    let stroke = Stroke::new(6.0);
    let p0 = Point::new(200., 200.);
    let p1 = p0 + 180. * Vec2::from_angle(t);
    let line = Line::new(p0, p1);
    let line_stroke_color = Color::rgb(0.9804, 0.702, 0.5294);
    scene.stroke(&stroke, Affine::IDENTITY, line_stroke_color, None, &line);
    if t < 1.0 {
        scene.stroke(
            &stroke,
            Affine::translate((0., 10.)),
            line_stroke_color,
            None,
            &line,
        );
    }
}

pub fn end_card(scene: &mut Scene) {
    let color = Color::rgb(0.1, 0.1, 0.8);
    let rect = Rect::new(100., 100., 1000., 1000.);
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        &color,
        None,
        &rect,
    );
}
