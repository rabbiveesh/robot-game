//! Render-side helpers for painting a [`Frame`](super::Frame). macroquad-only:
//! never called from `step` or the headless tests.

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

/// Filled box with a thin outline — the plain kid-panel tile.
pub fn boxed(r: UiRect, fill: Color, line: f32, stroke: Color) {
    draw_rectangle(r.x, r.y, r.w, r.h, fill);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, line, stroke);
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

/// Debug builds: warn (once) if the headless metrics used for layout ever
/// disagree with what macroquad actually renders.
fn debug_check_width(s: &str, size: u16) {
    #[cfg(debug_assertions)]
    {
        use super::metrics::{FontMetrics, MacroquadMetrics, TextMetrics};
        use std::cell::Cell;
        thread_local!(static WARNED: Cell<bool> = const { Cell::new(false) });
        if WARNED.with(|w| w.get()) {
            return;
        }
        let ours = FontMetrics::bundled().width(s, size);
        let theirs = MacroquadMetrics.width(s, size);
        if (ours - theirs).abs() > 1.0 {
            WARNED.with(|w| w.set(true));
            macroquad::logging::warn!(
                "layout metrics drift: {s:?} at {size}px is {ours} headless vs {theirs} rendered"
            );
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = (s, size);
}
