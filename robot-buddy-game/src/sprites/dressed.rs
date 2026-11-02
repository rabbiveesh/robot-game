//! Characters drawn *with* whatever they're wearing — the one way a body and
//! its swag reach the screen, so no screen can draw a buddy and forget the
//! kelp crown they were given.
//!
//! A [`Posture`] places the body: where its pivot (the middle of the body)
//! goes, how big, how squashed, how far spun. The body and its swag are both
//! drawn in the sprite's own 48×48 tile space under that one transform, so a
//! hat stays on the head through a squish, a spin or a backflip. The world's
//! sprites sit upright at their tile and skip the transform entirely.
//!
//! Who wears what lives in the domain wardrobe; the caller hands over the
//! wearer's set, colour and fit as an [`Outfit`].

use crate::prelude::*;
use std::collections::BTreeSet;

use super::npcs::{draw_clam_cartoon, ClamFace};
use super::swag::{draw_swag, SwagFit};
use super::Dir;
use crate::npc::SpriteType;

/// The point a posture pivots on, in tile space: the middle of the body.
pub const PIVOT: (f32, f32) = (24.0, 30.0);

/// What someone is wearing.
pub struct Outfit<'a> {
    pub worn: &'a BTreeSet<String>,
    /// Their Color Change colour (only used if they wear it).
    pub color: &'a str,
    pub fit: SwagFit,
}

/// Where and how a body is drawn.
#[derive(Debug, Clone, Copy)]
pub struct Posture {
    /// Screen position of the pivot.
    pub x: f32,
    pub y: f32,
    /// Overall size; 1 is a world tile.
    pub scale: f32,
    /// Squash: >1 wide and flat, <1 tall and thin. Area-preserving.
    pub squash: f32,
    /// Clockwise, degrees.
    pub spin: f32,
}

impl Posture {
    /// Upright at a tile whose top-left is (x, y) — how the world draws.
    pub fn tile(x: f32, y: f32) -> Self {
        Posture { x: x + PIVOT.0, y: y + PIVOT.1, scale: 1.0, squash: 1.0, spin: 0.0 }
    }

    fn is_plain(&self) -> bool {
        self.scale == 1.0 && self.squash == 1.0 && self.spin == 0.0
    }
}

/// Which body to draw.
#[derive(Debug, Clone, Copy)]
pub enum Body {
    /// A world NPC sprite. `asleep` only matters to the gate shark.
    Npc { sprite: SpriteType, asleep: bool },
    /// Sparky.
    Robot { frame: u32 },
    /// Shelly as Pearl Hop draws her: a big cartoon clam with a face.
    Clam(ClamFace),
}

/// Draw `body` in `posture`, wearing `outfit`. The world only ever passes
/// plain postures, which draw in whatever camera is active.
pub fn draw_dressed(posture: Posture, body: Body, dir: Dir, outfit: &Outfit, time: f32) {
    if posture.is_plain() {
        return draw_body(posture.x - PIVOT.0, posture.y - PIVOT.1, body, dir, outfit, time);
    }
    let p = posture;
    let squash = p.squash.max(0.2);
    let xf = Affine2::from_translation(vec2(p.x, p.y))
        * Affine2::from_angle(p.spin.to_radians())
        * Affine2::from_scale(vec2(p.scale * squash, p.scale / squash))
        * Affine2::from_translation(vec2(-PIVOT.0, -PIVOT.1));
    with_transform(xf, || draw_body(0.0, 0.0, body, dir, outfit, time));
}

/// The body and its swag at tile (ox, oy).
fn draw_body(ox: f32, oy: f32, body: Body, dir: Dir, outfit: &Outfit, time: f32) {
    match body {
        Body::Npc { sprite, asleep } => sprite.draw_sprite(ox, oy, dir, time, asleep),
        Body::Robot { frame } => super::robot::draw_robot(ox, oy, dir, frame, time),
        Body::Clam(face) => draw_clam_cartoon(ox, oy, face, time),
    }
    if !outfit.worn.is_empty() {
        draw_swag(ox, oy, dir, 0.0, outfit.worn, outfit.color, outfit.fit);
    }
}
