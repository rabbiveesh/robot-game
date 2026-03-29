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
pub struct DisplaySpeech {
    pub display: String,
    pub speech: String,
}
