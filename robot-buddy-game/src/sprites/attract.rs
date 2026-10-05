//! Attract beacons: the "there's a game here!" marker over a minigame host
//! (Shelly's Pearl Hop, Inkwell's dive). A big icon — a pearl or a dive
//! arrow — bouncing over their head in a glowing badge, a trail of bubbles
//! rising off it, and a slow ring of bubbles round the host. No words: the
//! kid can't read.
//!
//! Readable across the room but not strobing: one bouncy motion, one soft
//! glow pulse, and a dark rim on everything so it holds up on the reef's
//! bright teal AND on the Dogfish House's dark glitch screen.

use crate::prelude::*;

const TS: f32 = 48.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beacon {
    /// Shelly's Pearl Hop.
    Pearl,
    /// Inkwell's dive.
    Dive,
}

const RIM: Color = Color::new(0.05, 0.10, 0.22, 0.9);
const GOLD: Color = Color::new(1.0, 0.82, 0.30, 1.0);

/// Draw the beacon for an NPC whose sprite sits at tile-space (x, y).
pub fn draw_beacon(x: f32, y: f32, time: f32, beacon: Beacon) {
    let cx = x + TS / 2.0;
    // Offset each kind's phase so two hosts on screen don't pulse in lockstep.
    let t = time + match beacon {
        Beacon::Pearl => 0.0,
        Beacon::Dive => 1.7,
    };

    // Bubble ring round the host's middle, drifting slowly round.
    let (rx, ry) = (TS * 0.55, TS * 0.17);
    let ring_y = y + TS * 0.62;
    for i in 0..6 {
        let a = t * 0.9 + i as f32 * std::f32::consts::TAU / 6.0;
        let (bx, by) = (cx + a.cos() * rx, ring_y + a.sin() * ry);
        let alpha = if a.sin() > 0.0 { 0.9 } else { 0.45 };
        let r = 3.0 + (t * 2.0 + i as f32).sin() * 0.5;
        draw_circle(bx, by, r + 1.2, Color::new(RIM.r, RIM.g, RIM.b, 0.35 * alpha));
        draw_circle(bx, by, r, Color::new(0.85, 0.97, 1.0, alpha));
    }

    // The badge: a hop up and a squashy landing, twice a second-and-a-bit,
    // high over the head.
    let hop = (t * 2.6).sin().abs();
    let land = 1.0 - hop; // 1 at the bottom of the bounce
    let squash = 1.0 + 0.12 * land * land * land;
    let (bx, by) = (cx, y - 20.0 - hop * 10.0);
    let br = 16.0;
    let pulse = (t * 2.0).sin() * 0.5 + 0.5;

    // Bubbles streaming up off the badge.
    for i in 0..4 {
        let u = (t * 0.7 + i as f32 * 0.25) % 1.0;
        let wob = (t * 3.0 + i as f32 * 1.7).sin() * 4.0;
        let (px, py) = (bx + wob + (i as f32 - 1.5) * 4.0, by - br - 4.0 - u * 34.0);
        let a = 0.85 * (1.0 - u);
        draw_circle_lines(px, py, 2.0 + u * 3.0, 1.5, Color::new(1.0, 1.0, 1.0, a));
        draw_circle_lines(px, py, 3.0 + u * 3.0, 1.0, Color::new(RIM.r, RIM.g, RIM.b, a * 0.4));
    }

    // Soft glow, then the rimmed gold disc (squashed on landing), then the icon.
    draw_circle(bx, by, br + 8.0 + pulse * 5.0, Color::new(1.0, 0.9, 0.5, 0.16 + 0.14 * pulse));
    draw_ellipse(bx, by, (br + 2.5) * squash, (br + 2.5) / squash, 0.0, RIM);
    draw_ellipse(bx, by, br * squash, br / squash, 0.0, GOLD);
    draw_ellipse(bx - br * 0.3, by - br * 0.35, br * 0.35, br * 0.18, -30.0, Color::new(1.0, 1.0, 1.0, 0.45));
    match beacon {
        Beacon::Pearl => {
            // A fat pearl with a glint that winks.
            let pr = br * 0.6;
            draw_circle(bx, by + 1.0, pr + 1.5, RIM);
            draw_circle(bx, by + 1.0, pr, Color::new(0.97, 0.98, 1.0, 1.0));
            draw_circle(bx - pr * 0.35, by + 1.0 - pr * 0.35, pr * 0.3 * (0.8 + 0.4 * pulse), WHITE);
        }
        Beacon::Dive => {
            // A chunky down arrow: "down you go".
            let w = br * 0.34;
            draw_rectangle(bx - w / 2.0, by - br * 0.6, w, br * 0.62, RIM);
            draw_triangle(
                vec2(bx - br * 0.6, by + br * 0.02),
                vec2(bx + br * 0.6, by + br * 0.02),
                vec2(bx, by + br * 0.68),
                RIM,
            );
        }
    }
    // A little tail pointing down at the host, so the badge is clearly theirs.
    let tail_y = by + br / squash + 1.0;
    draw_triangle(vec2(bx - 5.0, tail_y), vec2(bx + 5.0, tail_y), vec2(bx, tail_y + 7.0), RIM);
}
