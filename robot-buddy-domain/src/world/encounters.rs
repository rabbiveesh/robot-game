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
//! happens. Challenge encounters then run the normal challenge lifecycle.

use rand::Rng;
use serde::{Deserialize, Serialize};

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
    /// Run the normal challenge lifecycle (CRA, show-me/tell-me).
    Challenge,
    /// Contextual math framed by the scene ("4 spots on each wing…").
    MathSighting { speaker: String, text: String },
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

/// Per-area flavor pool. Each entry is (kind-tag, speaker, text); "dum_dum"
/// yields a free Dum Dum, "sighting" yields contextual math, else plain flavor.
fn flavor_pool(area: &str) -> &'static [(&'static str, &'static str, &'static str)] {
    match area {
        "home" => &[
            ("flavor", "Sparky", "I found a dust bunny under the rug! It's so fluffy!"),
            ("flavor", "Sparky", "Mommy's cookies smell SO GOOD! Can robots eat cookies?"),
            ("dum_dum", "Sparky", "Hey boss, a shiny Dum Dum was hiding by the couch!"),
        ],
        "pond" | "dream" => &[
            ("sighting", "Sparky", "A frog! It jumped 3 times, then 2 more! How many jumps total?"),
            ("flavor", "Sparky", "The fish are swimming in circles. I'm getting dizzy!"),
            ("sighting", "Sparky", "That butterfly has 4 spots on each wing! How many spots in all?"),
        ],
        "reef" => &[
            ("flavor", "Sparky", "BLUB BLUB! Boss, I'm WATERPROOF! Best day EVER!"),
            ("sighting", "Sparky", "A school of 6 little fish! It split into 2 equal groups — how many in each?"),
            ("flavor", "Sparky", "That shark just SMILED at me! I think we're friends now!"),
            ("sighting", "Sparky", "Three crabs, then 4 more skittered out of the coral! How many crabs?"),
            ("dum_dum", "Sparky", "Ooh! A Dum Dum, sealed in a shiny bubble! Pop pop pop!"),
            ("flavor", "Sparky", "A jellyfish drifted by. It wiggled hello! ...I think."),
        ],
        "lab" => &[
            ("flavor", "Sparky", "BZZT! A blinky light! I love blinky lights!"),
            ("dum_dum", "Sparky", "A Dum Dum rolled under the workbench. Score!"),
        ],
        // overworld / unknown areas
        _ => &[
            ("flavor", "Sparky", "BZZZT! A ladybug landed on my antenna!"),
            ("dum_dum", "Sparky", "I found a shiny Dum Dum on the ground!"),
            ("sighting", "Sparky", "Two birds, then three more landed on that branch. How many birds?"),
        ],
    }
}

fn pick_flavor(area: &str, rng: &mut impl Rng) -> EncounterKind {
    let pool = flavor_pool(area);
    let (tag, speaker, text) = pool[rng.gen_range(0..pool.len())];
    match tag {
        "dum_dum" => EncounterKind::FoundDumDum,
        "sighting" => EncounterKind::MathSighting { speaker: speaker.into(), text: text.into() },
        _ => EncounterKind::FlavorDialogue { speaker: speaker.into(), text: text.into() },
    }
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
        for area in ["home", "pond", "reef", "lab", "overworld", "some_new_map"] {
            // Pull several to make sure every entry is constructible and non-empty.
            for _ in 0..50 {
                match pick_flavor(area, &mut r) {
                    EncounterKind::FlavorDialogue { text, .. }
                    | EncounterKind::MathSighting { text, .. } => assert!(!text.is_empty()),
                    EncounterKind::FoundDumDum => {}
                    EncounterKind::Challenge => panic!("flavor pool must never yield Challenge"),
                }
            }
        }
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
