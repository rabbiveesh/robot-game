//! Number-line jumps — a representational (CRA stage 2) manipulative. Pure
//! logic, no rendering.
//!
//! A character stands on a number line at `start`. The kid hops forward (for
//! addition) or backward (for subtraction) to land on the answer. Watching the
//! hop land on 8 when you add 5 to 3 IS the arithmetic — no multiple-choice
//! quiz. Overshooting is fine: jump back. Going below zero just stops at zero.
//!
//! Public surface mirrors the other logic modules:
//!   - `generate_number_line(a, b, op, &mut impl Rng)`
//!   - `NumberLineSession::new(puzzle)` → fresh session
//!   - `number_line_reducer(session, action)` → new session

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::types::Operation;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberLinePuzzle {
    pub start: u8,
    pub target: u8,
    pub operation: Operation,
    /// Largest tick on the rendered line; positions clamp to `0..=max`.
    pub max: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumberLinePhase {
    InProgress,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberLineSession {
    pub puzzle: NumberLinePuzzle,
    pub position: u8,
    pub jumps: u8,
    pub phase: NumberLinePhase,
}

impl NumberLineSession {
    pub fn new(puzzle: NumberLinePuzzle) -> Self {
        let phase = if puzzle.start == puzzle.target {
            NumberLinePhase::Complete
        } else {
            NumberLinePhase::InProgress
        };
        NumberLineSession { position: puzzle.start, jumps: 0, phase, puzzle }
    }

    /// Minimum unit steps from start to target — the "count-on" optimum. The
    /// game compares the actual `jumps`/steps taken against this to silently
    /// read whether the kid moved efficiently (counted on) vs. wandered.
    pub fn optimal_jumps(&self) -> u8 {
        abs_diff(self.puzzle.start, self.puzzle.target)
    }

    /// Steps from the current position to the target (0 = landed on it). Reads
    /// the same whether the kid is short of the target or has overshot it.
    pub fn distance_to_target(&self) -> u8 {
        abs_diff(self.position, self.puzzle.target)
    }
}

fn abs_diff(a: u8, b: u8) -> u8 {
    if a >= b { a - b } else { b - a }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum NumberLineAction {
    JumpForward { n: u8 },
    JumpBackward { n: u8 },
}

pub fn number_line_reducer(state: NumberLineSession, action: NumberLineAction) -> NumberLineSession {
    if state.phase == NumberLinePhase::Complete {
        return state;
    }
    let mut next = state.clone();
    match action {
        NumberLineAction::JumpForward { n } => {
            if n == 0 {
                return state;
            }
            next.position = next.position.saturating_add(n).min(next.puzzle.max);
            next.jumps = next.jumps.saturating_add(1);
        }
        NumberLineAction::JumpBackward { n } => {
            if n == 0 {
                return state;
            }
            next.position = next.position.saturating_sub(n); // clamps at 0
            next.jumps = next.jumps.saturating_add(1);
        }
    }
    if next.position == next.puzzle.target {
        next.phase = NumberLinePhase::Complete;
    }
    next
}

/// Build a number-line puzzle from explicit operands. Addition jumps forward
/// from `a` to `a+b`; subtraction jumps backward from the larger operand to the
/// difference (operands are clamped so the target is never negative).
pub fn generate_number_line(a: u8, b: u8, op: Operation, _rng: &mut impl Rng) -> NumberLinePuzzle {
    match op {
        Operation::Sub => {
            let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
            NumberLinePuzzle { start: hi, target: hi - lo, operation: Operation::Sub, max: hi }
        }
        _ => {
            let target = a.saturating_add(b);
            // A little headroom past the target so the kid can overshoot + correct.
            let max = target.saturating_add(3);
            NumberLinePuzzle { start: a, target, operation: Operation::Add, max }
        }
    }
}

/// Build a raw "go to mark N" puzzle — a target on a `0..=max` line that isn't
/// tied to an a+b operation. Used by the embodied number line: hop to stone N,
/// or set the dive gauge to depth N. `start` is where the token begins (often
/// 0), `target` the goal; `max` is clamped to at least the target (with a touch
/// of overshoot headroom) so the line always reaches it. The stored `operation`
/// is informational only (forward vs. backward).
pub fn generate_target(start: u8, target: u8, max: u8) -> NumberLinePuzzle {
    let operation = if target >= start { Operation::Add } else { Operation::Sub };
    let max = max.max(target.saturating_add(if target >= start { 2 } else { 0 })).max(start);
    NumberLinePuzzle { start, target, operation, max }
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
    fn forward_jump_to_target_completes_addition() {
        let p = generate_number_line(3, 5, Operation::Add, &mut rng());
        assert_eq!((p.start, p.target), (3, 8));
        let s = NumberLineSession::new(p);
        let s = number_line_reducer(s, NumberLineAction::JumpForward { n: 5 });
        assert_eq!(s.position, 8);
        assert_eq!(s.phase, NumberLinePhase::Complete);
        assert_eq!(s.jumps, 1);
    }

    #[test]
    fn unit_jumps_also_reach_the_target() {
        let p = generate_number_line(3, 5, Operation::Add, &mut rng());
        let mut s = NumberLineSession::new(p);
        for _ in 0..5 {
            s = number_line_reducer(s, NumberLineAction::JumpForward { n: 1 });
        }
        assert_eq!(s.phase, NumberLinePhase::Complete);
        assert_eq!(s.jumps, 5);
    }

    #[test]
    fn overshoot_is_recoverable() {
        let p = generate_number_line(3, 5, Operation::Add, &mut rng()); // target 8, max 11
        let s = NumberLineSession::new(p);
        let s = number_line_reducer(s, NumberLineAction::JumpForward { n: 7 }); // 10, not target
        assert_eq!(s.phase, NumberLinePhase::InProgress);
        assert_eq!(s.position, 10);
        let s = number_line_reducer(s, NumberLineAction::JumpBackward { n: 2 }); // 8
        assert_eq!(s.phase, NumberLinePhase::Complete);
    }

    #[test]
    fn forward_clamps_at_max() {
        let p = generate_number_line(3, 5, Operation::Add, &mut rng()); // max 11
        let s = NumberLineSession::new(p);
        let s = number_line_reducer(s, NumberLineAction::JumpForward { n: 100 });
        assert_eq!(s.position, 11, "can't jump past the end of the line");
        assert_eq!(s.phase, NumberLinePhase::InProgress);
    }

    #[test]
    fn backward_jump_completes_subtraction() {
        let p = generate_number_line(9, 4, Operation::Sub, &mut rng());
        assert_eq!((p.start, p.target), (9, 5));
        let s = NumberLineSession::new(p);
        let s = number_line_reducer(s, NumberLineAction::JumpBackward { n: 4 });
        assert_eq!(s.position, 5);
        assert_eq!(s.phase, NumberLinePhase::Complete);
    }

    #[test]
    fn backward_clamps_at_zero() {
        let p = generate_number_line(9, 4, Operation::Sub, &mut rng());
        let s = NumberLineSession::new(p);
        let s = number_line_reducer(s, NumberLineAction::JumpBackward { n: 50 });
        assert_eq!(s.position, 0, "never goes below zero");
    }

    #[test]
    fn subtraction_normalizes_reversed_operands() {
        let p = generate_number_line(2, 9, Operation::Sub, &mut rng());
        assert_eq!((p.start, p.target), (9, 7));
    }

    #[test]
    fn zero_jump_is_ignored() {
        let p = generate_number_line(3, 5, Operation::Add, &mut rng());
        let s = NumberLineSession::new(p);
        let s = number_line_reducer(s, NumberLineAction::JumpForward { n: 0 });
        assert_eq!(s.jumps, 0);
        assert_eq!(s.position, 3);
    }

    // ── Embodied number line: raw "go to mark N" target + count-on assessment ──

    #[test]
    fn target_puzzle_walks_forward_to_a_raw_mark() {
        // "Hop to stone 5" — start at 0, no a+b operation involved.
        let p = generate_target(0, 5, 8);
        assert_eq!((p.start, p.target, p.max), (0, 5, 8));
        let mut s = NumberLineSession::new(p);
        assert_eq!(s.optimal_jumps(), 5);
        assert_eq!(s.distance_to_target(), 5);
        for _ in 0..5 {
            s = number_line_reducer(s, NumberLineAction::JumpForward { n: 1 });
        }
        assert_eq!(s.phase, NumberLinePhase::Complete);
        assert_eq!(s.distance_to_target(), 0);
    }

    #[test]
    fn target_puzzle_can_descend_backward() {
        // A gauge that reads downward: from 7 to 3.
        let p = generate_target(7, 3, 10);
        let s = NumberLineSession::new(p);
        assert_eq!(s.optimal_jumps(), 4);
        let s = number_line_reducer(s, NumberLineAction::JumpBackward { n: 4 });
        assert_eq!(s.position, 3);
        assert_eq!(s.phase, NumberLinePhase::Complete);
    }

    #[test]
    fn generate_target_clamps_max_to_at_least_the_target() {
        let p = generate_target(0, 9, 4); // max too small
        assert!(p.max >= p.target, "max must reach the target");
    }

    #[test]
    fn distance_to_target_reflects_overshoot() {
        let p = generate_target(0, 5, 9);
        let s = NumberLineSession::new(p);
        let s = number_line_reducer(s, NumberLineAction::JumpForward { n: 7 }); // overshoot to 7
        assert_eq!(s.distance_to_target(), 2, "two past the target");
        assert_eq!(s.phase, NumberLinePhase::InProgress);
    }

    #[test]
    fn completed_session_ignores_further_jumps() {
        let p = generate_number_line(3, 5, Operation::Add, &mut rng());
        let s = NumberLineSession::new(p);
        let s = number_line_reducer(s, NumberLineAction::JumpForward { n: 5 });
        assert_eq!(s.phase, NumberLinePhase::Complete);
        let s = number_line_reducer(s, NumberLineAction::JumpForward { n: 1 });
        assert_eq!(s.position, 8, "no movement once complete");
    }
}
