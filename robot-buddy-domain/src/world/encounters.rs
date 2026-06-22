//! Random encounters. Pure logic — decides whether/what, not how it's drawn.
//!
//! As the kid explores, the world stays alive: Sparky finds shiny things, a
//! butterfly poses a counting question, a free Dum Dum sits on the path. Most
//! encounters are flavor, not quizzes (the spec targets ~60% flavor / ~40%
//! challenge), and the split is governed by the learner's `challenge_freq` dial
//! so the world never feels like a step-by-step test.
//!
//! The game calls `should_trigger_encounter` after each tile step and, when it
//! fires (and the player has stopped moving), `pick_encounter` to choose what
//! happens. Challenge encounters then run the normal challenge lifecycle —
//! framed by the scene via [`frame_sighting`] so the math reads as part of the
//! world ("A frog hops 3 times, then 2 more — how many hops?") rather than a
//! bare "What is 3 + 2?". The framing is cosmetic: the underlying challenge,
//! choices, and adaptive numbers are the real ones the learner would have got.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::types::Operation;

/// Snapshot the game passes in each time it asks about an encounter.
#[derive(Debug, Clone)]
pub struct EncounterConfig {
    pub steps_since_last_encounter: u32,
    /// Never fire two encounters closer together than this many steps.
    pub min_steps_between: u32,
    /// From the learner profile (0.0..=1.0): higher = more challenge encounters.
    pub challenge_freq: f64,
    /// Current map/area id, used to pick area-appropriate flavor.
    pub area: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum EncounterKind {
    /// Pure ambiance — a line from Sparky, no math, walk away anytime.
    FlavorDialogue { speaker: String, text: String },
    /// A free Dum Dum sitting on the ground — reward for exploring.
    FoundDumDum,
    /// Run the normal challenge lifecycle (CRA, show-me/tell-me). The game
    /// frames the prompt with the scene via [`frame_sighting`].
    Challenge,
}

/// Base odds an encounter fires on an eligible step (1 in 30, per spec).
const BASE_CHANCE: f64 = 1.0 / 30.0;

pub fn should_trigger_encounter(config: &EncounterConfig, rng: &mut impl Rng) -> bool {
    if config.steps_since_last_encounter < config.min_steps_between {
        return false;
    }
    rng.gen::<f64>() < BASE_CHANCE
}

pub fn pick_encounter(config: &EncounterConfig, rng: &mut impl Rng) -> EncounterKind {
    // challenge_freq is the probability this encounter is a challenge; the rest
    // are flavor (with the occasional free Dum Dum or contextual-math sighting).
    if rng.gen::<f64>() < config.challenge_freq.clamp(0.0, 1.0) {
        return EncounterKind::Challenge;
    }
    pick_flavor(&config.area, rng)
}

/// Per-area flavor pool: pure ambiance, no math. Each entry is (kind-tag,
/// speaker, text); "dum_dum" yields a free Dum Dum, else plain flavor. The
/// math moments are no longer in here — they fire as real, scene-framed
/// challenges (see [`pick_encounter`] + [`frame_sighting`]).
fn flavor_pool(area: &str) -> &'static [(&'static str, &'static str, &'static str)] {
    match area {
        "home" => &[
            ("flavor", "Sparky", "I found a dust bunny under the rug! It's so fluffy!"),
            ("flavor", "Sparky", "Mommy's cookies smell SO GOOD! Can robots eat cookies?"),
            ("dum_dum", "Sparky", "Hey boss, a shiny Dum Dum was hiding by the couch!"),
        ],
        "pond" | "dream" => &[
            ("flavor", "Sparky", "The fish are swimming in circles. I'm getting dizzy!"),
            ("flavor", "Sparky", "A dragonfly buzzed my antenna! Rude. ...Cool, but rude."),
            ("dum_dum", "Sparky", "A Dum Dum bobbing in the reeds! Don't worry, I got it!"),
        ],
        "reef" => &[
            ("flavor", "Sparky", "BLUB BLUB! Boss, I'm WATERPROOF! Best day EVER!"),
            ("flavor", "Sparky", "That shark just SMILED at me! I think we're friends now!"),
            ("dum_dum", "Sparky", "Ooh! A Dum Dum, sealed in a shiny bubble! Pop pop pop!"),
            ("flavor", "Sparky", "A jellyfish drifted by. It wiggled hello! ...I think."),
        ],
        "space_hub" | "moon" | "mars" | "asteroid_base" => &[
            ("flavor", "Sparky", "WHEEEE! Zero gravity! My bolts are FLOATING! BEEP BOOP!"),
            ("flavor", "Sparky", "A little green alien just waved at us! Hi, friend! *waves back with all arms*"),
            ("dum_dum", "Sparky", "A Dum Dum, floating in zero-g! I grabbed it before it drifted off!"),
            ("flavor", "Sparky", "Space is SO quiet out here. ...except for me. BLEEP BLOOP!"),
        ],
        "lab" => &[
            ("flavor", "Sparky", "BZZT! A blinky light! I love blinky lights!"),
            ("dum_dum", "Sparky", "A Dum Dum rolled under the workbench. Score!"),
        ],
        // overworld / unknown areas
        _ => &[
            ("flavor", "Sparky", "BZZZT! A ladybug landed on my antenna!"),
            ("dum_dum", "Sparky", "I found a shiny Dum Dum on the ground!"),
            ("flavor", "Sparky", "A breeze! My sensors say it smells like... adventure!"),
        ],
    }
}

fn pick_flavor(area: &str, rng: &mut impl Rng) -> EncounterKind {
    let pool = flavor_pool(area);
    let (tag, speaker, text) = pool[rng.gen_range(0..pool.len())];
    match tag {
        "dum_dum" => EncounterKind::FoundDumDum,
        _ => EncounterKind::FlavorDialogue { speaker: speaker.into(), text: text.into() },
    }
}

// ─── SCENE FRAMING ──────────────────────────────────────
//
// A challenge encounter is dressed in scene words that match the operation and
// numbers the adaptive generator actually produced. The framing is presentation
// only — the choices and correct answer come from the real challenge.

/// A scene-framed prompt for a challenge encounter, ready to drop onto the
/// challenge's display/speech text.
#[derive(Debug, Clone)]
pub struct SightingFrame {
    pub speaker: String,
    pub display_text: String,
    pub speech_text: String,
}

/// One scene template. `prompt` uses `{a}`/`{b}` placeholders (never the answer —
/// that would give it away) and must read as a natural question for `op`.
struct Scene {
    op: Operation,
    prompt: &'static str,
}

/// Scenes that work anywhere — the guaranteed fallback so every operation can
/// always be framed (NumberBond is the deliberate exception; see `frame_sighting`).
const GENERIC_SCENES: &[Scene] = &[
    Scene { op: Operation::Add,      prompt: "Sparky finds {a} gems, then {b} more! How many gems?" },
    Scene { op: Operation::Sub,      prompt: "{a} stars are out, {b} fade away. How many stars left?" },
    Scene { op: Operation::Multiply, prompt: "{a} boxes, {b} gems in each. How many gems?" },
    Scene { op: Operation::Divide,   prompt: "{a} gems, {b} friends, shared fair. How many each?" },
];

/// Area-specific scenes, tried before the generic pool for local color.
fn scene_pool(area: &str) -> &'static [Scene] {
    match area {
        "home" => &[
            Scene { op: Operation::Add,      prompt: "{a} cookies, then {b} more come out! How many?" },
            Scene { op: Operation::Sub,      prompt: "{a} cookies on the plate, you eat {b}! How many left?" },
            Scene { op: Operation::Multiply, prompt: "{a} plates, {b} cookies each. How many cookies?" },
            Scene { op: Operation::Divide,   prompt: "{a} cookies, {b} kids, shared fair. How many each?" },
        ],
        "pond" | "dream" => &[
            Scene { op: Operation::Add,      prompt: "A frog hops {a} times, then {b} more! How many hops?" },
            Scene { op: Operation::Sub,      prompt: "{a} ducks float by, {b} swim away. How many ducks?" },
            Scene { op: Operation::Multiply, prompt: "{a} lily pads, {b} frogs each. How many frogs?" },
            Scene { op: Operation::Divide,   prompt: "{a} frogs, {b} lily pads, shared fair. How many each?" },
        ],
        "reef" => &[
            Scene { op: Operation::Add,      prompt: "{a} fish swim up, then {b} more! How many fish?" },
            Scene { op: Operation::Sub,      prompt: "{a} crabs on the rock, {b} scuttle off. How many left?" },
            Scene { op: Operation::Multiply, prompt: "{a} starfish, {b} arms each. How many arms?" },
            Scene { op: Operation::Divide,   prompt: "{a} fish split into {b} schools. How many each?" },
        ],
        "space_hub" | "moon" | "mars" | "asteroid_base" => &[
            Scene { op: Operation::Add,      prompt: "{a} stars blink on, then {b} more! How many stars?" },
            Scene { op: Operation::Sub,      prompt: "{a} comets zoom by, {b} fly off. How many comets?" },
            Scene { op: Operation::Multiply, prompt: "{a} planets, {b} moons each. How many moons?" },
            Scene { op: Operation::Divide,   prompt: "{a} rocks, {b} aliens, shared fair. How many each?" },
        ],
        "lab" => &[
            Scene { op: Operation::Add,      prompt: "{a} lights blink, then {b} more! How many lights?" },
            Scene { op: Operation::Sub,      prompt: "{a} gadgets on the shelf, {b} fall. How many left?" },
            Scene { op: Operation::Multiply, prompt: "{a} shelves, {b} gadgets each. How many gadgets?" },
            Scene { op: Operation::Divide,   prompt: "{a} bolts, {b} bins, shared fair. How many each?" },
        ],
        _ => &[],
    }
}

/// Dress a generated challenge's numbers in scene words. `a`/`b` are the
/// challenge operands as shown (for division, dividend ÷ divisor). Returns
/// `None` when no scene fits the operation — notably `NumberBond`, whose
/// "what + b = total?" shape doesn't map to a tidy story; the caller keeps the
/// plain prompt in that case.
pub fn frame_sighting(area: &str, op: Operation, a: i32, b: i32, rng: &mut impl Rng) -> Option<SightingFrame> {
    let candidates: Vec<&Scene> = scene_pool(area)
        .iter()
        .chain(GENERIC_SCENES.iter())
        .filter(|s| s.op == op)
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let scene = candidates[rng.gen_range(0..candidates.len())];
    let text = scene
        .prompt
        .replace("{a}", &a.to_string())
        .replace("{b}", &b.to_string());
    Some(SightingFrame {
        speaker: "Sparky".into(),
        display_text: text.clone(),
        speech_text: text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn cfg(steps: u32, freq: f64, area: &str) -> EncounterConfig {
        EncounterConfig {
            steps_since_last_encounter: steps,
            min_steps_between: 15,
            challenge_freq: freq,
            area: area.into(),
        }
    }

    #[test]
    fn never_fires_before_min_steps() {
        let mut r = SmallRng::seed_from_u64(1);
        for _ in 0..1000 {
            assert!(!should_trigger_encounter(&cfg(14, 0.5, "overworld"), &mut r),
                "must not fire back-to-back before min_steps_between");
        }
    }

    #[test]
    fn eventually_fires_once_eligible() {
        let mut r = SmallRng::seed_from_u64(7);
        let fired = (0..2000).any(|_| should_trigger_encounter(&cfg(30, 0.5, "overworld"), &mut r));
        assert!(fired, "an eligible step should fire an encounter within many tries");
    }

    #[test]
    fn challenge_freq_one_always_challenges() {
        let mut r = SmallRng::seed_from_u64(3);
        for _ in 0..200 {
            assert_eq!(pick_encounter(&cfg(30, 1.0, "overworld"), &mut r), EncounterKind::Challenge);
        }
    }

    #[test]
    fn challenge_freq_zero_never_challenges() {
        let mut r = SmallRng::seed_from_u64(3);
        for _ in 0..200 {
            assert_ne!(pick_encounter(&cfg(30, 0.0, "pond"), &mut r), EncounterKind::Challenge);
        }
    }

    #[test]
    fn flavor_pools_cover_known_and_unknown_areas() {
        let mut r = SmallRng::seed_from_u64(9);
        for area in ["home", "pond", "reef", "moon", "space_hub", "lab", "overworld", "some_new_map"] {
            // Pull several to make sure every entry is constructible and non-empty.
            for _ in 0..50 {
                match pick_flavor(area, &mut r) {
                    EncounterKind::FlavorDialogue { text, .. } => assert!(!text.is_empty()),
                    EncounterKind::FoundDumDum => {}
                    EncounterKind::Challenge => panic!("flavor pool is pure ambiance, never a challenge"),
                }
            }
        }
    }

    #[test]
    fn frames_every_operation_in_every_area() {
        use crate::types::Operation::*;
        let mut r = SmallRng::seed_from_u64(11);
        for area in ["home", "pond", "dream", "reef", "moon", "lab", "overworld", "some_new_map"] {
            for op in [Add, Sub, Multiply, Divide] {
                let frame = frame_sighting(area, op, 6, 2, &mut r)
                    .unwrap_or_else(|| panic!("{op:?} should frame in {area}"));
                // The actual operands appear; the answer never does (no give-away).
                assert!(frame.display_text.contains('6') && frame.display_text.contains('2'));
                assert_eq!(frame.display_text, frame.speech_text);
                assert_eq!(frame.speaker, "Sparky");
            }
        }
    }

    #[test]
    fn number_bond_has_no_scene_frame() {
        let mut r = SmallRng::seed_from_u64(13);
        assert!(frame_sighting("pond", crate::types::Operation::NumberBond, 8, 3, &mut r).is_none());
    }

    #[test]
    fn mixed_freq_produces_both_kinds() {
        let mut r = SmallRng::seed_from_u64(42);
        let mut challenges = 0;
        let mut flavor = 0;
        for _ in 0..400 {
            match pick_encounter(&cfg(30, 0.4, "overworld"), &mut r) {
                EncounterKind::Challenge => challenges += 1,
                _ => flavor += 1,
            }
        }
        assert!(challenges > 0 && flavor > 0, "0.4 freq should yield a mix, got c={challenges} f={flavor}");
        assert!(flavor > challenges, "flavor should dominate at freq 0.4");
    }
}
