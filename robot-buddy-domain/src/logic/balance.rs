//! Balance-scale puzzles — visual algebra. Pure logic, no rendering.
//!
//! A scale must balance: the left side and the right side are equal. Some
//! blocks show a number; one or more are the mystery block `?`. The kid finds
//! the value that makes the scale level. This is number bonds (and early
//! algebra) as a physical metaphor — the scale tipping is the feedback, not a
//! red X. Broccoli-free: balancing a scale is the game, not a wrapped quiz.
//!
//! Public surface (mirrors `kenken` / `patterns`):
//!   - `generate_balance(level, rng)` / `generate_for_band(band, rng)`
//!   - `BalanceSession::new(puzzle)` → fresh session in InProgress
//!   - `balance_reducer(session, action)` → new session
//!
//! Every `Unknown` block on a puzzle holds the SAME value (`correct_answer`), so
//! `? + ? = 8` means each `?` is 4. One action: `Guess { value }` — the kid
//! picks a number; a wrong guess tips the scale (recorded for the renderer) and
//! lets them try again.

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::types::Operation;

// ─── Types ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum BalanceItem {
    Known { value: i32 },
    Unknown,
    Op { op: Operation },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalancePuzzle {
    pub left_side: Vec<BalanceItem>,
    pub right_side: Vec<BalanceItem>,
    /// The value every `Unknown` block stands for.
    pub correct_answer: i32,
    /// Multiple-choice options, already shuffled. Always contains the answer.
    pub choices: Vec<i32>,
}

impl BalancePuzzle {
    pub fn is_correct(&self, value: i32) -> bool {
        value == self.correct_answer
    }

    /// Weight on a side when each `Unknown` takes `unknown`. With `unknown ==
    /// correct_answer` the two sides are equal by construction.
    pub fn weigh(side: &[BalanceItem], unknown: i32) -> i32 {
        eval_side(side, unknown)
    }

    /// How the scale sits for a given guess: 0 balanced, <0 left-heavy,
    /// >0 right-heavy. Drives the renderer's tilt.
    pub fn tilt(&self, guess: i32) -> i32 {
        eval_side(&self.right_side, guess) - eval_side(&self.left_side, guess)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BalancePhase {
    InProgress,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSession {
    pub puzzle: BalancePuzzle,
    pub phase: BalancePhase,
    pub attempts: u8,
    /// The most recent wrong guess, for the renderer's tip-and-settle animation.
    /// Cleared on a correct guess.
    pub last_wrong: Option<i32>,
}

impl BalanceSession {
    pub fn new(puzzle: BalancePuzzle) -> Self {
        BalanceSession {
            puzzle,
            phase: BalancePhase::InProgress,
            attempts: 0,
            last_wrong: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum BalanceAction {
    Guess { value: i32 },
}

// ─── Evaluation ─────────────────────────────────────────

fn operand_value(item: &BalanceItem, unknown: i32) -> i32 {
    match item {
        BalanceItem::Known { value } => *value,
        BalanceItem::Unknown => unknown,
        BalanceItem::Op { .. } => 0, // operators carry no weight on their own
    }
}

/// Evaluate one pan. A side is either a single operand `[x]` or a binary
/// expression `[a, Op, b]`. That covers every template we generate; anything
/// longer is a generation bug and evaluates by folding left-to-right.
fn eval_side(side: &[BalanceItem], unknown: i32) -> i32 {
    match side {
        [a] => operand_value(a, unknown),
        [a, BalanceItem::Op { op }, b] => apply(operand_value(a, unknown), *op, operand_value(b, unknown)),
        _ => {
            // Defensive left-fold: operand, op, operand, op, operand, ...
            let mut acc: Option<i32> = None;
            let mut pending: Option<Operation> = None;
            for item in side {
                match item {
                    BalanceItem::Op { op } => pending = Some(*op),
                    operand => {
                        let v = operand_value(operand, unknown);
                        acc = Some(match (acc, pending.take()) {
                            (Some(a), Some(op)) => apply(a, op, v),
                            _ => v,
                        });
                    }
                }
            }
            acc.unwrap_or(0)
        }
    }
}

fn apply(a: i32, op: Operation, b: i32) -> i32 {
    match op {
        Operation::Add => a + b,
        Operation::Sub => a - b,
        Operation::Multiply => a * b,
        Operation::Divide => if b != 0 { a / b } else { 0 },
        Operation::NumberBond => a + b,
    }
}

// ─── Reducer ────────────────────────────────────────────

pub fn balance_reducer(state: BalanceSession, action: BalanceAction) -> BalanceSession {
    if state.phase == BalancePhase::Complete {
        return state;
    }
    match action {
        BalanceAction::Guess { value } => {
            let attempts = state.attempts.saturating_add(1);
            if state.puzzle.is_correct(value) {
                BalanceSession {
                    phase: BalancePhase::Complete,
                    attempts,
                    last_wrong: None,
                    ..state
                }
            } else {
                // Wrong guess: the scale tips, the kid tries again. No punishment.
                BalanceSession {
                    attempts,
                    last_wrong: Some(value),
                    ..state
                }
            }
        }
    }
}

// ─── Generation ─────────────────────────────────────────

fn known(v: i32) -> BalanceItem {
    BalanceItem::Known { value: v }
}
fn op(o: Operation) -> BalanceItem {
    BalanceItem::Op { op: o }
}

/// Build choices: the answer plus nearby positive distractors, shuffled.
fn make_choices(answer: i32, rng: &mut impl Rng) -> Vec<i32> {
    let mut choices = vec![answer];
    for d in [answer - 1, answer + 1, answer - 2, answer + 2, answer + 3] {
        if choices.len() >= 4 {
            break;
        }
        if d > 0 && !choices.contains(&d) {
            choices.push(d);
        }
    }
    choices.shuffle(rng);
    choices
}

fn assemble(left: Vec<BalanceItem>, right: Vec<BalanceItem>, answer: i32, rng: &mut impl Rng) -> BalancePuzzle {
    let choices = make_choices(answer, rng);
    let puzzle = BalancePuzzle { left_side: left, right_side: right, correct_answer: answer, choices };
    debug_assert_eq!(
        eval_side(&puzzle.left_side, answer),
        eval_side(&puzzle.right_side, answer),
        "generated balance puzzle must balance at the answer",
    );
    puzzle
}

/// Generate a balance puzzle for a difficulty level (1 = easiest).
///
/// - L1: `a + ? = c`            (single unknown, addition)
/// - L2: `? - a = c` / `a + ? = c` with the unknown on either side, +/-
/// - L3: `? + ? = c`            (two equal unknowns)
/// - L4: `n × ? = c`            (multiplication)
pub fn generate_balance(level: u8, rng: &mut impl Rng) -> BalancePuzzle {
    match level.max(1) {
        1 => {
            let answer = rng.gen_range(1..=9);
            let a = rng.gen_range(1..=9);
            let c = a + answer;
            if rng.gen_bool(0.5) {
                // a + ? = c
                assemble(vec![known(a), op(Operation::Add), BalanceItem::Unknown], vec![known(c)], answer, rng)
            } else {
                // ? + a = c
                assemble(vec![BalanceItem::Unknown, op(Operation::Add), known(a)], vec![known(c)], answer, rng)
            }
        }
        2 => {
            let answer = rng.gen_range(1..=9);
            match rng.gen_range(0..3) {
                0 if answer >= 2 => {
                    // ? - a = c with 1 <= a <= answer-1, so c = answer - a >= 1.
                    let a = rng.gen_range(1..answer); // 1..=answer-1
                    let c = answer - a;
                    assemble(vec![BalanceItem::Unknown, op(Operation::Sub), known(a)], vec![known(c)], answer, rng)
                }
                0 => {
                    // answer == 1: no room for `? - a = c`, fall back to a + ? = c.
                    let a = rng.gen_range(1..=9);
                    let c = a + answer;
                    assemble(vec![known(a), op(Operation::Add), BalanceItem::Unknown], vec![known(c)], answer, rng)
                }
                1 => {
                    // a - ? = c  (a > answer)
                    let c = rng.gen_range(1..=9);
                    let a = answer + c;
                    assemble(vec![known(a), op(Operation::Sub), BalanceItem::Unknown], vec![known(c)], answer, rng)
                }
                _ => {
                    // c = a + ?  (unknown on the right pan)
                    let a = rng.gen_range(1..=9);
                    let c = a + answer;
                    assemble(vec![known(c)], vec![known(a), op(Operation::Add), BalanceItem::Unknown], answer, rng)
                }
            }
        }
        3 => {
            // ? + ? = c, both unknowns equal → c = 2·answer
            let answer = rng.gen_range(1..=8);
            let c = answer * 2;
            assemble(
                vec![BalanceItem::Unknown, op(Operation::Add), BalanceItem::Unknown],
                vec![known(c)],
                answer,
                rng,
            )
        }
        _ => {
            // n × ? = c
            let answer = rng.gen_range(2..=6);
            let n = rng.gen_range(2..=5);
            let c = n * answer;
            if rng.gen_bool(0.5) {
                assemble(vec![known(n), op(Operation::Multiply), BalanceItem::Unknown], vec![known(c)], answer, rng)
            } else {
                assemble(vec![BalanceItem::Unknown, op(Operation::Multiply), known(n)], vec![known(c)], answer, rng)
            }
        }
    }
}

/// Map the kid's arithmetic band to a balance difficulty level, mirroring the
/// spec: single unknown early, either-side ± next, two unknowns / × later.
pub fn balance_level_for_band(band: u8) -> u8 {
    match band {
        0..=2 => 1,
        3..=4 => 2,
        5..=8 => 3,
        _ => 4,
    }
}

pub fn generate_for_band(band: u8, rng: &mut impl Rng) -> BalancePuzzle {
    generate_balance(balance_level_for_band(band), rng)
}

// ─── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn count_unknowns(side: &[BalanceItem]) -> usize {
        side.iter().filter(|i| matches!(i, BalanceItem::Unknown)).count()
    }

    #[test]
    fn every_level_balances_at_the_answer() {
        for level in 1..=4u8 {
            for seed in 0..500u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_balance(level, &mut r);
                let l = BalancePuzzle::weigh(&p.left_side, p.correct_answer);
                let rt = BalancePuzzle::weigh(&p.right_side, p.correct_answer);
                assert_eq!(l, rt, "level {level} seed {seed}: scale must balance at the answer");
                assert!(p.correct_answer > 0, "answer must be positive");
            }
        }
    }

    #[test]
    fn no_known_block_or_choice_is_negative() {
        for level in 1..=4u8 {
            for seed in 0..500u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_balance(level, &mut r);
                for side in [&p.left_side, &p.right_side] {
                    for item in side {
                        if let BalanceItem::Known { value } = item {
                            assert!(*value > 0, "level {level} seed {seed}: known {value} not positive");
                        }
                    }
                }
                for c in &p.choices {
                    assert!(*c > 0, "level {level} seed {seed}: choice {c} not positive");
                }
            }
        }
    }

    #[test]
    fn choices_contain_answer_and_are_distinct() {
        for level in 1..=4u8 {
            for seed in 0..200u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_balance(level, &mut r);
                assert!(p.choices.contains(&p.correct_answer));
                let mut sorted = p.choices.clone();
                let before = sorted.len();
                sorted.sort();
                sorted.dedup();
                assert_eq!(sorted.len(), before, "choices must be distinct: {:?}", p.choices);
            }
        }
    }

    #[test]
    fn level_three_has_two_equal_unknowns() {
        let mut r = SmallRng::seed_from_u64(1);
        let p = generate_balance(3, &mut r);
        // Two unknown blocks total; both equal the single answer value.
        let total = count_unknowns(&p.left_side) + count_unknowns(&p.right_side);
        assert_eq!(total, 2, "level 3 is ? + ? = c");
        // The two-unknown side sums to twice the answer.
        assert_eq!(BalancePuzzle::weigh(&p.left_side, p.correct_answer), p.correct_answer * 2);
    }

    #[test]
    fn single_unknown_levels_have_exactly_one_unknown() {
        for level in [1u8, 2, 4] {
            for seed in 0..100u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_balance(level, &mut r);
                let total = count_unknowns(&p.left_side) + count_unknowns(&p.right_side);
                assert_eq!(total, 1, "level {level} seed {seed}: expected one unknown");
            }
        }
    }

    #[test]
    fn tilt_is_balanced_only_at_the_answer() {
        let mut r = SmallRng::seed_from_u64(7);
        let p = generate_balance(2, &mut r);
        assert_eq!(p.tilt(p.correct_answer), 0, "balanced at the answer");
        // A guess one off should tip the scale.
        assert_ne!(p.tilt(p.correct_answer + 1), 0, "a wrong guess tips the scale");
    }

    #[test]
    fn correct_guess_completes() {
        let mut r = SmallRng::seed_from_u64(3);
        let p = generate_balance(1, &mut r);
        let answer = p.correct_answer;
        let s = BalanceSession::new(p);
        let s = balance_reducer(s, BalanceAction::Guess { value: answer });
        assert_eq!(s.phase, BalancePhase::Complete);
        assert_eq!(s.attempts, 1);
        assert_eq!(s.last_wrong, None);
    }

    #[test]
    fn wrong_guess_tips_and_allows_retry() {
        let mut r = SmallRng::seed_from_u64(3);
        let p = generate_balance(1, &mut r);
        let answer = p.correct_answer;
        let s = BalanceSession::new(p);
        let s = balance_reducer(s, BalanceAction::Guess { value: answer + 1 });
        assert_eq!(s.phase, BalancePhase::InProgress, "wrong guess never locks the puzzle");
        assert_eq!(s.last_wrong, Some(answer + 1));
        let s = balance_reducer(s, BalanceAction::Guess { value: answer });
        assert_eq!(s.phase, BalancePhase::Complete);
        assert_eq!(s.attempts, 2);
        assert_eq!(s.last_wrong, None);
    }

    #[test]
    fn complete_session_ignores_further_guesses() {
        let mut r = SmallRng::seed_from_u64(3);
        let p = generate_balance(1, &mut r);
        let answer = p.correct_answer;
        let s = BalanceSession::new(p);
        let s = balance_reducer(s, BalanceAction::Guess { value: answer });
        let s = balance_reducer(s, BalanceAction::Guess { value: answer + 1 });
        assert_eq!(s.attempts, 1, "no mutation once complete");
        assert_eq!(s.phase, BalancePhase::Complete);
    }

    #[test]
    fn band_maps_to_increasing_levels() {
        assert_eq!(balance_level_for_band(1), 1);
        assert_eq!(balance_level_for_band(3), 2);
        assert_eq!(balance_level_for_band(6), 3);
        assert_eq!(balance_level_for_band(10), 4);
        // Multiplication only appears for fluent bands.
        let mut r = SmallRng::seed_from_u64(0);
        let p = generate_for_band(10, &mut r);
        let has_mul = p.left_side.iter().chain(&p.right_side)
            .any(|i| matches!(i, BalanceItem::Op { op: Operation::Multiply }));
        assert!(has_mul, "band 10 balance should use multiplication");
    }
}
