// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! A very basic utility to extract SVG paths.

use std::time::Instant;
use std::{error::Error, io::Write};
use std::str::FromStr;

use kurbo::{BezPath, Line, Stroke};
use roxmltree::{Document, Node};

use crate::arc_segment::ArcSegment;
use crate::stroke::{stroke_undashed, LoweredPath, Lowering};

pub struct StyledPath {
    pub path: BezPath,
    pub style: Stroke,
    // can add color etc, but it's irrelevant for stroke expansion
}

pub struct SvgScene {
    pub paths: Vec<StyledPath>,
    // maybe viewport and extra stuff
}

impl SvgScene {
    pub fn load(xml_string: &str) -> Result<SvgScene, Box<dyn Error>> {
        let doc = Document::parse(xml_string)?;
        let root = doc.root_element();
        let mut scene = SvgScene { paths: vec![] };
        for node in root.children() {
            parse_rec(node, &mut scene)?;
        }
        Ok(scene)
    }
}

fn parse_rec(node: Node, scene: &mut SvgScene) -> Result<(), Box<dyn Error>> {
    match node.tag_name().name() {
        "g" => {
            for child in node.children() {
                parse_rec(child, scene)?;
            }
        }
        "path" => {
            let d = node.attribute("d").ok_or("missing 'd'")?;
            let path = BezPath::from_svg(d)?;
            let width = node
                .attribute("stroke-width")
                .map(|a| f64::from_str(a).unwrap_or(1.0))
                .unwrap_or(4.0);
            let style = Stroke::new(width);
            // TODO: cap and join styles
            scene.paths.push(StyledPath { path, style });
        }
        _ => (),
    }
    Ok(())
}

impl SvgScene {
    fn expand<L: Lowering>(&self, tolerance: f64) -> Vec<LoweredPath<L>> {
        self.paths
            .iter()
            .map(|path| stroke_undashed(&path.path, &path.style, tolerance))
            .collect()
    }

    fn to_svg(&self, out: &mut impl Write, paths: &[LoweredPath<impl Lowering>]) -> Result<(), Box<dyn Error>> {
        // these should probably be fields in the scene
        let width = 1024;
        let height = 1024;
        writeln!(out, "<svg width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" xmlns=\"http://www.w3.org/2000/svg\">")?;
        for path in paths {
            let svg_path = path.to_svg();
            writeln!(out, "  <path d='{svg_path}' />")?;
        }
        writeln!(out, "</svg>")?;
        Ok(())
    }
}

use clap::Parser;

#[derive(Parser)]
pub struct SvgArgs {
    filename: String,
    #[arg(short, long)]
    tolerance: Option<f64>,
    #[arg(short, long)]
    primitive: Option<String>,
    #[arg(short, long)]
    output_file: Option<String>,
}

#[derive(Clone, Parser)]

enum PrimType {
    Line,
    Arc,
}

impl Default for PrimType {
    fn default() -> Self {
        PrimType::Arc
    }
}

pub fn svg_main(args: SvgArgs) {
    let xml_str = std::fs::read_to_string(&args.filename).unwrap();
    let scene = SvgScene::load(&xml_str).unwrap();
    let tolerance = args.tolerance.unwrap_or(1.0);
    let prim_type = match args.primitive.as_deref() {
        Some("l") => PrimType::Line,
        Some("a") => PrimType::Arc,
        _ => PrimType::Arc,
    };
    for _ in 0..10 {
        let start = Instant::now();
        match prim_type {
            PrimType::Line => {
                let paths: Vec<LoweredPath<Line>> = scene.expand(tolerance);
                let total_segs: usize = paths.iter().map(|p| p.path.len()).sum();
                println!("{total_segs} lines, {:?}", start.elapsed());
            }
            PrimType::Arc => {
                let paths: Vec<LoweredPath<ArcSegment>> = scene.expand(tolerance);
                let total_segs: usize = paths.iter().map(|p| p.path.len()).sum();
                println!("{total_segs} arcs, {:?}", start.elapsed());
            }
        }
    }
    if let Some(out_file_name) = args.output_file {
        let mut f = std::fs::File::create(out_file_name).unwrap();
        match prim_type {
            PrimType::Line => {
                let paths: Vec<LoweredPath<Line>> = scene.expand(tolerance);
                scene.to_svg(&mut f, &paths).unwrap();
            }
            PrimType::Arc => {
                let paths: Vec<LoweredPath<ArcSegment>> = scene.expand(tolerance);
                scene.to_svg(&mut f, &paths).unwrap();
            }
        }
    }
    #[cfg(feature = "skia-safe")]
    for path in &scene.paths {
        let (p, elapsed) = crate::skia::stroke_expand(&path.path, &path.style);
        println!("{} {elapsed:?}", p.to_svg());
        println!("{} path segments", p.segments().count());
    }
}
