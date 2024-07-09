use flatten::{
    stroke::{LoweredPath, Lowering, StrokeOpts},
    ArcSegment,
};
use parley::{FontContext, Layout, LayoutContext};
use vello::{
    kurbo::{
        Affine, BezPath, Cap, Circle, CubicBez, Line, ParamCurve, ParamCurveArclen, Point, Rect,
        Shape, Stroke,
    },
    peniko::{Brush, Color},
    Scene,
};

use crate::text;

pub struct Anims {
    dens_curve: BezPath,
    g_path: BezPath,
    title: Scene,
    espc_density: Scene,
    font_context: FontContext,
    lcx: LayoutContext<Brush>,
    title_layout: Layout<Brush>,
    strong_layout: Layout<Brush>,
    weak_layout: Layout<Brush>,
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

fn label(
    font_context: &mut FontContext,
    lcx: &mut LayoutContext<Brush>,
    text: &str,
    size: f32,
) -> Layout<Brush> {
    let mut layout_builder = lcx.ranged_builder(font_context, text, 1.0);
    layout_builder.push_default(&parley::style::StyleProperty::Brush(Brush::Solid(
        Color::rgb8(0, 0, 0),
    )));
    layout_builder.push_default(&parley::style::StyleProperty::FontSize(size));
    let mut layout = layout_builder.build();
    layout.break_all_lines(Some(1800.0), parley::layout::Alignment::Start);
    layout
}

impl Anims {
    pub fn new() -> Self {
        let dens_curve = density_curve();
        let g_path = BezPath::from_svg(G_PATH_STR).unwrap();
        let mut espc_density = Scene::new();
        if let Err(e) = vello_svg::append(&mut espc_density, include_str!("../espc_density.svg")) {
            println!("error loading svg: {e:?}");
        }
        let mut title = Scene::new();
        if let Err(e) = vello_svg::append(&mut title, include_str!("../title.svg")) {
            println!("error loading svg: {e:?}");
        }
        let mut font_context = FontContext::default();
        let mut lcx = LayoutContext::new();
        let mut layout_builder =
            lcx.ranged_builder(&mut font_context, "GPU-Friendly Stroke Expansion", 1.0);
        layout_builder.push_default(&parley::style::StyleProperty::Brush(Brush::Solid(
            Color::rgb8(0, 0, 0),
        )));
        layout_builder.push_default(&parley::style::StyleProperty::FontSize(200.0));
        layout_builder.push_default(&parley::style::StyleProperty::LineHeight(1.2));
        let mut title_layout = layout_builder.build();
        title_layout.break_all_lines(Some(1800.0), parley::layout::Alignment::Start);

        let strong_layout = label(&mut font_context, &mut lcx, "strongly correct", 50.0);
        let weak_layout = label(&mut font_context, &mut lcx, "weakly correct", 50.0);
        Anims {
            dens_curve,
            g_path,
            espc_density,
            title,
            font_context,
            lcx,
            title_layout,
            strong_layout,
            weak_layout,
        }
    }

    pub fn render(&mut self, scene: &mut Scene, mut t: f64) {
        if timed(&mut t, 10.0) {
            //self.show_title(scene);
            self.text_card(scene, t);
        } else if timed(&mut t, STROKE_LEN) {
            self.anim_stroke(scene, t / STROKE_LEN);
        } else if timed(&mut t, 5.0) {
            self.show_g(scene, t);
        } else if timed(&mut t, 2.0) {
            self.show_density(scene);
        } else {
            self.end_card(scene);
        }
    }

    fn show_title(&self, scene: &mut Scene) {
        scene.append(
            &self.title,
            Some(Affine::translate((-430.0, -350.0)) * Affine::scale(10.0)),
        );
    }

    fn anim_stroke(&self, scene: &mut Scene, t: f64) {
        //let c = CubicBez::new((110., 290.), (110., 250.), (110., 160.), (140., 160.));
        let c = CubicBez::new(
            (368.4375, 162.91666666666666),
            (364.0625, 138.75),
            (445.3125, 113.75),
            (405., 144.),
        );
        let scale = 5.0;
        let c = Affine::scale(scale) * Affine::translate((-300.0, 0.0)) * c;
        const ARCLEN_EPS: f64 = 1e-6;
        let arclen = c.arclen(ARCLEN_EPS);
        let s_adjust = (t * 1.2).min(1.0);
        let t_adjust = c.inv_arclen(arclen * s_adjust, ARCLEN_EPS);
        let trimmed = c.subsegment(0.0..t_adjust);
        let flatten_style = Stroke::new(scale * 108.0).with_caps(Cap::Butt);
        let path = trimmed.to_path(1e-9);
        const W: f64 = 1000.;
        for i in 0..=1 {
            let opts = StrokeOpts { strong: i == 1 };
            let stroked: LoweredPath<Line> =
                flatten::stroke::stroke_undashed_opt(&path, &flatten_style, 0.05, opts);
            let stroked_path = stroked.to_bez();
            let stroke_color = Color::rgb(0., 0., 0.5);
            let thin_stroke_color = Color::rgb(0.5, 0.5, 0.5);
            let fill_color = Color::rgb8(0xff, 0x93, 0x8d);
            let stroke = Stroke::new(6.0);
            let thin_stroke = Stroke::new(4.0);
            let a = Affine::translate((W * i as f64, 0.));
            scene.fill(
                vello::peniko::Fill::NonZero,
                a,
                fill_color,
                None,
                &stroked_path,
            );
            scene.stroke(&thin_stroke, a, thin_stroke_color, None, &trimmed);
            scene.stroke(&stroke, a, stroke_color, None, &stroked_path);
        }
        text::render_text(scene, Affine::translate((100., 300.)), &self.weak_layout);
        text::render_text(
            scene,
            Affine::translate((100. + W, 300.)),
            &self.strong_layout,
        );
    }

    fn show_density(&self, scene: &mut Scene) {
        let stroke = Stroke::new(4.0);
        let stroke_color = Color::rgb(0.1, 0.1, 0.5);
        scene.stroke(
            &stroke,
            Affine::IDENTITY,
            &stroke_color,
            None,
            &self.dens_curve,
        );
        scene.append(&self.espc_density, Some(Affine::scale(8.0)));
    }

    fn show_g(&self, scene: &mut Scene, t: f64) {
        let tol = 10.0 * (-1.0 * t).exp();
        let stroke = Stroke::new(4.0);
        let stroke_color = Color::rgb(0.9804, 0.702, 0.5294);
        let flatten_style = Stroke::new(20.0).with_caps(Cap::Butt);
        let stroked: LoweredPath<Line> =
            flatten::stroke::stroke_undashed(&self.g_path, &flatten_style, tol);
        let stroked_path = stroked.to_bez();
        let line_affine = Affine::scale(1.5);
        scene.stroke(&stroke, line_affine, &stroke_color, None, &stroked_path);
        draw_subdivisions(scene, stroked, line_affine);

        let stroked_arcs: LoweredPath<ArcSegment> =
            flatten::stroke::stroke_undashed(&self.g_path, &flatten_style, tol);
        let stroked_path = stroked_arcs.to_bez();
        let arc_affine = Affine::translate((1000.0, 0.0)) * line_affine;
        scene.stroke(&stroke, arc_affine, &stroke_color, None, &stroked_path);
        draw_subdivisions(scene, stroked_arcs, arc_affine);
    }

    fn end_card(&self, scene: &mut Scene) {
        // placeholder for actual end card
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

    fn text_card(&mut self, scene: &mut Scene, t: f64) {
        let a = Affine::translate((200., 200.));
        for i in 0..10 {
            let w = t * (10 - i) as f64;
            let stroke = Stroke::new(w)
                .with_join(vello::kurbo::Join::Round)
                .with_miter_limit(2.0);
            let eo = (i % 2) as f64;
            let s = (0.5 + 0.5 * eo) * (1.0 - 0.5 * ((10 - i) as f64 * -0.2).exp());
            let brush = Color::rgb(0.9 * s, 0.7 * s, 0.2 * s);
            text::render_text_stroked(scene, a, &self.title_layout, &stroke, &brush);
        }
        text::render_text(scene, a, &self.title_layout);
    }
}

fn draw_subdivisions<L: Lowering>(scene: &mut Scene, path: LoweredPath<L>, a: Affine) {
    for seg in &path.path {
        let p = seg.end_point();
        let pt_color = Color::rgb(0.1, 0.1, 0.5);
        let circle = Circle::new(p, 5.0);
        scene.fill(vello::peniko::Fill::NonZero, a, pt_color, None, &circle)
    }
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
