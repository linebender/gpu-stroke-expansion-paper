// Copyright 2024 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use parley::Layout;
use vello::{
    kurbo::{Affine, Stroke},
    peniko::{Brush, BrushRef, Fill},
    Scene,
};

pub fn render_text(scene: &mut Scene, transform: Affine, layout: &Layout<Brush>) {
    for line in layout.lines() {
        for glyph_run in line.glyph_runs() {
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let run = glyph_run.run();
            let style = glyph_run.style();
            scene
                .draw_glyphs(run.font())
                .brush(&style.brush)
                .transform(transform)
                .font_size(run.font_size())
                .draw(
                    Fill::NonZero,
                    glyph_run.glyphs().map(|glyph| {
                        let gx = x + glyph.x;
                        let gy = y - glyph.y;
                        x += glyph.advance;
                        vello::glyph::Glyph {
                            id: glyph.id as _,
                            x: gx,
                            y: gy,
                        }
                    }),
                );
        }
    }
}

pub fn render_text_stroked<'b>(
    scene: &mut Scene,
    transform: Affine,
    layout: &Layout<Brush>,
    stroke: &Stroke,
    brush: impl Into<BrushRef<'b>>,
) {
    let brush_ref = brush.into();
    for line in layout.lines() {
        for glyph_run in line.glyph_runs() {
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let run = glyph_run.run();
            scene
                .draw_glyphs(run.font())
                .brush(brush_ref.clone())
                .transform(transform)
                .font_size(run.font_size())
                .draw(
                    stroke,
                    glyph_run.glyphs().map(|glyph| {
                        let gx = x + glyph.x;
                        let gy = y - glyph.y;
                        x += glyph.advance;
                        vello::glyph::Glyph {
                            id: glyph.id as _,
                            x: gx,
                            y: gy,
                        }
                    }),
                );
        }
    }
}
