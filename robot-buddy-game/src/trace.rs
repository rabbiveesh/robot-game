//! Opt-in input trace, for debugging how input *feels* on a real device.
//!
//! Run a native build with `ROBOT_TRACE=1` and every raw miniquad input event,
//! the `FrameInput` each frame turned into, and each drag decision are written
//! to stderr, one line apiece, prefixed with the frame number:
//!
//! ```text
//! ROBOT_TRACE=1 ./target/release/robot-buddy-game 2> input.log
//! ```
//!
//! The browser has no env vars, so a web build bakes the switch in at compile
//! time instead — `ROBOT_TRACE=1 ./build-wasm.sh` — and the lines go to the
//! browser console. Off by default; when off, every call is one relaxed load.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use macroquad::miniquad::{EventHandler, MouseButton, TouchPhase};

static ON: AtomicBool = AtomicBool::new(false);
static FRAME: AtomicU64 = AtomicU64::new(0);

/// Read `ROBOT_TRACE` once at startup (native: at run time; web: whatever it
/// was when the wasm was built).
pub fn init() {
    let flag = |v: &str| !v.is_empty() && v != "0";
    let on = std::env::var("ROBOT_TRACE").map_or(false, |v| flag(&v))
        || option_env!("ROBOT_TRACE").map_or(false, flag);
    ON.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Called once per frame by the main loop.
pub fn next_frame() {
    FRAME.fetch_add(1, Ordering::Relaxed);
}

pub fn line(args: std::fmt::Arguments) {
    if enabled() {
        let frame = FRAME.load(Ordering::Relaxed);
        #[cfg(target_arch = "wasm32")]
        macroquad::miniquad::info!("[{:>6}] {}", frame, args);
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!("[{:>6}] {}", frame, args);
    }
}

/// `trace!("grab {group}")` — formats only when tracing is on.
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        if $crate::trace::enabled() {
            $crate::trace::line(format_args!($($arg)*));
        }
    };
}

/// Replays macroquad's queue of raw miniquad input events into the trace, so
/// we see exactly what the OS delivered — before macroquad folds it into
/// "is the button down".
pub struct RawInputLog;

impl EventHandler for RawInputLog {
    fn update(&mut self) {}
    fn draw(&mut self) {}

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        line(format_args!("raw  move      ({x:.0},{y:.0})"));
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        line(format_args!("raw  down {button:?} ({x:.0},{y:.0})"));
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, x: f32, y: f32) {
        line(format_args!("raw  up   {button:?} ({x:.0},{y:.0})"));
    }

    fn touch_event(&mut self, phase: TouchPhase, id: u64, x: f32, y: f32) {
        line(format_args!("raw  touch#{id} {phase:?} ({x:.0},{y:.0})"));
    }
}
