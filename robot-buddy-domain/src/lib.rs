pub mod types;
pub mod learning;
pub mod challenge;
pub mod economy;

use wasm_bindgen::prelude::*;

// ─── WASM EXPORTS ───────────────────────────────────────

#[wasm_bindgen]
pub fn create_rolling_window(max_size: usize) -> Result<String, JsValue> {
    let window = learning::rolling_window::RollingWindow::new(max_size);
    serde_json::to_string(&window).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn push_window_entry(window_json: &str, entry_json: &str) -> Result<String, JsValue> {
    let window: learning::rolling_window::RollingWindow =
        serde_json::from_str(window_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let entry: learning::rolling_window::WindowEntry =
        serde_json::from_str(entry_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let new_window = window.push(entry);
    serde_json::to_string(&new_window).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn window_accuracy(window_json: &str) -> Result<f64, JsValue> {
    let window: learning::rolling_window::RollingWindow =
        serde_json::from_str(window_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(window.accuracy())
}

#[wasm_bindgen]
pub fn create_operation_stats() -> Result<String, JsValue> {
    let stats = learning::operation_stats::OperationStats::new();
    serde_json::to_string(&stats).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn record_operation(
    stats_json: &str,
    operation: &str,
    correct: bool,
    sub_skill: &str,
) -> Result<String, JsValue> {
    let stats: learning::operation_stats::OperationStats =
        serde_json::from_str(stats_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let op: types::Operation =
        serde_json::from_str(&format!("\"{}\"", operation))
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let sk: Option<types::SubSkill> = if sub_skill.is_empty() {
        None
    } else {
        Some(
            serde_json::from_str(&format!("\"{}\"", sub_skill))
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
        )
    };
    let new_stats = stats.record(op, correct, sk);
    serde_json::to_string(&new_stats).map_err(|e| JsValue::from_str(&e.to_string()))
}
