use serde::{Deserialize, Serialize};

use super::challenge_generator::{generate_challenge, ChallengeProfile, Challenge};
use super::operation_stats::OperationStats;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeAnswer {
    pub band: u8,
    pub correct: bool,
    pub response_time_ms: Option<f64>,
    pub skipped_text: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeResult {
    pub math_band: u8,
    pub pace: f64,
    pub scaffolding: f64,
    pub promote_threshold: f64,
    pub stretch_threshold: f64,
    pub text_speed: f64,
}

pub fn generate_intake_question(current_band: u8, _question_index: usize, rng: &mut impl rand::Rng) -> Challenge {
    let profile = ChallengeProfile {
        math_band: current_band,
        spread_width: 0.0,
        operation_stats: OperationStats::new(),
    };
    generate_challenge(&profile, rng)
}

pub fn process_intake_results(answers: &[IntakeAnswer], configured_band: Option<u8>) -> IntakeResult {
    let mut last_correct_band: u8 = 1;
    for a in answers {
        if a.correct { last_correct_band = a.band; }
    }
    let mut math_band = last_correct_band.max(1).min(10);
    if let Some(cb) = configured_band {
        if cb >= 1 { math_band = math_band.min(cb + 2); }
    }

    let times: Vec<f64> = answers.iter().filter_map(|a| a.response_time_ms).collect();
    let avg_time = if times.is_empty() { 5000.0 } else { times.iter().sum::<f64>() / times.len() as f64 };

    let pace = if avg_time < 3000.0 { 0.7 }
        else if avg_time < 5000.0 { 0.6 }
        else if avg_time > 8000.0 { 0.3 }
        else if avg_time > 6000.0 { 0.4 }
        else { 0.5 };

    let scaffolding = if avg_time > 8000.0 { 0.7 }
        else if avg_time > 6000.0 { 0.6 }
        else if avg_time < 3000.0 { 0.3 }
        else { 0.5 };

    let correct_count = answers.iter().filter(|a| a.correct).count();
    let (promote_threshold, stretch_threshold) = if avg_time < 3000.0 && correct_count >= 3 {
        (0.65, 0.50)
    } else {
        (0.75, 0.60)
    };

    let skipped_count = answers.iter().filter(|a| a.skipped_text).count();
    let text_speed = if skipped_count >= 2 { 0.02 }
        else if skipped_count >= 1 { 0.025 }
        else { 0.035 };

    IntakeResult { math_band, pace, scaffolding, promote_threshold, stretch_threshold, text_speed }
}

pub fn next_intake_band(current_band: u8, correct: bool, ceiling: u8) -> u8 {
    if correct { (current_band + 2).min(ceiling) } else { (current_band.max(1)) - if current_band > 1 { 1 } else { 0 } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_at_last_correct_band() {
        let answers = vec![
            IntakeAnswer { band: 3, correct: true, response_time_ms: Some(2000.0), skipped_text: false },
            IntakeAnswer { band: 5, correct: true, response_time_ms: Some(2500.0), skipped_text: false },
            IntakeAnswer { band: 7, correct: false, response_time_ms: Some(5000.0), skipped_text: false },
            IntakeAnswer { band: 6, correct: true, response_time_ms: Some(3000.0), skipped_text: false },
        ];
        let r = process_intake_results(&answers, None);
        assert_eq!(r.math_band, 6);
    }

    #[test]
    fn clamps_to_configured_band() {
        let answers = vec![
            IntakeAnswer { band: 3, correct: true, response_time_ms: Some(2000.0), skipped_text: false },
            IntakeAnswer { band: 5, correct: true, response_time_ms: Some(2500.0), skipped_text: false },
            IntakeAnswer { band: 7, correct: true, response_time_ms: Some(3000.0), skipped_text: false },
            IntakeAnswer { band: 9, correct: true, response_time_ms: Some(2000.0), skipped_text: false },
        ];
        let r = process_intake_results(&answers, Some(1));
        assert_eq!(r.math_band, 3); // configured(1) + 2
    }

    #[test]
    fn next_band_correct() {
        assert_eq!(next_intake_band(3, true, 10), 5);
        assert_eq!(next_intake_band(9, true, 10), 10);
    }

    #[test]
    fn next_band_wrong() {
        assert_eq!(next_intake_band(3, false, 10), 2);
        assert_eq!(next_intake_band(1, false, 10), 1);
    }
}
