// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! Conversion of SVG to Nehab's RVG format.

// This command may be useful:
// for i in paths/*.svg; do cargo run to-rvg $i > rvgs/`basename $i .svg`.rvg; done

use std::fmt::Write;

use clap::Parser;
use kurbo::PathEl;

use crate::svg::SvgScene;

#[derive(Parser)]
pub struct ToRvgArgs {
    filename: String,
}

fn to_rvg_path(path: &[PathEl]) -> String {
    let mut s = String::new();
    for el in path {
        match el {
            PathEl::MoveTo(p) => _ = write!(&mut s, "M,{},{},", p.x, p.y),
            PathEl::LineTo(p) => _ = write!(&mut s, "L,{},{},", p.x, p.y),
            PathEl::QuadTo(p1, p2) => _ = write!(&mut s, "Q,{},{},{},{},", p1.x, p1.y, p2.x, p2.y),
            PathEl::CurveTo(p1, p2, p3) => {
                _ = write!(
                    &mut s,
                    "C,{},{},{},{},{},{},",
                    p1.x, p1.y, p2.x, p2.y, p3.x, p3.y
                )
            }
            PathEl::ClosePath => _ = write!(&mut s, "Z,"),
        }
    }
    s
}

pub fn to_rvg_main(args: ToRvgArgs) {
    let xml_str = std::fs::read_to_string(&args.filename).unwrap();
    let scene = SvgScene::load(&xml_str).unwrap();
    // TODO: should get these out of SVG
    let width = 1024;
    let height = 1024;
    println!("local rvg = {{}}");
    println!(
        "rvg.window = window(0.0, 0.0, {}, {})",
        width as f64, height as f64
    );
    println!("rvg.viewport = viewport(0, 0, {}, {})", width, height);
    println!("rvg.scene = scene({{");
    for path in &scene.paths {
        let scale = path.scale;
        let d = to_rvg_path(path.path.elements());
        let t = if scale == 1.0 {
            String::new()
        } else {
            format!(":affine({}, 0, 0, 0, {}, 0)", scale, scale)
        };
        println!(
            "  fill(path{{{d}}}{t}:stroked({}), rgba8(0, 0, 0, 255)),",
            path.style.width * scale
        );
    }
    println!("}})");
    println!("return rvg");
}
