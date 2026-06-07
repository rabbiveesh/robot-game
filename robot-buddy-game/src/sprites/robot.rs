use macroquad::prelude::*;
use super::Dir;

const TS: f32 = 48.0;

pub fn draw_robot(x: f32, y: f32, dir: Dir, frame: u32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 2.0;
    let bob = (time * 3.0).sin() * 2.0;
    let walk_shift = if frame % 2 == 1 { 1.0 } else { -1.0 };

    // Shadow
    draw_ellipse(cx, y + TS - 3.0, 11.0, 4.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Antenna line
    draw_line(cx, cy - 16.0 + bob, cx, cy - 26.0 + bob, 2.0,
        Color::from_rgba(120, 144, 156, 255));
    // Antenna ball
    let antenna_bob = (time * 4.0).sin() * 2.0;
    draw_circle(cx, cy - 28.0 + bob + antenna_bob, 4.0, Color::from_rgba(255, 82, 82, 255));

    // Body
    let body_color = Color::from_rgba(176, 190, 197, 255);
    let body_x = cx - 12.0;
    let body_y = cy - 10.0 + bob;
    draw_rectangle(body_x, body_y, 24.0, 22.0, body_color);
    draw_rectangle_lines(body_x, body_y, 24.0, 22.0, 2.0, Color::from_rgba(120, 144, 156, 255));

    // Head
    let head_color = Color::from_rgba(207, 216, 220, 255);
    draw_rectangle(cx - 10.0, cy - 20.0 + bob, 20.0, 14.0, head_color);
    draw_rectangle_lines(cx - 10.0, cy - 20.0 + bob, 20.0, 14.0, 1.5,
        Color::from_rgba(120, 144, 156, 255));

    // Eyes
    let blink = (time * 5.0).sin() > 0.95;
    let eye_color = Color::from_rgba(0, 230, 118, 255);
    if blink {
        draw_rectangle(cx - 7.0, cy - 16.0 + bob, 6.0, 2.0, eye_color);
        draw_rectangle(cx + 1.0, cy - 16.0 + bob, 6.0, 2.0, eye_color);
    } else {
        draw_rectangle(cx - 7.0, cy - 18.0 + bob, 6.0, 6.0, eye_color);
        draw_rectangle(cx + 1.0, cy - 18.0 + bob, 6.0, 6.0, eye_color);
        // Pupils
        let pupil_color = Color::from_rgba(27, 94, 32, 255);
        let px = match dir { Dir::Left => -1.0, Dir::Right => 1.0, _ => 0.0 };
        let py = match dir { Dir::Up => -1.0, Dir::Down => 1.0, _ => 0.0 };
        draw_rectangle(cx - 6.0 + px, cy - 17.0 + bob + py, 3.0, 3.0, pupil_color);
        draw_rectangle(cx + 2.0 + px, cy - 17.0 + bob + py, 3.0, 3.0, pupil_color);
    }

    // Smile
    draw_line(cx - 4.0, cy - 8.0 + bob, cx + 4.0, cy - 8.0 + bob, 1.5, eye_color);

    // Arms
    let arm_color = Color::from_rgba(144, 164, 174, 255);
    draw_rectangle(cx - 16.0, cy - 6.0 + bob + walk_shift, 5.0, 12.0, arm_color);
    draw_rectangle(cx + 11.0, cy - 6.0 + bob - walk_shift, 5.0, 12.0, arm_color);

    // Legs
    let leg_color = Color::from_rgba(120, 144, 156, 255);
    draw_rectangle(cx - 8.0, cy + 12.0 + bob, 6.0, 8.0 + walk_shift, leg_color);
    draw_rectangle(cx + 2.0, cy + 12.0 + bob, 6.0, 8.0 - walk_shift, leg_color);

    // Chest light
    let pulse = (time * 2.0).sin() * 0.3 + 0.7;
    draw_circle(cx, cy + bob, 3.0, Color::from_rgba(0, 230, 118, (pulse * 255.0) as u8));
}

/// Draw cosmetics the kid has bought for Sparky over the base robot. `owned`
/// holds shop item ids (hat, bow_tie, jet_boots, color_change, sparkle_trail).
/// Aligns to the same center/bob as `draw_robot`, so call it right after.
pub fn draw_robot_cosmetics(
    x: f32,
    y: f32,
    time: f32,
    owned: &std::collections::HashSet<String>,
) {
    if owned.is_empty() {
        return;
    }
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 2.0;
    let bob = (time * 3.0).sin() * 2.0;

    // Color change: a translucent tint over the head + body.
    if owned.contains("color_change") {
        let tint = Color::new(0.55, 0.35, 0.95, 0.32);
        draw_rectangle(cx - 12.0, cy - 10.0 + bob, 24.0, 22.0, tint);
        draw_rectangle(cx - 10.0, cy - 20.0 + bob, 20.0, 14.0, tint);
    }
    // Jet boots: little flames under the feet.
    if owned.contains("jet_boots") {
        let flame = Color::from_rgba(255, 143, 0, 255);
        let flick = (time * 20.0).sin() * 1.5;
        draw_triangle(
            Vec2::new(cx - 8.0, y + TS - 5.0), Vec2::new(cx - 2.0, y + TS - 5.0),
            Vec2::new(cx - 5.0, y + TS + 4.0 + flick), flame,
        );
        draw_triangle(
            Vec2::new(cx + 2.0, y + TS - 5.0), Vec2::new(cx + 8.0, y + TS - 5.0),
            Vec2::new(cx + 5.0, y + TS + 4.0 - flick), flame,
        );
    }
    // Bow tie at the neck.
    if owned.contains("bow_tie") {
        let c = Color::from_rgba(216, 27, 96, 255);
        draw_triangle(
            Vec2::new(cx, cy - 6.0 + bob), Vec2::new(cx - 9.0, cy - 9.0 + bob),
            Vec2::new(cx - 9.0, cy - 3.0 + bob), c,
        );
        draw_triangle(
            Vec2::new(cx, cy - 6.0 + bob), Vec2::new(cx + 9.0, cy - 9.0 + bob),
            Vec2::new(cx + 9.0, cy - 3.0 + bob), c,
        );
        draw_rectangle(cx - 2.0, cy - 8.0 + bob, 4.0, 4.0, Color::from_rgba(136, 14, 79, 255));
    }
    // Hat: a red cap with a dark brim on top of the head.
    if owned.contains("hat") {
        draw_rectangle(cx - 11.0, cy - 20.0 + bob, 22.0, 3.0, Color::from_rgba(33, 33, 40, 255));
        draw_rectangle(cx - 8.0, cy - 27.0 + bob, 16.0, 7.0, Color::from_rgba(229, 57, 53, 255));
    }
    // Sparkle trail: a few twinkling motes orbiting Sparky.
    if owned.contains("sparkle_trail") {
        let gold = Color::from_rgba(255, 213, 79, 255);
        for i in 0..4 {
            let a = time * 2.0 + i as f32 * 1.7;
            let sx = cx + a.cos() * 18.0;
            let sy = cy - 4.0 + (a * 1.3).sin() * 14.0 + bob;
            let r = 1.5 + ((time * 6.0 + i as f32).sin() * 0.5 + 0.5) * 1.5;
            draw_circle(sx, sy, r, gold);
        }
    }
}
