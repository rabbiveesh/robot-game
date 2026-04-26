//! Truly headless integration test. Constructs a Game and drives it with
//! synthetic FrameInput. No macroquad window. No Window::from_config.
//! No `harness = false`. Just plain `cargo test`.
//!
//! If this runs green, step() is genuinely pure — no macroquad context needed.

use macroquad::prelude::KeyCode;
use robot_buddy_game::game::{Game, GameState};
use robot_buddy_game::input::FrameInput;

const SCREEN: (f32, f32) = (960.0, 720.0);
const DT: f32 = 1.0 / 60.0;

#[test]
fn empty_input_keeps_game_on_title() {
    let mut g = Game::new(42);
    let input = FrameInput::empty();
    for _ in 0..30 {
        g.step(&input, DT, SCREEN);
    }
    assert_eq!(g.state, GameState::Title, "no input → still on Title");
}

#[test]
fn key1_on_empty_slot_transitions_to_new_game() {
    let mut g = Game::new(42);

    // First frame: press Key1. Title sees an empty slot, fires NewGame(0).
    let press_1 = FrameInput::empty().with_key_pressed(KeyCode::Key1);
    g.step(&press_1, DT, SCREEN);

    assert_eq!(g.state, GameState::NewGame);
}
