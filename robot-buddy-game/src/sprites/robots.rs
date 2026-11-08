//! Robot Land's residents. Each one is broken in exactly one funny way, and
//! the sprite is where the joke lives: toast launches, heads boing off,
//! moustaches bristle. All animation is a pure function of `time`.

use crate::prelude::*;
use super::Dir;

const TS: f32 = 48.0;

fn shadow(cx: f32, y: f32, rx: f32) {
    draw_ellipse(cx, y + TS - 3.0, rx, 4.0, 0.0, Color::from_rgba(0, 0, 0, 45));
}

fn ink() -> Color { Color::from_rgba(33, 33, 40, 255) }

/// Little speech pop above a sprite ("BEEP!", "*squeak*"), shown for `on`
/// seconds out of every `every`. `phase` staggers different speakers.
fn pop_word(cx: f32, top: f32, time: f32, word: &str, every: f32, on: f32, phase: f32) {
    let t = (time + phase) % every;
    if t > on { return; }
    let rise = (t / on) * 6.0;
    let size = 15.0;
    let w = measure_text(word, None, size as u16, 1.0).width;
    let (bx, by) = (cx - w / 2.0 - 4.0, top - 16.0 - rise);
    draw_rectangle(bx, by, w + 8.0, 15.0, Color::new(1.0, 1.0, 1.0, 0.92));
    draw_rectangle_lines(bx, by, w + 8.0, 15.0, 1.0, ink());
    draw_text(word, bx + 4.0, by + 11.5, size, ink());
}

/// Toasty: a chrome toaster on stubby legs. Every couple of seconds he gets
/// excited and two slices of toast rocket out of his head and fall back in.
pub fn draw_toaster(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let cycle = 2.4;
    let t = time % cycle;
    // The whole toaster jolts on the POP.
    let jolt = if t < 0.12 { -3.0 } else { 0.0 };
    let by = y + 18.0 + jolt;
    shadow(cx, y, 14.0);

    // Legs
    draw_line(cx - 8.0, by + 18.0, cx - 9.0, y + TS - 4.0, 3.0, ink());
    draw_line(cx + 8.0, by + 18.0, cx + 9.0, y + TS - 4.0, 3.0, ink());

    // Toast flight: up and back down over the first 1.4s of the cycle.
    if t < 1.4 {
        let k = t / 1.4;
        let h = (k * std::f32::consts::PI).sin() * 34.0;
        for (i, dx) in [-5.0_f32, 5.0].iter().enumerate() {
            let tx = cx + dx + if i == 0 { -k * 4.0 } else { k * 4.0 };
            let ty = by - 4.0 - h;
            draw_rectangle(tx - 4.0, ty - 9.0, 8.0, 12.0, Color::from_rgba(170, 100, 40, 255));
            draw_rectangle(tx - 3.0, ty - 8.0, 6.0, 10.0, Color::from_rgba(240, 200, 120, 255));
        }
    }

    // Body — rounded chrome box
    let chrome = Color::from_rgba(200, 210, 222, 255);
    draw_rectangle(cx - 14.0, by, 28.0, 20.0, chrome);
    draw_circle(cx - 12.0, by + 2.0, 2.0, chrome);
    draw_circle(cx + 12.0, by + 2.0, 2.0, chrome);
    draw_rectangle(cx - 14.0, by + 2.0, 28.0, 3.0, Color::new(1.0, 1.0, 1.0, 0.5));
    // Slots
    draw_rectangle(cx - 10.0, by + 1.0, 8.0, 2.0, ink());
    draw_rectangle(cx + 2.0, by + 1.0, 8.0, 2.0, ink());
    // Lever
    let lever = if t < 1.4 { by + 4.0 } else { by + 12.0 };
    draw_rectangle(cx + 14.0, lever, 4.0, 3.0, Color::from_rgba(230, 60, 60, 255));

    // Face: eyes go wide at the pop, happy squint otherwise
    if t < 0.6 {
        draw_circle(cx - 5.0, by + 9.0, 3.0, WHITE);
        draw_circle(cx + 5.0, by + 9.0, 3.0, WHITE);
        draw_circle(cx - 5.0, by + 9.0, 1.4, ink());
        draw_circle(cx + 5.0, by + 9.0, 1.4, ink());
        draw_circle(cx, by + 15.0, 2.2, ink());
    } else {
        draw_line(cx - 7.0, by + 9.0, cx - 3.0, by + 8.0, 1.5, ink());
        draw_line(cx + 3.0, by + 8.0, cx + 7.0, by + 9.0, 1.5, ink());
        draw_line(cx - 4.0, by + 14.0, cx + 4.0, by + 14.0, 1.5, ink());
        draw_circle(cx - 9.0, by + 12.0, 2.0, Color::from_rgba(255, 140, 140, 160));
        draw_circle(cx + 9.0, by + 12.0, 2.0, Color::from_rgba(255, 140, 140, 160));
    }
    pop_word(cx, y - 20.0, time, "POP!", cycle, 0.35, 0.0);
}

/// Clank: a boxy robot whose head is on a spring. Every few seconds the head
/// launches skyward, spring stretched to its limit, eyes spinning.
pub fn draw_spring_bot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let body_top = y + 26.0;
    shadow(cx, y, 11.0);
    let blue = Color::from_rgba(80, 150, 230, 255);

    // Legs + body
    draw_rectangle(cx - 7.0, y + TS - 9.0, 5.0, 6.0, ink());
    draw_rectangle(cx + 2.0, y + TS - 9.0, 5.0, 6.0, ink());
    draw_rectangle(cx - 10.0, body_top, 20.0, 14.0, blue);
    draw_rectangle_lines(cx - 10.0, body_top, 20.0, 14.0, 1.5, ink());
    draw_circle(cx - 4.0, body_top + 7.0, 2.0, Color::from_rgba(255, 220, 80, 255));
    draw_circle(cx + 3.0, body_top + 7.0, 2.0, Color::from_rgba(255, 90, 90, 255));
    // Arms flail when the head goes
    let cycle = 3.0;
    let t = time % cycle;
    let launching = t < 1.1;
    let flail = if launching { (time * 25.0).sin() * 6.0 } else { 0.0 };
    draw_line(cx - 10.0, body_top + 3.0, cx - 16.0, body_top + 9.0 - flail, 2.0, ink());
    draw_line(cx + 10.0, body_top + 3.0, cx + 16.0, body_top + 9.0 + flail, 2.0, ink());

    // Head height: a big boing, then a damped wobble settling back on.
    let lift = if launching {
        (t / 1.1 * std::f32::consts::PI).sin() * 30.0
    } else {
        let s = t - 1.1;
        (s * 14.0).sin().abs() * 5.0 * (-s * 2.5).exp()
    };
    let head_y = body_top - 10.0 - lift;

    // Spring: zigzag from body to head
    let coils = 6;
    let (top, bot) = (head_y + 9.0, body_top);
    for i in 0..coils {
        let a = top + (bot - top) * i as f32 / coils as f32;
        let b = top + (bot - top) * (i + 1) as f32 / coils as f32;
        let side = if i % 2 == 0 { 4.0 } else { -4.0 };
        draw_line(cx - side, a, cx + side, b, 1.5, Color::from_rgba(190, 190, 200, 255));
    }

    // Head
    draw_rectangle(cx - 9.0, head_y - 7.0, 18.0, 15.0, blue);
    draw_rectangle_lines(cx - 9.0, head_y - 7.0, 18.0, 15.0, 1.5, ink());
    if launching {
        // Spiral-eyes of a robot whose head is currently in orbit.
        for ex in [cx - 4.0, cx + 4.0] {
            draw_circle(ex, head_y - 1.0, 3.0, WHITE);
            let a = time * 18.0;
            draw_line(ex, head_y - 1.0, ex + a.cos() * 2.5, head_y - 1.0 + a.sin() * 2.5, 1.2, ink());
        }
        draw_circle(cx, head_y + 5.0, 2.0, ink());
    } else {
        draw_circle(cx - 4.0, head_y - 1.0, 2.0, ink());
        draw_circle(cx + 4.0, head_y - 1.0, 2.0, ink());
        draw_line(cx - 3.0, head_y + 4.0, cx + 3.0, head_y + 4.0, 1.2, ink());
    }
    pop_word(cx, head_y - 8.0, time, "BOING!", cycle, 0.5, 0.0);
}

/// Sir Vacuums-a-Lot: a robot vacuum wearing a tiny crown and a magnificent
/// moustache, brushes whirring, with a little dust cloud behind him.
pub fn draw_roomba(x: f32, y: f32, dir: Dir, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS - 14.0;
    shadow(cx, y, 17.0);
    let shell = Color::from_rgba(70, 72, 84, 255);

    // Spinning side brushes
    for side in [-1.0_f32, 1.0] {
        let bx = cx + side * 15.0;
        for k in 0..3 {
            let a = time * 14.0 * side + k as f32 * 2.1;
            draw_line(bx, cy + 4.0, bx + a.cos() * 6.0, cy + 4.0 + a.sin() * 3.0, 1.2, Color::from_rgba(220, 220, 220, 255));
        }
    }
    // Disc body
    draw_ellipse(cx, cy, 17.0, 9.0, 0.0, shell);
    draw_ellipse(cx, cy - 2.0, 15.0, 7.0, 0.0, Color::from_rgba(96, 100, 116, 255));
    draw_ellipse_lines(cx, cy, 17.0, 9.0, 0.0, 1.5, ink());
    // Status light
    let blink = (time * 4.0).sin() > 0.0;
    draw_circle(cx + 9.0, cy - 3.0, 1.6, if blink { Color::from_rgba(80, 230, 120, 255) } else { Color::from_rgba(30, 90, 50, 255) });
    // Eyes looking where he's headed
    let look = match dir { Dir::Left => -2.0, Dir::Right => 2.0, _ => 0.0 };
    draw_circle(cx - 4.0, cy - 4.0, 2.4, WHITE);
    draw_circle(cx + 4.0, cy - 4.0, 2.4, WHITE);
    draw_circle(cx - 4.0 + look * 0.5, cy - 4.0, 1.1, ink());
    draw_circle(cx + 4.0 + look * 0.5, cy - 4.0, 1.1, ink());
    // The moustache, which twitches with dignity
    let tw = (time * 3.0).sin() * 1.5;
    let tache = Color::from_rgba(90, 50, 30, 255);
    draw_triangle(vec2(cx, cy), vec2(cx - 9.0, cy - 1.0 - tw), vec2(cx - 3.0, cy + 2.0), tache);
    draw_triangle(vec2(cx, cy), vec2(cx + 9.0, cy - 1.0 - tw), vec2(cx + 3.0, cy + 2.0), tache);
    draw_circle(cx - 9.0, cy - 2.0 - tw, 1.5, tache);
    draw_circle(cx + 9.0, cy - 2.0 - tw, 1.5, tache);
    // Tiny crown, slightly crooked
    let gold = Color::from_rgba(255, 205, 50, 255);
    let (kx, ky) = (cx + 1.0, cy - 9.0);
    draw_rectangle(kx - 5.0, ky - 2.0, 10.0, 3.0, gold);
    draw_triangle(vec2(kx - 5.0, ky - 2.0), vec2(kx - 4.0, ky - 7.0), vec2(kx - 2.0, ky - 2.0), gold);
    draw_triangle(vec2(kx - 2.0, ky - 2.0), vec2(kx, ky - 8.0), vec2(kx + 2.0, ky - 2.0), gold);
    draw_triangle(vec2(kx + 2.0, ky - 2.0), vec2(kx + 4.0, ky - 7.0), vec2(kx + 5.0, ky - 2.0), gold);
    draw_circle(kx, ky - 1.0, 1.0, Color::from_rgba(230, 50, 80, 255));
    // Dust puffs trailing behind
    for i in 0..3 {
        let k = ((time * 1.5 + i as f32 * 0.33) % 1.0).abs();
        let back = match dir { Dir::Left => 1.0, _ => -1.0 };
        draw_circle(cx + back * (18.0 + k * 10.0), cy + 3.0 - k * 6.0, 2.0 + k * 3.0, Color::new(0.8, 0.8, 0.8, 0.4 * (1.0 - k)));
    }
}

/// Beep: a very small dome robot with a very large antenna. Says one word.
pub fn draw_tiny_bot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let hop = ((time * 4.0).sin().max(0.0)) * 4.0;
    let cy = y + TS - 12.0 - hop;
    shadow(cx, y, 8.0);
    let body = Color::from_rgba(255, 150, 60, 255);
    // Feet
    draw_circle(cx - 4.0, y + TS - 6.0, 2.5, ink());
    draw_circle(cx + 4.0, y + TS - 6.0, 2.5, ink());
    // Dome
    draw_circle(cx, cy, 9.0, body);
    draw_rectangle(cx - 9.0, cy, 18.0, 5.0, body);
    draw_circle(cx - 3.0, cy - 2.0, 2.6, WHITE);
    draw_circle(cx + 3.0, cy - 2.0, 2.6, WHITE);
    draw_circle(cx - 3.0, cy - 1.5, 1.3, ink());
    draw_circle(cx + 3.0, cy - 1.5, 1.3, ink());
    // Absurdly tall wobbly antenna
    let sway = (time * 2.5).sin() * 5.0;
    draw_line(cx, cy - 9.0, cx + sway, cy - 30.0, 1.5, ink());
    let lit = (time * 6.0).sin() > 0.0;
    draw_circle(cx + sway, cy - 31.0, 3.0, if lit { Color::from_rgba(255, 60, 60, 255) } else { Color::from_rgba(120, 30, 30, 255) });
    pop_word(cx, cy - 34.0, time, "BEEP!", 2.2, 0.7, 0.9);
}

/// Grandpa Gearbox: a rusty, hunched old robot with a cane, a bushy steel-
/// wool moustache, and a squeak every few seconds. Bits of rust flake off.
pub fn draw_rusty_bot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let creak = ((time * 0.9).sin() * 0.5 + 0.5) * 2.0;
    let top = y + 16.0 + creak;
    shadow(cx, y, 12.0);
    let rust = Color::from_rgba(178, 98, 52, 255);
    let rust_dark = Color::from_rgba(124, 64, 34, 255);

    // Cane
    draw_line(cx + 14.0, top + 14.0, cx + 15.0, y + TS - 3.0, 2.0, Color::from_rgba(120, 80, 40, 255));
    draw_line(cx + 10.0, top + 14.0, cx + 14.0, top + 14.0, 2.0, Color::from_rgba(120, 80, 40, 255));
    // Legs, a bit bowed
    draw_line(cx - 5.0, top + 22.0, cx - 7.0, y + TS - 4.0, 3.0, rust_dark);
    draw_line(cx + 5.0, top + 22.0, cx + 7.0, y + TS - 4.0, 3.0, rust_dark);
    // Body, hunched forward
    draw_rectangle(cx - 10.0, top + 8.0, 20.0, 15.0, rust);
    for (px, py) in [(-6.0, 11.0), (4.0, 17.0), (-2.0, 20.0)] {
        draw_circle(cx + px, top + py, 1.6, rust_dark);
    }
    // Arm on the cane
    draw_line(cx + 8.0, top + 11.0, cx + 12.0, top + 14.0, 2.5, rust_dark);
    // Head
    draw_rectangle(cx - 9.0, top - 6.0, 16.0, 14.0, rust);
    draw_rectangle_lines(cx - 9.0, top - 6.0, 16.0, 14.0, 1.0, rust_dark);
    // One eye squinting, one big monocle eye
    draw_line(cx - 6.0, top, cx - 2.0, top, 1.5, ink());
    draw_circle(cx + 3.0, top, 3.5, WHITE);
    draw_circle_lines(cx + 3.0, top, 4.0, 1.2, Color::from_rgba(255, 205, 50, 255));
    draw_circle(cx + 3.0, top, 1.4, ink());
    // Steel-wool moustache
    let wool = Color::from_rgba(215, 215, 222, 255);
    for i in 0..5 {
        draw_circle(cx - 6.0 + i as f32 * 3.0, top + 5.0, 2.2, wool);
    }
    // Rust flakes drifting down
    for i in 0..3 {
        let k = ((time * 0.6 + i as f32 * 0.37) % 1.0).abs();
        draw_rectangle(cx - 8.0 + i as f32 * 7.0, top + 10.0 + k * 22.0, 2.0, 2.0, Color::new(0.7, 0.38, 0.2, 1.0 - k));
    }
    pop_word(cx, top - 8.0, time, "*squeak*", 3.4, 0.8, 1.7);
}

/// Sockbot: a tall, thin sorting robot with socks on every limb and one
/// sock-shaped hole in its heart where the missing stripy sock should be.
pub fn draw_sock_bot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let bob = (time * 2.0).sin() * 1.5;
    let top = y + 10.0 + bob;
    shadow(cx, y, 10.0);
    let teal = Color::from_rgba(60, 190, 180, 255);

    // Wheel
    draw_circle(cx, y + TS - 7.0, 5.0, ink());
    draw_circle(cx, y + TS - 7.0, 2.0, Color::from_rgba(160, 160, 170, 255));
    // Neck pole + body
    draw_rectangle(cx - 8.0, top + 12.0, 16.0, 18.0, teal);
    draw_rectangle_lines(cx - 8.0, top + 12.0, 16.0, 18.0, 1.5, ink());
    // Arms up, juggling socks
    let juggle = time * 3.0;
    draw_line(cx - 8.0, top + 15.0, cx - 16.0, top + 6.0, 2.0, ink());
    draw_line(cx + 8.0, top + 15.0, cx + 16.0, top + 6.0, 2.0, ink());
    let socks = [
        Color::from_rgba(240, 90, 160, 255),
        Color::from_rgba(255, 210, 60, 255),
        Color::from_rgba(120, 140, 255, 255),
    ];
    for (i, c) in socks.iter().enumerate() {
        let a = juggle + i as f32 * std::f32::consts::TAU / 3.0;
        let sx = cx + a.cos() * 15.0;
        let sy = top + 2.0 - a.sin().abs() * 12.0;
        draw_rectangle(sx - 2.0, sy - 4.0, 4.0, 7.0, *c);
        draw_rectangle(sx - 2.0, sy + 1.0, 7.0, 3.0, *c);
    }
    // Head
    draw_circle(cx, top + 6.0, 7.0, teal);
    draw_circle_lines(cx, top + 6.0, 7.0, 1.5, ink());
    draw_circle(cx - 2.5, top + 5.0, 1.5, ink());
    draw_circle(cx + 2.5, top + 5.0, 1.5, ink());
    draw_line(cx - 2.0, top + 9.0, cx + 2.0, top + 9.0, 1.0, ink());
    // A sock on the head, worn as a hat, obviously
    draw_rectangle(cx - 5.0, top - 4.0, 10.0, 5.0, Color::from_rgba(255, 255, 255, 255));
    draw_rectangle(cx - 5.0, top - 2.0, 10.0, 1.5, Color::from_rgba(230, 60, 60, 255));
    // Sock-shaped outline on the chest where the stripy one should be
    draw_rectangle_lines(cx - 3.0, top + 16.0, 4.0, 8.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.8));
    draw_rectangle_lines(cx - 3.0, top + 22.0, 7.0, 3.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.8));
}

// ─── The houses of Robot Land ───────────────────────────

/// Lampy: a desk lamp on a springy arm who is scared of the dark, so he never,
/// ever turns his light off. He jumps when it flickers.
pub fn draw_lamp_bot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    shadow(cx, y, 11.0);
    let metal = Color::from_rgba(70, 120, 200, 255);
    // A flicker every few seconds, and a startled jump with it.
    let flick = (time % 3.1) < 0.12;
    let jump = if flick { -4.0 } else { 0.0 };
    // Base.
    draw_ellipse(cx, y + TS - 6.0, 11.0, 4.0, 0.0, metal);
    // Two-part arm.
    let elbow = (cx - 6.0, y + 26.0 + jump);
    let head = (cx + 5.0, y + 12.0 + jump);
    draw_line(cx, y + TS - 8.0, elbow.0, elbow.1, 3.0, ink());
    draw_line(elbow.0, elbow.1, head.0, head.1, 3.0, ink());
    draw_circle(elbow.0, elbow.1, 2.5, metal);
    // Light cone (not when it flickers!).
    if !flick {
        draw_triangle(vec2(head.0 + 2.0, head.1 + 4.0), vec2(head.0 - 10.0, y + TS - 4.0), vec2(head.0 + 18.0, y + TS - 4.0),
            Color::new(1.0, 0.95, 0.55, 0.25));
    }
    // Shade, with worried eyes on it.
    draw_triangle(vec2(head.0 - 9.0, head.1 + 5.0), vec2(head.0 + 11.0, head.1 + 5.0), vec2(head.0 + 1.0, head.1 - 7.0), metal);
    draw_circle(head.0 - 2.0, head.1, 1.8, WHITE);
    draw_circle(head.0 + 4.0, head.1, 1.8, WHITE);
    draw_circle(head.0 - 2.0, head.1 + 0.5, 0.9, ink());
    draw_circle(head.0 + 4.0, head.1 + 0.5, 0.9, ink());
    let bulb = if flick { Color::from_rgba(90, 90, 70, 255) } else { Color::from_rgba(255, 240, 140, 255) };
    draw_circle(head.0 + 1.0, head.1 + 6.0, 3.0, bulb);
    if flick {
        pop_word(cx, y + 6.0, time, "EEK!", 3.1, 0.6, 0.0);
    }
}

/// Hic: a round little robot with the hiccups. Every hiccup bounces him up
/// with a puff of bubbles and a "HIC!".
pub fn draw_hiccup_bot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let t = time % 1.7;
    let hic = if t < 0.25 { (t / 0.25 * std::f32::consts::PI).sin() * 9.0 } else { 0.0 };
    let cy = y + TS - 18.0 - hic;
    shadow(cx, y, 10.0);
    let body = Color::from_rgba(120, 200, 110, 255);
    draw_circle(cx - 5.0, y + TS - 5.0, 2.8, ink());
    draw_circle(cx + 5.0, y + TS - 5.0, 2.8, ink());
    draw_circle(cx, cy, 13.0, ink());
    draw_circle(cx, cy, 11.5, body);
    // Surprised eyes and a little "o" mouth.
    draw_circle(cx - 4.0, cy - 3.0, 3.0, WHITE);
    draw_circle(cx + 4.0, cy - 3.0, 3.0, WHITE);
    draw_circle(cx - 4.0, cy - 3.0, 1.4, ink());
    draw_circle(cx + 4.0, cy - 3.0, 1.4, ink());
    draw_circle_lines(cx, cy + 5.0, 2.2, 1.2, ink());
    // Bubbles rising off each hiccup.
    for i in 0..3 {
        let u = ((time * 0.9 + i as f32 * 0.33) % 1.0) as f32;
        draw_circle_lines(cx + 8.0 + i as f32 * 3.0, cy - 12.0 - u * 18.0, 1.5 + u * 2.0, 1.0, Color::new(0.8, 0.95, 1.0, 1.0 - u));
    }
    pop_word(cx, cy - 14.0, time, "HIC!", 1.7, 0.45, 0.0);
}

/// Fridgey: a tall fridge robot who tells cold jokes. His door keeps swinging
/// open and a chilly puff comes out.
pub fn draw_fridge_bot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    shadow(cx, y, 13.0);
    let white = Color::from_rgba(235, 245, 250, 255);
    let (left, top, w, h) = (cx - 12.0, y + 4.0, 24.0, TS - 8.0);
    draw_rectangle(left - 1.5, top - 1.5, w + 3.0, h + 3.0, ink());
    draw_rectangle(left, top, w, h, white);
    // Freezer line and handle.
    draw_line(left, top + 14.0, left + w, top + 14.0, 1.5, ink());
    draw_rectangle(left + w - 5.0, top + 18.0, 2.0, 10.0, Color::from_rgba(150, 160, 170, 255));
    // Face on the freezer door.
    draw_circle(cx - 4.0, top + 7.0, 1.8, ink());
    draw_circle(cx + 4.0, top + 7.0, 1.8, ink());
    draw_line(cx - 3.0, top + 11.0, cx + 3.0, top + 11.0, 1.2, ink());
    // The door swings open now and then: a peek inside and a cold puff.
    let open = ((time * 0.8).sin() * 0.5 + 0.5).powf(6.0);
    if open > 0.05 {
        let dw = w * (1.0 - open * 0.7);
        draw_rectangle(left, top + 15.0, w, h - 15.0, Color::from_rgba(180, 230, 255, 255));
        draw_rectangle(left + 3.0, top + 22.0, 7.0, 6.0, Color::from_rgba(240, 120, 60, 255)); // cheese
        draw_rectangle(left, top + 15.0, dw, h - 15.0, white);
        draw_rectangle_lines(left, top + 15.0, dw, h - 15.0, 1.0, ink());
        for i in 0..3 {
            let u = (time * 1.5 + i as f32 * 0.3) % 1.0;
            draw_circle(left + w + 4.0 + u * 10.0, top + 24.0 - u * 6.0 + i as f32 * 3.0, 2.5 + u * 3.0, Color::new(0.85, 0.95, 1.0, 0.6 * (1.0 - u)));
        }
    }
    pop_word(cx, top, time, "brrr", 4.0, 0.8, 1.5);
}

/// Bubbles: a washing-machine robot, forever mid-spin. Everything she washes
/// comes out pink, and the porthole shows the socks going round.
pub fn draw_wash_bot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    let wobble = (time * 18.0).sin() * 1.2;
    shadow(cx, y, 13.0);
    let (left, top, w, h) = (cx - 13.0 + wobble, y + 10.0, 26.0, TS - 14.0);
    draw_rectangle(left - 1.5, top - 1.5, w + 3.0, h + 3.0, ink());
    draw_rectangle(left, top, w, h, Color::from_rgba(250, 250, 255, 255));
    draw_rectangle(left, top, w, 7.0, Color::from_rgba(200, 210, 225, 255));
    draw_circle(left + 5.0, top + 3.5, 1.8, Color::from_rgba(240, 80, 120, 255));
    draw_circle(left + 10.0, top + 3.5, 1.8, Color::from_rgba(80, 200, 120, 255));
    // Porthole with pink laundry tumbling round.
    let (px, py) = (cx + wobble, top + 18.0);
    draw_circle(px, py, 9.0, ink());
    draw_circle(px, py, 7.5, Color::from_rgba(150, 210, 240, 255));
    for i in 0..3 {
        let a = time * 6.0 + i as f32 * 2.1;
        draw_circle(px + a.cos() * 4.0, py + a.sin() * 4.0, 2.2, Color::from_rgba(245, 120, 180, 255));
    }
    // Eyes above the porthole, a bit dizzy.
    let d = (time * 3.0).sin() * 1.0;
    draw_circle(cx - 5.0 + wobble, top + 9.5, 1.5, ink());
    draw_circle(cx + 5.0 + wobble + d, top + 9.5, 1.5, ink());
    // Suds.
    for i in 0..4 {
        let u = (time * 0.7 + i as f32 * 0.25) % 1.0;
        draw_circle(left + 4.0 + i as f32 * 6.0, top - 2.0 - u * 12.0, 2.0 + u * 2.0, Color::new(1.0, 1.0, 1.0, 0.8 * (1.0 - u)));
    }
}

/// Ding-Dong: a doorbell robot. He rings himself whenever he's excited, which
/// is all the time.
pub fn draw_bell_bot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    shadow(cx, y, 10.0);
    let brass = Color::from_rgba(240, 190, 60, 255);
    let ring = (time % 2.0) < 0.5;
    let shake = if ring { (time * 50.0).sin() * 2.5 } else { 0.0 };
    // Little stand and feet.
    draw_line(cx - 6.0, y + 34.0, cx - 7.0, y + TS - 4.0, 3.0, ink());
    draw_line(cx + 6.0, y + 34.0, cx + 7.0, y + TS - 4.0, 3.0, ink());
    // The bell body.
    let (bx, by) = (cx + shake, y + 22.0);
    draw_triangle(vec2(bx - 14.0, by + 12.0), vec2(bx + 14.0, by + 12.0), vec2(bx, by - 14.0), ink());
    draw_circle(bx, by - 4.0, 10.0, ink());
    draw_circle(bx, by - 4.0, 8.8, brass);
    draw_triangle(vec2(bx - 12.5, by + 11.0), vec2(bx + 12.5, by + 11.0), vec2(bx, by - 10.0), brass);
    draw_rectangle(bx - 14.0, by + 10.0, 28.0, 3.0, ink());
    draw_circle(bx, by + 15.0, 2.5, ink()); // clapper
    // Big happy face.
    draw_circle(bx - 3.5, by - 3.0, 1.6, ink());
    draw_circle(bx + 3.5, by - 3.0, 1.6, ink());
    draw_line(bx - 3.0, by + 3.0, bx + 3.0, by + 3.0, 1.2, ink());
    if ring {
        for s in [-1.0, 1.0] {
            draw_line(bx + s * 16.0, by - 8.0, bx + s * 21.0, by - 12.0, 1.5, ink());
            draw_line(bx + s * 17.0, by - 2.0, bx + s * 23.0, by - 2.0, 1.5, ink());
        }
    }
    pop_word(cx, y + 6.0, time, "DING!", 2.0, 0.5, 0.0);
}

/// Mopsy: a mop robot who is terrified of puddles. She tiptoes and shakes her
/// mop-head hair every time she spots one.
pub fn draw_mop_bot(x: f32, y: f32, time: f32) {
    let cx = x + TS / 2.0;
    shadow(cx, y, 9.0);
    let tiptoe = ((time * 5.0).sin()).abs() * 2.0;
    let handle = Color::from_rgba(170, 120, 70, 255);
    // A tiny puddle she is scared of.
    draw_ellipse(cx + 14.0, y + TS - 4.0, 6.0, 2.5, 0.0, Color::from_rgba(120, 180, 240, 200));
    // Stick body.
    draw_line(cx, y + 18.0, cx, y + TS - 12.0 - tiptoe, 3.0, handle);
    // Bucket feet.
    draw_rectangle(cx - 7.0, y + TS - 12.0 - tiptoe, 14.0, 8.0, Color::from_rgba(230, 70, 70, 255));
    draw_rectangle_lines(cx - 7.0, y + TS - 12.0 - tiptoe, 14.0, 8.0, 1.0, ink());
    // Arms up, eek.
    draw_line(cx, y + 24.0, cx - 9.0, y + 16.0, 2.0, handle);
    draw_line(cx, y + 24.0, cx + 9.0, y + 16.0, 2.0, handle);
    // Mop-head hair, shaking.
    let shake = (time * 14.0).sin() * 1.5;
    for i in 0..7 {
        let a = -1.2 + i as f32 * 0.4;
        let (hx, hy) = (cx + shake + a.sin() * 9.0, y + 10.0 - a.cos() * 6.0);
        draw_line(cx + shake, y + 12.0, hx, hy, 2.5, Color::from_rgba(240, 235, 220, 255));
    }
    draw_circle(cx + shake, y + 13.0, 6.0, Color::from_rgba(245, 240, 228, 255));
    draw_circle(cx - 2.0 + shake, y + 13.0, 1.4, ink());
    draw_circle(cx + 2.0 + shake, y + 13.0, 1.4, ink());
    pop_word(cx, y + 2.0, time, "a PUDDLE!", 3.5, 0.8, 2.0);
}
