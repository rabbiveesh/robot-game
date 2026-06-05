use serde::{Deserialize, Serialize};
use super::give::can_give;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub can_receive_gifts: Option<bool>,
    #[serde(default)]
    pub has_shop: Option<bool>,
    #[serde(default)]
    pub is_puzzler: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    #[serde(default)]
    pub dum_dums: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionOption {
    #[serde(rename = "type")]
    pub option_type: String,
    pub label: String,
    pub key: String,
}

pub fn get_interaction_options(npc: &NpcInfo, player_state: &PlayerState) -> Vec<InteractionOption> {
    let mut options = vec![InteractionOption {
        option_type: "talk".into(),
        label: "Talk".into(),
        key: "1".into(),
    }];

    let can_receive = npc.can_receive_gifts.unwrap_or(true);
    if can_receive && can_give(player_state.dum_dums) {
        options.push(InteractionOption {
            option_type: "give".into(),
            label: "Give Dum Dum".into(),
            key: "2".into(),
        });
    }

    if npc.has_shop.unwrap_or(false) {
        let key = (options.len() + 1).to_string();
        options.push(InteractionOption {
            option_type: "shop".into(),
            label: "Buy".into(),
            key,
        });
    }

    if npc.is_puzzler.unwrap_or(false) {
        let key = (options.len() + 1).to_string();
        options.push(InteractionOption {
            option_type: "puzzle".into(),
            label: "Try a Puzzle".into(),
            key,
        });
        let key = (options.len() + 1).to_string();
        options.push(InteractionOption {
            option_type: "pattern".into(),
            label: "Spot the Pattern".into(),
            key,
        });
        let key = (options.len() + 1).to_string();
        options.push(InteractionOption {
            option_type: "balance".into(),
            label: "Balance the Scale".into(),
            key,
        });
    }

    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_includes_talk() {
        let opts = get_interaction_options(
            &NpcInfo { id: "robot".into(), can_receive_gifts: None, has_shop: None, is_puzzler: None },
            &PlayerState { dum_dums: 0 },
        );
        assert_eq!(opts[0].option_type, "talk");
    }

    #[test]
    fn includes_give_when_has_dum_dums() {
        let opts = get_interaction_options(
            &NpcInfo { id: "robot".into(), can_receive_gifts: None, has_shop: None, is_puzzler: None },
            &PlayerState { dum_dums: 3 },
        );
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[1].option_type, "give");
    }

    #[test]
    fn excludes_give_when_zero_dum_dums() {
        let opts = get_interaction_options(
            &NpcInfo { id: "robot".into(), can_receive_gifts: None, has_shop: None, is_puzzler: None },
            &PlayerState { dum_dums: 0 },
        );
        assert_eq!(opts.len(), 1);
    }

    #[test]
    fn excludes_give_when_cant_receive() {
        let opts = get_interaction_options(
            &NpcInfo { id: "chest".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: None },
            &PlayerState { dum_dums: 5 },
        );
        assert_eq!(opts.len(), 1);
    }

    #[test]
    fn includes_puzzle_when_npc_is_puzzler() {
        let opts = get_interaction_options(
            &NpcInfo { id: "sage".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: Some(true) },
            &PlayerState { dum_dums: 0 },
        );
        assert!(opts.iter().any(|o| o.option_type == "puzzle"),
            "puzzler NPCs should expose a 'puzzle' option, got: {:?}",
            opts.iter().map(|o| &o.option_type).collect::<Vec<_>>());
    }

    #[test]
    fn includes_pattern_when_npc_is_puzzler() {
        let opts = get_interaction_options(
            &NpcInfo { id: "sage".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: Some(true) },
            &PlayerState { dum_dums: 0 },
        );
        assert!(opts.iter().any(|o| o.option_type == "pattern"),
            "puzzler NPCs should expose a 'pattern' option, got: {:?}",
            opts.iter().map(|o| &o.option_type).collect::<Vec<_>>());
    }

    #[test]
    fn non_puzzler_has_no_pattern_option() {
        let opts = get_interaction_options(
            &NpcInfo { id: "robot".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: Some(false) },
            &PlayerState { dum_dums: 0 },
        );
        assert!(!opts.iter().any(|o| o.option_type == "pattern"));
    }

    #[test]
    fn includes_balance_when_npc_is_puzzler() {
        let opts = get_interaction_options(
            &NpcInfo { id: "sage".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: Some(true) },
            &PlayerState { dum_dums: 0 },
        );
        assert!(opts.iter().any(|o| o.option_type == "balance"),
            "puzzler NPCs should expose a 'balance' option, got: {:?}",
            opts.iter().map(|o| &o.option_type).collect::<Vec<_>>());
        // Keys stay unique and sequential even with the extra puzzle options.
        let keys: Vec<&str> = opts.iter().map(|o| o.key.as_str()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len(), "menu keys must be unique: {keys:?}");
    }
}
