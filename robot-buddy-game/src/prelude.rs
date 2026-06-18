//! Crate prelude. Use `crate::prelude::*` (or `robot_buddy_game::prelude::*`
//! from the binary) instead of `macroquad::prelude::*`.
//!
//! It re-exports macroquad's prelude, but overrides `draw_text` and
//! `measure_text` with the bundled-font versions from [`crate::text`] (an
//! explicit `pub use` shadows the glob re-export). That way every call site
//! renders through the bundled glyph font — covering `− × ÷`, `★`, and emoji —
//! without each file remembering to import the wrappers.
//!
//! `tests/font_coverage.rs` forbids `use macroquad::prelude::*` anywhere but
//! here, so the override can't be silently bypassed by a new file.

pub use macroquad::prelude::*;

// Shadows macroquad's draw_text/measure_text from the glob above.
pub use crate::text::{draw_text, measure_text};
