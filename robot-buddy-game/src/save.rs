use serde::{Deserialize, Serialize};

/// Persistent save data for one slot.
#[derive(Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub name: String,
    pub gender: Gender,
    pub map_id: String,
    pub player_x: usize,
    pub player_y: usize,
    pub player_dir: u8,
    pub sparky_x: usize,
    pub sparky_y: usize,
    pub math_band: u8,
    pub dum_dums: u32,
    pub play_time: f32,
    pub timestamp: u64,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Gender {
    Boy,
    Girl,
}

impl SaveData {
    pub fn new(name: &str, gender: Gender) -> Self {
        SaveData {
            version: 1,
            name: name.to_string(),
            gender,
            map_id: "overworld".into(),
            player_x: 14,
            player_y: 12,
            player_dir: 1, // down
            sparky_x: 14,
            sparky_y: 13,
            math_band: 1,
            dum_dums: 0,
            play_time: 0.0,
            timestamp: current_timestamp(),
        }
    }

    pub fn play_time_display(&self) -> String {
        let secs = self.play_time as u64;
        let mins = secs / 60;
        let hours = mins / 60;
        if hours > 0 {
            format!("{}h {}m", hours, mins % 60)
        } else {
            format!("{}m {}s", mins, secs % 60)
        }
    }

    pub fn date_display(&self) -> String {
        // Simple: just show "saved" for now — full date formatting needs more deps
        if self.timestamp > 0 { "Saved".into() } else { String::new() }
    }
}

const STORAGE_KEY: &str = "robotBuddySaves";

/// 3 save slots, each Option<SaveData>.
pub type SaveSlots = [Option<SaveData>; 3];

pub fn load_all_slots() -> SaveSlots {
    let json = read_storage(STORAGE_KEY);
    if let Some(json) = json {
        serde_json::from_str(&json).unwrap_or([None, None, None])
    } else {
        [None, None, None]
    }
}

pub fn save_to_slot(slot: usize, data: &SaveData) {
    let mut slots = load_all_slots();
    if slot < 3 {
        let mut data = data.clone();
        data.timestamp = current_timestamp();
        slots[slot] = Some(data);
        let json = serde_json::to_string(&slots).unwrap();
        write_storage(STORAGE_KEY, &json);
    }
}

pub fn delete_slot(slot: usize) {
    let mut slots = load_all_slots();
    if slot < 3 {
        slots[slot] = None;
        let json = serde_json::to_string(&slots).unwrap();
        write_storage(STORAGE_KEY, &json);
    }
}

// ─── PLATFORM STORAGE ───────────────────────────────────

// WASM: uses extern "C" functions provided by the localStorage plugin in index.html.
// Native: uses /tmp/ file storage for dev.

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn ls_get_len(key_ptr: *const u8, key_len: usize) -> i32;
    fn ls_get(key_ptr: *const u8, key_len: usize, buf_ptr: *mut u8, buf_len: usize);
    fn ls_set(key_ptr: *const u8, key_len: usize, val_ptr: *const u8, val_len: usize);
}

#[cfg(target_arch = "wasm32")]
fn read_storage(key: &str) -> Option<String> {
    unsafe {
        let len = ls_get_len(key.as_ptr(), key.len());
        if len < 0 { return None; }
        let len = len as usize;
        let mut buf = vec![0u8; len];
        ls_get(key.as_ptr(), key.len(), buf.as_mut_ptr(), len);
        String::from_utf8(buf).ok()
    }
}

#[cfg(target_arch = "wasm32")]
fn write_storage(key: &str, value: &str) {
    unsafe {
        ls_set(key.as_ptr(), key.len(), value.as_ptr(), value.len());
    }
}

#[cfg(target_arch = "wasm32")]
fn current_timestamp() -> u64 {
    // macroquad's get_time() returns seconds since start, not epoch.
    // For a rough timestamp, use 0 — proper epoch time needs JS interop.
    0
}

#[cfg(not(target_arch = "wasm32"))]
fn read_storage(key: &str) -> Option<String> {
    let path = format!("/tmp/{}.json", key);
    std::fs::read_to_string(&path).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn write_storage(key: &str, value: &str) {
    let path = format!("/tmp/{}.json", key);
    let _ = std::fs::write(&path, value);
}

#[cfg(not(target_arch = "wasm32"))]
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
