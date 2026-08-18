//! Shop economy. Pure logic — catalogs + purchase math, no rendering.
//!
//! Two counters, two currencies. Bolt on the surface sells cosmetics for Dum
//! Dums; Hermie the hermit crab, down in the trench, sells reef swag and
//! upgrades for pearls and runs a trade desk that turns pearls into Dum Dums.
//! Every purchase is a subtraction moment ("you have 12, it costs 5, how many
//! left?"), every unaffordable item is a number-bond moment ("you need 15 but
//! have 9 — how many more?"), and the trade desk is a division moment ("three
//! pearls make a Dum Dum — how many can you make?"). The economy IS math
//! practice without feeling like a quiz.
//!
//! This module owns the catalogs and the arithmetic. Presenting it as a CRA
//! challenge and logging spend/earn events is the game layer's job — here we
//! stay pure and headlessly testable.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// What a counter takes. Dum Dums come from puzzles anywhere; pearls come from
/// Shelly's leaps and clean dives, and only spend down in the reef.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Currency {
    DumDums,
    Pearls,
}

impl Currency {
    /// Plural name, for prices and balance lines.
    pub fn label(self) -> &'static str {
        match self {
            Currency::DumDums => "Dum Dums",
            Currency::Pearls => "pearls",
        }
    }

    /// Short form for a price tag.
    pub fn tag(self) -> &'static str {
        match self {
            Currency::DumDums => "DD",
            Currency::Pearls => "P",
        }
    }
}

/// What buying a thing actually does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ItemKind {
    /// Wearable. Goes into the wardrobe, and can be handed to a buddy — which
    /// takes it off the kid, so the counter will happily sell another.
    Swag,
    /// A permanent perk. Bought once, kept forever, never worn or given away.
    Upgrade,
    /// The trade desk: `rate` pearls become one Dum Dum, as many times over as
    /// the kid can afford. Never "owned"; it's a standing offer.
    Trade { rate: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopItem {
    pub id: String,
    pub name: String,
    pub cost: u32,
    pub currency: Currency,
    pub kind: ItemKind,
}

/// Which counter a catalog belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopKind {
    /// Bolt's shop in the village. Cosmetics for Dum Dums.
    Bolt,
    /// Hermie's stall in the trench. Reef swag, upgrades, and the trade desk.
    Hermie,
}

impl ShopKind {
    pub fn title(self) -> &'static str {
        match self {
            ShopKind::Bolt => "Bolt's Shop",
            ShopKind::Hermie => "Hermie's Deep Stall",
        }
    }

    pub fn currency(self) -> Currency {
        match self {
            ShopKind::Bolt => Currency::DumDums,
            ShopKind::Hermie => Currency::Pearls,
        }
    }

    pub fn catalog(self) -> Vec<ShopItem> {
        match self {
            ShopKind::Bolt => shop_catalog(),
            ShopKind::Hermie => pearl_catalog(),
        }
    }
}

/// Pearls that buy one Dum Dum at Hermie's trade desk. Pearls take a real
/// decision each to earn, so the rate stays kind — but not so kind that the
/// deep stall quietly buys out Bolt's whole shelf.
pub const TRADE_RATE: u32 = 3;

fn item(id: &str, name: &str, cost: u32, currency: Currency, kind: ItemKind) -> ShopItem {
    ShopItem { id: id.into(), name: name.into(), cost, currency, kind }
}

/// Bolt's cosmetic catalog, cheapest first. Costs match the dum-dum-economy spec.
pub fn shop_catalog() -> Vec<ShopItem> {
    [
        ("hat", "Hat", 3),
        ("bow_tie", "Bow Tie", 5),
        ("jet_boots", "Jet Boots", 8),
        ("color_change", "Color Change", 10),
        ("sparkle_trail", "Sparkle Trail", 15),
    ]
    .iter()
    .map(|(id, name, cost)| item(id, name, *cost, Currency::DumDums, ItemKind::Swag))
    .collect()
}

/// Hermie's stall, deep in the trench. Reef swag you can't get on the surface,
/// one upgrade that makes every future pearl worth more (the grind rewarding
/// the grind), and the standing trade offer.
pub fn pearl_catalog() -> Vec<ShopItem> {
    vec![
        item("kelp_crown", "Kelp Crown", 4, Currency::Pearls, ItemKind::Swag),
        item("shell_necklace", "Shell Necklace", 6, Currency::Pearls, ItemKind::Swag),
        item("starfish_badge", "Starfish Badge", 9, Currency::Pearls, ItemKind::Swag),
        item("glow_lantern", "Glow Lantern", 12, Currency::Pearls, ItemKind::Swag),
        item("diving_net", "Diving Net", 20, Currency::Pearls, ItemKind::Upgrade),
        item("trade_desk", "Trade for Dum Dums", TRADE_RATE, Currency::Pearls,
             ItemKind::Trade { rate: TRADE_RATE }),
    ]
}

/// Every item either counter sells, for lookups that don't know the counter.
pub fn all_items() -> Vec<ShopItem> {
    let mut all = shop_catalog();
    all.extend(pearl_catalog());
    all
}

/// Extra pearls the Diving Net adds to every find, once it's bought.
pub const DIVING_NET_BONUS: u32 = 1;

/// Item id of the yield upgrade, so the game can check for it without
/// hard-coding the string in three places.
pub const DIVING_NET: &str = "diving_net";

pub fn item_by_id(id: &str) -> Option<ShopItem> {
    all_items().into_iter().find(|i| i.id == id)
}

pub fn can_afford(balance: u32, cost: u32) -> bool {
    balance >= cost
}

/// How many more Dum Dums are needed to afford `cost` (the number-bond moment).
/// Zero when already affordable.
pub fn shortfall(balance: u32, cost: u32) -> u32 {
    cost.saturating_sub(balance)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseResult {
    pub item_id: String,
    pub spent: u32,
    pub new_balance: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PurchaseOutcome {
    /// The buy succeeded. `result.spent` and `result.new_balance` are the
    /// operands/answer of the embedded subtraction the kid solves.
    Bought { result: PurchaseResult },
    /// Cosmetics are one-per-customer; this item is already owned.
    AlreadyOwned,
    /// Not enough Dum Dums — `shortfall` is the "how many more?" number bond.
    CantAfford { shortfall: u32 },
    /// No such item id in the catalog.
    UnknownItem,
}

/// Resolve a purchase attempt without mutating anything — the caller applies
/// `new_balance` / adds the item to `owned` on a `Bought` outcome. `owned` is
/// whatever the kid already has of this counter's kind: the swag they're
/// wearing, or the upgrades they've bought. The trade desk is never "owned" —
/// it's a standing offer, so it's always buyable if they can afford one trade.
pub fn process_purchase(balance: u32, item_id: &str, owned: &BTreeSet<String>) -> PurchaseOutcome {
    let item = match item_by_id(item_id) {
        Some(i) => i,
        None => return PurchaseOutcome::UnknownItem,
    };
    if !matches!(item.kind, ItemKind::Trade { .. }) && owned.contains(&item.id) {
        return PurchaseOutcome::AlreadyOwned;
    }
    if !can_afford(balance, item.cost) {
        return PurchaseOutcome::CantAfford { shortfall: shortfall(balance, item.cost) };
    }
    PurchaseOutcome::Bought {
        result: PurchaseResult {
            item_id: item.id,
            spent: item.cost,
            new_balance: balance - item.cost,
        },
    }
}

/// What a trip to the trade desk is worth: how many pearls actually change
/// hands, how many Dum Dums come back, and what's left over. The leftovers are
/// the point — a remainder is a real division fact, not a rounding error, and
/// Hermie hands them straight back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeQuote {
    pub rate: u32,
    /// Pearls the kid brought to the counter.
    pub offered: u32,
    /// Pearls actually spent (`gain * rate`).
    pub spent: u32,
    /// Dum Dums handed back.
    pub gain: u32,
    /// Pearls that don't make a whole Dum Dum, kept by the kid.
    pub left_over: u32,
}

/// Quote a trade of every pearl the kid is carrying. Zero `gain` means they
/// can't make even one Dum Dum yet — that's a number bond, not a refusal.
pub fn quote_trade(pearls: u32, rate: u32) -> TradeQuote {
    let rate = rate.max(1);
    let gain = pearls / rate;
    let spent = gain * rate;
    TradeQuote { rate, offered: pearls, spent, gain, left_over: pearls - spent }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn catalog_is_sorted_cheapest_first_and_matches_spec() {
        let cat = shop_catalog();
        assert_eq!(cat.first().unwrap().cost, 3);
        assert_eq!(cat.last().unwrap().cost, 15);
        for w in cat.windows(2) {
            assert!(w[0].cost <= w[1].cost, "catalog must be cheapest-first");
        }
        assert_eq!(item_by_id("jet_boots").unwrap().cost, 8);
        assert!(item_by_id("nonexistent").is_none());
    }

    #[test]
    fn affordable_purchase_subtracts_the_cost() {
        // "You have 12, it costs 5, how many left?" → 7.
        let out = process_purchase(12, "bow_tie", &owned(&[]));
        match out {
            PurchaseOutcome::Bought { result } => {
                assert_eq!(result.spent, 5);
                assert_eq!(result.new_balance, 7);
                assert_eq!(result.item_id, "bow_tie");
            }
            other => panic!("expected Bought, got {other:?}"),
        }
    }

    #[test]
    fn exact_balance_can_buy_to_zero() {
        let out = process_purchase(3, "hat", &owned(&[]));
        assert_eq!(out, PurchaseOutcome::Bought {
            result: PurchaseResult { item_id: "hat".into(), spent: 3, new_balance: 0 },
        });
    }

    #[test]
    fn unaffordable_reports_shortfall_as_number_bond() {
        // "You need 15 but have 9 — how many more?" → 6.
        let out = process_purchase(9, "sparkle_trail", &owned(&[]));
        assert_eq!(out, PurchaseOutcome::CantAfford { shortfall: 6 });
        assert_eq!(shortfall(9, 15), 6);
        assert_eq!(shortfall(15, 15), 0);
    }

    #[test]
    fn cannot_rebuy_owned_cosmetic() {
        let out = process_purchase(20, "hat", &owned(&["hat"]));
        assert_eq!(out, PurchaseOutcome::AlreadyOwned);
    }

    #[test]
    fn unknown_item_is_reported() {
        assert_eq!(process_purchase(20, "rocket", &owned(&[])), PurchaseOutcome::UnknownItem);
    }

    #[test]
    fn the_deep_stall_takes_pearls_and_bolts_takes_dum_dums() {
        for i in shop_catalog() {
            assert_eq!(i.currency, Currency::DumDums, "{} is Bolt's", i.id);
        }
        for i in pearl_catalog() {
            assert_eq!(i.currency, Currency::Pearls, "{} is Hermie's", i.id);
        }
        assert_eq!(ShopKind::Hermie.currency(), Currency::Pearls);
        assert_eq!(ShopKind::Bolt.catalog().len(), shop_catalog().len());
    }

    #[test]
    fn item_ids_are_unique_across_both_counters() {
        let all = all_items();
        let mut ids: Vec<&str> = all.iter().map(|i| i.id.as_str()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "two items sharing an id would share a wardrobe slot");
    }

    #[test]
    fn the_trade_desk_is_never_owned_out() {
        // Everything else is one per customer; the trade desk is a standing
        // offer, so a kid can keep coming back with more pearls.
        let out = process_purchase(30, "trade_desk", &owned(&["trade_desk"]));
        assert!(matches!(out, PurchaseOutcome::Bought { .. }), "got {out:?}");
        assert_eq!(process_purchase(30, "diving_net", &owned(&["diving_net"])),
            PurchaseOutcome::AlreadyOwned);
    }

    #[test]
    fn a_trade_hands_back_the_remainder() {
        // "Three pearls make a Dum Dum. You have fourteen — how many?" Four,
        // with two pearls left in your hand.
        let q = quote_trade(14, TRADE_RATE);
        assert_eq!((q.gain, q.spent, q.left_over), (4, 12, 2));
        assert_eq!(q.offered, 14);
    }

    #[test]
    fn an_exact_pile_trades_clean() {
        let q = quote_trade(12, 3);
        assert_eq!((q.gain, q.spent, q.left_over), (4, 12, 0));
    }

    #[test]
    fn too_few_pearls_to_trade_is_a_number_bond_not_a_refusal() {
        let q = quote_trade(2, 3);
        assert_eq!(q.gain, 0, "not enough for one Dum Dum yet");
        assert_eq!(q.left_over, 2, "and nothing is taken");
        assert_eq!(shortfall(2, 3), 1, "...they need one more pearl");
    }

    #[test]
    fn trading_never_invents_or_loses_pearls() {
        for pearls in 0..40u32 {
            let q = quote_trade(pearls, TRADE_RATE);
            assert_eq!(q.spent + q.left_over, pearls, "pearls must balance at {pearls}");
            assert_eq!(q.spent, q.gain * TRADE_RATE);
            assert!(q.left_over < TRADE_RATE, "a whole trade was left on the table at {pearls}");
        }
    }

    #[test]
    fn can_afford_is_inclusive() {
        assert!(can_afford(5, 5));
        assert!(!can_afford(4, 5));
    }
}
