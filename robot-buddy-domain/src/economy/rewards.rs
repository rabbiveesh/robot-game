use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reward {
    pub reward_type: String,
    pub amount: u32,
}

/// A Dum Dum is EARNED by showing you know it: solved, with zero mistakes.
/// Completion always celebrates regardless (rule 7 — wrong answers are never
/// punished), but currency must not pay for exhausting the choices — a
/// four-year-old will happily grind any reward that pays for guessing.
/// `mistakes` is the activity's own error count before the solve: wrong
/// guesses on a challenge/balance/pattern, constraint violations on a grid.
/// This is THE payout rule; every solvable activity routes through it.
pub fn determine_reward(correct: bool, mistakes: u32) -> Option<Reward> {
    if correct && mistakes == 0 {
        Some(Reward { reward_type: "dum_dum".into(), amount: 1 })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_first_try_returns_reward() {
        let r = determine_reward(true, 0).unwrap();
        assert_eq!(r.reward_type, "dum_dum");
        assert_eq!(r.amount, 1);
    }

    #[test]
    fn wrong_returns_none() {
        assert!(determine_reward(false, 0).is_none());
    }

    #[test]
    fn guessed_down_solve_pays_nothing() {
        // Right answer, but only after wrong tries — celebrate, don't pay.
        assert!(determine_reward(true, 1).is_none());
        assert!(determine_reward(true, 7).is_none());
    }
}
