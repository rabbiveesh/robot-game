//! Hand-drawn art confined to one rect: the Pearl Hop lagoon and the hands-on
//! workspace's counters. Coordinates are absolute (in whatever surface the art
//! is drawing on — the screen or a panel's art slot); debug builds warn once
//! if art strays outside its rect.

use crate::prelude::*;
use crate::ui::metrics::{FontMetrics, TextMetrics};
use crate::ui::rect::UiRect;

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
    pub fn triangle(&self, a: (f32, f32), b: (f32, f32), c: (f32, f32), color: Color) {
        self.check(a.0.min(b.0).min(c.0), a.1.min(b.1).min(c.1), a.0.max(b.0).max(c.0), a.1.max(b.1).max(c.1));
        draw_triangle(vec2(a.0, a.1), vec2(b.0, b.1), vec2(c.0, c.1), color);
    }
    /// One line of text centred on `cx`, on `baseline`.
    pub fn text_centered(&self, s: &str, cx: f32, baseline: f32, size: u16, color: Color) {
        let w = FontMetrics::bundled().width(s, size);
        self.text(s, cx - w / 2.0, baseline, size, color);
    }
    /// One line of text, left edge at `x`, on `baseline`.
    pub fn text(&self, s: &str, x: f32, baseline: f32, size: u16, color: Color) {
        let m = FontMetrics::bundled();
        self.check(x, baseline - m.ascent(size), x + m.width(s, size), baseline + m.descent(size));
        draw_text(s, x, baseline, size as f32, color);
    }
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

#[cfg(debug_assertions)]
fn warn_once(msg: &str) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!("warning: {msg}");
    }
}
