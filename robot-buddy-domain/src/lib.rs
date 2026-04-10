pub mod types;
pub mod learning;
pub mod challenge;
pub mod economy;

// ─── WASM BRIDGE EXPORTS ───────────────────────────────
// Only compiled when building for the JS bridge (wasm-pack).
// The macroquad game crate uses domain types directly — no JSON, no bridge.
#[cfg(feature = "wasm-bridge")]
mod wasm_bridge {
    use wasm_bindgen::prelude::*;
    use crate::{learning, challenge, economy};

    // ── Learning: Profile ───────────────────────────────────

    #[wasm_bindgen]
    pub fn create_profile() -> String {
        serde_json::to_string(&learning::learner_profile::LearnerProfile::new()).unwrap()
    }

    #[wasm_bindgen]
    pub fn create_profile_with_overrides(overrides_json: &str) -> Result<String, JsValue> {
        let mut profile = learning::learner_profile::LearnerProfile::new();
        let overrides: serde_json::Value = serde_json::from_str(overrides_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        if let Some(band) = overrides.get("mathBand").and_then(|v| v.as_u64()) {
            profile.math_band = band as u8;
        }
        if let Some(sw) = overrides.get("spreadWidth").and_then(|v| v.as_f64()) {
            profile.spread_width = sw;
        }
        if let Some(p) = overrides.get("pace").and_then(|v| v.as_f64()) {
            profile.pace = p;
        }
        if let Some(s) = overrides.get("scaffolding").and_then(|v| v.as_f64()) {
            profile.scaffolding = s;
        }
        if let Some(pt) = overrides.get("promoteThreshold").and_then(|v| v.as_f64()) {
            profile.promote_threshold = pt;
        }
        if let Some(st) = overrides.get("stretchThreshold").and_then(|v| v.as_f64()) {
            profile.stretch_threshold = st;
        }
        if let Some(ts) = overrides.get("textSpeed").and_then(|v| v.as_f64()) {
            profile.text_speed = ts;
        }
        if let Some(ic) = overrides.get("intakeCompleted").and_then(|v| v.as_bool()) {
            profile.intake_completed = ic;
        }
        Ok(serde_json::to_string(&profile).unwrap())
    }

    #[wasm_bindgen]
    pub fn learner_reducer(state_json: &str, event_json: &str) -> Result<String, JsValue> {
        let state: learning::learner_profile::LearnerProfile = serde_json::from_str(state_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let event: learning::learner_profile::LearnerEvent = serde_json::from_str(event_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let new_state = learning::learner_profile::learner_reducer(state, event);
        Ok(serde_json::to_string(&new_state).unwrap())
    }

    // ── Learning: Challenge Generator ───────────────────────

    #[wasm_bindgen]
    pub fn generate_challenge(profile_json: &str, seed: f64) -> Result<String, JsValue> {
        use rand::SeedableRng;
        let profile: learning::challenge_generator::ChallengeProfile = serde_json::from_str(profile_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed as u64);
        let challenge = learning::challenge_generator::generate_challenge(&profile, &mut rng);
        Ok(serde_json::to_string(&challenge).unwrap())
    }

    // ── Learning: Frustration ───────────────────────────────

    #[wasm_bindgen]
    pub fn detect_frustration(window_json: &str, behaviors_json: &str) -> Result<String, JsValue> {
        let window: learning::rolling_window::RollingWindow = serde_json::from_str(window_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let behaviors: Vec<learning::frustration_detector::BehaviorSignal> = serde_json::from_str(behaviors_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let result = learning::frustration_detector::detect_frustration(&window, &behaviors);
        Ok(serde_json::to_string(&result).unwrap())
    }

    // ── Learning: Intake ────────────────────────────────────

    #[wasm_bindgen]
    pub fn generate_intake_question(current_band: u8, question_index: usize, seed: f64) -> Result<String, JsValue> {
        use rand::SeedableRng;
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed as u64);
        let challenge = learning::intake_assessor::generate_intake_question(current_band, question_index, &mut rng);
        Ok(serde_json::to_string(&challenge).unwrap())
    }

    #[wasm_bindgen]
    pub fn process_intake_results(answers_json: &str, configured_band: i32) -> Result<String, JsValue> {
        let answers: Vec<learning::intake_assessor::IntakeAnswer> = serde_json::from_str(answers_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let cb = if configured_band >= 0 { Some(configured_band as u8) } else { None };
        let result = learning::intake_assessor::process_intake_results(&answers, cb);
        Ok(serde_json::to_string(&result).unwrap())
    }

    #[wasm_bindgen]
    pub fn next_intake_band(current_band: u8, correct: bool, ceiling: u8) -> u8 {
        learning::intake_assessor::next_intake_band(current_band, correct, ceiling)
    }

    // ── Challenge Lifecycle ─────────────────────────────────

    #[wasm_bindgen]
    pub fn challenge_reducer(state_json: &str, action_json: &str) -> Result<String, JsValue> {
        let state: challenge::challenge_state::ChallengeState = serde_json::from_str(state_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let action: challenge::challenge_state::ChallengeAction = serde_json::from_str(action_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let new_state = challenge::challenge_state::challenge_reducer(state, action);
        Ok(serde_json::to_string(&new_state).unwrap())
    }

    // ── Economy ─────────────────────────────────────────────

    #[wasm_bindgen]
    pub fn process_give(dum_dums: u32, recipient_id: &str, gifts_json: &str) -> Result<String, JsValue> {
        let gifts: std::collections::HashMap<String, u32> = serde_json::from_str(gifts_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        match economy::give::process_give(dum_dums, recipient_id, &gifts) {
            Some(result) => Ok(serde_json::to_string(&result).unwrap()),
            None => Ok("null".to_string()),
        }
    }

    #[wasm_bindgen]
    pub fn determine_reward(correct: bool) -> String {
        match economy::rewards::determine_reward(correct) {
            Some(r) => serde_json::to_string(&r).unwrap(),
            None => "null".to_string(),
        }
    }

    #[wasm_bindgen]
    pub fn get_interaction_options(npc_json: &str, player_state_json: &str) -> Result<String, JsValue> {
        let npc: economy::interaction_options::NpcInfo = serde_json::from_str(npc_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let player_state: economy::interaction_options::PlayerState = serde_json::from_str(player_state_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let options = economy::interaction_options::get_interaction_options(&npc, &player_state);
        Ok(serde_json::to_string(&options).unwrap())
    }
}
