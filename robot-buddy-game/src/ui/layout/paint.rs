//! The painter: the ONLY place migrated panels' pixels come from.
//!
//! Migrated panels (`tests/layout_discipline.rs` lists them) may not call
//! macroquad's raw-coordinate drawing or measuring functions. They paint by
//! handing this module rects and placed texts taken from their `Frame`, plus
//! style (colors, radii, stroke widths). Custom art inside a `Region` goes
//! through a [`Canvas`] bound to that region's rect. macroquad-only: never
//! called from `step` or the headless tests.

use crate::prelude::*;

use super::frame::PlacedText;
use super::rect::UiRect;
use super::text::fit_size_by;

/// Draw every line of a placed text in `color`.
pub fn text(t: &PlacedText, color: Color) {
    for l in &t.lines {
        debug_check_width(&l.text, t.size);
        draw_text(&l.text, l.rect.x, l.baseline, t.size as f32, color);
    }
}

/// Same as [`text`] but for an optional element (absent = nothing to draw).
pub fn text_opt(t: Option<&PlacedText>, color: Color) {
    if let Some(t) = t {
        text(t, color);
    }
}

/// Draw only the first `chars` characters of a placed text, in reading order
/// (the dialogue typewriter). Line breaks were decided on the full text, so
/// words never jump lines as they appear.
pub fn text_prefix(t: &PlacedText, mut chars: usize, color: Color) {
    for l in &t.lines {
        if chars == 0 {
            break;
        }
        let n = l.text.chars().count();
        let shown: String = l.text.chars().take(chars).collect();
        draw_text(&shown, l.rect.x, l.baseline, t.size as f32, color);
        // +1 for the space the wrap swallowed between lines.
        chars = chars.saturating_sub(n + 1);
    }
}

/// Solid fill.
pub fn fill(r: UiRect, color: Color) {
    draw_rectangle(r.x, r.y, r.w, r.h, color);
}

/// Rectangle outline.
pub fn outline(r: UiRect, thickness: f32, color: Color) {
    draw_rectangle_lines(r.x, r.y, r.w, r.h, thickness, color);
}

/// Filled box with an outline — the plain kid-panel tile.
pub fn boxed(r: UiRect, fill_color: Color, thickness: f32, stroke: Color) {
    fill(r, fill_color);
    outline(r, thickness, stroke);
}

/// Filled rounded rectangle (center rect + corner circles).
pub fn round_rect(r: UiRect, radius: f32, color: Color) {
    let rad = radius.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    draw_rectangle(r.x + rad, r.y, r.w - 2.0 * rad, r.h, color);
    draw_rectangle(r.x, r.y + rad, r.w, r.h - 2.0 * rad, color);
    draw_circle(r.x + rad, r.y + rad, rad, color);
    draw_circle(r.x + r.w - rad, r.y + rad, rad, color);
    draw_circle(r.x + rad, r.y + r.h - rad, rad, color);
    draw_circle(r.x + r.w - rad, r.y + r.h - rad, rad, color);
}

/// Dim everything behind a modal (pass the frame's bounds).
pub fn dim(bounds: UiRect, alpha: f32) {
    fill(bounds, Color::new(0.0, 0.0, 0.0, alpha));
}

/// A slow on/off toggle for "press SPACE" hints (`hz` flips per second).
/// Wall-clock, so render-only — never read it from `step`.
pub fn blink(hz: f64) -> bool {
    (get_time() * hz).sin() > 0.0
}

/// Custom art confined to one rect from the frame (a `Region`, or a box the
/// art decorates). Coordinates are absolute; debug builds warn if art strays
/// outside the rect.
pub struct Canvas {
    pub rect: UiRect,
}

pub fn canvas(rect: UiRect) -> Canvas {
    Canvas { rect }
}

impl Canvas {
    fn check(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        #[cfg(debug_assertions)]
        if !self.rect.expand(1.0).contains_rect(&UiRect::new(x0, y0, x1 - x0, y1 - y0)) {
            warn_once("canvas art strays outside its rect");
        }
        #[cfg(not(debug_assertions))]
        let _ = (x0, y0, x1, y1);
    }
    pub fn circle(&self, x: f32, y: f32, r: f32, color: Color) {
        self.check(x - r, y - r, x + r, y + r);
        draw_circle(x, y, r, color);
    }
    pub fn circle_lines(&self, x: f32, y: f32, r: f32, thickness: f32, color: Color) {
        self.check(x - r, y - r, x + r, y + r);
        draw_circle_lines(x, y, r, thickness, color);
    }
    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        self.check(x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2));
        draw_line(x1, y1, x2, y2, thickness, color);
    }
}

/// Smallest font the unmigrated panels (leap, descent) shrink a line to.
pub const MIN_FONT: u16 = 11;

/// Draw `text` centered on baseline `y` in `p`, shrunk (down to [`MIN_FONT`])
/// to fit the panel's width less a 16px margin each side. For the panels that
/// still position lines by hand.
pub fn centered_fitted(text: &str, p: UiRect, y: f32, max: u16, color: Color) {
    let room = p.w - 32.0;
    let size = fit_size_by(text, room, max, MIN_FONT, |t, s| measure_text(t, None, s, 1.0).width);
    let w = measure_text(text, None, size, 1.0).width;
    draw_text(text, p.x + p.w / 2.0 - w / 2.0, y, size as f32, color);
}

#[cfg(debug_assertions)]
fn warn_once(msg: &str) {
    use std::cell::Cell;
    thread_local!(static WARNED: Cell<bool> = const { Cell::new(false) });
    if !WARNED.with(|w| w.replace(true)) {
        macroquad::logging::warn!("{msg}");
    }
}

/// Debug builds: warn (once) if the headless metrics used for layout ever
/// disagree with what macroquad actually renders.
fn debug_check_width(s: &str, size: u16) {
    #[cfg(debug_assertions)]
    {
        use super::metrics::{FontMetrics, MacroquadMetrics, TextMetrics};
        let ours = FontMetrics::bundled().width(s, size);
        let theirs = MacroquadMetrics.width(s, size);
        if (ours - theirs).abs() > 1.0 {
            warn_once(&format!("layout metrics drift: {s:?} at {size}px is {ours} headless vs {theirs} rendered"));
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = (s, size);
}
