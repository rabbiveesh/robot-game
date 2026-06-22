//! Base-ten block trading — a representational (CRA stage 2) manipulative for
//! multi-digit addition and subtraction. Pure logic, no rendering.
//!
//! Place value made physical: tens are rods, ones are cubes. To add 28 + 15 the
//! kid pools the blocks (13 ones, 3 tens) and TRADES ten ones for one ten — that
//! trade IS carrying. To subtract 32 − 15 the kid BREAKS a ten into ten ones
//! when there aren't enough to take from — that break IS borrowing. The quantity
//! is conserved through every legal trade; illegal trades are simply ignored
//! (never punished).
//!
//! Public surface mirrors the other logic modules:
//!   - `generate_base_ten(a, b, op, &mut impl Rng)`
//!   - `BaseTenSession::new(puzzle)` → fresh session
//!   - `base_ten_reducer(session, action)` → new session

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::types::Operation;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseTenPuzzle {
    pub a: u8,
    pub b: u8,
    pub operation: Operation,
    /// The quantity the workspace must represent in canonical form (ones < 10).
    pub target: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseTenPhase {
    InProgress,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseTenSession {
    pub puzzle: BaseTenPuzzle,
    pub ones: u8,
    pub tens: u8,
    pub phase: BaseTenPhase,
}

impl BaseTenSession {
    pub fn new(puzzle: BaseTenPuzzle) -> Self {
        // Addition pools both numbers' blocks (so the value already equals a+b
        // and the only task is regrouping). Subtraction starts with `a`'s
        // blocks, to be removed down to a-b.
        let (ones, tens) = match puzzle.operation {
            Operation::Sub => (puzzle.a % 10, puzzle.a / 10),
            _ => ((puzzle.a % 10) + (puzzle.b % 10), (puzzle.a / 10) + (puzzle.b / 10)),
        };
        let mut s = BaseTenSession { ones, tens, phase: BaseTenPhase::InProgress, puzzle };
        if s.is_solved() {
            s.phase = BaseTenPhase::Complete;
        }
        s
    }

    pub fn value(&self) -> u16 {
        self.tens as u16 * 10 + self.ones as u16
    }

    /// Solved when the workspace represents the answer AND is in canonical form
    /// (fewer than ten loose ones — i.e. all carrying/borrowing resolved).
    fn is_solved(&self) -> bool {
        self.value() == self.puzzle.target && self.ones < 10
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum BaseTenAction {
    /// Carry: trade ten loose ones for one ten-rod. Legal only with >= 10 ones.
    TradeUp,
    /// Borrow: break a ten-rod into ten ones. Legal only with >= 1 ten.
    BreakDown,
    /// Subtraction: remove one one-cube. Legal only with >= 1 one.
    RemoveOne,
    /// Subtraction: remove one ten-rod. Legal only with >= 1 ten.
    RemoveTen,
}

pub fn base_ten_reducer(state: BaseTenSession, action: BaseTenAction) -> BaseTenSession {
    if state.phase == BaseTenPhase::Complete {
        return state;
    }
    let is_sub = state.puzzle.operation == Operation::Sub;
    let mut next = state.clone();
    match action {
        BaseTenAction::TradeUp => {
            if next.ones < 10 {
                return state; // illegal — no-op, not a penalty
            }
            next.ones -= 10;
            next.tens = next.tens.saturating_add(1);
        }
        BaseTenAction::BreakDown => {
            if next.tens < 1 {
                return state;
            }
            next.tens -= 1;
            next.ones = next.ones.saturating_add(10);
        }
        BaseTenAction::RemoveOne => {
            if !is_sub || next.ones < 1 {
                return state;
            }
            next.ones -= 1;
        }
        BaseTenAction::RemoveTen => {
            if !is_sub || next.tens < 1 {
                return state;
            }
            next.tens -= 1;
        }
    }
    if next.is_solved() {
        next.phase = BaseTenPhase::Complete;
    }
    next
}

/// Build a base-ten puzzle from explicit operands (two-digit friendly).
/// Addition target is `a+b`; subtraction normalizes so `a >= b` and targets
/// `a-b`.
pub fn generate_base_ten(a: u8, b: u8, op: Operation, _rng: &mut impl Rng) -> BaseTenPuzzle {
    match op {
        Operation::Sub => {
            let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
            BaseTenPuzzle { a: hi, b: lo, operation: Operation::Sub, target: (hi - lo) as u16 }
        }
        _ => BaseTenPuzzle { a, b, operation: Operation::Add, target: a as u16 + b as u16 },
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

    #[test]
    fn addition_without_carry_starts_solved() {
        // 23 + 14: ones 3+4=7 (<10), tens 2+1=3 → already canonical 37.
        let p = generate_base_ten(23, 14, Operation::Add, &mut rng());
        assert_eq!(p.target, 37);
        let s = BaseTenSession::new(p);
        assert_eq!((s.tens, s.ones), (3, 7));
        assert_eq!(s.phase, BaseTenPhase::Complete);
    }

    #[test]
    fn addition_with_carry_requires_a_trade_up() {
        // 28 + 15: ones 8+5=13, tens 2+1=3 → must carry.
        let p = generate_base_ten(28, 15, Operation::Add, &mut rng());
        assert_eq!(p.target, 43);
        let s = BaseTenSession::new(p);
        assert_eq!((s.tens, s.ones), (3, 13));
        assert_eq!(s.phase, BaseTenPhase::InProgress, "13 loose ones is not canonical");
        let s = base_ten_reducer(s, BaseTenAction::TradeUp);
        assert_eq!((s.tens, s.ones), (4, 3));
        assert_eq!(s.value(), 43);
        assert_eq!(s.phase, BaseTenPhase::Complete);
    }

    #[test]
    fn trade_up_conserves_quantity() {
        let p = generate_base_ten(28, 15, Operation::Add, &mut rng());
        let s = BaseTenSession::new(p);
        let before = s.value();
        let s = base_ten_reducer(s, BaseTenAction::TradeUp);
        assert_eq!(s.value(), before, "carrying must not change the total");
    }

    #[test]
    fn illegal_trade_up_is_a_noop() {
        // An in-progress subtraction with fewer than ten ones: TradeUp has
        // nothing to carry and must leave the workspace untouched (no penalty).
        let p = generate_base_ten(32, 15, Operation::Sub, &mut rng()); // ones 2, tens 3
        let s = BaseTenSession::new(p);
        assert_eq!(s.phase, BaseTenPhase::InProgress);
        let after = base_ten_reducer(s, BaseTenAction::TradeUp);
        assert_eq!((after.tens, after.ones), (3, 2), "TradeUp with <10 ones is a no-op");
        assert_eq!(after.phase, BaseTenPhase::InProgress);
    }

    #[test]
    fn subtraction_without_borrow() {
        // 38 - 12: ones 8-2, tens 3-1 → 26, no borrow.
        let p = generate_base_ten(38, 12, Operation::Sub, &mut rng());
        assert_eq!(p.target, 26);
        let s = BaseTenSession::new(p);
        assert_eq!((s.tens, s.ones), (3, 8));
        // Remove 2 ones and 1 ten.
        let s = base_ten_reducer(s, BaseTenAction::RemoveOne);
        let s = base_ten_reducer(s, BaseTenAction::RemoveOne);
        let s = base_ten_reducer(s, BaseTenAction::RemoveTen);
        assert_eq!(s.value(), 26);
        assert_eq!(s.phase, BaseTenPhase::Complete);
    }

    #[test]
    fn subtraction_with_borrow_uses_break_down() {
        // 32 - 15: ones 2 < 5, must break a ten to borrow.
        let p = generate_base_ten(32, 15, Operation::Sub, &mut rng());
        assert_eq!(p.target, 17);
        let mut s = BaseTenSession::new(p);
        assert_eq!((s.tens, s.ones), (3, 2));
        // Can't remove 5 ones yet. Borrow: break a ten → 2 tens, 12 ones.
        s = base_ten_reducer(s, BaseTenAction::BreakDown);
        assert_eq!((s.tens, s.ones), (2, 12));
        // Now remove 5 ones and 1 ten → 1 ten, 7 ones = 17.
        for _ in 0..5 {
            s = base_ten_reducer(s, BaseTenAction::RemoveOne);
        }
        s = base_ten_reducer(s, BaseTenAction::RemoveTen);
        assert_eq!(s.value(), 17);
        assert_eq!(s.phase, BaseTenPhase::Complete);
    }

    #[test]
    fn break_down_conserves_quantity_and_is_gated() {
        let p = generate_base_ten(32, 15, Operation::Sub, &mut rng());
        let s = BaseTenSession::new(p);
        let before = s.value();
        let s = base_ten_reducer(s, BaseTenAction::BreakDown);
        assert_eq!(s.value(), before, "borrowing must not change the total");
        // Drain tens, then BreakDown is illegal (no rods left) → no-op.
        let mut s = s;
        while s.tens > 0 {
            s = base_ten_reducer(s, BaseTenAction::BreakDown);
            if s.phase == BaseTenPhase::Complete { break; }
        }
    }

    #[test]
    fn remove_actions_are_inert_for_addition() {
        let p = generate_base_ten(19, 15, Operation::Add, &mut rng()); // in progress (14 ones)
        let s = BaseTenSession::new(p);
        let before = (s.tens, s.ones);
        let s = base_ten_reducer(s, BaseTenAction::RemoveOne);
        assert_eq!((s.tens, s.ones), before, "addition workspace can't lose blocks");
    }

    #[test]
    fn subtraction_normalizes_reversed_operands() {
        let p = generate_base_ten(15, 32, Operation::Sub, &mut rng());
        assert_eq!((p.a, p.b, p.target), (32, 15, 17));
    }
}
