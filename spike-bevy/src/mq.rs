//! A ~40-line shim so the game's macroquad sprite functions port nearly
//! verbatim: same top-left, y-down pixel coordinates, same call shapes, drawn
//! through bevy_vector_shapes' immediate-mode painter.

use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;

pub struct Mq<'a, 'w, 's> { p: &'a mut ShapePainter<'w, 's>, w: f32, h: f32, z: f32 }

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color { Color::srgba_u8(r, g, b, a) }

impl<'a, 'w, 's> Mq<'a, 'w, 's> {
    pub fn new(p: &'a mut ShapePainter<'w, 's>, w: f32, h: f32) -> Self { Mq { p, w, h, z: 5.0 } }
    fn at(&mut self, x: f32, y: f32) {
        self.z += 0.001; // later calls paint on top, like macroquad
        self.p.set_translation(Vec3::new(x - self.w / 2.0, self.h / 2.0 - y, self.z));
    }
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        self.at(x + w / 2.0, y + h / 2.0);
        self.p.hollow = false;
        self.p.set_color(c);
        self.p.rect(Vec2::new(w, h));
    }
    pub fn rect_lines(&mut self, x: f32, y: f32, w: f32, h: f32, t: f32, c: Color) {
        self.at(x + w / 2.0, y + h / 2.0);
        self.p.hollow = true;
        self.p.thickness = t;
        self.p.set_color(c);
        self.p.rect(Vec2::new(w, h));
        self.p.hollow = false;
    }
    pub fn circle(&mut self, x: f32, y: f32, r: f32, c: Color) {
        self.at(x, y);
        self.p.set_color(c);
        self.p.circle(r);
    }
    pub fn ellipse(&mut self, x: f32, y: f32, rx: f32, ry: f32, c: Color) {
        self.at(x, y);
        self.p.set_color(c);
        self.p.set_scale(Vec3::new(1.0, ry / rx, 1.0));
        self.p.circle(rx);
        self.p.set_scale(Vec3::ONE);
    }
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, t: f32, c: Color) {
        self.at(0.0, 0.0);
        self.p.set_translation(Vec3::new(-self.w / 2.0, self.h / 2.0, self.z));
        self.p.thickness = t;
        self.p.set_color(c);
        self.p.line(Vec3::new(x1, -y1, 0.0), Vec3::new(x2, -y2, 0.0));
    }
}

const TS: f32 = 48.0;

/// `sprites/robot.rs::draw_robot`, facing down, standing still.
pub fn draw_robot(d: &mut Mq, x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 2.0;
    let bob = (time * 3.0).sin() * 2.0;
    let walk_shift = -1.0;

    d.ellipse(cx, y + TS - 3.0, 11.0, 4.0, rgba(0, 0, 0, 40));
    d.line(cx, cy - 16.0 + bob, cx, cy - 26.0 + bob, 2.0, rgba(120, 144, 156, 255));
    let antenna_bob = (time * 4.0).sin() * 2.0;
    d.circle(cx, cy - 28.0 + bob + antenna_bob, 4.0, rgba(255, 82, 82, 255));

    let body_x = cx - 12.0;
    let body_y = cy - 10.0 + bob;
    d.rect(body_x, body_y, 24.0, 22.0, rgba(176, 190, 197, 255));
    d.rect_lines(body_x, body_y, 24.0, 22.0, 2.0, rgba(120, 144, 156, 255));

    d.rect(cx - 10.0, cy - 20.0 + bob, 20.0, 14.0, rgba(207, 216, 220, 255));
    d.rect_lines(cx - 10.0, cy - 20.0 + bob, 20.0, 14.0, 1.5, rgba(120, 144, 156, 255));

    let blink = (time * 5.0).sin() > 0.95;
    let eye = rgba(0, 230, 118, 255);
    if blink {
        d.rect(cx - 7.0, cy - 16.0 + bob, 6.0, 2.0, eye);
        d.rect(cx + 1.0, cy - 16.0 + bob, 6.0, 2.0, eye);
    } else {
        d.rect(cx - 7.0, cy - 18.0 + bob, 6.0, 6.0, eye);
        d.rect(cx + 1.0, cy - 18.0 + bob, 6.0, 6.0, eye);
        let pupil = rgba(27, 94, 32, 255);
        d.rect(cx - 6.0, cy - 17.0 + bob + 1.0, 3.0, 3.0, pupil);
        d.rect(cx + 2.0, cy - 17.0 + bob + 1.0, 3.0, 3.0, pupil);
    }
    d.line(cx - 4.0, cy - 8.0 + bob, cx + 4.0, cy - 8.0 + bob, 1.5, eye);

    let arm = rgba(144, 164, 174, 255);
    d.rect(cx - 16.0, cy - 6.0 + bob + walk_shift, 5.0, 12.0, arm);
    d.rect(cx + 11.0, cy - 6.0 + bob - walk_shift, 5.0, 12.0, arm);
    let leg = rgba(120, 144, 156, 255);
    d.rect(cx - 8.0, cy + 12.0 + bob, 6.0, 8.0 + walk_shift, leg);
    d.rect(cx + 2.0, cy + 12.0 + bob, 6.0, 8.0 - walk_shift, leg);

    let pulse = (time * 2.0).sin() * 0.3 + 0.7;
    d.circle(cx, cy + bob, 3.0, rgba(0, 230, 118, (pulse * 255.0) as u8));
}
