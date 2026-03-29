use serde::{Deserialize, Serialize};

use crate::types::{CraStage, Operation, SubSkill};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowEntry {
    pub correct: bool,
    pub operation: Operation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_skill: Option<SubSkill>,
    pub band: u8,
    #[serde(default)]
    pub center_band: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<f64>,
    #[serde(default)]
    pub hint_used: bool,
    #[serde(default)]
    pub told_me: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cra_level_shown: Option<CraStage>,
    #[serde(default)]
    pub boredom: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingWindow {
    pub entries: Vec<WindowEntry>,
    pub max_size: usize,
}

impl RollingWindow {
    pub fn new(max_size: usize) -> Self {
        RollingWindow {
            entries: Vec::new(),
            max_size,
        }
    }

    pub fn from_entries(entries: Vec<WindowEntry>, max_size: usize) -> Self {
        let len = entries.len();
        let start = if len > max_size { len - max_size } else { 0 };
        RollingWindow {
            entries: entries[start..].to_vec(),
            max_size,
        }
    }

    pub fn push(&self, entry: WindowEntry) -> Self {
        let mut entries = self.entries.clone();
        entries.push(entry);
        if entries.len() > self.max_size {
            entries = entries[entries.len() - self.max_size..].to_vec();
        }
        RollingWindow {
            entries,
            max_size: self.max_size,
        }
    }

    pub fn accuracy(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let correct = self.entries.iter().filter(|e| e.correct).count();
        correct as f64 / self.entries.len() as f64
    }

    pub fn avg_response_time(&self) -> f64 {
        let times: Vec<f64> = self
            .entries
            .iter()
            .filter_map(|e| e.response_time_ms)
            .collect();
        if times.is_empty() {
            return 0.0;
        }
        times.iter().sum::<f64>() / times.len() as f64
    }

    pub fn consecutive_wrong(&self) -> usize {
        let mut count = 0;
        for entry in self.entries.iter().rev() {
            if !entry.correct {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    pub fn operation_accuracy(&self, operation: Operation) -> Option<f64> {
        let ops: Vec<&WindowEntry> = self
            .entries
            .iter()
            .filter(|e| e.operation == operation)
            .collect();
        if ops.is_empty() {
            return None;
        }
        let correct = ops.iter().filter(|e| e.correct).count();
        Some(correct as f64 / ops.len() as f64)
    }

    pub fn accuracy_at_band(&self, band: u8) -> (Option<f64>, usize) {
        let at: Vec<&WindowEntry> = self.entries.iter().filter(|e| e.band == band).collect();
        if at.is_empty() {
            return (None, 0);
        }
        let correct = at.iter().filter(|e| e.correct).count();
        (Some(correct as f64 / at.len() as f64), at.len())
    }

    pub fn accuracy_above_band(&self, band: u8) -> (Option<f64>, usize) {
        let above: Vec<&WindowEntry> = self.entries.iter().filter(|e| e.band > band).collect();
        if above.is_empty() {
            return (None, 0);
        }
        let correct = above.iter().filter(|e| e.correct).count();
        (Some(correct as f64 / above.len() as f64), above.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(correct: bool) -> WindowEntry {
        WindowEntry {
            correct,
            operation: Operation::Add,
            sub_skill: None,
            band: 1,
            center_band: 1,
            response_time_ms: Some(2000.0),
            hint_used: false,
            told_me: false,
            cra_level_shown: None,
            boredom: false,
            timestamp: None,
        }
    }

    #[test]
    fn creates_empty_window() {
        let w = RollingWindow::new(20);
        assert!(w.entries.is_empty());
        assert_eq!(w.max_size, 20);
    }

    #[test]
    fn push_and_cap() {
        let mut w = RollingWindow::new(3);
        for i in 0..5 {
            w = w.push(entry(i % 2 == 0));
        }
        assert_eq!(w.entries.len(), 3);
    }

    #[test]
    fn accuracy_empty() {
        assert_eq!(RollingWindow::new(20).accuracy(), 0.0);
    }

    #[test]
    fn accuracy_calculates() {
        let w = RollingWindow::from_entries(
            vec![entry(true), entry(true), entry(false), entry(true), entry(false)],
            20,
        );
        assert!((w.accuracy() - 0.6).abs() < 0.01);
    }

    #[test]
    fn consecutive_wrong_from_end() {
        let w = RollingWindow::from_entries(
            vec![entry(true), entry(false), entry(false), entry(false)],
            20,
        );
        assert_eq!(w.consecutive_wrong(), 3);
    }

    #[test]
    fn consecutive_wrong_zero_when_last_correct() {
        let w = RollingWindow::from_entries(
            vec![entry(false), entry(false), entry(true)],
            20,
        );
        assert_eq!(w.consecutive_wrong(), 0);
    }

    #[test]
    fn operation_accuracy_filters() {
        let mut entries = vec![entry(true), entry(false), entry(true)];
        entries[1].operation = Operation::Sub;
        let w = RollingWindow::from_entries(entries, 20);
        assert!((w.operation_accuracy(Operation::Add).unwrap() - 1.0).abs() < 0.01);
        assert!((w.operation_accuracy(Operation::Sub).unwrap() - 0.0).abs() < 0.01);
        assert!(w.operation_accuracy(Operation::Divide).is_none());
    }

    #[test]
    fn accuracy_at_band() {
        let mut entries = vec![entry(true), entry(false), entry(true)];
        entries[0].band = 5;
        entries[1].band = 5;
        entries[2].band = 6;
        let w = RollingWindow::from_entries(entries, 20);
        let (acc, count) = w.accuracy_at_band(5);
        assert_eq!(count, 2);
        assert!((acc.unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn accuracy_above_band() {
        let mut entries = vec![entry(true), entry(true), entry(false)];
        entries[0].band = 5;
        entries[1].band = 6;
        entries[2].band = 7;
        let w = RollingWindow::from_entries(entries, 20);
        let (acc, count) = w.accuracy_above_band(5);
        assert_eq!(count, 2);
        assert!((acc.unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn avg_response_time() {
        let mut entries = vec![entry(true), entry(true), entry(true)];
        entries[0].response_time_ms = Some(1000.0);
        entries[1].response_time_ms = Some(3000.0);
        entries[2].response_time_ms = Some(2000.0);
        let w = RollingWindow::from_entries(entries, 20);
        assert!((w.avg_response_time() - 2000.0).abs() < 0.01);
    }

    #[test]
    fn immutability() {
        let w1 = RollingWindow::new(20);
        let w2 = w1.push(entry(true));
        assert!(w1.entries.is_empty()); // original unchanged
        assert_eq!(w2.entries.len(), 1);
    }
}
