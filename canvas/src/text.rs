// pathfinder/canvas/src/text.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::{CanvasRenderingContext2D, TextAlign, TextBaseline};
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::hinting::HintingOptions;
use font_kit::loaders::default::Font;
use font_kit::metrics::Metrics;
use font_kit::properties::Properties;
use font_kit::source::{Source, SystemSource};
use font_kit::sources::mem::MemSource;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use pathfinder_renderer::paint::PaintId;
use pathfinder_text::{SceneExt, TextRenderMode};
use skribo::{FontCollection, FontFamily, FontRef, Layout, TextStyle};
use std::sync::Arc;

impl CanvasRenderingContext2D {
    pub fn fill_text(&mut self, string: &str, position: Vector2F) {
        let paint_id = self.canvas.scene.push_paint(&self.current_state.fill_paint);
        self.fill_or_stroke_text(string, position, paint_id, TextRenderMode::Fill);
    }

    pub fn stroke_text(&mut self, string: &str, position: Vector2F) {
        let paint_id = self.canvas.scene.push_paint(&self.current_state.stroke_paint);
        let render_mode = TextRenderMode::Stroke(self.current_state.resolve_stroke_style());
        self.fill_or_stroke_text(string, position, paint_id, render_mode);
    }

    pub fn measure_text(&self, string: &str) -> TextMetrics {
        TextMetrics { width: self.layout_text(string).width() }
    }

    pub fn fill_layout(&mut self, layout: &Layout, transform: Transform2F) {
        let paint_id = self.canvas.scene.push_paint(&self.current_state.fill_paint);

        let clip_path = self.current_state.clip_path;
        let blend_mode = self.current_state.global_composite_operation.to_blend_mode();
        let opacity = (self.current_state.global_alpha * 255.0) as u8;

        drop(self.canvas.scene.push_layout(&layout,
                                           &TextStyle { size: self.current_state.font_size },
                                           &(transform * self.current_state.transform),
                                           TextRenderMode::Fill,
                                           HintingOptions::None,
                                           clip_path,
                                           blend_mode,
                                           opacity,
                                           paint_id));
    }

    fn fill_or_stroke_text(&mut self,
                           string: &str,
                           mut position: Vector2F,
                           paint_id: PaintId,
                           render_mode: TextRenderMode) {
        let layout = self.layout_text(string);

        let clip_path = self.current_state.clip_path;
        let blend_mode = self.current_state.global_composite_operation.to_blend_mode();
        let opacity = (self.current_state.global_alpha * 255.0) as u8;

        match self.current_state.text_align {
            TextAlign::Left => {},
            TextAlign::Right => position.set_x(position.x() - layout.width()),
            TextAlign::Center => position.set_x(position.x() - layout.width() * 0.5),
        }

        match self.current_state.text_baseline {
            TextBaseline::Alphabetic => {}
            TextBaseline::Top => position.set_y(position.y() + layout.ascent()),
            TextBaseline::Middle => position.set_y(position.y() + layout.ascent() * 0.5),
            TextBaseline::Bottom => position.set_y(position.y() + layout.descent()),
            TextBaseline::Ideographic => {
                position.set_y(position.y() + layout.ideographic_baseline())
            }
            TextBaseline::Hanging => position.set_y(position.y() + layout.hanging_baseline()),
        }

        let transform = self.current_state.transform * Transform2F::from_translation(position);

        // TODO(pcwalton): Report errors.
        drop(self.canvas.scene.push_layout(&layout,
                                           &TextStyle { size: self.current_state.font_size },
                                           &transform,
                                           render_mode,
                                           HintingOptions::None,
                                           clip_path,
                                           blend_mode,
                                           opacity,
                                           paint_id));
    }

    fn layout_text(&self, string: &str) -> Layout {
        skribo::layout(&TextStyle { size: self.current_state.font_size },
                       &self.current_state.font_collection,
                       string)
    }

    // Text styles

    #[inline]
    pub fn font(&self) -> Arc<FontCollection> {
        self.current_state.font_collection.clone()
    }

    #[inline]
    pub fn set_font<FC>(&mut self, font_collection: FC) where FC: IntoFontCollection {
        let font_collection = font_collection.into_font_collection(&self.font_context);
        self.current_state.font_collection = font_collection; 
    }

    #[inline]
    pub fn font_size(&self) -> f32 {
        self.current_state.font_size
    }

    #[inline]
    pub fn set_font_size(&mut self, new_font_size: f32) {
        self.current_state.font_size = new_font_size;
    }

    #[inline]
    pub fn text_align(&self) -> TextAlign {
        self.current_state.text_align
    }

    #[inline]
    pub fn set_text_align(&mut self, new_text_align: TextAlign) {
        self.current_state.text_align = new_text_align;
    }

    #[inline]
    pub fn text_baseline(&self) -> TextBaseline {
        self.current_state.text_baseline
    }

    #[inline]
    pub fn set_text_baseline(&mut self, new_text_baseline: TextBaseline) {
        self.current_state.text_baseline = new_text_baseline;
    }
}

// TODO(pcwalton): Support other fields.
#[derive(Clone, Copy, Debug)]
pub struct TextMetrics {
    pub width: f32,
}

#[cfg(feature = "pf-text")]
#[derive(Clone)]
pub struct CanvasFontContext {
    #[allow(dead_code)]
    pub(super) font_source: Arc<dyn Source>,
    #[allow(dead_code)]
    pub(super) default_font_collection: Arc<FontCollection>,
}

impl CanvasFontContext {
    pub fn new(font_source: Arc<dyn Source>) -> CanvasFontContext {
        let mut default_font_collection = FontCollection::new();
        if let Ok(default_font) = font_source.select_best_match(&[FamilyName::SansSerif],
                                                                &Properties::new()) {
            if let Ok(default_font) = default_font.load() {
                default_font_collection.add_family(FontFamily::new_from_font(default_font));
            }
        }

        CanvasFontContext {
            font_source,
            default_font_collection: Arc::new(default_font_collection),
        }
    }

    /// A convenience method to create a font context with the system source.
    /// This allows usage of fonts installed on the system.
    pub fn from_system_source() -> CanvasFontContext {
        CanvasFontContext::new(Arc::new(SystemSource::new()))
    }

    /// A convenience method to create a font context with a set of in-memory fonts.
    pub fn from_fonts<I>(fonts: I) -> CanvasFontContext where I: Iterator<Item = Handle> {
        CanvasFontContext::new(Arc::new(MemSource::from_fonts(fonts).unwrap()))
    }
}

// Text layout utilities

pub trait LayoutExt {
    fn width(&self) -> f32;
    fn fold_metric<G, F>(&self, get: G, fold: F) -> f32 where G: FnMut(&Metrics) -> f32,
                                                              F: FnMut(f32, f32) -> f32;
    fn ascent(&self) -> f32;
    fn descent(&self) -> f32;
    fn hanging_baseline(&self) -> f32;
    fn ideographic_baseline(&self) -> f32;
}

impl LayoutExt for Layout {
    fn width(&self) -> f32 {
        let last_glyph = match self.glyphs.last() {
            None => return 0.0,
            Some(last_glyph) => last_glyph,
        };

        let glyph_id = last_glyph.glyph_id;
        let font_metrics = last_glyph.font.font.metrics();
        let glyph_rect = last_glyph.font.font.typographic_bounds(glyph_id).unwrap();
        let scale_factor = self.size / font_metrics.units_per_em as f32;
        last_glyph.offset.x + glyph_rect.max_x() * scale_factor
    }

    fn fold_metric<G, F>(&self, mut get: G, mut fold: F) -> f32 where G: FnMut(&Metrics) -> f32,
                                                                      F: FnMut(f32, f32) -> f32 {
        let (mut last_font_seen, mut value) = (None, 0.0);
        for glyph in &self.glyphs {
            if let Some(ref last_font_seen) = last_font_seen {
                if Arc::ptr_eq(last_font_seen, &glyph.font.font) {
                    continue;
                }
            }
            let font_metrics = glyph.font.font.metrics();
            let scale_factor = self.size / font_metrics.units_per_em as f32;
            value = fold(value, get(&font_metrics) * scale_factor);
            last_font_seen = Some(glyph.font.font.clone());
        }
        value
    }

    fn ascent(&self) -> f32 {
        self.fold_metric(|metrics| metrics.ascent, f32::max)
    }

    fn descent(&self) -> f32 {
        self.fold_metric(|metrics| metrics.descent, f32::min)
    }

    fn hanging_baseline(&self) -> f32 {
        // TODO(pcwalton)
        0.0
    }

    fn ideographic_baseline(&self) -> f32 {
        // TODO(pcwalton)
        0.0
    }
}

/// Various things that can be conveniently converted into font collections for use with
/// `CanvasRenderingContext2D::set_font()`.
pub trait IntoFontCollection {
    fn into_font_collection(self, font_context: &CanvasFontContext) -> Arc<FontCollection>;
}

impl IntoFontCollection for Arc<FontCollection> {
    #[inline]
    fn into_font_collection(self, _: &CanvasFontContext) -> Arc<FontCollection> {
        self
    }
}

impl IntoFontCollection for FontFamily {
    #[inline]
    fn into_font_collection(self, _: &CanvasFontContext) -> Arc<FontCollection> {
        let mut font_collection = FontCollection::new();
        font_collection.add_family(self);
        Arc::new(font_collection)
    }
}

impl IntoFontCollection for Vec<FontFamily> {
    #[inline]
    fn into_font_collection(self, _: &CanvasFontContext) -> Arc<FontCollection> {
        let mut font_collection = FontCollection::new();
        for family in self {
            font_collection.add_family(family);
        }
        Arc::new(font_collection)
    }
}

impl IntoFontCollection for Handle {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        self.load().expect("Failed to load the font!").into_font_collection(context)
    }
}

impl<'a> IntoFontCollection for &'a [Handle] {
    #[inline]
    fn into_font_collection(self, _: &CanvasFontContext) -> Arc<FontCollection> {
        let mut font_collection = FontCollection::new();
        for handle in self {
            let font = handle.load().expect("Failed to load the font!");
            font_collection.add_family(FontFamily::new_from_font(font));
        }
        Arc::new(font_collection)
    }
}

impl IntoFontCollection for Font {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        FontFamily::new_from_font(self).into_font_collection(context)
    }
}

impl<'a> IntoFontCollection for &'a [Font] {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        let mut family = FontFamily::new();
        for font in self {
            family.add_font(FontRef::new((*font).clone()))
        }
        family.into_font_collection(context)
    }
}

impl<'a> IntoFontCollection for &'a str {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        context.font_source
               .select_by_postscript_name(self)
               .expect("Couldn't find a font with that PostScript name!")
               .into_font_collection(context)
    }
}

impl<'a, 'b> IntoFontCollection for &'a [&'b str] {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        let mut font_collection = FontCollection::new();
        for postscript_name in self {
            let font = context.font_source
                              .select_by_postscript_name(postscript_name)
                              .expect("Failed to find a font with that PostScript name!")
                              .load()
                              .expect("Failed to load the font!");
            font_collection.add_family(FontFamily::new_from_font(font));
        }
        Arc::new(font_collection)
    }
}
