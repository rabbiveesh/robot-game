use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::types::{Operation, SubSkill};
use super::operation_stats::OperationStats;

// ─── BAND DISTRIBUTION ──────────────────────────────────

pub fn band_distribution(center_band: u8, spread_width: f64) -> [f64; 10] {
    let sw = spread_width.clamp(0.0, 1.0);
    let center_weight = 0.9 - 0.6 * sw;

    let offsets: [(i8, f64); 4] = [
        (0, center_weight),
        (1, 0.05 + 0.15 * sw),
        (2, (0.1 * sw - 0.005).max(0.0)),
        (3, (0.05 * (sw - 0.5) * 2.0).max(0.0)),
    ];

    let mut raw = [0.0f64; 10];
    let cb = center_band as i8;

    for &(d, base) in &offsets {
        if d == 0 {
            raw[(center_band - 1) as usize] += base;
        } else {
            let per_side = base / 2.0;
            let hi = cb + d;
            let lo = cb - d;
            if hi >= 1 && hi <= 10 {
                raw[(hi - 1) as usize] += per_side;
            } else {
                let fallback = ((cb + (d - 1).max(0)).min(10).max(1) - 1) as usize;
                raw[fallback] += per_side;
            }
            if lo >= 1 && lo <= 10 {
                raw[(lo - 1) as usize] += per_side;
            } else {
                let fallback = ((cb - (d - 1).max(0)).min(10).max(1) - 1) as usize;
                raw[fallback] += per_side;
            }
        }
    }

    let total: f64 = raw.iter().sum();
    if total > 0.0 {
        for v in raw.iter_mut() {
            *v /= total;
        }
    } else {
        raw[(center_band - 1) as usize] = 1.0;
    }
    raw
}

pub fn sample_from_distribution(dist: &[f64; 10], rng: &mut impl Rng) -> u8 {
    let r: f64 = rng.gen();
    let mut cumulative = 0.0;
    for (i, &p) in dist.iter().enumerate() {
        cumulative += p;
        if r < cumulative {
            return (i + 1) as u8;
        }
    }
    10
}

// ─── BAND OPERATIONS ────────────────────────────────────

pub fn band_operations(band: u8) -> &'static [Operation] {
    match band {
        1 => &[Operation::Add],
        2 => &[Operation::Add, Operation::Sub],
        3 => &[Operation::Add, Operation::Sub, Operation::NumberBond],
        4 => &[Operation::Add, Operation::Sub, Operation::NumberBond],
        5 => &[Operation::Multiply],
        6 => &[Operation::Add, Operation::Sub],
        7 => &[Operation::Add, Operation::Sub],
        8 => &[Operation::Multiply],
        9 => &[Operation::Add, Operation::Sub, Operation::Multiply],
        10 => &[Operation::Add, Operation::Sub, Operation::Multiply, Operation::Divide],
        _ => &[Operation::Add],
    }
}

// ─── SUB-SKILL CLASSIFICATION ───────────────────────────

pub fn classify_addition(a: i32, b: i32) -> SubSkill {
    if a < 10 && b < 10 {
        return SubSkill::AddSingle;
    }
    let ones_sum = (a % 10) + (b % 10);
    if ones_sum < 10 {
        return SubSkill::AddNoCarry;
    }
    let tens_sum = (a / 10) + (b / 10) + if ones_sum >= 10 { 1 } else { 0 };
    if tens_sum >= 10 {
        SubSkill::AddCarryTens
    } else {
        SubSkill::AddCarry
    }
}

pub fn classify_subtraction(a: i32, b: i32) -> SubSkill {
    if a < 10 && b < 10 {
        return SubSkill::SubSingle;
    }
    let ones_a = a % 10;
    let ones_b = b % 10;
    if ones_a >= ones_b {
        return SubSkill::SubNoBorrow;
    }
    let tens_a = (a / 10) % 10;
    let tens_b = (b / 10) % 10;
    if tens_a - 1 < tens_b {
        SubSkill::SubBorrowTens
    } else {
        SubSkill::SubBorrow
    }
}

pub fn classify_multiplication(a: i32, b: i32) -> SubSkill {
    let smaller = a.min(b);
    let larger = a.max(b);
    if smaller <= 2 {
        SubSkill::MulTrivial
    } else if smaller <= 5 && larger <= 6 {
        SubSkill::MulEasy
    } else {
        SubSkill::MulHard
    }
}

pub fn classify_division(dividend: i32, divisor: i32) -> SubSkill {
    let answer = dividend / divisor;
    match classify_multiplication(divisor, answer) {
        SubSkill::MulTrivial | SubSkill::MulEasy => SubSkill::DivEasy,
        _ => SubSkill::DivHard,
    }
}

pub fn classify_bond(total: i32, _part: i32) -> SubSkill {
    if total <= 10 {
        SubSkill::BondSmall
    } else {
        SubSkill::BondLarge
    }
}

fn classify_challenge(a: i32, b: i32, operation: Operation) -> Option<SubSkill> {
    match operation {
        Operation::Add => Some(classify_addition(a, b)),
        Operation::Sub => Some(classify_subtraction(a, b)),
        Operation::Multiply => Some(classify_multiplication(a, b)),
        Operation::Divide => Some(classify_division(a, b)),
        Operation::NumberBond => Some(classify_bond(a, b)),
    }
}

// ─── FEATURE EXTRACTION ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Features {
    pub carries: bool,
    pub carries_tens: bool,
    pub borrows: bool,
    pub borrows_tens: bool,
    pub crosses_ten_boundary: bool,
    pub max_digit: u8,
    pub max_digit_gte7: bool,
    pub has_round_number: bool,
    pub near_doubles: bool,
    pub answer_size: i32,
    pub answer_gte10: bool,
    pub answer_gte20: bool,
    pub answer_gte50: bool,
    pub operand_size: i32,
    pub is_square: bool,
    pub has_factor_five: bool,
    pub both_factors_gt5: bool,
}

pub fn extract_features(a: i32, b: i32, operation: Operation, answer: i32) -> Features {
    let ones_a = a % 10;
    let ones_b = b % 10;
    let tens_a = (a / 10) % 10;
    let tens_b = (b / 10) % 10;

    let max_digit = a.to_string().bytes().chain(b.to_string().bytes())
        .map(|c| (c - b'0') as u8)
        .max()
        .unwrap_or(0);

    Features {
        carries: operation == Operation::Add && ones_a + ones_b >= 10,
        carries_tens: operation == Operation::Add && tens_a + tens_b + if ones_a + ones_b >= 10 { 1 } else { 0 } >= 10,
        borrows: operation == Operation::Sub && ones_a < ones_b,
        borrows_tens: operation == Operation::Sub && tens_a - if ones_a < ones_b { 1 } else { 0 } < tens_b,
        crosses_ten_boundary: a / 10 != answer / 10,
        max_digit,
        max_digit_gte7: ones_a.max(ones_b) >= 7,
        has_round_number: ones_a == 0 || ones_b == 0,
        near_doubles: (a - b).abs() <= 2 && operation == Operation::Add,
        answer_size: answer,
        answer_gte10: answer >= 10,
        answer_gte20: answer >= 20,
        answer_gte50: answer >= 50,
        operand_size: a.max(b),
        is_square: (operation == Operation::Multiply || operation == Operation::Divide) && a == b,
        has_factor_five: (operation == Operation::Multiply || operation == Operation::Divide) && (a % 5 == 0 || b % 5 == 0),
        both_factors_gt5: operation == Operation::Multiply && a.min(b) > 5,
    }
}

// ─── NUMBER GENERATION ──────────────────────────────────

const DISPLAY_OP: &[(&str, &str)] = &[("+", "+"), ("-", "\u{2212}"), ("×", "×"), ("÷", "÷")];
const SPEECH_OP: &[(&str, &str)] = &[("+", "plus"), ("-", "minus"), ("×", "times"), ("÷", "divided by")];

fn display_op(op: &str) -> &str {
    DISPLAY_OP.iter().find(|(k, _)| *k == op).map(|(_, v)| *v).unwrap_or(op)
}

fn speech_op(op: &str) -> &str {
    SPEECH_OP.iter().find(|(k, _)| *k == op).map(|(_, v)| *v).unwrap_or(op)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberResult {
    pub a: i32,
    pub b: i32,
    pub answer: i32,
    pub op: String,
    pub format: String, // "standard" or "bond"
    pub bond_total: Option<i32>,
}

pub fn generate_numbers(band: u8, operation: Operation, rng: &mut impl Rng) -> NumberResult {
    let (a, b, answer, op, format, bond_total) = match band {
        1 => {
            let a = rng.gen_range(1..=4);
            let b = rng.gen_range(1..=(5 - a));
            (a, b, a + b, "+", "standard", None)
        }
        2 => {
            let do_sub = operation == Operation::Sub || (operation != Operation::Add && rng.gen::<f64>() < 0.3);
            if do_sub {
                let a = rng.gen_range(3..=9);
                let b = rng.gen_range(1..a);
                (a, b, a - b, "-", "standard", None)
            } else {
                let a = rng.gen_range(1..=7);
                let b = rng.gen_range(1..=(10 - a));
                (a, b, a + b, "+", "standard", None)
            }
        }
        3 => {
            if operation == Operation::NumberBond || (operation != Operation::Add && operation != Operation::Sub && rng.gen::<f64>() < 0.25) {
                let total = rng.gen_range(5..=14);
                let b = rng.gen_range(1..total);
                (total, b, total - b, "+", "bond", Some(total))
            } else if operation == Operation::Sub || rng.gen::<f64>() < 0.4 {
                let a = rng.gen_range(5..=14);
                let b = rng.gen_range(1..=(a - 1).min(8));
                (a, b, a - b, "-", "standard", None)
            } else {
                let a = rng.gen_range(2..=12);
                let b = rng.gen_range(1..=(15 - a));
                (a, b, a + b, "+", "standard", None)
            }
        }
        4 => {
            if operation == Operation::NumberBond || (operation != Operation::Add && operation != Operation::Sub && rng.gen::<f64>() < 0.2) {
                let total = rng.gen_range(10..=19);
                let b = rng.gen_range(1..=(total - 2));
                (total, b, total - b, "+", "bond", Some(total))
            } else if operation == Operation::Sub || rng.gen::<f64>() < 0.45 {
                let a = rng.gen_range(8..=19);
                let b = rng.gen_range(1..=(a - 1).min(10));
                (a, b, a - b, "-", "standard", None)
            } else {
                let a = rng.gen_range(2..=15);
                let b = rng.gen_range(1..=(20 - a));
                (a, b, a + b, "+", "standard", None)
            }
        }
        5 => {
            let multiplier = if rng.gen::<f64>() < 0.4 { 1 } else { 2 };
            let b = rng.gen_range(1..=10);
            (multiplier, b, multiplier * b, "×", "standard", None)
        }
        6 => {
            let do_sub = operation == Operation::Sub || (operation != Operation::Add && rng.gen::<f64>() < 0.45);
            if do_sub {
                let a = rng.gen_range(20..=49);
                let b = rng.gen_range(5..=(a - 5));
                (a, b, a - b, "-", "standard", None)
            } else {
                let a = rng.gen_range(5..=39);
                let b = rng.gen_range(1..=(50 - a - 1));
                (a, b, a + b, "+", "standard", None)
            }
        }
        7 => {
            let do_sub = operation == Operation::Sub || (operation != Operation::Add && rng.gen::<f64>() < 0.45);
            if do_sub {
                let a = rng.gen_range(25..=94);
                let b = rng.gen_range(5..=(a - 5));
                (a, b, a - b, "-", "standard", None)
            } else {
                let a = rng.gen_range(5..=84);
                let b = rng.gen_range(1..=(100 - a - 1));
                (a, b, a + b, "+", "standard", None)
            }
        }
        8 => {
            let a = rng.gen_range(1..=5);
            let b = rng.gen_range(1..=10);
            (a, b, a * b, "×", "standard", None)
        }
        9 => {
            if operation == Operation::Multiply || (operation != Operation::Add && operation != Operation::Sub) {
                let a = rng.gen_range(1..=12);
                let b = rng.gen_range(1..=12);
                (a, b, a * b, "×", "standard", None)
            } else if operation == Operation::Sub {
                let a = rng.gen_range(25..=94);
                let b = rng.gen_range(5..=(a - 5));
                (a, b, a - b, "-", "standard", None)
            } else {
                let a = rng.gen_range(5..=84);
                let b = rng.gen_range(1..=(100 - a - 1));
                (a, b, a + b, "+", "standard", None)
            }
        }
        10 => {
            if operation == Operation::Divide {
                let divisor = rng.gen_range(2..=12);
                let answer = rng.gen_range(1..=12);
                let a = divisor * answer;
                (a, divisor, answer, "÷", "standard", None)
            } else if operation == Operation::Multiply {
                let a = rng.gen_range(1..=12);
                let b = rng.gen_range(1..=12);
                (a, b, a * b, "×", "standard", None)
            } else if operation == Operation::Sub {
                let a = rng.gen_range(25..=94);
                let b = rng.gen_range(5..=(a - 5));
                (a, b, a - b, "-", "standard", None)
            } else {
                let a = rng.gen_range(5..=84);
                let b = rng.gen_range(1..=(100 - a - 1));
                (a, b, a + b, "+", "standard", None)
            }
        }
        _ => (1, 1, 2, "+", "standard", None),
    };

    NumberResult {
        a, b, answer,
        op: op.to_string(),
        format: format.to_string(),
        bond_total,
    }
}

// ─── CHOICE GENERATION ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub text: String,
    pub correct: bool,
}

pub fn make_choices(answer: i32, rng: &mut impl Rng) -> Vec<Choice> {
    let spread = if answer <= 20 { 3 } else if answer <= 50 { 5 } else if answer <= 100 { 10 } else { 15 };
    let mut choices = vec![Choice { text: answer.to_string(), correct: true }];
    let mut wrongs = std::collections::HashSet::new();

    while wrongs.len() < 2 {
        let offset = rng.gen_range(1..=spread);
        let mut wrong = if rng.gen::<f64>() < 0.5 { answer + offset } else { answer - offset };
        if wrong < 0 { wrong = answer + rng.gen_range(1..=spread); }
        if wrong != answer && !wrongs.contains(&wrong) {
            wrongs.insert(wrong);
            choices.push(Choice { text: wrong.to_string(), correct: false });
        }
    }

    // Shuffle
    for i in (1..choices.len()).rev() {
        let j = rng.gen_range(0..=i);
        choices.swap(i, j);
    }
    choices
}

// ─── FULL CHALLENGE ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Challenge {
    pub question: String,
    pub display_text: String,
    pub speech_text: String,
    pub correct_answer: i32,
    pub choices: Vec<Choice>,
    pub operation: Operation,
    pub sub_skill: Option<SubSkill>,
    pub features: Features,
    pub center_band: u8,
    pub sampled_band: u8,
    pub band: u8,
    pub numbers: Numbers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Numbers {
    pub a: i32,
    pub b: i32,
    pub op: String,
}

/// Minimal profile shape needed for challenge generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeProfile {
    pub math_band: u8,
    pub spread_width: f64,
    pub operation_stats: OperationStats,
}

fn pick_operation(available: &[Operation], stats: &OperationStats, rng: &mut impl Rng) -> Operation {
    if available.len() == 1 {
        return available[0];
    }

    let mut with_acc: Vec<(Operation, f64)> = available
        .iter()
        .map(|&op| {
            let s = stats.get_coarse(op);
            let acc = s.accuracy().unwrap_or(0.5);
            (op, acc)
        })
        .collect();
    with_acc.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mid = (with_acc.len() + 1) / 2;
    let strengths = &with_acc[..mid];
    let growth = &with_acc[mid..];

    if !growth.is_empty() && rng.gen::<f64>() < 0.4 {
        growth[rng.gen_range(0..growth.len())].0
    } else {
        strengths[rng.gen_range(0..strengths.len())].0
    }
}

pub fn generate_challenge(profile: &ChallengeProfile, rng: &mut impl Rng) -> Challenge {
    let dist = band_distribution(profile.math_band, profile.spread_width);
    let sampled_band = sample_from_distribution(&dist, rng);

    let available = band_operations(sampled_band);
    let operation = pick_operation(available, &profile.operation_stats, rng);

    let nums = generate_numbers(sampled_band, operation, rng);
    let choices = make_choices(nums.answer, rng);
    let sub_skill = classify_challenge(nums.a, nums.b, operation);
    let features = extract_features(nums.a, nums.b, operation, nums.answer);

    let d_op = display_op(&nums.op);
    let s_op = speech_op(&nums.op);
    let (display_text, speech_text, question) = if nums.format == "bond" {
        let bt = nums.bond_total.unwrap_or(nums.a);
        (
            format!("What {} {} = {}?", d_op, nums.b, bt),
            format!("What {} {} equals {}?", s_op, nums.b, bt),
            format!("What {} {} = {}?", nums.op, nums.b, bt),
        )
    } else {
        (
            format!("What is {} {} {}?", nums.a, d_op, nums.b),
            format!("What is {} {} {}?", nums.a, s_op, nums.b),
            format!("What is {} {} {}?", nums.a, nums.op, nums.b),
        )
    };

    Challenge {
        question,
        display_text,
        speech_text,
        correct_answer: nums.answer,
        choices,
        operation,
        sub_skill,
        features,
        center_band: profile.math_band,
        sampled_band,
        band: sampled_band,
        numbers: Numbers {
            a: nums.a,
            b: nums.b,
            op: nums.op,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn rng(seed: u64) -> SmallRng {
        SmallRng::seed_from_u64(seed)
    }

    fn profile(band: u8) -> ChallengeProfile {
        ChallengeProfile {
            math_band: band,
            spread_width: 0.0,
            operation_stats: OperationStats::new(),
        }
    }

    #[test]
    fn band_1_generates_addition() {
        let mut r = rng(42);
        let c = generate_challenge(&profile(1), &mut r);
        assert_eq!(c.operation, Operation::Add);
        assert!(c.correct_answer <= 5);
        assert!(c.correct_answer > 0);
    }

    #[test]
    fn band_10_generates_all_ops() {
        let mut ops = std::collections::HashSet::new();
        for seed in 0..200 {
            let mut r = rng(seed);
            let c = generate_challenge(&profile(10), &mut r);
            ops.insert(c.operation);
        }
        assert!(ops.contains(&Operation::Add));
        assert!(ops.contains(&Operation::Sub));
        assert!(ops.contains(&Operation::Multiply));
        assert!(ops.contains(&Operation::Divide));
    }

    #[test]
    fn always_3_choices() {
        for band in 1..=10 {
            let mut r = rng(band as u64);
            let c = generate_challenge(&profile(band), &mut r);
            assert_eq!(c.choices.len(), 3);
        }
    }

    #[test]
    fn exactly_one_correct() {
        let mut r = rng(42);
        let c = generate_challenge(&profile(5), &mut r);
        let corrects: Vec<_> = c.choices.iter().filter(|ch| ch.correct).collect();
        assert_eq!(corrects.len(), 1);
        assert_eq!(corrects[0].text.parse::<i32>().unwrap(), c.correct_answer);
    }

    #[test]
    fn deterministic_with_same_seed() {
        let c1 = generate_challenge(&profile(3), &mut rng(123));
        let c2 = generate_challenge(&profile(3), &mut rng(123));
        assert_eq!(c1.question, c2.question);
        assert_eq!(c1.correct_answer, c2.correct_answer);
    }

    #[test]
    fn display_and_speech_text() {
        let mut r = rng(42);
        let c = generate_challenge(&profile(5), &mut r);
        assert!(c.display_text.contains('×'));
        assert!(c.speech_text.contains("times"));
        assert!(!c.speech_text.contains('×'));
    }

    #[test]
    fn classify_addition_cases() {
        assert_eq!(classify_addition(3, 4), SubSkill::AddSingle);
        assert_eq!(classify_addition(23, 14), SubSkill::AddNoCarry);
        assert_eq!(classify_addition(28, 15), SubSkill::AddCarry);
        assert_eq!(classify_addition(85, 47), SubSkill::AddCarryTens);
    }

    #[test]
    fn classify_subtraction_cases() {
        assert_eq!(classify_subtraction(8, 3), SubSkill::SubSingle);
        assert_eq!(classify_subtraction(47, 23), SubSkill::SubNoBorrow);
        assert_eq!(classify_subtraction(42, 17), SubSkill::SubBorrow);
        assert_eq!(classify_subtraction(103, 47), SubSkill::SubBorrowTens);
    }

    #[test]
    fn classify_multiplication_cases() {
        assert_eq!(classify_multiplication(1, 7), SubSkill::MulTrivial);
        assert_eq!(classify_multiplication(3, 4), SubSkill::MulEasy);
        assert_eq!(classify_multiplication(7, 8), SubSkill::MulHard);
    }

    #[test]
    fn max_digit_extracts_individual_digits() {
        let f = extract_features(144, 12, Operation::Divide, 12);
        assert_eq!(f.max_digit, 4);
    }

    #[test]
    fn band_distribution_sums_to_1() {
        for sw in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let dist = band_distribution(5, sw);
            let sum: f64 = dist.iter().sum();
            assert!((sum - 1.0).abs() < 0.01, "spread_width={}, sum={}", sw, sum);
        }
    }

    #[test]
    fn tight_spread_concentrates_at_center() {
        let dist = band_distribution(5, 0.0);
        assert!(dist[4] > 0.85); // band 5 = index 4
    }
}
