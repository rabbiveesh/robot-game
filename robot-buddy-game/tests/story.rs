//! Story-style integration tests. Each test reads top-to-bottom as a player flow,
//! using the harness in tests/common/mod.rs.

mod common;

use common::Harness;
use robot_buddy_game::game::GameState;

#[test]
fn new_game_form_takes_name_and_starts_intake() {
    let mut h = Harness::new(42);
    h.start_new_game("Test");
    // Sparky's intake intro dialogue is now active — the player would press Space
    // to advance through it. Just confirm we reached Intake with dialogue running.
    assert_eq!(h.game.state, GameState::Intake);
}

#[test]
fn full_intake_lands_in_playing_with_completed_profile() {
    let mut h = Harness::new(42);
    h.start_new_game("Test");
    h.complete_intake_correctly();

    assert_eq!(h.game.state, GameState::Playing);
    assert!(h.game.profile.intake_completed, "profile should be flagged intake_completed");
}

#[test]
fn give_to_sparky_records_gift_and_decrements_dum_dums() {
    use macroquad::prelude::KeyCode;

    let mut h = Harness::new(42);
    h.start_dev_game();

    // Player spawns at (7, 10) facing Up; Sparky is at (8, 10) — already adjacent.
    // One frame of holding Right turns the player to face him without moving
    // (Sparky blocks for the first 0.12s of pushing).
    h.hold(KeyCode::Right);

    h.interact();
    assert_eq!(h.game.state, GameState::InteractionMenu,
        "Space adjacent to Sparky should open the interaction menu");

    h.select_option("give");

    // Sparky's reaction dialogue plays. Advance through it.
    h.wait_until(|g| g.state == GameState::Dialogue);
    h.finish_dialogue();
    h.wait_until(|g| g.state == GameState::Playing);

    assert_eq!(h.game.gifts_given.get("sparky").copied().unwrap_or(0), 1);
    assert_eq!(h.game.dum_dums, 19);
}

#[test]
fn walk_to_npc_then_interact_starts_dialogue() {
    let mut h = Harness::new(42);
    h.start_dev_game();

    // The sage is the second sprite in the dev gallery row. walk_to_npc
    // pathfinds to an adjacent walkable tile and turns to face them.
    h.walk_to_npc("sage");
    h.interact();

    // Sage in the dev map has only the "talk" option (no gift, no shop), so
    // the menu is skipped and dialogue starts immediately.
    assert_eq!(h.game.state, GameState::Dialogue,
        "single-option NPCs should auto-talk without showing the menu");
}
