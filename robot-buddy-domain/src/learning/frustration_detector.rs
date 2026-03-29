use serde::{Deserialize, Serialize};

use super::rolling_window::RollingWindow;
use crate::types::FrustrationLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorSignal {
    pub signal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrustrationResult {
    pub level: FrustrationLevel,
    pub recommendation: String,
}

pub fn detect_frustration(window: &RollingWindow, recent_behaviors: &[BehaviorSignal]) -> FrustrationResult {
    // HIGH: 3+ consecutive wrong
    if window.consecutive_wrong() >= 3 {
        return FrustrationResult {
            level: FrustrationLevel::High,
            recommendation: "drop_band".into(),
        };
    }

    // HIGH: rapid_clicking in last 3 behaviors
    let recent_rapid = recent_behaviors
        .iter()
        .rev()
        .take(3)
        .any(|b| b.signal == "rapid_clicking");
    if recent_rapid {
        return FrustrationResult {
            level: FrustrationLevel::High,
            recommendation: "drop_band".into(),
        };
    }

    // HIGH: accuracy < 40% with enough data
    if window.entries.len() >= 5 && window.accuracy() < 0.4 {
        return FrustrationResult {
            level: FrustrationLevel::High,
            recommendation: "switch_to_chat".into(),
        };
    }

    // MILD: long idle after wrong
    if let Some(last) = window.entries.last() {
        if !last.correct {
            if let Some(rt) = last.response_time_ms {
                if rt > 15000.0 {
                    return FrustrationResult {
                        level: FrustrationLevel::Mild,
                        recommendation: "encourage".into(),
                    };
                }
            }
        }
    }

    // MILD: chose easier path twice in a row
    let last_two: Vec<&BehaviorSignal> = recent_behaviors.iter().rev().take(2).collect();
    if last_two.len() == 2 && last_two.iter().all(|b| b.signal == "chose_easier_path") {
        return FrustrationResult {
            level: FrustrationLevel::Mild,
            recommendation: "offer_easier_path".into(),
        };
    }

    FrustrationResult {
        level: FrustrationLevel::None,
        recommendation: "continue".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::rolling_window::{RollingWindow, WindowEntry};
    use crate::types::Operation;

    fn entry(correct: bool) -> WindowEntry {
        WindowEntry {
            correct,
            operation: Operation::Add,
            sub_skill: None,
            band: 1,
            center_band: 1,
            response_time_ms: Some(3000.0),
            hint_used: false,
            told_me: false,
            cra_level_shown: None,
            boredom: false,
            timestamp: None,
        }
    }

    #[test]
    fn high_after_3_consecutive_wrong() {
        let w = RollingWindow::from_entries(
            vec![entry(true), entry(false), entry(false), entry(false)],
            20,
        );
        let r = detect_frustration(&w, &[]);
        assert_eq!(r.level, FrustrationLevel::High);
        assert_eq!(r.recommendation, "drop_band");
    }

    #[test]
    fn high_on_rapid_clicking() {
        let w = RollingWindow::from_entries(vec![entry(true)], 20);
        let behaviors = vec![BehaviorSignal {
            signal: "rapid_clicking".into(),
            timestamp: None,
        }];
        let r = detect_frustration(&w, &behaviors);
        assert_eq!(r.level, FrustrationLevel::High);
    }

    #[test]
    fn high_on_low_accuracy() {
        let w = RollingWindow::from_entries(
            vec![
                entry(false), entry(false), entry(false),
                entry(true), entry(false), entry(true), // 2/6 = 33%
            ],
            20,
        );
        let r = detect_frustration(&w, &[]);
        assert_eq!(r.level, FrustrationLevel::High);
        assert_eq!(r.recommendation, "switch_to_chat");
    }

    #[test]
    fn mild_on_long_idle_after_wrong() {
        let mut e = entry(false);
        e.response_time_ms = Some(20000.0);
        let w = RollingWindow::from_entries(vec![entry(true), e], 20);
        let r = detect_frustration(&w, &[]);
        assert_eq!(r.level, FrustrationLevel::Mild);
        assert_eq!(r.recommendation, "encourage");
    }

    #[test]
    fn none_when_healthy() {
        let w = RollingWindow::from_entries(
            vec![entry(true), entry(true), entry(true), entry(false), entry(true)],
            20,
        );
        let r = detect_frustration(&w, &[]);
        assert_eq!(r.level, FrustrationLevel::None);
        assert_eq!(r.recommendation, "continue");
    }

    #[test]
    fn none_for_empty() {
        let r = detect_frustration(&RollingWindow::new(20), &[]);
        assert_eq!(r.level, FrustrationLevel::None);
    }
}
