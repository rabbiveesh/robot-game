use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{BTreeMap, HashMap};
use crate::sprites::Dir;
use robot_buddy_domain::learning::learner_profile::LearnerProfile;
use robot_buddy_domain::economy::wardrobe::{self, Wardrobe};
use robot_buddy_domain::types::GamePace;

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
    /// Legacy field — cosmetics the kid owned, back when only the kid could
    /// wear them. Migrated into `wardrobe` on load (only when the save has no
    /// wardrobe of its own).
    ///
    /// Still *written* for one release, as a mirror of what the kid is
    /// wearing, so that rolling back to a build that predates the wardrobe
    /// keeps the kid's cosmetics. [`encode_save`] fills the mirror; in memory
    /// this field is always empty. Drop the mirror once nobody can roll back
    /// past the wardrobe.
    #[serde(default)]
    pub shop_owned: Vec<String>,
    /// Who's wearing which piece of shop swag — the kid under
    /// `wardrobe::PLAYER`, buddies under their NPC ids. A buddy keeps their
    /// swag while swapped out, so this outlives any one companion.
    #[serde(default)]
    pub wardrobe: Wardrobe,
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
    /// Permanent perks bought at a shop counter (e.g. Hermie's Diving Net).
    /// Unlike swag these are never worn or handed over. Older saves load empty.
    #[serde(default)]
    pub upgrades: Vec<String>,
    /// Arcade pace, set by a parent. Older saves load at the pace the cabinet
    /// shipped with, so nobody's game changes under them.
    #[serde(default)]
    pub game_pace: GamePace,
    /// Secret map ids whose arrival cutscene has already played. The long
    /// "we're UNDERWATER!" speech is a first-time thrill, not a toll on every
    /// dive. Older saves load empty and get one last replay (minus whatever
    /// `migrate_legacy` can infer they've already seen).
    #[serde(default)]
    pub seen_intros: Vec<String>,
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
        // Saves from before swag could change hands: everything the kid owned
        // was, by definition, worn by the kid. A save that already has a
        // wardrobe carries `shop_owned` only as a rollback mirror — the
        // wardrobe is the truth there, so the mirror is just dropped.
        let legacy = std::mem::take(&mut self.shop_owned);
        if self.wardrobe.is_empty() {
            for item in &legacy {
                self.wardrobe.put_on(wardrobe::PLAYER, item);
            }
        }
        // Saves from before intros were tracked: infer what's already been
        // seen so a veteran diver doesn't sit through the reef speech again.
        // A paid toll means they've been there; so does standing there now.
        if self.seen_intros.is_empty() {
            for map in self.paid_tolls.iter().chain(std::iter::once(&self.map_id)) {
                if !self.seen_intros.contains(map) {
                    self.seen_intros.push(map.clone());
                }
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

/// The localStorage key holding all three slots as one JSON array.
pub const STORAGE_KEY: &str = "robotBuddySaves";

/// 3 save slots, each Option<SaveData>.
pub type SaveSlots = [Option<SaveData>; 3];

// ─── SLOT CODEC ─────────────────────────────────────────
//
// Pure functions over strings. On disk the three slots are one JSON array;
// each slot is decoded on its own so one bad slot can never take the other
// two kids' saves down with it. A slot that won't decode is kept as its raw
// JSON text and written back byte-for-byte whenever another slot is saved —
// a later build that fixes the bug reads it again with nothing lost.

/// What one slot holds on disk.
#[derive(Clone)]
#[allow(clippy::large_enum_variant)] // three of these, ever
pub enum StoredSlot {
    Empty,
    Save(SaveData),
    /// Present, but this build can't read it (corrupt field, schema from a
    /// newer or broken build, ...). Never shown to the kid as "empty" and
    /// never dropped.
    Unreadable(UnreadableSlot),
}

/// A slot this build can't read, kept exactly as found.
#[derive(Clone, Debug, PartialEq)]
pub struct UnreadableSlot {
    /// The slot's JSON text, byte-for-byte as it sat in storage.
    pub raw: String,
    /// The kid's name, if the slot is still JSON with a `name` string —
    /// lets the title screen say whose file is resting.
    pub name: Option<String>,
    /// Why it didn't decode. For logs, never for the kid.
    pub reason: String,
}

/// The result of decoding the whole storage value.
pub struct DecodedSaves {
    pub slots: [StoredSlot; 3],
    /// Set when the value as a whole isn't a readable slot array (not JSON,
    /// not an array, or more than three entries). The blob must be backed up
    /// before the key is next written, since the rewrite can't preserve it.
    pub unreadable_file: Option<UnreadableSlot>,
}

fn empty_slots() -> [StoredSlot; 3] {
    [StoredSlot::Empty, StoredSlot::Empty, StoredSlot::Empty]
}

fn unreadable(raw: &str, reason: String) -> UnreadableSlot {
    let name = serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string))
        .filter(|n| !n.trim().is_empty());
    UnreadableSlot { raw: raw.to_string(), name, reason }
}

/// Decode one slot's JSON: `null` is empty, otherwise a save (migrated) or,
/// failing that, the untouched raw text.
pub fn decode_slot(raw: &str) -> StoredSlot {
    if raw.trim() == "null" {
        return StoredSlot::Empty;
    }
    match serde_json::from_str::<SaveData>(raw) {
        Ok(mut save) => {
            save.migrate_legacy();
            StoredSlot::Save(save)
        }
        Err(e) => StoredSlot::Unreadable(unreadable(raw, e.to_string())),
    }
}

/// Decode the whole storage value. Never fails: anything unreadable is
/// reported, not discarded.
pub fn decode_saves(json: &str) -> DecodedSaves {
    let entries: Vec<&serde_json::value::RawValue> = match serde_json::from_str(json) {
        Ok(entries) => entries,
        Err(e) => {
            return DecodedSaves {
                slots: empty_slots(),
                unreadable_file: Some(unreadable(json, format!("save file: {e}"))),
            };
        }
    };
    let mut slots = empty_slots();
    for (slot, raw) in slots.iter_mut().zip(&entries) {
        *slot = decode_slot(raw.get());
    }
    let unreadable_file = (entries.len() > 3).then(|| {
        unreadable(json, format!("save file has {} slots, expected 3", entries.len()))
    });
    DecodedSaves { slots, unreadable_file }
}

/// One save as it goes to disk. Fills the `shop_owned` rollback mirror with
/// what the kid is wearing, so a pre-wardrobe build still dresses them.
pub fn encode_save(save: &SaveData) -> String {
    let mut disk = save.clone();
    disk.shop_owned = save.wardrobe.worn_by(wardrobe::PLAYER).iter().cloned().collect();
    serde_json::to_string(&disk).expect("SaveData always serializes")
}

/// The whole storage value. Unreadable slots go back exactly as they came.
pub fn encode_saves(slots: &[StoredSlot; 3]) -> String {
    let parts: Vec<String> = slots
        .iter()
        .map(|slot| match slot {
            StoredSlot::Empty => "null".to_string(),
            StoredSlot::Save(save) => encode_save(save),
            StoredSlot::Unreadable(u) => u.raw.clone(),
        })
        .collect();
    format!("[{}]", parts.join(","))
}

/// Where an unreadable blob is copied for a parent (or a fix) to recover.
/// `which` is the slot index, or `file` for a whole unreadable value.
/// Keyed by content hash, not time: the WASM build has no wall clock, and a
/// content key makes the backup idempotent — the same bytes are backed up
/// once, however many times the title screen reloads.
pub fn backup_key(which: &str, raw: &str) -> String {
    format!("{STORAGE_KEY}.corrupt.{which}.{:016x}", fnv1a64(raw.as_bytes()))
}

/// FNV-1a: tiny, dependency-free, and stable across Rust releases (unlike
/// `DefaultHasher`), so a backup key never changes under a rebuild.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// ─── RAW STORAGE ────────────────────────────────────────
//
// Both backends run the same slot logic over a string key-value store, so
// what the tests exercise is what ships.

/// A string key-value store (browser localStorage, a /tmp dir, a map).
pub trait RawStorage {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str);
    /// Seconds since the epoch, or 0 where there's no clock.
    fn now(&self) -> u64 { 0 }
}

fn warn(msg: &str) {
    macroquad::logging::warn!("{msg}");
}

/// Copy an unreadable blob to its backup key unless it's already there.
/// Returns true iff the backup exists afterwards.
fn ensure_backup(store: &(impl RawStorage + ?Sized), which: &str, u: &UnreadableSlot) -> bool {
    let key = backup_key(which, &u.raw);
    if store.get(&key).as_deref() == Some(u.raw.as_str()) {
        return true;
    }
    store.set(&key, &u.raw);
    let ok = store.get(&key).as_deref() == Some(u.raw.as_str());
    if ok {
        warn(&format!("save {which} is unreadable ({}); kept in place and backed up to {key}", u.reason));
    } else {
        warn(&format!("save {which} is unreadable ({}) and could not be backed up to {key}", u.reason));
    }
    ok
}

/// Read and decode the slots, backing up anything unreadable. The flag is
/// false when a whole-file blob couldn't be backed up — writing the key then
/// would destroy it, so callers must not write.
fn read_slots(store: &(impl RawStorage + ?Sized)) -> ([StoredSlot; 3], bool) {
    let Some(json) = store.get(STORAGE_KEY) else {
        return (empty_slots(), true);
    };
    let decoded = decode_saves(&json);
    let mut safe_to_write = true;
    if let Some(blob) = &decoded.unreadable_file {
        safe_to_write = ensure_backup(store, "file", blob);
    }
    for (i, slot) in decoded.slots.iter().enumerate() {
        if let StoredSlot::Unreadable(u) = slot {
            ensure_backup(store, &i.to_string(), u);
        }
    }
    (decoded.slots, safe_to_write)
}

/// Replace one slot, leaving the other two exactly as they were.
fn write_slot(store: &(impl RawStorage + ?Sized), slot: usize, new: StoredSlot) {
    if slot >= 3 { return; }
    let (mut slots, safe_to_write) = read_slots(store);
    if !safe_to_write {
        warn("not saving: the unreadable save file has no backup yet");
        return;
    }
    if let StoredSlot::Unreadable(u) = &slots[slot] {
        // The title screen shouldn't offer this slot, but if something
        // writes here anyway the original must already be safe elsewhere.
        if !ensure_backup(store, &slot.to_string(), u) {
            warn(&format!("not saving slot {slot}: its unreadable save has no backup"));
            return;
        }
        warn(&format!("slot {slot} was unreadable; replacing it (original kept at {})",
            backup_key(&slot.to_string(), &u.raw)));
    }
    slots[slot] = new;
    store.set(STORAGE_KEY, &encode_saves(&slots));
}

fn save_slot(store: &(impl RawStorage + ?Sized), slot: usize, data: &SaveData) {
    let mut data = data.clone();
    data.timestamp = store.now();
    write_slot(store, slot, StoredSlot::Save(data));
}

// ─── BACKEND ────────────────────────────────────────────
//
// Production runs against `LocalStorageBackend` (browser localStorage on WASM,
// /tmp files on native dev). Tests construct an `InMemoryBackend` so each
// `Game` owns isolated storage — no /tmp races, no cross-test contamination.
// Both go through the same codec above.

pub trait SaveBackend {
    /// Every slot as stored, including ones this build can't read.
    fn load_slots(&self) -> [StoredSlot; 3];
    fn save_to(&self, slot: usize, data: &SaveData);
    fn delete(&self, slot: usize);
    /// True when the host wants the game to flush state right now (browser tab
    /// becoming hidden). Non-browser backends always return false.
    fn is_page_hidden(&self) -> bool { false }

    /// The readable saves. An unreadable slot reads as `None` here — use
    /// [`SaveBackend::unreadable_slots`] to tell it apart from a free one.
    fn load_all(&self) -> SaveSlots {
        self.load_slots().map(|slot| match slot {
            StoredSlot::Save(save) => Some(save),
            StoredSlot::Empty | StoredSlot::Unreadable(_) => None,
        })
    }

    /// Slots that hold a save this build can't read. The title screen must
    /// not offer these as free.
    fn unreadable_slots(&self) -> [Option<UnreadableSlot>; 3] {
        self.load_slots().map(|slot| match slot {
            StoredSlot::Unreadable(u) => Some(u),
            _ => None,
        })
    }
}

pub struct LocalStorageBackend;

impl RawStorage for LocalStorageBackend {
    fn get(&self, key: &str) -> Option<String> { read_storage(key) }
    fn set(&self, key: &str, value: &str) { write_storage(key, value) }
    fn now(&self) -> u64 { current_timestamp() }
}

impl SaveBackend for LocalStorageBackend {
    fn load_slots(&self) -> [StoredSlot; 3] { read_slots(self).0 }
    fn save_to(&self, slot: usize, data: &SaveData) { save_slot(self, slot, data) }
    fn delete(&self, slot: usize) { write_slot(self, slot, StoredSlot::Empty) }

    fn is_page_hidden(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        { unsafe { page_is_hidden() != 0 } }
        #[cfg(not(target_arch = "wasm32"))]
        { false }
    }
}

/// Test backend: a private string map standing in for localStorage. Runs the
/// exact same encode/decode/migrate/backup path as production.
///
/// Clones share one store, like two tabs on one localStorage — hand a clone
/// to `Game::with_backend` and keep one to inspect what got written.
#[derive(Default, Clone)]
pub struct InMemoryBackend {
    store: Rc<RefCell<BTreeMap<String, String>>>,
}

impl InMemoryBackend {
    /// Start from a given storage value, as if a browser already had it.
    pub fn with_raw_saves(json: &str) -> Self {
        let b = Self::default();
        b.set(STORAGE_KEY, json);
        b
    }

    /// The raw value under `key` (the saves live under [`STORAGE_KEY`]).
    pub fn raw(&self, key: &str) -> Option<String> {
        self.store.borrow().get(key).cloned()
    }

    /// Every key in storage, sorted.
    pub fn keys(&self) -> Vec<String> {
        self.store.borrow().keys().cloned().collect()
    }
}

impl RawStorage for InMemoryBackend {
    fn get(&self, key: &str) -> Option<String> { self.raw(key) }
    fn set(&self, key: &str, value: &str) {
        self.store.borrow_mut().insert(key.to_string(), value.to_string());
    }
}

impl SaveBackend for InMemoryBackend {
    fn load_slots(&self) -> [StoredSlot; 3] { read_slots(self).0 }
    fn save_to(&self, slot: usize, data: &SaveData) { save_slot(self, slot, data) }
    fn delete(&self, slot: usize) { write_slot(self, slot, StoredSlot::Empty) }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A save written before intros were tracked still shouldn't replay the
    /// reef speech: a paid dive toll (and the map you're standing on) prove
    /// you've already been there.
    #[test]
    fn legacy_save_infers_which_intros_were_already_seen() {
        let json = r#"{
            "version": 1, "name": "Ari", "gender": "Girl",
            "map_id": "trench", "player_x": 3, "player_y": 4, "player_dir": "Down",
            "sparky_x": 3, "sparky_y": 5,
            "dum_dums": 7, "play_time": 120.0, "timestamp": 0,
            "paid_tolls": ["reef"]
        }"#;
        let mut save: SaveData = serde_json::from_str(json).expect("legacy save should load");
        assert!(save.seen_intros.is_empty(), "old saves have no intro list");

        save.migrate_legacy();
        assert!(save.seen_intros.contains(&"reef".to_string()),
            "a paid dive toll means the reef intro already played");
        assert!(save.seen_intros.contains(&"trench".to_string()),
            "standing in the trench means its intro already played");
    }
}
