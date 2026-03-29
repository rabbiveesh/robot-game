# CLI Simulators — Rust Native

## Overview

Two Rust binary targets that exercise the domain directly — no WASM boundary, no JSON serialization, no Node. They compile alongside the domain crate and run as native executables.

```
cargo run --bin simulate -- --profile gifted
cargo run --bin simulate-challenge -- --answer wrong,show-me,correct --trace
```

## 1. Learning Simulator (`src/bin/simulate.rs`)

Replaces `tools/simulate.js`. Simulates a kid going through intake + play session, printing profile evolution.

### Usage

```bash
# Simulate a gifted 4-year-old (fast, mostly correct, gets bored)
cargo run --bin simulate -- --profile gifted

# Simulate a struggling kid (slow, mostly wrong, needs patience)
cargo run --bin simulate -- --profile struggling

# Simulate a 7-year-old
cargo run --bin simulate -- --profile seven-year-old

# 2e kid (high reasoning, slow processing)
cargo run --bin simulate -- --profile 2e

# Custom intake pattern
cargo run --bin simulate -- --intake correct,correct,wrong,correct --speed fast --questions 30

# Just intake, no follow-up questions
cargo run --bin simulate -- --profile gifted --intake-only

# Deterministic (same output every time)
cargo run --bin simulate -- --profile gifted --seed 42
```

### Output

Same format as the old JS simulator — colored terminal output:

```
═══════════════════════════════════════════════════
  INTAKE (Sparky's Calibration)
═══════════════════════════════════════════════════

 Q1  band:3  What is 7 − 5?       ✓  1.4s  → next band: 5
 Q2  band:5  What is 2 × 9?       ✓  1.9s  → next band: 7  [skipped text]
 Q3  band:7  What is 36 − 17?     ✗  2.1s  → next band: 6
 Q4  band:6  What is 43 − 21?     ✓  2.2s  → next band: 8

 Intake result:
   Placed at band: 6 (+/- <50)
   Pace: 0.70   Scaffolding: 0.30
   Promote threshold: 0.65  Stretch threshold: 0.50

═══════════════════════════════════════════════════
  PLAY SESSION (30 challenges)
═══════════════════════════════════════════════════

  #1  add_no_carry   band:6       What is 13 + 1?        ✗   3.1s  sw:0.50
  #2  sub_no_borrow  band:6       What is 27 − 6?        ✓   1.9s  sw:0.50
  ...

═══════════════════════════════════════════════════
  FINAL PROFILE
═══════════════════════════════════════════════════

  Band: 6 (+/- <50)        Questions: 30
  Spread: 0.40             Pace: 0.90        Scaffolding: 0.15
  Frustration events: 1

  Sub-skill breakdown:
    Addition:
      add no carry      80% (4/5)   strength
      add carry         50% (1/2)   developing
    Subtraction:
      sub no borrow     89% (8/9)   strength
      sub borrow        20% (1/5)   growth area
    Multiplication:
      mul trivial       67% (4/6)   developing
```

### Simulated Kid Profiles

Same 4 profiles as the JS version, now as Rust structs:

```rust
struct KidProfile {
    name: &'static str,
    // Probability of correct answer at each band (index 0 = band 1)
    accuracy: [f64; 10],
    // Response time range [min_ms, max_ms]
    speed_fast: (f64, f64),
    speed_normal: (f64, f64),
    boredom_chance: f64,
    skips_text: bool,
}

const GIFTED: KidProfile = KidProfile {
    name: "Gifted 4yo",
    accuracy: [0.99, 0.95, 0.90, 0.85, 0.75, 0.50, 0.30, 0.20, 0.10, 0.05],
    speed_fast: (800.0, 2500.0),
    speed_normal: (1500.0, 4000.0),
    boredom_chance: 0.15,
    skips_text: true,
};

const STRUGGLING: KidProfile = KidProfile {
    name: "Struggling 5yo",
    accuracy: [0.80, 0.60, 0.40, 0.20, 0.10, 0.05, 0.02, 0.01, 0.01, 0.01],
    speed_fast: (4000.0, 8000.0),
    speed_normal: (6000.0, 12000.0),
    boredom_chance: 0.0,
    skips_text: false,
};

// ... SEVEN_YEAR_OLD, TWO_E
```

### Implementation

```rust
// robot-buddy-domain/src/bin/simulate.rs

use robot_buddy_domain::learning::*;
use robot_buddy_domain::types::*;
use rand::SeedableRng;
use rand::rngs::SmallRng;

fn main() {
    let args = parse_args();
    let mut rng = SmallRng::seed_from_u64(args.seed);
    let profile_def = get_profile(&args.profile_name);

    // Run intake
    let intake_result = run_intake(&mut rng, profile_def, args.configured_band);
    let mut profile = learner_profile::LearnerProfile::new();
    profile = learner_profile::learner_reducer(profile, intake_completed_event(&intake_result));

    if args.intake_only { return; }

    // Run N challenges
    for q in 1..=args.questions {
        let challenge_profile = challenge_generator::ChallengeProfile {
            math_band: profile.math_band,
            spread_width: profile.spread_width,
            operation_stats: profile.operation_stats.clone(),
        };
        let challenge = challenge_generator::generate_challenge(&challenge_profile, &mut rng);
        let sim = simulate_answer(profile_def, challenge.sampled_band, &mut rng);

        let event = puzzle_attempted_event(&challenge, &sim, &profile);
        profile = learner_profile::learner_reducer(profile, event);

        print_challenge_line(q, &challenge, &sim, &profile);
    }

    print_final_profile(&profile);
}
```

Direct function calls — no JSON, no WASM boundary, no serialization. The domain types are used natively. `SmallRng` is the same PRNG the WASM bridge uses.

### Cargo.toml addition

```toml
[[bin]]
name = "simulate"
path = "src/bin/simulate.rs"

[[bin]]
name = "simulate-challenge"
path = "src/bin/simulate_challenge.rs"
```

### Package.json script (convenience)

```json
"scripts": {
  "simulate": "cargo run --manifest-path robot-buddy-domain/Cargo.toml --bin simulate --"
}
```

Then: `npm run simulate -- --profile gifted`

## 2. Challenge Simulator (`src/bin/simulate_challenge.rs`)

Replaces the specced `tools/simulate-challenge.js`. Exercises the challenge lifecycle reducer.

### Usage

```bash
# Simulate a single challenge interaction
cargo run --bin simulate-challenge -- --answer correct --cra abstract

# Kid needs hints
cargo run --bin simulate-challenge -- --answer wrong,show-me,wrong,tell-me --cra abstract

# Voice input
cargo run --bin simulate-challenge -- --answer voice:0.6:correct --cra representational

# 20 random challenges with a kid profile
cargo run --bin simulate-challenge -- --profile hesitant --count 20

# Full state trace
cargo run --bin simulate-challenge -- --answer wrong,retry,correct --trace
```

### Action Sequences

Parsed from comma-separated strings:

```
correct              → AnswerSubmitted(correct_answer)
wrong                → AnswerSubmitted(wrong_answer)
show-me              → ShowMe
tell-me              → TellMe
retry                → Retry
voice:0.9:correct    → VoiceListenStart → VoiceResult(confidence=0.9) → AnswerSubmitted(correct)
voice:0.6:correct    → VoiceListenStart → VoiceResult(confidence=0.6) → VoiceConfirm(yes) → AnswerSubmitted(correct)
voice:0.3:wrong      → VoiceListenStart → VoiceResult(confidence=0.3, retry)
```

### Challenge Profiles

```rust
struct ChallengeKidProfile {
    name: &'static str,
    first_attempt_accuracy: f64,
    retry_accuracy: f64,
    uses_show_me: f64,
    uses_tell_me: f64,
    uses_voice: f64,
    voice_confidence: (f64, f64),
}

const CONFIDENT: ChallengeKidProfile = ChallengeKidProfile {
    name: "Confident kid",
    first_attempt_accuracy: 0.85,
    retry_accuracy: 0.95,
    uses_show_me: 0.05,
    uses_tell_me: 0.01,
    uses_voice: 0.3,
    voice_confidence: (0.7, 0.95),
};

// ... HESITANT, EXPLORER, FRUSTRATED
```

### Composition

The two simulators compose via JSON piping:

```bash
# Learning sim outputs JSON lines
cargo run --bin simulate -- --profile gifted --output json | \
  cargo run --bin simulate-challenge -- --stdin --profile hesitant

# Or combined mode
cargo run --bin simulate -- --profile gifted --challenge-detail --challenge-profile hesitant
```

The `--challenge-detail` flag runs both simulators internally — learning domain picks the problem, challenge domain simulates the interaction.

## Files

```
robot-buddy-domain/
  src/
    bin/
      simulate.rs              # Learning simulator
      simulate_challenge.rs    # Challenge lifecycle simulator
  Cargo.toml                   # [[bin]] targets added
```

## What to delete

```
tools/simulate.js              # Replaced by cargo run --bin simulate
docs/cli-simulator-spec.md     # This file replaces it
```

## Acceptance Criteria

1. `cargo run --bin simulate -- --profile gifted --seed 42` produces deterministic colored output
2. All 4 learning profiles (gifted, struggling, seven-year-old, 2e) produce plausible output
3. `cargo run --bin simulate-challenge -- --answer wrong,show-me,correct --trace` shows state transitions
4. All 4 challenge profiles (confident, hesitant, explorer, frustrated) work
5. `--output json` piping works between the two tools
6. `--challenge-detail` combined mode works
7. `tools/simulate.js` deleted
8. No Node dependency for simulation
