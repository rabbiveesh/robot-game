//! Shop economy. Pure logic — catalog + purchase math, no rendering.
//!
//! The shopkeeper (Bolt) sells cosmetic items for Sparky. Every purchase is a
//! subtraction moment ("you have 12, it costs 5, how many left?") and every
//! unaffordable item is a number-bond moment ("you need 15 but have 9 — how
//! many more?"). The economy IS math practice without feeling like a quiz.
//!
//! This module owns the catalog and the purchase arithmetic. Presenting the
//! subtraction as a CRA challenge (or simple dialogue) and logging spend/earn
//! events is the game layer's job — here we stay pure and headlessly testable.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopItem {
    pub id: String,
    pub name: String,
    pub cost: u32,
}

/// The cosmetic catalog, cheapest first. Costs match the dum-dum-economy spec.
pub fn shop_catalog() -> Vec<ShopItem> {
    [
        ("hat", "Hat", 3),
        ("bow_tie", "Bow Tie", 5),
        ("jet_boots", "Jet Boots", 8),
        ("color_change", "Color Change", 10),
        ("sparkle_trail", "Sparkle Trail", 15),
    ]
    .iter()
    .map(|(id, name, cost)| ShopItem { id: (*id).into(), name: (*name).into(), cost: *cost })
    .collect()
}

pub fn item_by_id(id: &str) -> Option<ShopItem> {
    shop_catalog().into_iter().find(|i| i.id == id)
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
/// `new_balance` / adds the item to `owned` on a `Bought` outcome.
pub fn process_purchase(balance: u32, item_id: &str, owned: &HashSet<String>) -> PurchaseOutcome {
    let item = match item_by_id(item_id) {
        Some(i) => i,
        None => return PurchaseOutcome::UnknownItem,
    };
    if owned.contains(&item.id) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(ids: &[&str]) -> HashSet<String> {
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
    fn can_afford_is_inclusive() {
        assert!(can_afford(5, 5));
        assert!(!can_afford(4, 5));
    }
}
