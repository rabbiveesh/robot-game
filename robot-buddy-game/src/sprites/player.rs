use crate::prelude::*;
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

/// Outfit tints the kid can pick from after buying Color Change in Bolt's
/// shop. Ids are persisted in saves — don't rename them. First entry is the
/// default for saves that predate the picker.
pub const OUTFIT_COLORS: &[(&str, Color)] = &[
    ("purple", Color::new(0.55, 0.35, 0.95, 1.0)),
    ("red", Color::new(0.95, 0.25, 0.25, 1.0)),
    ("blue", Color::new(0.25, 0.45, 0.95, 1.0)),
    ("green", Color::new(0.20, 0.80, 0.35, 1.0)),
    ("orange", Color::new(1.00, 0.55, 0.10, 1.0)),
    ("pink", Color::new(1.00, 0.40, 0.75, 1.0)),
    ("teal", Color::new(0.10, 0.80, 0.80, 1.0)),
    ("gold", Color::new(1.00, 0.80, 0.20, 1.0)),
];

pub fn outfit_color(id: &str) -> Color {
    OUTFIT_COLORS
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, c)| *c)
        .unwrap_or(OUTFIT_COLORS[0].1)
}

/// Draw cosmetics the kid has bought (from Bolt's shop) over the player avatar,
/// for either gender. `owned` holds shop item ids; `color_choice` is the
/// outfit color id picked for Color Change. Aligns to the same center /
/// bob as the player sprites, so call it right after drawing the player.
pub fn draw_player_cosmetics(
    x: f32,
    y: f32,
    frame: u32,
    owned: &std::collections::HashSet<String>,
    color_choice: &str,
) {
    if owned.is_empty() {
        return;
    }
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0;
    let bob = if frame % 2 == 1 { -2.0 } else { 0.0 };
    let time = get_time() as f32;

    // Color change: recolor the kid's clothes. A bold tint over the torso plus
    // a solid sash so even subtle hues read clearly and swapping colors is
    // unmistakable (the old faint 32% wash was easy to miss).
    if owned.contains("color_change") {
        let base = outfit_color(color_choice);
        let mut tint = base;
        tint.a = 0.6;
        draw_rectangle(cx - 10.0, cy - 3.0 + bob, 20.0, 17.0, tint);
        // Solid colored sash across the chest — unambiguous at a glance.
        draw_rectangle(cx - 10.0, cy + 5.0 + bob, 20.0, 4.0, base);
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

/// The rocketship the kid pilots on the orbital hub. Faces `dir`; a little
/// flame flickers from the engine. Drawn in place of the on-foot kid whenever
/// the player is on the space hub.
pub fn draw_rocket(x: f32, y: f32, dir: Dir, frame: u32, time: f32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0;
    let body = Color::from_rgba(236, 240, 245, 255);
    let trim = Color::from_rgba(229, 57, 53, 255);
    let window = Color::from_rgba(129, 212, 250, 255);
    let flame_flicker = if frame % 2 == 0 { 0.0 } else { 2.0 };

    // Soft thruster glow under the ship.
    draw_ellipse(cx, y + TS - 5.0, 10.0, 4.0, 0.0, Color::new(0.4, 0.7, 1.0, 0.18));

    // Orient the nose toward dir by drawing a rotated-ish silhouette. We keep it
    // simple: nose triangle + body capsule + fins, flipped per axis.
    let (nose, tail, side) = match dir {
        Dir::Up    => (vec2(cx, cy - 16.0), vec2(cx, cy + 14.0), 8.0),
        Dir::Down  => (vec2(cx, cy + 16.0), vec2(cx, cy - 14.0), 8.0),
        Dir::Left  => (vec2(cx - 16.0, cy), vec2(cx + 14.0, cy), 8.0),
        Dir::Right => (vec2(cx + 16.0, cy), vec2(cx - 14.0, cy), 8.0),
    };

    // Engine flame out the tail.
    let flame = Color::from_rgba(255, 167, 38, 255);
    let flame_tip = match dir {
        Dir::Up    => vec2(cx, tail.y + 8.0 + flame_flicker),
        Dir::Down  => vec2(cx, tail.y - 8.0 - flame_flicker),
        Dir::Left  => vec2(tail.x + 8.0 + flame_flicker, cy),
        Dir::Right => vec2(tail.x - 8.0 - flame_flicker, cy),
    };
    let (f1, f2) = match dir {
        Dir::Up | Dir::Down => (vec2(tail.x - 4.0, tail.y), vec2(tail.x + 4.0, tail.y)),
        Dir::Left | Dir::Right => (vec2(tail.x, tail.y - 4.0), vec2(tail.x, tail.y + 4.0)),
    };
    draw_triangle(f1, f2, flame_tip, flame);

    // Body capsule.
    draw_ellipse(cx, cy, side, side + 4.0, 0.0, body);
    // Nose cone.
    let (n1, n2) = match dir {
        Dir::Up | Dir::Down => (vec2(cx - side, cy), vec2(cx + side, cy)),
        Dir::Left | Dir::Right => (vec2(cx, cy - side), vec2(cx, cy + side)),
    };
    draw_triangle(n1, n2, nose, trim);

    // Window with the kid inside.
    draw_circle(cx, cy - 1.0, 4.5, window);
    draw_circle(cx, cy - 1.0, 4.5, Color::new(0.0, 0.0, 0.0, 0.0));
    draw_circle_lines(cx, cy - 1.0, 4.5, 1.5, trim);

    // Fins.
    draw_triangle(
        vec2(cx - side, cy + 2.0), vec2(cx - side - 4.0, cy + 8.0), vec2(cx - side, cy + 8.0), trim);
    draw_triangle(
        vec2(cx + side, cy + 2.0), vec2(cx + side + 4.0, cy + 8.0), vec2(cx + side, cy + 8.0), trim);

    // A twinkle off the hull.
    let tw = (time * 4.0).sin() * 0.5 + 0.5;
    draw_circle(cx + 3.0, cy - 5.0, 1.2, Color::new(1.0, 1.0, 1.0, tw));
}

/// A clear space-helmet bubble drawn over the on-foot kid on planet surfaces.
/// Rides on top of the normal player sprite so the spacesuit reads instantly.
pub fn draw_spacesuit_overlay(x: f32, y: f32, frame: u32) {
    let cx = x + TS / 2.0;
    let cy = y + TS / 2.0 + 4.0;
    let bob = if frame % 2 == 1 { -2.0 } else { 0.0 };
    let hy = cy - 9.0 + bob;
    // Helmet bubble around the head.
    draw_circle_lines(cx, hy, 12.5, 2.0, Color::from_rgba(200, 230, 255, 230));
    draw_circle(cx, hy, 12.5, Color::new(0.6, 0.85, 1.0, 0.10));
    // Glassy highlight.
    draw_circle(cx - 5.0, hy - 5.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.6));
    // Air collar.
    draw_rectangle(cx - 9.0, hy + 9.0, 18.0, 4.0, Color::from_rgba(224, 224, 224, 255));
}
