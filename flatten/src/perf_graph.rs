// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0

//! A module to generate data for the primitive count graph.

use std::time::{Duration, Instant};

use clap::Parser;
use kurbo::Line;

use crate::{arc_segment::ArcSegment, stroke::LoweredPath, svg::SvgScene};

#[derive(Parser)]
pub struct PrimCountArgs {
    dir: String,
    chart_type: String,
}

#[derive(Debug)]
enum PrimType {
    Line,
    Arc,
    // Not really a primitive type, but useful for comparison
    #[cfg(feature = "skia-safe")]
    Skia,
}

#[derive(PartialEq)]
enum GraphType {
    Bar,
    Sum,
}

pub fn perf_graph(args: PrimCountArgs) {
    // TODO: set as arg?
    let graph_type = match &*args.chart_type {
        "count" => GraphType::Sum,
        "timing" => GraphType::Bar,
        _ => panic!("unknown chart type"),
    };
    let mut scenes = vec![];
    let mut names = vec![];
    for entry in std::fs::read_dir(&args.dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let xml_str = std::fs::read_to_string(&path).unwrap();
        names.push(path);
        let scene = SvgScene::load(&xml_str).unwrap();
        scenes.push(scene);
    }
    let tolerances = if args.chart_type == "timing" {
        &[0.25]
    } else {
        &[1.0, 0.5, 0.2, 0.1, 0.05, 0.02, 0.01, 0.005, 0.002, 0.001][..]
    };
    let n_iter = if args.chart_type == "timing" { 10 } else { 1 };
    if args.chart_type == "timing" {
        print!("prim");
        for name in &names {
            let stem = name
                .to_str()
                .unwrap()
                .rsplit('/')
                .nth(0)
                .unwrap()
                .strip_suffix(".svg")
                .unwrap();
            print!(" {stem}");
        }
        println!();
    }
    for prim_type in [
        PrimType::Line,
        PrimType::Arc,
        #[cfg(feature = "skia-safe")]
        PrimType::Skia,
    ] {
        match graph_type {
            GraphType::Sum => println!("\"{:?}\"", prim_type),
            _ => (),
        }
        for tolerance in tolerances {
            let mut n_segs = 0;
            let mut duration = Duration::ZERO;
            if args.chart_type == "count" {
                print!("{tolerance}");
            } else {
                print!("{prim_type:?}");
            }
            for scene in &scenes {
                for _ in 0..n_iter {
                    let start = Instant::now();
                    if graph_type == GraphType::Bar {
                        duration = Duration::ZERO;
                        n_segs = 0;
                    }
                    match prim_type {
                        PrimType::Line => {
                            let paths: Vec<LoweredPath<Line>> = scene.expand(*tolerance);
                            n_segs += paths.iter().map(|p| p.path.len()).sum::<usize>();
                            duration += start.elapsed();
                        }
                        PrimType::Arc => {
                            let paths: Vec<LoweredPath<ArcSegment>> = scene.expand(*tolerance);
                            n_segs += paths.iter().map(|p| p.path.len()).sum::<usize>();
                            duration += start.elapsed();
                        }
                        #[cfg(feature = "skia-safe")]
                        PrimType::Skia => {
                            for path in &scene.paths {
                                let (p, elapsed) = crate::skia::stroke_expand(
                                    &path.path,
                                    &path.style,
                                    tolerance / path.scale,
                                );
                                duration += elapsed;
                                n_segs += p.segments().count();
                            }
                        }
                    }
                }
                if args.chart_type == "timing" {
                    print!(" {}", duration.as_secs_f64() / n_iter as f64);
                }
            }
            if args.chart_type == "count" {
                println!(" {n_segs} {}", duration.as_secs_f64());
            } else {
                println!();
            }
        }
        if args.chart_type == "count" {
            println!();
            println!();
        }
    }
}
