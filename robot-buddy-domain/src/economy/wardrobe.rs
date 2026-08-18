//! Who's wearing which swag. Pure bookkeeping — no rendering, no game state.
//!
//! Cosmetics bought from Bolt's shop don't have to stay on the kid. Any piece
//! the kid is wearing can be handed to a buddy, and it stays on that buddy
//! forever — including after they're swapped out for a different one. That's
//! what makes the shop reusable: once the hat is on Echo the dolphin, the kid
//! isn't wearing a hat any more, so they can go buy another one.
//!
//! Everyone who can wear something is identified by the same stable id strings
//! the rest of the game uses (`NpcKind::as_str()`, `"sparky"`), plus [`PLAYER`]
//! for the kid themselves. One map, one rule, no special cases.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// Wearer id for the kid. Every other wearer uses their NPC id string.
pub const PLAYER: &str = "player";

/// What happened when swag changed hands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandOver {
    /// The item moved: `from` took it off, `to` put it on.
    Given,
    /// `from` isn't wearing that item, so there's nothing to hand over.
    NotWorn,
    /// `to` already has one of those. Nobody wears two hats.
    AlreadyWearing,
}

/// Who wears what, for everyone in the world at once.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Wardrobe {
    /// wearer id → the item ids they're wearing. Sorted maps so a save file
    /// round-trips byte-identically and tests don't chase hash order.
    worn: BTreeMap<String, BTreeSet<String>>,
}

fn empty_set() -> &'static BTreeSet<String> {
    static EMPTY: OnceLock<BTreeSet<String>> = OnceLock::new();
    EMPTY.get_or_init(BTreeSet::new)
}

impl Wardrobe {
    pub fn new() -> Self {
        Wardrobe::default()
    }

    /// Everything `who` is wearing. Empty for anyone who's never been given
    /// anything — an unknown wearer isn't an error, they're just plain.
    pub fn worn_by(&self, who: &str) -> &BTreeSet<String> {
        self.worn.get(who).unwrap_or_else(|| empty_set())
    }

    pub fn is_wearing(&self, who: &str, item: &str) -> bool {
        self.worn.get(who).is_some_and(|s| s.contains(item))
    }

    /// True when nobody in the world is wearing anything.
    pub fn is_empty(&self) -> bool {
        self.worn.values().all(|s| s.is_empty())
    }

    /// Put `item` on `who`. Returns false if they already had one.
    pub fn put_on(&mut self, who: &str, item: &str) -> bool {
        self.worn.entry(who.to_string()).or_default().insert(item.to_string())
    }

    /// Take `item` off `who`. Returns false if they weren't wearing it.
    pub fn take_off(&mut self, who: &str, item: &str) -> bool {
        let Some(set) = self.worn.get_mut(who) else { return false };
        let had = set.remove(item);
        if set.is_empty() {
            self.worn.remove(who);
        }
        had
    }

    /// Replace everything `who` wears in one shot. Used when a shop session
    /// closes and hands back the kid's updated outfit.
    pub fn set_worn<I: IntoIterator<Item = String>>(&mut self, who: &str, items: I) {
        let set: BTreeSet<String> = items.into_iter().collect();
        if set.is_empty() {
            self.worn.remove(who);
        } else {
            self.worn.insert(who.to_string(), set);
        }
    }

    /// Move one piece of swag from one wearer to another. The giver takes it
    /// off in the same breath the receiver puts it on — swag is never in two
    /// places, which is exactly why the shop can sell you another one.
    pub fn hand_over(&mut self, from: &str, to: &str, item: &str) -> HandOver {
        if !self.is_wearing(from, item) {
            return HandOver::NotWorn;
        }
        if from != to && self.is_wearing(to, item) {
            return HandOver::AlreadyWearing;
        }
        if from == to {
            return HandOver::Given; // handing it to yourself is a no-op, not an error
        }
        self.take_off(from, item);
        self.put_on(to, item);
        HandOver::Given
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kid_with_hat() -> Wardrobe {
        let mut w = Wardrobe::new();
        w.put_on(PLAYER, "hat");
        w
    }

    #[test]
    fn a_fresh_wardrobe_dresses_nobody() {
        let w = Wardrobe::new();
        assert!(w.is_empty());
        assert!(w.worn_by(PLAYER).is_empty());
        assert!(w.worn_by("dolphin").is_empty(), "unknown wearers are just plain");
    }

    #[test]
    fn giving_swag_away_moves_it_off_the_giver() {
        let mut w = kid_with_hat();
        assert_eq!(w.hand_over(PLAYER, "dolphin", "hat"), HandOver::Given);
        assert!(w.is_wearing("dolphin", "hat"), "Echo should be wearing the hat");
        assert!(!w.is_wearing(PLAYER, "hat"),
            "the kid gave it away, so the shop can sell them another one");
    }

    #[test]
    fn cannot_give_what_you_are_not_wearing() {
        let mut w = kid_with_hat();
        assert_eq!(w.hand_over(PLAYER, "dolphin", "jet_boots"), HandOver::NotWorn);
        assert!(w.worn_by("dolphin").is_empty());
    }

    #[test]
    fn nobody_wears_two_hats() {
        let mut w = kid_with_hat();
        w.put_on("dolphin", "hat");
        assert_eq!(w.hand_over(PLAYER, "dolphin", "hat"), HandOver::AlreadyWearing);
        assert!(w.is_wearing(PLAYER, "hat"), "a refused hand-over changes nothing");
    }

    #[test]
    fn swag_stays_on_a_buddy_who_is_swapped_out() {
        // The whole point: the wardrobe is keyed by who, not by who's active.
        let mut w = kid_with_hat();
        w.hand_over(PLAYER, "dolphin", "hat");
        // ...kid recruits someone else entirely, dresses them too...
        w.put_on(PLAYER, "bow_tie");
        w.hand_over(PLAYER, "pip", "bow_tie");
        assert!(w.is_wearing("dolphin", "hat"), "Echo keeps her hat while off-duty");
        assert!(w.is_wearing("pip", "bow_tie"));
        assert!(w.worn_by(PLAYER).is_empty());
    }

    #[test]
    fn taking_the_last_item_off_forgets_the_wearer() {
        let mut w = kid_with_hat();
        assert!(w.take_off(PLAYER, "hat"));
        assert!(!w.take_off(PLAYER, "hat"), "already off");
        assert!(w.is_empty(), "an undressed wearer shouldn't linger in the save");
    }

    #[test]
    fn set_worn_replaces_the_whole_outfit() {
        let mut w = kid_with_hat();
        w.set_worn(PLAYER, ["bow_tie".to_string(), "jet_boots".to_string()]);
        assert!(!w.is_wearing(PLAYER, "hat"));
        assert_eq!(w.worn_by(PLAYER).len(), 2);
        w.set_worn(PLAYER, Vec::new());
        assert!(w.is_empty());
    }

    #[test]
    fn round_trips_through_json_as_a_plain_map() {
        let mut w = kid_with_hat();
        w.put_on("dolphin", "sparkle_trail");
        let json = serde_json::to_string(&w).unwrap();
        assert!(json.starts_with('{'), "wardrobe should serialize as a bare map: {json}");
        let back: Wardrobe = serde_json::from_str(&json).unwrap();
        assert_eq!(back, w);
    }
}
