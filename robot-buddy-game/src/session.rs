use serde::Serialize;
use std::collections::HashMap;
use robot_buddy_domain::learning::learner_profile::LearnerProfile;

/// A single challenge attempt record for the session log.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeRecord {
    pub question: String,
    pub correct_answer: i32,
    pub player_answer: Option<i32>,
    pub correct: bool,
    pub operation: String,
    pub band: u8,
    pub sampled_band: u8,
    pub hint_used: bool,
    pub told_me: bool,
    pub attempts: u32,
    pub source: String,          // "sparky", "npc", "chest"
    pub play_time_at_event: f32, // seconds since game start
}

/// A give event record.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GiveRecord {
    pub recipient_id: String,
    pub recipient_name: String,
    pub dum_dums_before: u32,
    pub play_time_at_event: f32,
}

/// Session event — tagged union for the export log.
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SessionEvent {
    #[serde(rename = "CHALLENGE_COMPLETED")]
    ChallengeCompleted(ChallengeRecord),
    #[serde(rename = "GIFT_GIVEN")]
    GiftGiven(GiveRecord),
}

/// Accumulates events during a play session.
pub struct SessionLog {
    pub events: Vec<SessionEvent>,
}

impl SessionLog {
    pub fn new() -> Self {
        SessionLog { events: Vec::new() }
    }

    pub fn record_challenge(&mut self, record: ChallengeRecord) {
        self.events.push(SessionEvent::ChallengeCompleted(record));
    }

    pub fn record_give(&mut self, record: GiveRecord) {
        self.events.push(SessionEvent::GiftGiven(record));
    }

    pub fn challenge_count(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, SessionEvent::ChallengeCompleted(_))).count()
    }

    pub fn correct_count(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, SessionEvent::ChallengeCompleted(r) if r.correct)).count()
    }
}

/// The full export payload.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionExport {
    pub export_date: String,
    pub player_name: String,
    pub session_events: Vec<SessionEvent>,
    pub summary: SessionSummary,
    pub metadata: ExportMetadata,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub challenges_attempted: usize,
    pub challenges_correct: usize,
    pub accuracy: f64,
    pub gifts_given: HashMap<String, u32>,
    pub dum_dums_at_export: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportMetadata {
    pub game_version: String,
    pub total_play_time_secs: f32,
    pub math_band: u8,
    pub spread_width: f64,
    pub pace: f64,
    pub scaffolding: f64,
    pub streak: i32,
    pub rolling_window_size: usize,
    pub intake_completed: bool,
    pub map_id: String,
}

/// Build the export JSON string.
pub fn build_export(
    player_name: &str,
    session_log: &SessionLog,
    gifts_given: &HashMap<String, u32>,
    dum_dums: u32,
    play_time: f32,
    profile: &LearnerProfile,
    map_id: &str,
) -> String {
    let attempted = session_log.challenge_count();
    let correct = session_log.correct_count();
    let accuracy = if attempted > 0 { correct as f64 / attempted as f64 } else { 0.0 };

    let export = SessionExport {
        export_date: current_iso_date(),
        player_name: player_name.to_string(),
        session_events: session_log.events.clone(),
        summary: SessionSummary {
            challenges_attempted: attempted,
            challenges_correct: correct,
            accuracy,
            gifts_given: gifts_given.clone(),
            dum_dums_at_export: dum_dums,
        },
        metadata: ExportMetadata {
            game_version: "0.2.0-macroquad".into(),
            total_play_time_secs: play_time,
            math_band: profile.math_band,
            spread_width: profile.spread_width,
            pace: profile.pace,
            scaffolding: profile.scaffolding,
            streak: profile.streak,
            rolling_window_size: profile.rolling_window.entries.len(),
            intake_completed: profile.intake_completed,
            map_id: map_id.to_string(),
        },
    };

    serde_json::to_string_pretty(&export).unwrap_or_else(|_| "{}".into())
}

/// Trigger a file download with the given JSON content.
pub fn download_json(json: &str, filename: &str) {
    platform_download(json, filename);
}

// ─── PLATFORM ───────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn download_file(
        data_ptr: *const u8, data_len: usize,
        name_ptr: *const u8, name_len: usize,
    );
}

#[cfg(target_arch = "wasm32")]
fn platform_download(data: &str, filename: &str) {
    unsafe {
        download_file(
            data.as_ptr(), data.len(),
            filename.as_ptr(), filename.len(),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_download(data: &str, filename: &str) {
    let path = format!("/tmp/{}", filename);
    match std::fs::write(&path, data) {
        Ok(_) => eprintln!("[session export] Wrote {}", path),
        Err(e) => eprintln!("[session export] Failed to write {}: {}", path, e),
    }
}

#[cfg(target_arch = "wasm32")]
fn current_iso_date() -> String {
    // Simple fallback — no js_sys date access without extra deps
    "unknown".into()
}

#[cfg(not(target_arch = "wasm32"))]
fn current_iso_date() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}
