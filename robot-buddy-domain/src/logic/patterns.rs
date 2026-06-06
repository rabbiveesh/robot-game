//! Pattern-sequence puzzles. Pure logic — no rendering, no input.
//!
//! "What comes next?" The kid continues a sequence: alternating sprites for a
//! 4-year-old, skip-counting and squares for a 7-year-old. Pattern recognition
//! is the gameplay, not a quiz wrapped around arithmetic — it passes the
//! broccoli test by construction (nobody looks at 🔴🔵🔴🔵 and thinks "math
//! test").
//!
//! Public surface (mirrors `kenken`):
//!   - `generate_pattern(kind, rng)` / `generate_for_level(level, rng)`
//!   - `PatternSession::new(puzzle)` → fresh session in InProgress
//!   - `pattern_reducer(session, action)` → new session
//!
//! One action exists: `Select { choice }`. A correct pick completes the puzzle;
//! a wrong pick "bounces back" (recorded for the renderer) and lets the kid try
//! again — natural consequence, never punishment (architecture invariant #7).

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

// ─── Types ──────────────────────────────────────────────

/// One slot in a pattern. Pictures for young kids, numbers for older ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PatternElement {
    Sprite { name: String },
    Color { color: String },
    Shape { shape: String },
    Number { value: i32 },
}

/// The shape of the sequence. Repeat* are picture patterns (no arithmetic);
/// the rest are numeric and scale toward multiplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PatternKind {
    /// A B A B … — the simplest repeat.
    RepeatAb,
    /// A B B A B B … — a longer repeating unit.
    RepeatAbb,
    /// A B C A B C … — a three-element repeat.
    RepeatAbc,
    /// Arithmetic sequence: start, start+step, start+2·step, …
    /// `step` of 1 is counting; 2/3/5 are skip-counting (multiplication seeds).
    CountBy { step: i32 },
    /// Doubling: 1, 2, 4, 8, … — geometric growth.
    Double,
    /// Square numbers: 1, 4, 9, 16, … — advanced.
    Squares,
}

impl PatternKind {
    /// Numeric patterns work with `Number` elements; the rest are pictures.
    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            PatternKind::CountBy { .. } | PatternKind::Double | PatternKind::Squares
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternPuzzle {
    pub kind: PatternKind,
    /// What the kid sees, left to right, with the next slot left blank.
    pub visible_elements: Vec<PatternElement>,
    /// The element that belongs in the blank slot (index == visible_elements.len()).
    pub correct_answer: PatternElement,
    /// Multiple-choice options, already shuffled. Always contains the answer.
    pub choices: Vec<PatternElement>,
}

impl PatternPuzzle {
    /// Index into `choices` that is the correct answer.
    pub fn correct_choice(&self) -> usize {
        self.choices
            .iter()
            .position(|c| *c == self.correct_answer)
            .expect("choices always contains the correct answer")
    }

    pub fn is_correct(&self, choice: usize) -> bool {
        self.choices.get(choice) == Some(&self.correct_answer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternPhase {
    InProgress,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternSession {
    pub puzzle: PatternPuzzle,
    pub phase: PatternPhase,
    pub attempts: u8,
    /// Index of the most recent wrong pick, for the renderer's bounce-back
    /// animation. Cleared on a correct pick.
    pub last_wrong: Option<usize>,
}

impl PatternSession {
    pub fn new(puzzle: PatternPuzzle) -> Self {
        PatternSession {
            puzzle,
            phase: PatternPhase::InProgress,
            attempts: 0,
            last_wrong: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PatternAction {
    Select { choice: usize },
}

// ─── Reducer ────────────────────────────────────────────

pub fn pattern_reducer(state: PatternSession, action: PatternAction) -> PatternSession {
    if state.phase == PatternPhase::Complete {
        return state;
    }
    match action {
        PatternAction::Select { choice } => {
            if choice >= state.puzzle.choices.len() {
                return state;
            }
            let attempts = state.attempts.saturating_add(1);
            if state.puzzle.is_correct(choice) {
                PatternSession {
                    phase: PatternPhase::Complete,
                    attempts,
                    last_wrong: None,
                    ..state
                }
            } else {
                // Wrong pick: bounce it back, let the kid try again. No punishment.
                PatternSession {
                    attempts,
                    last_wrong: Some(choice),
                    ..state
                }
            }
        }
    }
}

// ─── Generation ─────────────────────────────────────────

const SPRITES: &[&str] = &["sparky", "dum_dum", "star", "gear", "battery"];
const COLORS: &[&str] = &["red", "blue", "green", "yellow", "purple"];
const SHAPES: &[&str] = &["circle", "square", "triangle", "diamond", "heart"];

/// Number of elements shown before the blank. Enough to make the pattern
/// unambiguous — at least one full repeating unit plus a bit more.
fn visible_len(kind: PatternKind) -> usize {
    match kind {
        PatternKind::RepeatAb => 5,   // A B A B A → ?(B)
        PatternKind::RepeatAbb => 6,  // A B B A B B → ?(A)
        PatternKind::RepeatAbc => 6,  // A B C A B C → ?(A)
        PatternKind::CountBy { .. } => 4,
        PatternKind::Double => 4,
        PatternKind::Squares => 4,
    }
}

pub fn generate_pattern(kind: PatternKind, rng: &mut impl Rng) -> PatternPuzzle {
    if kind.is_numeric() {
        generate_numeric(kind, rng)
    } else {
        generate_repeat(kind, rng)
    }
}

/// Pick a fresh picture palette (sprites, colors, or shapes) of `count` distinct
/// elements. Which family is used is randomized for variety.
fn picture_palette(count: usize, rng: &mut impl Rng) -> Vec<PatternElement> {
    let family = rng.gen_range(0..3);
    let pool: Vec<PatternElement> = match family {
        0 => SPRITES.iter().map(|s| PatternElement::Sprite { name: (*s).into() }).collect(),
        1 => COLORS.iter().map(|c| PatternElement::Color { color: (*c).into() }).collect(),
        _ => SHAPES.iter().map(|s| PatternElement::Shape { shape: (*s).into() }).collect(),
    };
    let mut pool = pool;
    pool.shuffle(rng);
    pool.truncate(count);
    pool
}

fn generate_repeat(kind: PatternKind, rng: &mut impl Rng) -> PatternPuzzle {
    let unit: Vec<usize> = match kind {
        PatternKind::RepeatAb => vec![0, 1],
        PatternKind::RepeatAbb => vec![0, 1, 1],
        PatternKind::RepeatAbc => vec![0, 1, 2],
        _ => unreachable!("generate_repeat called with numeric kind"),
    };
    let distinct = unit.iter().copied().max().unwrap() + 1;
    let palette = picture_palette(distinct, rng);

    let len = visible_len(kind);
    let full: Vec<PatternElement> = (0..=len)
        .map(|i| palette[unit[i % unit.len()]].clone())
        .collect();
    let correct_answer = full[len].clone();
    let visible_elements = full[..len].to_vec();

    // Choices are the distinct palette elements — picking the right one IS the
    // pattern game. Shuffled so position carries no information.
    let mut choices = palette;
    choices.shuffle(rng);

    PatternPuzzle { kind, visible_elements, correct_answer, choices }
}

fn generate_numeric(kind: PatternKind, rng: &mut impl Rng) -> PatternPuzzle {
    let len = visible_len(kind);
    let seq: Vec<i32> = match kind {
        PatternKind::CountBy { step } => {
            let start = rng.gen_range(0..=3);
            (0..=len as i32).map(|i| start + i * step).collect()
        }
        PatternKind::Double => {
            let start = rng.gen_range(1..=2);
            (0..=len as i32).map(|i| start * 2i32.pow(i as u32)).collect()
        }
        PatternKind::Squares => (1..=(len as i32 + 1)).map(|i| i * i).collect(),
        _ => unreachable!("generate_numeric called with picture kind"),
    };
    let correct = seq[len];
    let visible_elements: Vec<PatternElement> = seq[..len]
        .iter()
        .map(|&v| PatternElement::Number { value: v })
        .collect();
    let correct_answer = PatternElement::Number { value: correct };

    // Nearby distractors. Don't show negative tiles unless the answer itself is
    // negative (keeps a counting question kid-friendly), and guarantee at least
    // three distinct tiles regardless of the answer's sign/magnitude — so the
    // choice set is never degenerate even for an unusual sequence.
    let mut values = vec![correct];
    for d in [correct + 1, correct - 1, correct + 2, correct - 2, correct + 3, correct + 4] {
        if values.len() >= 4 {
            break;
        }
        let sign_ok = correct < 0 || d >= 0;
        if sign_ok && !values.contains(&d) {
            values.push(d);
        }
    }
    let mut extra = correct + 5;
    while values.len() < 3 {
        if !values.contains(&extra) {
            values.push(extra);
        }
        extra += 1;
    }
    let mut choices: Vec<PatternElement> =
        values.into_iter().map(|v| PatternElement::Number { value: v }).collect();
    choices.shuffle(rng);

    PatternPuzzle { kind, visible_elements, correct_answer, choices }
}

// ─── Profile-driven helpers ─────────────────────────────

/// Pattern kinds unlocked at a given level (1 = youngest). Mirrors the spec's
/// progression: picture repeats first, then counting, skip-counting, doubling,
/// squares. The caller picks one at random for variety.
pub fn pattern_kinds_for_level(level: u8) -> Vec<PatternKind> {
    let mut kinds = vec![PatternKind::RepeatAb];
    if level >= 2 {
        kinds.push(PatternKind::RepeatAbb);
    }
    if level >= 3 {
        kinds.push(PatternKind::RepeatAbc);
    }
    if level >= 4 {
        kinds.push(PatternKind::CountBy { step: 1 });
    }
    if level >= 5 {
        kinds.push(PatternKind::CountBy { step: 2 });
    }
    if level >= 6 {
        kinds.push(PatternKind::Double);
    }
    if level >= 7 {
        kinds.push(PatternKind::Squares);
    }
    kinds
}

/// Generate a pattern appropriate for the kid's pattern level, biased toward the
/// newest unlocked kinds so they keep meeting fresh challenges.
pub fn generate_for_level(level: u8, rng: &mut impl Rng) -> PatternPuzzle {
    let kinds = pattern_kinds_for_level(level);
    let kind = *kinds.last().unwrap();
    // 60% newest kind, 40% review of an earlier one.
    let chosen = if rng.gen_range(0..100) < 60 || kinds.len() == 1 {
        kind
    } else {
        kinds[rng.gen_range(0..kinds.len())]
    };
    generate_pattern(chosen, rng)
}

// ─── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn rng() -> SmallRng {
        SmallRng::seed_from_u64(42)
    }

    fn nums(p: &PatternPuzzle) -> Vec<i32> {
        p.visible_elements
            .iter()
            .map(|e| match e {
                PatternElement::Number { value } => *value,
                other => panic!("expected Number, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn repeat_ab_alternates_and_continues() {
        let p = generate_pattern(PatternKind::RepeatAb, &mut rng());
        let v = &p.visible_elements;
        assert_eq!(v.len(), 5);
        // Alternating: even slots equal, odd slots equal, even != odd.
        assert_eq!(v[0], v[2]);
        assert_eq!(v[0], v[4]);
        assert_eq!(v[1], v[3]);
        assert_ne!(v[0], v[1]);
        // The 6th element continues the alternation → equals v[1].
        assert_eq!(p.correct_answer, v[1]);
    }

    #[test]
    fn repeat_abb_has_three_element_unit() {
        let p = generate_pattern(PatternKind::RepeatAbb, &mut rng());
        let v = &p.visible_elements;
        assert_eq!(v.len(), 6);
        // unit A B B repeated: positions 0,3 are A; 1,2,4,5 are B.
        assert_eq!(v[0], v[3]);
        assert_eq!(v[1], v[2]);
        assert_eq!(v[1], v[4]);
        assert_eq!(v[1], v[5]);
        assert_ne!(v[0], v[1]);
        // Next after ...A B B (index 6) restarts the unit → A.
        assert_eq!(p.correct_answer, v[0]);
    }

    #[test]
    fn repeat_abc_cycles_three() {
        let p = generate_pattern(PatternKind::RepeatAbc, &mut rng());
        let v = &p.visible_elements;
        assert_eq!(v.len(), 6);
        assert_eq!(v[0], v[3]);
        assert_eq!(v[1], v[4]);
        assert_eq!(v[2], v[5]);
        // three distinct elements
        assert_ne!(v[0], v[1]);
        assert_ne!(v[1], v[2]);
        assert_ne!(v[0], v[2]);
        assert_eq!(p.correct_answer, v[0]);
    }

    #[test]
    fn count_by_two_is_skip_counting() {
        // Try several seeds; the arithmetic relationship must always hold.
        for seed in 0..50u64 {
            let mut r = SmallRng::seed_from_u64(seed);
            let p = generate_pattern(PatternKind::CountBy { step: 2 }, &mut r);
            let v = nums(&p);
            for w in v.windows(2) {
                assert_eq!(w[1] - w[0], 2, "step must be 2");
            }
            let last = *v.last().unwrap();
            assert_eq!(p.correct_answer, PatternElement::Number { value: last + 2 });
        }
    }

    #[test]
    fn double_is_geometric() {
        for seed in 0..50u64 {
            let mut r = SmallRng::seed_from_u64(seed);
            let p = generate_pattern(PatternKind::Double, &mut r);
            let v = nums(&p);
            for w in v.windows(2) {
                assert_eq!(w[1], w[0] * 2, "each term doubles");
            }
            let last = *v.last().unwrap();
            assert_eq!(p.correct_answer, PatternElement::Number { value: last * 2 });
        }
    }

    #[test]
    fn squares_are_perfect_squares() {
        let p = generate_pattern(PatternKind::Squares, &mut rng());
        let v = nums(&p);
        assert_eq!(v, vec![1, 4, 9, 16]);
        assert_eq!(p.correct_answer, PatternElement::Number { value: 25 });
    }

    #[test]
    fn choices_always_contain_answer_and_are_distinct() {
        let kinds = [
            PatternKind::RepeatAb,
            PatternKind::RepeatAbb,
            PatternKind::RepeatAbc,
            PatternKind::CountBy { step: 1 },
            PatternKind::CountBy { step: 3 },
            PatternKind::Double,
            PatternKind::Squares,
        ];
        for kind in kinds {
            for seed in 0..50u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_pattern(kind, &mut r);
                assert!(
                    p.choices.contains(&p.correct_answer),
                    "{kind:?} seed={seed}: choices must contain the answer",
                );
                let mut seen = p.choices.clone();
                let before = seen.len();
                seen.dedup_by(|a, b| a == b);
                // dedup only removes *consecutive* dups; sort first for a real check.
                let mut sorted = p.choices.clone();
                sorted.sort_by_key(|e| format!("{e:?}"));
                sorted.dedup_by(|a, b| a == b);
                assert_eq!(
                    sorted.len(),
                    before,
                    "{kind:?} seed={seed}: choices must be distinct, got {:?}",
                    p.choices,
                );
                assert!(p.choices.len() >= 2, "need at least two choices");
            }
        }
    }

    #[test]
    fn numeric_choices_are_all_positive() {
        for kind in [PatternKind::CountBy { step: 1 }, PatternKind::Double, PatternKind::Squares] {
            for seed in 0..50u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_pattern(kind, &mut r);
                for c in &p.choices {
                    if let PatternElement::Number { value } = c {
                        assert!(*value >= 0, "{kind:?} seed={seed}: choice {value} negative");
                    }
                }
            }
        }
    }

    #[test]
    fn numeric_choices_stay_well_formed_for_odd_sequences() {
        // Even a non-game sequence (negative step → negative answer) must yield
        // a usable choice set: the answer is present, choices are distinct, and
        // there are at least three tiles. Guards the choice generator's floor.
        for kind in [
            PatternKind::CountBy { step: -2 },
            PatternKind::CountBy { step: 0 },
            PatternKind::CountBy { step: 1 },
        ] {
            for seed in 0..50u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_pattern(kind, &mut r);
                assert!(p.choices.contains(&p.correct_answer), "{kind:?}: answer present");
                assert!(p.choices.len() >= 3, "{kind:?} seed={seed}: too few choices {:?}", p.choices);
                let mut seen = p.choices.clone();
                seen.sort_by_key(|e| match e {
                    PatternElement::Number { value } => *value,
                    _ => 0,
                });
                seen.dedup();
                assert_eq!(seen.len(), p.choices.len(), "{kind:?}: choices distinct");
            }
        }
    }

    #[test]
    fn correct_selection_completes() {
        let p = generate_pattern(PatternKind::RepeatAb, &mut rng());
        let correct = p.correct_choice();
        let s = PatternSession::new(p);
        assert_eq!(s.phase, PatternPhase::InProgress);
        let s = pattern_reducer(s, PatternAction::Select { choice: correct });
        assert_eq!(s.phase, PatternPhase::Complete);
        assert_eq!(s.attempts, 1);
        assert_eq!(s.last_wrong, None);
    }

    #[test]
    fn wrong_selection_bounces_back_and_allows_retry() {
        let p = generate_pattern(PatternKind::RepeatAbc, &mut rng());
        let correct = p.correct_choice();
        let wrong = (0..p.choices.len()).find(|&i| i != correct).unwrap();
        let s = PatternSession::new(p);
        let s = pattern_reducer(s, PatternAction::Select { choice: wrong });
        // Still solvable — graceful failure, no lockout.
        assert_eq!(s.phase, PatternPhase::InProgress);
        assert_eq!(s.last_wrong, Some(wrong));
        assert_eq!(s.attempts, 1);
        // Retry with the right answer succeeds.
        let correct = s.puzzle.correct_choice();
        let s = pattern_reducer(s, PatternAction::Select { choice: correct });
        assert_eq!(s.phase, PatternPhase::Complete);
        assert_eq!(s.attempts, 2);
        assert_eq!(s.last_wrong, None);
    }

    #[test]
    fn complete_session_ignores_further_input() {
        let p = generate_pattern(PatternKind::RepeatAb, &mut rng());
        let correct = p.correct_choice();
        let s = PatternSession::new(p);
        let s = pattern_reducer(s, PatternAction::Select { choice: correct });
        let attempts_at_complete = s.attempts;
        let s = pattern_reducer(s, PatternAction::Select { choice: correct });
        assert_eq!(s.attempts, attempts_at_complete, "no further mutation once complete");
    }

    #[test]
    fn out_of_range_choice_is_ignored() {
        let p = generate_pattern(PatternKind::RepeatAb, &mut rng());
        let n = p.choices.len();
        let s = PatternSession::new(p);
        let s = pattern_reducer(s, PatternAction::Select { choice: n + 5 });
        assert_eq!(s.phase, PatternPhase::InProgress);
        assert_eq!(s.attempts, 0, "ignored input doesn't count as an attempt");
    }

    #[test]
    fn levels_unlock_kinds_progressively() {
        assert_eq!(pattern_kinds_for_level(1), vec![PatternKind::RepeatAb]);
        assert!(pattern_kinds_for_level(2).contains(&PatternKind::RepeatAbb));
        assert!(!pattern_kinds_for_level(2).contains(&PatternKind::Squares));
        assert!(pattern_kinds_for_level(7).contains(&PatternKind::Squares));
        // Higher levels never lose access to earlier kinds.
        assert!(pattern_kinds_for_level(7).contains(&PatternKind::RepeatAb));
    }

    #[test]
    fn generate_for_level_only_uses_unlocked_kinds() {
        for level in 1..=7u8 {
            let allowed = pattern_kinds_for_level(level);
            for seed in 0..40u64 {
                let mut r = SmallRng::seed_from_u64(seed);
                let p = generate_for_level(level, &mut r);
                assert!(
                    allowed.contains(&p.kind),
                    "level {level} seed {seed}: produced {:?} not in {:?}",
                    p.kind,
                    allowed,
                );
            }
        }
    }

    #[test]
    fn generation_is_seed_deterministic() {
        let a = generate_pattern(PatternKind::CountBy { step: 2 }, &mut rng());
        let b = generate_pattern(PatternKind::CountBy { step: 2 }, &mut rng());
        assert_eq!(nums(&a), nums(&b));
        assert_eq!(a.correct_answer, b.correct_answer);
    }
}
