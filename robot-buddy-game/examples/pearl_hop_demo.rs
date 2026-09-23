//! Pearl Hop demo video: a scripted run through the real `Game`, every frame
//! rendered by the real draw code and saved as a PNG, with the simulated
//! finger drawn on top so the drag is visible.
//!
//!   DEMO_DIR=/tmp/pearl_hop_frames DEMO_MP4=/tmp/pearl_hop_demo.mp4 \
//!     cargo run --release -p robot-buddy-game --example pearl_hop_demo
//!
//! Frames stream as raw RGBA into one `ffmpeg` process (DEMO_FFMPEG, default
//! `ffmpeg` on PATH), which writes both the numbered PNGs and the H.264 MP4
//! (yuv420p, even size) — encoding PNGs in-process ran at ~1 frame/second.
//!
//! The window opens at DEMO_W×DEMO_H physical pixels (default 1920×1080). On
//! a 2× display macroquad lays out at half that — a 960×540 game.

#[path = "../tests/common/mod.rs"]
#[allow(dead_code)]
mod common;

use common::Harness;
use macroquad::prelude::*;
use robot_buddy_domain::logic::pearl_hop::{HopPhase, Landing};
use robot_buddy_game::game::GameState;
use robot_buddy_game::input::FrameInput;
use robot_buddy_game::npc::{self as npc_mod, NpcKind};
use robot_buddy_game::tilemap::Map;

const FPS: f32 = 30.0;
const DT: f32 = 1.0 / FPS;

fn env_f(key: &str, default: f32) -> f32 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn conf() -> Conf {
    Conf {
        window_title: "Pearl Hop demo".into(),
        window_width: env_f("DEMO_W", 1920.0) as i32,
        window_height: env_f("DEMO_H", 1080.0) as i32,
        window_resizable: false,
        high_dpi: true,
        // Render as fast as the encoder takes frames; no vsync wait.
        platform: miniquad::conf::Platform { swap_interval: Some(0), ..Default::default() },
        ..Default::default()
    }
}

/// Drives the game one rendered frame at a time.
struct Director {
    h: Harness,
    /// ffmpeg, fed raw RGBA frames on stdin. Started on the first frame, once
    /// the framebuffer size is known.
    enc: Option<std::process::Child>,
    dir: String,
    mp4: String,
    frame_no: usize,
    /// The simulated finger, in logical screen coordinates.
    cursor: (f32, f32),
    pressed: bool,
    show_cursor: bool,
    /// A small title card for whoever's watching (not part of the game).
    banner: String,
}

impl Director {
    fn screen(&self) -> (f32, f32) {
        (screen_width(), screen_height())
    }

    async fn frame(&mut self, input: FrameInput) {
        let screen = self.screen();
        self.h.game.step(&input, DT, screen);
        clear_background(BLACK);
        self.h.game.render(screen, &input);
        self.overlay();
        let img = get_screen_data();
        let (w, h) = (img.width as usize, img.height as usize);
        let enc = self.enc.get_or_insert_with(|| spawn_encoder(w, h, &self.dir, &self.mp4));
        use std::io::Write;
        enc.stdin.as_mut().unwrap().write_all(&img.bytes).expect("ffmpeg took the frame");
        self.frame_no += 1;
        next_frame().await;
    }

    fn overlay(&self) {
        if !self.banner.is_empty() {
            let size = 18.0;
            let w = measure_text(&self.banner, None, size as u16, 1.0).width;
            // Just under the top row, clear of the game's buttons.
            let (x, y) = (screen_width() / 2.0 - w / 2.0 - 12.0, 62.0);
            draw_rectangle(x, y, w + 24.0, 28.0, Color::new(0.0, 0.0, 0.0, 0.55));
            draw_text(&self.banner, x + 12.0, y + 20.0, size, Color::new(1.0, 1.0, 1.0, 0.9));
        }
        if self.show_cursor {
            let (x, y) = self.cursor;
            // A fingertip: a soft disc, darker when pressed, with a ring.
            let fill = if self.pressed { Color::new(1.0, 0.55, 0.2, 0.75) } else { Color::new(1.0, 1.0, 1.0, 0.55) };
            draw_circle(x, y, if self.pressed { 13.0 } else { 15.0 }, fill);
            draw_circle_lines(x, y, 15.0, 2.5, Color::new(0.1, 0.1, 0.2, 0.9));
            if self.pressed {
                draw_circle_lines(x, y, 22.0, 2.0, Color::new(1.0, 0.55, 0.2, 0.6));
            }
        }
    }

    fn input_here(&self) -> FrameInput {
        let (x, y) = self.cursor;
        if self.pressed { FrameInput::empty().with_mouse_held(x, y) } else { FrameInput::empty().with_mouse_at(x, y) }
    }

    async fn idle(&mut self, secs: f32) {
        for _ in 0..(secs * FPS).round() as usize {
            let i = self.input_here();
            self.frame(i).await;
        }
    }

    /// Glide the finger to `to` over `secs` (dragging if it's pressed).
    async fn glide(&mut self, to: (f32, f32), secs: f32) {
        let from = self.cursor;
        let n = ((secs * FPS).round() as usize).max(1);
        for i in 1..=n {
            let t = i as f32 / n as f32;
            let e = t * t * (3.0 - 2.0 * t);
            self.cursor = (from.0 + (to.0 - from.0) * e, from.1 + (to.1 - from.1) * e);
            let inp = self.input_here();
            self.frame(inp).await;
        }
    }

    async fn press(&mut self) {
        self.pressed = true;
        let (x, y) = self.cursor;
        self.frame(FrameInput::empty().with_mouse_click(x, y)).await;
    }

    async fn release(&mut self) {
        self.pressed = false;
        let (x, y) = self.cursor;
        self.frame(FrameInput::empty().with_mouse_release(x, y)).await;
    }

    async fn tap(&mut self, at: (f32, f32)) {
        self.glide(at, 0.6).await;
        self.idle(0.15).await;
        self.press().await;
        self.release().await;
    }

    async fn key(&mut self, k: KeyCode) {
        let i = self.input_here().with_key_pressed(k);
        self.frame(i).await;
    }

    async fn wait_until(&mut self, pred: impl Fn(&Harness) -> bool, max_secs: f32) {
        for _ in 0..(max_secs * FPS) as usize {
            if pred(&self.h) {
                return;
            }
            let i = self.input_here();
            self.frame(i).await;
        }
        eprintln!("wait_until: gave up after {max_secs}s");
    }

    /// Walk the kid one tile at a time by holding an arrow key.
    async fn walk(&mut self, key: KeyCode, tiles: usize) {
        for _ in 0..tiles {
            let start = (self.h.game.player.tile_x, self.h.game.player.tile_y);
            for _ in 0..30 {
                let i = self.input_here().with_key_down(key);
                self.frame(i).await;
                let now = (self.h.game.player.tile_x, self.h.game.player.tile_y);
                if now != start && self.h.game.player_at_rest() {
                    break;
                }
            }
        }
    }

    /// Drag Shelly back to the pull for `aim` — slowly, so the ghost arc and
    /// the count-aloud can be followed — hold a beat, and let go. Then watch.
    async fn fling(&mut self, aim: u16, drag_secs: f32) {
        let l = self.h.game.pearl_hop_layout(self.screen()).expect("Pearl Hop open");
        let home = l.scene.home();
        let target = l.scene.drag_point_for(aim);
        self.glide(home, 0.7).await;
        self.idle(0.2).await;
        self.press().await;
        self.glide(target, drag_secs).await;
        self.idle(0.6).await;
        self.release().await;
        // Move the finger out of the way while she flies.
        let rest = (self.cursor.0 + 60.0, self.cursor.1 + 40.0);
        self.glide(rest, 0.4).await;
        self.wait_until(
            |h| h.game.active_pearl_hop().is_none_or(|a| matches!(a.session.phase, HopPhase::Won | HopPhase::Aiming)),
            12.0,
        ).await;
    }

    fn round(&self) -> robot_buddy_domain::logic::pearl_hop::HopRound {
        self.h.game.active_pearl_hop().unwrap().session.round.clone()
    }

    fn aim_landing(&self, want: Landing, near: Option<bool>) -> u16 {
        let r = self.round();
        let mut aims: Vec<u16> = (r.min_aim..=r.max_aim).collect();
        // Prefer a miss close to the pearl — it reads as "almost".
        let p = r.pearl_pos();
        aims.sort_by_key(|&a| r.resolve(a).end().abs_diff(p));
        aims.into_iter()
            .find(|&a| {
                let t = r.resolve(a);
                t.landing == want && near.is_none_or(|n| t.near == n)
            })
            .expect("an aim with that landing")
    }

    fn win_aim(&self) -> u16 {
        let r = self.round();
        let wins = r.winning_aims();
        // On the estimation line, the middle of the target window.
        wins[wins.len() / 2]
    }

    async fn again_with_band(&mut self, band: u8) {
        self.h.game.profile.math_band = band;
        let l = self.h.game.pearl_hop_layout(self.screen()).unwrap();
        let at = l.again().expect("Again! is up after a win").center();
        self.tap(at).await;
    }
}

/// One ffmpeg reading raw RGBA (bottom-up, as GL reads it) and writing the
/// PNG frames and the MP4 side by side.
fn spawn_encoder(w: usize, h: usize, dir: &str, mp4: &str) -> std::process::Child {
    let ffmpeg = std::env::var("DEMO_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    std::process::Command::new(ffmpeg)
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba"])
        .args(["-s", &format!("{w}x{h}"), "-r", &format!("{FPS}"), "-i", "-"])
        .args(["-vf", "vflip", &format!("{dir}/%05d.png")])
        .args(["-vf", "vflip,scale=trunc(iw/2)*2:trunc(ih/2)*2", "-c:v", "libx264", "-preset", "medium"])
        .args(["-crf", "18", "-pix_fmt", "yuv420p", "-movflags", "+faststart", mp4])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("start ffmpeg (set DEMO_FFMPEG)")
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

#[macroquad::main(conf)]
async fn main() {
    if let Err(e) = robot_buddy_game::text::init() {
        eprintln!("\n!!! {e}\n");
    }
    let dir = std::env::var("DEMO_DIR").unwrap_or_else(|_| "pearl_hop_frames".into());
    std::fs::create_dir_all(&dir).unwrap();
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        if entry.path().extension().is_some_and(|e| e == "png") {
            let _ = std::fs::remove_file(entry.path());
        }
    }

    let mut h = Harness::new(2024);
    h.start_dev_game();
    to_reef(&mut h);
    h.game.profile.math_band = 1;
    let mp4 = std::env::var("DEMO_MP4").unwrap_or_else(|_| "pearl_hop_demo.mp4".into());
    let mut d = Director {
        h, enc: None, dir, mp4, frame_no: 0, cursor: (0.0, 0.0), pressed: false, show_cursor: false, banner: String::new(),
    };
    println!("screen {:?}", d.screen());
    d.cursor = (d.screen().0 * 0.6, d.screen().1 * 0.6);

    // 1. The reef: Inkwell's and Shelly's beacons, then walk up and talk.
    d.banner = "Inkwell runs the dive: her beacon says there's a game here".into();
    d.h.warp_to(33, 8);
    d.idle(3.0).await;
    d.banner = "Shelly's beacon: a pearl. Walk up and talk to her".into();
    d.h.warp_to(8, 13);
    d.idle(1.5).await;
    d.walk(KeyCode::Left, 4).await;
    d.idle(0.6).await;
    d.key(KeyCode::Space).await;
    d.idle(0.4).await;
    if d.h.game.state == GameState::InteractionMenu {
        d.show_cursor = true;
        let menu = robot_buddy_game::ui::interaction_menu::layout(&d.h.game.menu_options, d.screen());
        let talk = menu.buttons.iter().find(|b| b.option_type == "talk").unwrap().rect;
        d.tap((talk.0 + talk.2 / 2.0, talk.1 + talk.3 / 2.0)).await;
    }
    assert_eq!(d.h.game.state, GameState::PearlHop, "talking to Shelly opens Pearl Hop");

    // 2. Her first-time show-off: she flings herself (misses), then a hand
    //    shows the pull.
    d.show_cursor = false;
    d.banner = "First time ever: Shelly shows off (and misses on purpose), then a hand shows the pull".into();
    d.wait_until(|h| h.game.active_pearl_hop().is_some_and(|a| a.demo.is_none()), 20.0).await;
    d.idle(0.8).await;

    // 3. Stage 1: count the stones. A bit too far first — sploosh — then right.
    d.show_cursor = true;
    d.banner = "Stage 1 (youngest): one toss, count the stones to the pearl".into();
    let past = d.aim_landing(Landing::Past, None);
    d.fling(past, 1.8).await;
    d.idle(0.4).await;
    let win = d.win_aim();
    d.fling(win, 1.6).await;
    d.idle(2.0).await;

    // 4. Stage 2: skip counting. One overshoot, then a size that lands.
    d.again_with_band(2).await;
    d.banner = "Stage 2: she always hops the same size, counting out loud (skip counting)".into();
    d.idle(1.2).await;
    let over = d.aim_landing(Landing::Past, None);
    d.fling(over, 1.4).await;
    d.idle(0.4).await;
    let win = d.win_aim();
    d.fling(win, 1.4).await;
    d.idle(2.0).await;

    // 5. Stage 3: reach the pearl in K hops (division).
    d.again_with_band(3).await;
    d.banner = "Stage 3: reach the pearl in K hops (the bubbles). Pick the hop size".into();
    d.idle(1.4).await;
    // Straight in, first try: the pearl plus the first-try bonus.
    let win = d.win_aim();
    d.fling(win, 1.5).await;
    d.idle(2.0).await;

    // 6. Stage 4: open-water estimation. A near miss, then a hit.
    d.again_with_band(6).await;
    d.banner = "Stage 4: no stones. Land near the number on an open 0-100 line".into();
    d.idle(1.4).await;
    let near = d.aim_landing(Landing::Past, Some(true));
    d.fling(near, 1.8).await;
    d.idle(0.4).await;
    let win = d.win_aim();
    d.fling(win, 1.8).await;
    d.idle(2.2).await;

    // 7. Leave, back to the reef.
    d.banner = "Leave is always one tap away".into();
    let leave = d.h.game.pearl_hop_layout(d.screen()).unwrap().leave().unwrap().center();
    d.tap(leave).await;
    d.show_cursor = false;
    d.banner = "Back on the reef".into();
    d.idle(2.5).await;

    println!("{} frames ({:.1}s) in {}", d.frame_no, d.frame_no as f32 / FPS, d.dir);
    println!("pearls: {}", d.h.game.pearls);
    if let Some(mut enc) = d.enc.take() {
        drop(enc.stdin.take()); // EOF: let ffmpeg finish the files
        let status = enc.wait().expect("ffmpeg ran");
        println!("ffmpeg: {status}; video at {}", d.mp4);
    }
    std::process::exit(0);
}
