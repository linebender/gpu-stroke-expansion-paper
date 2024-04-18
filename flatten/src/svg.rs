// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! A very basic utility to extract SVG paths.

use std::str::FromStr;
use std::time::Instant;
use std::{error::Error, io::Write};

use kurbo::{BezPath, Cap, Join, Line, Stroke};
use roxmltree::{Document, Node};

use crate::arc_segment::ArcSegment;
use crate::stroke::{stroke_undashed, LoweredPath, Lowering};

pub struct StyledPath {
    pub path: BezPath,
    pub style: Stroke,
    pub scale: f64,
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
            parse_rec(node, &mut scene, 1.0)?;
        }
        Ok(scene)
    }
}

#[derive(Debug, PartialEq)]
enum Token<'a> {
    Ident(&'a str),
    Number(f64),
    Symbol(char),
}

struct Lexer<'a>(&'a str);

fn scan_num(s: &str) -> usize {
    let mut i = 0;
    let b = s.as_bytes()[i];
    if b == b'-' {
        i += 1;
    }
    while i < s.len() {
        let b = s.as_bytes()[i];
        // a bit sloppy, we allow multiple decimal points
        if (b'0'..=b'9').contains(&b) || b == b'.' {
            i += 1;
        } else {
            break;
        }
    }
    if i < s.len() {
        let b = s.as_bytes()[i];
        if b == b'e' || b == b'E' {
            i += 1;
            if i < s.len() {
                let b = s.as_bytes()[i];
                if b == b'-' || b == b'+' {
                    i += 1;
                }
            }
            while i < s.len() {
                let b = s.as_bytes()[i];
                if !(b'0'..=b'9').contains(&b) {
                    break;
                }
                i += 1;
            }
        }
    }
    i
}

fn scan_ident(s: &str) -> usize {
    let mut i = 1;
    while i < s.len() {
        let b = s.as_bytes()[i];
        if b == b'_'
            || (b'0'..=b'9').contains(&b)
            || (b'a'..=b'z').contains(&b)
            || (b'A'..=b'Z').contains(&b)
        {
            i += 1;
        } else {
            break;
        }
    }
    i
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();
        if let Some(c) = self.peek() {
            if ('0'..='9').contains(&c) || c == '-' || c == '+' || c == '.' {
                let len = scan_num(self.0);
                let number = self.0[0..len].parse().unwrap();
                self.0 = &self.0[len..];
                return Some(Token::Number(number));
            }
            if ('a'..='z').contains(&c) || ('A'..='z').contains(&c) {
                let len = scan_ident(self.0);
                let ident = &self.0[0..len];
                self.0 = &self.0[len..];
                return Some(Token::Ident(ident));
            }
            self.0 = &self.0[c.len_utf8()..];
            Some(Token::Symbol(c))
        } else {
            None
        }
    }
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.0.chars().next()
    }

    fn skip_whitespace(&mut self) {
        while self.peek() == Some(' ') {
            self.0 = &self.0[1..];
        }
    }
}

// This is very fragile and is designed only to work on the test samples.
fn get_transform_scale(transform: &str) -> f64 {
    let mut scale = 1.0;
    let mut lexer = Lexer(transform);
    while let Some(func) = lexer.next() {
        match func {
            Token::Ident("scale") => {
                if lexer.next() == Some(Token::Symbol('(')) {
                    if let Some(Token::Number(n)) = lexer.next() {
                        scale *= n;
                        if lexer.next() != Some(Token::Symbol(')')) {
                            break;
                        }
                    }
                }
            }
            Token::Ident("matrix") => {
                if lexer.next() == Some(Token::Symbol('(')) {
                    if let Some(Token::Number(n)) = lexer.next() {
                        scale *= n;
                        while let Some(tok) = lexer.next() {
                            if tok == Token::Symbol(')') {
                                break;
                            }
                        }
                    }
                }
            }
            // Sloppy but hopefully good enough
            _ => (),
        }
    }
    scale
}

fn parse_rec(node: Node, scene: &mut SvgScene, scale: f64) -> Result<(), Box<dyn Error>> {
    match node.tag_name().name() {
        "g" => {
            let mut child_scale = scale;
            if let Some(transform) = node.attribute("transform") {
                child_scale *= get_transform_scale(transform);
            }
            for child in node.children() {
                parse_rec(child, scene, child_scale)?;
            }
        }
        "path" => {
            let d = node.attribute("d").ok_or("missing 'd'")?;
            let path = BezPath::from_svg(d)?;
            let width = node
                .attribute("stroke-width")
                .map(|a| f64::from_str(a).unwrap_or(1.0))
                .unwrap_or(4.0);
            let mut cap = Cap::Butt;
            if let Some(linecap) = node.attribute("stroke-linecap") {
                match linecap {
                    "round" => cap = Cap::Round,
                    "square" => cap = Cap::Square,
                    _ => (),
                }
            }
            let mut join = Join::Miter;
            if let Some(linejoin) = node.attribute("stroke-linejoin") {
                match linejoin {
                    "round" => join = Join::Round,
                    "bevel" => join = Join::Bevel,
                    _ => (),
                }
            }
            // TODO: miter limit
            let style = Stroke::new(width).with_caps(cap).with_join(join);
            scene.paths.push(StyledPath { path, style, scale });
        }
        _ => (),
    }
    Ok(())
}

impl SvgScene {
    pub fn expand<L: Lowering>(&self, tolerance: f64) -> Vec<LoweredPath<L>> {
        self.paths
            .iter()
            .map(|path| stroke_undashed(&path.path, &path.style, tolerance / path.scale))
            .collect()
    }

    fn to_svg(
        &self,
        out: &mut impl Write,
        paths: &[LoweredPath<impl Lowering>],
    ) -> Result<(), Box<dyn Error>> {
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
    for _ in 0..1 {
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
        let (p, elapsed) = crate::skia::stroke_expand(&path.path, &path.style, tolerance);
        println!("{} {elapsed:?}", p.to_svg());
        println!("{} path segments", p.segments().count());
    }
}
