use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reward {
    pub reward_type: String,
    pub amount: u32,
}

pub fn determine_reward(correct: bool) -> Option<Reward> {
    if correct {
        Some(Reward { reward_type: "dum_dum".into(), amount: 1 })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_returns_reward() {
        let r = determine_reward(true).unwrap();
        assert_eq!(r.reward_type, "dum_dum");
        assert_eq!(r.amount, 1);
    }

    #[test]
    fn wrong_returns_none() {
        assert!(determine_reward(false).is_none());
    }
}
