//! Path-following behavior shared by entities that trail the player.
//!
//! Two layers:
//!
//! - `Pathing` is the pure decision helper: it owns the queue of tiles to
//!   retrace and decides each frame whether the follower should Stay or step
//!   toward the next tile. Knows nothing about Entity or rendering.
//!
//! - `Follower` bundles an Entity with a Pathing. Sparky is exactly a
//!   `Follower` (no other state). A companion NPC isn't a `Follower` because
//!   it carries extra NPC metadata (kind, dialogue flags, home), but it uses
//!   the same `Pathing` internally and applies decisions the same way.

use robot_buddy_domain::world::movement::{Direction, MoveIntent};
use crate::game::Entity;
use crate::sprites::Dir;

#[derive(Clone, Copy)]
pub struct FollowDecision {
    pub intent: MoveIntent,
    /// Direction to face this frame, if the follower should turn. `None` keeps
    /// the current facing (used when mid-step or holding still with no signal).
    pub face: Option<Dir>,
}

impl FollowDecision {
    fn stay() -> Self {
        FollowDecision { intent: MoveIntent::Stay, face: None }
    }
}

#[derive(Clone, Default)]
pub struct Pathing {
    queue: Vec<(usize, usize)>,
}

impl Pathing {
    pub fn new() -> Self { Pathing { queue: Vec::new() } }

    pub fn clear(&mut self) { self.queue.clear(); }

    pub fn is_empty(&self) -> bool { self.queue.is_empty() }

    /// Record the player's current tile. Idempotent — re-recording the most
    /// recent tile is a no-op, so it's safe to call every frame.
    pub fn record_player_pos(&mut self, tx: usize, ty: usize) {
        if self.queue.last() != Some(&(tx, ty)) {
            self.queue.push((tx, ty));
        }
    }

    /// Decide what the follower at `at` wants to do this frame, given the
    /// player is at `player_at`. `moving` is true if the follower is mid-step.
    ///
    /// Does NOT pop the queue on Move — the apply phase calls `on_move_granted`
    /// after the resolver, so a denied move retries on the next frame.
    pub fn next_intent(
        &mut self,
        at: (usize, usize),
        moving: bool,
        player_at: (usize, usize),
    ) -> FollowDecision {
        if moving || self.queue.is_empty() {
            return FollowDecision::stay();
        }

        // Already adjacent: drop any backlog, just face the player.
        let dx_abs = (at.0 as i32 - player_at.0 as i32).abs();
        let dy_abs = (at.1 as i32 - player_at.1 as i32).abs();
        if dx_abs + dy_abs <= 1 {
            self.queue.clear();
            let fdx = player_at.0 as i32 - at.0 as i32;
            let fdy = player_at.1 as i32 - at.1 as i32;
            let face = if fdx < 0 { Some(Dir::Left) }
                else if fdx > 0 { Some(Dir::Right) }
                else if fdy < 0 { Some(Dir::Up) }
                else if fdy > 0 { Some(Dir::Down) }
                else { None };
            return FollowDecision { intent: MoveIntent::Stay, face };
        }

        // Peek the next queue entry. Don't pop -- the apply phase pops on grant.
        let (nx, ny) = self.queue[0];
        if (nx, ny) == player_at {
            // Next step would land on the player. Skip it; try again next frame.
            self.queue.remove(0);
            return FollowDecision::stay();
        }
        let dx = nx as i32 - at.0 as i32;
        let dy = ny as i32 - at.1 as i32;
        let (dir, face) = match (dx.signum(), dy.signum()) {
            (-1, 0) => (Direction::Left,  Dir::Left),
            ( 1, 0) => (Direction::Right, Dir::Right),
            (0, -1) => (Direction::Up,    Dir::Up),
            (0,  1) => (Direction::Down,  Dir::Down),
            _ => return FollowDecision::stay(),
        };
        FollowDecision { intent: MoveIntent::Move(dir), face: Some(face) }
    }

    /// Call after the resolver grants a Move for this follower so the queue
    /// advances. Denied moves don't pop, so the follower retries next frame.
    pub fn on_move_granted(&mut self) {
        if !self.queue.is_empty() {
            self.queue.remove(0);
        }
    }
}

/// An entity that exists only to trail the player — Sparky is one of these.
/// NPC companions don't use `Follower` because they carry NPC metadata too;
/// they reuse `Pathing` directly.
pub struct Follower {
    pub entity: Entity,
    pub pathing: Pathing,
}

impl Follower {
    pub fn new(tile_x: usize, tile_y: usize) -> Self {
        Follower {
            entity: Entity::new(tile_x, tile_y),
            pathing: Pathing::new(),
        }
    }

    /// Pixel-level interpolation toward the current tile target. Returns true
    /// on the frame the follower's pixel position catches up to its tile.
    pub fn animate(&mut self, dt: f32) -> bool {
        self.entity.move_toward_target(dt)
    }

    /// Decide what the follower wants to do this frame and apply the facing
    /// direction returned by the path queue. Caller still pushes the intent
    /// into the resolver and calls `on_move_granted` on a Granted resolution.
    pub fn next_intent(&mut self, player_at: (usize, usize)) -> MoveIntent {
        let d = self.pathing.next_intent(
            (self.entity.tile_x, self.entity.tile_y),
            self.entity.moving,
            player_at,
        );
        if let Some(face) = d.face { self.entity.dir = face; }
        d.intent
    }

    pub fn record_player_pos(&mut self, tx: usize, ty: usize) {
        self.pathing.record_player_pos(tx, ty);
    }

    pub fn on_move_granted(&mut self) { self.pathing.on_move_granted(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_queue_stays() {
        let mut p = Pathing::new();
        let d = p.next_intent((5, 5), false, (10, 10));
        assert!(matches!(d.intent, MoveIntent::Stay));
        assert!(d.face.is_none());
    }

    #[test]
    fn mid_step_stays() {
        let mut p = Pathing::new();
        p.record_player_pos(6, 5);
        let d = p.next_intent((5, 5), true, (6, 5));
        assert!(matches!(d.intent, MoveIntent::Stay));
    }

    #[test]
    fn adjacent_clears_and_faces_player() {
        let mut p = Pathing::new();
        p.record_player_pos(10, 10);
        p.record_player_pos(11, 10);
        // Follower at (10,10), player at (11,10) -> face Right, stay.
        let d = p.next_intent((10, 10), false, (11, 10));
        assert!(matches!(d.intent, MoveIntent::Stay));
        assert_eq!(d.face, Some(Dir::Right));
        assert!(p.is_empty());
    }

    #[test]
    fn moves_toward_queued_tile() {
        let mut p = Pathing::new();
        p.record_player_pos(6, 5);
        // Follower at (5,5), player walked to (6,5); not adjacent yet because
        // queue has (6,5) and we ARE non-adjacent until we move. But Manhattan
        // distance is 1 so adjacent rule triggers. Use a farther player.
        let d = p.next_intent((5, 5), false, (10, 10));
        assert!(matches!(d.intent, MoveIntent::Move(Direction::Right)));
        assert_eq!(d.face, Some(Dir::Right));
    }

    #[test]
    fn skips_queue_entry_landing_on_player() {
        let mut p = Pathing::new();
        // Player stepped backward (5,5) onto a tile we'd queued.
        p.record_player_pos(5, 5);
        // Follower at (4,5) sees queue entry (5,5) == player at (5,5).
        // We're already adjacent (Manhattan = 1) so adjacent branch fires first;
        // use a non-adjacent follower to exercise the skip path.
        let d = p.next_intent((2, 5), false, (5, 5));
        assert!(matches!(d.intent, MoveIntent::Stay));
        assert!(p.is_empty(), "the landing-on-player entry should have been removed");
    }

    #[test]
    fn on_move_granted_pops_queue() {
        let mut p = Pathing::new();
        p.record_player_pos(6, 5);
        p.record_player_pos(7, 5);
        assert_eq!(p.queue.len(), 2);
        p.on_move_granted();
        assert_eq!(p.queue.len(), 1);
        assert_eq!(p.queue[0], (7, 5));
    }
}
