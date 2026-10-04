//! Robot Land demo reel: drives the real game through a scripted tour —
//! Gizmo's lab teleporter, then a stroll past every robot — and pipes each
//! rendered frame straight into ffmpeg.
//!
//!   DEMO_OUT=/tmp/robot-land.mp4 SHOT_W=1280 SHOT_H=960 \
//!     cargo run --release -p robot-buddy-game --example robot_land_demo
//!
//! Logic steps at the harness's fixed 60 fps, so the video is the same every
//! run. SHOT_W/SHOT_H are PHYSICAL pixels (double them on a 2× display).

#[path = "../tests/common/mod.rs"]
#[allow(dead_code)]
mod common;

use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::process::{Child, Command, Stdio};

use common::{Harness, SCREEN};
use macroquad::prelude::*;
use robot_buddy_game::game::GameState;
use robot_buddy_game::input::FrameInput;
use robot_buddy_game::npc::{self as npc_mod, NpcKind};
use robot_buddy_game::tilemap::Map;

const DT: f32 = 1.0 / 60.0;

fn env_f(key: &str, default: f32) -> f32 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn conf() -> Conf {
    Conf {
        window_title: "Robot Land demo".into(),
        window_width: env_f("SHOT_W", 960.0) as i32,
        window_height: env_f("SHOT_H", 720.0) as i32,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

/// Owns the harness and the ffmpeg pipe. Every `frame` steps the game once,
/// renders it, draws any title card on top, and ships the pixels.
struct Reel {
    h: Harness,
    ffmpeg: Option<Child>,
    size: (u32, u32),
    card: Option<(String, String, f32)>, // (title, subtitle, seconds shown so far)
    frames: u32,
}

impl Reel {
    async fn frame(&mut self, input: &FrameInput) {
        self.h.game.step(input, DT, SCREEN);
        clear_background(BLACK);
        let screen = (screen_width(), screen_height());
        self.h.game.render(screen, input);
        self.draw_card(screen);
        let img = get_screen_data();
        let (w, h) = (img.width as u32, img.height as u32);
        if self.ffmpeg.is_none() {
            self.size = (w, h);
            self.ffmpeg = Some(spawn_ffmpeg(w, h));
        }
        if (w, h) == self.size {
            let stdin = self.ffmpeg.as_mut().unwrap().stdin.as_mut().unwrap();
            stdin.write_all(&img.bytes).expect("ffmpeg pipe closed");
        }
        self.frames += 1;
        next_frame().await;
    }

    async fn idle(&mut self, seconds: f32) {
        for _ in 0..(seconds * 60.0) as u32 {
            self.frame(&FrameInput::empty()).await;
        }
    }

    async fn press(&mut self, key: KeyCode) {
        self.frame(&FrameInput::empty().with_key_pressed(key)).await;
    }

    async fn hold(&mut self, key: KeyCode) {
        self.frame(&FrameInput::empty().with_key_down(key)).await;
    }

    fn draw_card(&mut self, (sw, sh): (f32, f32)) {
        let Some((title, sub, t)) = self.card.as_mut() else { return };
        *t += DT;
        let alpha = (*t / 0.4).min(1.0);
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.05, 0.04, 0.12, 0.82 * alpha));
        let bounce = ((*t * 3.0).sin() * 6.0).abs();
        let size = (sw / 9.0).min(110.0);
        let tw = measure_text(title, None, size as u16, 1.0).width;
        let ty = sh * 0.45 - bounce;
        draw_text(title, (sw - tw) / 2.0 + 4.0, ty + 4.0, size, Color::from_rgba(120, 60, 200, (255.0 * alpha) as u8));
        draw_text(title, (sw - tw) / 2.0, ty, size, Color::new(1.0, 0.84, 0.25, alpha));
        let ss = size * 0.38;
        let sw2 = measure_text(sub, None, ss as u16, 1.0).width;
        draw_text(sub, (sw - sw2) / 2.0, ty + size * 0.75, ss, Color::new(0.85, 0.95, 1.0, alpha));
    }

    async fn card(&mut self, title: &str, sub: &str, seconds: f32) {
        self.card = Some((title.into(), sub.into(), 0.0));
        self.idle(seconds).await;
        self.card = None;
    }

    /// Play a scripted conversation, letting each line type out and then sit
    /// on screen long enough for a kid to hear it read aloud.
    async fn talk(&mut self, lines: &[(&str, &str)]) {
        self.h.game.say(lines);
        for (_, text) in lines {
            let typing = text.chars().count() as f32 * 0.03;
            let reading = 1.4 + text.chars().count() as f32 * 0.035;
            self.idle(typing + reading).await;
            self.press(KeyCode::Space).await;
        }
        while self.h.game.is_dialogue_active() {
            self.press(KeyCode::Space).await;
        }
        while self.h.game.state != GameState::Playing {
            self.frame(&FrameInput::empty()).await;
        }
    }

    /// Walk one tile at a time toward `goal`, replanning every step so
    /// wandering robots never leave us stuck.
    async fn walk_to(&mut self, goal: (usize, usize)) {
        for _ in 0..400 {
            let at = (self.h.game.player.tile_x, self.h.game.player.tile_y);
            if at == goal && self.h.game.player_at_rest() { return; }
            let Some(next) = next_step(&self.h, at, goal) else {
                self.idle(0.1).await; // someone's in the way; let them pass
                continue;
            };
            let key = dir_key(at, next);
            for _ in 0..30 {
                if (self.h.game.player.tile_x, self.h.game.player.tile_y) == next && self.h.game.player_at_rest() { break; }
                self.hold(key).await;
            }
        }
    }

    /// Walk up beside a robot (it holds still for its close-up) and face it.
    async fn visit(&mut self, kind: NpcKind, from: (i32, i32)) {
        let npc = self.h.game.npcs.iter_mut().find(|n| n.kind == kind).expect("robot on map");
        npc.wanders = false;
        let (nx, ny) = (npc.entity.tile_x, npc.entity.tile_y);
        let spot = ((nx as i32 + from.0) as usize, (ny as i32 + from.1) as usize);
        self.walk_to(spot).await;
        self.hold(dir_key(spot, (nx, ny))).await;
        self.idle(0.3).await;
    }
}

fn dir_key(from: (usize, usize), to: (usize, usize)) -> KeyCode {
    match (to.0 as i32 - from.0 as i32, to.1 as i32 - from.1 as i32) {
        (0, -1) => KeyCode::Up,
        (0, 1) => KeyCode::Down,
        (-1, 0) => KeyCode::Left,
        _ => KeyCode::Right,
    }
}

fn next_step(h: &Harness, start: (usize, usize), goal: (usize, usize)) -> Option<(usize, usize)> {
    let g = &h.game;
    let blocked = |p: (usize, usize)| {
        g.map.is_solid(p.0, p.1)
            || g.npcs.iter().any(|n| (n.entity.tile_x, n.entity.tile_y) == p)
    };
    let mut prev = HashMap::from([(start, start)]);
    let mut q = VecDeque::from([start]);
    while let Some(p) = q.pop_front() {
        if p == goal {
            let mut cur = goal;
            while prev[&cur] != start { cur = prev[&cur]; }
            return Some(cur);
        }
        for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
            let (nx, ny) = (p.0 as i32 + dx, p.1 as i32 + dy);
            if nx < 0 || ny < 0 { continue; }
            let n = (nx as usize, ny as usize);
            if prev.contains_key(&n) || (blocked(n) && n != goal) { continue; }
            prev.insert(n, p);
            q.push_back(n);
        }
    }
    None
}

fn spawn_ffmpeg(w: u32, h: u32) -> Child {
    let out = std::env::var("DEMO_OUT").unwrap_or_else(|_| "robot-land.mp4".into());
    Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error",
            "-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{w}x{h}"), "-r", "60",
            "-i", "-",
            // macroquad's screen grab is bottom-up; flip, and land on a
            // friendly 1280-wide 30 fps H.264 any phone will play.
            "-vf", "vflip,scale=1280:-2,fps=30",
            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "20", "-movflags", "+faststart",
            &out])
        .stdin(Stdio::piped())
        .spawn()
        .expect("ffmpeg on PATH")
}

#[macroquad::main(conf)]
async fn main() {
    let mut h = Harness::new(42);
    h.start_dev_game();
    h.game.dum_dums = 20;

    // Open in Gizmo's lab, a few steps from the new teleporter pad.
    h.game.map = Map::lab();
    h.game.npcs = npc_mod::npcs_for_map("lab");
    h.game.npcs_offstage.clear();
    h.game.companion = None;
    h.warp_to(4, 4);
    let s = &mut h.game.sparky.entity;
    (s.tile_x, s.tile_y) = (3, 4);
    (s.x, s.y) = (3.0 * 48.0, 4.0 * 48.0);
    (s.target_x, s.target_y) = (s.x, s.y);
    let mut reel = Reel { h, ffmpeg: None, size: (0, 0), card: None, frames: 0 };
    let sparky = reel.h.game.current_buddy_name();
    let sparky = sparky.as_str();

    reel.card("ROBOT LAND", "a brand new place in Robot Buddy Adventure!", 3.0).await;
    reel.idle(0.4).await;
    reel.talk(&[
        ("Professor Gizmo", "Behold! My Robo-Teleporter! It beams you straight to ROBOT LAND!"),
        ("Professor Gizmo", "I am... sixty percent sure it works. Off you go!"),
        (sparky, "Sixty?! ...okay, let's GO!"),
    ]).await;

    // Over to the gear pad in the corner and step on.
    reel.walk_to((1, 3)).await;
    for _ in 0..90 {
        if reel.h.game.map.id == "robot_land" { break; }
        reel.hold(KeyCode::Up).await;
    }
    while reel.h.game.is_dialogue_active() { reel.press(KeyCode::Space).await; }
    reel.idle(1.6).await;
    reel.talk(&[(sparky, "WHOA. Everything here is a ROBOT! Even the wall is spinning!")]).await;

    // Toasty, who cannot stop making toast.
    reel.visit(NpcKind::Toaster, (-1, 0)).await;
    reel.idle(1.0).await;
    reel.talk(&[
        ("Toasty", "POP! Oh no. I toasted my homework again."),
        (sparky, "Is that... toast on the ceiling?"),
        ("Toasty", "Want some? Too late. It went to space."),
    ]).await;
    reel.idle(1.2).await;

    // Beep.
    reel.visit(NpcKind::TinyBot, (-1, 0)).await;
    reel.talk(&[
        ("Beep", "Beep."),
        ("Beep", "Beep beep? BEEEEP! ...boop."),
        (sparky, "I understood that completely."),
    ]).await;

    // Clank, across the conveyor belt.
    reel.visit(NpcKind::SpringBot, (0, 1)).await;
    reel.idle(1.4).await;
    reel.talk(&[
        ("Clank", "Hi! I'm Clank! My head is on a spring. BOING! It's a feature!"),
        ("Clank", "I tried to nod yes and my head flew away. So... yes!"),
    ]).await;
    reel.idle(1.0).await;

    // His Majesty.
    reel.visit(NpcKind::Roomba, (0, 1)).await;
    reel.talk(&[
        ("Sir Vacuums-a-Lot", "Halt! I am Sir Vacuums-a-Lot!"),
        ("Sir Vacuums-a-Lot", "I have cleaned this exact spot four hundred times. It is SPOTLESS."),
        ("Sir Vacuums-a-Lot", "Excuse me. You are standing on a crumb. A ROYAL crumb."),
    ]).await;

    // Sockbot, with the math in it.
    reel.visit(NpcKind::SockBot, (-1, 0)).await;
    reel.idle(0.8).await;
    reel.talk(&[
        ("Sockbot", "Socks come in PAIRS! I have nine socks."),
        ("Sockbot", "That's four pairs... and one very lonely sock."),
        (sparky, "Two, four, six, eight... it's true! One sock left over!"),
        ("Sockbot", "Have you seen a stripy sock? He's my favourite. He's always missing."),
    ]).await;

    // Grandpa Gearbox gets the last word.
    reel.visit(NpcKind::RustyBot, (1, 0)).await;
    reel.idle(1.2).await;
    reel.talk(&[
        ("Grandpa Gearbox", "*squeak* Back in my day we only had ONE number."),
        ("Grandpa Gearbox", "It was zero. We LOVED it."),
        ("Grandpa Gearbox", "And I didn't run on batteries. I ran on a hamster named Gerald."),
        (sparky, "...I love it here."),
    ]).await;
    reel.idle(1.5).await;

    reel.card("ROBOT LAND", "beep beep! coming soon!", 3.5).await;

    let mut child = reel.ffmpeg.take().expect("rendered at least one frame");
    drop(child.stdin.take());
    let status = child.wait().expect("ffmpeg finished");
    println!("wrote {} frames ({:.1}s) — ffmpeg {status}", reel.frames, reel.frames as f32 / 60.0);
    std::process::exit(if status.success() { 0 } else { 1 });
}
