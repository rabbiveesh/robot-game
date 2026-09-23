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
//!
//! Color Change is the one piece with a setting: its colour. Every wearer has
//! their own ([`Wardrobe::color_of`]), the kid included — so recolouring the
//! kid never repaints the shirt Tali was given. The colour travels with the
//! shirt when it's handed over, and the new wearer can pick another.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::shop::COLOR_CHANGE;

/// Wearer id for the kid. Every other wearer uses their NPC id string.
pub const PLAYER: &str = "player";

/// Everything that can change who wears what. The wardrobe only changes
/// through [`wardrobe_reducer`] (Invariant 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WardrobeAction {
    /// `who` puts on a new `item` (bought it, or an old save migrated).
    PutOn { who: String, item: String },
    /// `from` hands `item` to `to`.
    HandOver { from: String, to: String, item: String },
    /// `who` picks `color` for their Color Change outfit. `color` is an
    /// outfit colour id (the game's palette); the wardrobe just remembers it.
    SetColor { who: String, color: String },
}

impl WardrobeAction {
    pub fn put_on(who: &str, item: &str) -> Self {
        WardrobeAction::PutOn { who: who.into(), item: item.into() }
    }
    pub fn hand_over(from: &str, to: &str, item: &str) -> Self {
        WardrobeAction::HandOver { from: from.into(), to: to.into(), item: item.into() }
    }
    pub fn set_color(who: &str, color: &str) -> Self {
        WardrobeAction::SetColor { who: who.into(), color: color.into() }
    }
}

/// The one way the wardrobe changes: state in, (state, what happened) out.
pub fn wardrobe_reducer(mut w: Wardrobe, action: WardrobeAction) -> (Wardrobe, HandOver) {
    let outcome = match action {
        WardrobeAction::PutOn { who, item } => {
            if w.put_on(&who, &item) { HandOver::Given } else { HandOver::AlreadyWearing }
        }
        WardrobeAction::HandOver { from, to, item } => w.hand_over(&from, &to, &item),
        WardrobeAction::SetColor { who, color } => {
            // A colour is a preference, kept whether or not the shirt is on
            // right now: give it away, buy another, and it comes back in the
            // colour you liked.
            w.colors.insert(who, color);
            HandOver::Given
        }
    };
    (w, outcome)
}

/// What happened when the wardrobe was asked to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandOver {
    /// It happened: the item was put on, or moved from `from` to `to`.
    Given,
    /// `from` isn't wearing that item, so there's nothing to hand over.
    NotWorn,
    /// The wearer already has one of those. Nobody wears two hats.
    AlreadyWearing,
}

/// Who wears what, for everyone in the world at once.
///
/// On disk: `{"worn": {wearer: [items]}, "colors": {wearer: colour}}`. Saves
/// from before colours were per-wearer stored the bare `worn` map; those still
/// load, with no colours (the game migrates the kid's old single colour).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "WardrobeOnDisk")]
pub struct Wardrobe {
    /// wearer id → the item ids they're wearing. Sorted maps so a save file
    /// round-trips byte-identically and tests don't chase hash order.
    worn: BTreeMap<String, BTreeSet<String>>,
    /// wearer id → the colour id of their Color Change outfit.
    colors: BTreeMap<String, String>,
}

/// Either shape a save may hold. `Current` needs its `worn` key, so a legacy
/// bare map (whose keys are wearer ids, never "worn") falls through to `Legacy`.
#[derive(Deserialize)]
#[serde(untagged)]
enum WardrobeOnDisk {
    Current {
        worn: BTreeMap<String, BTreeSet<String>>,
        #[serde(default)]
        colors: BTreeMap<String, String>,
    },
    Legacy(BTreeMap<String, BTreeSet<String>>),
}

impl From<WardrobeOnDisk> for Wardrobe {
    fn from(d: WardrobeOnDisk) -> Self {
        match d {
            WardrobeOnDisk::Current { worn, colors } => Wardrobe { worn, colors },
            WardrobeOnDisk::Legacy(worn) => Wardrobe { worn, colors: BTreeMap::new() },
        }
    }
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

    /// The Color Change colour `who` picked, if they ever picked one.
    pub fn color_of(&self, who: &str) -> Option<&str> {
        self.colors.get(who).map(String::as_str)
    }

    /// Everyone wearing `item`, in id order.
    pub fn wearers_of<'a>(&'a self, item: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.worn.iter().filter(move |(_, s)| s.contains(item)).map(|(who, _)| who.as_str())
    }

    /// Put `item` on `who`. Returns false if they already had one.
    fn put_on(&mut self, who: &str, item: &str) -> bool {
        self.worn.entry(who.to_string()).or_default().insert(item.to_string())
    }

    /// Take `item` off `who`. Returns false if they weren't wearing it.
    fn take_off(&mut self, who: &str, item: &str) -> bool {
        let Some(set) = self.worn.get_mut(who) else { return false };
        let had = set.remove(item);
        if set.is_empty() {
            self.worn.remove(who);
        }
        had
    }

    /// Move one piece of swag from one wearer to another. The giver takes it
    /// off in the same breath the receiver puts it on — swag is never in two
    /// places, which is exactly why the shop can sell you another one.
    fn hand_over(&mut self, from: &str, to: &str, item: &str) -> HandOver {
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
        // The shirt arrives in the colour it was. The new wearer can pick
        // another, but nothing changes colour just by changing hands.
        if item == COLOR_CHANGE {
            if let Some(c) = self.colors.get(from).cloned() {
                self.colors.insert(to.to_string(), c);
            }
        }
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
    fn the_reducer_is_the_way_in() {
        let (w, out) = wardrobe_reducer(Wardrobe::new(), WardrobeAction::put_on(PLAYER, "hat"));
        assert_eq!(out, HandOver::Given);
        let (w, out) = wardrobe_reducer(w, WardrobeAction::put_on(PLAYER, "hat"));
        assert_eq!(out, HandOver::AlreadyWearing, "nobody wears two hats");
        let (w, out) = wardrobe_reducer(w, WardrobeAction::hand_over(PLAYER, "dolphin", "hat"));
        assert_eq!(out, HandOver::Given);
        assert!(w.is_wearing("dolphin", "hat") && !w.is_wearing(PLAYER, "hat"));
        let (_, out) = wardrobe_reducer(w, WardrobeAction::hand_over(PLAYER, "kid_1", "hat"));
        assert_eq!(out, HandOver::NotWorn, "the kid gave it away already");
    }

    #[test]
    fn round_trips_through_json_with_everyones_colours() {
        let mut w = kid_with_hat();
        w.put_on("dolphin", "sparkle_trail");
        let (w, _) = wardrobe_reducer(w, WardrobeAction::set_color("dolphin", "teal"));
        let (w, _) = wardrobe_reducer(w, WardrobeAction::set_color(PLAYER, "gold"));
        let json = serde_json::to_string(&w).unwrap();
        let back: Wardrobe = serde_json::from_str(&json).unwrap();
        assert_eq!(back, w);
        assert_eq!(back.color_of("dolphin"), Some("teal"));
        assert_eq!(back.color_of(PLAYER), Some("gold"));
    }

    #[test]
    fn a_wardrobe_saved_as_a_bare_map_still_loads() {
        // Before colours were per-wearer the wardrobe was just the worn map.
        let old = r#"{"player":["hat"],"kid_1":["color_change"]}"#;
        let w: Wardrobe = serde_json::from_str(old).unwrap();
        assert!(w.is_wearing(PLAYER, "hat"));
        assert!(w.is_wearing("kid_1", "color_change"));
        assert_eq!(w.color_of("kid_1"), None, "no colours yet: the game fills them in");
        let empty: Wardrobe = serde_json::from_str("{}").unwrap();
        assert!(empty.is_empty());
    }

    // ── Colour per wearer ──

    #[test]
    fn each_wearer_has_their_own_colour() {
        let (w, _) = wardrobe_reducer(Wardrobe::new(), WardrobeAction::put_on(PLAYER, COLOR_CHANGE));
        let (w, _) = wardrobe_reducer(w, WardrobeAction::set_color(PLAYER, "red"));
        let (w, _) = wardrobe_reducer(w, WardrobeAction::hand_over(PLAYER, "kid_1", COLOR_CHANGE));
        let (w, _) = wardrobe_reducer(w, WardrobeAction::set_color("kid_1", "teal"));
        // The kid buys another and goes gold: Tali stays teal.
        let (w, _) = wardrobe_reducer(w, WardrobeAction::put_on(PLAYER, COLOR_CHANGE));
        let (w, _) = wardrobe_reducer(w, WardrobeAction::set_color(PLAYER, "gold"));
        assert_eq!(w.color_of("kid_1"), Some("teal"));
        assert_eq!(w.color_of(PLAYER), Some("gold"));
    }

    #[test]
    fn a_handed_over_shirt_keeps_its_colour() {
        let (w, _) = wardrobe_reducer(Wardrobe::new(), WardrobeAction::put_on(PLAYER, COLOR_CHANGE));
        let (w, _) = wardrobe_reducer(w, WardrobeAction::set_color(PLAYER, "pink"));
        let (w, _) = wardrobe_reducer(w, WardrobeAction::hand_over(PLAYER, "dolphin", COLOR_CHANGE));
        assert_eq!(w.color_of("dolphin"), Some("pink"), "Echo gets it in the colour it was");
        assert_eq!(w.color_of(PLAYER), Some("pink"), "the kid still likes pink for next time");
        assert_eq!(w.wearers_of(COLOR_CHANGE).collect::<Vec<_>>(), vec!["dolphin"]);
    }

    #[test]
    fn handing_over_other_swag_leaves_colours_alone() {
        let mut w = kid_with_hat();
        w.put_on("dolphin", COLOR_CHANGE);
        let (w, _) = wardrobe_reducer(w, WardrobeAction::set_color("dolphin", "green"));
        let (w, _) = wardrobe_reducer(w, WardrobeAction::set_color(PLAYER, "red"));
        let (w, _) = wardrobe_reducer(w, WardrobeAction::hand_over(PLAYER, "dolphin", "hat"));
        assert_eq!(w.color_of("dolphin"), Some("green"));
    }
}
