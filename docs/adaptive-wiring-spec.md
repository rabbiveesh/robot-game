# Adaptive System Wiring Spec

The domain has ~1300 lines of adaptive learning logic. The game uses ~30 lines of it. This spec wires the rest.

Priority: **critical** — this is the entire educational value of the game. Without it, challenges are random and difficulty never changes.

## Current State (broken)

What the game does now:
- `math_band` is a bare `u8` chosen once on the title screen, never changes
- `CraStage::Abstract` hardcoded for every operation
- `spread_width` hardcoded at 0.5
- `OperationStats` always empty (fresh every challenge)
- No `LearnerProfile` exists at runtime
- `learner_reducer` never called — challenge results go nowhere
- `RollingWindow` not tracking — no attempt history
- Frustration detector not called — 20 wrong in a row changes nothing
- Intake assessor not used
- Response times not captured

## What Needs to Happen

### 1. LearnerProfile as the single source of truth

Replace the scattered state (`math_band: u8`, hardcoded `spread_width`, empty `OperationStats`) with a `LearnerProfile` that lives for the duration of a game session.

```rust
// In main.rs — replace these:
let mut math_band: u8 = 1;
// with:
let mut profile = LearnerProfile::new();
```

Every place that currently reads `math_band` reads `profile.math_band` instead. Every place that hardcodes `spread_width: 0.5` reads `profile.spread_width`. Every place that creates empty `OperationStats` reads `profile.operation_stats.clone()`.

### 2. Feed challenge results into the reducer

When a challenge completes (correct answer or teaching complete), build a `LearnerEvent::PuzzleAttempted` and call `learner_reducer`.

```rust
// In the Challenge dismiss block (main.rs, where dismiss = true):
if let Some(ref ac) = active_challenge {
    let event = LearnerEvent::PuzzleAttempted {
        correct: ac.state.correct == Some(true),
        operation: ac.challenge.operation,
        sub_skill: ac.challenge.sub_skill,
        band: ac.challenge.sampled_band,
        center_band: Some(ac.challenge.center_band),
        response_time_ms: Some(ac.response_time_ms()),
        hint_used: ac.state.hint_used,
        told_me: ac.state.told_me,
        cra_level_shown: Some(ac.state.render_hint.cra_stage),
        timestamp: Some(game_time as f64 * 1000.0),
    };
    profile = learner_reducer(profile, event);
}
```

This is the critical wiring. Every challenge result now flows through the reducer which updates: band (promote/demote), streak, pace, scaffolding, spread width, CRA stages, rolling window, operation stats. All from one call.

### 3. Capture response time

Add a `start_time` field to `ActiveChallenge`:

```rust
struct ActiveChallenge {
    state: ChallengeState,
    challenge: Challenge,
    choice_bounds: Vec<ChoiceBound>,
    scaffold: ScaffoldBounds,
    complete_timer: f32,
    start_time: f32,  // NEW — game_time when challenge was presented
}
```

Set `start_time: game_time` in `start_challenge`. Calculate response time on answer:

```rust
impl ActiveChallenge {
    fn response_time_ms(&self, current_time: f32) -> f64 {
        ((current_time - self.start_time) as f64 * 1000.0).min(30000.0) // cap at 30s
    }
}
```

The 30s cap prevents AFK inflation — same fix we applied in the JS era.

### 4. Use profile for challenge generation

`make_challenge_profile` reads from the live profile instead of hardcoding:

```rust
fn make_challenge_profile(profile: &LearnerProfile) -> ChallengeProfile {
    ChallengeProfile {
        math_band: profile.math_band,
        spread_width: profile.spread_width,
        operation_stats: profile.operation_stats.clone(),
    }
}
```

And `start_challenge` takes the profile:

```rust
fn start_challenge(rng: &mut SmallRng, profile: &LearnerProfile) -> ActiveChallenge {
    let cp = make_challenge_profile(profile);
    let challenge = generate_challenge(&cp, rng);
    let cra = profile.cra_stages
        .get(&challenge.operation)
        .copied()
        .unwrap_or(CraStage::Concrete);
    // ... use `cra` instead of hardcoded CraStage::Abstract
}
```

Now challenges adapt: band comes from the profile (which the reducer promotes/demotes), spread width widens with confidence, operation weighting uses actual stats, CRA stage is per-operation.

### 5. Frustration detection

Check frustration after each challenge result. If high, fire a `FrustrationDetected` event:

```rust
// After the PuzzleAttempted event above:
let frustration = detect_frustration(&profile.rolling_window, &behavior_signals);
if frustration.level == FrustrationLevel::High {
    profile = learner_reducer(profile, LearnerEvent::FrustrationDetected {
        level: "high".into(),
    });
    // Optional: show encouraging dialogue from Sparky
    dialogue.start(vec![DialogueLine {
        speaker: "Sparky".into(),
        text: "Hey boss, let's try something a little different! BEEP BOOP!".into(),
    }]);
}
```

The reducer handles the rest: drops band by 1, reduces spread, sets wrongs_before_teach to 1.

### 6. Behavior signals

Track basic behavioral signals that feed into frustration detection:

```rust
let mut behavior_signals: Vec<BehaviorSignal> = vec![];
```

Signals to capture:
- **rapid_clicking**: If the kid submits an answer within 1 second of the challenge appearing, that's a click-through, not thinking. Record it.
- **text_skipped**: If the kid advances dialogue before the typewriter finishes (first Space skips to full text), record it. Fire `LearnerEvent::Behavior { signal: "text_skipped" }`.

These are cheap to capture and give the frustration detector real data.

### 7. Persist the profile in save data

Replace `math_band: u8` in `SaveData` with the full `LearnerProfile`:

```rust
pub struct SaveData {
    pub version: u32,
    pub name: String,
    pub gender: Gender,
    pub map_id: String,
    pub player_x: usize,
    pub player_y: usize,
    pub player_dir: Dir,
    pub sparky_x: usize,
    pub sparky_y: usize,
    pub dum_dums: u32,
    pub play_time: f32,
    pub timestamp: u64,
    pub gifts_given: HashMap<String, u32>,
    pub profile: LearnerProfile,  // REPLACES math_band: u8
}
```

`LearnerProfile` is already `Serialize + Deserialize`. The old `math_band` field gets migrated: if loading old data, create a `LearnerProfile::new()` and set its `math_band` from the old value. Serde's `#[serde(default)]` on the profile field handles this — old saves without the field get `LearnerProfile::new()`.

`gather_save_data` clones the profile in. `load_from_save` restores it.

### 8. Intake quiz (medium priority)

Currently the title screen lets parents pick a band manually. The domain has an intake assessor that calibrates automatically.

Flow:
1. New game starts with `intake_completed: false` on the profile
2. Before entering the overworld, run 4-6 intake questions
3. Use `generate_intake_question(band, index, rng)` for each
4. Track answers + response times
5. Call `process_intake_results(answers, configured_band)` → returns calibrated band + dials
6. Fire `LearnerEvent::IntakeCompleted { ... }` to set the profile
7. Proceed to overworld

The manual band picker stays as a parent override / starting hint. The intake assessor uses it as a ceiling (`configured_band + 2` max).

This can be a separate GameState:

```rust
enum GameState {
    Title,
    NewGame,
    Intake,   // NEW — 4-6 assessment questions
    Playing,
    InteractionMenu,
    Dialogue,
    Challenge,
}
```

### 9. Parent debug overlay shows real data

The debug overlay (P key) currently shows hardcoded placeholders. Wire it to the live profile:

```rust
pub fn draw(&self, map_id: &str, tx: usize, ty: usize, 
            dum_dums: u32, play_time: f32, profile: &LearnerProfile) {
    // ...
    draw_text(&format!("Band: {}  Streak: {}", profile.math_band, profile.streak), ...);
    draw_text(&format!("Pace: {:.2}  Scaffolding: {:.2}", profile.pace, profile.scaffolding), ...);
    draw_text(&format!("Spread: {:.2}  Window: {}/20", profile.spread_width, 
        profile.rolling_window.entries.len()), ...);
    draw_text(&format!("Intake: {}", if profile.intake_completed { "done" } else { "pending" }), ...);
    // CRA per operation
    for (op, cra) in &profile.cra_stages {
        draw_text(&format!("  {:?}: {:?}", op, cra), ...);
    }
}
```

## Implementation Order

Do these in sequence. Each one is independently valuable and testable.

1. **Profile as source of truth** — Create `LearnerProfile`, replace hardcoded values. Game behavior doesn't change yet, but the plumbing is in place.
2. **Feed results + response time** — `learner_reducer` called on dismiss. Bands start changing. This is the payoff.
3. **Use profile for generation** — `spread_width` and `operation_stats` now matter. CRA per-operation.
4. **Frustration detection** — 3 wrong in a row drops the band. Kid gets encouraged.
5. **Persist profile** — Save/load preserves all learning state.
6. **Debug overlay** — Parent can see what's happening. Useful for verifying the above works.
7. **Behavior signals** — text_skipped, rapid_clicking feed into frustration and pace.
8. **Intake quiz** — Automatic calibration on new game.

Steps 1-5 are the critical path. Steps 6-8 are polish.

## What NOT to Change

- `challenge_reducer` stays as-is — it handles the UI phase machine (Presented → Feedback → Teaching → Complete). This is presentation logic.
- `learner_reducer` stays as-is — it handles the learning state machine. All 7 tests pass.
- The domain crate gets zero changes. This is purely a wiring task in the game crate.

## Testing

The wiring itself is hard to unit test (it's game loop integration), but you can verify:
- `cargo test` still passes (domain unchanged)
- Play a session, get 5+ correct → band should promote (check P overlay)
- Get 3 wrong in a row → band should drop
- Use "Show me" → CRA should demote for that operation
- Save, reload → profile should be restored (band, window, CRA stages all match)
- Check P overlay shows real data, not placeholder text
