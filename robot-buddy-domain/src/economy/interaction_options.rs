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
    /// True for the keeper of a dive shaft (Inkwell). She gets a "Dive!" option
    /// alongside the usual ones — she's still a buddy you can feed and dress up,
    /// she just also runs the way down.
    #[serde(default)]
    pub runs_dive: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    #[serde(default)]
    pub dum_dums: u32,
    /// How many pieces of shop swag the kid is wearing right now. Anything
    /// they're wearing can be handed to a buddy, so a non-zero count is what
    /// puts the "Give Swag" option on the menu.
    #[serde(default)]
    pub swag_worn: u32,
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

    // Swag changes hands to anyone who'd take a Dum Dum — dressing up your
    // buddy is the same gesture as feeding them, just sillier.
    if can_receive && player_state.swag_worn > 0 {
        let key = (options.len() + 1).to_string();
        options.push(InteractionOption {
            option_type: "swag".into(),
            label: "Give Swag".into(),
            key,
        });
    }

    if npc.runs_dive.unwrap_or(false) {
        let key = (options.len() + 1).to_string();
        options.push(InteractionOption {
            option_type: "dive".into(),
            label: "Dive!".into(),
            key,
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
        let key = (options.len() + 1).to_string();
        options.push(InteractionOption {
            option_type: "sudoku".into(),
            label: "Animal Sudoku".into(),
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
            &NpcInfo { id: "robot".into(), can_receive_gifts: None, has_shop: None, is_puzzler: None, runs_dive: None },
            &PlayerState { dum_dums: 0, swag_worn: 0 },
        );
        assert_eq!(opts[0].option_type, "talk");
    }

    #[test]
    fn includes_give_when_has_dum_dums() {
        let opts = get_interaction_options(
            &NpcInfo { id: "robot".into(), can_receive_gifts: None, has_shop: None, is_puzzler: None, runs_dive: None },
            &PlayerState { dum_dums: 3, swag_worn: 0 },
        );
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[1].option_type, "give");
    }

    #[test]
    fn excludes_give_when_zero_dum_dums() {
        let opts = get_interaction_options(
            &NpcInfo { id: "robot".into(), can_receive_gifts: None, has_shop: None, is_puzzler: None, runs_dive: None },
            &PlayerState { dum_dums: 0, swag_worn: 0 },
        );
        assert_eq!(opts.len(), 1);
    }

    #[test]
    fn excludes_give_when_cant_receive() {
        let opts = get_interaction_options(
            &NpcInfo { id: "chest".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: None, runs_dive: None },
            &PlayerState { dum_dums: 5, swag_worn: 0 },
        );
        assert_eq!(opts.len(), 1);
    }

    #[test]
    fn includes_puzzle_when_npc_is_puzzler() {
        let opts = get_interaction_options(
            &NpcInfo { id: "sage".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: Some(true), runs_dive: None },
            &PlayerState { dum_dums: 0, swag_worn: 0 },
        );
        assert!(opts.iter().any(|o| o.option_type == "puzzle"),
            "puzzler NPCs should expose a 'puzzle' option, got: {:?}",
            opts.iter().map(|o| &o.option_type).collect::<Vec<_>>());
    }

    #[test]
    fn includes_pattern_when_npc_is_puzzler() {
        let opts = get_interaction_options(
            &NpcInfo { id: "sage".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: Some(true), runs_dive: None },
            &PlayerState { dum_dums: 0, swag_worn: 0 },
        );
        assert!(opts.iter().any(|o| o.option_type == "pattern"),
            "puzzler NPCs should expose a 'pattern' option, got: {:?}",
            opts.iter().map(|o| &o.option_type).collect::<Vec<_>>());
    }

    #[test]
    fn non_puzzler_has_no_pattern_option() {
        let opts = get_interaction_options(
            &NpcInfo { id: "robot".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: Some(false), runs_dive: None },
            &PlayerState { dum_dums: 0, swag_worn: 0 },
        );
        assert!(!opts.iter().any(|o| o.option_type == "pattern"));
    }

    #[test]
    fn includes_swag_when_wearing_something_giftable() {
        let opts = get_interaction_options(
            &NpcInfo { id: "dolphin".into(), can_receive_gifts: None, has_shop: None, is_puzzler: None, runs_dive: None },
            &PlayerState { dum_dums: 0, swag_worn: 2 },
        );
        assert!(opts.iter().any(|o| o.option_type == "swag"),
            "wearing swag should offer to hand it over, got: {:?}",
            opts.iter().map(|o| &o.option_type).collect::<Vec<_>>());
    }

    #[test]
    fn excludes_swag_when_wearing_nothing() {
        let opts = get_interaction_options(
            &NpcInfo { id: "dolphin".into(), can_receive_gifts: None, has_shop: None, is_puzzler: None, runs_dive: None },
            &PlayerState { dum_dums: 0, swag_worn: 0 },
        );
        assert!(!opts.iter().any(|o| o.option_type == "swag"));
    }

    #[test]
    fn excludes_swag_for_someone_who_takes_no_gifts() {
        let opts = get_interaction_options(
            &NpcInfo { id: "chest".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: None, runs_dive: None },
            &PlayerState { dum_dums: 5, swag_worn: 3 },
        );
        assert!(!opts.iter().any(|o| o.option_type == "swag"));
    }

    #[test]
    fn the_dive_keeper_still_takes_gifts_and_swag() {
        // Inkwell runs the shaft, but she's a reef buddy first — short-cutting
        // straight to the dive would mean you could never hand her a hat.
        let opts = get_interaction_options(
            &NpcInfo {
                id: "octopus".into(),
                can_receive_gifts: Some(true),
                has_shop: None,
                is_puzzler: None,
                runs_dive: Some(true),
            },
            &PlayerState { dum_dums: 4, swag_worn: 1 },
        );
        let kinds: Vec<&str> = opts.iter().map(|o| o.option_type.as_str()).collect();
        assert!(kinds.contains(&"dive"), "she has to offer the way down: {kinds:?}");
        assert!(kinds.contains(&"give"), "...and still take a Dum Dum: {kinds:?}");
        assert!(kinds.contains(&"swag"), "...and still take a hat: {kinds:?}");
        let keys: Vec<&str> = opts.iter().map(|o| o.key.as_str()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len(), "menu keys must be unique: {keys:?}");
    }

    #[test]
    fn includes_balance_when_npc_is_puzzler() {
        let opts = get_interaction_options(
            &NpcInfo { id: "sage".into(), can_receive_gifts: Some(false), has_shop: None, is_puzzler: Some(true), runs_dive: None },
            &PlayerState { dum_dums: 0, swag_worn: 0 },
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
