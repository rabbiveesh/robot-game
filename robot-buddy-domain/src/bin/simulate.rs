//! CLI Intake Simulator — simulates a kid going through intake + challenges.
//! Usage: cargo run --bin simulate -- --profile gifted --seed 42

use rand::rngs::SmallRng;
use rand::SeedableRng;
use robot_buddy_domain::learning::challenge_generator::{generate_challenge, ChallengeProfile};
use robot_buddy_domain::learning::frustration_detector::detect_frustration;
use robot_buddy_domain::learning::intake_assessor::{
    generate_intake_question, next_intake_band, process_intake_results, IntakeAnswer,
};
use robot_buddy_domain::learning::learner_profile::{learner_reducer, LearnerEvent, LearnerProfile};
use robot_buddy_domain::types::Operation;
use std::env;

const BAND_NAMES: &[&str] = &[
    "", "Add <5", "+/- <10", "+/- <15", "+/- <20", "x1 x2",
    "+/- <50", "+/- <100", "x1-5", "x1-12", "Divide",
];

struct KidProfile {
    name: &'static str,
    accuracy: [f64; 11], // index 0 unused, 1-10
    speed_fast: (f64, f64),
    speed_normal: (f64, f64),
    #[allow(dead_code)] boredom_chance: f64,
}

const PROFILES: &[(&str, KidProfile)] = &[
    ("gifted", KidProfile {
        name: "Gifted 4yo",
        accuracy: [0.0, 0.99, 0.95, 0.90, 0.85, 0.75, 0.50, 0.30, 0.20, 0.10, 0.05],
        speed_fast: (800.0, 2500.0),
        speed_normal: (1500.0, 4000.0),
        boredom_chance: 0.15,
    }),
    ("struggling", KidProfile {
        name: "Struggling 5yo",
        accuracy: [0.0, 0.80, 0.60, 0.40, 0.20, 0.10, 0.05, 0.02, 0.01, 0.01, 0.01],
        speed_fast: (4000.0, 8000.0),
        speed_normal: (6000.0, 12000.0),
        boredom_chance: 0.0,
    }),
    ("seven-year-old", KidProfile {
        name: "Typical 7yo",
        accuracy: [0.0, 0.99, 0.98, 0.95, 0.92, 0.85, 0.80, 0.70, 0.55, 0.35, 0.20],
        speed_fast: (1000.0, 3000.0),
        speed_normal: (2000.0, 5000.0),
        boredom_chance: 0.10,
    }),
    ("2e", KidProfile {
        name: "2e kid (high reasoning, slow processing)",
        accuracy: [0.0, 0.95, 0.95, 0.90, 0.88, 0.80, 0.75, 0.65, 0.50, 0.30, 0.15],
        speed_fast: (5000.0, 9000.0),
        speed_normal: (7000.0, 15000.0),
        boredom_chance: 0.05,
    }),
];

fn simulate_answer(kid: &KidProfile, band: u8, rng: &mut SmallRng) -> (bool, f64) {
    use rand::Rng;
    let acc = kid.accuracy[band as usize];
    let correct = rng.gen::<f64>() < acc;
    let range = if correct { kid.speed_fast } else { kid.speed_normal };
    let rt = range.0 + rng.gen::<f64>() * (range.1 - range.0);
    (correct, rt)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut profile_name = "gifted";
    let mut seed: u64 = 42;
    let mut questions: usize = 30;
    let mut intake_only = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--profile" => { i += 1; profile_name = Box::leak(args[i].clone().into_boxed_str()); }
            "--seed" => { i += 1; seed = args[i].parse().unwrap_or(42); }
            "--questions" => { i += 1; questions = args[i].parse().unwrap_or(30); }
            "--intake-only" => { intake_only = true; }
            _ => {}
        }
        i += 1;
    }

    let kid = PROFILES.iter().find(|(n, _)| *n == profile_name).map(|(_, p)| p)
        .unwrap_or_else(|| {
            eprintln!("Unknown profile: {}. Available: gifted, struggling, seven-year-old, 2e", profile_name);
            std::process::exit(1);
        });

    let mut rng = SmallRng::seed_from_u64(seed);
    println!("\n\x1b[1m\x1b[36mSimulating: {}\x1b[0m  (seed: {}, questions: {})\n",
        kid.name, seed, if intake_only { "intake only".to_string() } else { questions.to_string() });

    // ─── INTAKE ──────────────────────────────────────
    println!("\x1b[1m\x1b[33m{}\x1b[0m", "═".repeat(50));
    println!("\x1b[1m\x1b[33m  INTAKE\x1b[0m");
    println!("\x1b[1m\x1b[33m{}\x1b[0m\n", "═".repeat(50));

    let mut current_band: u8 = 3;
    let mut intake_answers = Vec::new();

    for q in 0..4u8 {
        let challenge = generate_intake_question(current_band, q as usize, &mut rng);
        let (correct, rt) = simulate_answer(kid, current_band, &mut rng);
        intake_answers.push(IntakeAnswer {
            band: current_band,
            correct,
            response_time_ms: Some(rt),
            skipped_text: false,
        });
        let mark = if correct { "\x1b[32m✓\x1b[0m" } else { "\x1b[31m✗\x1b[0m" };
        let next = next_intake_band(current_band, correct, 10);
        println!("  Q{}  band:{}  {:20}  {}  {:.1}s  → next: {}",
            q + 1, current_band, challenge.question, mark, rt / 1000.0, next);
        current_band = next;
    }

    let intake_result = process_intake_results(&intake_answers, None);
    println!("\n  \x1b[1mPlaced at band: {} ({})\x1b[0m", intake_result.math_band,
        BAND_NAMES.get(intake_result.math_band as usize).unwrap_or(&"?"));
    println!("  Pace: {:.2}  Scaffolding: {:.2}", intake_result.pace, intake_result.scaffolding);

    if intake_only { return; }

    // ─── PLAY SESSION ────────────────────────────────
    let mut profile = LearnerProfile::new();
    profile = learner_reducer(profile, LearnerEvent::IntakeCompleted {
        math_band: intake_result.math_band,
        pace: intake_result.pace,
        scaffolding: intake_result.scaffolding,
        promote_threshold: intake_result.promote_threshold,
        stretch_threshold: intake_result.stretch_threshold,
        text_speed: intake_result.text_speed,
    });

    println!("\n\x1b[1m\x1b[33m{}\x1b[0m", "═".repeat(50));
    println!("\x1b[1m\x1b[33m  PLAY SESSION ({} challenges)\x1b[0m", questions);
    println!("\x1b[1m\x1b[33m{}\x1b[0m\n", "═".repeat(50));

    let mut total_correct = 0u32;
    let mut frustration_events = 0u32;

    for q in 1..=questions {
        let cp = ChallengeProfile {
            math_band: profile.math_band,
            spread_width: profile.spread_width,
            operation_stats: profile.operation_stats.clone(),
        };
        let challenge = generate_challenge(&cp, &mut rng);
        let sampled = challenge.sampled_band;
        let (correct, rt) = simulate_answer(kid, sampled, &mut rng);
        if correct { total_correct += 1; }

        let prev_band = profile.math_band;
        let center = profile.math_band;
        profile = learner_reducer(profile, LearnerEvent::PuzzleAttempted {
            correct,
            operation: challenge.operation,
            sub_skill: challenge.sub_skill,
            band: sampled,
            center_band: Some(center),
            response_time_ms: Some(rt),
            hint_used: false,
            told_me: false,
            cra_level_shown: None,
            timestamp: None,
        });

        // Frustration check
        let frust = detect_frustration(&profile.rolling_window, &[]);
        if frust.level == robot_buddy_domain::types::FrustrationLevel::High {
            profile = learner_reducer(profile, LearnerEvent::FrustrationDetected { level: "high".into() });
            frustration_events += 1;
        }

        let mark = if correct { "\x1b[32m✓\x1b[0m" } else { "\x1b[31m✗\x1b[0m" };
        let skill = challenge.sub_skill.map(|s| format!("{:?}", s)).unwrap_or_else(|| format!("{:?}", challenge.operation));
        let band_tag = if profile.math_band > prev_band {
            format!("  \x1b[42m\x1b[1m ⬆ band:{} \x1b[0m", profile.math_band)
        } else if profile.math_band < prev_band {
            format!("  \x1b[41m\x1b[1m ⬇ band:{} \x1b[0m", profile.math_band)
        } else { String::new() };

        println!("  {:>3}  {:14} band:{}  {:22}  {}  {:>5.1}s  sw:{:.2}{}",
            q, skill, sampled, challenge.question, mark, rt / 1000.0,
            profile.spread_width, band_tag);
    }

    // ─── FINAL PROFILE ───────────────────────────────
    println!("\n\x1b[1m\x1b[33m{}\x1b[0m", "═".repeat(50));
    println!("\x1b[1m\x1b[33m  FINAL PROFILE\x1b[0m");
    println!("\x1b[1m\x1b[33m{}\x1b[0m\n", "═".repeat(50));

    println!("  Band: \x1b[1m{}\x1b[0m ({})  Questions: {}",
        profile.math_band, BAND_NAMES.get(profile.math_band as usize).unwrap_or(&"?"), questions);
    println!("  Spread: {:.2}  Pace: {:.2}  Scaffolding: {:.2}",
        profile.spread_width, profile.pace, profile.scaffolding);
    println!("  Frustration events: {}", frustration_events);
    println!("  Overall accuracy: {}% ({}/{})", total_correct * 100 / questions as u32, total_correct, questions);

    println!("\n  \x1b[1mSub-skill breakdown:\x1b[0m");
    for (op, label) in [(Operation::Add, "add"), (Operation::Sub, "sub"),
        (Operation::Multiply, "mult"), (Operation::Divide, "div"), (Operation::NumberBond, "bond")] {
        let s = profile.operation_stats.get_coarse(op);
        if s.attempts > 0 {
            let pct = s.correct * 100 / s.attempts;
            let tag = if pct >= 75 { "\x1b[32mstrength\x1b[0m" }
                else if pct < 50 { "\x1b[31mgrowth\x1b[0m" }
                else { "\x1b[33mdeveloping\x1b[0m" };
            println!("    {:6} {:>3}% ({}/{})  {}", format!("{}:", label), pct, s.correct, s.attempts, tag);
        }
    }
    println!();
}
