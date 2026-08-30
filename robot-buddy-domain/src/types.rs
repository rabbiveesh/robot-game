use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Add,
    Sub,
    Multiply,
    Divide,
    NumberBond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubSkill {
    AddSingle,
    AddNoCarry,
    AddCarry,
    AddCarryTens,
    SubSingle,
    SubNoBorrow,
    SubBorrow,
    SubBorrowTens,
    MulTrivial,
    MulEasy,
    MulHard,
    DivEasy,
    DivHard,
    BondSmall,
    BondLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CraStage {
    Concrete,
    Representational,
    Abstract,
}

impl CraStage {
    pub fn order(&self) -> u8 {
        match self {
            CraStage::Concrete => 0,
            CraStage::Representational => 1,
            CraStage::Abstract => 2,
        }
    }

    pub fn next(&self) -> CraStage {
        match self {
            CraStage::Concrete => CraStage::Representational,
            CraStage::Representational => CraStage::Abstract,
            CraStage::Abstract => CraStage::Abstract,
        }
    }

    /// One stage more concrete (toward manipulatives). Saturates at Concrete.
    pub fn prev(&self) -> CraStage {
        match self {
            CraStage::Abstract => CraStage::Representational,
            CraStage::Representational => CraStage::Concrete,
            CraStage::Concrete => CraStage::Concrete,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrustrationLevel {
    None,
    Mild,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Presented,
    Feedback,
    Teaching,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySpeech {
    pub display: String,
    pub speech: String,
}

/// How fast the real-time arcade cabinet runs. This is a *pace* dial, not a
/// difficulty one — it never touches which numbers a kid is asked for, only how
/// long they have to think while the aliens drift down. A kid who can do the
/// maths but not at speed should be slowed down, not moved to easier bonds.
///
/// Parent-facing only (Invariant 6): it lives in the parent section of the
/// settings overlay and the child never sees a label for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamePace {
    /// Roughly half speed — a full descent takes ~25s instead of ~13s.
    Relaxed,
    /// The pace the cabinet shipped with.
    Steady,
    /// For a kid who's outgrown Steady and wants the arcade to bite.
    Brisk,
}

impl Default for GamePace {
    fn default() -> Self {
        GamePace::Steady
    }
}

impl GamePace {
    /// Every pace, slowest first — the order the parent panel lists them in.
    pub const ALL: [GamePace; 3] = [GamePace::Relaxed, GamePace::Steady, GamePace::Brisk];

    /// Scales how fast the aliens drift down. This is the dial that actually
    /// decides whether the game feels frantic.
    pub fn drift_multiplier(self) -> f32 {
        match self {
            GamePace::Relaxed => 0.55,
            GamePace::Steady => 1.0,
            GamePace::Brisk => 1.35,
        }
    }

    /// Scales how fast the ship glides. Relaxed nudges it up as well: a kid who
    /// needs more thinking time usually needs more aiming time too, and a
    /// nippier ship is help, never a handicap.
    pub fn ship_multiplier(self) -> f32 {
        match self {
            GamePace::Relaxed => 1.15,
            GamePace::Steady | GamePace::Brisk => 1.0,
        }
    }

    /// Shown in the parent panel only.
    pub fn label(self) -> &'static str {
        match self {
            GamePace::Relaxed => "Relaxed",
            GamePace::Steady => "Steady",
            GamePace::Brisk => "Brisk",
        }
    }
}

#[cfg(test)]
mod pace_tests {
    use super::*;

    #[test]
    fn steady_is_the_default_and_changes_nothing() {
        assert_eq!(GamePace::default(), GamePace::Steady);
        assert_eq!(GamePace::Steady.drift_multiplier(), 1.0);
        assert_eq!(GamePace::Steady.ship_multiplier(), 1.0);
    }

    #[test]
    fn the_dial_only_ever_goes_one_way() {
        let m: Vec<f32> = GamePace::ALL.iter().map(|p| p.drift_multiplier()).collect();
        for w in m.windows(2) {
            assert!(w[0] < w[1], "paces must be listed slowest first: {m:?}");
        }
        assert!(GamePace::Relaxed.drift_multiplier() < 0.6,
            "Relaxed has to be a real difference, not a nudge");
        // Slowing the drift must never also slow the ship.
        for p in GamePace::ALL {
            assert!(p.ship_multiplier() >= 1.0, "{p:?} would handicap the ship");
        }
    }
}
