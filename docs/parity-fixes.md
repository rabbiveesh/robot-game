# Parity Fixes

Bugs and refactors to address before nuking the JS.

## 1. Gift history not persisted

`gifts_given: HashMap<String, u32>` in main.rs is never saved or loaded. A kid gives 20 Dum Dums to Sparky, saves, reloads — all gift history is gone. Milestones reset (they get "My FIRST Dum Dum?!" again on reload).

### Fix

Add `gifts_given` to `SaveData` in save.rs:

```rust
pub struct SaveData {
    // ... existing fields ...
    pub gifts_given: HashMap<String, u32>,
}
```

Default it to `HashMap::new()` in `SaveData::new()`.

Write it in `gather_save_data` (main.rs) — pass `&gifts_given` through and clone it into the struct.

Read it in `load_from_save` (main.rs) — restore into the `gifts_given` local.

Serde handles `HashMap<String, u32>` natively. Old saves without this field will deserialize as an empty map (add `#[serde(default)]` on the field to be safe).

## 2. Dead click targets in title screen

title_screen.rs lines 315-323 — the level picker arrow click detection in `draw()` has empty bodies:

```rust
if clicked && mx >= arr_left_x - 20.0 && mx <= arr_left_x + 10.0
    && my >= arr_y && my <= arr_y + arr_h {
    // handled in handle_level_click   <-- does nothing
}
```

The actual click handling is duplicated in `handle_gender_click()` (lines 393-407).

### Fix

1. Remove the empty click checks from `draw()` (lines 315-323).
2. Rename `handle_gender_click` to `handle_form_clicks` since it handles both gender and level picker clicks now.
3. Update the call site in main.rs (`form.handle_gender_click()` → `form.handle_form_clicks()`).

## 3. Tile IDs are magic u8 numbers — should be a Rust enum

`tiles: Vec<Vec<u8>>` with raw numbers everywhere. `is_solid` matches on `2 | 3 | 4 | 6 | 7 | ...`. Chest detection is `== 13`. Dream palette matches on u8. Add a tile type and the compiler can't tell you which match arms you forgot.

### Fix

Create a `Tile` enum in tilemap.rs:

```rust
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Tile {
    Grass, Path, Water, Wall, Tree, Flower,
    HouseWall, Roof, Door, Window, ShopAwning, Sign, Bridge, Chest,
    WoodFloor, Rug, Table, Bookshelf,
    Glitch95, Glitch96, Glitch97, Glitch98, GlitchWall,
}
```

Then:
- `Map.tiles` becomes `Vec<Vec<Tile>>` (or `Map.cells` if doing the zone refactor at the same time)
- `is_solid` → `matches!(tile, Tile::Water | Tile::Wall | Tile::Tree | ...)`
- `tile_color` and `draw_tile_detail` → match on `Tile` variants, compiler enforces exhaustiveness
- Chest detection → `map.tile_at(x, y) == Tile::Chest` instead of `== 13`
- Map data uses `use Tile::*` for compact declarations:
  ```rust
  vec![Tree, Tree, Tree, Grass, Grass, Flower, ...]
  ```

Every non-exhaustive match becomes a compile error. Add a new tile type → the compiler finds every place that needs handling.

## 4. Direction is a magic u8 in portals and save data

`Dir` enum already exists in `sprites/mod.rs`. But `Portal.dir` is `u8` (0=up, 1=down, 2=left, 3=right) and `SaveData.player_dir` is `u8`. Two separate match blocks in main.rs convert between Dir and u8 manually. Add a direction and both match blocks silently do the wrong thing.

### Fix

1. Move `Dir` to a shared location (tilemap.rs or a new `types.rs` in the game crate) so both `save.rs` and `tilemap.rs` can use it without circular deps.
2. Add `#[derive(Serialize, Deserialize)]` to `Dir`.
3. Change `Portal.dir: u8` → `Portal.dir: Dir`.
4. Change `SaveData.player_dir: u8` → `SaveData.player_dir: Dir`.
5. Delete every `match dir { 0 => Dir::Up, 1 => Dir::Down, ... }` block in main.rs — there are at least 3.

Old saves with `player_dir: 1` will need a serde migration. Simplest: keep `#[serde(alias = "1")]` or do a one-time version bump. Or just add `#[serde(try_from = "u8")]` with a `TryFrom<u8>` impl as a fallback for old data.
