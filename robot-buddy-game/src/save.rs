use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use crate::sprites::Dir;
use robot_buddy_domain::learning::learner_profile::LearnerProfile;

/// Persistent save data for one slot.
#[derive(Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub name: String,
    pub gender: Gender,
    pub map_id: String,
    pub player_x: usize,
    pub player_y: usize,
    #[serde(deserialize_with = "deserialize_dir")]
    pub player_dir: Dir,
    pub sparky_x: usize,
    pub sparky_y: usize,
    /// True iff an NPC has replaced Sparky as the buddy and Sparky is waiting
    /// at his home tile. Defaults to false so old saves load Sparky as active.
    #[serde(default)]
    pub sparky_parked: bool,
    /// Legacy field — kept for deserializing old saves. Migrated into `profile` on load.
    #[serde(default)]
    #[serde(skip_serializing)]
    pub(crate) math_band: Option<u8>,
    pub dum_dums: u32,
    #[serde(default)]
    pub pearls: u32,
    pub play_time: f32,
    pub timestamp: u64,
    #[serde(default)]
    pub gifts_given: HashMap<String, u32>,
    #[serde(default = "LearnerProfile::new")]
    pub profile: LearnerProfile,
    /// The NPC currently following the player, if any. Identified by the
    /// stable id string (`NpcKind::as_str()`); position is restored from the
    /// saved tile, all other fields are rebuilt from `npcs_for_map` of the
    /// kind's home map. Older saves without this field deserialize as None.
    #[serde(default)]
    pub companion: Option<CompanionSave>,
    /// Cosmetics bought from Bolt's shop (item ids). Older saves load as empty.
    #[serde(default)]
    pub shop_owned: Vec<String>,
    /// Outfit color picked for the Color Change cosmetic. Older saves load as
    /// the default (the tint Color Change shipped with before the picker).
    #[serde(default = "default_color_choice")]
    pub color_choice: String,
    /// Gate ids the kid has solved (e.g. the reef shark). A solved guardian
    /// stays stepped-aside across sessions. Older saves load as empty.
    #[serde(default)]
    pub satisfied_gates: Vec<String>,
    /// Destination maps whose one-time entry toll is already paid (e.g. the
    /// reef dive). Older saves load as empty. Reusable for any paid portal.
    #[serde(default)]
    pub paid_tolls: Vec<String>,
    /// Rocket fuel for space jumps. Older saves load with a full tank.
    #[serde(default = "default_fuel")]
    pub fuel: u32,
}

fn default_fuel() -> u32 { 10 }

fn default_color_choice() -> String {
    crate::sprites::player::OUTFIT_COLORS[0].0.to_string()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CompanionSave {
    pub kind: String,
    pub home_map: String,
    pub tile_x: usize,
    pub tile_y: usize,
}

impl SaveData {
    /// Migrate legacy saves: if `math_band` was present but profile is default, apply it.
    pub fn migrate_legacy(&mut self) {
        if let Some(band) = self.math_band.take() {
            if self.profile.math_band == 1 && band != 1 {
                self.profile.math_band = band;
            }
        }
    }
}

/// Deserialize Dir from either the enum name ("Up") or legacy u8 (0).
fn deserialize_dir<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Dir, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DirOrU8 {
        Dir(Dir),
        Legacy(u8),
    }
    match DirOrU8::deserialize(d)? {
        DirOrU8::Dir(dir) => Ok(dir),
        DirOrU8::Legacy(v) => Ok(Dir::from_u8(v)),
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Gender {
    Boy,
    Girl,
}

impl SaveData {
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

// ─── BACKEND ────────────────────────────────────────────
//
// Production runs against `LocalStorageBackend` (browser localStorage on WASM,
// /tmp files on native dev). Tests construct an `InMemoryBackend` so each
// `Game` owns isolated storage — no /tmp races, no cross-test contamination.

pub trait SaveBackend {
    fn load_all(&self) -> SaveSlots;
    fn save_to(&self, slot: usize, data: &SaveData);
    fn delete(&self, slot: usize);
    /// True when the host wants the game to flush state right now (browser tab
    /// becoming hidden). Non-browser backends always return false.
    fn is_page_hidden(&self) -> bool { false }
}

pub struct LocalStorageBackend;

impl SaveBackend for LocalStorageBackend {
    fn load_all(&self) -> SaveSlots {
        let json = read_storage(STORAGE_KEY);
        if let Some(json) = json {
            let mut slots: SaveSlots = serde_json::from_str(&json).unwrap_or([None, None, None]);
            for slot in slots.iter_mut() {
                if let Some(ref mut save) = slot {
                    save.migrate_legacy();
                }
            }
            slots
        } else {
            [None, None, None]
        }
    }

    fn save_to(&self, slot: usize, data: &SaveData) {
        let mut slots = self.load_all();
        if slot < 3 {
            let mut data = data.clone();
            data.timestamp = current_timestamp();
            slots[slot] = Some(data);
            let json = serde_json::to_string(&slots).unwrap();
            write_storage(STORAGE_KEY, &json);
        }
    }

    fn delete(&self, slot: usize) {
        let mut slots = self.load_all();
        if slot < 3 {
            slots[slot] = None;
            let json = serde_json::to_string(&slots).unwrap();
            write_storage(STORAGE_KEY, &json);
        }
    }

    fn is_page_hidden(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        { unsafe { page_is_hidden() != 0 } }
        #[cfg(not(target_arch = "wasm32"))]
        { false }
    }
}

#[derive(Default)]
pub struct InMemoryBackend {
    slots: RefCell<SaveSlots>,
}

impl SaveBackend for InMemoryBackend {
    fn load_all(&self) -> SaveSlots {
        self.slots.borrow().clone()
    }

    fn save_to(&self, slot: usize, data: &SaveData) {
        if slot >= 3 { return; }
        let mut data = data.clone();
        data.timestamp = 0;
        self.slots.borrow_mut()[slot] = Some(data);
    }

    fn delete(&self, slot: usize) {
        if slot >= 3 { return; }
        self.slots.borrow_mut()[slot] = None;
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
    fn page_is_hidden() -> i32;
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
