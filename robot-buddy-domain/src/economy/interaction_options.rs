use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub can_receive_gifts: Option<bool>,
    #[serde(default)]
    pub has_shop: Option<bool>,
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
    if can_receive && player_state.dum_dums > 0 {
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

    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_includes_talk() {
        let opts = get_interaction_options(
            &NpcInfo { id: "robot".into(), can_receive_gifts: None, has_shop: None },
            &PlayerState { dum_dums: 0 },
        );
        assert_eq!(opts[0].option_type, "talk");
    }

    #[test]
    fn includes_give_when_has_dum_dums() {
        let opts = get_interaction_options(
            &NpcInfo { id: "robot".into(), can_receive_gifts: None, has_shop: None },
            &PlayerState { dum_dums: 3 },
        );
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[1].option_type, "give");
    }

    #[test]
    fn excludes_give_when_zero_dum_dums() {
        let opts = get_interaction_options(
            &NpcInfo { id: "robot".into(), can_receive_gifts: None, has_shop: None },
            &PlayerState { dum_dums: 0 },
        );
        assert_eq!(opts.len(), 1);
    }

    #[test]
    fn excludes_give_when_cant_receive() {
        let opts = get_interaction_options(
            &NpcInfo { id: "chest".into(), can_receive_gifts: Some(false), has_shop: None },
            &PlayerState { dum_dums: 5 },
        );
        assert_eq!(opts.len(), 1);
    }
}
