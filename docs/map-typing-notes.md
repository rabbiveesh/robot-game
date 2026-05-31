# Notes: First-Class Map Typing

Status: **proposal / not started.** Captured 2026-05-31 after adding the `annex`
validation map (see the new-map genericity tests in `tests/story.rs`).

## The smell

Maps are stringly-typed end to end. `Map.id`, `Portal.from_map`/`to_map`,
`Npc.home_map`, the `npcs_offstage` keys, save data, and `SPARKY_HOME_MAP` are
all `&str` / `String`. There are ~108 bare map-id string literals in
`robot-buddy-game/src`, and several **parallel match-on-id** sites that must be
kept in sync by hand:

- `tilemap::Map::by_id(&str) -> Map` — `_ => Self::overworld()` (silent fallback)
- `npc::npcs_for_map(&str) -> Vec<Npc>` — `_ => vec![]`
- `ui::hud::get_area_name(&str)` — `_ => "???"`
- `SECRET_WALKABLE: &[(&str, usize, usize)]`
- dream special-casing in `game::handle_portal` (`dest_map == "dream"`, …)

Why it bites:

1. **Typos compile.** `"overwrld"` is a valid `&str`; it fails at runtime as a
   silent fallback (wrong geometry / empty roster / `"???"` area name), not a
   compile error.
2. **No exhaustiveness.** Adding a map doesn't force you to touch every site.
   We just lived this twice: the `display_name_for_buddy_id` hardcoded map list
   leaked a raw `"pip"` token (now fixed via `NpcKind::from_id`), and the
   `npc_dialogue_lines` match *did* catch `Pip` at compile time — the difference
   is exactly enum-match vs. string-list.
3. **Silent registration gaps.** A map missing from `by_id` resolves to the
   overworld instead of failing loudly.

Contrast: `NpcKind` is already a first-class enum, and its exhaustive matches
(`as_str`, `display_name`, `npc_dialogue_lines`) are what make adding an NPC
safe. Maps deserve the same treatment.

## Proposed shape

Introduce `enum MapId` (in `tilemap.rs`):

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MapId {
    Overworld, Home, Lab, Shop, Dream, Doghouse, Grove, Dev, Control, Annex,
}

impl MapId {
    pub const ALL: &'static [MapId] = &[ /* … */ ];
    pub fn as_str(self) -> &'static str { /* stable save tokens */ }
    pub fn from_str(s: &str) -> Option<MapId> { Self::ALL.iter().copied().find(|m| m.as_str() == s) }

    pub fn geometry(self) -> Map;            // replaces by_id, EXHAUSTIVE — no fallback
    pub fn roster(self) -> Vec<npc::Npc>;    // replaces npcs_for_map, exhaustive
    pub fn area_name(self) -> &'static str;  // replaces get_area_name's map arm
    pub fn render_mode(self) -> RenderMode;
    pub fn is_secret_entry(self) -> bool;    // or model secret tiles as data
}
```

Then thread the type through:

- `Map.id: MapId`
- `Portal { from_map: MapId, to_map: MapId, … }`
- `Npc.home_map: MapId`
- `npcs_offstage: HashMap<MapId, Vec<Npc>>`
- `SPARKY_HOME_MAP: MapId`
- Save format keeps **string tokens** (`as_str`/`from_str` at the load/save
  boundary only) so existing saves keep working; `from_str -> Option` drops an
  unknown/removed map gracefully instead of panicking.

Stretch goal — a **single registration table**: one `&[(MapId, MapMeta)]` where
`MapMeta` bundles `geometry fn`, `roster fn`, `area_name`, `render_mode`,
`is_secret`. Adding a map becomes one row; every accessor reads the table. That
collapses the parallel matches into one source of truth.

## What stays genuinely special (typing won't erase it)

The **dream mirror** mechanic is real game logic, not stringly-typed accident:
`dream` is a render-recolored copy of the overworld, and while `dreaming` the
player's overworld-bound portals redirect to `dream`. First-class typing makes
this *explicit* rather than removing it — e.g. `MapId::Dream.mirror_of() ==
Some(MapId::Overworld)`, or a small `MapKind { Normal, DreamMirror(MapId), … }`.

## Migration (incremental, compiler-guided)

1. Add `MapId` + `as_str`/`from_str`/`ALL`. No behavior change.
2. Swap `Map::by_id` → `MapId::geometry` (exhaustive match deletes the silent
   `_ => overworld()`). Convert call sites; save boundary uses `from_str`.
3. Thread `MapId` into `Portal`, `Map.id`, `Npc.home_map`, `npcs_offstage`,
   `SPARKY_HOME_MAP`. Each is a compiler-chased ripple.
4. Fold `get_area_name`, `SECRET_WALKABLE`, render-mode override onto `MapId`.
5. (Optional) collapse into the single `MapMeta` registration table.

Touches `tilemap.rs`, `npc.rs`, `game.rs`, `ui/hud.rs`, `save.rs`. Medium effort;
the type system does most of the chasing. Net result: adding a map is a single
registration + a set of compile errors that walk you to every site — same
guarantee `NpcKind` already gives us, and the same guarantee the architecture
invariants in `CLAUDE.md` promise ("the compiler finds every place that needs
updating").
