//! Crate prelude: the drawing vocabulary every render function uses
//! (`draw_rectangle`, `Color`, `KeyCode`, ...). Use `crate::prelude::*`.
//!
//! `tests/font_coverage.rs` forbids glob-importing a drawing module anywhere
//! but here, so text can't silently bypass the bundled font.

pub use crate::draw::*;
pub use crate::input::KeyCode;
