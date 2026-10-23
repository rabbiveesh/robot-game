//! Robot Buddy game library. `src/main.rs` starts a Bevy app ([`app`]) that
//! drives `Game::step` + `Game::render` each frame; tests in `tests/` exercise
//! `Game` directly with no window.

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
pub mod pathfinding;
pub mod number_track;
pub mod text;
pub mod draw;
pub mod app;
#[cfg(target_arch = "wasm32")]
pub mod web;
pub mod prelude;
pub mod game;
