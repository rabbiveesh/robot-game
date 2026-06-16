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
fn sage_offers_pattern_and_solving_it_rewards() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to_npc(NpcKind::Sage);

    h.interact();
    assert_eq!(h.game.state, GameState::InteractionMenu,
        "puzzler NPC should open the menu with a pattern option");

    let mark = h.mark();
    let start_dums = h.game.dum_dums;
    h.select_option("pattern");
    h.wait_until(|g| g.state == GameState::Pattern);

    h.solve_pattern_correctly();

    let events = h.events_since(mark);
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::PatternStarted { .. })),
        "expected PatternStarted; got: {:?}", events,
    );
    let resolved = events.iter().find_map(|e| match e {
        GameEvent::PatternResolved { correct, attempts, .. } => Some((*correct, *attempts)),
        _ => None,
    }).expect(&format!("expected PatternResolved; got: {:?}", events));
    assert_eq!(resolved.0, true, "solving with the correct first pick is correct");
    assert_eq!(resolved.1, 1, "one clean pick = one attempt");
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::DumDumsAwarded { .. })),
        "solving a pattern should award dum_dums; got: {:?}", events,
    );
    assert_eq!(h.game.dum_dums, start_dums + 1);
    assert_eq!(h.game.state, GameState::Playing);
}

#[test]
fn pattern_wrong_pick_bounces_back_then_solves() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to_npc(NpcKind::Sage);
    h.interact();

    let mark = h.mark();
    h.select_option("pattern");
    h.wait_until(|g| g.state == GameState::Pattern);

    // A wrong pick must NOT end the puzzle — it bounces back for another try
    // (fail gracefully, never punish).
    h.select_wrong_pattern_choice();
    assert_eq!(h.game.state, GameState::Pattern, "wrong pick should keep the puzzle open");
    {
        let ap = h.game.active_pattern().expect("pattern still active after a wrong pick");
        assert!(ap.session.last_wrong.is_some(), "wrong pick recorded for bounce-back");
    }

    h.solve_pattern_correctly();
    let events = h.events_since(mark);
    let attempts = events.iter().find_map(|e| match e {
        GameEvent::PatternResolved { attempts, .. } => Some(*attempts),
        _ => None,
    }).expect("expected PatternResolved");
    assert!(attempts >= 2, "a wrong pick then a right one is at least two attempts, got {attempts}");
    assert_eq!(h.game.state, GameState::Playing);
}

#[test]
fn sage_offers_balance_and_solving_it_rewards() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to_npc(NpcKind::Sage);
    h.interact();
    assert_eq!(h.game.state, GameState::InteractionMenu);

    let mark = h.mark();
    let start_dums = h.game.dum_dums;
    h.select_option("balance");
    h.wait_until(|g| g.state == GameState::Balance);

    h.solve_balance_correctly();

    let events = h.events_since(mark);
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::BalanceStarted { .. })),
        "expected BalanceStarted; got: {:?}", events,
    );
    let resolved = events.iter().find_map(|e| match e {
        GameEvent::BalanceResolved { correct, attempts, .. } => Some((*correct, *attempts)),
        _ => None,
    }).expect(&format!("expected BalanceResolved; got: {:?}", events));
    assert_eq!(resolved, (true, 1), "one clean guess solves it");
    assert_eq!(h.game.dum_dums, start_dums + 1, "solving a balance awards a dum dum");
    assert_eq!(h.game.state, GameState::Playing);
}

#[test]
fn balance_wrong_guess_tips_then_solves() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to_npc(NpcKind::Sage);
    h.interact();

    let mark = h.mark();
    h.select_option("balance");
    h.wait_until(|g| g.state == GameState::Balance);

    h.select_wrong_balance_value();
    assert_eq!(h.game.state, GameState::Balance, "a wrong guess keeps the scale open");
    {
        let ab = h.game.active_balance().expect("balance still active after a wrong guess");
        assert!(ab.session.last_wrong.is_some(), "wrong guess recorded for the tip animation");
    }

    h.solve_balance_correctly();
    let events = h.events_since(mark);
    let attempts = events.iter().find_map(|e| match e {
        GameEvent::BalanceResolved { attempts, .. } => Some(*attempts),
        _ => None,
    }).expect("expected BalanceResolved");
    assert!(attempts >= 2, "wrong then right is at least two attempts, got {attempts}");
}

#[test]
fn sage_offers_sudoku_and_solving_it_rewards() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to_npc(NpcKind::Sage);
    h.interact();
    assert_eq!(h.game.state, GameState::InteractionMenu);

    let mark = h.mark();
    let start_dums = h.game.dum_dums;
    h.select_option("sudoku");
    h.wait_until(|g| g.state == GameState::Sudoku);

    let grid = h.game.active_sudoku().expect("active Sudoku after picking 'sudoku'")
        .session.puzzle.grid_size;
    assert!(grid == 4 || grid == 6, "sudoku grid should be 4 or 6, got {grid}");

    h.solve_sudoku_correctly();

    let events = h.events_since(mark);
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::SudokuStarted { .. })),
        "expected SudokuStarted; got: {:?}", events,
    );
    let resolved = events.iter().find_map(|e| match e {
        GameEvent::SudokuResolved { correct, grid_size, constraint_violations, .. } =>
            Some((*correct, *grid_size, *constraint_violations)),
        _ => None,
    }).expect(&format!("expected SudokuResolved; got: {:?}", events));
    assert_eq!(resolved, (true, grid, 0),
        "solving by the known solution resolves correct with no violations");
    assert_eq!(h.game.dum_dums, start_dums + 1, "solving a sudoku awards a dum dum");
    assert_eq!(h.game.state, GameState::Playing);
}

#[test]
fn shopkeeper_sells_a_cosmetic_via_embedded_subtraction() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to_npc(NpcKind::Shopkeeper);
    h.interact();
    assert_eq!(h.game.state, GameState::InteractionMenu, "shopkeeper should offer a menu");

    let start = h.game.dum_dums; // dev game starts at 20
    h.select_option("shop");
    h.wait_until(|g| g.state == GameState::Shop);

    let mark = h.mark();
    h.buy_shop_item("hat"); // costs 3 → solve 20 - 3 = 17

    assert_eq!(h.game.dum_dums, start - 3, "buying the hat spends its cost");
    let events = h.events_since(mark);
    let spent = events.iter().find_map(|e| match e {
        GameEvent::DumDumsSpent { amount, item } => Some((*amount, item.clone())),
        _ => None,
    }).expect(&format!("expected DumDumsSpent; got: {:?}", events));
    assert_eq!(spent, (3, "hat".to_string()));

    h.close_shop();
    assert_eq!(h.game.state, GameState::Playing);
}

#[test]
fn click_to_walk_routes_the_player_toward_the_tapped_tile() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    let start = (h.game.player.tile_x, h.game.player.tile_y);
    let (w, ht) = (h.game.map.width, h.game.map.height);

    // Pick the farthest reachable tile within a small radius — a real multi-tile
    // path that the BFS router can actually solve over the dev map.
    let mut target = start;
    let mut best = 0usize;
    for r in start.1.saturating_sub(4)..=(start.1 + 4).min(ht - 1) {
        for c in start.0.saturating_sub(4)..=(start.0 + 4).min(w - 1) {
            let goal = (c, r);
            if goal == start {
                continue;
            }
            let walkable = |cc: usize, rr: usize| !h.game.map.is_solid(cc, rr);
            if let Some(p) = robot_buddy_game::pathfinding::find_path(start, goal, w, ht, walkable) {
                if p.len() > best {
                    best = p.len();
                    target = goal;
                }
            }
        }
    }
    assert!(best > 0, "expected a reachable tile near the player");

    let manhattan = |a: (usize, usize), b: (usize, usize)| {
        (a.0 as i32 - b.0 as i32).unsigned_abs() + (a.1 as i32 - b.1 as i32).unsigned_abs()
    };
    let before = manhattan(start, target);

    h.click_tile(target.0, target.1);
    h.advance(300); // ~16 frames/tile; plenty to traverse a few tiles

    let now = (h.game.player.tile_x, h.game.player.tile_y);
    assert_ne!(now, start, "a tap should set the player walking");
    assert!(
        manhattan(now, target) < before,
        "the player should end up closer to the tapped tile (start {:?}, now {:?}, target {:?})",
        start, now, target,
    );
}

#[test]
fn tapping_an_npc_walks_over_and_auto_interacts() {
    let mut h = Harness::new(7);
    h.start_dev_game();
    // Stand next to Sage first (reliable), then tap Sage — arrival should fire
    // the interaction automatically (Sage is a puzzler → the menu opens), no
    // separate Space press.
    h.walk_to_npc(NpcKind::Sage);
    let sage = h.game.npcs.iter().find(|n| n.kind == NpcKind::Sage)
        .map(|n| (n.entity.tile_x, n.entity.tile_y))
        .expect("Sage on the dev map");

    h.click_tile(sage.0, sage.1);
    h.wait_until(|g| g.state == GameState::InteractionMenu);

    let p = (h.game.player.tile_x, h.game.player.tile_y);
    let dist = (p.0 as i32 - sage.0 as i32).abs() + (p.1 as i32 - sage.1 as i32).abs();
    assert_eq!(dist, 1, "player should be standing next to Sage when it auto-interacts");
}

#[test]
fn parent_overlay_toggles_feature_flags_in_game() {
    use robot_buddy_game::ui::settings_overlay::Feature;
    let mut h = Harness::new(7);
    h.start_dev_game();
    assert!(!h.game.features.encounters);
    assert!(!h.game.features.quest);

    // Open settings → reveal the parent section → flip two features on.
    h.open_settings();
    h.click_parent_options();
    h.toggle_feature_in_settings(Feature::Encounters);
    assert!(h.game.features.encounters, "parent overlay should enable encounters");
    h.toggle_feature_in_settings(Feature::Quest);
    assert!(h.game.features.quest, "parent overlay should enable quests");

    // Toggling again turns it back off (it's a real toggle).
    h.toggle_feature_in_settings(Feature::Encounters);
    assert!(!h.game.features.encounters, "toggling again disables it");
}

#[test]
fn dev_toggle_flips_the_quest_flag() {
    use macroquad::prelude::KeyCode;
    let mut h = Harness::new(7);
    h.start_dev_game();
    assert!(!h.game.features.quest, "quests default OFF");
    h.walk_to(2, 9);
    h.step_through_portal(KeyCode::Left, "control");
    h.walk_to_npc(NpcKind::CtrlToggleQuest);
    h.interact();
    h.wait_until(|g| g.state == GameState::Dialogue);
    assert!(h.game.features.quest, "dev knob should enable quests");
}

#[test]
fn dev_quest_runs_to_completion_with_embedded_math() {
    use macroquad::prelude::KeyCode;
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to(2, 9);
    h.step_through_portal(KeyCode::Left, "control");
    h.walk_to_npc(NpcKind::CtrlStartQuest);

    let mark = h.mark();
    let start = h.game.dum_dums;
    h.interact();
    h.wait_until(|g| g.state == GameState::Quest);
    assert!(h.game.active_quest().is_some(), "a quest should be running");

    h.play_quest();
    assert_eq!(h.game.state, GameState::Playing, "finishing the quest returns to play");
    let events = h.events_since(mark);
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::QuestCompleted)),
        "expected QuestCompleted; got: {:?}", events,
    );
    assert_eq!(h.game.dum_dums, start + 3, "the welcome quest pays out 3 Dum Dums");
}

#[test]
fn dev_toggle_flips_the_manipulatives_flag() {
    use macroquad::prelude::KeyCode;
    let mut h = Harness::new(7);
    h.start_dev_game();
    assert!(!h.game.features.cra_manipulatives, "manipulatives default OFF");
    h.walk_to(2, 9);
    h.step_through_portal(KeyCode::Left, "control");
    h.walk_to_npc(NpcKind::CtrlToggleManipulatives);
    h.interact();
    h.wait_until(|g| g.state == GameState::Dialogue);
    assert!(h.game.features.cra_manipulatives, "dev knob should enable CRA manipulatives");
}

#[test]
fn dev_manipulative_is_hands_on_and_rewards_like_a_challenge() {
    use macroquad::prelude::KeyCode;
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to(2, 9);
    h.step_through_portal(KeyCode::Left, "control");
    h.walk_to_npc(NpcKind::CtrlTriggerManipulative);

    let mark = h.mark();
    let start = h.game.dum_dums;
    h.interact();
    h.wait_until(|g| g.state == GameState::Manipulative);
    assert!(h.game.active_manipulative().is_some(), "a manipulative should be active");

    h.solve_manipulative();
    assert_eq!(h.game.state, GameState::Playing);
    assert_eq!(h.game.dum_dums, start + 1, "solving the manipulative rewards a Dum Dum");
    let events = h.events_since(mark);
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::ChallengeResolved { correct: true, .. })),
        "manipulative completion feeds the same resolved signal; got: {:?}", events,
    );
}

#[test]
fn dev_toggle_flips_the_encounters_flag() {
    use macroquad::prelude::KeyCode;
    let mut h = Harness::new(7);
    h.start_dev_game();
    assert!(!h.game.features.encounters, "encounters default OFF (suite + normal play unaffected)");

    h.walk_to(2, 9);
    h.step_through_portal(KeyCode::Left, "control");
    h.walk_to_npc(NpcKind::CtrlToggleEncounters);
    h.interact();
    h.wait_until(|g| g.state == GameState::Dialogue);
    assert!(h.game.features.encounters, "the dev knob should enable encounters for playtesting");
}

#[test]
fn dev_trigger_fires_and_routes_an_encounter() {
    use macroquad::prelude::KeyCode;
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.walk_to(2, 9);
    h.step_through_portal(KeyCode::Left, "control");
    h.walk_to_npc(NpcKind::CtrlTriggerEncounter);

    let mark = h.mark();
    h.interact();
    let events = h.events_since(mark);
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::EncounterTriggered { .. })),
        "expected an EncounterTriggered event; got: {:?}", events,
    );
    // Encounters route to dialogue (flavor / sighting / found-dum-dum) or a challenge.
    assert!(
        matches!(h.game.state, GameState::Dialogue | GameState::Challenge),
        "encounter should open dialogue or a challenge, got {:?}", h.game.state,
    );
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
fn pushing_a_wandering_kid_shoves_them_one_tile_and_player_takes_their_spot() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;
    use macroquad::prelude::KeyCode;

    // Wandering NPCs are tagged `Solidity::PushableAfter(0.18)` in the
    // resolver snapshot. Holding direction into one for ~0.18s should pop
    // them one tile in that direction and slide the player onto their spot.
    let mut h = Harness::new(1);
    h.start_dev_game();

    // Switch to the home map directly. Same trick as
    // `wandering_npc_walking_onto_portal_transfers_to_destination_map`:
    // the push behavior doesn't depend on the journey to home, only on the
    // map's layout (open WoodFloor corridor along row 5).
    h.game.map = Map::home();
    h.game.npcs = npc_mod::npcs_for_map("home");
    h.game.npcs_offstage.clear();

    // Park Sparky in a corner so his follow path can't wander into the
    // push corridor and block (5,5).
    h.game.sparky.entity.tile_x = 1;
    h.game.sparky.entity.tile_y = 1;
    h.game.sparky.entity.x = 1.0 * 48.0;
    h.game.sparky.entity.y = 1.0 * 48.0;
    h.game.sparky.entity.target_x = h.game.sparky.entity.x;
    h.game.sparky.entity.target_y = h.game.sparky.entity.y;
    h.game.sparky.entity.moving = false;

    // Pin Kid1 at (4,5) and freeze their wander roll so the only force that
    // moves them is the player's push.
    let kid_idx = h.game.npcs.iter()
        .position(|n| n.kind == NpcKind::Kid1)
        .expect("home spawns a Kid1");
    {
        let n = &mut h.game.npcs[kid_idx];
        n.entity.tile_x = 4;
        n.entity.tile_y = 5;
        n.entity.x = 4.0 * 48.0;
        n.entity.y = 5.0 * 48.0;
        n.entity.target_x = n.entity.x;
        n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.wander_cooldown = 9999.0;
    }
    // Kid2 and Mommy could wander into / sit on the push destination —
    // park them well clear and freeze their cooldowns too.
    for n in h.game.npcs.iter_mut() {
        if n.kind == NpcKind::Kid1 { continue; }
        n.entity.tile_x = 1;
        n.entity.tile_y = 2;
        n.entity.x = 1.0 * 48.0;
        n.entity.y = 2.0 * 48.0;
        n.entity.target_x = n.entity.x;
        n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.wander_cooldown = 9999.0;
    }

    // Player at (3,5), facing the kid at (4,5). Push will send the kid to
    // (5,5) — clear WoodFloor — and slide the player onto (4,5).
    h.game.player.tile_x = 3;
    h.game.player.tile_y = 5;
    h.game.player.x = 3.0 * 48.0;
    h.game.player.y = 5.0 * 48.0;
    h.game.player.target_x = h.game.player.x;
    h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;

    // Lean Right. Pressure builds at dt=1/60 per frame; threshold 0.18s
    // is ~11 frames of holding before the push fires. tile_x/tile_y update
    // the instant `start_move` runs, so we don't need to wait for the slide
    // to finish — but we DO need to stop before the player settles and a
    // second lean rolls into a second push (~27 frames per cycle). 25 is in
    // the safe middle.
    for _ in 0..25 {
        h.hold(KeyCode::Right);
    }

    let kid = h.game.npcs.iter().find(|n| n.kind == NpcKind::Kid1)
        .expect("Kid1 still on home map after push");
    assert_eq!((kid.entity.tile_x, kid.entity.tile_y), (5, 5),
        "Kid1 should be shoved one tile right of their original (4,5) spot");
    assert_eq!((h.game.player.tile_x, h.game.player.tile_y), (4, 5),
        "player should now sit on Kid1's old tile (4,5)");
}

#[test]
fn npc_mid_slide_does_not_overshoot_when_a_huge_dt_arrives() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;

    // In the browser, a backgrounded tab can pause requestAnimationFrame for
    // seconds; when it resumes, macroquad reports a single huge dt for that
    // frame. `move_toward_target` advances pixels by MOVE_SPEED * dt — without
    // a clamp, an NPC mid-slide flies hundreds of tiles past their target and
    // then "ghost walks" back at normal speed on subsequent normal-dt frames.
    let mut h = Harness::new(1);
    h.start_dev_game();

    h.game.map = Map::home();
    h.game.npcs = npc_mod::npcs_for_map("home");
    h.game.npcs_offstage.clear();

    let kid_idx = h.game.npcs.iter()
        .position(|n| n.kind == NpcKind::Kid1)
        .expect("home spawns a Kid1");
    {
        let n = &mut h.game.npcs[kid_idx];
        n.entity.tile_x = 4;
        n.entity.tile_y = 5;
        n.entity.x = 4.0 * 48.0;
        n.entity.y = 5.0 * 48.0;
        n.entity.target_x = n.entity.x;
        n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.wander_cooldown = 9999.0;
    }
    // Manually start a slide one tile right. (5,5) is open WoodFloor.
    h.game.npcs[kid_idx].entity.start_move(5, 5);

    // One frame with a 30-second dt (tab was hidden ~30s, then refocused).
    // tile_x already updated to (5,5); target_x = 5 * 48 = 240. The kid is
    // mid-pixel-slide somewhere short of 240. With unbounded step the kid
    // would zoom past target by thousands of pixels.
    h.game.step(&robot_buddy_game::input::FrameInput::empty(), 30.0, common::SCREEN);

    let kid = &h.game.npcs[kid_idx];
    assert_eq!((kid.entity.tile_x, kid.entity.tile_y), (5, 5),
        "tile coords should still reflect the destination");
    let target_px = (5.0 * 48.0, 5.0 * 48.0);
    assert!(
        (kid.entity.x - target_px.0).abs() < 0.5
            && (kid.entity.y - target_px.1).abs() < 0.5,
        "after a huge dt the kid should be snapped to the target tile, not \
         overshooting it. target=({}, {}), got=({}, {})",
        target_px.0, target_px.1, kid.entity.x, kid.entity.y,
    );
    assert!(!kid.entity.moving,
        "snapping to target should also clear `moving`, otherwise the next \
         normal frame will see dist≈0 and idle correctly but the entity will \
         look 'stuck on' for one extra render");
}

#[test]
fn pushing_a_wandering_kid_onto_a_portal_transfers_them() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;
    use macroquad::prelude::KeyCode;

    // Push and the NPC-portal handler need to compose. Pushing a kid onto a
    // portal tile should transfer them to the destination map's stash, same
    // as if they'd wandered there on their own. Player and pushee slide for
    // the same number of frames and arrive together — the arrival bookkeeping
    // has to handle that without dropping the NPC's portal trigger.
    let mut h = Harness::new(1);
    h.start_dev_game();

    h.game.map = Map::home();
    h.game.npcs = npc_mod::npcs_for_map("home");
    h.game.npcs_offstage.clear();

    h.game.sparky.entity.tile_x = 1;
    h.game.sparky.entity.tile_y = 1;
    h.game.sparky.entity.x = 1.0 * 48.0;
    h.game.sparky.entity.y = 1.0 * 48.0;
    h.game.sparky.entity.target_x = h.game.sparky.entity.x;
    h.game.sparky.entity.target_y = h.game.sparky.entity.y;
    h.game.sparky.entity.moving = false;

    // Park Kid1 right next to the home → overworld door at (4,6). One push
    // east lands them on the portal.
    let kid_idx = h.game.npcs.iter()
        .position(|n| n.kind == NpcKind::Kid1)
        .expect("home spawns a Kid1");
    {
        let n = &mut h.game.npcs[kid_idx];
        n.entity.tile_x = 3;
        n.entity.tile_y = 6;
        n.entity.x = 3.0 * 48.0;
        n.entity.y = 6.0 * 48.0;
        n.entity.target_x = n.entity.x;
        n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.wander_cooldown = 9999.0;
    }
    for n in h.game.npcs.iter_mut() {
        if n.kind == NpcKind::Kid1 { continue; }
        n.entity.tile_x = 1;
        n.entity.tile_y = 2;
        n.entity.x = 1.0 * 48.0;
        n.entity.y = 2.0 * 48.0;
        n.entity.target_x = n.entity.x;
        n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.wander_cooldown = 9999.0;
    }

    // Player at (2,6), pushing Right.
    h.game.player.tile_x = 2;
    h.game.player.tile_y = 6;
    h.game.player.x = 2.0 * 48.0;
    h.game.player.y = 6.0 * 48.0;
    h.game.player.target_x = h.game.player.x;
    h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;

    // Pressure build is ~11 frames; once the push fires both entities are
    // mid-step and ignore further input. Release as soon as the push triggers
    // so the player doesn't follow the kid onto the portal in a chain push.
    for _ in 0..12 {
        h.hold(KeyCode::Right);
    }
    // Idle through the slide + portal-handler arrival.
    h.advance(20);

    assert_eq!(h.game.map.id, "home",
        "player should still be on home — only the kid crossed the portal");
    assert!(
        !h.game.npcs.iter().any(|n| n.kind == NpcKind::Kid1),
        "Kid1 should have left the home roster after being pushed onto the portal; \
         current kinds: {:?}",
        h.game.npcs.iter().map(|n| n.kind).collect::<Vec<_>>(),
    );
    let stash = h.game.npcs_offstage.get("overworld")
        .expect("overworld stash should exist after the push-portal transfer");
    assert!(
        stash.iter().any(|n| n.kind == NpcKind::Kid1),
        "Kid1 should now live in npcs_offstage['overworld']; got kinds: {:?}",
        stash.iter().map(|n| n.kind).collect::<Vec<_>>(),
    );
}

#[test]
fn pressing_into_a_kid_with_no_room_to_go_blocks_player() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;
    use macroquad::prelude::KeyCode;

    // If the kid's would-be destination is a wall, push fails and the player
    // stays blocked — same outcome as walking into any solid NPC.
    let mut h = Harness::new(1);
    h.start_dev_game();

    h.game.map = Map::home();
    h.game.npcs = npc_mod::npcs_for_map("home");
    h.game.npcs_offstage.clear();

    h.game.sparky.entity.tile_x = 1;
    h.game.sparky.entity.tile_y = 1;
    h.game.sparky.entity.x = 1.0 * 48.0;
    h.game.sparky.entity.y = 1.0 * 48.0;
    h.game.sparky.entity.target_x = h.game.sparky.entity.x;
    h.game.sparky.entity.target_y = h.game.sparky.entity.y;
    h.game.sparky.entity.moving = false;

    // Pin Kid1 right against the right wall (col 8 is the last floor col;
    // col 9 is Wall). Pushing Right into Kid1 has nowhere for them to go.
    let kid_idx = h.game.npcs.iter()
        .position(|n| n.kind == NpcKind::Kid1)
        .expect("home spawns a Kid1");
    {
        let n = &mut h.game.npcs[kid_idx];
        n.entity.tile_x = 8;
        n.entity.tile_y = 5;
        n.entity.x = 8.0 * 48.0;
        n.entity.y = 5.0 * 48.0;
        n.entity.target_x = n.entity.x;
        n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.wander_cooldown = 9999.0;
    }
    for n in h.game.npcs.iter_mut() {
        if n.kind == NpcKind::Kid1 { continue; }
        n.entity.tile_x = 1;
        n.entity.tile_y = 2;
        n.entity.x = 1.0 * 48.0;
        n.entity.y = 2.0 * 48.0;
        n.entity.target_x = n.entity.x;
        n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.wander_cooldown = 9999.0;
    }

    h.game.player.tile_x = 7;
    h.game.player.tile_y = 5;
    h.game.player.x = 7.0 * 48.0;
    h.game.player.y = 5.0 * 48.0;
    h.game.player.target_x = h.game.player.x;
    h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;

    for _ in 0..60 {
        h.hold(KeyCode::Right);
    }

    // Nobody moved.
    let kid = h.game.npcs.iter().find(|n| n.kind == NpcKind::Kid1).unwrap();
    assert_eq!((kid.entity.tile_x, kid.entity.tile_y), (8, 5),
        "kid pinned to wall has no push destination — should stay put");
    assert_eq!((h.game.player.tile_x, h.game.player.tile_y), (7, 5),
        "player should remain blocked when push has no destination");
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

#[test]
fn giving_a_dum_dum_recruits_the_npc_as_companion_and_returns_the_previous_one() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;

    // Setup: home map, where Mommy/Kid1/Kid2 are all gift-receivers. Player
    // starts with 20 dum_dums from dev mode. We warp directly to home rather
    // than walking — the swap mechanic doesn't depend on how we got there.
    let mut h = Harness::new(7);
    h.start_dev_game();
    h.game.map = Map::home();
    h.game.npcs = npc_mod::npcs_for_map("home");
    h.game.npcs_offstage.clear();
    h.game.companion = None;

    // Park Sparky in a corner so his follow path doesn't crowd the test area.
    h.game.sparky.entity.tile_x = 1;
    h.game.sparky.entity.tile_y = 1;
    h.game.sparky.entity.x = 1.0 * 48.0;
    h.game.sparky.entity.y = 1.0 * 48.0;
    h.game.sparky.entity.target_x = h.game.sparky.entity.x;
    h.game.sparky.entity.target_y = h.game.sparky.entity.y;
    h.game.sparky.entity.moving = false;
    // Freeze the kid wanderers' cooldowns so their roaming doesn't interfere
    // with walking the player up to a specific NPC.
    for n in h.game.npcs.iter_mut() {
        n.wander_cooldown = 9999.0;
    }
    h.game.player.tile_x = 5;
    h.game.player.tile_y = 3;
    h.game.player.x = 5.0 * 48.0;
    h.game.player.y = 3.0 * 48.0;
    h.game.player.target_x = h.game.player.x;
    h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;

    // Step 1: give Kid1 a dum dum. Expect Kid1 to leave the roster and
    // appear as the player's companion.
    h.walk_to_npc(NpcKind::Kid1);
    let mark = h.mark();
    h.interact();
    h.select_option("give");
    h.wait_until(|g| g.state == GameState::Dialogue);
    h.finish_dialogue();
    h.wait_until(|g| g.state == GameState::Playing);

    let events = h.events_since(mark);
    let first_swap = events.iter().find_map(|e| match e {
        GameEvent::CompanionChanged { joined, left } => Some((joined.clone(), left.clone())),
        _ => None,
    }).expect(&format!("expected CompanionChanged after first gift; got: {:?}", events));
    assert_eq!(first_swap.0.as_deref(), Some("kid_1"));
    assert_eq!(first_swap.1.as_deref(), Some("sparky"),
        "first NPC swap displaces Sparky; got left = {:?}", first_swap.1);

    assert!(
        h.game.companion.as_ref().map(|c| c.kind) == Some(NpcKind::Kid1),
        "companion slot should hold Kid1; got {:?}",
        h.game.companion.as_ref().map(|c| c.kind),
    );
    assert!(
        h.game.companion.as_ref().unwrap().is_following(),
        "companion should be in follower mode after the swap",
    );
    assert!(
        !h.game.npcs.iter().any(|n| n.kind == NpcKind::Kid1),
        "Kid1 should no longer appear in the home roster after recruitment",
    );
    assert!(h.game.sparky_parked,
        "Sparky should be parked after an NPC took over as buddy");

    // Step 2: give Mommy a dum dum. Expect Mommy to become the companion
    // and Kid1 to return home — back into self.npcs at her home tile.
    h.walk_to_npc(NpcKind::Mommy);
    let mark = h.mark();
    h.interact();
    h.select_option("give");
    h.wait_until(|g| g.state == GameState::Dialogue);
    h.finish_dialogue();
    h.wait_until(|g| g.state == GameState::Playing);

    let events = h.events_since(mark);
    let second_swap = events.iter().find_map(|e| match e {
        GameEvent::CompanionChanged { joined, left } => Some((joined.clone(), left.clone())),
        _ => None,
    }).expect(&format!("expected CompanionChanged after second gift; got: {:?}", events));
    assert_eq!(second_swap.0.as_deref(), Some("mommy"),
        "second swap should install Mommy as the new companion");
    assert_eq!(second_swap.1.as_deref(), Some("kid_1"),
        "second swap should release Kid1 (the previous companion)");

    assert!(
        h.game.companion.as_ref().map(|c| c.kind) == Some(NpcKind::Mommy),
        "companion slot should hold Mommy now; got {:?}",
        h.game.companion.as_ref().map(|c| c.kind),
    );
    let returned_kid = h.game.npcs.iter().find(|n| n.kind == NpcKind::Kid1)
        .expect("Kid1 should be back in the home roster after being swapped out");
    assert_eq!((returned_kid.entity.tile_x, returned_kid.entity.tile_y), (6, 5),
        "swapped-out Kid1 should snap back to her static home tile (6, 5)");
    assert!(!returned_kid.is_following(),
        "released companion should drop follower mode and resume normal NPC behavior");
}

#[test]
fn gifting_parked_sparky_brings_him_back_and_sends_the_npc_home() {
    use robot_buddy_game::game::{SPARKY_HOME_MAP, SPARKY_HOME_TX, SPARKY_HOME_TY};

    // Dev start gives 20 dum_dums. The dev map happens to have a Mommy NPC
    // who can't normally receive gifts (the dev gallery sets gifts off), so
    // we warp to the real home map first where Mommy is gift-eligible.
    let mut h = Harness::new(11);
    h.start_dev_game();

    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;
    h.game.map = Map::home();
    h.game.npcs = npc_mod::npcs_for_map("home");
    h.game.npcs_offstage.clear();
    h.game.companion = None;
    h.game.sparky_parked = false;
    for n in h.game.npcs.iter_mut() { n.wander_cooldown = 9999.0; }
    h.game.player.tile_x = 4;
    h.game.player.tile_y = 3;
    h.game.player.x = 4.0 * 48.0;
    h.game.player.y = 3.0 * 48.0;
    h.game.player.target_x = h.game.player.x;
    h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;

    // Recruit Mommy. Sparky should park at his home tile on the overworld.
    h.walk_to_npc(NpcKind::Mommy);
    h.interact();
    h.select_option("give");
    h.wait_until(|g| g.state == GameState::Dialogue);
    h.finish_dialogue();
    h.wait_until(|g| g.state == GameState::Playing);

    assert_eq!(h.game.current_buddy_id(), "mommy",
        "Mommy should be the active buddy after gifting her");
    assert!(h.game.sparky_parked,
        "Sparky should be parked once an NPC has taken over");
    assert_eq!(
        (h.game.sparky.entity.tile_x, h.game.sparky.entity.tile_y),
        (SPARKY_HOME_TX, SPARKY_HOME_TY),
        "parked Sparky should sit at his home tile near Professor Gizmo",
    );
    assert!(!h.game.sparky_is_here(),
        "while the player is on 'home', parked Sparky (on '{}') shouldn't be here",
        SPARKY_HOME_MAP,
    );

    // Now walk to the overworld so we can face Sparky and gift him.
    h.game.map = Map::overworld();
    h.game.npcs.clear();
    let mut overworld = npc_mod::npcs_for_map("overworld");
    overworld.retain(|n| n.kind != h.game.companion.as_ref().unwrap().kind);
    h.game.npcs = overworld;
    assert!(h.game.sparky_is_here(),
        "with the player back on overworld, parked Sparky should be visible");

    // Park player one tile right of Sparky's home so we can turn-and-interact.
    h.game.player.tile_x = SPARKY_HOME_TX + 1;
    h.game.player.tile_y = SPARKY_HOME_TY;
    h.game.player.x = (SPARKY_HOME_TX + 1) as f32 * 48.0;
    h.game.player.y = SPARKY_HOME_TY as f32 * 48.0;
    h.game.player.target_x = h.game.player.x;
    h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;
    // Companion is hovering somewhere from following us. Park them off to
    // the side so they don't collide with the interaction.
    {
        let c = h.game.companion.as_mut().unwrap();
        c.entity.tile_x = SPARKY_HOME_TX + 2;
        c.entity.tile_y = SPARKY_HOME_TY;
        c.entity.x = (SPARKY_HOME_TX + 2) as f32 * 48.0;
        c.entity.y = SPARKY_HOME_TY as f32 * 48.0;
        c.entity.target_x = c.entity.x;
        c.entity.target_y = c.entity.y;
        c.entity.moving = false;
        if let Some(p) = c.pathing.as_mut() { p.clear(); }
    }

    // Face Sparky (he's to the left) by holding Left for one frame — that
    // sets player.dir without moving since Sparky soft-blocks.
    h.hold(macroquad::prelude::KeyCode::Left);

    let mark = h.mark();
    h.interact();
    h.select_option("give");
    h.wait_until(|g| g.state == GameState::Dialogue);
    h.finish_dialogue();
    h.wait_until(|g| g.state == GameState::Playing);

    let events = h.events_since(mark);
    let swap = events.iter().find_map(|e| match e {
        GameEvent::CompanionChanged { joined, left } => Some((joined.clone(), left.clone())),
        _ => None,
    }).expect(&format!("expected CompanionChanged when gifting parked Sparky; got: {:?}", events));
    assert_eq!(swap.0.as_deref(), Some("sparky"),
        "Sparky should rejoin as the active buddy");
    assert_eq!(swap.1.as_deref(), Some("mommy"),
        "Mommy should be released back home when Sparky takes over again");

    assert_eq!(h.game.current_buddy_id(), "sparky",
        "after swap-back the buddy id should be 'sparky'");
    assert!(!h.game.sparky_parked,
        "Sparky should no longer be parked after swapping back in");
    assert!(h.game.companion.is_none(),
        "companion slot should be empty after Sparky returns as buddy");
}

#[test]
fn companion_snaps_to_the_player_after_a_portal_warp() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;
    use macroquad::prelude::KeyCode;

    // A swapped-in NPC companion used to keep its old-map coordinates and stale
    // path trail across a warp — landing it in some random spot on the new map,
    // adrift until the player walked up and tripped the adjacency reset. After
    // the fix the companion gets the same post-warp treatment Sparky gets:
    // teleported to the player's side with a cleared queue.
    let mut h = Harness::new(3);
    h.start_dev_game();
    h.game.map = Map::home();
    h.game.npcs = npc_mod::npcs_for_map("home");
    h.game.npcs_offstage.clear();

    // Recruit Mommy as the companion by hand (the swap mechanics are covered by
    // other tests; here we only care about her position after the warp).
    let mut mommy = h.game.npcs.iter().find(|n| n.kind == NpcKind::Mommy).cloned().unwrap();
    h.game.npcs.retain(|n| n.kind != NpcKind::Mommy);
    mommy.start_following();
    // Strand her far from the door with an empty queue — the exact state that
    // used to survive a warp untouched.
    mommy.entity.tile_x = 1; mommy.entity.tile_y = 1;
    mommy.entity.x = 48.0; mommy.entity.y = 48.0;
    mommy.entity.target_x = mommy.entity.x; mommy.entity.target_y = mommy.entity.y;
    h.game.companion = Some(mommy);
    h.game.sparky_parked = true;

    // Stand the player on the tile just above the home→overworld door (4,6).
    h.game.player.tile_x = 4; h.game.player.tile_y = 5;
    h.game.player.x = 4.0 * 48.0; h.game.player.y = 5.0 * 48.0;
    h.game.player.target_x = h.game.player.x; h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;

    h.step_through_portal(KeyCode::Down, "overworld");

    let c = h.game.companion.as_ref().expect("companion should survive the warp");
    let dist = (c.entity.tile_x as i32 - h.game.player.tile_x as i32).abs()
        + (c.entity.tile_y as i32 - h.game.player.tile_y as i32).abs();
    assert!(dist <= 1,
        "companion should land adjacent to the player after the warp; player at ({},{}), companion at ({},{})",
        h.game.player.tile_x, h.game.player.tile_y, c.entity.tile_x, c.entity.tile_y);
    assert!(!c.entity.moving, "companion should be at rest immediately after the warp");
    assert!(c.is_following(), "companion keeps follower mode across a warp");
    assert!(c.pathing.as_ref().unwrap().is_empty(),
        "companion's stale path trail should be cleared by the warp");
}

#[test]
fn swapping_npc_companions_voices_each_line_by_the_right_character() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;
    use macroquad::prelude::KeyCode;

    // Bug: when an NPC handed off to another NPC (e.g. Mommy → Bolt), Sparky
    // narrated both the "joined" and "left" lines — even though he's parked off
    // on his home map and isn't even in the scene. Each line should come from
    // the character it's about: the newcomer greets, the departer says goodbye.
    let mut h = Harness::new(5);
    h.start_dev_game();
    h.game.map = Map::shop();
    h.game.npcs = npc_mod::npcs_for_map("shop");
    h.game.npcs_offstage.clear();

    // Mommy is the current companion; Sparky is parked away off-map.
    let mut mommy = npc_mod::npcs_for_map("home").into_iter()
        .find(|n| n.kind == NpcKind::Mommy).unwrap();
    mommy.start_following();
    h.game.companion = Some(mommy);
    h.game.sparky_parked = true;

    // Stand the player at (4,2), just left of Bolt the Shopkeeper at (5,2), and
    // face him.
    h.game.player.tile_x = 4; h.game.player.tile_y = 2;
    h.game.player.x = 4.0 * 48.0; h.game.player.y = 2.0 * 48.0;
    h.game.player.target_x = h.game.player.x; h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;
    h.hold(KeyCode::Right); // face the shopkeeper

    h.interact();
    h.select_option("give");
    h.wait_until(|g| g.state == GameState::Dialogue);

    let lines = h.game.dialogue_lines();
    let speakers: Vec<&str> = lines.iter().map(|(s, _)| s.as_str()).collect();
    assert!(speakers.contains(&"Bolt the Shopkeeper"),
        "the newcomer should greet in their own voice; lines were {:?}", lines);
    assert!(speakers.contains(&"Mommy"),
        "the departing companion should say goodbye in her own voice; lines were {:?}", lines);
    assert!(!speakers.contains(&"Sparky"),
        "Sparky isn't in this scene and shouldn't speak; lines were {:?}", lines);
}

#[test]
fn wandering_npc_on_the_dream_portal_crosses_into_the_dream() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;

    // The secret dream portal is just an ordinary door to a wanderer: a kid
    // that lands on it crosses into the dream world like any other portal,
    // landing in the offstage dream roster to reappear when the player visits.
    let mut h = Harness::new(2);
    h.start_dev_game();
    h.game.map = Map::overworld();
    h.game.npcs = npc_mod::npcs_for_map("overworld");
    h.game.npcs_offstage.clear();
    h.game.dreaming = false;
    // Keep the player (and the player-side portal handler) well away from the
    // dream tile so only the NPC handler fires.
    h.game.player.tile_x = 5; h.game.player.tile_y = 8;
    h.game.player.x = 5.0 * 48.0; h.game.player.y = 8.0 * 48.0;
    h.game.player.target_x = h.game.player.x; h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;

    // Drive the overworld Sage onto the secret dream portal tile (16,14) from
    // the adjacent bridge tile (15,14).
    let kind = h.game.npcs[0].kind;
    {
        let n = &mut h.game.npcs[0];
        n.entity.tile_x = 15; n.entity.tile_y = 14;
        n.entity.x = 15.0 * 48.0; n.entity.y = 14.0 * 48.0;
        n.entity.target_x = n.entity.x; n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.entity.start_move(16, 14);
    }
    h.advance(30);

    assert!(!h.game.npcs.iter().any(|n| n.kind == kind),
        "NPC should have left the overworld roster after crossing the dream portal");
    let dream_stash = h.game.npcs_offstage.get("dream")
        .expect("NPC should be stashed in the dream map after crossing the secret portal");
    assert!(dream_stash.iter().any(|n| n.kind == kind),
        "the wanderer should now live in the dream roster; got {:?}",
        dream_stash.iter().map(|n| n.kind).collect::<Vec<_>>());
}

#[test]
fn pushing_an_npc_onto_the_dream_portal_sends_them_into_the_dream() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;
    use macroquad::prelude::KeyCode;

    // The player can also deliberately shove a wandering kid through the secret
    // dream portal: push composes with the NPC-portal handler the same way it
    // does for ordinary doors.
    let mut h = Harness::new(4);
    h.start_dev_game();
    h.game.map = Map::overworld();
    h.game.npcs_offstage.clear();
    h.game.dreaming = false;

    // Lone wandering kid on the bridge tile (15,14), right next to the dream
    // portal water tile (16,14). Cooldown frozen so she holds still to be shoved.
    let mut kid = npc_mod::npcs_for_map("home").into_iter()
        .find(|n| n.kind == NpcKind::Kid1).unwrap();
    kid.entity.tile_x = 15; kid.entity.tile_y = 14;
    kid.entity.x = 15.0 * 48.0; kid.entity.y = 14.0 * 48.0;
    kid.entity.target_x = kid.entity.x; kid.entity.target_y = kid.entity.y;
    kid.entity.moving = false;
    kid.wander_cooldown = 9999.0;
    h.game.npcs = vec![kid];

    // Keep Sparky out of the way on the far side of the map.
    h.game.sparky.entity.tile_x = 5; h.game.sparky.entity.tile_y = 8;
    h.game.sparky.entity.x = 5.0 * 48.0; h.game.sparky.entity.y = 8.0 * 48.0;
    h.game.sparky.entity.target_x = h.game.sparky.entity.x; h.game.sparky.entity.target_y = h.game.sparky.entity.y;
    h.game.sparky.entity.moving = false;

    // Player on the bridge at (14,14), pushing Right into the kid.
    h.game.player.tile_x = 14; h.game.player.tile_y = 14;
    h.game.player.x = 14.0 * 48.0; h.game.player.y = 14.0 * 48.0;
    h.game.player.target_x = h.game.player.x; h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;

    // Build pressure until the push fires (~11 frames), then release so the
    // player doesn't chain onto the portal after the kid clears it.
    for _ in 0..12 { h.hold(KeyCode::Right); }
    h.advance(20);

    assert_eq!(h.game.map.id, "overworld",
        "player should stay on the overworld; only the kid crosses");
    assert!(!h.game.npcs.iter().any(|n| n.kind == NpcKind::Kid1),
        "kid should have left the overworld roster after being pushed through; got {:?}",
        h.game.npcs.iter().map(|n| n.kind).collect::<Vec<_>>());
    let dream = h.game.npcs_offstage.get("dream")
        .expect("dream stash should exist after pushing the kid through");
    assert!(dream.iter().any(|n| n.kind == NpcKind::Kid1),
        "kid should now live in the dream roster; got {:?}",
        dream.iter().map(|n| n.kind).collect::<Vec<_>>());
}

// ─── New-map genericity: a freshly added map ("annex") plus a fresh NPC
// ("Pip") exercise the portal / wander / companion-swap / warp-snap systems
// with zero per-map special-casing. If any of these break, the architecture
// has grown a hidden coupling to the existing map list. ───────────────────

#[test]
fn a_brand_new_map_supports_portals_companions_and_warp_snap() {
    use macroquad::prelude::KeyCode;

    let mut h = Harness::new(8);
    h.start_dev_game();

    // Stand on the dev tile just above the Annex door (13,10) and walk through.
    h.game.player.tile_x = 13; h.game.player.tile_y = 9;
    h.game.player.x = 13.0 * 48.0; h.game.player.y = 9.0 * 48.0;
    h.game.player.target_x = h.game.player.x; h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;

    h.step_through_portal(KeyCode::Down, "annex");
    assert_eq!(h.game.map.id, "annex", "player portal into a new map should work");

    // The new map's wandering NPC loaded from the roster. Freeze her so we can
    // walk up deliberately.
    let pip = h.game.npcs.iter_mut().find(|n| n.kind == NpcKind::Pip)
        .expect("annex roster should spawn Pip");
    assert!(pip.wanders, "Pip should be a wanderer on the new map");
    pip.wander_cooldown = 9999.0;

    // Recruit Pip with a dum dum. The swap dialogue must name her properly —
    // this is the generic display-name path that used to depend on a hardcoded
    // list of map ids (which would have shown the raw "pip" token instead).
    h.walk_to_npc(NpcKind::Pip);
    h.interact();
    h.select_option("give");
    h.wait_until(|g| g.state == GameState::Dialogue);

    let lines = h.game.dialogue_lines();
    assert!(lines.iter().any(|(speaker, _)| speaker == "Pip"),
        "the new-map NPC should greet under her real display name; lines were {:?}", lines);
    // The join line is voiced via display_name_for_buddy_id(joined). The old
    // map-list lookup didn't know about "annex", so it leaked the raw "pip"
    // token as the speaker. Map-agnostic resolution must never do that.
    assert!(!lines.iter().any(|(speaker, _)| speaker == "pip"),
        "raw id token leaked as a speaker — display-name resolution is still map-coupled; lines were {:?}", lines);
    h.finish_dialogue();
    h.wait_until(|g| g.state == GameState::Playing);

    assert_eq!(h.game.companion.as_ref().map(|c| c.kind), Some(NpcKind::Pip),
        "Pip should now be the companion");
    assert!(h.game.sparky_parked, "Sparky parks when an NPC takes over");

    // Walk back to the Annex door and warp out — the companion from a brand-new
    // map gets the same post-warp snap as any other buddy.
    h.walk_to(4, 5);
    h.step_through_portal(KeyCode::Down, "dev");
    assert_eq!(h.game.map.id, "dev");

    let c = h.game.companion.as_ref().expect("Pip should survive the warp");
    let dist = (c.entity.tile_x as i32 - h.game.player.tile_x as i32).abs()
        + (c.entity.tile_y as i32 - h.game.player.tile_y as i32).abs();
    assert!(dist <= 1,
        "Pip should snap adjacent after warping off the new map; player ({},{}), Pip ({},{})",
        h.game.player.tile_x, h.game.player.tile_y, c.entity.tile_x, c.entity.tile_y);
    assert!(c.pathing.as_ref().unwrap().is_empty(),
        "Pip's path trail should be cleared by the warp");
}

#[test]
fn pushing_an_npc_through_a_new_maps_portal_transfers_them() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;
    use macroquad::prelude::KeyCode;

    // The NPC-portal handler is map-agnostic: shoving Pip onto the Annex's exit
    // door carries her to the dev map's offstage roster, no annex-specific code.
    let mut h = Harness::new(9);
    h.start_dev_game();
    h.game.map = Map::annex();
    h.game.npcs = npc_mod::npcs_for_map("annex");
    h.game.npcs_offstage.clear();

    let pip_idx = h.game.npcs.iter().position(|n| n.kind == NpcKind::Pip).unwrap();
    {
        let n = &mut h.game.npcs[pip_idx];
        n.entity.tile_x = 4; n.entity.tile_y = 5;
        n.entity.x = 4.0 * 48.0; n.entity.y = 5.0 * 48.0;
        n.entity.target_x = n.entity.x; n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.wander_cooldown = 9999.0;
    }

    // Keep Sparky out of the way.
    h.game.sparky.entity.tile_x = 1; h.game.sparky.entity.tile_y = 1;
    h.game.sparky.entity.x = 48.0; h.game.sparky.entity.y = 48.0;
    h.game.sparky.entity.target_x = h.game.sparky.entity.x; h.game.sparky.entity.target_y = h.game.sparky.entity.y;
    h.game.sparky.entity.moving = false;

    // Player above Pip at (4,4), pushing Down toward the door at (4,6).
    h.game.player.tile_x = 4; h.game.player.tile_y = 4;
    h.game.player.x = 4.0 * 48.0; h.game.player.y = 4.0 * 48.0;
    h.game.player.target_x = h.game.player.x; h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;

    for _ in 0..12 { h.hold(KeyCode::Down); }
    h.advance(20);

    assert_eq!(h.game.map.id, "annex", "player should stay on the annex; only Pip crosses");
    assert!(!h.game.npcs.iter().any(|n| n.kind == NpcKind::Pip),
        "Pip should have left the annex roster after being pushed through");
    let dev = h.game.npcs_offstage.get("dev")
        .expect("dev stash should exist after pushing Pip through the annex door");
    assert!(dev.iter().any(|n| n.kind == NpcKind::Pip),
        "Pip should now live in the dev roster; got {:?}",
        dev.iter().map(|n| n.kind).collect::<Vec<_>>());
}

// ─── Coral reef: paid dive portal + gate-guardian shark ──────────────────

/// Helpers shared by the reef tests: park Sparky out of the way and snap the
/// player to a tile without animating there.
fn park_sparky(h: &mut Harness) {
    h.game.sparky.entity.tile_x = 1;
    h.game.sparky.entity.tile_y = 1;
    h.game.sparky.entity.x = 48.0;
    h.game.sparky.entity.y = 48.0;
    h.game.sparky.entity.target_x = 48.0;
    h.game.sparky.entity.target_y = 48.0;
    h.game.sparky.entity.moving = false;
}

fn snap_player(h: &mut Harness, tx: usize, ty: usize) {
    h.game.player.tile_x = tx;
    h.game.player.tile_y = ty;
    h.game.player.x = tx as f32 * 48.0;
    h.game.player.y = ty as f32 * 48.0;
    h.game.player.target_x = h.game.player.x;
    h.game.player.target_y = h.game.player.y;
    h.game.player.moving = false;
}

#[test]
fn diving_into_the_reef_costs_dum_dums() {
    use macroquad::prelude::KeyCode;
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;

    let mut h = Harness::new(11);
    h.start_dev_game(); // 20 dum dums
    // Warp to the overworld grass just below the reef dive spot (17,15).
    h.game.map = Map::overworld();
    h.game.npcs = npc_mod::npcs_for_map("overworld");
    h.game.npcs_offstage.clear();
    h.game.companion = None;
    park_sparky(&mut h);
    snap_player(&mut h, 17, 16);

    // Too poor to dive: Sparky gently turns us back, no transfer.
    h.game.dum_dums = 1;
    let mark = h.mark();
    h.hold(KeyCode::Up);     // step onto the dive tile
    h.advance(24);           // finish the step + portal check
    assert_eq!(h.game.map.id, "overworld", "can't dive without paying the toll");
    h.finish_dialogue();     // dismiss Sparky's "we need more Dum Dums"

    // Now we can afford it — first dive charges the toll.
    snap_player(&mut h, 17, 16);
    h.game.dum_dums = 10;
    h.step_through_portal(KeyCode::Up, "reef");
    assert_eq!(h.game.map.id, "reef", "paying the toll opens the dive");
    assert_eq!(h.game.dum_dums, 7, "the first dive spends the 3-Dum-Dum toll");
    assert!(
        h.events_since(mark).iter().any(|e| matches!(e, GameEvent::DumDumsSpent { .. })),
        "a paid dive emits DumDumsSpent; got {:?}", h.events_since(mark),
    );
    h.finish_dialogue(); // dismiss the "we're underwater!" entry beat

    // The toll is ONE-TIME: warp back to the dive spot and dive again — free now.
    h.game.map = Map::overworld();
    h.game.npcs = npc_mod::npcs_for_map("overworld");
    h.game.npcs_offstage.clear();
    h.game.companion = None;
    park_sparky(&mut h);
    snap_player(&mut h, 17, 16);
    let before_second = h.game.dum_dums; // 7
    h.step_through_portal(KeyCode::Up, "reef");
    assert_eq!(h.game.map.id, "reef", "the unlocked dive still works");
    assert_eq!(h.game.dum_dums, before_second, "a second dive is free (one-time toll)");
}

#[test]
fn solving_the_reef_shark_opens_the_passage() {
    use macroquad::prelude::KeyCode;
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;

    let mut h = Harness::new(5);
    h.start_dev_game();
    // Warp straight into the reef's lower lagoon.
    h.game.map = Map::reef();
    h.game.npcs = npc_mod::npcs_for_map("reef");
    h.game.npcs_offstage.clear();
    h.game.companion = None;
    park_sparky(&mut h);
    // Freeze ambient critters so they don't wander into the path mid-walk.
    for n in h.game.npcs.iter_mut() { n.wander_cooldown = 9999.0; }
    snap_player(&mut h, 8, 9);

    // Chompy starts as a closed gate plugging the only gap in the coral wall.
    assert!(
        h.game.npcs.iter().any(|n| n.kind == NpcKind::ReefShark && n.gate),
        "the shark starts as a closed gate",
    );

    // Approach the shark from below (the gap's only reachable side) and face up.
    h.walk_to(8, 6);
    h.hold(KeyCode::Up);
    let mark = h.mark();
    h.interact();                                   // gate dialogue + pending challenge
    h.finish_dialogue();                            // dialogue → challenge
    h.wait_until(|g| g.state == GameState::Challenge);
    h.answer_correctly();
    h.wait_until(|g| g.state == GameState::Playing);

    assert!(
        h.events_since(mark).iter().any(|e| matches!(e, GameEvent::GateOpened { .. })),
        "solving the shark emits GateOpened; got {:?}", h.events_since(mark),
    );
    assert!(
        h.game.npcs.iter().any(|n| n.kind == NpcKind::ReefShark && !n.gate),
        "the shark steps aside once its puzzle is solved",
    );
    assert!(
        h.game.gate_is_solved("reef_gate_1"),
        "the solved gate is recorded so it stays open across sessions",
    );
}

#[test]
fn pushing_an_npc_into_an_unvisited_map_keeps_its_regular_residents() {
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;
    use macroquad::prelude::KeyCode;

    // Regression: shoving an NPC into a map whose roster was never stashed must
    // NOT replace that map's residents with just the intruder. Push Pip
    // annex->dev (dev has no stash yet), then follow her in — dev must still
    // have its own regulars alongside Pip.
    let mut h = Harness::new(9);
    h.start_dev_game();
    h.game.map = Map::annex();
    h.game.npcs = npc_mod::npcs_for_map("annex");
    h.game.npcs_offstage.clear();
    h.game.companion = None;

    let pip_idx = h.game.npcs.iter().position(|n| n.kind == NpcKind::Pip).unwrap();
    {
        let n = &mut h.game.npcs[pip_idx];
        n.entity.tile_x = 4; n.entity.tile_y = 5;
        n.entity.x = 4.0 * 48.0; n.entity.y = 5.0 * 48.0;
        n.entity.target_x = n.entity.x; n.entity.target_y = n.entity.y;
        n.entity.moving = false;
        n.wander_cooldown = 9999.0;
    }
    park_sparky(&mut h);
    snap_player(&mut h, 4, 4);

    // Shove Pip down through the annex door at (4,6).
    for _ in 0..12 { h.hold(KeyCode::Down); }
    h.advance(20);
    assert!(!h.game.npcs.iter().any(|n| n.kind == NpcKind::Pip),
        "Pip should have been pushed off the annex");

    // Follow her through the same door into dev.
    h.step_through_portal(KeyCode::Down, "dev");
    assert_eq!(h.game.map.id, "dev");

    // Dev's own residents must still spawn (the bug wiped them)...
    assert!(h.game.npcs.iter().any(|n| n.kind == NpcKind::Sage),
        "dev's regular residents must still be present; got {:?}",
        h.game.npcs.iter().map(|n| n.kind).collect::<Vec<_>>());
    // ...with the pushed-in Pip joining them.
    assert!(h.game.npcs.iter().any(|n| n.kind == NpcKind::Pip),
        "the pushed-in Pip should be present on dev too");
}

// ─── Space: launchpad, rocket fuel, depot refill ─────────────────────────

#[test]
fn launching_from_the_lab_reaches_the_orbital_hub() {
    use macroquad::prelude::KeyCode;
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;

    let mut h = Harness::new(3);
    h.start_dev_game();
    h.game.map = Map::lab();
    h.game.npcs = npc_mod::npcs_for_map("lab");
    h.game.npcs_offstage.clear();
    h.game.companion = None;
    park_sparky(&mut h);

    // Stand just below Gizmo's launchpad at (10,2) and blast off.
    snap_player(&mut h, 10, 3);
    let fuel_before = h.game.fuel();
    h.step_through_portal(KeyCode::Up, "space_hub");
    assert_eq!(h.game.map.id, "space_hub", "the launchpad blasts off to the hub");
    assert_eq!(h.game.fuel(), fuel_before, "launching to the hub is free");

    // The Moon is a free hop from the hub.
    h.finish_dialogue(); // "BLAST OFF!" entry beat
    snap_player(&mut h, 3, 3); // just below the Moon pad at (3,2)
    h.step_through_portal(KeyCode::Up, "moon");
    assert_eq!(h.game.map.id, "moon");
    assert_eq!(h.game.fuel(), fuel_before, "the Moon hop costs no fuel");
}

#[test]
fn rocket_jumps_burn_fuel_and_the_depot_refills() {
    use macroquad::prelude::KeyCode;
    use robot_buddy_game::tilemap::Map;
    use robot_buddy_game::npc as npc_mod;

    let mut h = Harness::new(21);
    h.start_dev_game();
    h.game.map = Map::space_hub();
    h.game.npcs = npc_mod::npcs_for_map("space_hub");
    h.game.npcs_offstage.clear();
    h.game.companion = None;
    park_sparky(&mut h);

    // Empty tank: the Mars jump (3 fuel) is gently refused — no transfer.
    h.game.set_fuel(0);
    snap_player(&mut h, 11, 3); // just below the Mars pad at (11,2)
    for _ in 0..24 { h.hold(KeyCode::Up); }
    assert_eq!(h.game.map.id, "space_hub", "an empty tank can't make the Mars jump");
    h.finish_dialogue();

    // Refuel at Tank the fuel droid by solving its puzzle.
    snap_player(&mut h, 12, 8); // just above the depot at (12,9)
    h.hold(KeyCode::Down);      // face the droid
    let mark = h.mark();
    h.interact();
    h.finish_dialogue();        // depot greeting → refill puzzle
    h.wait_until(|g| g.state == GameState::Challenge);
    h.answer_correctly();
    h.wait_until(|g| g.state == GameState::Playing);
    assert_eq!(h.game.fuel(), 10, "solving the depot tops the tank to full");
    assert!(
        h.events_since(mark).iter().any(|e| matches!(e, GameEvent::Refueled { .. })),
        "refueling emits Refueled; got {:?}", h.events_since(mark),
    );

    // Now the Mars jump goes through and burns 3 fuel.
    snap_player(&mut h, 11, 3);
    h.step_through_portal(KeyCode::Up, "mars");
    assert_eq!(h.game.map.id, "mars");
    assert_eq!(h.game.fuel(), 7, "the Mars jump burns 3 fuel");
}
