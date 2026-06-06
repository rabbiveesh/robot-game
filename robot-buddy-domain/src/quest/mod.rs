//! Quest system. Pure logic — no rendering, no input.
//!
//! Quests are the narrative spine that makes math the gameplay rather than a
//! pop-quiz: a story sets up a need, the kid travels and solves an *embedded*
//! math puzzle to move the story forward, and a reward lands. The math is the
//! mechanic, not chrome bolted onto a flash card.
//!
//! This module owns the quest data model and the step-advancing reducer. It
//! deliberately does NOT generate the actual arithmetic challenge — a
//! `MathPuzzle` step carries the operation/band (and optional story operands)
//! and the game wires that to the existing challenge generator or a CRA
//! manipulative. Wrong answers never lock a quest (fail gracefully).
//!
//! Public surface mirrors the logic modules:
//!   - example quests: `welcome_quest()`, `dum_dum_heist()`
//!   - `generate_micro_quest(band, &mut impl Rng)`
//!   - `QuestSession::new(quest)` / `start`, then `quest_reducer(session, action)`

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::types::Operation;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum QuestStep {
    /// An NPC sets up context; the player taps through (AdvanceStep).
    Dialogue { speaker: String, lines: Vec<String> },
    /// Go to a map tile; satisfied by `ArriveAt` with matching coords.
    Travel { map: String, x: u8, y: u8 },
    /// The core: a story-embedded math problem. `operands` lets a quest pin the
    /// exact numbers the story mentions; when `None`, the game generates from
    /// `band`. Advanced by `CompletePuzzle { correct: true }`.
    MathPuzzle {
        operation: Operation,
        band: u8,
        context: String,
        #[serde(default)]
        operands: Option<(u16, u16)>,
    },
    /// A branching decision; advanced by `ChooseOption`.
    Choice { prompt: String, options: Vec<String> },
    /// Celebration + payout; advanced by `AdvanceStep` after the game awards.
    Reward { dum_dums: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub steps: Vec<QuestStep>,
    pub math_domain: Vec<Operation>,
    pub min_band: u8,
    pub max_band: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestStatus {
    NotStarted,
    Active,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestSession {
    pub quest: Quest,
    pub status: QuestStatus,
    pub current_step: usize,
    /// Wrong puzzle attempts on the current step; resets each time a step is
    /// advanced. Drives "try again / teaching mode" without ever locking out.
    pub puzzle_attempts: u8,
    /// Index of the option chosen on the most recent Choice step (for branching
    /// the game may layer on later); None until a choice is made.
    pub last_choice: Option<usize>,
}

impl QuestSession {
    pub fn new(quest: Quest) -> Self {
        QuestSession {
            quest,
            status: QuestStatus::NotStarted,
            current_step: 0,
            puzzle_attempts: 0,
            last_choice: None,
        }
    }

    pub fn current_step(&self) -> Option<&QuestStep> {
        if self.status == QuestStatus::Active {
            self.quest.steps.get(self.current_step)
        } else {
            None
        }
    }

    /// Dum Dums to award right now, if the current step is a Reward. The game
    /// pays this out, then sends `AdvanceStep` to finish the step.
    pub fn pending_reward(&self) -> Option<u32> {
        match self.current_step() {
            Some(QuestStep::Reward { dum_dums }) => Some(*dum_dums),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum QuestAction {
    Start,
    /// Tap through a Dialogue or Reward step.
    AdvanceStep,
    /// Report arrival at a tile (satisfies a matching Travel step).
    ArriveAt { map: String, x: u8, y: u8 },
    /// Report the outcome of the current MathPuzzle step.
    CompletePuzzle { correct: bool },
    /// Pick an option on the current Choice step.
    ChooseOption { index: usize },
}

pub fn quest_reducer(state: QuestSession, action: QuestAction) -> QuestSession {
    match action {
        QuestAction::Start => {
            if state.status != QuestStatus::NotStarted {
                return state;
            }
            let mut next = state;
            next.status = QuestStatus::Active;
            next.current_step = 0;
            next.puzzle_attempts = 0;
            if next.quest.steps.is_empty() {
                next.status = QuestStatus::Complete;
            }
            next
        }
        _ if state.status != QuestStatus::Active => state,
        QuestAction::AdvanceStep => match state.current_step() {
            Some(QuestStep::Dialogue { .. }) | Some(QuestStep::Reward { .. }) => advance(state),
            // A degenerate Choice with no options is just a beat — let it
            // advance rather than soft-lock (never trap the kid; invariant #7).
            Some(QuestStep::Choice { options, .. }) if options.is_empty() => advance(state),
            _ => state, // Travel/Puzzle/non-empty Choice need their specific action
        },
        QuestAction::ArriveAt { map, x, y } => match state.current_step() {
            Some(QuestStep::Travel { map: m, x: tx, y: ty }) if *m == map && *tx == x && *ty == y => {
                advance(state)
            }
            _ => state,
        },
        QuestAction::CompletePuzzle { correct } => match state.current_step() {
            Some(QuestStep::MathPuzzle { .. }) => {
                if correct {
                    advance(state)
                } else {
                    // Fail gracefully: record the attempt, never lock the quest.
                    let mut next = state;
                    next.puzzle_attempts = next.puzzle_attempts.saturating_add(1);
                    next
                }
            }
            _ => state,
        },
        QuestAction::ChooseOption { index } => match state.current_step() {
            Some(QuestStep::Choice { options, .. }) if index < options.len() => {
                let mut next = advance(state);
                next.last_choice = Some(index);
                next
            }
            _ => state,
        },
    }
}

fn advance(mut state: QuestSession) -> QuestSession {
    state.current_step += 1;
    state.puzzle_attempts = 0;
    if state.current_step >= state.quest.steps.len() {
        state.status = QuestStatus::Complete;
    }
    state
}

// ─── Hand-authored quests ───────────────────────────────

fn dialogue(speaker: &str, line: &str) -> QuestStep {
    QuestStep::Dialogue { speaker: speaker.into(), lines: vec![line.into()] }
}

/// Starter quest from the spec — teaches basic addition through helping Sparky
/// set up his house.
pub fn welcome_quest() -> Quest {
    Quest {
        id: "welcome".into(),
        title: "Welcome to Robot Village".into(),
        description: "Help Sparky set up his new house!".into(),
        steps: vec![
            dialogue("Sparky", "Boss! I just moved here and I need to set up my house! Can you help me carry stuff?"),
            QuestStep::Travel { map: "overworld".into(), x: 22, y: 4 },
            QuestStep::MathPuzzle {
                operation: Operation::Add,
                band: 1,
                context: "I need 3 bolts and 2 gears. How many parts is that?".into(),
                operands: Some((3, 2)),
            },
            QuestStep::Travel { map: "overworld".into(), x: 13, y: 12 },
            QuestStep::MathPuzzle {
                operation: Operation::Add,
                band: 1,
                context: "We have 4 bolts in the toolbox. Now we add 3 more. How many total?".into(),
                operands: Some((4, 3)),
            },
            dialogue("Sparky", "We did it! My house has a googly-eyed mailbox now. BEST. BOSS. EVER."),
            QuestStep::Reward { dum_dums: 3 },
        ],
        math_domain: vec![Operation::Add],
        min_band: 1,
        max_band: 2,
    }
}

/// Mid-game multi-operation quest (simplified from the spec's "Great Dum Dum
/// Heist"), with a branching choice.
pub fn dum_dum_heist() -> Quest {
    Quest {
        id: "dum_dum_heist".into(),
        title: "The Great Dum Dum Heist".into(),
        description: "Someone stole the shop's Dum Dums! Track them down.".into(),
        steps: vec![
            dialogue("Bolt", "Disaster! All 48 Dum Dums are gone! Help me find the thief!"),
            QuestStep::MathPuzzle {
                operation: Operation::Divide,
                band: 5,
                context: "The thief left footprints in groups of 4. There are 12 footprints. How many trips did they make?".into(),
                operands: Some((12, 4)),
            },
            QuestStep::Travel { map: "overworld".into(), x: 5, y: 20 },
            QuestStep::MathPuzzle {
                operation: Operation::Sub,
                band: 5,
                context: "The thief started with 48 but dropped 13. How many do they still have?".into(),
                operands: Some((48, 13)),
            },
            QuestStep::Choice {
                prompt: "Confront the thief, or set a trap?".into(),
                options: vec!["Confront".into(), "Set a trap".into()],
            },
            dialogue("Sparky", "We got them! And the thief said sorry. New friend unlocked!"),
            QuestStep::Reward { dum_dums: 10 },
        ],
        math_domain: vec![Operation::Divide, Operation::Sub],
        min_band: 4,
        max_band: 6,
    }
}

// ─── Procedural micro-quests ────────────────────────────

/// Scale a small operand to the band so generated puzzles aren't all tiny.
fn scaled(band: u8, rng: &mut impl Rng, lo: u16, hi: u16) -> u16 {
    let bump = (band as u16).saturating_sub(1); // higher band → bigger numbers
    rng.gen_range(lo..=hi) + bump
}

/// Build a fresh single-puzzle micro-quest scaled to the kid's band — infinite
/// filler content between the hand-authored milestones. Deterministic per seed.
pub fn generate_micro_quest(band: u8, rng: &mut impl Rng) -> Quest {
    let templates: [&str; 3] = ["delivery", "shop_trip", "fetch_and_count"];
    let template = *templates.choose(rng).unwrap();
    let (operation, context, operands, op_domain) = match template {
        "delivery" => {
            let trips = scaled(band, rng, 2, 5);
            let per = scaled(band, rng, 2, 4);
            let total = trips * per;
            (
                Operation::Divide,
                format!("Deliver {total} parcels, {per} per trip. How many trips?"),
                (total, per),
                Operation::Divide,
            )
        }
        "shop_trip" => {
            let each = scaled(band, rng, 2, 5);
            let count = scaled(band, rng, 2, 4);
            (
                Operation::Multiply,
                format!("Gears cost {each} bolts each. You need {count}. How many bolts?"),
                (each, count),
                Operation::Multiply,
            )
        }
        _ => {
            let red = scaled(band, rng, 2, 6);
            let blue = scaled(band, rng, 2, 6);
            (
                Operation::Add,
                format!("You found {red} red gems and {blue} blue gems. How many in all?"),
                (red, blue),
                Operation::Add,
            )
        }
    };
    let reward = 1 + (band as u32) / 3;
    Quest {
        id: format!("micro_{template}"),
        title: "A Quick Favor".into(),
        description: context.clone(),
        steps: vec![
            QuestStep::Dialogue { speaker: "Sparky".into(), lines: vec![context.clone()] },
            QuestStep::MathPuzzle { operation, band, context, operands: Some(operands) },
            QuestStep::Reward { dum_dums: reward },
        ],
        math_domain: vec![op_domain],
        min_band: band.saturating_sub(1).max(1),
        max_band: band.saturating_add(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn travel_to(s: QuestSession) -> QuestSession {
        // Drive an Active session past a Travel step using its own coords.
        if let Some(QuestStep::Travel { map, x, y }) = s.current_step() {
            let (m, x, y) = (map.clone(), *x, *y);
            quest_reducer(s, QuestAction::ArriveAt { map: m, x, y })
        } else {
            s
        }
    }

    #[test]
    fn welcome_quest_runs_start_to_finish_in_order() {
        let mut s = QuestSession::new(welcome_quest());
        assert_eq!(s.status, QuestStatus::NotStarted);
        s = quest_reducer(s, QuestAction::Start);
        assert_eq!(s.status, QuestStatus::Active);

        // Step 0: dialogue
        s = quest_reducer(s, QuestAction::AdvanceStep);
        // Step 1: travel
        s = travel_to(s);
        // Step 2: math puzzle (3+2)
        assert!(matches!(s.current_step(), Some(QuestStep::MathPuzzle { .. })));
        s = quest_reducer(s, QuestAction::CompletePuzzle { correct: true });
        // Step 3: travel
        s = travel_to(s);
        // Step 4: math puzzle (4+3)
        s = quest_reducer(s, QuestAction::CompletePuzzle { correct: true });
        // Step 5: dialogue
        s = quest_reducer(s, QuestAction::AdvanceStep);
        // Step 6: reward
        assert_eq!(s.pending_reward(), Some(3));
        s = quest_reducer(s, QuestAction::AdvanceStep);
        assert_eq!(s.status, QuestStatus::Complete);
    }

    #[test]
    fn wrong_puzzle_answer_never_locks_the_quest() {
        let mut s = quest_reducer(QuestSession::new(welcome_quest()), QuestAction::Start);
        s = quest_reducer(s, QuestAction::AdvanceStep); // dialogue
        s = travel_to(s); // travel → now on the puzzle
        let step_before = s.current_step;
        s = quest_reducer(s, QuestAction::CompletePuzzle { correct: false });
        assert_eq!(s.current_step, step_before, "wrong answer does not advance");
        assert_eq!(s.status, QuestStatus::Active, "and never locks");
        assert_eq!(s.puzzle_attempts, 1);
        // A correct retry advances.
        s = quest_reducer(s, QuestAction::CompletePuzzle { correct: true });
        assert_ne!(s.current_step, step_before);
    }

    #[test]
    fn travel_requires_matching_coords() {
        let mut s = quest_reducer(QuestSession::new(welcome_quest()), QuestAction::Start);
        s = quest_reducer(s, QuestAction::AdvanceStep); // now on Travel(22,4)
        // Wrong tile: no progress.
        let before = s.current_step;
        s = quest_reducer(s, QuestAction::ArriveAt { map: "overworld".into(), x: 0, y: 0 });
        assert_eq!(s.current_step, before);
        // Tapping (AdvanceStep) doesn't skip a travel step either.
        s = quest_reducer(s, QuestAction::AdvanceStep);
        assert_eq!(s.current_step, before);
        // Correct tile advances.
        s = quest_reducer(s, QuestAction::ArriveAt { map: "overworld".into(), x: 22, y: 4 });
        assert_eq!(s.current_step, before + 1);
    }

    #[test]
    fn choice_step_records_selection_and_advances() {
        let mut s = quest_reducer(QuestSession::new(dum_dum_heist()), QuestAction::Start);
        // Walk to the Choice step.
        s = quest_reducer(s, QuestAction::AdvanceStep); // dialogue
        s = quest_reducer(s, QuestAction::CompletePuzzle { correct: true }); // division
        s = travel_to(s); // travel
        s = quest_reducer(s, QuestAction::CompletePuzzle { correct: true }); // subtraction
        assert!(matches!(s.current_step(), Some(QuestStep::Choice { .. })));
        let before = s.current_step;
        // Out-of-range option ignored.
        s = quest_reducer(s, QuestAction::ChooseOption { index: 9 });
        assert_eq!(s.current_step, before);
        s = quest_reducer(s, QuestAction::ChooseOption { index: 1 });
        assert_eq!(s.last_choice, Some(1));
        assert_eq!(s.current_step, before + 1);
    }

    #[test]
    fn degenerate_empty_choice_advances_instead_of_locking() {
        // A Choice with no options must not trap the kid — AdvanceStep passes it.
        let quest = Quest {
            id: "t".into(),
            title: "t".into(),
            description: "t".into(),
            steps: vec![
                QuestStep::Choice { prompt: "?".into(), options: vec![] },
                QuestStep::Reward { dum_dums: 1 },
            ],
            math_domain: vec![],
            min_band: 1,
            max_band: 1,
        };
        let mut s = quest_reducer(QuestSession::new(quest), QuestAction::Start);
        assert!(matches!(s.current_step(), Some(QuestStep::Choice { .. })));
        s = quest_reducer(s, QuestAction::AdvanceStep);
        assert!(matches!(s.current_step(), Some(QuestStep::Reward { .. })),
            "empty choice should advance to the next step, not lock");
    }

    #[test]
    fn completed_quest_ignores_further_actions() {
        let mut s = quest_reducer(QuestSession::new(welcome_quest()), QuestAction::Start);
        // Blast through everything.
        for _ in 0..20 {
            s = match s.current_step() {
                Some(QuestStep::Travel { map, x, y }) => {
                    let (m, x, y) = (map.clone(), *x, *y);
                    quest_reducer(s, QuestAction::ArriveAt { map: m, x, y })
                }
                Some(QuestStep::MathPuzzle { .. }) => quest_reducer(s, QuestAction::CompletePuzzle { correct: true }),
                Some(_) => quest_reducer(s, QuestAction::AdvanceStep),
                None => break,
            };
        }
        assert_eq!(s.status, QuestStatus::Complete);
        let step = s.current_step;
        s = quest_reducer(s, QuestAction::AdvanceStep);
        assert_eq!(s.current_step, step, "no mutation once complete");
    }

    #[test]
    fn micro_quest_is_deterministic_and_well_formed() {
        for band in 1..=10u8 {
            let mut a = SmallRng::seed_from_u64(band as u64);
            let mut b = SmallRng::seed_from_u64(band as u64);
            let qa = generate_micro_quest(band, &mut a);
            let qb = generate_micro_quest(band, &mut b);
            assert_eq!(qa.id, qb.id, "same seed → same quest");
            // Has exactly one MathPuzzle and a Reward, band within range.
            let puzzles = qa.steps.iter().filter(|s| matches!(s, QuestStep::MathPuzzle { .. })).count();
            assert_eq!(puzzles, 1, "micro-quest has one embedded puzzle");
            assert!(qa.steps.iter().any(|s| matches!(s, QuestStep::Reward { .. })));
            assert!(qa.min_band <= band && band <= qa.max_band);
        }
    }

    #[test]
    fn micro_quest_can_be_completed() {
        let mut s = quest_reducer(
            QuestSession::new(generate_micro_quest(3, &mut SmallRng::seed_from_u64(1))),
            QuestAction::Start,
        );
        s = quest_reducer(s, QuestAction::AdvanceStep); // dialogue
        s = quest_reducer(s, QuestAction::CompletePuzzle { correct: true }); // puzzle
        assert!(s.pending_reward().is_some());
        s = quest_reducer(s, QuestAction::AdvanceStep); // reward
        assert_eq!(s.status, QuestStatus::Complete);
    }
}
