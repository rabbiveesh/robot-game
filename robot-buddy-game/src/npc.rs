use macroquad::prelude::*;
use ::rand::{Rng, rngs::SmallRng};
use robot_buddy_domain::world::movement::{Direction, MoveIntent};
use crate::game::Entity;
use crate::sprites::{self, Dir};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NpcKind {
    Sage,
    SageLab,
    DreamSage,
    Mommy,
    Kid1,
    Kid2,
    Shopkeeper,
    GlitchDog,
    GroveSpirit,
    CtrlBand,
    CtrlKenkenLevel,
    CtrlCraReset,
    CtrlIntroReset,
    CtrlTriggerKenken,
    CtrlTriggerChallenge,
}

impl NpcKind {
    /// Stable string token used by save data, dialogue keys, and menu_target_id.
    /// Matches the legacy id strings exactly so existing saves keep working.
    pub fn as_str(self) -> &'static str {
        match self {
            NpcKind::Sage => "sage",
            NpcKind::SageLab => "sage_lab",
            NpcKind::DreamSage => "dream_sage",
            NpcKind::Mommy => "mommy",
            NpcKind::Kid1 => "kid_1",
            NpcKind::Kid2 => "kid_2",
            NpcKind::Shopkeeper => "shopkeeper",
            NpcKind::GlitchDog => "glitch_dog",
            NpcKind::GroveSpirit => "grove_spirit",
            NpcKind::CtrlBand => "ctrl_band",
            NpcKind::CtrlKenkenLevel => "ctrl_kenken_level",
            NpcKind::CtrlCraReset => "ctrl_cra_reset",
            NpcKind::CtrlIntroReset => "ctrl_intro_reset",
            NpcKind::CtrlTriggerKenken => "ctrl_trigger_kenken",
            NpcKind::CtrlTriggerChallenge => "ctrl_trigger_challenge",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            NpcKind::Sage | NpcKind::SageLab => "Professor Gizmo",
            NpcKind::DreamSage => "???",
            NpcKind::Mommy => "Mommy",
            NpcKind::Kid1 => "Tali",
            NpcKind::Kid2 => "Noa",
            NpcKind::Shopkeeper => "Bolt the Shopkeeper",
            NpcKind::GlitchDog => "B0RK.exe",
            NpcKind::GroveSpirit => "Old Oak",
            NpcKind::CtrlBand => "Band Knob",
            NpcKind::CtrlKenkenLevel => "KenKen Knob",
            NpcKind::CtrlCraReset => "CRA Reset",
            NpcKind::CtrlIntroReset => "Intro Reset",
            NpcKind::CtrlTriggerKenken => "Trigger KenKen",
            NpcKind::CtrlTriggerChallenge => "Trigger Challenge",
        }
    }

    pub fn is_dev_control(self) -> bool {
        matches!(self,
            NpcKind::CtrlBand | NpcKind::CtrlKenkenLevel | NpcKind::CtrlCraReset
            | NpcKind::CtrlIntroReset | NpcKind::CtrlTriggerKenken
            | NpcKind::CtrlTriggerChallenge)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SpriteType {
    Mommy,
    Sage,
    Shopkeeper,
    Dog,
    Kid1,
    Kid2,
    OldOak,
}

/// Manhattan radius an NPC may wander away from its home tile. Keeps wanderers
/// from drifting across the whole map; small enough that the player can find
/// them reliably.
const WANDER_RADIUS: i32 = 3;
const WANDER_COOLDOWN_MIN: f32 = 1.5;
const WANDER_COOLDOWN_MAX: f32 = 3.0;

#[derive(Clone)]
pub struct Npc {
    pub kind: NpcKind,
    pub entity: Entity,
    pub sprite: SpriteType,
    pub can_receive_gifts: bool,
    pub never_challenge: bool,
    pub is_puzzler: bool,
    /// If true, this NPC emits random `Move` intents on a cooldown. The
    /// resolver decides whether each move actually happens; blocked moves
    /// turn into a Stay for that frame.
    pub wanders: bool,
    pub home_tx: usize,
    pub home_ty: usize,
    /// Time until the next wander attempt. Counts down only while stationary.
    pub wander_cooldown: f32,
}

impl Npc {
    pub fn name(&self) -> &'static str { self.kind.display_name() }
    pub fn id_str(&self) -> &'static str { self.kind.as_str() }

    /// Pixel-level interpolation. No movement decisions here -- those go
    /// through `next_intent` and the resolver.
    pub fn animate(&mut self, dt: f32) {
        self.entity.move_toward_target(dt);
    }

    /// Decide what this NPC wants to do this frame.
    ///
    /// Stationary, non-wandering NPCs always Stay. Wanderers tick down a
    /// cooldown; when it expires, they roll a random direction. The resolver
    /// (run after this) decides whether the move actually happens; blocks
    /// just mean the NPC stays put and rolls again on the next cooldown.
    ///
    /// Sets `entity.dir` to face the rolled direction so the NPC visibly
    /// "looks where it's going" even if the move ends up blocked.
    pub fn next_intent(&mut self, dt: f32, rng: &mut SmallRng) -> MoveIntent {
        if !self.wanders { return MoveIntent::Stay; }
        if self.entity.moving { return MoveIntent::Stay; }
        self.wander_cooldown -= dt;
        if self.wander_cooldown > 0.0 { return MoveIntent::Stay; }
        self.wander_cooldown = WANDER_COOLDOWN_MIN
            + rng.gen::<f32>() * (WANDER_COOLDOWN_MAX - WANDER_COOLDOWN_MIN);

        let dirs = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];
        let dir = dirs[rng.gen_range(0..4)];
        let (dx, dy) = dir.delta();
        let nx = self.entity.tile_x as i32 + dx;
        let ny = self.entity.tile_y as i32 + dy;
        if nx < 0 || ny < 0 { return MoveIntent::Stay; }

        // Tether to home so wanderers don't drift across the whole map. The
        // resolver handles wall/entity collisions on top.
        if (nx - self.home_tx as i32).abs() > WANDER_RADIUS
            || (ny - self.home_ty as i32).abs() > WANDER_RADIUS
        {
            return MoveIntent::Stay;
        }

        self.entity.dir = match dir {
            Direction::Up => Dir::Up,
            Direction::Down => Dir::Down,
            Direction::Left => Dir::Left,
            Direction::Right => Dir::Right,
        };
        MoveIntent::Move(dir)
    }

    /// Builder: mark this NPC as a wanderer. Sets the initial cooldown so
    /// wanderers don't all twitch on frame 1.
    pub fn wandering(mut self) -> Self {
        self.wanders = true;
        self.wander_cooldown = WANDER_COOLDOWN_MIN;
        self
    }

    pub fn draw(&self, time: f32) {
        let x = self.entity.x;
        let y = self.entity.y;
        match self.sprite {
            SpriteType::Mommy => sprites::npcs::draw_mommy(x, y, time),
            SpriteType::Sage => sprites::npcs::draw_sage(x, y, time),
            SpriteType::Shopkeeper => sprites::npcs::draw_shopkeeper(x, y, time),
            SpriteType::Dog => sprites::npcs::draw_dog(x, y, time),
            SpriteType::Kid1 => sprites::npcs::draw_kid(x, y,
                Color::from_rgba(255, 112, 67, 255),  // orange hair
                Color::from_rgba(253, 216, 53, 255),   // yellow shirt
                true, time),
            SpriteType::Kid2 => sprites::npcs::draw_kid(x, y,
                Color::from_rgba(109, 76, 65, 255),    // brown hair
                Color::from_rgba(102, 187, 106, 255),  // green shirt
                false, time),
            SpriteType::OldOak => sprites::npcs::draw_old_oak(x, y, time),
        }
    }
}

/// Constructor helper -- keeps the per-map NPC tables tidy. Stationary by
/// default; chain `.wandering()` to opt in.
fn npc(kind: NpcKind, tx: usize, ty: usize, sprite: SpriteType,
       can_receive_gifts: bool, never_challenge: bool, is_puzzler: bool) -> Npc {
    Npc {
        kind,
        entity: Entity::new(tx, ty),
        sprite,
        can_receive_gifts,
        never_challenge,
        is_puzzler,
        wanders: false,
        home_tx: tx,
        home_ty: ty,
        wander_cooldown: 0.0,
    }
}

pub fn npcs_for_map(map_id: &str) -> Vec<Npc> {
    use NpcKind::*;
    use SpriteType as S;
    match map_id {
        "overworld" => vec![
            npc(Sage, 12, 12, S::Sage, true, false, true),
        ],
        "home" => vec![
            npc(Mommy, 3, 3, S::Mommy, true, false, false),
            npc(Kid1,  6, 5, S::Kid1,  true, true,  false).wandering(),
            npc(Kid2,  8, 5, S::Kid2,  true, true,  false).wandering(),
        ],
        "lab" => vec![
            npc(SageLab, 5, 3, S::Sage, true, false, true),
        ],
        "shop" => vec![
            npc(Shopkeeper, 5, 2, S::Shopkeeper, true, false, false),
        ],
        "dream" => vec![
            npc(DreamSage, 15, 8, S::Sage, false, false, false),
        ],
        "doghouse" => vec![
            npc(GlitchDog, 7, 5, S::Dog, true, false, false),
        ],
        "grove" => vec![
            npc(GroveSpirit, 6, 4, S::OldOak, true, false, false),
        ],
        "control" => vec![
            // Dev knob bay -- each NPC is one control. game.rs intercepts dev-control
            // kinds before the normal interaction flow and applies the effect.
            npc(CtrlBand,             2,  2, S::Sage,       false, true, false),
            npc(CtrlKenkenLevel,      5,  2, S::Shopkeeper, false, true, false),
            npc(CtrlCraReset,         8,  2, S::OldOak,     false, true, false),
            npc(CtrlIntroReset,      10,  2, S::Dog,        false, true, false),
            npc(CtrlTriggerKenken,    3,  5, S::Kid1,       false, true, false),
            npc(CtrlTriggerChallenge, 8,  5, S::Kid2,       false, true, false),
        ],
        "dev" => vec![
            // Sprite gallery -- one of each NPC, lined up. Natural talk = TTS test.
            // Sage flagged as puzzler so dev/test flows can deterministically open a KenKen.
            npc(Mommy,       2, 3, S::Mommy,      false, true, false),
            npc(Sage,        4, 3, S::Sage,       false, true, true),
            npc(Shopkeeper,  6, 3, S::Shopkeeper, false, true, false),
            npc(Kid1,        8, 3, S::Kid1,       false, true, false),
            npc(Kid2,       10, 3, S::Kid2,       false, true, false),
            npc(GlitchDog,  12, 3, S::Dog,        false, true, false),
            npc(GroveSpirit,13, 3, S::OldOak,     false, true, false),
        ],
        _ => vec![],
    }
}

/// Check if the player is facing an NPC and return it
pub fn get_interact_target(
    player_tx: usize, player_ty: usize, dir: sprites::Dir, npcs: &[Npc],
) -> Option<&Npc> {
    let (tx, ty) = match dir {
        sprites::Dir::Up => (player_tx as i32, player_ty as i32 - 1),
        sprites::Dir::Down => (player_tx as i32, player_ty as i32 + 1),
        sprites::Dir::Left => (player_tx as i32 - 1, player_ty as i32),
        sprites::Dir::Right => (player_tx as i32 + 1, player_ty as i32),
    };
    if tx < 0 || ty < 0 { return None; }
    npcs.iter().find(|n| n.entity.tile_x == tx as usize && n.entity.tile_y == ty as usize)
}

/// Check if facing Sparky (the robot)
pub fn is_facing_sparky(
    player_tx: usize, player_ty: usize, dir: sprites::Dir,
    sparky_tx: usize, sparky_ty: usize,
) -> bool {
    let (tx, ty) = match dir {
        sprites::Dir::Up => (player_tx as i32, player_ty as i32 - 1),
        sprites::Dir::Down => (player_tx as i32, player_ty as i32 + 1),
        sprites::Dir::Left => (player_tx as i32 - 1, player_ty as i32),
        sprites::Dir::Right => (player_tx as i32 + 1, player_ty as i32),
    };
    tx >= 0 && ty >= 0 && tx as usize == sparky_tx && ty as usize == sparky_ty
}
