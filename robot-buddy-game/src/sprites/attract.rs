//! Attract beacons: the "there's a game here!" marker over a minigame host
//! (Shelly's Pearl Hop, Inkwell's dive). A lazy ring of bubbles circling the
//! host and a small pulsing badge over their head with the game's icon — a
//! pearl or a dive arrow. No words: the kid can't read.
//!
//! Kept quiet on purpose: slow motion, small, and a dark rim on everything so
//! it reads on the reef's bright teal AND on the Dogfish House's dark glitch
//! screen without shouting.

use crate::prelude::*;

const TS: f32 = 48.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beacon {
    /// Shelly's Pearl Hop.
    Pearl,
    /// Inkwell's dive.
    Dive,
}

const RIM: Color = Color::new(0.05, 0.10, 0.22, 0.85);
const GOLD: Color = Color::new(1.0, 0.82, 0.30, 1.0);

/// Draw the beacon for an NPC whose sprite sits at tile-space (x, y).
pub fn draw_beacon(x: f32, y: f32, time: f32, beacon: Beacon) {
    let cx = x + TS / 2.0;
    // Offset each kind's phase so two hosts on screen don't pulse in lockstep.
    let phase = match beacon {
        Beacon::Pearl => 0.0,
        Beacon::Dive => 1.7,
    };
    let t = time + phase;

    // Bubble ring: six bubbles on a flat ellipse round the host's middle,
    // drifting slowly round. Back half fainter so it reads as a ring.
    let (rx, ry) = (TS * 0.52, TS * 0.16);
    let ring_y = y + TS * 0.62;
    for i in 0..6 {
        let a = t * 0.9 + i as f32 * std::f32::consts::TAU / 6.0;
        let bx = cx + a.cos() * rx;
        let by = ring_y + a.sin() * ry;
        let front = a.sin() > 0.0;
        let alpha = if front { 0.9 } else { 0.45 };
        let r = 2.6 + (t * 2.0 + i as f32).sin() * 0.5;
        draw_circle(bx, by, r + 1.2, Color::new(RIM.r, RIM.g, RIM.b, 0.35 * alpha));
        draw_circle(bx, by, r, Color::new(0.85, 0.97, 1.0, alpha));
        draw_circle(bx - r * 0.35, by - r * 0.35, r * 0.3, Color::new(1.0, 1.0, 1.0, alpha));
    }

    // Badge: bobs gently and breathes in size; a soft halo pulses behind it.
    let pulse = (t * 3.0).sin() * 0.5 + 0.5;
    let bob = (t * 2.0).sin() * 2.5;
    let (bx, by) = (cx, y - 8.0 + bob);
    let br = 10.0 + pulse * 1.2;
    draw_circle(bx, by, br + 5.0 + pulse * 3.0, Color::new(1.0, 0.9, 0.5, 0.18 + 0.12 * pulse));
    draw_circle(bx, by, br + 2.0, RIM);
    draw_circle(bx, by, br, GOLD);
    match beacon {
        Beacon::Pearl => {
            // A pearl with its highlight.
            let pr = br * 0.55;
            draw_circle(bx, by, pr + 1.0, RIM);
            draw_circle(bx, by, pr, Color::new(0.97, 0.98, 1.0, 1.0));
            draw_circle(bx - pr * 0.35, by - pr * 0.35, pr * 0.32, WHITE);
        }
        Beacon::Dive => {
            // A chunky down arrow: "down you go".
            let w = br * 0.28;
            draw_rectangle(bx - w / 2.0, by - br * 0.55, w, br * 0.6, RIM);
            draw_triangle(
                vec2(bx - br * 0.55, by + br * 0.02),
                vec2(bx + br * 0.55, by + br * 0.02),
                vec2(bx, by + br * 0.62),
                RIM,
            );
        }
    }
    // Tail pointing down at the host, so the badge clearly belongs to them.
    draw_triangle(vec2(bx - 4.0, by + br + 1.0), vec2(bx + 4.0, by + br + 1.0), vec2(bx, by + br + 6.0), RIM);
}
