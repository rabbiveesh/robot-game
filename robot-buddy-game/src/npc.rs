use macroquad::prelude::*;
use ::rand::{Rng, rngs::SmallRng};
use robot_buddy_domain::world::movement::{Direction, MoveIntent};
use crate::follower::Pathing;
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
    Pip,
    CtrlBand,
    CtrlKenkenLevel,
    CtrlCraReset,
    CtrlIntroReset,
    CtrlTriggerKenken,
    CtrlTriggerPattern,
    CtrlTriggerBalance,
    CtrlTriggerSudoku,
    CtrlTriggerChallenge,
    CtrlToggleEncounters,
    CtrlTriggerEncounter,
    CtrlToggleManipulatives,
    CtrlTriggerManipulative,
    CtrlToggleQuest,
    CtrlStartQuest,
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
            NpcKind::Pip => "pip",
            NpcKind::CtrlBand => "ctrl_band",
            NpcKind::CtrlKenkenLevel => "ctrl_kenken_level",
            NpcKind::CtrlCraReset => "ctrl_cra_reset",
            NpcKind::CtrlIntroReset => "ctrl_intro_reset",
            NpcKind::CtrlTriggerKenken => "ctrl_trigger_kenken",
            NpcKind::CtrlTriggerPattern => "ctrl_trigger_pattern",
            NpcKind::CtrlTriggerBalance => "ctrl_trigger_balance",
            NpcKind::CtrlTriggerSudoku => "ctrl_trigger_sudoku",
            NpcKind::CtrlTriggerChallenge => "ctrl_trigger_challenge",
            NpcKind::CtrlToggleEncounters => "ctrl_toggle_encounters",
            NpcKind::CtrlTriggerEncounter => "ctrl_trigger_encounter",
            NpcKind::CtrlToggleManipulatives => "ctrl_toggle_manipulatives",
            NpcKind::CtrlTriggerManipulative => "ctrl_trigger_manipulative",
            NpcKind::CtrlToggleQuest => "ctrl_toggle_quest",
            NpcKind::CtrlStartQuest => "ctrl_start_quest",
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
            NpcKind::Pip => "Pip",
            NpcKind::CtrlBand => "Band Knob",
            NpcKind::CtrlKenkenLevel => "KenKen Knob",
            NpcKind::CtrlCraReset => "CRA Reset",
            NpcKind::CtrlIntroReset => "Intro Reset",
            NpcKind::CtrlTriggerKenken => "Trigger KenKen",
            NpcKind::CtrlTriggerPattern => "Trigger Pattern",
            NpcKind::CtrlTriggerBalance => "Trigger Balance",
            NpcKind::CtrlTriggerSudoku => "Trigger Sudoku",
            NpcKind::CtrlTriggerChallenge => "Trigger Challenge",
            NpcKind::CtrlToggleEncounters => "Encounters Flag",
            NpcKind::CtrlTriggerEncounter => "Trigger Encounter",
            NpcKind::CtrlToggleManipulatives => "Manipulatives Flag",
            NpcKind::CtrlTriggerManipulative => "Trigger Manipulative",
            NpcKind::CtrlToggleQuest => "Quest Flag",
            NpcKind::CtrlStartQuest => "Start Quest",
        }
    }

    /// Every kind, in declaration order. The single roster the string⇄kind
    /// inverse walks — add a variant and the `as_str`/`display_name` matches
    /// stop compiling until it's handled, which is the nudge to list it here.
    pub const ALL: &'static [NpcKind] = &[
        NpcKind::Sage, NpcKind::SageLab, NpcKind::DreamSage, NpcKind::Mommy,
        NpcKind::Kid1, NpcKind::Kid2, NpcKind::Shopkeeper, NpcKind::GlitchDog,
        NpcKind::GroveSpirit, NpcKind::Pip, NpcKind::CtrlBand, NpcKind::CtrlKenkenLevel,
        NpcKind::CtrlCraReset, NpcKind::CtrlIntroReset, NpcKind::CtrlTriggerKenken,
        NpcKind::CtrlTriggerPattern, NpcKind::CtrlTriggerBalance, NpcKind::CtrlTriggerSudoku,
        NpcKind::CtrlTriggerChallenge, NpcKind::CtrlToggleEncounters, NpcKind::CtrlTriggerEncounter,
        NpcKind::CtrlToggleManipulatives, NpcKind::CtrlTriggerManipulative,
        NpcKind::CtrlToggleQuest, NpcKind::CtrlStartQuest,
    ];

    /// Inverse of `as_str`. Resolves a stable id token back to its kind without
    /// caring which map the NPC lives on — so a buddy from any map (including a
    /// brand-new one) gets a real display name, not a raw id fallback.
    pub fn from_id(id: &str) -> Option<NpcKind> {
        NpcKind::ALL.iter().copied().find(|k| k.as_str() == id)
    }

    pub fn is_dev_control(self) -> bool {
        matches!(self,
            NpcKind::CtrlBand | NpcKind::CtrlKenkenLevel | NpcKind::CtrlCraReset
            | NpcKind::CtrlIntroReset | NpcKind::CtrlTriggerKenken
            | NpcKind::CtrlTriggerPattern | NpcKind::CtrlTriggerBalance
            | NpcKind::CtrlTriggerSudoku | NpcKind::CtrlTriggerChallenge
            | NpcKind::CtrlToggleEncounters | NpcKind::CtrlTriggerEncounter
            | NpcKind::CtrlToggleManipulatives | NpcKind::CtrlTriggerManipulative
            | NpcKind::CtrlToggleQuest | NpcKind::CtrlStartQuest)
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
pub const WANDER_RADIUS: i32 = 3;
pub const WANDER_COOLDOWN_MIN: f32 = 1.5;
pub const WANDER_COOLDOWN_MAX: f32 = 3.0;

/// Roll a tethered wander intent. Returns `Stay` if `moving` is true, the
/// cooldown is still warm, the random direction would go off-grid, or it
/// would exceed `radius` from `home`. When it returns `Move`, also returns
/// the matching facing direction so the caller can update the entity's
/// sprite (we always set facing even on blocked rolls — feels alive).
///
/// Shared by stationary-roster NPCs (via `Npc::next_intent`) and by parked
/// Sparky idling near Professor Gizmo. The two only differ in where their
/// cooldown and entity live; the dice-roll itself is identical.
pub fn next_wander_intent(
    at: (usize, usize),
    moving: bool,
    home: (usize, usize),
    radius: i32,
    cooldown: &mut f32,
    dt: f32,
    rng: &mut SmallRng,
) -> (MoveIntent, Option<Dir>) {
    if moving { return (MoveIntent::Stay, None); }
    *cooldown -= dt;
    if *cooldown > 0.0 { return (MoveIntent::Stay, None); }
    *cooldown = WANDER_COOLDOWN_MIN
        + rng.gen::<f32>() * (WANDER_COOLDOWN_MAX - WANDER_COOLDOWN_MIN);

    let dirs = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];
    let dir = dirs[rng.gen_range(0..4)];
    let (dx, dy) = dir.delta();
    let nx = at.0 as i32 + dx;
    let ny = at.1 as i32 + dy;
    if nx < 0 || ny < 0 { return (MoveIntent::Stay, None); }

    if (nx - home.0 as i32).abs() > radius || (ny - home.1 as i32).abs() > radius {
        return (MoveIntent::Stay, None);
    }

    let face = match dir {
        Direction::Up => Dir::Up,
        Direction::Down => Dir::Down,
        Direction::Left => Dir::Left,
        Direction::Right => Dir::Right,
    };
    (MoveIntent::Move(dir), Some(face))
}

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
    /// Map this NPC belongs to. When the player gives them a dum dum they
    /// detach from their map roster and follow; when swapped out, they go
    /// home — back to (`home_tx`, `home_ty`) on `home_map`.
    pub home_map: &'static str,
    pub home_tx: usize,
    pub home_ty: usize,
    /// Time until the next wander attempt. Counts down only while stationary.
    pub wander_cooldown: f32,
    /// `Some` while this NPC is the player's companion follower. Wander and
    /// stationary behaviors yield to the path queue when this is set.
    pub pathing: Option<Pathing>,
}

impl Npc {
    pub fn name(&self) -> &'static str { self.kind.display_name() }
    pub fn id_str(&self) -> &'static str { self.kind.as_str() }

    /// Pixel-level interpolation. No movement decisions here -- those go
    /// through `next_intent` and the resolver. Returns true on the frame the
    /// NPC's pixel position catches up to its tile target — game.rs uses that
    /// signal to fire portal teleports without re-triggering on subsequent
    /// frames the NPC sits on the same tile.
    pub fn animate(&mut self, dt: f32) -> bool {
        self.entity.move_toward_target(dt)
    }

    /// True if the player has chosen this NPC as their travel companion. A
    /// companion ignores wander/stationary behavior and follows the player's
    /// path queue instead — see `next_follower_intent`.
    pub fn is_following(&self) -> bool { self.pathing.is_some() }

    /// Mark this NPC as the player's companion. The path queue starts empty —
    /// it fills as the player moves. Idempotent.
    pub fn start_following(&mut self) {
        if self.pathing.is_none() {
            self.pathing = Some(Pathing::new());
        }
    }

    /// Drop the follower role. After this the NPC resumes its normal wander or
    /// stationary behavior, anchored to whatever tile it's standing on.
    pub fn stop_following(&mut self) { self.pathing = None; }

    /// Snap this NPC back to its home tile and clear any follower state.
    /// Used when a companion is swapped out — the caller still has to decide
    /// which bucket (current map roster vs `npcs_offstage`) to place them in.
    /// Pure transformation on the NPC's own state.
    pub fn reset_to_home(&mut self) {
        self.stop_following();
        self.entity.tile_x = self.home_tx;
        self.entity.tile_y = self.home_ty;
        self.entity.x = self.home_tx as f32 * crate::tilemap::TILE_SIZE;
        self.entity.y = self.home_ty as f32 * crate::tilemap::TILE_SIZE;
        self.entity.target_x = self.entity.x;
        self.entity.target_y = self.entity.y;
        self.entity.moving = false;
        self.wander_cooldown = 0.0;
    }

    /// Ask the path queue what the follower wants to do this frame. Caller
    /// must clear `pathing` first if the NPC isn't following.
    pub fn next_follower_intent(&mut self, player_tx: usize, player_ty: usize) -> MoveIntent {
        let p = self.pathing.as_mut().expect("next_follower_intent on non-follower NPC");
        let d = p.next_intent(
            (self.entity.tile_x, self.entity.tile_y),
            self.entity.moving,
            (player_tx, player_ty),
        );
        if let Some(face) = d.face { self.entity.dir = face; }
        d.intent
    }

    /// Called when the resolver grants this follower's move so the path
    /// queue advances. No-op when not following.
    pub fn on_follower_move_granted(&mut self) {
        if let Some(p) = self.pathing.as_mut() {
            p.on_move_granted();
        }
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
        let (intent, face) = next_wander_intent(
            (self.entity.tile_x, self.entity.tile_y),
            self.entity.moving,
            (self.home_tx, self.home_ty),
            WANDER_RADIUS,
            &mut self.wander_cooldown,
            dt, rng,
        );
        if let Some(f) = face { self.entity.dir = f; }
        intent
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
/// default; chain `.wandering()` to opt in. `home_map` is the map id this
/// roster belongs to; passed by `npcs_for_map` so swapped-out companions
/// can find their way home.
fn npc(home_map: &'static str, kind: NpcKind, tx: usize, ty: usize, sprite: SpriteType,
       can_receive_gifts: bool, never_challenge: bool, is_puzzler: bool) -> Npc {
    Npc {
        kind,
        entity: Entity::new(tx, ty),
        sprite,
        can_receive_gifts,
        never_challenge,
        is_puzzler,
        wanders: false,
        home_map,
        home_tx: tx,
        home_ty: ty,
        wander_cooldown: 0.0,
        pathing: None,
    }
}

pub fn npcs_for_map(map_id: &'static str) -> Vec<Npc> {
    use NpcKind::*;
    use SpriteType as S;
    let n = |kind, tx, ty, sprite, can_gift, no_challenge, puzzler|
        npc(map_id, kind, tx, ty, sprite, can_gift, no_challenge, puzzler);
    match map_id {
        "overworld" => vec![
            n(Sage, 12, 12, S::Sage, true, false, true),
        ],
        "home" => vec![
            n(Mommy, 3, 3, S::Mommy, true, false, false),
            n(Kid1,  6, 5, S::Kid1,  true, true,  false).wandering(),
            n(Kid2,  8, 5, S::Kid2,  true, true,  false).wandering(),
        ],
        "lab" => vec![
            n(SageLab, 5, 3, S::Sage, true, false, true),
        ],
        "shop" => vec![
            n(Shopkeeper, 5, 2, S::Shopkeeper, true, false, false),
        ],
        "dream" => vec![
            n(DreamSage, 15, 8, S::Sage, false, false, false),
        ],
        "doghouse" => vec![
            n(GlitchDog, 7, 5, S::Dog, true, false, false),
        ],
        "grove" => vec![
            n(GroveSpirit, 6, 4, S::OldOak, true, false, false),
        ],
        // Dev-tier validation area, reachable from the dev map. Pip is a
        // wandering, gift-eligible critter whose only home is here — used to
        // prove the portal / wander / companion-swap systems work against a
        // freshly added map with no special-casing.
        "annex" => vec![
            n(Pip, 4, 3, S::Kid2, true, true, false).wandering(),
        ],
        "control" => vec![
            // Dev knob bay -- each NPC is one control. game.rs intercepts dev-control
            // kinds before the normal interaction flow and applies the effect.
            n(CtrlBand,             2,  2, S::Sage,       false, true, false),
            n(CtrlKenkenLevel,      5,  2, S::Shopkeeper, false, true, false),
            n(CtrlCraReset,         8,  2, S::OldOak,     false, true, false),
            n(CtrlIntroReset,      10,  2, S::Dog,        false, true, false),
            n(CtrlTriggerKenken,    3,  5, S::Kid1,       false, true, false),
            n(CtrlTriggerPattern,   5,  5, S::Sage,       false, true, false),
            n(CtrlTriggerBalance,   6,  5, S::OldOak,     false, true, false),
            n(CtrlTriggerSudoku,    7,  5, S::Kid1,       false, true, false),
            n(CtrlToggleEncounters, 2,  7, S::Dog,        false, true, false),
            n(CtrlTriggerEncounter, 4,  7, S::Sage,       false, true, false),
            n(CtrlToggleManipulatives, 6, 3, S::Kid2,    false, true, false),
            n(CtrlTriggerManipulative, 8, 7, S::Mommy,    false, true, false),
            n(CtrlToggleQuest,      10, 5, S::OldOak,     false, true, false),
            n(CtrlStartQuest,       10, 7, S::Sage,       false, true, false),
            n(CtrlTriggerChallenge, 8,  5, S::Kid2,       false, true, false),
        ],
        "dev" => vec![
            // Sprite gallery -- one of each NPC, lined up. Natural talk = TTS test.
            // Sage flagged as puzzler so dev/test flows can deterministically open a KenKen.
            n(Mommy,       2, 3, S::Mommy,      false, true, false),
            n(Sage,        4, 3, S::Sage,       false, true, true),
            n(Shopkeeper,  6, 3, S::Shopkeeper, false, true, false),
            n(Kid1,        8, 3, S::Kid1,       false, true, false),
            n(Kid2,       10, 3, S::Kid2,       false, true, false),
            n(GlitchDog,  12, 3, S::Dog,        false, true, false),
            n(GroveSpirit,13, 3, S::OldOak,     false, true, false),
        ],
        _ => vec![],
    }
}

/// Walk an NPC arriving at `(target_x, target_y)` to the closest non-blocking
/// tile. `is_solid` reports terrain walls; `is_occupied` reports player /
/// Sparky / other NPCs already on the destination map. Used after a portal
/// transfer so a teleporting NPC never lands on top of the player or another
/// entity. Falls back to the original target if nothing nearby is free.
pub fn find_npc_spawn_spot<S, O>(
    target_x: usize,
    target_y: usize,
    map_w: usize,
    map_h: usize,
    is_solid: S,
    is_occupied: O,
) -> (usize, usize)
where
    S: Fn(usize, usize) -> bool,
    O: Fn(usize, usize) -> bool,
{
    let blocked = |x: usize, y: usize| -> bool {
        x >= map_w || y >= map_h || is_solid(x, y) || is_occupied(x, y)
    };
    if !blocked(target_x, target_y) { return (target_x, target_y); }
    // Spiral outwards in Manhattan rings up to radius 5. Past that we just
    // give up and stack on the target — the wander step will naturally
    // de-stack on the next frame because tiles only carry one NPC.
    for radius in 1..=5_i32 {
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                if dx.abs() + dy.abs() != radius { continue; }
                let nx = target_x as i32 + dx;
                let ny = target_y as i32 + dy;
                if nx < 0 || ny < 0 { continue; }
                let (nx, ny) = (nx as usize, ny as usize);
                if blocked(nx, ny) { continue; }
                return (nx, ny);
            }
        }
    }
    (target_x, target_y)
}

/// Check if the player is facing an NPC and return it
pub fn get_interact_target(
    player_tx: usize, player_ty: usize, dir: sprites::Dir, npcs: &[Npc],
) -> Option<&Npc> {
    let (tx, ty) = facing_tile(player_tx, player_ty, dir)?;
    npcs.iter().find(|n| n.entity.tile_x == tx && n.entity.tile_y == ty)
}

/// Same as `get_interact_target`, but also considers the player's companion
/// (which doesn't live in the map roster). The companion takes priority — when
/// it's standing on the facing tile, it answers; otherwise we fall back to the
/// roster lookup.
pub fn get_interact_target_with_companion<'a>(
    player_tx: usize, player_ty: usize, dir: sprites::Dir,
    npcs: &'a [Npc], companion: Option<&'a Npc>,
) -> Option<&'a Npc> {
    let (tx, ty) = facing_tile(player_tx, player_ty, dir)?;
    if let Some(c) = companion {
        if c.entity.tile_x == tx && c.entity.tile_y == ty {
            return Some(c);
        }
    }
    npcs.iter().find(|n| n.entity.tile_x == tx && n.entity.tile_y == ty)
}

fn facing_tile(player_tx: usize, player_ty: usize, dir: sprites::Dir) -> Option<(usize, usize)> {
    let (tx, ty) = match dir {
        sprites::Dir::Up    => (player_tx as i32, player_ty as i32 - 1),
        sprites::Dir::Down  => (player_tx as i32, player_ty as i32 + 1),
        sprites::Dir::Left  => (player_tx as i32 - 1, player_ty as i32),
        sprites::Dir::Right => (player_tx as i32 + 1, player_ty as i32),
    };
    if tx < 0 || ty < 0 { None } else { Some((tx as usize, ty as usize)) }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_npc(home_map: &'static str, kind: NpcKind, tx: usize, ty: usize) -> Npc {
        npc(home_map, kind, tx, ty, SpriteType::Mommy, true, false, false)
    }

    #[test]
    fn start_following_installs_pathing() {
        let mut n = test_npc("home", NpcKind::Mommy, 3, 3);
        assert!(!n.is_following());
        n.start_following();
        assert!(n.is_following());
    }

    #[test]
    fn start_following_is_idempotent() {
        let mut n = test_npc("home", NpcKind::Mommy, 3, 3);
        n.start_following();
        // Seed a tile so we can tell if the queue got reset.
        n.pathing.as_mut().unwrap().record_player_pos(5, 5);
        n.start_following();
        assert!(!n.pathing.as_ref().unwrap().is_empty(),
            "second start_following should preserve the queue, not reset it");
    }

    #[test]
    fn reset_to_home_snaps_position_and_clears_following() {
        let mut n = test_npc("home", NpcKind::Mommy, 3, 3);
        n.entity.tile_x = 10;
        n.entity.tile_y = 10;
        n.entity.x = 480.0;
        n.entity.y = 480.0;
        n.start_following();

        n.reset_to_home();

        assert_eq!((n.entity.tile_x, n.entity.tile_y), (3, 3));
        assert_eq!(n.entity.x, 3.0 * crate::tilemap::TILE_SIZE);
        assert_eq!(n.entity.y, 3.0 * crate::tilemap::TILE_SIZE);
        assert!(!n.is_following());
        assert!(!n.entity.moving);
    }

    #[test]
    fn home_map_survives_a_round_trip_through_following() {
        let mut n = test_npc("home", NpcKind::Mommy, 3, 3);
        assert_eq!(n.home_map, "home");
        n.entity.tile_x = 5;
        n.entity.tile_y = 5;
        n.start_following();
        n.reset_to_home();
        assert_eq!(n.home_map, "home",
            "swap-out must not lose the NPC's home map — it's how they find their way back");
    }
}
