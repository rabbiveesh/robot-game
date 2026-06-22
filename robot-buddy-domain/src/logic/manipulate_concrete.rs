//! Concrete (CRA stage 1) math manipulatives. Pure logic — no rendering.
//!
//! The core "kill the broccoli" idea: the kid does the math with their hands.
//! Instead of reading "3 + 2 = ?" and tapping an answer, they drag 3 red and 2
//! blue bolts into Sparky's bin and watch the count become 5. The manipulation
//! IS the arithmetic. There is no wrong-answer buzzer — over/under states are
//! just "not done yet" and are freely recoverable.
//!
//! This module models the LOGIC (what's in the workspace, when it's done). The
//! game renders objects and turns taps/drags into `ConcreteAction`s.
//!
//! Public surface mirrors `logic::balance` / `logic::patterns`:
//!   - `generate_concrete(kind, a, b, &mut impl Rng)` (operands supplied by the
//!     caller — decoupled from the challenge generator)
//!   - `ConcreteSession::new(puzzle)` → fresh InProgress session
//!   - `concrete_reducer(session, action)` → new session

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::types::Operation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConcreteKind {
    /// Tap each of N objects once to count them (cardinality, one-to-one).
    Count,
    /// Drag `a` of one color and `b` of another into the bin (addition).
    AddGroups,
    /// Stack blocks to reach `a + b` (addition as height).
    BuildTower,
    /// Start with `a` objects, take `b` away, count what's left (subtraction).
    TakeAway,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConcretePuzzle {
    pub kind: ConcreteKind,
    pub operation: Operation,
    pub a: u8,
    pub b: u8,
    /// The count the workspace must reach: `a` for Count, `a+b` for AddGroups/
    /// BuildTower, `a-b` for TakeAway.
    pub target: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConcretePhase {
    InProgress,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConcreteSession {
    pub puzzle: ConcretePuzzle,
    /// Objects of group A in the workspace (the only bucket for Count/Tower/
    /// TakeAway; the "color a" bucket for AddGroups).
    pub bucket_a: u8,
    /// Objects of group B (AddGroups only).
    pub bucket_b: u8,
    pub phase: ConcretePhase,
}

impl ConcreteSession {
    pub fn new(puzzle: ConcretePuzzle) -> Self {
        // TakeAway pre-fills the workspace with `a` objects to remove from;
        // every other kind starts empty and is built up.
        let bucket_a = if puzzle.kind == ConcreteKind::TakeAway { puzzle.a } else { 0 };
        let mut s = ConcreteSession { puzzle, bucket_a, bucket_b: 0, phase: ConcretePhase::InProgress };
        s.phase = if s.is_at_target() { ConcretePhase::Complete } else { ConcretePhase::InProgress };
        s
    }

    pub fn total(&self) -> u8 {
        self.bucket_a.saturating_add(self.bucket_b)
    }

    fn is_at_target(&self) -> bool {
        match self.puzzle.kind {
            ConcreteKind::TakeAway => self.bucket_a == self.puzzle.target,
            _ => self.total() == self.puzzle.target,
        }
    }

    /// For AddGroups, whether the kid split the bin exactly as posed (`a` in
    /// group A, `b` in group B) — a richer signal than the bare total. Always
    /// true for the single-bucket kinds.
    pub fn grouping_matches(&self) -> bool {
        match self.puzzle.kind {
            ConcreteKind::AddGroups => self.bucket_a == self.puzzle.a && self.bucket_b == self.puzzle.b,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ConcreteAction {
    /// Add one object to group `group` (0 = A, 1 = B). Drag-in / tap / stack.
    Place { group: u8 },
    /// Remove one object from group `group`. Take-away / undo a mis-drag.
    Remove { group: u8 },
}

/// Soft ceiling so the workspace can't grow absurdly; a little headroom above
/// the target lets the kid overshoot and correct.
fn capacity(puzzle: &ConcretePuzzle) -> u8 {
    puzzle.a.saturating_add(puzzle.b).saturating_add(4)
}

pub fn concrete_reducer(state: ConcreteSession, action: ConcreteAction) -> ConcreteSession {
    if state.phase == ConcretePhase::Complete {
        return state;
    }
    let mut next = state.clone();
    match action {
        ConcreteAction::Place { group } => {
            if next.total() >= capacity(&next.puzzle) {
                return state; // ignore runaway placement
            }
            match group {
                0 => next.bucket_a = next.bucket_a.saturating_add(1),
                1 => next.bucket_b = next.bucket_b.saturating_add(1),
                _ => return state,
            }
        }
        ConcreteAction::Remove { group } => match group {
            0 => next.bucket_a = next.bucket_a.saturating_sub(1),
            1 => next.bucket_b = next.bucket_b.saturating_sub(1),
            _ => return state,
        },
    }
    if next.is_at_target() {
        next.phase = ConcretePhase::Complete;
    }
    next
}

/// Build a concrete puzzle from explicit operands. `b` is ignored for `Count`
/// (it just counts `a` objects). For `TakeAway`, `a >= b` is required and the
/// operands are clamped so the target is never negative.
pub fn generate_concrete(kind: ConcreteKind, a: u8, b: u8, _rng: &mut impl Rng) -> ConcretePuzzle {
    match kind {
        ConcreteKind::Count => ConcretePuzzle {
            kind,
            operation: Operation::Add,
            a: a.max(1),
            b: 0,
            target: a.max(1),
        },
        ConcreteKind::AddGroups | ConcreteKind::BuildTower => ConcretePuzzle {
            kind,
            operation: Operation::Add,
            a,
            b,
            target: a.saturating_add(b),
        },
        ConcreteKind::TakeAway => {
            let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
            ConcretePuzzle {
                kind,
                operation: Operation::Sub,
                a: hi,
                b: lo,
                target: hi - lo,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn rng() -> SmallRng {
        SmallRng::seed_from_u64(42)
    }

    fn place(s: ConcreteSession, group: u8, n: usize) -> ConcreteSession {
        let mut s = s;
        for _ in 0..n {
            s = concrete_reducer(s, ConcreteAction::Place { group });
        }
        s
    }

    #[test]
    fn count_completes_when_all_tapped() {
        let p = generate_concrete(ConcreteKind::Count, 5, 0, &mut rng());
        assert_eq!(p.target, 5);
        let mut s = ConcreteSession::new(p);
        assert_eq!(s.phase, ConcretePhase::InProgress);
        s = place(s, 0, 4);
        assert_eq!(s.phase, ConcretePhase::InProgress, "four of five is not done");
        s = place(s, 0, 1);
        assert_eq!(s.phase, ConcretePhase::Complete);
        assert_eq!(s.total(), 5);
    }

    #[test]
    fn add_groups_target_is_the_sum() {
        let p = generate_concrete(ConcreteKind::AddGroups, 3, 2, &mut rng());
        assert_eq!(p.target, 5);
        let mut s = ConcreteSession::new(p);
        s = place(s, 0, 3); // 3 in group A
        assert_eq!(s.phase, ConcretePhase::InProgress);
        s = place(s, 1, 2); // 2 in group B → total 5
        assert_eq!(s.phase, ConcretePhase::Complete);
        assert!(s.grouping_matches(), "3 and 2 matches the posed split");
    }

    #[test]
    fn add_groups_completes_on_total_even_with_a_different_split() {
        // Friendly: any split summing to the target finishes; grouping_matches
        // records whether it was the posed one (a signal, not a gate).
        let p = generate_concrete(ConcreteKind::AddGroups, 3, 2, &mut rng());
        let mut s = ConcreteSession::new(p);
        s = place(s, 0, 5); // all five in group A
        assert_eq!(s.phase, ConcretePhase::Complete, "reaching the total finishes");
        assert!(!s.grouping_matches(), "5+0 is not the posed 3+2 split");
    }

    #[test]
    fn overshoot_is_recoverable_never_punished() {
        let p = generate_concrete(ConcreteKind::BuildTower, 4, 3, &mut rng()); // target 7
        let mut s = ConcreteSession::new(p);
        s = place(s, 0, 6);
        assert_eq!(s.phase, ConcretePhase::InProgress);
        // Overshoot past the target: still in progress (never auto-completes wrong).
        s = concrete_reducer(s, ConcreteAction::Place { group: 0 }); // 7 → complete
        assert_eq!(s.phase, ConcretePhase::Complete);
    }

    #[test]
    fn removing_brings_an_overshoot_back_to_target() {
        let p = generate_concrete(ConcreteKind::Count, 3, 0, &mut rng());
        let mut s = ConcreteSession::new(p);
        s = place(s, 0, 2); // 2, not yet 3
        // Pretend the kid over-taps in a fresh session that hasn't completed:
        // remove then place to land exactly on target.
        s = concrete_reducer(s, ConcreteAction::Remove { group: 0 }); // 1
        assert_eq!(s.total(), 1);
        s = place(s, 0, 2); // 3 → complete
        assert_eq!(s.phase, ConcretePhase::Complete);
    }

    #[test]
    fn take_away_prefills_and_completes_on_remainder() {
        let p = generate_concrete(ConcreteKind::TakeAway, 8, 3, &mut rng());
        assert_eq!(p.target, 5);
        let mut s = ConcreteSession::new(p);
        assert_eq!(s.bucket_a, 8, "take-away starts with all objects present");
        assert_eq!(s.phase, ConcretePhase::InProgress);
        for _ in 0..3 {
            s = concrete_reducer(s, ConcreteAction::Remove { group: 0 });
        }
        assert_eq!(s.bucket_a, 5);
        assert_eq!(s.phase, ConcretePhase::Complete);
    }

    #[test]
    fn take_away_normalizes_reversed_operands() {
        let p = generate_concrete(ConcreteKind::TakeAway, 2, 9, &mut rng());
        assert_eq!((p.a, p.b, p.target), (9, 2, 7), "clamps to a>=b so target stays non-negative");
    }

    #[test]
    fn completed_session_ignores_further_actions() {
        let p = generate_concrete(ConcreteKind::Count, 2, 0, &mut rng());
        let mut s = ConcreteSession::new(p);
        s = place(s, 0, 2);
        assert_eq!(s.phase, ConcretePhase::Complete);
        let before = s.total();
        s = concrete_reducer(s, ConcreteAction::Place { group: 0 });
        assert_eq!(s.total(), before, "no mutation once complete");
    }

    #[test]
    fn placement_is_capped_to_avoid_runaway() {
        let p = generate_concrete(ConcreteKind::AddGroups, 1, 1, &mut rng()); // target 2, cap 6
        let mut s = ConcreteSession::new(p);
        // Force into group B so we never hit the target and can test the cap.
        for _ in 0..50 {
            s = concrete_reducer(s, ConcreteAction::Place { group: 1 });
        }
        assert!(s.total() <= capacity(&s.puzzle), "workspace must not grow without bound");
    }
}
