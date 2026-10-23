//! Immediate-mode 2D drawing with macroquad's call shapes, drawn by Bevy.
//!
//! The game's procedural art (sprites, tiles, minigame boards) is ~5k lines of
//! `draw_rectangle` / `draw_circle` / `draw_text` calls in top-left, y-down
//! pixel coordinates. Rather than rewrite all of it, this module keeps those
//! signatures: each call appends a [`Cmd`] to a per-frame list, and
//! [`crate::app`] paints the list through `bevy_vector_shapes` (shapes) and a
//! pool of `Text2d` entities (text), later calls on top, exactly like
//! macroquad's painter's order.
//!
//! Recording is pure (no Bevy world needed), so `Game::render` can run in
//! tests too. `set_camera` is a translation only: the game's camera never
//! zooms or rotates.

use std::cell::RefCell;

pub use bevy::math::{vec2, Vec2};

/// An sRGB color with straight alpha, components in `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color { r, g, b, a }
    }
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0)
    }
}

pub const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
pub const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
pub const BLANK: Color = Color::new(0.0, 0.0, 0.0, 0.0);
pub const RED: Color = Color::new(0.90, 0.16, 0.22, 1.0);
pub const GOLD: Color = Color::new(1.0, 0.80, 0.0, 1.0);
pub const ORANGE: Color = Color::new(1.0, 0.63, 0.0, 1.0);

/// Axis-aligned rectangle, top-left origin.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
}

/// What `measure_text` reports, as macroquad did: advance width, glyph-box
/// height, and the distance from the glyph box's top to the baseline.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TextDimensions {
    pub width: f32,
    pub height: f32,
    pub offset_y: f32,
}

/// The game camera: `target` is the world point at the middle of the screen.
/// `zoom` is accepted for macroquad compatibility and must be 1:1
/// (`2 / screen`), which is all the game ever used.
#[derive(Clone, Copy, Debug, Default)]
pub struct Camera2D {
    pub zoom: Vec2,
    pub target: Vec2,
}

/// One recorded draw call, in screen pixels (camera already applied).
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    Rect { x: f32, y: f32, w: f32, h: f32, color: Color },
    RectLines { x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color },
    Circle { x: f32, y: f32, r: f32, color: Color },
    CircleLines { x: f32, y: f32, r: f32, thickness: f32, color: Color },
    Ellipse { x: f32, y: f32, rx: f32, ry: f32, rotation: f32, color: Color },
    Line { x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color },
    Triangle { a: Vec2, b: Vec2, c: Vec2, color: Color },
    /// Baseline-anchored text, like macroquad's `draw_text`.
    Text { text: String, x: f32, y: f32, size: u16, color: Color },
    /// Everything after this goes to render layer `n`: 0 is the screen, any
    /// other is one panel's art slot (see [`art`]).
    Layer(u8),
}

#[derive(Default)]
struct Ctx {
    cmds: Vec<Cmd>,
    /// Subtracted from every world coordinate while a camera is set.
    offset: Vec2,
    screen: (f32, f32),
    time: f64,
}

thread_local! {
    static CTX: RefCell<Ctx> = RefCell::new(Ctx { screen: (960.0, 720.0), ..Default::default() });
}

fn with<R>(f: impl FnOnce(&mut Ctx) -> R) -> R {
    CTX.with(|c| f(&mut c.borrow_mut()))
}

fn push(cmd: Cmd) {
    with(|c| c.cmds.push(cmd));
}

fn off() -> Vec2 {
    with(|c| c.offset)
}

/// Start a frame: clear the list, reset the camera, record screen size and
/// the wall clock `get_time` reports.
pub fn begin_frame(screen: (f32, f32), time: f64) {
    with(|c| {
        c.cmds.clear();
        c.offset = Vec2::ZERO;
        c.screen = screen;
        c.time = time;
    });
}

/// Everything recorded since [`begin_frame`], in paint order.
pub fn take_commands() -> Vec<Cmd> {
    with(|c| std::mem::take(&mut c.cmds))
}

/// Record art for the panel slot on render layer `layer` (`w`×`h` logical
/// pixels): `f` draws in the slot's own coordinates, top-left at (0, 0), and
/// the slot shows exactly that (anything outside is cut off). The camera is
/// ignored inside and restored after.
pub fn art(layer: u8, f: impl FnOnce()) {
    debug_assert!(layer > 0, "layer 0 is the screen");
    let saved = with(|c| std::mem::replace(&mut c.offset, Vec2::ZERO));
    push(Cmd::Layer(layer));
    f();
    push(Cmd::Layer(0));
    with(|c| c.offset = saved);
}

pub fn screen_width() -> f32 {
    with(|c| c.screen.0)
}

pub fn screen_height() -> f32 {
    with(|c| c.screen.1)
}

/// Seconds since the app started. Render-only: `step` gets `dt` instead.
pub fn get_time() -> f64 {
    with(|c| c.time)
}

pub fn set_camera(cam: &Camera2D) {
    with(|c| {
        debug_assert!(
            (cam.zoom.x * c.screen.0 - 2.0).abs() < 0.01,
            "draw::Camera2D only supports 1:1 zoom"
        );
        c.offset = cam.target - Vec2::new(c.screen.0, c.screen.1) / 2.0;
    });
}

pub fn set_default_camera() {
    with(|c| c.offset = Vec2::ZERO);
}

/// Fill the whole screen (ignores the camera).
pub fn clear_background(color: Color) {
    let (w, h) = with(|c| c.screen);
    push(Cmd::Rect { x: 0.0, y: 0.0, w, h, color });
}

pub fn draw_rectangle(x: f32, y: f32, w: f32, h: f32, color: Color) {
    let o = off();
    push(Cmd::Rect { x: x - o.x, y: y - o.y, w, h, color });
}

/// Outline drawn inside the rect.
pub fn draw_rectangle_lines(x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color) {
    let o = off();
    push(Cmd::RectLines { x: x - o.x, y: y - o.y, w, h, thickness, color });
}

pub fn draw_circle(x: f32, y: f32, r: f32, color: Color) {
    let o = off();
    push(Cmd::Circle { x: x - o.x, y: y - o.y, r, color });
}

/// Ring centered on radius `r`.
pub fn draw_circle_lines(x: f32, y: f32, r: f32, thickness: f32, color: Color) {
    let o = off();
    push(Cmd::CircleLines { x: x - o.x, y: y - o.y, r, thickness, color });
}

/// `w`/`h` are the semi-axes; `rotation` is in degrees.
pub fn draw_ellipse(x: f32, y: f32, w: f32, h: f32, rotation: f32, color: Color) {
    let o = off();
    push(Cmd::Ellipse { x: x - o.x, y: y - o.y, rx: w, ry: h, rotation, color });
}

pub fn draw_line(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
    let o = off();
    push(Cmd::Line { x1: x1 - o.x, y1: y1 - o.y, x2: x2 - o.x, y2: y2 - o.y, thickness, color });
}

pub fn draw_triangle(a: Vec2, b: Vec2, c: Vec2, color: Color) {
    let o = off();
    push(Cmd::Triangle { a: a - o, b: b - o, c: c - o, color });
}

/// Regular polygon, first vertex at `rotation` degrees from +x (y-down).
pub fn draw_poly(x: f32, y: f32, sides: u8, radius: f32, rotation: f32, color: Color) {
    let center = vec2(x, y);
    let vertex = |i: u8| {
        let a = (i as f32 / sides as f32) * std::f32::consts::TAU + rotation.to_radians();
        center + vec2(a.cos(), a.sin()) * radius
    };
    for i in 0..sides {
        draw_triangle(center, vertex(i), vertex((i + 1) % sides), color);
    }
}

/// Text with its baseline at `y`, in the bundled font.
pub fn draw_text(text: &str, x: f32, y: f32, font_size: f32, color: Color) -> TextDimensions {
    let size = font_size as u16;
    if !text.is_empty() {
        let o = off();
        push(Cmd::Text { text: text.to_string(), x: x - o.x, y: y - o.y, size, color });
    }
    measure_text(text, None, size, 1.0)
}

/// Measure `text` in the bundled font. `_font` and `font_scale` mirror
/// macroquad's signature; call sites pass `None` and `1.0`.
pub fn measure_text(text: &str, _font: Option<&()>, font_size: u16, font_scale: f32) -> TextDimensions {
    let size = (font_size as f32 * font_scale).round() as u16;
    crate::ui::metrics::FontMetrics::bundled().dimensions(text, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_offsets_world_draws_but_not_the_background() {
        begin_frame((800.0, 600.0), 0.0);
        set_camera(&Camera2D { zoom: vec2(2.0 / 800.0, 2.0 / 600.0), target: vec2(500.0, 400.0) });
        clear_background(BLACK);
        draw_rectangle(100.0, 100.0, 10.0, 10.0, WHITE);
        set_default_camera();
        draw_circle(5.0, 5.0, 1.0, RED);
        let cmds = take_commands();
        assert_eq!(cmds[0], Cmd::Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0, color: BLACK });
        assert_eq!(cmds[1], Cmd::Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0, color: WHITE });
        assert_eq!(cmds[2], Cmd::Circle { x: 5.0, y: 5.0, r: 1.0, color: RED });
    }

    #[test]
    fn text_measures_in_the_bundled_font() {
        // Unifont ASCII is half-width: 8px per char at 16px.
        let d = measure_text("Done", None, 16, 1.0);
        assert_eq!(d.width, 32.0);
        assert!(d.height > 0.0 && d.offset_y > 0.0);
    }
}
