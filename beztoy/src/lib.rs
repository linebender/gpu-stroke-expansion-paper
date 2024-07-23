// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! An interactive toy for experimenting with rendering of Bézier paths,
//! including Euler spiral based stroke expansion.

use wasm_bindgen::prelude::*;
use xilem_web::{
    elements::svg::{g, svg},
    get_element_by_id,
    interfaces::*,
    svg::{
        kurbo::{BezPath, Cap, Circle, CubicBez, Line, PathEl, Point, Shape, Stroke},
        peniko::Color,
    },
    App, PointerMsg, DomView,
};

use flatten::{
    ArcSegment,
    euler::{CubicParams, CubicToEulerIter},
    flatten::flatten_offset,
    stroke::{stroke_undashed, LoweredPath},
};

#[derive(Default)]
struct AppState {
    p0: Point,
    p1: Point,
    p2: Point,
    p3: Point,
    grab: GrabState,
}

#[derive(Default)]
struct GrabState {
    is_down: bool,
    id: i32,
    dx: f64,
    dy: f64,
}

impl GrabState {
    fn handle(&mut self, pt: &mut Point, p: &PointerMsg) {
        match p {
            PointerMsg::Down(e) => {
                if e.button == 0 {
                    self.dx = pt.x - e.x;
                    self.dy = pt.y - e.y;
                    self.id = e.id;
                    self.is_down = true;
                }
            }
            PointerMsg::Move(e) => {
                if self.is_down && self.id == e.id {
                    pt.x = (self.dx + e.x).min(792.).max(8.);
                    pt.y = (self.dy + e.y).min(592.).max(8.);
                }
            }
            PointerMsg::Up(e) => {
                if self.id == e.id {
                    self.is_down = false;
                }
            }
        }
    }
}

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

fn app_logic(state: &mut AppState) -> impl DomView<AppState> {
    let mut path = BezPath::new();
    path.move_to(state.p0);
    path.curve_to(state.p1, state.p2, state.p3);
    let stroke = xilem_web::svg::kurbo::Stroke::new(2.0);
    let stroke_thick = xilem_web::svg::kurbo::Stroke::new(12.0);
    let stroke_thin = xilem_web::svg::kurbo::Stroke::new(2.0);
    const NONE: Color = Color::TRANSPARENT;
    const HANDLE_RADIUS: f64 = 8.0;
    let c = CubicBez::new(state.p0, state.p1, state.p2, state.p3);
    let params = CubicParams::from_cubic(c);
    let err = params.est_euler_err();
    let mut spirals = vec![];
    const TOL: f64 = 1.0;
    for (i, es) in CubicToEulerIter::new(c, TOL).enumerate() {
        let path = if es.params.cubic_ok() {
            es.to_cubic().into_path(1.0)
        } else {
            // Janky rendering, we should be more sophisticated
            // and subdivide into cubics with appropriate bounds
            let mut path = BezPath::new();
            const N: usize = 20;
            path.move_to(es.p0);
            for i in 1..N {
                let t = i as f64 / N as f64;
                path.line_to(es.eval(t));
            }
            path.line_to(es.p1);
            path
        };
        let color = RAINBOW_PALETTE[(i * 7) % 12];
        spirals.push(path.stroke(color, stroke_thick.clone()));
    }
    let offset = 100.0;
    let style = Stroke::new(offset).with_caps(Cap::Butt);
    //let stroked: LoweredPath<ArcSegment > = stroke_undashed(c.to_path(1.), &style, 1.);
    let stroked: LoweredPath<Line> = stroke_undashed(c.to_path(1.), &style, 1.);
    let flat = stroked.to_bez();
    let mut flat_pts = vec![];
    for seg in flat.elements().iter() {
        match seg {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                let circle = Circle::new(*p, 2.0).fill(Color::BLACK);
                flat_pts.push(circle);
            }
            _ => (),
        }
    }
    svg(g((
        g(spirals).fill(NONE),
        g((
            path.stroke(Color::BLACK, stroke_thin.clone()),
            flat.stroke(Color::DARK_GREEN, stroke_thin.clone()),
        )).fill(NONE),
        g(flat_pts),
        Line::new(state.p0, state.p1).stroke(Color::BLUE, stroke.clone()),
        Line::new(state.p2, state.p3).stroke(Color::BLUE, stroke.clone()),
        Line::new((790., 300.), (790., 300. - 1000. * err)).stroke(Color::RED, stroke.clone()),
        g((
            Circle::new(state.p0, HANDLE_RADIUS)
                .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p0, &msg)),
            Circle::new(state.p1, HANDLE_RADIUS)
                .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p1, &msg)),
            Circle::new(state.p2, HANDLE_RADIUS)
                .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p2, &msg)),
            Circle::new(state.p3, HANDLE_RADIUS)
                .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p3, &msg)),
        )),
    )))
    .attr("width", 800)
    .attr("height", 600)
}

#[wasm_bindgen]
pub fn run_beztoy() {
    console_error_panic_hook::set_once();
    let mut state = AppState::default();
    state.p0 = Point::new(55.0, 466.0);
    state.p1 = Point::new(350.0, 146.0);
    state.p2 = Point::new(496.0, 537.0);
    state.p3 = Point::new(739.0, 244.0);
    App::new(get_element_by_id("beztoy-container"), state, app_logic).run();
}
