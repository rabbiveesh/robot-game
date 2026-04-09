use macroquad::prelude::*;
use super::Dir;

const TS: f32 = 48.0;

pub fn draw_player_boy(x: f32, y: f32, dir: Dir, frame: u32, _time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0;
    let bob = if frame % 2 == 1 { -2.0 } else { 0.0 };

    // Shadow
    draw_ellipse(cx, y + TS - 4.0, 12.0, 5.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Body (blue shirt)
    draw_rectangle(cx - 8.0, cy - 2.0 + bob, 16.0, 14.0, Color::from_rgba(66, 165, 245, 255));

    // Legs
    let leg_color = Color::from_rgba(93, 64, 55, 255);
    let leg_offset = if frame % 2 == 1 { 3.0 } else { 0.0 };
    draw_rectangle(cx - 6.0, cy + 12.0 + bob, 5.0, 8.0 - leg_offset, leg_color);
    draw_rectangle(cx + 1.0, cy + 12.0 + bob, 5.0, 8.0 - (if frame % 2 == 0 { 3.0 } else { 0.0 }), leg_color);

    // Head
    draw_circle(cx, cy - 8.0 + bob, 10.0, Color::from_rgba(255, 204, 128, 255));

    // Hair
    draw_circle(cx, cy - 12.0 + bob, 10.0, Color::from_rgba(93, 64, 55, 255));
    // Cover bottom half of hair circle with head color
    draw_rectangle(cx - 11.0, cy - 8.0 + bob, 22.0, 10.0, Color::from_rgba(255, 204, 128, 255));

    // Eyes (direction-dependent)
    let eye_color = Color::from_rgba(51, 51, 51, 255);
    match dir {
        Dir::Left => {
            draw_rectangle(cx - 6.0, cy - 10.0 + bob, 3.0, 3.0, eye_color);
            draw_rectangle(cx - 1.0, cy - 10.0 + bob, 3.0, 3.0, eye_color);
        }
        Dir::Right => {
            draw_rectangle(cx - 1.0, cy - 10.0 + bob, 3.0, 3.0, eye_color);
            draw_rectangle(cx + 4.0, cy - 10.0 + bob, 3.0, 3.0, eye_color);
        }
        Dir::Up => {} // facing away
        Dir::Down => {
            draw_rectangle(cx - 5.0, cy - 10.0 + bob, 3.0, 3.0, eye_color);
            draw_rectangle(cx + 2.0, cy - 10.0 + bob, 3.0, 3.0, eye_color);
            // Smile
            draw_line(cx - 3.0, cy - 4.0 + bob, cx + 3.0, cy - 4.0 + bob, 1.5, eye_color);
        }
    }
}
