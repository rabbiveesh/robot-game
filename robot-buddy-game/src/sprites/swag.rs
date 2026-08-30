//! Shop swag, drawn on whoever happens to be wearing it.
//!
//! Every wearer publishes three anchor lines — where the crown of their head
//! is, where a collar sits, and where they meet the ground — measured off the
//! sprite that actually gets drawn. A hat rests ON the head line whether that
//! head belongs to a nine-year-old, a shark, or an octopus whose mantle happens
//! to reach as high as the kid's hair. The earlier "shift everything down a bit
//! for sea folk" fudge is what put a hat across Inkwell's face.
//!
//! Who wears what lives in `robot_buddy_domain::economy::wardrobe`; this module
//! only knows how to paint it.

use crate::prelude::*;
use super::Dir;
use super::player::outfit_color;
use std::collections::BTreeSet;

const TS: f32 = 48.0;

/// A wearer's anchor lines, as y-offsets from the top edge of their tile. These
/// are measured from the sprite functions themselves — if you change how a body
/// is drawn, change its fit too.
#[derive(Debug, Clone, Copy)]
pub struct SwagFit {
    /// Crown of the head: a hat brim rests here, a lantern floats above it.
    pub head: f32,
    /// Collar line: bow ties, necklaces and badges sit here.
    pub chest: f32,
    /// Where the body meets the sea floor: jet-boot flames start here.
    pub ground: f32,
    /// Body width relative to the kid's, for sizing each piece.
    pub scale: f32,
}

impl SwagFit {
    /// The kid, and anyone built like them — head circle centred 8px above the
    /// sprite's midline with a 10px radius, feet at the shadow.
    pub const KID: SwagFit = SwagFit { head: 9.0, chest: 27.0, ground: 43.0, scale: 1.0 };
    /// Sparky: a shorter body under a tall antenna, so the hat sits on the
    /// casing rather than on the aerial.
    pub const ROBOT: SwagFit = SwagFit { head: 10.0, chest: 28.0, ground: 44.0, scale: 0.95 };
    /// Inkwell. Her mantle reaches as high as the kid's head — this is the one
    /// the old blanket offset got most wrong.
    pub const OCTOPUS: SwagFit = SwagFit { head: 10.0, chest: 26.0, ground: 45.0, scale: 0.95 };
    /// Long swimmers held level in the water: shark and dolphin.
    pub const SWIMMER: SwagFit = SwagFit { head: 19.0, chest: 31.0, ground: 44.0, scale: 0.85 };
    /// Shelldon under his shell.
    pub const TURTLE: SwagFit = SwagFit { head: 20.0, chest: 32.0, ground: 44.0, scale: 0.85 };
    /// Glimmer and Wiggles — narrow bodies, low in the tile.
    pub const DEEP_FISH: SwagFit = SwagFit { head: 19.0, chest: 29.0, ground: 44.0, scale: 0.8 };
    /// Wobble's bell, which drifts a little higher than the rest.
    pub const JELLY: SwagFit = SwagFit { head: 15.0, chest: 26.0, ground: 42.0, scale: 0.7 };
    /// Pinchy, low and wide.
    pub const CRAB: SwagFit = SwagFit { head: 25.0, chest: 33.0, ground: 45.0, scale: 0.7 };
    /// Shelly, who is mostly shell.
    pub const CLAM: SwagFit = SwagFit { head: 26.0, chest: 32.0, ground: 45.0, scale: 0.7 };
    /// Hermie, wearing a shop on his back.
    pub const HERMIT: SwagFit = SwagFit { head: 14.0, chest: 31.0, ground: 45.0, scale: 0.75 };
}

/// Paint the *gear* a wearer owns — permanent perks from a shop counter, which
/// live in their own set because they can't be taken off or handed to a buddy.
/// Drawn after the body, same as swag, and hung off the same anchor lines.
///
/// Kept separate from [`draw_swag`] deliberately: mixing the two sets would
/// make it possible to gift away a perk.
pub fn draw_gear(
    x: f32,
    y: f32,
    dir: Dir,
    bob: f32,
    gear: &BTreeSet<String>,
    fit: SwagFit,
) {
    if gear.is_empty() {
        return;
    }
    let s = fit.scale;
    let cx = x + TS / 2.0;
    let time = get_time() as f32;
    let chest = y + fit.chest + bob;
    let ground = y + fit.ground;

    // Diving Net: a mesh pouch slung at the hip with a couple of pearls
    // glinting in it. Sits to the side rather than the back so it reads from
    // every facing — and swaps sides when the kid turns, so it never covers
    // their face.
    if gear.contains("diving_net") {
        let side = if dir == Dir::Right { -1.0 } else { 1.0 };
        let px = |dx: f32| cx + side * dx * s;
        let hip = (chest + ground) / 2.0 + 1.0 * s;
        let sway = (time * 2.0).sin() * 0.8 * s;

        let rope = Color::from_rgba(226, 214, 180, 235);
        let mesh = Color::new(0.90, 0.94, 0.86, 0.85);

        // Strap over the shoulder down to the pouch.
        draw_line(px(-3.0), chest - 2.0 * s, px(10.0), hip - 4.0 * s + sway, 1.6, rope);
        // Hoop mouth, then the bag hanging off it.
        let bag_x = px(11.0);
        let bag_y = hip + sway;
        draw_ellipse(bag_x, bag_y - 4.0 * s, 5.0 * s, 2.0 * s, 0.0, rope);
        draw_ellipse(bag_x, bag_y + 1.0 * s, 5.5 * s, 6.0 * s, 0.0,
            Color::new(0.85, 0.92, 0.85, 0.35));
        // Crosshatch so it reads as netting rather than a sack.
        for i in 0..3 {
            let o = (i as f32 - 1.0) * 3.2 * s;
            draw_line(bag_x + o, bag_y - 3.0 * s, bag_x + o * 0.4, bag_y + 6.0 * s, 0.9, mesh);
        }
        for i in 0..2 {
            let o = bag_y + i as f32 * 3.4 * s;
            draw_line(bag_x - 5.0 * s, o, bag_x + 5.0 * s, o, 0.9, mesh);
        }
        // The catch: two pearls, one catching the light.
        draw_circle(bag_x - 1.6 * s, bag_y + 1.6 * s, 1.9 * s,
            Color::from_rgba(245, 250, 252, 255));
        draw_circle(bag_x + 1.9 * s, bag_y + 3.4 * s, 1.5 * s,
            Color::from_rgba(226, 238, 245, 255));
        let glint = (time * 3.0).sin() * 0.5 + 0.5;
        draw_circle(bag_x - 2.2 * s, bag_y + 0.9 * s, 0.7 * s,
            Color::new(1.0, 1.0, 1.0, 0.4 + 0.5 * glint));
    }
}

/// Paint everything in `worn` over a sprite already drawn at (`x`, `y`).
///
/// `bob` is the wearer's current vertical wobble (the kid's walk cycle), so the
/// hat rides along instead of hovering. `color_choice` is the outfit tint the
/// Color Change piece uses — one shared palette pick, whoever's wearing it.
pub fn draw_swag(
    x: f32,
    y: f32,
    dir: Dir,
    bob: f32,
    worn: &BTreeSet<String>,
    color_choice: &str,
    fit: SwagFit,
) {
    if worn.is_empty() {
        return;
    }
    let s = fit.scale;
    let cx = x + TS / 2.0;
    let time = get_time() as f32;
    // Horizontal offsets are "kid units" from the body's centre line; vertical
    // ones hang off whichever anchor the piece belongs to.
    let hx = |dx: f32| cx + dx * s;
    let head = y + fit.head + bob;
    let chest = y + fit.chest + bob;
    let ground = y + fit.ground;

    // Color change: recolor the wearer's middle, from just under the collar to
    // most of the way to the ground.
    if worn.contains("color_change") {
        let base = outfit_color(color_choice);
        let mut tint = base;
        tint.a = 0.6;
        let torso = ((ground - chest) * 0.8).max(8.0);
        draw_rectangle(hx(-10.0), chest - 2.0, 20.0 * s, torso, tint);
        // Solid colored sash across the chest — unambiguous at a glance.
        draw_rectangle(hx(-10.0), chest + torso * 0.35, 20.0 * s, 4.0 * s, base);
    }
    // Jet boots: flames under the feet, on the wearer's own ground line.
    if worn.contains("jet_boots") {
        let flame = Color::from_rgba(255, 143, 0, 255);
        let flick = (time * 20.0).sin() * 1.5;
        draw_triangle(
            vec2(hx(-7.0), ground), vec2(hx(-1.0), ground),
            vec2(hx(-4.0), ground + 9.0 * s + flick), flame,
        );
        draw_triangle(
            vec2(hx(1.0), ground), vec2(hx(7.0), ground),
            vec2(hx(4.0), ground + 9.0 * s - flick), flame,
        );
    }
    // Bow tie at the collar. Hidden when facing away — a bow tie on the back
    // of the neck just looks silly.
    if worn.contains("bow_tie") && dir != Dir::Up {
        let c = Color::from_rgba(216, 27, 96, 255);
        draw_triangle(
            vec2(hx(0.0), chest), vec2(hx(-7.0), chest - 2.0 * s),
            vec2(hx(-7.0), chest + 2.0 * s), c,
        );
        draw_triangle(
            vec2(hx(0.0), chest), vec2(hx(7.0), chest - 2.0 * s),
            vec2(hx(7.0), chest + 2.0 * s), c,
        );
        draw_rectangle(hx(-1.5), chest - 1.5 * s, 3.0 * s, 3.0 * s,
            Color::from_rgba(136, 14, 79, 255));
    }
    // Hat: a red cap with a dark brim, brim resting on the head line.
    if worn.contains("hat") {
        draw_rectangle(hx(-10.0), head, 20.0 * s, 3.0 * s, Color::from_rgba(33, 33, 40, 255));
        draw_rectangle(hx(-7.0), head - 8.0 * s, 14.0 * s, 8.0 * s,
            Color::from_rgba(229, 57, 53, 255));
    }

    // ── Reef swag, from Hermie's deep stall ──────────────────────────────
    // Kelp crown: a ring of green fronds around the head line.
    if worn.contains("kelp_crown") {
        let kelp = Color::from_rgba(74, 158, 96, 255);
        let band = Color::from_rgba(52, 122, 74, 255);
        draw_rectangle(hx(-9.0), head, 18.0 * s, 3.5 * s, band);
        for i in 0..5 {
            let fx = -8.0 + i as f32 * 4.0;
            let wave = (time * 2.0 + i as f32).sin() * 1.5;
            draw_triangle(
                vec2(hx(fx - 1.5), head),
                vec2(hx(fx + 1.5), head),
                vec2(hx(fx + wave), head - (8.0 + (i % 2) as f32 * 3.0) * s),
                kelp,
            );
        }
    }
    // Shell necklace: a string of little shells across the collar.
    if worn.contains("shell_necklace") && dir != Dir::Up {
        let cord = Color::from_rgba(228, 220, 200, 200);
        let shell = Color::from_rgba(247, 198, 176, 255);
        draw_line(hx(-8.0), chest, hx(8.0), chest, 1.2 * s.max(0.6), cord);
        for i in 0..5 {
            let sx = -8.0 + i as f32 * 4.0;
            let dip = if i == 2 { 3.0 } else { (i as f32 - 2.0).abs() * -0.6 + 2.0 };
            draw_circle(hx(sx), chest + dip * s, 2.0 * s, shell);
        }
    }
    // Starfish badge: a five-pointed star pinned just under the collar.
    if worn.contains("starfish_badge") && dir != Dir::Up {
        let star = Color::from_rgba(255, 166, 77, 255);
        let by = chest + 5.0 * s;
        for i in 0..5 {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 5.0;
            let b = a + std::f32::consts::TAU / 10.0;
            let c = a - std::f32::consts::TAU / 10.0;
            draw_triangle(
                vec2(cx + a.cos() * 6.0 * s, by + a.sin() * 6.0 * s),
                vec2(cx + b.cos() * 2.6 * s, by + b.sin() * 2.6 * s),
                vec2(cx + c.cos() * 2.6 * s, by + c.sin() * 2.6 * s),
                star,
            );
        }
        draw_circle(cx, by, 2.4 * s, Color::from_rgba(255, 205, 140, 255));
    }
    // Glow lantern: a lamp on a stalk, floating just above and ahead of the
    // head, throwing a soft pool of light. The trench's status symbol.
    if worn.contains("glow_lantern") {
        let swing = (time * 1.8).sin() * 2.0;
        let lx = hx(11.0);
        let ly = head - 9.0 * s + swing;
        draw_line(hx(4.0), head - 1.0, lx, ly, 1.4, Color::new(0.85, 0.9, 0.9, 0.7));
        for ring in 0..3 {
            let r = (5.0 + ring as f32 * 4.0) * s;
            let a = 0.30 - ring as f32 * 0.09;
            draw_circle(lx, ly, r, Color::new(1.0, 0.94, 0.62, a));
        }
        draw_circle(lx, ly, 3.2 * s, Color::from_rgba(255, 246, 190, 255));
    }

    // Sparkle trail: a few twinkling motes orbiting the whole body.
    if worn.contains("sparkle_trail") {
        let gold = Color::from_rgba(255, 213, 79, 255);
        let mid = (head + ground) / 2.0;
        let reach = (ground - head).max(12.0) / 2.0 + 4.0;
        for i in 0..4 {
            let a = time * 2.0 + i as f32 * 1.7;
            let sx = cx + a.cos() * 17.0 * s;
            let sy = mid + (a * 1.3).sin() * reach;
            let r = (1.5 + ((time * 6.0 + i as f32).sin() * 0.5 + 0.5) * 1.5) * s;
            draw_circle(sx, sy, r, gold);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc::SpriteType;

    /// Anchors describe a real body: head above collar above ground, and the
    /// whole lot inside the tile. A fit that fails this puts a hat on
    /// somebody's face, which is exactly the bug these replaced.
    #[test]
    fn every_sprites_anchors_describe_a_body() {
        for sprite in SpriteType::ALL {
            let fit = sprite.swag_fit();
            assert!(fit.head < fit.chest,
                "{sprite:?}: the head has to be above the collar ({} vs {})", fit.head, fit.chest);
            assert!(fit.chest < fit.ground,
                "{sprite:?}: the collar has to be above the feet ({} vs {})", fit.chest, fit.ground);
            assert!(fit.head >= 0.0 && fit.ground <= TS,
                "{sprite:?}: anchors must stay inside the tile ({}..{})", fit.head, fit.ground);
            assert!(fit.scale > 0.3 && fit.scale <= 1.0, "{sprite:?}: odd scale {}", fit.scale);
            // A hat is 8 kid-units tall and sits on the head line; it mustn't
            // poke out the top of the tile above it.
            assert!(fit.head - 8.0 * fit.scale >= -4.0,
                "{sprite:?}: a hat would float off the top of the tile");
        }
    }

    #[test]
    fn inkwells_hat_sits_above_her_eyes() {
        // Regression: the old blanket sea-creature offset dropped the hat 13px
        // onto Inkwell's face. Her mantle tops out level with the kid's head,
        // so her head anchor has to be up there too.
        let octopus = SwagFit::OCTOPUS;
        assert!((octopus.head - SwagFit::KID.head).abs() <= 3.0,
            "an octopus mantle is as tall as a kid's head, so the hats agree");
        // Her eyes are drawn ~21px down the tile; the brim must clear them.
        assert!(octopus.head < 21.0, "the brim has to sit above her eyes");
    }
}
