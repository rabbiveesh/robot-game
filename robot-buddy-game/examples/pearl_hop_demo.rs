//! Pearl Hop demo video: a scripted run through the real `Game`, every frame
//! recorded by the real render code (screen draws and Bevy UI panels), with
//! the simulated finger drawn on top so the drag is visible. Then a Bevy
//! window plays the recording back one frame at a time, saves each as a PNG,
//! and `ffmpeg` turns the PNGs into an MP4.
//!
//!   DEMO_DIR=/tmp/pearl_hop_frames DEMO_MP4=/tmp/pearl_hop_demo.mp4 \
//!     cargo run --release -p robot-buddy-game --example pearl_hop_demo --features screenshots
//!
//! DEMO_FFMPEG picks the ffmpeg (default `ffmpeg` on PATH). The game is the
//! test harness's 960×720 (so its tap helpers line up), one PNG pixel per
//! logical pixel. In the cloud, run it under `xvfb-run -a -s "-screen 0 1920x1080x24"`.

#[path = "../tests/common/mod.rs"]
#[allow(dead_code)]
mod common;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::WindowResolution;
use common::{Harness, SCREEN};
use robot_buddy_domain::logic::pearl_hop::{HopPhase, Landing};
use robot_buddy_game::app::{DrawList, DrawPlugin};
use robot_buddy_game::draw::{self, Cmd, Color};
use robot_buddy_game::game::GameState;
use robot_buddy_game::input::{FrameInput, KeyCode};
use robot_buddy_game::npc::{self as npc_mod, NpcKind};
use robot_buddy_game::tilemap::Map;
use robot_buddy_game::ui::kit::ArtSizes;
use robot_buddy_game::ui::pearl_hop::HopId;
use robot_buddy_game::ui::views::UiViews;
use robot_buddy_game::ui::UiAction;

const FPS: f32 = 30.0;
const DT: f32 = 1.0 / FPS;

/// One recorded frame: the clock, the screen's draw calls (finger and
/// banner included) and the panels' views.
struct Frame {
    time: f64,
    cmds: Vec<Cmd>,
    views: UiViews,
}

/// Drives the game one frame at a time, recording each.
struct Director {
    h: Harness,
    frames: Vec<Frame>,
    /// The simulated finger, in logical screen coordinates.
    cursor: (f32, f32),
    pressed: bool,
    show_cursor: bool,
    /// A small title card for whoever's watching (not part of the game).
    banner: String,
}

impl Director {
    fn screen(&self) -> (f32, f32) {
        SCREEN
    }

    fn frame(&mut self, input: FrameInput) {
        let screen = self.screen();
        self.h.game.step(&input, DT, screen);
        let time = self.frames.len() as f64 * DT as f64;
        draw::begin_frame(screen, time);
        self.h.game.render(screen, &input);
        self.overlay();
        let cmds = draw::take_commands();
        self.frames.push(Frame { time, cmds, views: self.h.game.ui_views(screen) });
    }

    fn overlay(&self) {
        if !self.banner.is_empty() {
            let size = 18.0;
            let w = draw::measure_text(&self.banner, None, size as u16, 1.0).width;
            // Just under the top row, clear of the game's buttons.
            let (x, y) = (self.screen().0 / 2.0 - w / 2.0 - 12.0, 62.0);
            draw::draw_rectangle(x, y, w + 24.0, 28.0, Color::new(0.0, 0.0, 0.0, 0.55));
            draw::draw_text(&self.banner, x + 12.0, y + 20.0, size, Color::new(1.0, 1.0, 1.0, 0.9));
        }
        if self.show_cursor {
            let (x, y) = self.cursor;
            // A fingertip: a soft disc, darker when pressed, with a ring.
            let fill = if self.pressed { Color::new(1.0, 0.55, 0.2, 0.75) } else { Color::new(1.0, 1.0, 1.0, 0.55) };
            draw::draw_circle(x, y, if self.pressed { 13.0 } else { 15.0 }, fill);
            draw::draw_circle_lines(x, y, 15.0, 2.5, Color::new(0.1, 0.1, 0.2, 0.9));
            if self.pressed {
                draw::draw_circle_lines(x, y, 22.0, 2.0, Color::new(1.0, 0.55, 0.2, 0.6));
            }
        }
    }

    fn input_here(&self) -> FrameInput {
        let (x, y) = self.cursor;
        if self.pressed { FrameInput::empty().with_mouse_held(x, y) } else { FrameInput::empty().with_mouse_at(x, y) }
    }

    fn idle(&mut self, secs: f32) {
        for _ in 0..(secs * FPS).round() as usize {
            let i = self.input_here();
            self.frame(i);
        }
    }

    /// Glide the finger to `to` over `secs` (dragging if it's pressed).
    fn glide(&mut self, to: (f32, f32), secs: f32) {
        let from = self.cursor;
        let n = ((secs * FPS).round() as usize).max(1);
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let e = t * t * (3.0 - 2.0 * t);
            self.cursor = (from.0 + (to.0 - from.0) * e, from.1 + (to.1 - from.1) * e);
            let inp = self.input_here();
            self.frame(inp);
        }
    }

    fn press(&mut self) {
        self.pressed = true;
        let (x, y) = self.cursor;
        self.frame(FrameInput::empty().with_mouse_click(x, y));
    }

    fn release(&mut self) {
        self.pressed = false;
        let (x, y) = self.cursor;
        self.frame(FrameInput::empty().with_mouse_release(x, y));
    }

    fn tap(&mut self, at: (f32, f32)) {
        self.glide(at, 0.6);
        self.idle(0.15);
        self.press();
        self.release();
    }

    fn key(&mut self, k: KeyCode) {
        let i = self.input_here().with_key_pressed(k);
        self.frame(i);
    }

    fn wait_until(&mut self, pred: impl Fn(&Harness) -> bool, max_secs: f32) {
        for _ in 0..(max_secs * FPS) as usize {
            if pred(&self.h) {
                return;
            }
            let i = self.input_here();
            self.frame(i);
        }
        eprintln!("wait_until: gave up after {max_secs}s");
    }

    /// Walk the kid one tile at a time by holding an arrow key.
    fn walk(&mut self, key: KeyCode, tiles: usize) {
        for _ in 0..tiles {
            let start = (self.h.game.player.tile_x, self.h.game.player.tile_y);
            for _ in 0..30 {
                let i = self.input_here().with_key_down(key);
                self.frame(i);
                let now = (self.h.game.player.tile_x, self.h.game.player.tile_y);
                if now != start && self.h.game.player_at_rest() {
                    break;
                }
            }
        }
    }

    /// Drag Shelly back to the pull for `aim` — slowly, so the ghost arc and
    /// the count-aloud can be followed — hold a beat, and let go. Then watch.
    fn fling(&mut self, aim: u16, drag_secs: f32) {
        let l = self.h.game.pearl_hop_layout(self.screen()).expect("Pearl Hop open");
        let home = l.scene.home();
        let target = l.scene.drag_point_for(aim);
        self.glide(home, 0.7);
        self.idle(0.2);
        self.press();
        self.glide(target, drag_secs);
        self.idle(0.6);
        self.release();
        // Move the finger out of the way while she flies.
        let rest = (self.cursor.0 + 60.0, self.cursor.1 + 40.0);
        self.glide(rest, 0.4);
        self.wait_until(
            |h| h.game.active_pearl_hop().is_none_or(|a| a.session.phase == HopPhase::Aiming || a.session.win_done()),
            14.0,
        );
    }

    fn round(&self) -> robot_buddy_domain::logic::pearl_hop::HopRound {
        self.h.game.active_pearl_hop().unwrap().session.round.clone()
    }

    /// A miss of the given kind, as close to the pearl as there is — it reads
    /// as "almost".
    fn aim_landing(&self, want: Landing) -> u16 {
        let r = self.round();
        let mut aims: Vec<u16> = (r.min_aim..=r.max_aim).collect();
        aims.sort_by_key(|&a| r.resolve(a).end().abs_diff(r.pearl));
        aims.into_iter().find(|&a| r.resolve(a).landing == want).expect("an aim with that landing")
    }

    fn win_aim(&self) -> u16 {
        self.round().winning_aims()[0]
    }

    fn again_with_band(&mut self, band: u8) {
        self.h.game.profile.math_band = band;
        self.tap_ui(UiAction::PearlHop(HopId::Again));
    }

    /// Tap a Bevy UI button the way a finger does: glide to it, press, let go.
    fn tap_ui(&mut self, action: UiAction) {
        let at = self.h.button_center(action);
        self.glide(at, 0.6);
        self.idle(0.15);
        self.pressed = true;
        self.frame(FrameInput::empty().with_mouse_click(at.0, at.1).with_ui_press(action));
        self.release();
    }
}

/// Dress Shelly up off camera, the way a kid would: buy Color Change from
/// Bolt and a Kelp Crown from Hermie, then hand both to her on the reef.
fn dress_shelly(h: &mut Harness) {
    h.game.dum_dums = 99;
    h.walk_to_npc(NpcKind::Shopkeeper);
    h.interact();
    h.select_option("shop");
    h.wait_until(|g| g.state == GameState::Shop);
    h.buy_shop_item("color_change");
    h.pick_shop_color("orange");
    h.close_shop();
    h.visit_map("trench", 22, 9);
    h.game.pearls = 30;
    h.walk_to_npc(NpcKind::HermitCrab);
    h.interact();
    h.select_option("shop");
    h.wait_until(|g| g.state == GameState::Shop);
    h.buy_shop_item("kelp_crown");
    h.close_shop();
    h.game.pearls = 0;
    h.visit_map("reef", 5, 13);
    for item in ["kelp_crown", "color_change"] {
        h.walk_to_npc(NpcKind::Clam);
        h.interact();
        h.select_option("swag");
        h.wait_until(|g| g.state == GameState::Swag);
        h.give_swag(item);
        if item == "color_change" {
            h.pick_swag_color("orange");
        }
        h.close_swag();
    }
    assert!(h.game.swag_worn_by("clam").contains("kelp_crown"));
}

fn to_reef(h: &mut Harness) {
    h.game.map = Map::reef();
    // Keep the demo's cast still: Shelly, Inkwell and a couple of friends.
    h.game.npcs = npc_mod::npcs_for_map("reef")
        .into_iter()
        .filter(|n| matches!(n.kind, NpcKind::Clam | NpcKind::Octopus | NpcKind::ReefShark))
        .collect();
    h.game.npcs_offstage.clear();
}

fn main() {
    let dir = std::env::var("DEMO_DIR").unwrap_or_else(|_| "pearl_hop_frames".into());
    std::fs::create_dir_all(&dir).unwrap();
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        if entry.path().extension().is_some_and(|e| e == "png") {
            let _ = std::fs::remove_file(entry.path());
        }
    }

    let mut h = Harness::new(2024);
    h.start_dev_game();
    dress_shelly(&mut h);
    to_reef(&mut h);
    h.game.profile.math_band = 1;
    let mut d = Director {
        h, frames: Vec::new(), cursor: (0.0, 0.0), pressed: false, show_cursor: false, banner: String::new(),
    };
    println!("screen {:?}", d.screen());
    d.cursor = (d.screen().0 * 0.6, d.screen().1 * 0.6);

    // 1. The reef: Inkwell's and Shelly's beacons, then walk up and talk.
    d.banner = "Inkwell runs the dive: her beacon says there's a game here".into();
    d.h.warp_to(33, 8);
    d.idle(3.0);
    d.banner = "Shelly's beacon: a pearl. Walk up and talk to her".into();
    d.h.warp_to(8, 13);
    d.idle(1.5);
    d.walk(KeyCode::Left, 4);
    d.idle(0.6);
    d.key(KeyCode::Space);
    d.idle(0.4);
    if d.h.game.state == GameState::InteractionMenu {
        d.show_cursor = true;
        let menu = robot_buddy_game::ui::interaction_menu::layout(&d.h.game.menu_options, d.screen());
        let talk = menu.buttons.iter().find(|b| b.option_type == "talk").unwrap().rect;
        d.tap((talk.0 + talk.2 / 2.0, talk.1 + talk.3 / 2.0));
    }
    assert_eq!(d.h.game.state, GameState::PearlHop, "talking to Shelly opens Pearl Hop");

    // 2. Her first-time show-off: she flings herself (misses), then a hand
    //    shows the pull.
    d.show_cursor = false;
    d.banner = "First time ever: Shelly shows off (and misses on purpose), then a hand shows the pull".into();
    d.wait_until(|h| h.game.active_pearl_hop().is_some_and(|a| a.demo.is_none()), 20.0);
    d.idle(0.8);

    // 3. Stage 1: the pearl hides under one of the stones; the dots say how
    //    many. A short toss is counted out — one, two, three — with dots
    //    left over; then the right one.
    d.show_cursor = true;
    d.banner = "Stage 1: the dots say how many stones. Count to find the hidden pearl".into();
    let pearl = d.round().pearl;
    let short = 3.min(pearl - 1);
    d.fling(short, 1.8);
    d.idle(0.4);
    let win = d.win_aim();
    d.fling(win, 1.8);
    d.idle(1.0);

    // 4. Stage 2: skip counting. One overshoot, then a size that lands.
    d.again_with_band(2);
    d.banner = "Stage 2: she always hops the same size, counting out loud (skip counting)".into();
    d.idle(1.2);
    let over = d.aim_landing(Landing::Past);
    d.fling(over, 1.4);
    d.idle(0.4);
    let win = d.win_aim();
    d.fling(win, 1.4);
    d.idle(1.0);

    // 5. Stage 3: reach the pearl in X hops (division), first try.
    d.again_with_band(3);
    d.banner = "Stage 3: reach the pearl in X hops (the bubbles). Pick the hop size".into();
    d.idle(1.4);
    let win = d.win_aim();
    d.fling(win, 1.5);
    d.idle(1.0);

    // 6. Higher bands: the same X hops with bigger numbers, on a base-ten
    //    path. One wrong pick, then the right one.
    d.again_with_band(6);
    d.banner = "Higher bands: bigger numbers, same idea. The path groups in fives and tens".into();
    d.idle(1.6);
    let short = d.aim_landing(Landing::Short);
    d.fling(short, 1.6);
    d.idle(0.4);
    let win = d.win_aim();
    d.fling(win, 1.6);
    d.idle(1.2);

    // 7. Leave, back to the reef.
    d.banner = "Leave is always one tap away".into();
    d.tap_ui(UiAction::PearlHop(HopId::Leave));
    d.show_cursor = false;
    d.banner = "Back on the reef".into();
    d.idle(2.5);

    println!("{} frames ({:.1}s); pearls: {}", d.frames.len(), d.frames.len() as f32 / FPS, d.h.game.pearls);
    play(d.frames, dir);
}

/// Frames each recorded frame is shown before its screenshot (text needs one
/// to lay out).
const SETTLE: u32 = 1;
/// App frames per recorded frame.
const PER_FRAME: u32 = 2;
/// Frames before the first one, while the renderer builds its pipelines.
const WARMUP: u32 = 30;
/// Frames after the last screenshot, for the PNG writes to finish.
const DRAIN: u32 = 60;

#[derive(Resource)]
struct Recording {
    frames: Vec<Frame>,
    dir: String,
}

fn play(frames: Vec<Frame>, dir: String) {
    let (w, h) = SCREEN;
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Pearl Hop demo".into(),
                resolution: WindowResolution::new(w as u32, h as u32).with_scale_factor_override(1.0),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DrawPlugin)
        .insert_resource(Recording { frames, dir })
        .add_systems(Update, show_frame.before(robot_buddy_game::ui::kit::PanelSystems))
        .run();
}

fn show_frame(
    mut tick: Local<u32>,
    rec: Res<Recording>,
    mut list: ResMut<DrawList>,
    mut views: ResMut<UiViews>,
    art: Res<ArtSizes>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    *tick += 1;
    let Some(t) = tick.checked_sub(WARMUP) else { return };
    let i = (t / PER_FRAME) as usize;
    let Some(f) = rec.frames.get(i) else {
        if t >= rec.frames.len() as u32 * PER_FRAME + DRAIN {
            encode(&rec.dir);
            exit.write(AppExit::Success);
        }
        return;
    };
    *views = f.views.clone();
    draw::begin_frame(SCREEN, f.time);
    views.draw_all_art(&art.0);
    list.0 = f.cmds.iter().cloned().chain(draw::take_commands()).collect();
    if t % PER_FRAME == SETTLE {
        let path = format!("{}/{:05}.png", rec.dir, i + 1);
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
}

/// The numbered PNGs into an H.264 MP4 (yuv420p, even size).
fn encode(dir: &str) {
    let mp4 = std::env::var("DEMO_MP4").unwrap_or_else(|_| "pearl_hop_demo.mp4".into());
    let ffmpeg = std::env::var("DEMO_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let status = std::process::Command::new(ffmpeg)
        .args(["-y", "-loglevel", "error", "-framerate", &format!("{FPS}"), "-i", &format!("{dir}/%05d.png")])
        .args(["-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2", "-c:v", "libx264", "-preset", "medium"])
        .args(["-crf", "18", "-pix_fmt", "yuv420p", "-movflags", "+faststart", &mp4])
        .status();
    match status {
        Ok(s) => println!("ffmpeg: {s}; video at {mp4}"),
        Err(e) => eprintln!("ffmpeg didn't run ({e}); the frames are in {dir}"),
    }
}
