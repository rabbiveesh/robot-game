use serde::{Deserialize, Serialize};

use crate::logic::shooter::WaveRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reward {
    pub reward_type: String,
    pub amount: u32,
}

/// A Dum Dum is EARNED by showing you know it: solved, with zero mistakes.
/// Completion always celebrates regardless (rule 7 — wrong answers are never
/// punished), but currency must not pay for exhausting the choices — a
/// four-year-old will happily grind any reward that pays for guessing.
/// `mistakes` is the activity's own error count before the solve: wrong
/// guesses on a challenge/balance/pattern, constraint violations on a grid.
/// This is THE payout rule; every solvable activity routes through it.
pub fn determine_reward(correct: bool, mistakes: u32) -> Option<Reward> {
    if correct && mistakes == 0 {
        Some(Reward { reward_type: "dum_dum".into(), amount: 1 })
    } else {
        None
    }
}

/// Waves in a shooter run that were paired off with no wrong pairs.
pub fn clean_waves(cleared: &[WaveRecord]) -> u32 {
    cleared.iter().filter(|w| w.is_clean()).count() as u32
}

/// What a *finished* number-bond shooter run pays: the usual one Dum Dum for
/// finishing (a bond hunt is trial-and-error, so mis-pairs never void it), plus
/// one extra for every clean wave — a wave paired off with no wrong pairs.
/// Nothing about speed, ever (Invariant 4): `WaveRecord::secs` is silent
/// assessment only and never touches the payout.
pub fn shooter_payout(cleared: &[WaveRecord]) -> u32 {
    let base = determine_reward(true, 0).map_or(0, |r| r.amount);
    base + clean_waves(cleared)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wave(misses: u32, secs: f32) -> WaveRecord {
        WaveRecord { wave: 0, target: 10, secs, hits: 3, misses }
    }

    #[test]
    fn a_shooter_run_pays_one_plus_one_per_clean_wave() {
        assert_eq!(shooter_payout(&[wave(0, 9.0), wave(0, 12.0), wave(0, 30.0)]), 1 + 3);
        assert_eq!(shooter_payout(&[wave(1, 9.0), wave(0, 12.0), wave(0, 30.0)]), 1 + 2);
        assert_eq!(shooter_payout(&[wave(2, 9.0), wave(5, 12.0), wave(1, 30.0)]), 1,
            "mis-pairs never void the base payout for finishing");
    }

    #[test]
    fn a_slow_clean_wave_pays_the_same_as_a_fast_one() {
        // Invariant 4: no speed component, however long the kid took.
        assert_eq!(shooter_payout(&[wave(0, 2.0)]), shooter_payout(&[wave(0, 600.0)]));
    }

    #[test]
    fn clean_first_try_returns_reward() {
        let r = determine_reward(true, 0).unwrap();
        assert_eq!(r.reward_type, "dum_dum");
        assert_eq!(r.amount, 1);
    }

    #[test]
    fn wrong_returns_none() {
        assert!(determine_reward(false, 0).is_none());
    }

    #[test]
    fn guessed_down_solve_pays_nothing() {
        // Right answer, but only after wrong tries — celebrate, don't pay.
        assert!(determine_reward(true, 1).is_none());
        assert!(determine_reward(true, 7).is_none());
    }
}
