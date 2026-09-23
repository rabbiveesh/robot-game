//! The generic layout sweep: every migrated panel × every sweep screen × the
//! awkward data that used to collide, run through `assert_sane` (no overlap,
//! nothing outside its panel, nothing clipped, no text overflowing its fit
//! policy). Plus reachability: every list row lands on some page.
//!
//! Engine-agnostic: it only reads `Frame`s, so it guards a future taffy engine
//! exactly as it guards the in-house one.

use std::collections::BTreeSet;

use robot_buddy_domain::economy::shop::{pearl_catalog, quote_trade, shop_catalog, swag_items, Currency, ShopKind};
use robot_buddy_game::sprites::player::OUTFIT_COLORS;
use robot_buddy_game::ui::layout::{assert_sane, check_sane, screen_rect, SWEEP_SCREENS};
use robot_buddy_game::ui::{shop, swag};

// ─── Shop ────────────────────────────────────────────────

fn owned(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

const LONG_MESSAGE: &str = "Diving Net — Catches 1 extra pearl every time you find one!";

/// Lay out every catalog page; assert each sane and every row reachable.
fn sweep_shop_catalog(kind: ShopKind, message: Option<&str>) {
    let catalog = kind.catalog();
    let owned = owned(&["hat", "kelp_crown", "diving_net"]);
    for &screen in &SWEEP_SCREENS {
        let mut seen = BTreeSet::new();
        let mut page = 0;
        loop {
            let m = shop::ShopModel {
                shop: kind,
                catalog: &catalog,
                owned: &owned,
                balance: 7,
                view: shop::ShopView::Browsing,
                message,
                page,
            };
            let l = shop::layout(&m, screen);
            assert_sane(&l.frame, screen_rect(screen));
            for i in l.page.rows() {
                assert!(l.item(i).is_some(), "{kind:?} row {i} missing at {screen:?}");
                seen.insert(i);
            }
            if !l.page.has_next() {
                break;
            }
            page += 1;
        }
        assert_eq!(seen.len(), catalog.len(), "{kind:?}: every item reachable at {screen:?}");
    }
}

#[test]
fn bolt_catalog_is_sane_everywhere() {
    sweep_shop_catalog(ShopKind::Bolt, None);
    sweep_shop_catalog(ShopKind::Bolt, Some("You need 12 more Dum Dums!"));
}

#[test]
fn hermie_catalog_is_sane_everywhere() {
    // The reported bug: Hermie's six rows ran under the purchase message.
    sweep_shop_catalog(ShopKind::Hermie, None);
    sweep_shop_catalog(ShopKind::Hermie, Some(LONG_MESSAGE));
    sweep_shop_catalog(ShopKind::Hermie, Some("Not enough for a Dum Dum yet — you need 2 more pearls!"));
}

#[test]
fn hermies_full_shelf_fits_one_page_at_the_default_window() {
    let catalog = pearl_catalog();
    let none = BTreeSet::new();
    let m = shop::ShopModel {
        shop: ShopKind::Hermie,
        catalog: &catalog,
        owned: &none,
        balance: 40,
        view: shop::ShopView::Browsing,
        message: Some(LONG_MESSAGE),
        page: 0,
    };
    let l = shop::layout(&m, (960.0, 720.0));
    assert!(!l.page.paged(), "six rows + a message fit the default window without paging");
}

#[test]
fn buying_and_trading_are_sane_everywhere() {
    let catalog = pearl_catalog();
    let none = BTreeSet::new();
    let quote = quote_trade(30, Currency::Pearls, 3, Currency::DumDums);
    let choices = [10, 9, 11, 12];
    for &screen in &SWEEP_SCREENS {
        for message in [None, Some("Hmm, let me count again..."), Some(LONG_MESSAGE)] {
            let views = [
                shop::ShopView::Buying { item: &catalog[4], balance: 30, cost: 20, choices: &choices },
                shop::ShopView::Trading { quote: &quote, choices: &choices },
                shop::ShopView::PickingColor { colors: OUTFIT_COLORS, current: 2 },
            ];
            for view in views {
                let m = shop::ShopModel {
                    shop: ShopKind::Hermie,
                    catalog: &catalog,
                    owned: &none,
                    balance: 30,
                    view,
                    message,
                    page: 0,
                };
                let l = shop::layout(&m, screen);
                if let Err(e) = check_sane(&l.frame, screen_rect(screen)) {
                    let els: Vec<_> = l.frame.elements().iter().map(|e| (e.id, e.rect)).collect();
                    panic!("view at {screen:?} with {message:?}:\n  - {}\n{els:#?}", e.join("\n  - "));
                }
                for (i, v) in m.view.choices().iter().enumerate() {
                    assert!(l.answer(&m.view, *v).is_some(), "answer tile {i} missing at {screen:?}");
                }
            }
        }
    }
}

#[test]
fn bolts_shelf_works_with_bolts_catalog_too() {
    // Guard the other counter's data shape (no blurbs, Dum Dum prices).
    let catalog = shop_catalog();
    assert!(catalog.iter().all(|i| i.blurb.is_empty()));
}

// ─── Swag ────────────────────────────────────────────────

#[test]
fn swag_picker_is_sane_with_a_full_wardrobe() {
    let items = swag_items(); // every wearable both counters sell (9)
    assert!(items.len() >= 5);
    let taken = owned(&["hat", "glow_lantern"]);
    for &screen in &SWEEP_SCREENS {
        for message in [None, Some("Sparky puts on the Shell Necklace!")] {
            for recipient in ["Sparky", "Professor Gizmo"] {
                let mut seen = BTreeSet::new();
                let mut page = 0;
                loop {
                    let m = swag::SwagModel { recipient, items: &items, taken: &taken, message, page };
                    let l = swag::layout(&m, screen);
                    assert_sane(&l.frame, screen_rect(screen));
                    seen.extend(l.page.rows());
                    if !l.page.has_next() {
                        break;
                    }
                    page += 1;
                }
                assert_eq!(seen.len(), items.len(), "every piece reachable at {screen:?}");
            }
        }
    }
}

#[test]
fn empty_swag_picker_is_sane() {
    let taken = BTreeSet::new();
    for &screen in &SWEEP_SCREENS {
        let m = swag::SwagModel { recipient: "Sparky", items: &[], taken: &taken, message: None, page: 0 };
        let l = swag::layout(&m, screen);
        assert_sane(&l.frame, screen_rect(screen));
    }
}

// ─── Quest ───────────────────────────────────────────────

#[test]
fn quest_beats_are_sane_everywhere() {
    use robot_buddy_game::ui::quest::{self, QuestView};
    let lines: Vec<String> = [
        "The old lighthouse keeper lost count of his lanterns again.",
        "He had twelve on the shelf, but the storm knocked some into the sea, and now the harbour is dark.",
        "Can you help him figure out how many are left before the ships come in tonight?",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let options: Vec<String> = [
        "Row out to the rocks and look for the lanterns that fell",
        "Ask the seagulls",
        "Count what's left on the shelf first",
        "Go and find Professor Gizmo for a brand-new lantern design",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let choices = [7, 8, 9, 6];
    let prompt = "The keeper had 12 lanterns. The storm knocked 5 into the sea. How many are still on the shelf?";
    for &screen in &SWEEP_SCREENS {
        for message in [None, Some("Hmm, let's count those again together!")] {
            let views = [
                QuestView::Narrative { speaker: "Lighthouse Keeper", lines: &lines },
                QuestView::Travel { label: "Head to harbour at (12, 30)...".into() },
                QuestView::Puzzle { prompt, choices: &choices },
                QuestView::Choice { prompt: "What should we do first?", options: &options },
                QuestView::Choice { prompt: "Nothing to pick here.", options: &[] },
                QuestView::Reward { dum_dums: 5 },
            ];
            for view in views {
                let l = quest::layout(&view, "The Case of the Missing Lanterns", message, screen);
                assert_sane(&l.frame, screen_rect(screen));
                assert_eq!(l.continue_btn().is_some(), view.has_continue());
            }
        }
    }
}
