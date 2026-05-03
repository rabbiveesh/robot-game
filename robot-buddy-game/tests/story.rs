//! Story-style integration tests. Each test reads top-to-bottom as a player flow,
//! using the harness in tests/common/mod.rs.
//!
//! Assertions lean on the GameEvent log via `h.mark()` / `h.events_since(mark)`.
//! Events describe *what happened*, which catches regressions that pure end-state
//! checks miss (e.g. "did this single-option NPC accidentally pop the menu first?").

mod common;

use common::Harness;
use robot_buddy_game::game::{GameEvent, GameState};
use robot_buddy_game::npc::NpcKind;

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
fn sage_offers_kenken_and_solving_it_completes_the_session() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to_npc(NpcKind::Sage);

    h.interact();
    assert_eq!(h.game.state, GameState::InteractionMenu,
        "puzzler NPC should open the menu (Talk + Try a Puzzle)");

    let mark = h.mark();
    h.select_option("puzzle");
    h.wait_until(|g| g.state == GameState::KenKen);

    let started_grid = {
        let ak = h.game.active_kenken().expect("active KenKen after picking 'puzzle'");
        ak.session.puzzle.grid_size
    };
    assert!(started_grid >= 2 && started_grid <= 4,
        "kenken_level should clamp to 2..=4, got {}", started_grid);

    h.solve_kenken_correctly();

    let events = h.events_since(mark);
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::KenKenStarted { .. })),
        "expected KenKenStarted; got: {:?}", events,
    );
    let resolved = events.iter().find_map(|e| match e {
        GameEvent::KenKenResolved { correct, hints_used, grid_size, .. } =>
            Some((*correct, *hints_used, *grid_size)),
        _ => None,
    }).expect(&format!("expected KenKenResolved; got: {:?}", events));
    assert_eq!(resolved, (true, 0, started_grid),
        "fully solving the puzzle by hand should resolve correct=true with 0 hints");
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::DumDumsAwarded { .. })),
        "solving a kenken should award dum_dums; got: {:?}", events,
    );
    assert_eq!(h.game.state, GameState::Playing);
}

#[test]
fn kenken_intro_shows_on_first_puzzle_only() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    assert!(!h.game.profile.kenken_intro_seen,
        "fresh dev profile should not have seen the intro");

    h.walk_to_npc(NpcKind::Sage);
    h.interact();
    h.select_option("puzzle");
    h.wait_until(|g| g.state == GameState::KenKen);

    let step = h.game.active_kenken().unwrap().intro_step;
    assert_eq!(step, Some(0), "first KenKen should start at intro step 0");

    h.skip_kenken_intro();
    assert!(h.game.profile.kenken_intro_seen,
        "finishing the intro should flip the profile flag");
    assert_eq!(h.game.active_kenken().unwrap().intro_step, None);

    h.solve_kenken_correctly();

    // Second KenKen — no intro this time.
    h.walk_to_npc(NpcKind::Sage);
    h.interact();
    h.select_option("puzzle");
    h.wait_until(|g| g.state == GameState::KenKen);
    assert_eq!(h.game.active_kenken().unwrap().intro_step, None,
        "second KenKen should skip the intro");
}

#[test]
fn kenken_hint_marks_resolution_as_hint_used() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to_npc(NpcKind::Sage);
    h.interact();
    h.select_option("puzzle");
    h.wait_until(|g| g.state == GameState::KenKen);

    // One hint, then solve the rest.
    h.request_kenken_hint();
    let mark = h.mark();
    h.solve_kenken_correctly();

    let events = h.events_since(mark);
    let hints_used = events.iter().find_map(|e| match e {
        GameEvent::KenKenResolved { hints_used, .. } => Some(*hints_used),
        _ => None,
    }).expect(&format!("expected KenKenResolved; got: {:?}", events));
    assert_eq!(hints_used, 1, "one hint button click should record one hint");
}

#[test]
fn control_room_band_knob_cycles_math_band() {
    use macroquad::prelude::KeyCode;
    let mut h = Harness::new(7);
    h.start_dev_game();

    // Door tile is at (1, 9). Walk to the tile next to it, then step onto it.
    h.walk_to(2, 9);
    let mark = h.mark();
    h.step_through_portal(KeyCode::Left, "control");
    let events = h.events_since(mark);
    assert!(
        events.iter().any(|e| matches!(e,
            GameEvent::MapTransitioned { to, .. } if to == "control")),
        "expected MapTransitioned to 'control'; got: {:?}", events,
    );

    let before = h.game.profile.math_band;
    h.walk_to_npc(NpcKind::CtrlBand);
    h.interact();
    h.wait_until(|g| g.state == GameState::Dialogue);
    h.finish_dialogue();
    h.wait_until(|g| g.state == GameState::Playing);
    let after = h.game.profile.math_band;
    let expected = if before >= 10 { 1 } else { before + 1 };
    assert_eq!(after, expected,
        "ctrl_band should cycle math_band: {} → {}", before, after);
}

#[test]
fn control_room_intro_reset_replays_kenken_intro() {
    let mut h = Harness::new(7);
    h.start_dev_game();

    // Mark intro as already-seen by walking through it once.
    h.walk_to_npc(NpcKind::Sage);
    h.interact();
    h.select_option("puzzle");
    h.wait_until(|g| g.state == GameState::KenKen);
    h.skip_kenken_intro();
    assert!(h.game.profile.kenken_intro_seen);
    h.solve_kenken_correctly();

    // Walk to control room and reset the intro flag.
    h.walk_to(2, 9);
    h.step_through_portal(macroquad::prelude::KeyCode::Left, "control");
    h.walk_to_npc(NpcKind::CtrlIntroReset);
    h.interact();
    h.wait_until(|g| g.state == GameState::Dialogue);
    h.finish_dialogue();
    h.wait_until(|g| g.state == GameState::Playing);

    assert!(!h.game.profile.kenken_intro_seen,
        "ctrl_intro_reset should clear the intro flag");
}

#[test]
fn wandering_npc_walking_onto_portal_transfers_to_destination_map() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;

    // The home map has Kid1 (a wanderer) and a portal at (4,6) that leads
    // back to the overworld. Driving Kid1 onto that tile should pull them out
    // of `npcs` and stash them under `npcs_offstage["overworld"]` so the next
    // visit to the overworld brings the kid along.
    let mut h = Harness::new(1);
    h.start_dev_game();

    // Warp to the home map directly. Going dev → overworld → home would
    // require completing intake and walking many screens; nothing about the
    // portal-transfer logic depends on how we got there, so cheat the
    // setup. We re-spawn home's default NPC roster, clear any leftover
    // offstage state from the dev game, and put the player well clear of
    // the door tile so nothing fires the player-portal handler mid-test.
    h.game.map = Map::home();
    h.game.npcs = npc_mod::npcs_for_map("home");
    h.game.npcs_offstage.clear();
    h.game.player.tile_x = 5;
    h.game.player.tile_y = 3;
    h.game.player.x = 5.0 * 48.0;
    h.game.player.y = 3.0 * 48.0;
    h.game.player.target_x = h.game.player.x;
    h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;
    // Park Sparky on a benign tile so his follow logic doesn't bumble around.
    h.game.sparky.entity.tile_x = 6;
    h.game.sparky.entity.tile_y = 3;
    h.game.sparky.entity.x = 6.0 * 48.0;
    h.game.sparky.entity.y = 3.0 * 48.0;
    h.game.sparky.entity.target_x = h.game.sparky.entity.x;
    h.game.sparky.entity.target_y = h.game.sparky.entity.y;
    h.game.sparky.entity.moving = false;

    // Place Kid1 on the tile next to the door and start the slide onto the
    // portal. The animation runs each frame; once Kid1's pixels reach the
    // tile, `handle_npc_portals` should pick them up via the just-arrived
    // signal and teleport them.
    let kid_idx = h.game.npcs.iter()
        .position(|n| n.kind == NpcKind::Kid1)
        .expect("home should spawn a Kid1");
    {
        let n = &mut h.game.npcs[kid_idx];
        n.entity.tile_x = 4;
        n.entity.tile_y = 5;
        n.entity.x = 4.0 * 48.0;
        n.entity.y = 5.0 * 48.0;
        n.entity.target_x = n.entity.x;
        n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.entity.start_move(4, 6); // step onto the (4,6) home → overworld portal
    }

    // 30 frames covers a full tile slide (~14 frames) plus the arrival frame.
    h.advance(30);

    assert!(
        !h.game.npcs.iter().any(|n| n.kind == NpcKind::Kid1),
        "Kid1 should be gone from the home roster after the portal transfer; current: {:?}",
        h.game.npcs.iter().map(|n| n.kind).collect::<Vec<_>>(),
    );
    let stash = h.game.npcs_offstage.get("overworld")
        .expect("expected an overworld stash after Kid1's transfer");
    assert!(
        stash.iter().any(|n| n.kind == NpcKind::Kid1),
        "Kid1 should now live in npcs_offstage['overworld']; got kinds: {:?}",
        stash.iter().map(|n| n.kind).collect::<Vec<_>>(),
    );
    // After teleport the kid's home tether re-anchors at the destination so
    // they hover near the portal exit instead of trying to drift back across
    // the map. The overworld home portal lands the player at (5,8); Kid1's
    // landing tile may be displaced if blocked, but should be near (5,8).
    let landed = stash.iter().find(|n| n.kind == NpcKind::Kid1).unwrap();
    assert_eq!((landed.home_tx, landed.home_ty),
        (landed.entity.tile_x, landed.entity.tile_y),
        "transfer should re-anchor home tether to the new location");
}

#[test]
fn player_portaling_onto_offstage_npc_displaces_them() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;
    use macroquad::prelude::KeyCode;

    // Realistic block-prevention scenario: a wanderer walks through a portal
    // and lingers on its arrival tile. Later the player follows through that
    // same portal — without intervention they'd land on the kid's head.
    // `handle_portal` calls `displace_npcs_at` on the destination tile to
    // bounce the kid to a nearby free tile.
    let mut h = Harness::new(1);
    h.start_dev_game();

    // Set up: player on home, ready to step through (4,6) → overworld (5,8).
    // The overworld stash gets pre-seeded with Kid1 squatting on (5,8).
    h.game.map = Map::home();
    h.game.npcs = npc_mod::npcs_for_map("home");
    h.game.npcs_offstage.clear();
    let mut over_npcs = npc_mod::npcs_for_map("overworld");
    let mut kid1 = h.game.npcs.iter()
        .find(|n| n.kind == NpcKind::Kid1)
        .expect("home spawns a Kid1 we can clone")
        .clone();
    kid1.entity.tile_x = 5;
    kid1.entity.tile_y = 8;
    kid1.entity.x = 5.0 * 48.0;
    kid1.entity.y = 8.0 * 48.0;
    kid1.entity.target_x = kid1.entity.x;
    kid1.entity.target_y = kid1.entity.y;
    kid1.entity.moving = false;
    kid1.home_tx = 5;
    kid1.home_ty = 8;
    over_npcs.push(kid1);
    h.game.npcs_offstage.insert("overworld".into(), over_npcs);

    // Pull Kid1 out of the home roster so we don't double-count.
    h.game.npcs.retain(|n| n.kind != NpcKind::Kid1);

    // Park player just above the door, Sparky out of the way.
    h.game.player.tile_x = 4;
    h.game.player.tile_y = 5;
    h.game.player.x = 4.0 * 48.0;
    h.game.player.y = 5.0 * 48.0;
    h.game.player.target_x = h.game.player.x;
    h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;
    h.game.sparky.entity.tile_x = 5;
    h.game.sparky.entity.tile_y = 5;
    h.game.sparky.entity.x = 5.0 * 48.0;
    h.game.sparky.entity.y = 5.0 * 48.0;
    h.game.sparky.entity.target_x = h.game.sparky.entity.x;
    h.game.sparky.entity.target_y = h.game.sparky.entity.y;
    h.game.sparky.entity.moving = false;

    // Step through the door. Portal swaps maps, displaces any NPC on (5,8).
    h.step_through_portal(KeyCode::Down, "overworld");

    assert_eq!((h.game.player.tile_x, h.game.player.tile_y), (5, 8),
        "player should land at the overworld portal exit");
    let kid = h.game.npcs.iter().find(|n| n.kind == NpcKind::Kid1)
        .expect("Kid1 should now be on overworld (came along via the stash)");
    assert_ne!(
        (kid.entity.tile_x, kid.entity.tile_y), (5, 8),
        "displace_npcs_at should bounce Kid1 off the player's arrival tile",
    );
}

#[test]
fn walk_to_npc_then_interact_starts_dialogue() {
    let mut h = Harness::new(42);
    h.start_dev_game();

    // Mommy in the dev gallery has only the "talk" option (gifts off, no puzzle,
    // no challenge). walk_to_npc pathfinds to an adjacent walkable tile and turns
    // to face them.
    h.walk_to_npc(NpcKind::Mommy);

    let mark = h.mark();
    h.interact();

    // Stronger than asserting state==Dialogue: assert the menu was *skipped* — no
    // transition through InteractionMenu — and DialogueStarted fired.
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
