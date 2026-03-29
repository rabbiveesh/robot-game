use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Operation, SubSkill};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatEntry {
    pub correct: u32,
    pub attempts: u32,
}

impl StatEntry {
    pub fn zero() -> Self {
        StatEntry {
            correct: 0,
            attempts: 0,
        }
    }

    pub fn accuracy(&self) -> Option<f64> {
        if self.attempts == 0 {
            None
        } else {
            Some(self.correct as f64 / self.attempts as f64)
        }
    }

    fn bump(&self, correct: bool) -> Self {
        StatEntry {
            correct: self.correct + if correct { 1 } else { 0 },
            attempts: self.attempts + 1,
        }
    }
}

/// Tracks both coarse (per-operation) and fine (per-sub-skill) stats.
/// Nested: { coarse: { add: {...} }, fine: { addSingle: {...} } }
/// JS adapter accesses via profileState.operationStats.coarse.add
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationStats {
    pub coarse: HashMap<Operation, StatEntry>,
    pub fine: HashMap<SubSkill, StatEntry>,
}

impl OperationStats {
    pub fn new() -> Self {
        use Operation::*;
        use SubSkill::*;

        let mut coarse = HashMap::new();
        for op in [Add, Sub, Multiply, Divide, NumberBond] {
            coarse.insert(op, StatEntry::zero());
        }

        let mut fine = HashMap::new();
        for sk in [
            AddSingle, AddNoCarry, AddCarry, AddCarryTens,
            SubSingle, SubNoBorrow, SubBorrow, SubBorrowTens,
            MulTrivial, MulEasy, MulHard,
            DivEasy, DivHard,
            BondSmall, BondLarge,
        ] {
            fine.insert(sk, StatEntry::zero());
        }

        OperationStats { coarse, fine }
    }

    pub fn record(&self, operation: Operation, correct: bool, sub_skill: Option<SubSkill>) -> Self {
        let mut coarse = self.coarse.clone();
        coarse
            .entry(operation)
            .and_modify(|s| *s = s.bump(correct));

        let mut fine = self.fine.clone();
        if let Some(sk) = sub_skill {
            fine.entry(sk).and_modify(|s| *s = s.bump(correct));
        }

        OperationStats { coarse, fine }
    }

    pub fn get_coarse(&self, op: Operation) -> &StatEntry {
        self.coarse.get(&op).unwrap_or(&ZERO_STAT)
    }

    pub fn get_fine(&self, sk: SubSkill) -> &StatEntry {
        self.fine.get(&sk).unwrap_or(&ZERO_STAT)
    }
}

static ZERO_STAT: StatEntry = StatEntry {
    correct: 0,
    attempts: 0,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Operation, SubSkill};

    #[test]
    fn new_stats_all_zero() {
        let stats = OperationStats::new();
        assert_eq!(stats.get_coarse(Operation::Add).attempts, 0);
        assert_eq!(stats.get_fine(SubSkill::AddCarry).attempts, 0);
    }

    #[test]
    fn records_coarse_and_fine() {
        let stats = OperationStats::new();
        let stats = stats.record(Operation::Add, true, Some(SubSkill::AddCarry));
        assert_eq!(stats.get_coarse(Operation::Add).correct, 1);
        assert_eq!(stats.get_coarse(Operation::Add).attempts, 1);
        assert_eq!(stats.get_fine(SubSkill::AddCarry).correct, 1);
        assert_eq!(stats.get_fine(SubSkill::AddCarry).attempts, 1);
    }

    #[test]
    fn records_coarse_without_sub_skill() {
        let stats = OperationStats::new();
        let stats = stats.record(Operation::Sub, false, None);
        assert_eq!(stats.get_coarse(Operation::Sub).correct, 0);
        assert_eq!(stats.get_coarse(Operation::Sub).attempts, 1);
    }

    #[test]
    fn tracks_independently() {
        let stats = OperationStats::new();
        let stats = stats.record(Operation::Add, true, Some(SubSkill::AddSingle));
        let stats = stats.record(Operation::Add, true, Some(SubSkill::AddSingle));
        let stats = stats.record(Operation::Add, false, Some(SubSkill::AddCarry));
        assert_eq!(stats.get_coarse(Operation::Add).correct, 2);
        assert_eq!(stats.get_coarse(Operation::Add).attempts, 3);
        assert_eq!(stats.get_fine(SubSkill::AddSingle).correct, 2);
        assert_eq!(stats.get_fine(SubSkill::AddSingle).attempts, 2);
        assert_eq!(stats.get_fine(SubSkill::AddCarry).correct, 0);
        assert_eq!(stats.get_fine(SubSkill::AddCarry).attempts, 1);
    }

    #[test]
    fn immutability() {
        let s1 = OperationStats::new();
        let s2 = s1.record(Operation::Add, true, None);
        assert_eq!(s1.get_coarse(Operation::Add).attempts, 0);
        assert_eq!(s2.get_coarse(Operation::Add).attempts, 1);
    }
}
