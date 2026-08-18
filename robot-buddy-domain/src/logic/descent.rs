//! The descent — diving a reef shaft, modelled as number-line decomposition.
//! Pure logic, no rendering.
//!
//! The trench door sits at a known depth. The diver sinks in *kicks* of fixed
//! sizes (say 1, 2 and 5 marks) and has to come to rest on the door EXACTLY:
//! 12 deep with kicks of 5, 5 and 2. Rock shelves jut out at some depths — you
//! can't come to rest on one, so the decomposition has to route around them.
//! Picking the kicks IS the arithmetic; there is no question to answer.
//!
//! Nothing here can fail. Overshoot and you settle on the shaft floor and kick
//! back up; bonk a shelf and the current sets you back where you were. Kicks
//! are unlimited and untimed — `kicks_used` exists only so the game can tell a
//! clean dive (matched `optimal_kicks`) from a scenic one, which is stealth
//! assessment, not a score the kid is shown.
//!
//! Public surface mirrors the other logic modules:
//!   - `generate_dive(band, &mut impl Rng)` → a solvable-by-construction shaft
//!   - `DiveSession::new(puzzle)` → fresh session
//!   - `dive_reducer(session, action)` → new session

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DivePuzzle {
    /// Depth of the trench door — the mark the diver must land on exactly.
    pub door: u8,
    /// Deepest mark in the shaft. Sinking past it just settles you here, which
    /// is how overshooting feels: you're on the bottom, kick back up.
    pub floor: u8,
    /// Kick sizes, ascending. The same sizes work downward and upward.
    pub kicks: Vec<u8>,
    /// Depths with a rock shelf. You can't come to rest on one.
    pub shelves: Vec<u8>,
}

impl DivePuzzle {
    pub fn is_shelf(&self, depth: u8) -> bool {
        self.shelves.contains(&depth)
    }

    /// Fewest kicks from the surface to the door, routing around shelves. The
    /// yardstick for a clean dive; also proves the shaft is solvable at all.
    pub fn optimal_kicks(&self) -> u8 {
        match self.best_route() {
            Some(route) => route.len() as u8,
            None => u8::MAX, // unreachable by construction; never shown to the kid
        }
    }

    /// One shortest sequence of kicks from the surface to the door, or `None`
    /// if the shaft can't be dived (which generation rules out). Breadth-first,
    /// so the first route found is a shortest one. Used for the clean-dive
    /// yardstick — and it's what a hint would replay if we ever offer one.
    pub fn best_route(&self) -> Option<Vec<DiveAction>> {
        let n = self.floor as usize + 1;
        // (previous depth, the kick that got here) for each visited depth.
        let mut came_from: Vec<Option<(u8, DiveAction)>> = vec![None; n];
        let mut seen = vec![false; n];
        let mut queue = VecDeque::new();
        seen[0] = true;
        queue.push_back(0u8);

        while let Some(depth) = queue.pop_front() {
            if depth == self.door {
                let mut route = Vec::new();
                let mut at = depth;
                while let Some((prev, action)) = came_from[at as usize] {
                    route.push(action);
                    at = prev;
                }
                route.reverse();
                return Some(route);
            }
            for &k in &self.kicks {
                let down = depth.saturating_add(k).min(self.floor);
                let up = depth.saturating_sub(k);
                for (next, action) in
                    [(down, DiveAction::Sink { n: k }), (up, DiveAction::Rise { n: k })]
                {
                    if next == depth || self.is_shelf(next) || seen[next as usize] {
                        continue;
                    }
                    seen[next as usize] = true;
                    came_from[next as usize] = Some((depth, action));
                    queue.push_back(next);
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivePhase {
    Diving,
    /// Rested on the door. The trench opens.
    Landed,
}

/// What the last kick did, so the game can react without inspecting deltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiveNudge {
    /// Moved normally (or hasn't moved yet).
    None,
    /// Ran into a rock shelf; the current set the diver back.
    Bumped,
    /// Sank past the door and settled on the shaft floor.
    Bottomed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiveSession {
    pub puzzle: DivePuzzle,
    pub depth: u8,
    pub kicks_used: u8,
    pub phase: DivePhase,
    /// Result of the most recent action — drives one beat of feedback.
    pub nudge: DiveNudge,
}

impl DiveSession {
    pub fn new(puzzle: DivePuzzle) -> Self {
        let phase = if puzzle.door == 0 { DivePhase::Landed } else { DivePhase::Diving };
        DiveSession { depth: 0, kicks_used: 0, phase, nudge: DiveNudge::None, puzzle }
    }

    /// Marks between the diver and the door — reads the same whether they're
    /// short of it or below it.
    pub fn distance_to_door(&self) -> u8 {
        if self.depth >= self.puzzle.door {
            self.depth - self.puzzle.door
        } else {
            self.puzzle.door - self.depth
        }
    }

    /// True when the dive took the fewest kicks it could have. Worth a pearl;
    /// never worth a scolding when it isn't.
    pub fn was_clean(&self) -> bool {
        self.phase == DivePhase::Landed && self.kicks_used <= self.puzzle.optimal_kicks()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DiveAction {
    /// Kick down `n` marks.
    Sink { n: u8 },
    /// Kick back up `n` marks. Always free, always available.
    Rise { n: u8 },
}

pub fn dive_reducer(state: DiveSession, action: DiveAction) -> DiveSession {
    let mut next = state;
    if next.phase == DivePhase::Landed {
        return next; // the door's open; nothing left to do
    }

    let (n, downward) = match action {
        DiveAction::Sink { n } => (n, true),
        DiveAction::Rise { n } => (n, false),
    };
    // Only the kicks this shaft offers do anything. An unknown size is a
    // no-op rather than an error — the UI can only offer real ones anyway.
    if n == 0 || !next.puzzle.kicks.contains(&n) {
        return next;
    }

    let raw = if downward {
        next.depth.saturating_add(n)
    } else {
        next.depth.saturating_sub(n)
    };
    let landed_on = raw.min(next.puzzle.floor);

    next.kicks_used = next.kicks_used.saturating_add(1);

    if next.puzzle.is_shelf(landed_on) {
        // A ledge you can't rest on: the current sets you back where you were.
        next.nudge = DiveNudge::Bumped;
        return next;
    }

    next.depth = landed_on;
    next.nudge = if landed_on == next.puzzle.floor && landed_on != next.puzzle.door {
        DiveNudge::Bottomed
    } else {
        DiveNudge::None
    };
    if next.depth == next.puzzle.door {
        next.phase = DivePhase::Landed;
    }
    next
}

/// Shaft shape by math band. Shallow shafts with unit kicks are pure counting;
/// deeper ones with a 5 and a 10 are decomposition, and the shelves force a
/// re-plan instead of one long slide. Bands above the table reuse its last row.
fn shaft_for_band(band: u8) -> (u8, Vec<u8>, usize, usize) {
    // (floor, kicks, shelves, kicks in the constructed solution)
    match band {
        0 | 1 => (8, vec![1, 2], 0, 2),
        2 => (12, vec![1, 2, 5], 1, 3),
        3 => (20, vec![1, 2, 5, 10], 2, 3),
        4 => (24, vec![1, 2, 3, 5, 10], 2, 4),
        _ => (30, vec![1, 2, 3, 5, 10], 3, 4),
    }
}

/// Build a shaft the diver can definitely reach the bottom door of: walk a
/// random legal route down first, then hang the shelves anywhere that route
/// didn't touch. Solvable by construction, so no reroll loop and no shaft that
/// strands a kid.
pub fn generate_dive(band: u8, rng: &mut impl Rng) -> DivePuzzle {
    let (floor, kicks, shelf_count, steps) = shaft_for_band(band);

    // Sink along a random route, staying shallow enough that the door always
    // has water beneath it — overshooting has to be possible, that's the game.
    let ceiling = floor.saturating_sub(2).max(1);
    let mut door = 0u8;
    let mut route = vec![0u8];
    for _ in 0..steps {
        let k = kicks[rng.gen_range(0..kicks.len())];
        if door.saturating_add(k) > ceiling {
            continue;
        }
        door += k;
        route.push(door);
    }
    // A route of all skipped kicks would leave the door at the surface; nudge
    // it down to the smallest real kick so there's always a dive to make.
    if door == 0 {
        door = kicks[0].min(ceiling);
        route.push(door);
    }

    let mut shelves: Vec<u8> = Vec::new();
    let mut candidates: Vec<u8> = (1..floor).filter(|d| !route.contains(d)).collect();
    for _ in 0..shelf_count {
        if candidates.is_empty() {
            break;
        }
        let i = rng.gen_range(0..candidates.len());
        shelves.push(candidates.remove(i));
    }
    shelves.sort_unstable();

    DivePuzzle { door, floor, kicks, shelves }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    /// Door at 12, kicks of 1/2/5, a rock shelf at 10. The tidy way down is
    /// 5 + 2 + 5; the obvious 5 + 5 runs straight into the shelf.
    fn shaft() -> DivePuzzle {
        DivePuzzle { door: 12, floor: 16, kicks: vec![1, 2, 5], shelves: vec![10] }
    }

    /// The same shaft with the ledge knocked off, for testing plain sinking.
    fn open_shaft() -> DivePuzzle {
        DivePuzzle { shelves: vec![], ..shaft() }
    }

    #[test]
    fn landing_on_the_door_opens_it() {
        let mut s = DiveSession::new(shaft());
        for n in [5, 2, 5] {
            s = dive_reducer(s, DiveAction::Sink { n });
        }
        assert_eq!(s.depth, 12);
        assert_eq!(s.phase, DivePhase::Landed);
        assert_eq!(s.kicks_used, 3);
        assert!(s.was_clean(), "three kicks is the fewest this shaft allows");
    }

    #[test]
    fn a_shelf_sets_the_diver_back_instead_of_stopping_them() {
        let mut s = DiveSession::new(shaft());
        s = dive_reducer(s, DiveAction::Sink { n: 5 });
        s = dive_reducer(s, DiveAction::Sink { n: 5 }); // straight into the ledge at 10
        assert_eq!(s.depth, 5, "the shelf is unrestable, so the current returns you");
        assert_eq!(s.nudge, DiveNudge::Bumped);
        assert_eq!(s.phase, DivePhase::Diving, "bonking is never an ending");
        // ...and a different decomposition gets past it.
        s = dive_reducer(s, DiveAction::Sink { n: 2 });
        s = dive_reducer(s, DiveAction::Sink { n: 5 });
        assert_eq!(s.depth, 12, "5 + 2 + 5 routes around the ledge");
        assert_eq!(s.phase, DivePhase::Landed);
    }

    #[test]
    fn overshooting_settles_on_the_floor_and_is_recoverable() {
        let mut s = DiveSession::new(open_shaft());
        for _ in 0..4 {
            s = dive_reducer(s, DiveAction::Sink { n: 5 });
        }
        assert_eq!(s.depth, 16, "sinking past the bottom just lands you on it");
        assert_eq!(s.nudge, DiveNudge::Bottomed);
        assert_eq!(s.phase, DivePhase::Diving);
        assert_eq!(s.distance_to_door(), 4);

        // Kick back up and finish the dive — no restart, no loss.
        s = dive_reducer(s, DiveAction::Rise { n: 2 });
        s = dive_reducer(s, DiveAction::Rise { n: 2 });
        assert_eq!(s.depth, 12);
        assert_eq!(s.phase, DivePhase::Landed);
    }

    #[test]
    fn rising_never_goes_above_the_surface() {
        let mut s = DiveSession::new(shaft());
        s = dive_reducer(s, DiveAction::Rise { n: 5 });
        assert_eq!(s.depth, 0);
    }

    #[test]
    fn kicks_the_shaft_does_not_offer_do_nothing() {
        let mut s = DiveSession::new(shaft());
        s = dive_reducer(s, DiveAction::Sink { n: 7 });
        assert_eq!(s.depth, 0);
        assert_eq!(s.kicks_used, 0, "an impossible kick isn't a wasted one");
    }

    #[test]
    fn optimal_kicks_routes_around_shelves() {
        // 5 + 5 + 2 and 5 + 2 + 5 are both three kicks; the ledge at 10 only
        // rules out the first, so the best dive is the same length either way.
        assert_eq!(open_shaft().optimal_kicks(), 3);
        assert_eq!(shaft().optimal_kicks(), 3);

        // Walling off every three-kick route makes the best dive longer.
        let fenced = DivePuzzle { shelves: vec![7, 10, 11], ..shaft() };
        assert!(fenced.optimal_kicks() > 3, "ledges should force a longer route");
    }

    #[test]
    fn a_scenic_dive_still_opens_the_door() {
        let mut s = DiveSession::new(open_shaft());
        for _ in 0..12 {
            s = dive_reducer(s, DiveAction::Sink { n: 1 });
        }
        assert_eq!(s.phase, DivePhase::Landed, "one mark at a time is a fine way down");
        assert!(!s.was_clean(), "...just not the fewest kicks");
    }

    #[test]
    fn generated_shafts_are_always_divable() {
        for band in 1..=6u8 {
            for seed in 0..40u64 {
                let mut rng = SmallRng::seed_from_u64(seed);
                let p = generate_dive(band, &mut rng);
                assert!(p.door >= 1, "band {band} seed {seed}: there must be a dive to make");
                assert!(p.door < p.floor, "band {band} seed {seed}: the door needs water beneath it");
                assert!(!p.is_shelf(p.door), "band {band} seed {seed}: the door can't be a shelf");
                assert!(!p.is_shelf(0), "band {band} seed {seed}: the surface can't be a shelf");
                let best = p.optimal_kicks();
                assert!(best != u8::MAX, "band {band} seed {seed}: shaft is unsolvable: {p:?}");
                assert!(best >= 1);
            }
        }
    }

    #[test]
    fn the_best_route_actually_reaches_the_door() {
        for band in 1..=6u8 {
            for seed in 0..25u64 {
                let mut rng = SmallRng::seed_from_u64(seed);
                let p = generate_dive(band, &mut rng);
                let route = p.best_route()
                    .unwrap_or_else(|| panic!("band {band} seed {seed}: no route down {p:?}"));
                let mut s = DiveSession::new(p.clone());
                for action in &route {
                    s = dive_reducer(s, *action);
                }
                assert_eq!(s.phase, DivePhase::Landed,
                    "band {band} seed {seed}: the best route should open the door: {p:?} via {route:?}");
                assert!(s.was_clean(), "band {band} seed {seed}: the best route is a clean dive");
            }
        }
    }

    #[test]
    fn deeper_bands_dive_deeper() {
        let mut rng = SmallRng::seed_from_u64(42);
        let shallow = generate_dive(1, &mut rng);
        let deep = generate_dive(5, &mut rng);
        assert!(deep.floor > shallow.floor);
        assert!(deep.kicks.len() > shallow.kicks.len());
    }

    #[test]
    fn the_same_seed_digs_the_same_shaft() {
        let a = generate_dive(3, &mut SmallRng::seed_from_u64(7));
        let b = generate_dive(3, &mut SmallRng::seed_from_u64(7));
        assert_eq!(a, b);
    }
}
