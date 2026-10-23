//! Every Bevy UI panel × five screens (a 360×640 phone up) × the awkward data
//! that used to collide, laid out by Bevy headlessly (`ui::probe`). Each must
//! come out sound (`Snapshot::problems`: no button off screen or on another,
//! no text running out of its box) with the buttons a kid needs on it.
//!
//! The story tests check the same on every screen they tap through; this
//! sweep covers the sizes and data those don't reach.

use std::collections::BTreeSet;
use std::sync::Arc;

use robot_buddy_domain::economy::shop::{pearl_catalog, quote_trade, shop_catalog, swag_items, Currency, ShopKind};
use robot_buddy_game::save::Gender;
use robot_buddy_game::sprites::player::OUTFIT_COLORS;
use robot_buddy_game::ui::probe::{Snapshot, UiProbe};
use robot_buddy_game::ui::views::UiViews;
use robot_buddy_game::ui::{shop, swag, ArtId, UiAction};

const SCREENS: [(f32, f32); 5] = [(360.0, 640.0), (480.0, 800.0), (640.0, 480.0), (960.0, 720.0), (1600.0, 900.0)];

const LONG_MESSAGE: &str = "Diving Net — Catches 1 extra pearl every time you find one!";

/// Lay `views` out on every screen; fail on any problem. `check` sees each
/// snapshot too.
fn sweep(label: &str, views: impl Fn((f32, f32)) -> UiViews, check: impl Fn(&Snapshot, (f32, f32))) {
    for screen in SCREENS {
        let snap = UiProbe::new(screen).show(views(screen));
        let problems = snap.problems();
        assert!(problems.is_empty(), "{label} at {screen:?}:\n  {}", problems.join("\n  "));
        check(&snap, screen);
    }
}

/// Like `sweep`, many views per screen through one probe.
fn sweep_all(label: &str, views: impl Fn((f32, f32)) -> Vec<(String, UiViews)>, check: impl Fn(&str, &Snapshot)) {
    for screen in SCREENS {
        let mut probe = UiProbe::new(screen);
        for (name, v) in views(screen) {
            let snap = probe.show(v);
            let problems = snap.problems();
            assert!(problems.is_empty(), "{label}: {name} at {screen:?}:\n  {}", problems.join("\n  "));
            check(&format!("{name} at {screen:?}"), &snap);
        }
    }
}

fn set(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

fn kid() -> shop::KidLook {
    shop::KidLook { gender: Gender::Girl, worn: set(&["hat"]), color: "teal".into() }
}

fn has(snap: &Snapshot, a: UiAction) -> bool {
    snap.button(a).is_some()
}

// ─── Shop ────────────────────────────────────────────────

fn shelf(kind: ShopKind) -> Vec<(String, UiViews)> {
    let catalog = kind.catalog();
    let owned = set(&["hat", "kelp_crown", "diving_net"]);
    [None, Some("You need 12 more Dum Dums!"), Some(LONG_MESSAGE)]
        .into_iter()
        .map(|message| {
            let m = shop::ShopModel { shop: kind, catalog: &catalog, owned: &owned, balance: 7, view: shop::ShopView::Browsing, message };
            (format!("{kind:?} shelf, {message:?}"), UiViews { shop: Some(shop::ShopPanelView::new(&m, kid())), ..Default::default() })
        })
        .collect()
}

/// Every row is in the list (it scrolls), and Done never scrolls away.
#[test]
fn both_shelves_are_sound_everywhere() {
    for kind in [ShopKind::Bolt, ShopKind::Hermie] {
        let n = kind.catalog().len();
        sweep_all("shelf", |_| shelf(kind), |name, snap| {
            for i in 0..n {
                assert!(has(snap, UiAction::Shop(shop::ShopId::Item(i))), "{name}: row {i} missing");
            }
            let done = snap.button(UiAction::Shop(shop::ShopId::Done)).expect("Done");
            assert!(done.scroll.is_none(), "{name}: Done is outside the list");
        });
    }
}

#[test]
fn buying_trading_and_picking_a_colour_are_sound_everywhere() {
    let catalog = pearl_catalog();
    let none = BTreeSet::new();
    let quote = quote_trade(30, Currency::Pearls, 3, Currency::DumDums);
    let choices = [10, 9, 11, 12];
    sweep_all(
        "counter",
        |_| {
            let mut out = Vec::new();
            for message in [None, Some("Hmm, let me count again..."), Some(LONG_MESSAGE)] {
                let views = [
                    ("buying", shop::ShopView::Buying { item: &catalog[4], balance: 30, cost: 20, choices: &choices }),
                    ("trading", shop::ShopView::Trading { quote: &quote, choices: &choices }),
                    ("colour", shop::ShopView::PickingColor { colors: OUTFIT_COLORS, current: 2 }),
                ];
                for (name, view) in views {
                    let m = shop::ShopModel { shop: ShopKind::Hermie, catalog: &catalog, owned: &none, balance: 30, view, message };
                    out.push((format!("{name}, {message:?}"), UiViews { shop: Some(shop::ShopPanelView::new(&m, kid())), ..Default::default() }));
                }
            }
            out
        },
        |name, snap| {
            if name.starts_with("colour") {
                for i in 0..OUTFIT_COLORS.len() {
                    assert!(has(snap, UiAction::Shop(shop::ShopId::Swatch(i))), "{name}: swatch {i}");
                }
                assert!(snap.art(ArtId::ShopPreview).is_some(), "{name}: the kid in the mirror");
            } else {
                for i in 0..4 {
                    assert!(has(snap, UiAction::Shop(shop::ShopId::Answer(i))), "{name}: answer tile {i}");
                }
            }
        },
    );
}

#[test]
fn bolts_shelf_has_no_blurbs() {
    // Guard the other counter's data shape (no blurbs, Dum Dum prices).
    assert!(shop_catalog().iter().all(|i| i.blurb.is_empty()));
}

// ─── Swag ────────────────────────────────────────────────

fn buddy() -> swag::BuddyLook {
    swag::BuddyLook { sprite: None, worn: BTreeSet::new(), color: "teal".into() }
}

#[test]
fn the_swag_picker_is_sound_with_a_full_wardrobe() {
    let items = swag_items();
    let taken = set(&["hat", "glow_lantern"]);
    sweep_all(
        "swag",
        |_| {
            let mut out = Vec::new();
            for message in [None, Some("Sparky puts on the Shell Necklace!")] {
                for recipient in ["Sparky", "Professor Gizmo"] {
                    let m = swag::SwagModel { recipient, items: &items, taken: &taken, message, picking: None };
                    out.push((format!("{recipient}, {message:?}"), UiViews { swag: Some(swag::SwagPanelView::new(&m, buddy())), ..Default::default() }));
                }
            }
            let empty = swag::SwagModel { recipient: "Sparky", items: &[], taken: &taken, message: None, picking: None };
            out.push(("nothing to give".into(), UiViews { swag: Some(swag::SwagPanelView::new(&empty, buddy())), ..Default::default() }));
            out
        },
        |name, snap| {
            let n = if name.starts_with("nothing") { 0 } else { items.len() };
            for i in 0..n {
                assert!(has(snap, UiAction::Swag(swag::SwagId::Item(i))), "{name}: piece {i}");
            }
            assert!(has(snap, UiAction::Swag(swag::SwagId::Done)), "{name}: Done");
        },
    );
}

#[test]
fn the_buddy_colour_picker_is_sound_everywhere() {
    let items = swag_items();
    let taken = set(&["color_change"]);
    sweep_all(
        "buddy colour",
        |_| {
            let mut out = Vec::new();
            for message in [None, Some("Professor Gizmo looks great!")] {
                for recipient in ["Tali", "Professor Gizmo"] {
                    let picking = Some(swag::ColorPick { colors: OUTFIT_COLORS, current: 3 });
                    let m = swag::SwagModel { recipient, items: &items, taken: &taken, message, picking };
                    out.push((format!("{recipient}, {message:?}"), UiViews { swag: Some(swag::SwagPanelView::new(&m, buddy())), ..Default::default() }));
                }
            }
            out
        },
        |name, snap| {
            for i in 0..OUTFIT_COLORS.len() {
                assert!(has(snap, UiAction::Swag(swag::SwagId::Swatch(i))), "{name}: swatch {i}");
            }
            assert!(has(snap, UiAction::Swag(swag::SwagId::Done)), "{name}: Done");
            assert!(snap.art(ArtId::SwagPreview).is_some(), "{name}: the buddy preview");
        },
    );
}

// ─── Challenge ───────────────────────────────────────────

mod challenges {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;
    use robot_buddy_domain::challenge::challenge_state::{
        challenge_reducer, ChallengeAction, ChallengeState, DisplaySpeech, RenderHint, VoiceState,
    };
    use robot_buddy_domain::learning::challenge_generator::{generate_challenge, Challenge, ChallengeProfile};
    use robot_buddy_domain::learning::operation_stats::OperationStats;
    use robot_buddy_domain::types::{CraStage, Phase};
    use robot_buddy_game::ui::challenge::{ChallengeId, ChallengeView};

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
    fn all() -> Vec<Arc<Challenge>> {
        let mut rng = SmallRng::seed_from_u64(42);
        (1..=10u8)
            .flat_map(|band| {
                let profile = ChallengeProfile { math_band: band, spread_width: 0.0, operation_stats: OperationStats::new() };
                (0..3).map(|_| Arc::new(generate_challenge(&profile, &mut rng))).collect::<Vec<_>>()
            })
            .collect()
    }

    fn views(cs: &ChallengeState, c: &Challenge, screen: (f32, f32)) -> UiViews {
        UiViews { challenge: Some(ChallengeView::new(cs, c, screen)), ..Default::default() }
    }

    // One test per screen, so the sweep spreads across cores (it is most of
    // this file's time).
    #[test]
    fn every_challenge_phase_is_sound_on_a_phone() {
        every_challenge_phase_is_sound_at(SCREENS[0]);
    }
    #[test]
    fn every_challenge_phase_is_sound_on_a_big_phone() {
        every_challenge_phase_is_sound_at(SCREENS[1]);
    }
    #[test]
    fn every_challenge_phase_is_sound_on_a_small_window() {
        every_challenge_phase_is_sound_at(SCREENS[2]);
    }
    #[test]
    fn every_challenge_phase_is_sound_on_a_tablet() {
        every_challenge_phase_is_sound_at(SCREENS[3]);
    }
    #[test]
    fn every_challenge_phase_is_sound_on_a_desktop() {
        every_challenge_phase_is_sound_at(SCREENS[4]);
    }

    fn every_challenge_phase_is_sound_at(screen: (f32, f32)) {
        let all = all();
        let mut probe = UiProbe::new(screen);
        for c in &all {
            for display in [c.display_text.as_str(), WORD_PROBLEM] {
                for (name, cs) in phases(c, display) {
                    let snap = probe.show(views(&cs, c, screen));
                    let what = format!("{name} ({} {} {} band {}, {display:?}) at {screen:?}",
                        c.numbers.a, c.numbers.op, c.numbers.b, c.sampled_band);
                    let problems = snap.problems();
                    assert!(problems.is_empty(), "{what}:\n  {}", problems.join("\n  "));
                    // The CRA visual painted into its slot (the same plan)
                    // must fit the slot.
                    if let Some(r) = snap.art(ArtId::ChallengeVisual) {
                        let v = robot_buddy_game::ui::visuals::plan(c, r.w);
                        assert!(v.bbox.w <= r.w + 0.5 && v.bbox.h <= r.h + 0.5,
                            "{what}: visual is {:?}, its slot is {r:?}", v.bbox);
                    }
                }
            }
        }
    }

    /// A miss shows "Hmm, not quite!" in a slot that was already there, so the
    /// answer buttons don't jump out from under the kid's finger.
    #[test]
    fn a_wrong_answer_does_not_move_the_answer_buttons() {
        let all = all();
        for screen in SCREENS {
            let mut probe = UiProbe::new(screen);
            for c in all.iter().take(12) {
                for display in [c.display_text.as_str(), WORD_PROBLEM] {
                    let p = presented(c, display);
                    let fed = challenge_reducer(p.clone(), ChallengeAction::AnswerSubmitted { answer: wrong(c) });
                    assert_eq!(fed.phase, Phase::Feedback);
                    let before = probe.show(views(&p, c, screen));
                    let after = probe.show(views(&fed, c, screen));
                    for i in 0..c.choices.len() {
                        let at = |s: &Snapshot| s.button(UiAction::Challenge(ChallengeId::Choice(i))).map(|b| b.rect);
                        assert_eq!(at(&before), at(&after), "button {i} moved at {screen:?} for {display:?}");
                    }
                }
            }
        }
    }
}

// ─── Dialogue, settings, quest ───────────────────────────

#[test]
fn dialogue_lines_are_sound_everywhere() {
    use robot_buddy_game::ui::dialogue::DialogueView;
    let four_liner = "Oh no, oh no! The storm blew all of my lanterns off the shelf and into the harbour, \
        and the fishing boats are coming home tonight. If the lighthouse is dark they'll never find the \
        way in. Could you and Sparky count how many are still up here with me, and how many we need to fish out?";
    let lines = [
        ("Sparky", "Hi!"),
        ("Professor Gizmo", "Let's build something amazing together today!"),
        ("Bolt the Shopkeeper", four_liner),
        ("A Very Long Speaker Name That Keeps Going", four_liner),
    ];
    sweep_all(
        "dialogue",
        |_| {
            lines
                .iter()
                .map(|(speaker, text)| {
                    let d = DialogueView { speaker: speaker.to_string(), text: text.to_string(), waiting: true };
                    (speaker.to_string(), UiViews { dialogue: Some(d), dialogue_reveal: text.chars().count(), ..Default::default() })
                })
                .collect()
        },
        |name, snap| {
            assert!(snap.texts.iter().any(|t| t.text.contains("SPACE")), "{name}: \"SPACE >\" is shown");
        },
    );
}

#[test]
fn the_settings_overlay_is_sound_everywhere() {
    use robot_buddy_domain::types::GamePace;
    use robot_buddy_game::game::FeatureFlags;
    use robot_buddy_game::ui::settings_overlay::{SettingsId, SettingsModel, SettingsView};
    for parent_open in [false, true] {
        sweep(
            "settings",
            |screen| {
                let m = SettingsModel { features: FeatureFlags::default(), parent_open, pace: GamePace::ALL[1] };
                UiViews { settings: Some(SettingsView::new(m, screen)), ..Default::default() }
            },
            |snap, screen| {
                assert!(has(snap, UiAction::Settings(SettingsId::Done)), "Done at {screen:?}");
                assert!(has(snap, UiAction::Settings(SettingsId::BackToTitle)), "Back to title at {screen:?}");
                if parent_open {
                    for pace in GamePace::ALL {
                        assert!(has(snap, UiAction::Settings(SettingsId::Pace(pace))), "{pace:?} at {screen:?}");
                    }
                }
            },
        );
    }
}

#[test]
fn quest_beats_are_sound_everywhere() {
    use robot_buddy_game::ui::quest::{QuestId, QuestPanelView, QuestView};
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
    let prompt = "The keeper had 12 lanterns. The storm knocked 5 into the sea. How many are still on the shelf?".to_string();
    sweep_all(
        "quest",
        |screen| {
            let mut out = Vec::new();
            for message in [None, Some("Hmm, let's count those again together!".to_string())] {
                let beats = [
                    QuestView::Narrative { speaker: "Lighthouse Keeper".into(), lines: lines.clone() },
                    QuestView::Travel { label: "Head to harbour at (12, 30)...".into() },
                    QuestView::Puzzle { prompt: prompt.clone(), choices: vec![7, 8, 9, 6] },
                    QuestView::Choice { prompt: "What should we do first?".into(), options: options.clone() },
                    QuestView::Choice { prompt: "Nothing to pick here.".into(), options: vec![] },
                    QuestView::Reward { dum_dums: 5 },
                ];
                for view in beats {
                    let name = format!("{view:?}");
                    let v = QuestPanelView {
                        view,
                        title: "The Case of the Missing Lanterns".into(),
                        message: message.clone(),
                        screen_h: screen.1,
                    };
                    out.push((name, UiViews { quest: Some(v), ..Default::default() }));
                }
            }
            out
        },
        |name, snap| {
            let options_up = name.starts_with("Puzzle") || (name.starts_with("Choice") && !name.contains("options: []"));
            assert_eq!(has(snap, UiAction::Quest(QuestId::Continue)), !options_up, "{name}");
        },
    );
}

// ─── Leap, descent, shooter ──────────────────────────────

#[test]
fn shellys_strip_is_sound_everywhere() {
    use robot_buddy_domain::logic::leap::{leap_reducer, Clue, LeapAction, LeapPuzzle, LeapSession};
    use robot_buddy_game::ui::leap::{LeapId, LeapView};
    let fresh = LeapSession::new(LeapPuzzle {
        max: 24,
        pearl: 20,
        size: 5,
        count: 4,
        choices: vec![3, 4, 5, 6],
        clue: Clue::Count { n: 4 },
    });
    let committed = leap_reducer(fresh.clone(), LeapAction::Choose { size: 5 });
    let call = "My pearl's under stone 20! You get there in 4 leaps — how big is each one?".to_string();
    sweep_all(
        "leap",
        |_| {
            [("choosing", &fresh), ("committed", &committed)]
                .into_iter()
                .map(|(n, s)| (n.to_string(), UiViews { leap: Some(LeapView::new(s, call.clone())), ..Default::default() }))
                .collect()
        },
        |name, snap| {
            if name.starts_with("choosing") {
                for size in [3, 4, 5, 6] {
                    assert!(has(snap, UiAction::Leap(LeapId::Size(size))), "{name}: size {size}");
                }
            } else {
                assert!(has(snap, UiAction::Leap(LeapId::Leap)) && has(snap, UiAction::Leap(LeapId::SwimBack)), "{name}");
            }
        },
    );
}

#[test]
fn the_dive_shaft_is_sound_everywhere() {
    use robot_buddy_domain::logic::descent::{DivePuzzle, DiveSession};
    use robot_buddy_game::ui::descent::{DescentId, DescentView};
    let deep = DiveSession::new(DivePuzzle { door: 17, floor: 24, kicks: vec![1, 2, 3, 5, 10], shelves: vec![4, 9] });
    sweep(
        "descent",
        |_| UiViews { descent: Some(DescentView::new(&deep, Some(LONG_MESSAGE))), ..Default::default() },
        |snap, screen| {
            for n in [1, 2, 3, 5, 10] {
                assert!(has(snap, UiAction::Descent(DescentId::Sink(n))), "sink {n} at {screen:?}");
                assert!(has(snap, UiAction::Descent(DescentId::Rise(n))), "rise {n} at {screen:?}");
            }
            assert!(has(snap, UiAction::Descent(DescentId::Leave)));
            let shaft = snap.art(ArtId::DescentShaft).expect("the shaft");
            assert!(shaft.h >= 100.0, "the shaft keeps room to read at {screen:?}: {shaft:?}");
        },
    );
}

#[test]
fn the_shooter_footer_is_sound_everywhere() {
    use robot_buddy_game::ui::shooter::{FooterId, FooterView};
    sweep("shooter footer", |_| UiViews { shooter_footer: Some(FooterView), ..Default::default() }, |snap, _| {
        assert!(has(snap, UiAction::Shooter(FooterId::Leave)));
    });
}

// ─── Scrolling lists ─────────────────────────────────────

/// A list scrolled down stays scrolled when its panel is rebuilt (tapping a
/// row rebuilds it): the kid isn't thrown back to the top.
#[test]
fn a_rebuilt_list_keeps_its_place() {
    let catalog = pearl_catalog();
    let none = BTreeSet::new();
    let mut probe = UiProbe::new((360.0, 640.0));
    let mk = |message| {
        let m = shop::ShopModel { shop: ShopKind::Hermie, catalog: &catalog, owned: &none, balance: 1, view: shop::ShopView::Browsing, message };
        UiViews { shop: Some(shop::ShopPanelView::new(&m, kid())), ..Default::default() }
    };
    let last = UiAction::Shop(shop::ShopId::Item(catalog.len() - 1));
    let top = probe.show(mk(None));
    assert!(top.button(last).unwrap().visible().h < top.button(last).unwrap().rect.h,
        "the last row starts out of sight on a phone");
    let scrolled = probe.show_for(mk(None), last);
    let rebuilt = probe.show(mk(Some("You need 9 more pearls!")));
    assert_eq!(rebuilt.button(last).unwrap().visible().h, rebuilt.button(last).unwrap().rect.h,
        "the row the kid scrolled to is still in sight after the panel is rebuilt");
    assert!(scrolled.button(last).unwrap().visible().h > 0.0);
}
