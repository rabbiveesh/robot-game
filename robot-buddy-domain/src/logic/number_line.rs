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
