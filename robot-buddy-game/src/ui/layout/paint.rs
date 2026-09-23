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

/// A top-to-bottom colour fade over `r`, in `steps` flat bands.
pub fn vgradient(r: UiRect, top: Color, bottom: Color, steps: usize) {
    let steps = steps.max(1);
    let band = r.h / steps as f32;
    for i in 0..steps {
        let t = if steps == 1 { 0.0 } else { i as f32 / (steps - 1) as f32 };
        let c = Color::new(
            top.r + (bottom.r - top.r) * t,
            top.g + (bottom.g - top.g) * t,
            top.b + (bottom.b - top.b) * t,
            top.a + (bottom.a - top.a) * t,
        );
        // +1 so rounding never leaves a hairline gap between bands.
        draw_rectangle(r.x, r.y + band * i as f32, r.w, (band + 1.0).min(r.bottom() - (r.y + band * i as f32)), c);
    }
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
    pub fn rect(&self, r: UiRect, color: Color) {
        self.check(r.x, r.y, r.right(), r.bottom());
        draw_rectangle(r.x, r.y, r.w, r.h, color);
    }
    /// Outline drawn inside `r`.
    pub fn rect_lines(&self, r: UiRect, thickness: f32, color: Color) {
        self.check(r.x, r.y, r.right(), r.bottom());
        draw_rectangle_lines(r.x, r.y, r.w, r.h, thickness, color);
    }
    /// Filled ellipse with radii `rx`, `ry`, turned `rotation` degrees.
    pub fn ellipse(&self, x: f32, y: f32, rx: f32, ry: f32, rotation: f32, color: Color) {
        let r = rx.max(ry);
        self.check(x - r, y - r, x + r, y + r);
        draw_ellipse(x, y, rx, ry, rotation, color);
    }
    /// Filled triangle.
    pub fn triangle(&self, a: (f32, f32), b: (f32, f32), c: (f32, f32), color: Color) {
        self.check(a.0.min(b.0).min(c.0), a.1.min(b.1).min(c.1), a.0.max(b.0).max(c.0), a.1.max(b.1).max(c.1));
        draw_triangle(vec2(a.0, a.1), vec2(b.0, b.1), vec2(c.0, c.1), color);
    }
    /// One line of text centred on `cx`, on `baseline`.
    pub fn text_centered(&self, s: &str, cx: f32, baseline: f32, size: u16, color: Color) {
        use super::metrics::{FontMetrics, TextMetrics};
        let w = FontMetrics::bundled().width(s, size);
        self.text(s, cx - w / 2.0, baseline, size, color);
    }
    /// One line of text, left edge at `x`, on `baseline`.
    pub fn text(&self, s: &str, x: f32, baseline: f32, size: u16, color: Color) {
        use super::metrics::{FontMetrics, TextMetrics};
        let m = FontMetrics::bundled();
        self.check(x, baseline - m.ascent(size), x + m.width(s, size), baseline + m.descent(size));
        debug_check_width(s, size);
        draw_text(s, x, baseline, size as f32, color);
    }
}

/// Smallest font the unmigrated panels (descent) shrink a line to.
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
