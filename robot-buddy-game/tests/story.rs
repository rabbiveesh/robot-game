//! Story-style integration tests. Each test reads top-to-bottom as a player flow,
//! using the harness in tests/common/mod.rs.
//!
//! Assertions lean on the GameEvent log via `h.mark()` / `h.events_since(mark)`.
//! Events describe *what happened*, which catches regressions that pure end-state
//! checks miss (e.g. "did this single-option NPC accidentally pop the menu first?").

mod common;

use common::Harness;
use robot_buddy_game::game::{GameEvent, GameState};

#[test]
fn new_game_form_takes_name_and_starts_intake() {
    let mut h = Harness::new(42);
    let mark = h.mark();
    h.start_new_game("Test");

    let events = h.events_since(mark);
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::StateChanged { to: GameState::Intake, .. })),
        "expected a StateChanged → Intake; got: {:?}", events,
    );
}

#[test]
fn full_intake_lands_in_playing_with_completed_profile() {
    let mut h = Harness::new(42);
    h.start_new_game("Test");

    let mark = h.mark();
    h.complete_intake_correctly();

    let events = h.events_since(mark);
    let band = events.iter().find_map(|e| match e {
        GameEvent::IntakeCompleted { math_band } => Some(*math_band),
        _ => None,
    }).expect(&format!("expected IntakeCompleted event; got: {:?}", events));
    assert!(band >= 1, "intake should produce a real math band, got {}", band);
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::StateChanged { to: GameState::Playing, .. })),
        "expected a StateChanged → Playing after intake; got: {:?}", events,
    );
    assert!(h.game.profile.intake_completed, "profile.intake_completed flag should be set");
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

    let mark = h.mark();
    h.interact();
    h.select_option("give");
    h.wait_until(|g| g.state == GameState::Dialogue);
    h.finish_dialogue();
    h.wait_until(|g| g.state == GameState::Playing);

    let events = h.events_since(mark);
    let gift = events.iter().find_map(|e| match e {
        GameEvent::GiftGiven { recipient_id, total } => Some((recipient_id.as_str(), *total)),
        _ => None,
    }).expect(&format!("expected GiftGiven event; got: {:?}", events));
    assert_eq!(gift, ("sparky", 1));
    assert_eq!(h.game.dum_dums, 19, "giving a dum_dum should cost 1");
}

#[test]
fn talking_to_sparky_can_roll_a_challenge_and_award_dum_dums() {
    use macroquad::prelude::KeyCode;

    // Seed 0 is hand-picked: on the dev map, talking to Sparky rolls a challenge
    // (50% chance per RNG draw at game.rs handle_interaction_menu "talk" branch).
    // If the random behavior changes, find a new seed via a scratch test.
    let mut h = Harness::new(0);
    h.start_dev_game();
    h.hold(KeyCode::Right); // turn to face Sparky on the adjacent tile

    h.interact();
    assert_eq!(h.game.state, GameState::InteractionMenu,
        "Space adjacent to Sparky should open the give/talk menu");

    let mark = h.mark();
    h.select_option("talk");
    h.finish_dialogue();                              // post-talk line(s)
    h.wait_until(|g| g.state == GameState::Challenge);
    h.answer_correctly();
    h.wait_until(|g| g.state == GameState::Playing);

    let events = h.events_since(mark);

    assert!(
        events.iter().any(|e| matches!(e, GameEvent::ChallengeStarted { .. })),
        "expected ChallengeStarted after talk; got: {:?}", events,
    );
    let resolved = events.iter().find_map(|e| match e {
        GameEvent::ChallengeResolved { correct, .. } => Some(*correct),
        _ => None,
    }).expect(&format!("expected ChallengeResolved; got: {:?}", events));
    assert!(resolved, "answered correctly, expected ChallengeResolved {{ correct: true }}");
    let award = events.iter().find_map(|e| match e {
        GameEvent::DumDumsAwarded { amount } => Some(*amount),
        _ => None,
    }).expect(&format!("correct answer should award dum_dums; got: {:?}", events));
    assert!(award > 0, "reward amount should be positive");
    assert_eq!(h.game.dum_dums, 20 + award,
        "post-challenge dum_dums should equal starting balance + reward");
}

#[test]
fn walk_to_npc_then_interact_starts_dialogue() {
    let mut h = Harness::new(42);
    h.start_dev_game();

    // The sage is the second sprite in the dev gallery row. walk_to_npc
    // pathfinds to an adjacent walkable tile and turns to face them.
    h.walk_to_npc("sage");

    let mark = h.mark();
    h.interact();

    // Sage in the dev map has only the "talk" option. Stronger than asserting
    // state==Dialogue: assert the menu was *skipped* — no transition through
    // InteractionMenu — and DialogueStarted fired.
    let events = h.events_since(mark);
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::DialogueStarted { .. })),
        "expected DialogueStarted; got: {:?}", events,
    );
    assert!(
        !events.iter().any(|e| matches!(e,
            GameEvent::StateChanged { to: GameState::InteractionMenu, .. })),
        "single-option NPC should skip the menu, but a transition into InteractionMenu was logged: {:?}",
        events,
    );
}
