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
//! zooms or rotates. [`with_transform`] places a body (scaled, squashed,
//! spun) by mapping each call's points before they are recorded.

use std::cell::RefCell;

pub use bevy::math::{vec2, Affine2, Mat2, Vec2};

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
    /// Applied to every point before the camera (see [`with_transform`]).
    xf: Option<Affine2>,
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

fn xf() -> Option<Affine2> {
    with(|c| c.xf)
}

/// Draw with `xf` applied to everything `f` draws (on top of any transform
/// already in force): a sprite drawn in its own tile space, placed, scaled,
/// squashed and spun as one. Circles and ellipses stay true ellipses; text
/// only moves and scales.
pub fn with_transform(xf: Affine2, f: impl FnOnce()) {
    let saved = with(|c| {
        let before = c.xf;
        c.xf = Some(before.map_or(xf, |b| b * xf));
        before
    });
    f();
    with(|c| c.xf = saved);
}

/// The ellipse a `rx`×`ry` ellipse turned `rotation` degrees becomes under
/// `m`: its semi-axes and turn (degrees).
fn map_ellipse(m: Mat2, rx: f32, ry: f32, rotation: f32) -> (f32, f32, f32) {
    let a = m * Mat2::from_angle(rotation.to_radians()) * Mat2::from_diagonal(vec2(rx, ry));
    // Singular values of `a` (closed form for 2×2) and the turn of the first
    // left singular vector.
    let (e, f) = ((a.x_axis.x + a.y_axis.y) / 2.0, (a.x_axis.x - a.y_axis.y) / 2.0);
    let (g, h) = ((a.x_axis.y + a.y_axis.x) / 2.0, (a.x_axis.y - a.y_axis.x) / 2.0);
    let (q, r) = ((e * e + h * h).sqrt(), (f * f + g * g).sqrt());
    let (a1, a2) = (g.atan2(f), h.atan2(e));
    let turn = (a2 + a1) / 2.0;
    (q + r, (q - r).abs(), turn.to_degrees())
}

/// How much `m` scales lengths on average (for line widths and text).
fn mean_scale(m: Mat2) -> f32 {
    m.determinant().abs().sqrt()
}

/// Start a frame: clear the list, reset the camera, record screen size and
/// the wall clock `get_time` reports.
pub fn begin_frame(screen: (f32, f32), time: f64) {
    with(|c| {
        c.cmds.clear();
        c.offset = Vec2::ZERO;
        c.xf = None;
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
    let saved = with(|c| (std::mem::replace(&mut c.offset, Vec2::ZERO), c.xf.take()));
    push(Cmd::Layer(layer));
    f();
    push(Cmd::Layer(0));
    with(|c| (c.offset, c.xf) = saved);
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
    if let Some(m) = xf() {
        let [a, b, c, d] = [vec2(x, y), vec2(x + w, y), vec2(x + w, y + h), vec2(x, y + h)].map(|p| m.transform_point2(p));
        let o = off();
        push(Cmd::Triangle { a: a - o, b: b - o, c: c - o, color });
        push(Cmd::Triangle { a: a - o, b: c - o, c: d - o, color });
        return;
    }
    let o = off();
    push(Cmd::Rect { x: x - o.x, y: y - o.y, w, h, color });
}

/// Outline drawn inside the rect.
pub fn draw_rectangle_lines(x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color) {
    if xf().is_some() {
        let t = thickness / 2.0;
        let (l, r, top, bot) = (x + t, x + w - t, y + t, y + h - t);
        draw_line(x, top, x + w, top, thickness, color);
        draw_line(x, bot, x + w, bot, thickness, color);
        draw_line(l, y, l, y + h, thickness, color);
        draw_line(r, y, r, y + h, thickness, color);
        return;
    }
    let o = off();
    push(Cmd::RectLines { x: x - o.x, y: y - o.y, w, h, thickness, color });
}

pub fn draw_circle(x: f32, y: f32, r: f32, color: Color) {
    if xf().is_some() {
        return draw_ellipse(x, y, r, r, 0.0, color);
    }
    let o = off();
    push(Cmd::Circle { x: x - o.x, y: y - o.y, r, color });
}

/// Ring centered on radius `r`.
pub fn draw_circle_lines(x: f32, y: f32, r: f32, thickness: f32, color: Color) {
    if xf().is_some() {
        // A squashed ring: a closed run of short lines.
        const N: usize = 24;
        let at = |i: usize| {
            let a = i as f32 / N as f32 * std::f32::consts::TAU;
            (x + r * a.cos(), y + r * a.sin())
        };
        for i in 0..N {
            let ((x1, y1), (x2, y2)) = (at(i), at(i + 1));
            draw_line(x1, y1, x2, y2, thickness, color);
        }
        return;
    }
    let o = off();
    push(Cmd::CircleLines { x: x - o.x, y: y - o.y, r, thickness, color });
}

/// Ellipse outline: `w`/`h` are the semi-axes, `rotation` is in degrees.
/// Drawn as a closed run of short lines, so it follows any transform.
pub fn draw_ellipse_lines(x: f32, y: f32, w: f32, h: f32, rotation: f32, thickness: f32, color: Color) {
    const N: usize = 28;
    let (s, c) = rotation.to_radians().sin_cos();
    let at = |i: usize| {
        let a = i as f32 / N as f32 * std::f32::consts::TAU;
        let (ex, ey) = (w * a.cos(), h * a.sin());
        (x + ex * c - ey * s, y + ex * s + ey * c)
    };
    for i in 0..N {
        let ((x1, y1), (x2, y2)) = (at(i), at(i + 1));
        draw_line(x1, y1, x2, y2, thickness, color);
    }
}

/// `w`/`h` are the semi-axes; `rotation` is in degrees.
pub fn draw_ellipse(x: f32, y: f32, w: f32, h: f32, rotation: f32, color: Color) {
    let o = off();
    match xf() {
        Some(m) => {
            let p = m.transform_point2(vec2(x, y)) - o;
            let (rx, ry, rotation) = map_ellipse(m.matrix2, w, h, rotation);
            push(Cmd::Ellipse { x: p.x, y: p.y, rx, ry, rotation, color });
        }
        None => push(Cmd::Ellipse { x: x - o.x, y: y - o.y, rx: w, ry: h, rotation, color }),
    }
}

pub fn draw_line(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
    let o = off();
    let (a, b, thickness) = match xf() {
        Some(m) => (m.transform_point2(vec2(x1, y1)), m.transform_point2(vec2(x2, y2)), thickness * mean_scale(m.matrix2)),
        None => (vec2(x1, y1), vec2(x2, y2), thickness),
    };
    push(Cmd::Line { x1: a.x - o.x, y1: a.y - o.y, x2: b.x - o.x, y2: b.y - o.y, thickness, color });
}

pub fn draw_triangle(a: Vec2, b: Vec2, c: Vec2, color: Color) {
    let o = off();
    let [a, b, c] = match xf() {
        Some(m) => [a, b, c].map(|p| m.transform_point2(p)),
        None => [a, b, c],
    };
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
        let (p, drawn) = match xf() {
            Some(m) => (m.transform_point2(vec2(x, y)), (font_size * mean_scale(m.matrix2)).round().max(1.0) as u16),
            None => (vec2(x, y), size),
        };
        push(Cmd::Text { text: text.to_string(), x: p.x - o.x, y: p.y - o.y, size: drawn, color });
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
    fn a_transformed_circle_is_the_ellipse_it_maps_to() {
        // Squashed 2× wide, then spun a quarter turn: tall and thin.
        let m = Mat2::from_angle(std::f32::consts::FRAC_PI_2) * Mat2::from_diagonal(vec2(2.0, 0.5));
        let (rx, ry, turn) = map_ellipse(m, 10.0, 10.0, 0.0);
        assert!((rx - 20.0).abs() < 1e-3 && (ry - 5.0).abs() < 1e-3, "{rx} {ry}");
        assert!((turn.rem_euclid(180.0) - 90.0).abs() < 1e-3, "{turn}");
        // No transform at all: unchanged.
        let (rx, ry, turn) = map_ellipse(Mat2::IDENTITY, 6.0, 3.0, 30.0);
        assert!((rx - 6.0).abs() < 1e-3 && (ry - 3.0).abs() < 1e-3 && (turn - 30.0).abs() < 1e-3);
    }

    #[test]
    fn a_transform_moves_shapes_and_unwinds_after() {
        begin_frame((800.0, 600.0), 0.0);
        with_transform(Affine2::from_translation(vec2(100.0, 50.0)), || draw_circle(1.0, 2.0, 3.0, RED));
        draw_circle(1.0, 2.0, 3.0, RED);
        let cmds = take_commands();
        assert!(matches!(cmds[0], Cmd::Ellipse { x, y, rx, ry, .. } if x == 101.0 && y == 52.0 && (rx - 3.0).abs() < 1e-4 && (ry - 3.0).abs() < 1e-4));
        assert_eq!(cmds[1], Cmd::Circle { x: 1.0, y: 2.0, r: 3.0, color: RED });
    }

    #[test]
    fn text_measures_in_the_bundled_font() {
        // Unifont ASCII is half-width: 8px per char at 16px.
        let d = measure_text("Done", None, 16, 1.0);
        assert_eq!(d.width, 32.0);
        assert!(d.height > 0.0 && d.offset_y > 0.0);
    }
}
