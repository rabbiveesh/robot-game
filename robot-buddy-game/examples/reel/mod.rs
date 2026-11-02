//! The playback half of a demo video, shared by the demo examples: a list of
//! recorded frames (the screen's draw calls and the panels' views) is shown in
//! a Bevy window one frame at a time, each saved as a PNG, and `ffmpeg`
//! turns the PNGs into an MP4.
//!
//! Recording is the demo's own business: step the real `Game`, call
//! `Frame::capture` after drawing anything extra on top.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::WindowResolution;
use robot_buddy_game::app::{DrawList, DrawPlugin};
use robot_buddy_game::draw::{self, Cmd};
use robot_buddy_game::game::Game;
use robot_buddy_game::ui::kit::ArtSizes;
use robot_buddy_game::ui::views::UiViews;

/// One recorded frame: the clock, the screen's draw calls and the panels' views.
pub struct Frame {
    pub time: f64,
    pub cmds: Vec<Cmd>,
    pub views: UiViews,
}

impl Frame {
    /// Start recording a frame: call before `game.render`.
    pub fn begin(screen: (f32, f32), time: f64) {
        draw::begin_frame(screen, time);
    }

    /// Everything drawn since `begin`, plus the game's panels.
    pub fn capture(game: &Game, screen: (f32, f32), time: f64) -> Frame {
        Frame { time, cmds: draw::take_commands(), views: game.ui_views(screen) }
    }
}

/// Where the video goes and how it plays.
pub struct Output {
    /// Window title while it records.
    pub title: String,
    /// Directory for the numbered PNGs (emptied of old PNGs first).
    pub dir: String,
    /// The MP4 to write.
    pub mp4: String,
    pub fps: f32,
    /// Logical screen size the frames were recorded at.
    pub screen: (f32, f32),
}

impl Output {
    /// `DEMO_DIR` / `DEMO_MP4` override the defaults.
    pub fn from_env(title: &str, default_dir: &str, default_mp4: &str, fps: f32, screen: (f32, f32)) -> Output {
        let dir = std::env::var("DEMO_DIR").unwrap_or_else(|_| default_dir.into());
        std::fs::create_dir_all(&dir).unwrap();
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            if entry.path().extension().is_some_and(|e| e == "png") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        Output {
            title: title.into(),
            dir,
            mp4: std::env::var("DEMO_MP4").unwrap_or_else(|_| default_mp4.into()),
            fps,
            screen,
        }
    }
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
    out: Output,
}

/// Show the frames, screenshot each, encode, exit.
pub fn play(frames: Vec<Frame>, out: Output) {
    let (w, h) = out.screen;
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: out.title.clone(),
                resolution: WindowResolution::new(w as u32, h as u32).with_scale_factor_override(1.0),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DrawPlugin)
        .insert_resource(Recording { frames, out })
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
            encode(&rec.out);
            exit.write(AppExit::Success);
        }
        return;
    };
    *views = f.views.clone();
    draw::begin_frame(rec.out.screen, f.time);
    views.draw_all_art(&art.0);
    list.0 = f.cmds.iter().cloned().chain(draw::take_commands()).collect();
    if t % PER_FRAME == SETTLE {
        let path = format!("{}/{:05}.png", rec.out.dir, i + 1);
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
}

/// The numbered PNGs into an H.264 MP4 (yuv420p, even size). `DEMO_FFMPEG`
/// picks the ffmpeg (default `ffmpeg` on PATH).
fn encode(out: &Output) {
    let ffmpeg = std::env::var("DEMO_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let status = std::process::Command::new(ffmpeg)
        .args(["-y", "-loglevel", "error", "-framerate", &format!("{}", out.fps), "-i", &format!("{}/%05d.png", out.dir)])
        .args(["-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2", "-c:v", "libx264", "-preset", "medium"])
        .args(["-crf", "18", "-pix_fmt", "yuv420p", "-movflags", "+faststart", &out.mp4])
        .status();
    match status {
        Ok(s) => println!("ffmpeg: {s}; video at {}", out.mp4),
        Err(e) => eprintln!("ffmpeg didn't run ({e}); the frames are in {}", out.dir),
    }
}
