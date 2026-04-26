# Challenge Lifecycle — Domain State Machine

The challenge lifecycle (presented → feedback → teaching → complete) is a domain state machine. Single reducer, single source of truth for "what happens next". Lives in `robot-buddy-domain/src/challenge/challenge_state.rs`.

## Why a state machine

Five bugs from playtesting share one root cause: no domain model.

| Bug                                            | Root cause                                                              |
|------------------------------------------------|-------------------------------------------------------------------------|
| Dum Dum awarded on wrong answer                | Reward logic split across 3 presentation paths; 2 had it backwards.     |
| "Hmm, not quite!" persists between challenges  | Feedback was a global; nobody reset it.                                 |
| TTS reads math symbols wrong                   | One string served display + speech with no structured separation.       |
| Voice answers bypassed the reducer             | Adapter patched one path; voice took another.                           |
| Stale voice state between challenges           | Voice state lived on a global mutable challenge object.                 |

All five become impossible when the lifecycle is a reducer that produces every output (display text, speech text, feedback, reward, next phase) and the previous state can't be mutated.

## State shape

```rust
pub struct ChallengeState {
    pub phase: Phase,            // Presented | Feedback | Teaching | Complete
    pub challenge: Challenge,    // from generate_challenge()
    pub context: Context,        // source: Robot/Npc/Chest, npc_name
    pub attempts: u32,
    pub max_attempts: u32,       // 2 wrong → teaching mode
    pub correct: Option<bool>,

    pub question: DisplaySpeech, // { display, speech }
    pub feedback: Option<DisplaySpeech>,
    pub reward: Option<Reward>,

    pub render_hint: RenderHint, // cra_stage, answer_mode, interaction_type
    pub hint_used: bool,
    pub hint_level: u32,
    pub told_me: bool,

    pub voice: VoiceState,       // listening, confirming, retries, last_result, text
}
```

`DisplaySpeech` carries both forms in lockstep — display has `×`/`÷` symbols, speech has `"times"`/`"divided by"`. The TTS layer never transforms strings; the separation happens at the source.

## Actions

| Action                | Effect                                                                                                       |
|-----------------------|--------------------------------------------------------------------------------------------------------------|
| `AnswerSubmitted`     | Correct → `Complete` + reward. Wrong (1st) → `Feedback`. Wrong (2nd) → `Teaching`, reward cleared.           |
| `Retry`               | `Feedback` → `Presented`, feedback cleared.                                                                  |
| `TeachingComplete`    | `Teaching` → `Complete`.                                                                                     |
| `ShowMe`              | Drops `render_hint.cra_stage` one level (abstract → representational → concrete). Sets `hint_used`, `hint_level += 1`. No-op at concrete. |
| `TellMe`              | Phase → `Teaching`, `told_me = true`, reward cleared, render hint forced to concrete, feedback shows answer. |
| `VoiceListenStart`    | `voice.listening = true`, clear voice text.                                                                  |
| `VoiceResult`         | `confidence ≥ 0.8` → ready to submit. `0.5–0.8` → confirming. `< 0.5` or no number → retry.                  |
| `VoiceConfirm(yes)`   | Yes → ready to submit. No → retry.                                                                            |
| `VoiceError(kind)`    | `not-allowed` → mic-blocked text. Otherwise generic retry text.                                              |

Reward invariants enforced by the reducer: `Some` iff `correct == Some(true)`, `None` whenever `correct == Some(false)` or `told_me`.

## Challenge generator: display + speech

The generator produces both forms from structured data — no regex, no post-processing.

```rust
const DISPLAY_OP: [&str; 4] = ["+", "-", "×", "÷"];
const SPEECH_OP:  [&str; 4] = ["plus", "minus", "times", "divided by"];

display_text = format!("What is {a} {} {b}?", DISPLAY_OP[op as usize]);
speech_text  = format!("What is {a} {} {b}?", SPEECH_OP[op as usize]);
```

Number-bond questions follow the same pattern (`What ÷ 4 = 6?` / `What divided by four equals six?`).

## Render hint and pluggable renderers

The challenge reducer doesn't render anything. Every challenge state carries a `RenderHint` that the presentation layer maps to a renderer:

```rust
pub struct RenderHint {
    pub cra_stage: CraStage,             // Concrete | Representational | Abstract
    pub answer_mode: AnswerMode,         // Choice | Eliminate | FreeInput | Voice
    pub interaction_type: InteractionType, // Quiz | Puzzle | Shop | Drag | NumberLine
}
```

Selection rule:

```text
interaction_type == Shop      → ShopRenderer
interaction_type == Puzzle    → PuzzleRenderer
otherwise, branch on cra_stage:
  Concrete         → CraConcreteRenderer (dots, ten-frames, base-10 blocks)
  Representational → CraRepresRenderer  (number line, bar models)
  Abstract         → QuizRenderer       (numerals + multiple choice)
```

Today only `QuizRenderer` exists (`robot-buddy-game/src/ui/challenge.rs`). New renderers slot in via the same interface — read `&ChallengeState`, return click/key actions, never mutate state. The lifecycle reducer is shared.

### Renderer interface (Rust)

```rust
pub trait ChallengeRenderer {
    fn build_layout(&self, state: &ChallengeState, w: f32, h: f32) -> ChallengeLayout;
    fn hit_test(&self, layout: &ChallengeLayout, x: f32, y: f32) -> Option<ChallengeAction>;
    fn handle_key(&self, key: KeyCode, state: &ChallengeState) -> Option<ChallengeAction>;
}
```

Layout is testable (pure function returning bounding rects + flags). Drawing reads the layout and puts pixels. See architecture-spec for the layout/draw split pattern.

### Show-me without renderer thrash

`ShowMe` doesn't swap renderers mid-challenge — it dispatches the action, the reducer drops `render_hint.cra_stage` one level, and on the next frame the presentation picks a renderer that matches the new hint. Transition can be cross-fade. The state machine never knows about rendering.

### Quest puzzles

A quest creates a challenge with `interaction_type: Puzzle` and any CRA stage; `PuzzleRenderer` draws the keypad/door/etc. Lifecycle is identical — `AnswerSubmitted` flows through the same reducer. Same code path for math, just different visuals.

## CRA adaptive feedback loop

Hint and CRA signals flow from challenge → learner profile via `LearnerEvent::PuzzleAttempted`:

```rust
PuzzleAttempted {
    correct: bool,
    operation: Operation,
    band: u8,
    hint_used: bool,
    hint_level: u32,
    told_me: bool,
    cra_level_shown: Option<CraStage>,
    response_time_ms: u32,
    answer_mode: AnswerMode,
    // voice fields when AnswerMode::Voice
}
```

### Learner reducer rules

- **Promote.** 3 consecutive no-hint correct answers at the same CRA stage for an operation → next stage (concrete → representational → abstract). No promotion above abstract.
- **Demote on hint-assisted correct.** If the kid is marked `Abstract` for an operation but used `ShowMe` to drop to `Representational` and got it right, demote the operation to `Representational`. The level they succeed at is their level.
- **Demote on repeated tell-me.** 2 `told_me` events for the same operation in the rolling window → demote to `Concrete`.
- **Streak resets.** Wrong answer or hint-used answer resets the no-hint counter for that operation.

CRA stages are tracked per-operation independently — addition can be `Abstract` while division is `Concrete`.

### Helpers (in domain)

```rust
fn count_consecutive_no_hint_correct(window: &RollingWindow, op: Operation, stage: CraStage) -> u32;
fn count_recent_tell_me(window: &RollingWindow, op: Operation) -> u32;
```

The rolling window must store `hint_used`, `told_me`, and `cra_level_shown` on each entry — they live on events but need to be retained for these queries.

### Closing the loop

1. Challenge created with `render_hint.cra_stage = profile.cra_stages[operation]`.
2. Kid uses show-me → `cra_stage` drops → presentation picks a lower-CRA renderer → kid succeeds.
3. `PuzzleAttempted` fires with `hint_used = true`, `cra_level_shown = Representational` (say).
4. Learner reducer demotes `cra_stages[operation]` to `Representational`.
5. Next challenge for that operation starts at the updated stage.

## Lifecycle ↔ learner boundary

When `phase == Complete`, the application layer:

1. Adds `state.reward` to the player's Dum Dum total (if `Some`).
2. Dispatches `PuzzleAttempted` to the learner reducer with `correct`, `hint_used`, `told_me`, `cra_level_shown`, response time, answer mode, and voice metadata if applicable.
3. Records the event in the session log.

The challenge state then drops out of scope. New challenge → fresh `ChallengeState` → no shared mutable state, no leakage.
