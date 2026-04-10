use macroquad::prelude::*;
use super::Dir;

const TS: f32 = 48.0;

pub fn draw_player_girl(x: f32, y: f32, dir: Dir, frame: u32, _time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0;
    let bob = if frame % 2 == 1 { -2.0 } else { 0.0 };

    // Shadow
    draw_ellipse(cx, y + TS - 4.0, 12.0, 5.0, 0.0, Color::from_rgba(0, 0, 0, 40));

    // Dress (pink trapezoid — wider at bottom)
    let dress_color = Color::from_rgba(244, 143, 177, 255); // #F48FB1
    let stripe_color = Color::from_rgba(236, 64, 122, 255); // #EC407A
    draw_rectangle(cx - 8.0, cy - 2.0 + bob, 16.0, 7.0, dress_color);
    draw_rectangle(cx - 9.0, cy + 5.0 + bob, 18.0, 3.0, stripe_color);
    draw_rectangle(cx - 10.0, cy + 8.0 + bob, 20.0, 4.0, dress_color);

    // Legs (skin tone)
    let leg_color = Color::from_rgba(255, 204, 128, 255);
    let leg_offset = if frame % 2 == 1 { 3.0 } else { 0.0 };
    draw_rectangle(cx - 6.0, cy + 12.0 + bob, 5.0, 8.0 - leg_offset, leg_color);
    draw_rectangle(cx + 1.0, cy + 12.0 + bob, 5.0, 8.0 - (if frame % 2 == 0 { 3.0 } else { 0.0 }), leg_color);

    // Head
    let skin = Color::from_rgba(255, 204, 128, 255);
    draw_circle(cx, cy - 8.0 + bob, 10.0, skin);

    // Hair (brown, longer)
    let hair = Color::from_rgba(93, 64, 55, 255);
    draw_circle(cx, cy - 12.0 + bob, 10.0, hair);
    draw_rectangle(cx - 11.0, cy - 8.0 + bob, 22.0, 10.0, skin);
    // Pigtails
    draw_rectangle(cx - 12.0, cy - 14.0 + bob, 4.0, 14.0, hair);
    draw_circle(cx - 10.0, cy + bob, 2.5, hair);
    draw_rectangle(cx + 8.0, cy - 14.0 + bob, 4.0, 14.0, hair);
    draw_circle(cx + 10.0, cy + bob, 2.5, hair);

    // Hair bow (red)
    let bow = Color::from_rgba(255, 82, 82, 255);
    draw_triangle(
        vec2(cx - 4.0, cy - 18.0 + bob),
        vec2(cx, cy - 22.0 + bob),
        vec2(cx + 4.0, cy - 18.0 + bob),
        bow,
    );

    // Eyes (slightly taller + eyelashes)
    let eye_color = Color::from_rgba(51, 51, 51, 255);
    match dir {
        Dir::Left => {
            draw_rectangle(cx - 6.0, cy - 11.0 + bob, 3.0, 4.0, eye_color);
            draw_rectangle(cx - 1.0, cy - 11.0 + bob, 3.0, 4.0, eye_color);
            draw_line(cx - 6.0, cy - 12.0 + bob, cx - 4.0, cy - 13.0 + bob, 1.0, eye_color);
            draw_line(cx - 1.0, cy - 12.0 + bob, cx + 1.0, cy - 13.0 + bob, 1.0, eye_color);
        }
        Dir::Right => {
            draw_rectangle(cx - 1.0, cy - 11.0 + bob, 3.0, 4.0, eye_color);
            draw_rectangle(cx + 4.0, cy - 11.0 + bob, 3.0, 4.0, eye_color);
            draw_line(cx, cy - 12.0 + bob, cx + 2.0, cy - 13.0 + bob, 1.0, eye_color);
            draw_line(cx + 5.0, cy - 12.0 + bob, cx + 7.0, cy - 13.0 + bob, 1.0, eye_color);
        }
        Dir::Up => {}
        Dir::Down => {
            draw_rectangle(cx - 5.0, cy - 11.0 + bob, 3.0, 4.0, eye_color);
            draw_rectangle(cx + 2.0, cy - 11.0 + bob, 3.0, 4.0, eye_color);
            draw_line(cx - 5.0, cy - 12.0 + bob, cx - 3.0, cy - 13.0 + bob, 1.0, eye_color);
            draw_line(cx + 2.0, cy - 12.0 + bob, cx + 4.0, cy - 13.0 + bob, 1.0, eye_color);
            // Pink smile
            draw_line(cx - 3.0, cy - 4.0 + bob, cx + 3.0, cy - 4.0 + bob, 1.5,
                Color::from_rgba(233, 30, 99, 255));
        }
    }
}

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
