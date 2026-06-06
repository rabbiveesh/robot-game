#[path = "src/logic/patterns.rs"]
mod patterns;

use patterns::{PatternKind, generate_pattern};
use rand::SeedableRng;
use rand::rngs::SmallRng;

fn main() {
    let mut rng = SmallRng::seed_from_u64(42);
    let kind = PatternKind::CountBy { step: -3 };
    let p = generate_pattern(kind, &mut rng);
    println!("Generated puzzle with CountBy step=-3");
    println!("Correct answer: {:?}", p.correct_answer);
    println!("Number of choices: {}", p.choices.len());
}
