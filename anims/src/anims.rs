use flatten::{stroke::LoweredPath, ArcSegment};
use vello::{
    kurbo::{Affine, BezPath, Cap, CubicBez, Line, ParamCurve, Point, Rect, Shape, Stroke},
    peniko::Color,
    Scene,
};

pub struct Anims {
    dens_curve: BezPath,
    g_path: BezPath,
}

fn timed(t: &mut f64, duration: f64) -> bool {
    if *t < duration {
        true
    } else {
        *t -= duration;
        false
    }
}

const STROKE_LEN: f64 = 5.0;

const G_PATH_STR: &str = "M470 295h-83c14 32 19 55 19 84c0 50 -15 85 -48 113c-31 27 -71 42 -108 42c-2 0 -21 -2 -57 -5c-27 8 -60 43 -60 63c0 16 24 25 78 27l129 6c74 3 121 45 121 107c0 38 -17 70 -55 101c-52 42 -130 68 -205 68c-96 0 -173 -44 -173 -97c0 -37 26 -70 98 -122
c-42 -20 -53 -31 -53 -53c0 -17 6 -28 27 -50c3 -4 15 -15 36 -35l26 -24c-67 -33 -93 -71 -93 -134c0 -92 74 -163 167 -163c26 0 53 5 80 15l22 8c20 7 34 10 55 10h77v39zM147 685c-40 48 -49 63 -49 86c0 44 57 73 146 73c113 0 189 -39 189 -97c0 -36 -33 -49 -124 -49
c-49 0 -128 -6 -162 -13zM152 345v3c0 96 41 161 103 161c46 0 74 -35 74 -91c0 -40 -11 -85 -30 -120c-16 -30 -42 -47 -73 -47c-46 0 -74 35 -74 94z";

impl Anims {
    pub fn new() -> Self {
        let dens_curve = density_curve();
        let g_path = BezPath::from_svg(G_PATH_STR).unwrap();
        Anims { dens_curve, g_path }
    }

    pub fn render(&self, scene: &mut Scene, mut t: f64) {
        if timed(&mut t, STROKE_LEN) {
            anim_stroke(scene, t / STROKE_LEN);
        } else if timed(&mut t, 5.0) {
            self.show_g(scene, t);
        } else if timed(&mut t, 2.0) {
            self.show_density(scene);
        } else {
            end_card(scene);
        }
    }

    fn show_density(&self, scene: &mut Scene) {
        let stroke = Stroke::new(4.0);
        let stroke_color = Color::rgb(0.9804, 0.702, 0.5294);
        scene.stroke(&stroke, Affine::IDENTITY, &stroke_color, None, &self.dens_curve);
    }

    fn show_g(&self, scene: &mut Scene, t: f64) {
        let tol = 10.0 * (-1.0 * t).exp();
        let stroke = Stroke::new(4.0);
        let stroke_color = Color::rgb(0.9804, 0.702, 0.5294);
        let flatten_style = Stroke::new(20.0).with_caps(Cap::Butt);
        let stroked: LoweredPath<Line> = flatten::stroke::stroke_undashed(&self.g_path, &flatten_style, tol);
        let stroked_path = stroked.to_bez();
        scene.stroke(&stroke, Affine::IDENTITY, &stroke_color, None, &stroked_path);

        let stroked_arcs: LoweredPath<ArcSegment> = flatten::stroke::stroke_undashed(&self.g_path, &flatten_style, tol);
        let stroked_path = stroked_arcs.to_bez();
        scene.stroke(&stroke, Affine::translate((500.0, 0.0)), &stroke_color, None, &stroked_path);
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
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        fill_color,
        None,
        &stroked_path,
    );
    scene.stroke(&stroke, Affine::IDENTITY, stroke_color, None, &stroked_path);
    scene.stroke(
        &thin_stroke,
        Affine::IDENTITY,
        thin_stroke_color,
        None,
        &trimmed,
    );
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

fn density_curve() -> BezPath {
    const N: usize = 300;
    let mut path = BezPath::new();
    let a = Affine::scale_non_uniform(300.0, -300.0) * Affine::translate((2.0, -2.0));
    for i in 0..=N {
        let x = 3.0 * (i as f64 / N as f64 - 0.5);
        let y = (1.0 - x * x).abs().sqrt();
        let p = a * Point::new(x, y);
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path
}
