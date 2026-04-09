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
