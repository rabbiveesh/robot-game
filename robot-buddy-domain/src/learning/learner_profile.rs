use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{CraStage, Operation};
use super::operation_stats::OperationStats;
use super::rolling_window::{RollingWindow, WindowEntry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnerProfile {
    pub math_band: u8,
    pub streak: i32,
    pub pace: f64,
    pub scaffolding: f64,
    pub challenge_freq: f64,
    pub spread_width: f64,
    pub promote_threshold: f64,
    pub stretch_threshold: f64,
    pub wrongs_before_teach: u8,
    pub hint_visibility: f64,
    pub text_speed: f64,
    pub framing_style: f64,
    pub representation_style: f64,
    pub cra_stages: HashMap<Operation, CraStage>,
    pub intake_completed: bool,
    pub text_skip_count: u32,
    pub rolling_window: RollingWindow,
    pub operation_stats: OperationStats,
}

impl LearnerProfile {
    pub fn new() -> Self {
        let mut cra = HashMap::new();
        for op in [Operation::Add, Operation::Sub, Operation::Multiply, Operation::Divide, Operation::NumberBond] {
            cra.insert(op, CraStage::Concrete);
        }
        LearnerProfile {
            math_band: 1,
            streak: 0,
            pace: 0.5,
            scaffolding: 0.5,
            challenge_freq: 0.5,
            spread_width: 0.5,
            promote_threshold: 0.75,
            stretch_threshold: 0.60,
            wrongs_before_teach: 2,
            hint_visibility: 0.5,
            text_speed: 0.035,
            framing_style: 0.5,
            representation_style: 0.5,
            cra_stages: cra,
            intake_completed: false,
            text_skip_count: 0,
            rolling_window: RollingWindow::new(20),
            operation_stats: OperationStats::new(),
        }
    }
}

// ─── Events ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LearnerEvent {
    #[serde(rename = "PUZZLE_ATTEMPTED")]
    PuzzleAttempted {
        correct: bool,
        operation: Operation,
        #[serde(default)]
        sub_skill: Option<crate::types::SubSkill>,
        band: u8,
        #[serde(default)]
        center_band: Option<u8>,
        #[serde(default)]
        response_time_ms: Option<f64>,
        #[serde(default)]
        hint_used: bool,
        #[serde(default)]
        told_me: bool,
        #[serde(default)]
        cra_level_shown: Option<CraStage>,
        #[serde(default)]
        timestamp: Option<f64>,
    },
    #[serde(rename = "BEHAVIOR")]
    Behavior { signal: String },
    #[serde(rename = "FRUSTRATION_DETECTED")]
    FrustrationDetected { level: String },
    #[serde(rename = "INTAKE_COMPLETED")]
    IntakeCompleted {
        math_band: u8,
        pace: f64,
        scaffolding: f64,
        promote_threshold: f64,
        stretch_threshold: f64,
        text_speed: f64,
    },
}

// ─── Helpers ────────────────────────────────────────────

fn is_boredom_wrong(window: &RollingWindow, correct: bool, response_time_ms: Option<f64>) -> bool {
    if correct { return false; }
    match response_time_ms {
        Some(t) if t <= 2000.0 => {},
        _ => return false,
    }
    let entries = &window.entries;
    if entries.len() < 2 { return false; }
    let prev1 = &entries[entries.len() - 1];
    let prev2 = &entries[entries.len() - 2];
    prev1.correct && prev2.correct
}

fn should_promote(window: &RollingWindow, center_band: u8, promote_threshold: f64, stretch_threshold: f64) -> bool {
    let (at_acc, at_count) = window.accuracy_at_band(center_band);
    if at_count < 4 { return false; }
    if at_acc.unwrap_or(0.0) < promote_threshold { return false; }
    let (above_acc, above_count) = window.accuracy_above_band(center_band);
    if above_count >= 2 && above_acc.unwrap_or(0.0) < stretch_threshold { return false; }
    true
}

fn should_demote(window: &RollingWindow, center_band: u8) -> bool {
    let (at_acc, at_count) = window.accuracy_at_band(center_band);
    if at_count < 4 { return false; }
    at_acc.unwrap_or(1.0) < 0.5
}

fn count_consecutive_no_hint_correct(window: &RollingWindow, operation: Operation, cra_stage: CraStage) -> usize {
    let mut count = 0;
    for entry in window.entries.iter().rev() {
        if entry.operation != operation { continue; }
        if !entry.correct || entry.hint_used || entry.told_me { break; }
        if let Some(shown) = entry.cra_level_shown {
            if shown != cra_stage { break; }
        }
        count += 1;
    }
    count
}

fn count_recent_tell_me(window: &RollingWindow, operation: Operation) -> usize {
    window.entries.iter()
        .filter(|e| e.operation == operation && e.told_me)
        .count()
}

// ─── Reducer ────────────────────────────────────────────

pub fn learner_reducer(state: LearnerProfile, event: LearnerEvent) -> LearnerProfile {
    match event {
        LearnerEvent::PuzzleAttempted {
            correct, operation, sub_skill, band, center_band,
            response_time_ms, hint_used, told_me, cra_level_shown, timestamp,
        } => {
            let boredom = is_boredom_wrong(&state.rolling_window, correct, response_time_ms);

            let entry = WindowEntry {
                correct,
                operation,
                sub_skill,
                band,
                center_band: center_band.unwrap_or(band),
                response_time_ms,
                hint_used,
                told_me,
                cra_level_shown,
                boredom,
                timestamp,
            };
            let new_window = state.rolling_window.push(entry);
            let new_stats = state.operation_stats.record(operation, correct, sub_skill);

            // Streak (display only)
            let mut streak = state.streak;
            if boredom {
                streak = streak.max(0);
            } else if correct {
                streak = streak.max(0) + 1;
            } else {
                streak = streak.min(0) - 1;
            }

            // Band promotion/demotion
            let mut new_band = state.math_band;
            let mut spread = state.spread_width;

            if !boredom {
                if state.math_band < 10 && should_promote(&new_window, state.math_band, state.promote_threshold, state.stretch_threshold) {
                    new_band = state.math_band + 1;
                    streak = 0;
                    spread = (state.spread_width - 0.1).max(0.2);
                } else if state.math_band > 1 && should_demote(&new_window, state.math_band) {
                    new_band = state.math_band - 1;
                    streak = 0;
                    spread = (state.spread_width - 0.15).max(0.1);
                }
            }

            // Spread widening
            let rolling_acc = if new_window.entries.len() >= 10 { Some(new_window.accuracy()) } else { None };
            if new_band == state.math_band {
                if let Some(acc) = rolling_acc {
                    if acc > 0.75 && spread < 0.8 {
                        let step = if new_band == 10 { 0.1 } else { 0.05 };
                        spread = (spread + step).min(1.0);
                    }
                }
            }

            // Scaffolding
            let mut scaffolding = state.scaffolding;
            if let Some(acc) = rolling_acc {
                if acc > 0.85 && scaffolding > 0.1 {
                    scaffolding = (scaffolding - 0.03).max(0.0);
                } else if acc < 0.5 && scaffolding < 0.9 {
                    scaffolding = (scaffolding + 0.05).min(1.0);
                }
            }

            // Pace
            let mut pace = state.pace;
            if let Some(rt) = response_time_ms {
                if correct {
                    if rt < 3000.0 && pace < 1.0 { pace = (pace + 0.02).min(1.0); }
                    else if rt > 10000.0 && pace > 0.0 { pace = (pace - 0.02).max(0.0); }
                }
            }

            // CRA progression
            let mut cra_stages = state.cra_stages.clone();
            if let Some(current_cra) = cra_stages.get(&operation).copied() {
                if correct && !hint_used && !told_me {
                    let no_hint = count_consecutive_no_hint_correct(&new_window, operation, current_cra);
                    if no_hint >= 3 && current_cra != CraStage::Abstract {
                        cra_stages.insert(operation, current_cra.next());
                    }
                }
                if hint_used && correct {
                    if let Some(shown) = cra_level_shown {
                        if shown.order() < current_cra.order() {
                            cra_stages.insert(operation, shown);
                        }
                    }
                }
                if told_me {
                    let tell_count = count_recent_tell_me(&new_window, operation);
                    if tell_count >= 2 && current_cra != CraStage::Concrete {
                        cra_stages.insert(operation, CraStage::Concrete);
                    }
                }
            }

            LearnerProfile {
                math_band: new_band,
                streak,
                pace,
                scaffolding,
                spread_width: spread,
                cra_stages,
                rolling_window: new_window,
                operation_stats: new_stats,
                ..state
            }
        }

        LearnerEvent::Behavior { signal } => {
            match signal.as_str() {
                "text_skipped" => LearnerProfile {
                    pace: (state.pace + 0.1).min(1.0),
                    text_speed: (state.text_speed - 0.005).max(0.01),
                    text_skip_count: state.text_skip_count + 1,
                    ..state
                },
                "rapid_clicking" => LearnerProfile {
                    wrongs_before_teach: state.wrongs_before_teach.max(1) - if state.wrongs_before_teach > 1 { 1 } else { 0 },
                    ..state
                },
                _ => state,
            }
        }

        LearnerEvent::FrustrationDetected { level } => {
            if level == "high" {
                LearnerProfile {
                    math_band: state.math_band.max(1) - if state.math_band > 1 { 1 } else { 0 },
                    wrongs_before_teach: 1,
                    pace: (state.pace - 0.2).max(0.0),
                    streak: 0,
                    spread_width: (state.spread_width - 0.15).max(0.1),
                    ..state
                }
            } else {
                state
            }
        }

        LearnerEvent::IntakeCompleted {
            math_band, pace, scaffolding, promote_threshold, stretch_threshold, text_speed,
        } => {
            LearnerProfile {
                math_band,
                pace,
                scaffolding,
                promote_threshold,
                stretch_threshold,
                text_speed,
                intake_completed: true,
                ..state
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(correct: bool) -> LearnerEvent {
        LearnerEvent::PuzzleAttempted {
            correct,
            operation: Operation::Add,
            sub_skill: None,
            band: 1,
            center_band: Some(1),
            response_time_ms: Some(2000.0),
            hint_used: false,
            told_me: false,
            cra_level_shown: None,
            timestamp: None,
        }
    }

    #[test]
    fn increments_streak_on_correct() {
        let s = learner_reducer(LearnerProfile::new(), attempt(true));
        assert_eq!(s.streak, 1);
    }

    #[test]
    fn decrements_streak_on_wrong() {
        let s = learner_reducer(LearnerProfile::new(), attempt(false));
        assert_eq!(s.streak, -1);
    }

    #[test]
    fn cra_promotes_after_3_no_hint_correct() {
        let mut s = LearnerProfile::new();
        for _ in 0..3 {
            s = learner_reducer(s, attempt(true));
        }
        assert_eq!(*s.cra_stages.get(&Operation::Add).unwrap(), CraStage::Representational);
    }

    #[test]
    fn cra_does_not_promote_above_abstract() {
        let mut s = LearnerProfile::new();
        s.cra_stages.insert(Operation::Add, CraStage::Abstract);
        for _ in 0..5 {
            s = learner_reducer(s, attempt(true));
        }
        assert_eq!(*s.cra_stages.get(&Operation::Add).unwrap(), CraStage::Abstract);
    }

    #[test]
    fn cra_demotes_on_hint_used() {
        let mut s = LearnerProfile::new();
        s.cra_stages.insert(Operation::Add, CraStage::Abstract);
        s = learner_reducer(s, LearnerEvent::PuzzleAttempted {
            correct: true,
            operation: Operation::Add,
            sub_skill: None,
            band: 1,
            center_band: Some(1),
            response_time_ms: Some(2000.0),
            hint_used: true,
            told_me: false,
            cra_level_shown: Some(CraStage::Representational),
            timestamp: None,
        });
        assert_eq!(*s.cra_stages.get(&Operation::Add).unwrap(), CraStage::Representational);
    }

    #[test]
    fn frustration_drops_band() {
        let mut s = LearnerProfile::new();
        s.math_band = 5;
        s = learner_reducer(s, LearnerEvent::FrustrationDetected { level: "high".into() });
        assert_eq!(s.math_band, 4);
        assert_eq!(s.wrongs_before_teach, 1);
    }

    #[test]
    fn intake_sets_dials() {
        let s = learner_reducer(LearnerProfile::new(), LearnerEvent::IntakeCompleted {
            math_band: 5,
            pace: 0.7,
            scaffolding: 0.3,
            promote_threshold: 0.65,
            stretch_threshold: 0.50,
            text_speed: 0.02,
        });
        assert_eq!(s.math_band, 5);
        assert_eq!(s.pace, 0.7);
        assert!(s.intake_completed);
    }

    #[test]
    fn immutability() {
        let s1 = LearnerProfile::new();
        let s2 = learner_reducer(s1.clone(), attempt(true));
        assert_eq!(s1.streak, 0);
        assert_eq!(s2.streak, 1);
    }
}
