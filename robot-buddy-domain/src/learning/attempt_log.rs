//! The attempt log: one detailed record per challenge, kept across play
//! sessions so a parent can tell whether the help is working.
//!
//! The learner profile already keeps a rolling window of the last 20 answers
//! for the adaptive system. That's too short, and too thin, to answer "is he
//! learning?" This log keeps what the window drops: the numbers, every answer
//! given and when, which help was used, and what the kid did in the hands-on
//! workspace. `attempt_analysis` turns it into a report.
//!
//! Bounded: the newest [`MAX_ATTEMPTS`] records are kept (a few weeks of
//! steady play), so the save stays small.

use serde::{Deserialize, Serialize};

use crate::types::{CraStage, Operation, SubSkill};

/// How many records the log keeps (newest win). At ~300 bytes each, about
/// 300KB of save per kid.
pub const MAX_ATTEMPTS: usize = 1000;

/// What "Show me" put on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Help {
    /// No Show me.
    None,
    /// The static CRA picture.
    Picture,
    /// The hands-on workspace (see `workspace` for what was done in it).
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceKind {
    PutTogether,
    TakeAway,
}

/// What the kid did in the hands-on workspace.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceUse {
    pub kind: Option<WorkspaceKind>,
    /// Milliseconds from opening the workspace to the model being complete;
    /// `None` if it was never finished.
    pub built_ms: Option<u32>,
    /// Counters moved one at a time.
    pub singles: u16,
    /// Of those, how many were moved while a whole row of five was there to
    /// take — the "could have grouped" count.
    pub singles_past_a_row: u16,
    /// Whole rows of five moved as one.
    pub rows: u16,
    /// Full frames snapped into a ten-rod.
    pub rods_snapped: u16,
    /// Ten-rods opened back into a frame.
    pub rods_opened: u16,
    /// Carried counters let go off target (they slid home) — motor friction.
    pub misses: u16,
}

/// One answer, and how long after the question appeared it came.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerAt {
    pub value: i32,
    pub ms: u32,
}

/// Everything about one challenge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptRecord {
    /// Wall clock when it was resolved, in Unix seconds (0 if unknown).
    pub at: f64,
    /// The save's cumulative play time then, in seconds.
    pub play_secs: f32,
    /// Who asked (NPC id, "sparky", …).
    pub source: String,
    pub operation: Operation,
    pub sub_skill: Option<SubSkill>,
    pub a: i32,
    pub b: i32,
    /// "standard" or "bond".
    pub format: String,
    /// The band the numbers were drawn from, and the learner's band then.
    pub band: u8,
    pub center_band: u8,
    /// The learner's CRA stage for this operation when it was asked.
    pub cra_stage: CraStage,
    pub correct_answer: i32,
    /// Every answer given, in order.
    pub answers: Vec<AnswerAt>,
    /// Ended solved (rather than taught).
    pub correct: bool,
    pub help: Help,
    /// When Show me was pressed, ms after the question appeared.
    pub help_ms: Option<u32>,
    pub told_me: bool,
    pub workspace: Option<WorkspaceUse>,
}

impl AttemptRecord {
    /// No Show me, no Tell me: the kid's own work.
    pub fn unaided(&self) -> bool {
        self.help == Help::None && !self.told_me
    }

    /// The first answer was right.
    pub fn first_try_correct(&self) -> bool {
        self.answers.first().is_some_and(|a| a.value == self.correct_answer)
    }

    /// How long the first answer took.
    pub fn first_answer_ms(&self) -> Option<u32> {
        self.answers.first().map(|a| a.ms)
    }

    /// Identity for de-duplicating records that appear in several exports.
    pub fn key(&self) -> (u64, u32, i32, i32) {
        (self.at.to_bits(), self.play_secs.to_bits(), self.a, self.b)
    }
}

/// The bounded, persistent log. Only ever grows through [`AttemptLog::record`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttemptLog {
    records: Vec<AttemptRecord>,
}

impl AttemptLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// The log with `record` added, dropping the oldest past [`MAX_ATTEMPTS`].
    pub fn record(mut self, record: AttemptRecord) -> Self {
        self.records.push(record);
        if self.records.len() > MAX_ATTEMPTS {
            let extra = self.records.len() - MAX_ATTEMPTS;
            self.records.drain(..extra);
        }
        self
    }

    pub fn records(&self) -> &[AttemptRecord] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A plain unaided addition, for tests here and in `attempt_analysis`.
    pub(crate) fn attempt(a: i32, b: i32, answer: i32, ms: u32) -> AttemptRecord {
        AttemptRecord {
            at: 1_790_000_000.0,
            play_secs: 0.0,
            source: "sparky".into(),
            operation: Operation::Add,
            sub_skill: None,
            a,
            b,
            format: "standard".into(),
            band: 2,
            center_band: 2,
            cra_stage: CraStage::Concrete,
            correct_answer: a + b,
            answers: vec![AnswerAt { value: answer, ms }],
            correct: answer == a + b,
            help: Help::None,
            help_ms: None,
            told_me: false,
            workspace: None,
        }
    }

    #[test]
    fn the_log_keeps_the_newest_records() {
        let mut log = AttemptLog::new();
        for i in 0..(MAX_ATTEMPTS + 5) {
            log = log.record(attempt(i as i32, 1, i as i32 + 1, 1000));
        }
        assert_eq!(log.len(), MAX_ATTEMPTS);
        assert_eq!(log.records()[0].a, 5, "the five oldest fell off");
    }

    #[test]
    fn unaided_means_no_show_me_and_no_tell_me() {
        let mut r = attempt(3, 2, 5, 2000);
        assert!(r.unaided() && r.first_try_correct());
        r.help = Help::Workspace;
        assert!(!r.unaided());
        r.help = Help::None;
        r.told_me = true;
        assert!(!r.unaided());
    }

    #[test]
    fn records_round_trip_through_json() {
        let mut r = attempt(8, 5, 13, 4200);
        r.help = Help::Workspace;
        r.workspace = Some(WorkspaceUse { kind: Some(WorkspaceKind::PutTogether), rows: 1, ..Default::default() });
        let log = AttemptLog::new().record(r);
        let json = serde_json::to_string(&log).unwrap();
        assert!(json.starts_with('['), "the log serializes as a plain array");
        let back: AttemptLog = serde_json::from_str(&json).unwrap();
        assert_eq!(back, log);
    }
}
