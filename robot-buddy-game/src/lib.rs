//! Robot Buddy game library. `src/main.rs` is a thin macroquad shim that drives
//! `Game::step` + `Game::render` each frame; tests in `tests/` exercise `Game`
//! directly with no window.

pub mod tilemap;
pub mod sprites;
pub mod npc;
pub mod follower;
pub mod ui;
pub mod save;
pub mod audio;
pub mod session;
pub mod settings;
pub mod input;
pub mod text;
pub mod prelude;
pub mod game;
