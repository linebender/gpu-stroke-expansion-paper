// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use flatten::{
    euler::{self, EulerParams, EulerSeg},
    stroke::{LoweredPath, Lowering, StrokeOpts},
    ArcSegment,
};
use parley::{FontContext, Layout, LayoutContext};
use vello::{
    kurbo::{
        fit_to_bezpath_opt, Affine, BezPath, Cap, Circle, CubicBez, Line, ParamCurve,
        ParamCurveArclen, ParamCurveFit, Point, Rect, Shape, Stroke,
    },
    peniko::{Brush, Color},
    Scene,
};

use crate::{log_aesthetic, text};

pub struct Anims {
    dens_curve: BezPath,
    g_path: BezPath,
    #[allow(unused)]
    title: Scene,
    pipeline: Scene,
    espc_density: Scene,
    #[allow(unused)]
    font_context: FontContext,
    #[allow(unused)]
    lcx: LayoutContext<Brush>,
    title_layout: Layout<Brush>,
    strong_layout: Layout<Brush>,
    weak_layout: Layout<Brush>,
    raph_layout: Layout<Brush>,
    arman_layout: Layout<Brush>,
    arclen_layout: Layout<Brush>,
    subdiv_layout: Layout<Brush>,
    mmark: crate::mmark::MMark,
    spiral: BezPath,
}

fn timed(t: &mut f64, duration: f64) -> bool {
    if *t < duration {
        true
    } else {
        *t -= duration;
        false
    }
}

const STROKE_LEN: f64 = 4.5;

const G_PATH_STR: &str = "M470 295h-83c14 32 19 55 19 84c0 50 -15 85 -48 113c-31 27 -71 42 -108 42c-2 0 -21 -2 -57 -5c-27 8 -60 43 -60 63c0 16 24 25 78 27l129 6c74 3 121 45 121 107c0 38 -17 70 -55 101c-52 42 -130 68 -205 68c-96 0 -173 -44 -173 -97c0 -37 26 -70 98 -122
c-42 -20 -53 -31 -53 -53c0 -17 6 -28 27 -50c3 -4 15 -15 36 -35l26 -24c-67 -33 -93 -71 -93 -134c0 -92 74 -163 167 -163c26 0 53 5 80 15l22 8c20 7 34 10 55 10h77v39zM147 685c-40 48 -49 63 -49 86c0 44 57 73 146 73c113 0 189 -39 189 -97c0 -36 -33 -49 -124 -49
c-49 0 -128 -6 -162 -13zM152 345v3c0 96 41 161 103 161c46 0 74 -35 74 -91c0 -40 -11 -85 -30 -120c-16 -30 -42 -47 -73 -47c-46 0 -74 35 -74 94z";

const SUBDIV_Y: f64 = 1400.;
const SUBDIV_X: f64 = 1000.;

// https://iamkate.com/data/12-bit-rainbow/
const RAINBOW_PALETTE: [Color; 12] = [
    Color::rgb8(0x88, 0x11, 0x66),
    Color::rgb8(0xaa, 0x33, 0x55),
    Color::rgb8(0xcc, 0x66, 0x66),
    Color::rgb8(0xee, 0x99, 0x44),
    Color::rgb8(0xee, 0xdd, 0x00),
    Color::rgb8(0x99, 0xdd, 0x55),
    Color::rgb8(0x44, 0xdd, 0x88),
    Color::rgb8(0x22, 0xcc, 0xbb),
    Color::rgb8(0x00, 0xbb, 0xcc),
    Color::rgb8(0x00, 0x99, 0xcc),
    Color::rgb8(0x33, 0x66, 0xbb),
    Color::rgb8(0x66, 0x33, 0x99),
];

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
        let mut pipeline = Scene::new();
        if let Err(e) = vello_svg::append(&mut pipeline, include_str!("../pipeline-clean.svg")) {
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

        let raph_layout = label(&mut font_context, &mut lcx, "Raph Levien", 100.0);
        let arman_layout = label(&mut font_context, &mut lcx, "Arman Uguray", 100.0);
        let strong_layout = label(&mut font_context, &mut lcx, "strongly correct", 50.0);
        let weak_layout = label(&mut font_context, &mut lcx, "weakly correct", 50.0);
        let arclen_layout = label(&mut font_context, &mut lcx, "arc length", 40.0);
        let subdiv_layout = label(&mut font_context, &mut lcx, "subdivision density", 40.0);
        let mmark = crate::mmark::MMark::new(100);
        let spiral = mk_spiral();
        Anims {
            dens_curve,
            g_path,
            espc_density,
            title,
            pipeline,
            font_context,
            lcx,
            title_layout,
            raph_layout,
            arman_layout,
            strong_layout,
            weak_layout,
            arclen_layout,
            subdiv_layout,
            mmark,
            spiral,
        }
    }

    pub fn render(&mut self, scene: &mut Scene, mut t: f64) {
        const LEAD_IN: f64 = 5.0;
        // if timed(&mut t, 10.0) {
        //    self.end_card(scene);
        //    return
        // }
        if timed(&mut t, LEAD_IN + 7.0) {
            self.text_card(scene, (t - LEAD_IN).max(0.0));
        } else if timed(&mut t, 6.0) {
            self.cubic_to_euler(scene, t);
        } else if timed(&mut t, 11.0) {
            let show_dens = t > 5.0;
            self.euler_spiral(scene, t, show_dens);
            if show_dens {
                self.show_density(scene);
            }
        } else if timed(&mut t, 4.5) {
            self.show_g(scene, t);
        } else if timed(&mut t, STROKE_LEN) {
            self.anim_stroke(scene, t / STROKE_LEN);
        } else {
            self.show_pipeline(scene);
            const PIPELINE_T: f64 = 4.0;
            if t >= PIPELINE_T {
                let n = (100 + ((t - PIPELINE_T) * 20000.) as usize).min(70_000);
                self.mmark.render(scene, n);
            }
        }
    }

    #[allow(unused)]
    fn show_title(&self, scene: &mut Scene) {
        scene.append(
            &self.title,
            Some(Affine::translate((-430.0, -350.0)) * Affine::scale(10.0)),
        );
    }

    fn show_pipeline(&self, scene: &mut Scene) {
        scene.append(
            &self.pipeline,
            Some(Affine::translate((150., 0.)) * Affine::scale(1.65)),
        );
    }

    fn cubic_to_euler(&self, scene: &mut Scene, t: f64) {
        let c = CubicBez::new((100., 400.), (200., 200.), (400., 300.), (500., 400.));
        let a = Affine::scale(3.0);
        let stroke = Stroke::new(2.0);
        let stroke_thick = Stroke::new(5.0);
        let thin_stroke = Stroke::new(1.0);
        let stroke_color = Color::rgb(0., 0., 0.5);
        let thin_stroke_color = Color::rgb(0., 0., 1.0);
        let ctrl_pt_color = Color::rgb(0.5, 0., 0.5);
        let t2 = easing(t * 0.5);
        const ES_DIST: f64 = 40.;
        let es_a = Affine::translate((0., -ES_DIST * t2)) * a;
        for (i, es) in flatten::euler::CubicToEulerIter::new(c, 1.0).enumerate() {
            let item_a = Affine::translate(((i as f64 - 1.5) * ES_DIST * t2, 0.)) * es_a;
            let color = &RAINBOW_PALETTE[(i * 7) % RAINBOW_PALETTE.len()]
                .with_alpha_factor(t.min(1.0) as f32);
            scene.stroke(&stroke_thick, item_a, color, None, &es.to_cubic());
        }
        scene.stroke(&stroke, a, stroke_color, None, &c);
        scene.stroke(
            &thin_stroke,
            a,
            thin_stroke_color,
            None,
            &Line::new(c.p0, c.p1),
        );
        scene.stroke(
            &thin_stroke,
            a,
            thin_stroke_color,
            None,
            &Line::new(c.p2, c.p3),
        );
        let circ_p1 = Circle::new(c.p1, 3.0);
        let circ_p2 = Circle::new(c.p2, 3.0);
        scene.fill(
            vello::peniko::Fill::NonZero,
            a,
            &ctrl_pt_color,
            None,
            &circ_p1,
        );
        scene.fill(
            vello::peniko::Fill::NonZero,
            a,
            &ctrl_pt_color,
            None,
            &circ_p2,
        );
    }

    fn euler_spiral(&self, scene: &mut Scene, t: f64, show_dens: bool) {
        let stroke = Stroke::new(2.0);
        let stroke_color = Color::rgb(0., 0., 0.5);
        let t2 = easing(t * 0.5);
        scene.stroke(&stroke, Affine::IDENTITY, stroke_color, None, &self.spiral);
        const MAX_OFFSET: f64 = 400.;
        draw_espc(scene, t2 * -MAX_OFFSET, 0.25, false);
        if !show_dens {
            draw_espc(scene, t2 * MAX_OFFSET, 0.25, false);
        }
        if t > 2.0 {
            let subdiv_t = easing(0.5 * (t - 2.0));
            let tol = 10.0 * (-5.0 * subdiv_t).exp();
            draw_espc(scene, t2 * -MAX_OFFSET, tol, true);
        }
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
        let c = Affine::scale(scale) * Affine::translate((-280.0, 0.0)) * c;
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
        text::render_text(scene, Affine::translate((200., 300.)), &self.weak_layout);
        text::render_text(
            scene,
            Affine::translate((200. + W, 300.)),
            &self.strong_layout,
        );
    }

    fn show_density(&self, scene: &mut Scene) {
        let axis_stroke = Stroke::new(2.0);
        let axis_color = Color::rgb(0.1, 0.1, 0.1);
        let x_axis = Line::new((SUBDIV_X - 400., SUBDIV_Y), (SUBDIV_X + 600., SUBDIV_Y));
        scene.stroke(&axis_stroke, Affine::IDENTITY, &axis_color, None, &x_axis);
        let y_axis = Line::new((SUBDIV_X, SUBDIV_Y - 500.), (SUBDIV_X, SUBDIV_Y + 100.));
        scene.stroke(&axis_stroke, Affine::IDENTITY, &axis_color, None, &y_axis);
        let stroke = Stroke::new(4.0);
        let stroke_color = Color::rgb(0.1, 0.1, 0.5);
        scene.stroke(
            &stroke,
            Affine::IDENTITY,
            &stroke_color,
            None,
            &self.dens_curve,
        );
        scene.append(
            &self.espc_density,
            Some(Affine::translate((SUBDIV_X + 200., 1050.)) * Affine::scale(4.0)),
        );
        text::render_text(
            scene,
            Affine::translate((SUBDIV_X + 400., SUBDIV_Y + 10.)),
            &self.arclen_layout,
        );
        text::render_text(
            scene,
            Affine::translate((SUBDIV_X + 15., SUBDIV_Y - 500.)),
            &self.subdiv_layout,
        );
    }

    fn show_g(&self, scene: &mut Scene, t: f64) {
        let tol = 10.0 * (-1.0 * t.min(4.0)).exp();
        let stroke = Stroke::new(2.0);
        let stroke_color = Color::rgb(0.2, 0.7, 0.2);
        let flatten_style = Stroke::new(20.0).with_caps(Cap::Butt);
        let stroked: LoweredPath<Line> =
            flatten::stroke::stroke_undashed(&self.g_path, &flatten_style, tol);
        let stroked_path = stroked.to_bez();
        let line_affine = Affine::translate((150.0, 0.)) * Affine::scale(1.5);
        scene.stroke(&stroke, line_affine, &stroke_color, None, &stroked_path);
        draw_subdivisions(scene, stroked, line_affine);

        let stroked_arcs: LoweredPath<ArcSegment> =
            flatten::stroke::stroke_undashed(&self.g_path, &flatten_style, tol);
        let stroked_path = stroked_arcs.to_bez();
        let arc_affine = Affine::translate((1000.0, 0.0)) * line_affine;
        scene.stroke(&stroke, arc_affine, &stroke_color, None, &stroked_path);
        draw_subdivisions(scene, stroked_arcs, arc_affine);
    }

    #[allow(unused)]
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
        const SPEED: f64 = 40.;
        const WIDTH: f64 = 10.;
        let max_w = t * SPEED;
        let n = 1 + (max_w / WIDTH).floor() as usize;
        for i in 0..n {
            let w = max_w - WIDTH * i as f64;
            let stroke = Stroke::new(w)
                .with_join(vello::kurbo::Join::Round)
                .with_miter_limit(2.0);
            let eo = (i % 2) as f64;
            let s = (0.6 + 0.4 * eo) * (1.0 - 0.5 * ((n - i) as f64 * -0.2).exp());
            let brush = Color::rgb(1. * s, 0.8 * s, 0.3 * s);
            text::render_text_stroked(scene, a, &self.title_layout, &stroke, &brush);
        }
        text::render_text(scene, a, &self.title_layout);
        text::render_text(scene, Affine::translate((200., 1180.)), &self.raph_layout);
        text::render_text(scene, Affine::translate((200., 1300.)), &self.arman_layout);
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
    const SCALE: f64 = 300.0;
    let a = Affine::translate((SUBDIV_X, SUBDIV_Y)) * Affine::scale_non_uniform(SCALE, -SCALE);
    for i in 0..=N {
        let x = 3.0 * (i as f64 / N as f64 - 0.38);
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

const K1: f64 = 10.0;

const SPIRAL_Y: f64 = 600.;

fn mk_spiral() -> BezPath {
    let params = log_aesthetic::LogAestheticParams::new(1., -0.5 * K1, K1);
    let p0 = Point::new(500., SPIRAL_Y);
    let p1 = Point::new(1500., SPIRAL_Y);
    let lac = log_aesthetic::LogAestheticCurve::from_points_params(params, p0, p1);
    let th = lac.sample_pt_deriv(0.0).1.atan2();
    //println!("{:?} {th}", lac.sample_pt_deriv(0.0));
    //println!("{:?}", lac.sample_pt_deriv(1.0));
    fit_to_bezpath_opt(&lac, 0.1)
}

// Originally the idea was to display a more complete Euler spiral over one
// or more of the Euler spiral segments, but I gave up on the idea, partly
// because I wasn't sure it would be clear visually, partly because the
// math to line them up wasn't trivial.
fn transform_for_es_params(euler_params: EulerParams) -> Affine {
    // TODO: retain the curve
    let params = log_aesthetic::LogAestheticParams::new(1., -0.5 * K1, K1);
    let p0 = Point::new(500., 1000.);
    let p1 = Point::new(1500., 1000.);
    let lac = log_aesthetic::LogAestheticCurve::from_points_params(params, p0, p1);
    let s_center = 0.5 + euler_params.k0 / euler_params.k1;
    todo!()
}

fn draw_espc(scene: &mut Scene, offset: f64, tol: f64, show_subdiv: bool) {
    // angle for s-shaped curve with K1 = 10.0
    const TH: f64 = 0.839061107799705;
    let es_params = EulerParams::from_angles(TH, -TH);
    let p0 = Point::new(500., SPIRAL_Y);
    let p1 = Point::new(1500., SPIRAL_Y);
    let es = EulerSeg::from_params(p0, p1, es_params);
    let mut path = BezPath::new();
    let stroke = Stroke::new(2.0);
    let line_color = Color::rgb(0.75, 0.75, 0.75);
    let pt_color = Color::rgb(0.1, 0.1, 0.5);
    flatten::flatten::flatten_offset2(&es, 0.0..1.0, offset, tol, |p, offset_p| {
        if show_subdiv {
            let line = Line::new(p, offset_p);
            scene.stroke(&stroke, Affine::IDENTITY, line_color, None, &line);
            let circ = Circle::new(p, 4.0);
            scene.fill(
                vello::peniko::Fill::NonZero,
                Affine::IDENTITY,
                &pt_color,
                None,
                &circ,
            );
        }
        if path.elements().is_empty() {
            path.move_to(offset_p);
        } else {
            path.line_to(offset_p);
        }
    });
    let stroke_color = if show_subdiv {
        Color::rgb(0.1, 0.7, 0.1)
    } else {
        Color::rgb(0., 0., 0.5)
    };
    scene.stroke(&stroke, Affine::IDENTITY, stroke_color, None, &path);
}

fn easing(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else {
        (3. - 2. * t) * t * t
    }
}
