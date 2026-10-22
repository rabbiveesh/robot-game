//! Native visual check: drive the real game into each UI state with the test
//! harness, record its draw calls with the real render code, then paint each
//! recording in a Bevy window and save a PNG.
//!
//!   SHOT_W=640 SHOT_H=480 SHOT_DIR=/tmp/shots cargo run -p robot-buddy-game --example screenshots
//!
//! Game logic steps at the harness's 960×720 (so its click helpers line up);
//! each shot is rendered at SHOT_W×SHOT_H logical pixels, and the PNG is that
//! many pixels (the window's scale factor is pinned to 1). In the cloud, run
//! it under `xvfb-run -a -s "-screen 0 1920x1080x24"`.

#[path = "../tests/common/mod.rs"]
#[allow(dead_code)]
mod common;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::WindowResolution;
use common::Harness;
use robot_buddy_game::app::{DrawList, DrawPlugin};
use robot_buddy_game::draw::{self, Cmd};
use robot_buddy_game::game::GameState;
use robot_buddy_game::input::{FrameInput, KeyCode};
use robot_buddy_game::npc::{self as npc_mod, NpcKind};
use robot_buddy_game::tilemap::Map;

fn env_f(key: &str, default: f32) -> f32 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn shot_screen() -> (f32, f32) {
    (env_f("SHOT_W", 960.0), env_f("SHOT_H", 720.0))
}

/// Recorded shots, painted later by the Bevy app.
#[derive(Resource, Default)]
struct Shots(Vec<(String, Vec<Cmd>)>);

impl Shots {
    /// Record what `draw` paints at the shot size. The clock is pinned so
    /// blinking hints show.
    fn snap(&mut self, name: &str, draw: impl FnOnce((f32, f32))) {
        let screen = shot_screen();
        draw::begin_frame(screen, 0.1);
        draw(screen);
        self.0.push((name.to_string(), draw::take_commands()));
    }

    fn snap_game(&mut self, name: &str, h: &mut Harness) {
        let input = FrameInput::empty();
        self.snap(name, |screen| h.game.render(screen, &input));
    }
}

fn to_trench(h: &mut Harness) {
    h.game.map = Map::trench();
    h.game.npcs = npc_mod::npcs_for_map("trench");
    h.game.npcs_offstage.clear();
    h.warp_to(22, 9);
}

fn to_home(h: &mut Harness) {
    h.game.map = Map::home();
    h.game.npcs = npc_mod::npcs_for_map("home");
    h.game.npcs_offstage.clear();
    h.warp_to(5, 3);
}

fn open_hermie(h: &mut Harness) {
    h.walk_to_npc(NpcKind::HermitCrab);
    h.interact();
    h.select_option("shop");
    h.wait_until(|g| g.state == GameState::Shop);
}

/// Hop the shooter's ship to alien `id` with the arrows, fire, and let the
/// bolt land.
fn shooter_hop_and_fire(h: &mut Harness, id: u32) {
    for _ in 0..12 {
        let s = &h.game.active_shooter().unwrap().session;
        let Some(ax) = s.aliens.iter().find(|a| a.id == id).map(|a| a.x) else { return };
        if (s.ship_x - ax).abs() <= 0.5 { break; }
        let key = if s.ship_x < ax { KeyCode::Right } else { KeyCode::Left };
        h.press(key);
    }
    h.press(KeyCode::Space);
    h.run_until(|g| g.active_shooter().is_none_or(|a| a.session.shots.is_empty()), 300);
}

/// Pair off the shooter's current wave, correctly.
fn shooter_clear_wave(h: &mut Harness) {
    let wave = h.game.active_shooter().unwrap().session.wave;
    while h.game.active_shooter().is_some_and(|a| a.session.wave == wave) {
        let (a, b) = {
            let s = &h.game.active_shooter().unwrap().session;
            let v = &s.aliens;
            (0..v.len())
                .flat_map(|i| ((i + 1)..v.len()).map(move |j| (i, j)))
                .find(|&(i, j)| v[i].value + v[j].value == s.target)
                .map(|(i, j)| (v[i].id, v[j].id))
                .unwrap()
        };
        shooter_hop_and_fire(h, a);
        shooter_hop_and_fire(h, b);
    }
}

fn open_bolt(h: &mut Harness) {
    h.walk_to_npc(NpcKind::Shopkeeper);
    h.interact();
    h.select_option("shop");
    h.wait_until(|g| g.state == GameState::Shop);
}

fn record() -> Shots {
    let mut shots = Shots::default();
    // ── Hermie: browse, the original overlap (message after a buy), trade ──
    {
        let mut h = Harness::new(17);
        h.start_dev_game();
        to_trench(&mut h);
        h.game.pearls = 10;
        open_hermie(&mut h);
        shots.snap_game("01_hermie_browse", &mut h);
        h.buy_shop_item("kelp_crown");
        shots.snap_game("02_hermie_after_buy_message", &mut h);
        h.select_shop_item("diving_net"); // can't afford → shortfall message
        shots.snap_game("03_hermie_cant_afford", &mut h);
        h.game.pearls = 30;
        h.select_shop_item("trade_desk");
        shots.snap_game("04_hermie_trade_30_pearls", &mut h);
    }

    // ── Bolt: buying view ──
    {
        let mut h = Harness::new(11);
        h.start_dev_game();
        open_bolt(&mut h);
        shots.snap_game("05_bolt_browse", &mut h);
        h.select_shop_item("sparkle_trail");
        shots.snap_game("06_bolt_buying_math", &mut h);
    }

    // ── Swag picker with a full wardrobe (Bolt + Hermie swag) ──
    {
        let mut h = Harness::new(11);
        h.start_dev_game();
        h.game.dum_dums = 99;
        open_bolt(&mut h);
        for id in ["hat", "bow_tie", "jet_boots", "sparkle_trail"] {
            h.buy_shop_item(id);
        }
        h.close_shop();
        to_trench(&mut h);
        h.game.pearls = 99;
        open_hermie(&mut h);
        for id in ["kelp_crown", "shell_necklace", "starfish_badge", "glow_lantern"] {
            h.buy_shop_item(id);
        }
        h.close_shop();
        to_home(&mut h);
        h.walk_to_npc(NpcKind::Kid1);
        h.interact();
        h.select_option("swag");
        h.wait_until(|g| g.state == GameState::Swag);
        shots.snap_game("07_swag_8_items", &mut h);
        h.give_swag("kelp_crown");
        shots.snap_game("08_swag_after_give_message", &mut h);
    }

    // ── Gift picker: hand Tali Color Change, pick her colour ──
    {
        let mut h = Harness::new(11);
        h.start_dev_game();
        h.game.dum_dums = 99;
        open_bolt(&mut h);
        h.buy_shop_item("color_change");
        h.pick_shop_color("red");
        h.close_shop();
        to_home(&mut h);
        h.walk_to_npc(NpcKind::Kid1);
        h.interact();
        h.select_option("swag");
        h.wait_until(|g| g.state == GameState::Swag);
        h.give_swag("color_change");
        shots.snap_game("08b_swag_colour_for_tali", &mut h);
        h.pick_swag_color("teal");
        shots.snap_game("08c_swag_tali_picked_teal", &mut h);
    }

    // ── Settings with the parent section open ──
    {
        let mut h = Harness::new(3);
        h.start_dev_game();
        h.open_settings();
        shots.snap_game("09_settings", &mut h);
        h.click_parent_options();
        shots.snap_game("10_settings_parent", &mut h);
    }

    // ── Quest: first few beats ──
    {
        let mut h = Harness::new(7);
        h.start_dev_game();
        h.walk_to(2, 9);
        h.step_through_portal(KeyCode::Left, "control");
        h.walk_to_npc(NpcKind::CtrlStartQuest);
        h.interact();
        h.wait_until(|g| g.state == GameState::Quest);
        for beat in 0..6 {
            shots.snap_game(&format!("11_quest_beat_{beat}"), &mut h);
            if h.game.state != GameState::Quest { break; }
            h.press(KeyCode::Space);
            if h.game.state == GameState::Quest {
                // A puzzle/choice beat ignores Space; pick the first tile.
                h.press(KeyCode::Key1);
            }
        }
    }

    // ── A live in-game challenge (whatever the dev map's first NPC asks) ──
    {
        let mut h = Harness::new(5);
        h.start_dev_game();
        h.walk_to_npc(NpcKind::Kid1);
        h.interact();
        if h.game.state == GameState::InteractionMenu {
            shots.snap_game("12_interaction_menu", &mut h);
        }
    }

    // ── The Goyish Map shooter: lanes, a mid-hop glide, the clean-wave cheer ──
    {
        let mut h = Harness::new(7);
        h.start_dev_game();
        h.game.map = Map::goyish_map();
        h.game.npcs = npc_mod::npcs_for_map("goyish_map");
        h.game.npcs_offstage.clear();
        h.game.sparky_parked = true;
        h.warp_to(6, 4);
        h.hold(KeyCode::Up);
        h.interact();
        h.wait_until(|g| g.state == GameState::Shooter);
        h.advance(240); // let the wave drift down a way
        shots.snap_game("15_shooter_in_a_lane", &mut h);
        h.press(KeyCode::Right);
        shots.snap_game("16_shooter_mid_hop", &mut h);
        shooter_clear_wave(&mut h);
        shots.snap_game("17_shooter_perfect_wave", &mut h);
        shooter_clear_wave(&mut h);
        shooter_clear_wave(&mut h);
        shots.snap_game("18_shooter_all_clear", &mut h);
    }

    // ── The Dogfish House: dive with Inkwell from a map with no shaft ──
    // Must read as underwater AND glitchy at once.
    {
        let mut h = Harness::new(9);
        h.start_dev_game();
        to_home(&mut h);
        h.game.npcs.clear();
        h.bring_buddy(NpcKind::Octopus, "reef");
        h.dive_with_buddy();
        h.advance(120); // let the arrival line type out
        shots.snap_game("19_dogfish_house_arrival", &mut h);
        h.finish_dialogue();
        h.walk_to(7, 8);
        h.advance(40);
        shots.snap_game("20_dogfish_house", &mut h);
    }

    // ── Panels drawn directly: a 4-line dialogue and a wrapped word problem ──
    {
        use robot_buddy_game::ui::dialogue::{DialogueBox, DialogueLine};
        let text = "Oh no, oh no! The storm blew all of my lanterns off the shelf and into the harbour, \
            and the fishing boats are coming home tonight. If the lighthouse is dark they'll never find the \
            way in. Could you and Sparky count how many are still up here with me, and how many we need to fish out?";
        let mut d = DialogueBox::new();
        d.start(vec![DialogueLine { speaker: "Bolt the Shopkeeper".into(), text: text.into() }]);
        for _ in 0..600 { d.update(1.0 / 60.0); }
        shots.snap("13_dialogue_4_lines", |screen| d.draw(screen));
    }
    {
        use ::rand::SeedableRng;
        use robot_buddy_domain::challenge::challenge_state::{
            challenge_reducer, ChallengeAction, ChallengeState, DisplaySpeech, RenderHint, VoiceState,
        };
        use robot_buddy_domain::learning::challenge_generator::{generate_challenge, ChallengeProfile};
        use robot_buddy_domain::learning::operation_stats::OperationStats;
        use robot_buddy_domain::types::{CraStage, Phase};
        use robot_buddy_game::ui::challenge;

        let q = "A frog hops 7 times across the lily pads, then 3 more times to reach the far bank! How many hops is that?";
        let mut rng = ::rand::rngs::SmallRng::seed_from_u64(42);
        for (band, stage) in [(2u8, CraStage::Concrete), (6, CraStage::Representational)] {
            let profile = ChallengeProfile { math_band: band, spread_width: 0.0, operation_stats: OperationStats::new() };
            let c = generate_challenge(&profile, &mut rng);
            let p = ChallengeState {
                phase: Phase::Presented,
                correct_answer: c.correct_answer,
                attempts: 0,
                max_attempts: 2,
                correct: None,
                question: DisplaySpeech { display: q.into(), speech: q.into() },
                feedback: None,
                reward: None,
                render_hint: RenderHint { cra_stage: stage, answer_mode: "choice".into(), interaction_type: "quiz".into() },
                hint_used: false,
                hint_level: 0,
                told_me: false,
                voice: VoiceState::reset(),
            };
            let wrong = c.choices.iter().find(|ch| !ch.correct).and_then(|ch| ch.text.parse().ok()).unwrap_or(c.correct_answer + 1);
            let hinted = challenge_reducer(p.clone(), ChallengeAction::ShowMe);
            let hinted_fed = challenge_reducer(hinted.clone(), ChallengeAction::AnswerSubmitted { answer: wrong });
            let taught = challenge_reducer(
                challenge_reducer(p.clone(), ChallengeAction::AnswerSubmitted { answer: wrong }),
                ChallengeAction::AnswerSubmitted { answer: wrong },
            );
            for (name, cs) in [("presented", &p), ("showme_feedback", &hinted_fed), ("taught", &taught)] {
                shots.snap(&format!("14_challenge_band{band}_{name}"), |screen| {
                    let l = challenge::layout(cs, &c, screen);
                    challenge::draw(&l, cs, &c, 1.0);
                });
            }
        }
    }
    shots
}

/// Frames each shot is shown before its screenshot (text needs one to lay out).
const SETTLE: u32 = 4;
/// Frames between shots.
const PER_SHOT: u32 = 8;

fn main() {
    let shots = record();
    let (w, h) = shot_screen();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "robot-buddy screenshots".into(),
                resolution: WindowResolution::new(w as u32, h as u32).with_scale_factor_override(1.0),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DrawPlugin)
        .insert_resource(shots)
        .add_systems(Update, show_shots)
        .run();
}

fn show_shots(
    mut frame: Local<u32>,
    shots: Res<Shots>,
    mut list: ResMut<DrawList>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    let i = (*frame / PER_SHOT) as usize;
    let phase = *frame % PER_SHOT;
    *frame += 1;
    let Some((name, cmds)) = shots.0.get(i) else {
        if phase == PER_SHOT - 1 {
            exit.write(AppExit::Success);
        }
        return;
    };
    list.0 = cmds.clone();
    if phase == SETTLE {
        let dir = std::env::var("SHOT_DIR").unwrap_or_else(|_| "shots".into());
        std::fs::create_dir_all(&dir).unwrap();
        let path = format!("{dir}/{name}.png");
        println!("{path}");
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
}
