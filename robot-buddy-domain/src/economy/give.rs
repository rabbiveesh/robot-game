use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub recipient_id: String,
    pub total: u32,
    pub reaction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiveResult {
    pub new_dum_dums: u32,
    pub new_total_gifts: HashMap<String, u32>,
    pub milestone: Option<Milestone>,
}

const MILESTONES: &[(u32, &str)] = &[
    (1, "first"), (5, "spin"), (10, "accessory"), (20, "color_change"), (50, "ultimate"),
];

pub fn can_give(dum_dums: u32) -> bool {
    dum_dums > 0
}

pub fn process_give(dum_dums: u32, recipient_id: &str, total_gifts: &HashMap<String, u32>) -> Option<GiveResult> {
    if dum_dums == 0 { return None; }
    let new_total = total_gifts.get(recipient_id).copied().unwrap_or(0) + 1;
    let mut new_gifts = total_gifts.clone();
    new_gifts.insert(recipient_id.to_string(), new_total);

    let milestone = MILESTONES.iter()
        .rev()
        .find(|(count, _)| *count == new_total)
        .map(|(_, reaction)| Milestone {
            recipient_id: recipient_id.to_string(),
            total: new_total,
            reaction: reaction.to_string(),
        });

    Some(GiveResult {
        new_dum_dums: dum_dums - 1,
        new_total_gifts: new_gifts,
        milestone,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decrements_dum_dums() {
        let r = process_give(5, "robot", &HashMap::new()).unwrap();
        assert_eq!(r.new_dum_dums, 4);
    }

    #[test]
    fn first_milestone() {
        let r = process_give(5, "robot", &HashMap::new()).unwrap();
        assert_eq!(r.milestone.as_ref().unwrap().reaction, "first");
    }

    #[test]
    fn none_when_broke() {
        assert!(process_give(0, "robot", &HashMap::new()).is_none());
    }
}
