//! Tile-grid movement resolver.
//!
//! The game runs a two-phase frame: each entity emits a `MoveIntent`, and
//! `resolve_moves` decides — purely against the world snapshot — which intents
//! become `Granted` moves. The game crate then applies the granted moves by
//! calling `Entity::start_move` (or equivalent).
//!
//! This module is pure Rust with no game-crate or macroquad dependencies, so
//! every collision rule is unit-testable as a plain `cargo test`.

use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Direction { Up, Down, Left, Right }

impl Direction {
    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::Up    => (0, -1),
            Direction::Down  => (0,  1),
            Direction::Left  => (-1, 0),
            Direction::Right => ( 1, 0),
        }
    }
}

/// Stable identifier for entities the resolver can move. The `Npc` variant
/// carries an opaque token; the game crate decides what it means (typically a
/// `NpcKind` cast to u32).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum EntityId {
    Player,
    Sparky,
    Npc(u32),
}

/// What an entity is willing to do this frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveIntent {
    Stay,
    Move(Direction),
}

/// How the resolver treats an entity's current and target tiles for blocking.
///
/// `Solid` is the default — anyone trying to enter the tile is blocked.
///
/// `SoftAfter(threshold)` means the entity blocks normally, *unless* the
/// pressure accumulated against this entity (from the per-frame pressure map)
/// has reached `threshold` seconds. Once that happens, this entity's tiles
/// are treated as passable for the duration of this frame's resolution. This
/// is how Sparky becomes phase-through after the player presses into him for
/// 0.12s — the legacy `sparky_push_timer` becomes one row of this map.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Solidity {
    Solid,
    SoftAfter(f32),
}

/// Snapshot of an entity's position the resolver sees this frame. The game
/// crate builds these fresh each frame from its mutable entity storage.
///
/// `moving_to` is `Some(dest_tile)` for entities currently mid-step (their
/// pixel-level interpolation hasn't caught up to their tile coords yet). The
/// resolver reserves *both* `(tile_x, tile_y)` and `moving_to` so other
/// entities can't walk into a tile that's about to be claimed.
#[derive(Clone, Debug)]
pub struct EntityState {
    pub id: EntityId,
    pub tile_x: usize,
    pub tile_y: usize,
    pub moving_to: Option<(usize, usize)>,
    pub solidity: Solidity,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockReason {
    Wall,
    OutOfBounds,
    Entity(EntityId),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveResolution {
    Granted { entity: EntityId, from: (usize, usize), to: (usize, usize) },
    Stayed  { entity: EntityId },
    Blocked { entity: EntityId, reason: BlockReason },
}

/// Map dimensions. Tiles are addressed as (x, y) with x in 0..width.
#[derive(Clone, Copy, Debug)]
pub struct GridDims { pub width: usize, pub height: usize }

/// Resolve a frame's worth of intents against a static map and a snapshot of
/// every entity's position.
///
/// Order of `intents` matters: when two intents target the same tile, the one
/// processed first wins and the later one is `Blocked` against it. Callers
/// typically pass intents in (Player, Sparky, NPCs) order so the player has
/// frame-priority.
///
/// `pressure_against` is a per-entity "soft-block pressure has accumulated
/// for this long against me" map. Only entities with `Solidity::SoftAfter`
/// consult it; the rest are unconditionally Solid.
pub fn resolve_moves<W>(
    entities: &[EntityState],
    intents: &[(EntityId, MoveIntent)],
    grid: GridDims,
    is_wall: W,
    pressure_against: &HashMap<EntityId, f32>,
) -> Vec<MoveResolution>
where
    W: Fn(usize, usize) -> bool,
{
    // Pre-build occupancy: every entity reserves its current tile, and any
    // mid-step entity also reserves its destination.
    let mut occ: HashMap<(usize, usize), EntityId> = HashMap::with_capacity(entities.len() * 2);
    for e in entities {
        occ.insert((e.tile_x, e.tile_y), e.id);
        if let Some(dest) = e.moving_to {
            occ.entry(dest).or_insert(e.id);
        }
    }

    let solidity_of: HashMap<EntityId, Solidity> =
        entities.iter().map(|e| (e.id, e.solidity)).collect();

    // Tiles claimed by *this frame's* granted moves. Prevents two intents
    // resolving onto the same empty tile.
    let mut reserved: HashSet<(usize, usize)> = HashSet::new();

    let mut resolutions = Vec::with_capacity(intents.len());

    for (id, intent) in intents {
        let from = match entities.iter().find(|e| e.id == *id) {
            Some(s) => (s.tile_x, s.tile_y),
            // Caller passed an intent for an unknown entity. Treat as Stayed
            // rather than panicking — keeps the resolver robust to bugs.
            None => { resolutions.push(MoveResolution::Stayed { entity: *id }); continue; }
        };

        let dir = match intent {
            MoveIntent::Stay => {
                resolutions.push(MoveResolution::Stayed { entity: *id });
                continue;
            }
            MoveIntent::Move(d) => *d,
        };

        let (dx, dy) = dir.delta();
        let nx = from.0 as i32 + dx;
        let ny = from.1 as i32 + dy;
        if nx < 0 || ny < 0 {
            resolutions.push(MoveResolution::Blocked { entity: *id, reason: BlockReason::OutOfBounds });
            continue;
        }
        let to = (nx as usize, ny as usize);
        if to.0 >= grid.width || to.1 >= grid.height {
            resolutions.push(MoveResolution::Blocked { entity: *id, reason: BlockReason::OutOfBounds });
            continue;
        }
        if is_wall(to.0, to.1) {
            resolutions.push(MoveResolution::Blocked { entity: *id, reason: BlockReason::Wall });
            continue;
        }
        // Earlier-this-frame grant — beats any soft-block decision.
        if reserved.contains(&to) {
            // The grant that put `to` into reserved tells us who's there now.
            // Prefer that EntityId in the block reason for clarity.
            let blocker = resolutions.iter().rev().find_map(|r| match r {
                MoveResolution::Granted { entity, to: t, .. } if *t == to => Some(*entity),
                _ => None,
            }).unwrap_or(*id);
            resolutions.push(MoveResolution::Blocked { entity: *id, reason: BlockReason::Entity(blocker) });
            continue;
        }
        if let Some(&other_id) = occ.get(&to) {
            if other_id != *id {
                let passable = match solidity_of.get(&other_id).copied().unwrap_or(Solidity::Solid) {
                    Solidity::Solid => false,
                    Solidity::SoftAfter(threshold) => {
                        pressure_against.get(&other_id).copied().unwrap_or(0.0) >= threshold
                    }
                };
                if !passable {
                    resolutions.push(MoveResolution::Blocked { entity: *id, reason: BlockReason::Entity(other_id) });
                    continue;
                }
            }
        }

        reserved.insert(to);
        resolutions.push(MoveResolution::Granted { entity: *id, from, to });
    }

    resolutions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: EntityId, x: usize, y: usize) -> EntityState {
        EntityState { id, tile_x: x, tile_y: y, moving_to: None, solidity: Solidity::Solid }
    }

    fn no_walls(_x: usize, _y: usize) -> bool { false }
    fn dims(w: usize, h: usize) -> GridDims { GridDims { width: w, height: h } }

    #[test]
    fn stay_intent_yields_stayed() {
        let states = [entity(EntityId::Player, 5, 5)];
        let intents = [(EntityId::Player, MoveIntent::Stay)];
        let r = resolve_moves(&states, &intents, dims(10, 10), no_walls, &HashMap::new());
        assert_eq!(r, vec![MoveResolution::Stayed { entity: EntityId::Player }]);
    }

    #[test]
    fn move_into_open_tile_is_granted() {
        let states = [entity(EntityId::Player, 5, 5)];
        let intents = [(EntityId::Player, MoveIntent::Move(Direction::Right))];
        let r = resolve_moves(&states, &intents, dims(10, 10), no_walls, &HashMap::new());
        assert_eq!(r, vec![MoveResolution::Granted {
            entity: EntityId::Player, from: (5, 5), to: (6, 5),
        }]);
    }

    #[test]
    fn move_into_wall_is_blocked() {
        let states = [entity(EntityId::Player, 5, 5)];
        let intents = [(EntityId::Player, MoveIntent::Move(Direction::Right))];
        let is_wall = |x: usize, _y: usize| x == 6;
        let r = resolve_moves(&states, &intents, dims(10, 10), is_wall, &HashMap::new());
        assert_eq!(r, vec![MoveResolution::Blocked {
            entity: EntityId::Player, reason: BlockReason::Wall,
        }]);
    }

    #[test]
    fn move_off_grid_is_blocked_out_of_bounds() {
        let states = [entity(EntityId::Player, 0, 5)];
        let intents = [(EntityId::Player, MoveIntent::Move(Direction::Left))];
        let r = resolve_moves(&states, &intents, dims(10, 10), no_walls, &HashMap::new());
        assert_eq!(r, vec![MoveResolution::Blocked {
            entity: EntityId::Player, reason: BlockReason::OutOfBounds,
        }]);

        let states = [entity(EntityId::Player, 9, 5)];
        let intents = [(EntityId::Player, MoveIntent::Move(Direction::Right))];
        let r = resolve_moves(&states, &intents, dims(10, 10), no_walls, &HashMap::new());
        assert_eq!(r, vec![MoveResolution::Blocked {
            entity: EntityId::Player, reason: BlockReason::OutOfBounds,
        }]);
    }

    #[test]
    fn move_into_solid_entity_is_blocked() {
        let states = [
            entity(EntityId::Player, 5, 5),
            entity(EntityId::Npc(1), 6, 5),
        ];
        let intents = [(EntityId::Player, MoveIntent::Move(Direction::Right))];
        let r = resolve_moves(&states, &intents, dims(10, 10), no_walls, &HashMap::new());
        assert_eq!(r, vec![MoveResolution::Blocked {
            entity: EntityId::Player, reason: BlockReason::Entity(EntityId::Npc(1)),
        }]);
    }

    #[test]
    fn moving_entity_reserves_target_tile_too() {
        // NPC is mid-step from (6,5) to (7,5). Player at (5,5) tries to walk
        // to (6,5) — currently the NPC's source tile. That tile is still
        // reserved by the NPC's `(tile_x, tile_y)` snapshot (since the game
        // crate's convention is `tile_x = destination once moving`, the
        // source is whichever non-destination tile the NPC also occupies via
        // moving_to inverse — for this test we'll model it as the NPC's
        // tile_x being the source and moving_to being the destination, which
        // matches a "just-issued move that hasn't applied yet").
        let mut npc = entity(EntityId::Npc(1), 6, 5);
        npc.moving_to = Some((7, 5));
        let states = [
            entity(EntityId::Player, 5, 5),
            npc,
        ];

        // Player→(6,5): blocked by NPC's source.
        let intents = [(EntityId::Player, MoveIntent::Move(Direction::Right))];
        let r = resolve_moves(&states, &intents, dims(10, 10), no_walls, &HashMap::new());
        assert_eq!(r[0], MoveResolution::Blocked {
            entity: EntityId::Player, reason: BlockReason::Entity(EntityId::Npc(1)),
        });

        // Another NPC tries to walk into (7,5) — the target tile of the moving
        // NPC. Should also be blocked.
        let mut other = entity(EntityId::Npc(2), 8, 5);
        other.tile_x = 8; other.tile_y = 5;
        let states2 = [
            entity(EntityId::Player, 0, 0),
            states[1].clone(),
            other,
        ];
        let intents2 = [(EntityId::Npc(2), MoveIntent::Move(Direction::Left))];
        let r = resolve_moves(&states2, &intents2, dims(10, 10), no_walls, &HashMap::new());
        assert_eq!(r[0], MoveResolution::Blocked {
            entity: EntityId::Npc(2), reason: BlockReason::Entity(EntityId::Npc(1)),
        });
    }

    #[test]
    fn two_intents_into_same_tile_first_wins() {
        let states = [
            entity(EntityId::Npc(1), 4, 5),
            entity(EntityId::Npc(2), 6, 5),
        ];
        let intents = [
            (EntityId::Npc(1), MoveIntent::Move(Direction::Right)), // → (5,5)
            (EntityId::Npc(2), MoveIntent::Move(Direction::Left)),  // → (5,5)
        ];
        let r = resolve_moves(&states, &intents, dims(10, 10), no_walls, &HashMap::new());
        assert_eq!(r[0], MoveResolution::Granted {
            entity: EntityId::Npc(1), from: (4, 5), to: (5, 5),
        });
        assert_eq!(r[1], MoveResolution::Blocked {
            entity: EntityId::Npc(2), reason: BlockReason::Entity(EntityId::Npc(1)),
        });
    }

    #[test]
    fn soft_after_blocks_until_pressure_threshold_met() {
        let mut sparky = entity(EntityId::Sparky, 6, 5);
        sparky.solidity = Solidity::SoftAfter(0.12);
        let states = [
            entity(EntityId::Player, 5, 5),
            sparky,
        ];
        let intents = [(EntityId::Player, MoveIntent::Move(Direction::Right))];

        // Below threshold → blocked.
        let mut p = HashMap::new();
        p.insert(EntityId::Sparky, 0.05);
        let r = resolve_moves(&states, &intents, dims(10, 10), no_walls, &p);
        assert_eq!(r[0], MoveResolution::Blocked {
            entity: EntityId::Player, reason: BlockReason::Entity(EntityId::Sparky),
        });

        // At threshold → granted.
        p.insert(EntityId::Sparky, 0.12);
        let r = resolve_moves(&states, &intents, dims(10, 10), no_walls, &p);
        assert_eq!(r[0], MoveResolution::Granted {
            entity: EntityId::Player, from: (5, 5), to: (6, 5),
        });
    }
}
