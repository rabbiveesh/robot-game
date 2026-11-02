//! Opt-in input trace, for debugging how input *feels* on a real device.
//!
//! Run a native build with `ROBOT_TRACE=1` and every pointer press, drag and
//! release (the `FrameInput` each frame turned into, with the panel button it
//! hit) and each drag decision are written to stderr, one line apiece,
//! prefixed with the frame number:
//!
//! ```text
//! ROBOT_TRACE=1 ./target/release/robot-buddy-game 2> input.log
//! ```
//!
//! The browser has no env vars, so a web build bakes the switch in at compile
//! time instead — `ROBOT_TRACE=1 ./build-wasm.sh` — and the lines go to the
//! browser console. Off by default; when off, every call is one relaxed load.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

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

/// Called once per frame, as input is captured.
pub fn next_frame() {
    FRAME.fetch_add(1, Ordering::Relaxed);
}

pub fn line(args: std::fmt::Arguments) {
    if enabled() {
        let frame = FRAME.load(Ordering::Relaxed);
        #[cfg(target_arch = "wasm32")]
        bevy::log::info!("[{:>6}] {}", frame, args);
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
