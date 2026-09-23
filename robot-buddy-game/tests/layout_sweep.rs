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

// ─── Challenge ───────────────────────────────────────────

mod challenge_sweep {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;
    use robot_buddy_domain::challenge::challenge_state::{
        challenge_reducer, ChallengeAction, ChallengeState, DisplaySpeech, RenderHint, VoiceState,
    };
    use robot_buddy_domain::learning::challenge_generator::{generate_challenge, Challenge, ChallengeProfile};
    use robot_buddy_domain::learning::operation_stats::OperationStats;
    use robot_buddy_domain::types::{CraStage, Phase};
    use robot_buddy_game::ui::challenge;

    const WORD_PROBLEM: &str =
        "A frog hops 7 times across the lily pads, then 3 more times to reach the far bank! How many hops is that?";

    fn presented(c: &Challenge, display: &str) -> ChallengeState {
        ChallengeState {
            phase: Phase::Presented,
            correct_answer: c.correct_answer,
            attempts: 0,
            max_attempts: 2,
            correct: None,
            question: DisplaySpeech { display: display.into(), speech: display.into() },
            feedback: None,
            reward: None,
            render_hint: RenderHint {
                cra_stage: CraStage::Abstract,
                answer_mode: "choice".into(),
                interaction_type: "quiz".into(),
            },
            hint_used: false,
            hint_level: 0,
            told_me: false,
            voice: VoiceState::reset(),
        }
    }

    fn wrong(c: &Challenge) -> i32 {
        c.choices.iter().find(|ch| !ch.correct).and_then(|ch| ch.text.parse().ok()).unwrap_or(c.correct_answer + 1)
    }

    /// Every phase a challenge can show.
    fn phases(c: &Challenge, display: &str) -> Vec<(&'static str, ChallengeState)> {
        let p = presented(c, display);
        let fed = challenge_reducer(p.clone(), ChallengeAction::AnswerSubmitted { answer: wrong(c) });
        let hinted = challenge_reducer(p.clone(), ChallengeAction::ShowMe);
        let hinted_fed = challenge_reducer(hinted.clone(), ChallengeAction::AnswerSubmitted { answer: wrong(c) });
        let solved = challenge_reducer(p.clone(), ChallengeAction::AnswerSubmitted { answer: c.correct_answer });
        let hinted_solved = challenge_reducer(hinted.clone(), ChallengeAction::AnswerSubmitted { answer: c.correct_answer });
        let told = challenge_reducer(p.clone(), ChallengeAction::TellMe);
        let taught = challenge_reducer(fed.clone(), ChallengeAction::AnswerSubmitted { answer: wrong(c) });
        let after = challenge_reducer(taught.clone(), ChallengeAction::TeachingComplete);
        vec![
            ("presented", p),
            ("feedback", fed),
            ("show me", hinted),
            ("show me + feedback", hinted_fed),
            ("solved", solved),
            ("show me + solved", hinted_solved),
            ("tell me", told),
            ("taught", taught),
            ("after teaching", after),
        ]
    }

    /// A spread of real generated challenges: every band, so every visual
    /// family (dots, bonds, base-ten blocks, groups) shows up.
    fn challenges() -> Vec<Challenge> {
        let mut rng = SmallRng::seed_from_u64(42);
        (1..=10u8)
            .flat_map(|band| {
                let profile = ChallengeProfile { math_band: band, spread_width: 0.0, operation_stats: OperationStats::new() };
                (0..6).map(|_| generate_challenge(&profile, &mut rng)).collect::<Vec<_>>()
            })
            .collect()
    }

    #[test]
    fn every_challenge_phase_is_sane_everywhere() {
        let all = challenges();
        for &screen in &SWEEP_SCREENS {
            for c in &all {
                for display in [c.display_text.as_str(), WORD_PROBLEM] {
                    for (name, cs) in phases(c, display) {
                        let l = challenge::layout(&cs, c, screen);
                        if let Err(e) = check_sane(&l.frame, screen_rect(screen)) {
                            let els: Vec<_> = l.frame.elements().iter().map(|e| (e.id, e.rect)).collect();
                            panic!(
                                "{name} ({} {} {} band {}, {display:?}) at {screen:?}:\n  - {}\n{els:?}",
                                c.numbers.a, c.numbers.op, c.numbers.b, c.sampled_band,
                                e.join("\n  - ")
                            );
                        }
                    }
                }
            }
        }
    }

    /// The drift bug: hit rects used to sit ~30px above the drawn buttons when
    /// the question wrapped. Now the tap target IS the drawn button, and a tap
    /// on its center answers.
    #[test]
    fn tapping_a_drawn_button_answers_it_even_under_a_wrapped_question() {
        let c = &challenges()[0];
        let cs = presented(c, WORD_PROBLEM);
        let l = challenge::layout(&cs, c, (960.0, 720.0));
        let q = l.frame.get(challenge::ChallengeId::Question).unwrap();
        let wrapped = match &q.kind {
            robot_buddy_game::ui::layout::Kind::Text(t) => t.lines.len(),
            _ => 0,
        };
        assert_eq!(wrapped, 2, "the word problem wraps at the default window");
        for (i, choice) in c.choices.iter().enumerate() {
            let r = l.choice(i).expect("button laid out");
            assert!(r.y >= q.rect.bottom(), "button {i} sits below the wrapped question");
            let (x, y) = r.center();
            match challenge::handle_click(x, y, &cs, c, &l) {
                Some(ChallengeAction::AnswerSubmitted { answer }) => {
                    assert_eq!(answer.to_string(), choice.text)
                }
                _ => panic!("tapping button {i} should answer it"),
            }
        }
    }
}

// ─── Dialogue ────────────────────────────────────────────

#[test]
fn dialogue_lines_are_sane_everywhere() {
    use robot_buddy_game::ui::dialogue::{self, DialogueId};
    use robot_buddy_game::ui::layout::Kind;
    let four_liner = "Oh no, oh no! The storm blew all of my lanterns off the shelf and into the harbour, \
        and the fishing boats are coming home tonight. If the lighthouse is dark they'll never find the \
        way in. Could you and Sparky count how many are still up here with me, and how many we need to fish out?";
    let lines = [
        ("Sparky", "Hi!"),
        ("Professor Gizmo", "Let's build something amazing together today!"),
        ("Bolt the Shopkeeper", four_liner),
        ("A Very Long Speaker Name That Keeps Going", four_liner),
    ];
    for &screen in &SWEEP_SCREENS {
        for (speaker, text) in lines {
            let f = dialogue::layout(speaker, text, screen);
            assert_sane(&f, screen_rect(screen));
            // The whole line is on screen: nothing cut, "SPACE >" still there.
            assert!(f.rect(DialogueId::Continue).is_some());
            let Some(Kind::Text(body)) = f.get(DialogueId::Body).map(|e| &e.kind) else { panic!("body text") };
            let shown: String = body.lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ");
            assert_eq!(shown, text.split_whitespace().collect::<Vec<_>>().join(" "), "at {screen:?}");
        }
    }
}

// ─── Settings ────────────────────────────────────────────

#[test]
fn settings_overlay_is_sane_everywhere() {
    use robot_buddy_domain::types::GamePace;
    use robot_buddy_game::game::FeatureFlags;
    use robot_buddy_game::ui::settings_overlay::{self, SettingsId, SettingsModel};
    for &screen in &SWEEP_SCREENS {
        for parent_open in [false, true] {
            let m = SettingsModel { features: FeatureFlags::default(), parent_open, pace: GamePace::ALL[1] };
            let f = settings_overlay::layout(screen, m);
            assert_sane(&f, screen_rect(screen));
            // The section labels are laid out too — not hand-placed.
            assert_eq!(f.rect(SettingsId::SpeedLabel).is_some(), !parent_open);
            assert_eq!(f.rect(SettingsId::Note).is_some(), parent_open);
            assert!(f.rect(SettingsId::Done).is_some(), "Done reachable at {screen:?}");
        }
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
