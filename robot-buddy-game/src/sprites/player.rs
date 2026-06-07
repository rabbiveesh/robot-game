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

/// Draw cosmetics the kid has bought (from Bolt's shop) over the player avatar,
/// for either gender. `owned` holds shop item ids. Aligns to the same center /
/// bob as the player sprites, so call it right after drawing the player.
pub fn draw_player_cosmetics(
    x: f32,
    y: f32,
    frame: u32,
    owned: &std::collections::HashSet<String>,
) {
    if owned.is_empty() {
        return;
    }
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0;
    let bob = if frame % 2 == 1 { -2.0 } else { 0.0 };
    let time = get_time() as f32;

    // Color change: a translucent tint over the kid's clothes.
    if owned.contains("color_change") {
        draw_rectangle(cx - 9.0, cy - 2.0 + bob, 18.0, 14.0, Color::new(0.55, 0.35, 0.95, 0.32));
    }
    // Jet boots: flames under the feet.
    if owned.contains("jet_boots") {
        let flame = Color::from_rgba(255, 143, 0, 255);
        let flick = (time * 20.0).sin() * 1.5;
        draw_triangle(
            vec2(cx - 7.0, y + TS - 5.0), vec2(cx - 1.0, y + TS - 5.0),
            vec2(cx - 4.0, y + TS + 4.0 + flick), flame,
        );
        draw_triangle(
            vec2(cx + 1.0, y + TS - 5.0), vec2(cx + 7.0, y + TS - 5.0),
            vec2(cx + 4.0, y + TS + 4.0 - flick), flame,
        );
    }
    // Bow tie at the collar.
    if owned.contains("bow_tie") {
        let c = Color::from_rgba(216, 27, 96, 255);
        draw_triangle(
            vec2(cx, cy - 1.0 + bob), vec2(cx - 7.0, cy - 3.0 + bob),
            vec2(cx - 7.0, cy + 1.0 + bob), c,
        );
        draw_triangle(
            vec2(cx, cy - 1.0 + bob), vec2(cx + 7.0, cy - 3.0 + bob),
            vec2(cx + 7.0, cy + 1.0 + bob), c,
        );
        draw_rectangle(cx - 1.5, cy - 2.5 + bob, 3.0, 3.0, Color::from_rgba(136, 14, 79, 255));
    }
    // Hat: a red cap with a dark brim on top of the head.
    if owned.contains("hat") {
        draw_rectangle(cx - 10.0, cy - 19.0 + bob, 20.0, 3.0, Color::from_rgba(33, 33, 40, 255));
        draw_rectangle(cx - 7.0, cy - 27.0 + bob, 14.0, 8.0, Color::from_rgba(229, 57, 53, 255));
    }
    // Sparkle trail: a few twinkling motes orbiting the kid.
    if owned.contains("sparkle_trail") {
        let gold = Color::from_rgba(255, 213, 79, 255);
        for i in 0..4 {
            let a = time * 2.0 + i as f32 * 1.7;
            let sx = cx + a.cos() * 17.0;
            let sy = cy - 6.0 + (a * 1.3).sin() * 14.0 + bob;
            let r = 1.5 + ((time * 6.0 + i as f32).sin() * 0.5 + 0.5) * 1.5;
            draw_circle(sx, sy, r, gold);
        }
    }
}
