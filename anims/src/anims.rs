use flatten::stroke::LoweredPath;
use vello::{
    kurbo::{Affine, Cap, CubicBez, Line, ParamCurve, Rect, Shape, Stroke},
    peniko::Color,
    Scene,
};

pub struct Anims;

fn timed(t: &mut f64, duration: f64) -> bool {
    if *t < duration {
        true
    } else {
        *t -= duration;
        false
    }
}

const STROKE_LEN: f64 = 5.0;

impl Anims {
    pub fn new() -> Self {
        Anims
    }

    pub fn render(&self, scene: &mut Scene, mut t: f64) {
        if timed(&mut t, STROKE_LEN) {
            anim_stroke(scene, t / STROKE_LEN);
        } else {
            end_card(scene);
        }
    }    
}


fn anim_stroke(scene: &mut Scene, t: f64) {
    let c = CubicBez::new((110., 290.), (110., 250.), (110., 160.), (140., 160.));
    let c = Affine::scale(3.) * c;
    let t_adjust = (t * 1.2).min(1.0);
    let trimmed = c.subsegment(0.0..t_adjust);
    let flatten_style = Stroke::new(300.0).with_caps(Cap::Butt);
    let path = trimmed.to_path(1e-9);
    let stroked: LoweredPath<Line> = flatten::stroke::stroke_undashed(path, &flatten_style, 0.1);
    let stroked_path = stroked.to_bez();
    let stroke_color = Color::rgb(0.9804, 0.702, 0.5294);
    let thin_stroke_color = Color::rgb(0.5, 0.5, 1.);
    let fill_color = Color::rgb8(0xff, 0x93, 0x8d);
    let stroke = Stroke::new(6.0);
    let thin_stroke = Stroke::new(4.0);
    scene.fill(vello::peniko::Fill::NonZero, Affine::IDENTITY, fill_color, None, &stroked_path);
    scene.stroke(&stroke, Affine::IDENTITY, stroke_color, None, &stroked_path);
    scene.stroke(&thin_stroke, Affine::IDENTITY, thin_stroke_color, None, &trimmed);
}

fn end_card(scene: &mut Scene) {
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
