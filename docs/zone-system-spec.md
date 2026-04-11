# Zone System Spec

Zones replace area-name rectangles. Every tile in every map has a zone. The data lives on the Map struct, not in a separate HUD file. Desync between tile data and area names becomes structurally impossible.

Schedule: after migration completes. Low urgency — the current `hud.rs` area rects work, this is a correctness/maintainability upgrade.

## Problem

Area names (defined in `hud.rs` as `AreaRect` coordinate ranges) and tile data (defined in `tilemap.rs` as `Vec<Vec<u8>>`) are two independent declarations about the same grid. They can disagree silently. Move a house in the tile grid, forget to update the area rects, and the HUD says "Pond" while the player is standing on grass.

Rust's type system should make this impossible.

## Design

### Per-cell zone storage

Every cell carries both a tile type and a zone ID:

```rust
#[derive(Clone, Copy)]
pub struct Cell {
    pub tile: u8,
    pub zone: u8,
}
```

The Map struct holds cells instead of raw tile IDs:

```rust
pub struct Map {
    pub id: &'static str,
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<Cell>>,
    pub zone_names: &'static [&'static str],
    pub render_mode: RenderMode,
}
```

Area name lookup becomes a direct cell read:

```rust
pub fn zone_name(&self, tx: usize, ty: usize) -> &'static str {
    if tx < self.width && ty < self.height {
        self.zone_names[self.cells[ty][tx].zone as usize]
    } else {
        "???"
    }
}
```

### Tile access

Existing code that reads `map.tiles[row][col]` changes to `map.cells[row][col].tile`. The `is_solid` method reads from the cell's tile field:

```rust
pub fn is_solid(&self, col: usize, row: usize) -> bool {
    if col >= self.width || row >= self.height { return true; }
    if is_secret_walkable(self.id, col, row) { return false; }
    let tile = self.cells[row][col].tile;
    matches!(tile, 2 | 3 | 4 | 6 | 7 | 9 | 10 | 16 | 17 | 99)
}
```

Same for `tile_color` — reads `map.cells[row][col].tile`.

### Map construction: zone painting

Declaring per-cell zones inline would double the verbosity of every map definition. Instead, maps are built in two phases: tile grid first, then zone paint.

```rust
impl Map {
    /// Create a map with all tiles in zone 0 (the default zone).
    fn from_tiles(
        id: &'static str,
        width: usize,
        height: usize,
        tiles: Vec<Vec<u8>>,
        zone_names: &'static [&'static str],
        render_mode: RenderMode,
    ) -> Self {
        let cells = tiles.into_iter()
            .map(|row| row.into_iter().map(|t| Cell { tile: t, zone: 0 }).collect())
            .collect();
        Map { id, width, height, cells, zone_names, render_mode }
    }

    /// Paint a rectangular region with a zone ID.
    fn paint_zone(&mut self, zone: u8, x1: usize, y1: usize, x2: usize, y2: usize) {
        for y in y1..=y2.min(self.height - 1) {
            for x in x1..=x2.min(self.width - 1) {
                self.cells[y][x].zone = zone;
            }
        }
    }
}
```

### Overworld example

```rust
pub fn overworld() -> Self {
    const ZONES: &[&str] = &[
        "The Wild",        // 0 — default, unpainted tiles
        "Home",            // 1
        "Main Path",       // 2
        "Pond",            // 3
        "East House",      // 4
        "South House",     // 5
        "Forest Edge",     // 6
        "South Meadow",    // 7
        "Treasure Woods",  // 8
    ];

    let tiles = vec![
        vec![4,4,4,4,4,4,4, /* ... existing tile data ... */],
        // ... all 25 rows unchanged ...
    ];

    let mut m = Map::from_tiles("overworld", 30, 25, tiles, ZONES, RenderMode::Normal);
    m.paint_zone(1, 3, 3, 8, 9);      // Home
    m.paint_zone(2, 9, 1, 18, 10);    // Main Path
    m.paint_zone(3, 14, 11, 22, 15);  // Pond
    m.paint_zone(4, 20, 3, 26, 7);    // East House
    m.paint_zone(5, 22, 15, 27, 19);  // South House
    m.paint_zone(6, 1, 10, 5, 22);    // Forest Edge
    m.paint_zone(7, 6, 15, 14, 22);   // South Meadow
    m.paint_zone(8, 22, 8, 28, 14);   // Treasure Woods
    m
}
```

### Interior maps

Single-zone maps don't need `paint_zone` at all. Zone 0 covers everything:

```rust
pub fn home() -> Self {
    const ZONES: &[&str] = &["Home (Inside)"];
    let tiles = vec![ /* ... */ ];
    Map::from_tiles("home", 10, 8, tiles, ZONES, RenderMode::Normal)
}

pub fn doghouse() -> Self {
    const ZONES: &[&str] = &["D0GH0USE.exe"];
    let tiles = vec![ /* ... */ ];
    Map::from_tiles("doghouse", 16, 12, tiles, ZONES, RenderMode::Glitch)
}
```

## What moves, what dies

| Before | After |
|--------|-------|
| `hud.rs`: `AreaRect` struct + `OVERWORLD_AREAS` table + `get_area_name()` match on map_id | Deleted. |
| `tilemap.rs`: `Map.tiles: Vec<Vec<u8>>` | `Map.cells: Vec<Vec<Cell>>` |
| `hud.rs`: `draw_area_name(map_id, tx, ty)` | `draw_area_name(map, tx, ty)` — reads from map directly |
| `tilemap.rs`: `draw_map` reads `map.tiles[row][col]` | reads `map.cells[row][col].tile` |
| `tilemap.rs`: `is_solid` reads `map.tiles[row][col]` | reads `map.cells[row][col].tile` |
| `hud.rs`: `DebugOverlay::draw` calls `get_area_name(map_id, tx, ty)` | calls `map.zone_name(tx, ty)` |

## Validation (optional, debug-only)

Sanity checks that run in debug builds after map construction:

```rust
#[cfg(debug_assertions)]
fn validate_zones(map: &Map) {
    // Every zone ID in cells is a valid index into zone_names
    for row in &map.cells {
        for cell in row {
            assert!(
                (cell.zone as usize) < map.zone_names.len(),
                "Map '{}' has zone ID {} but only {} zone names",
                map.id, cell.zone, map.zone_names.len()
            );
        }
    }

    // Every zone name is used by at least one cell (no orphan zones)
    for (i, name) in map.zone_names.iter().enumerate() {
        let used = map.cells.iter().any(|row| row.iter().any(|c| c.zone == i as u8));
        assert!(used, "Map '{}' has unused zone '{}' (index {})", map.id, name, i);
    }
}
```

These panic at startup in debug builds if zones are misconfigured, but cost nothing in release.

## Testing

Zone lookup is pure logic. Testable without macroquad:

```rust
#[test]
fn overworld_home_zone() {
    let map = Map::overworld();
    assert_eq!(map.zone_name(5, 7), "Home");
}

#[test]
fn overworld_fallback_zone() {
    let map = Map::overworld();
    // Corner tile not in any painted zone
    assert_eq!(map.zone_name(0, 0), "The Wild");
}

#[test]
fn interior_single_zone() {
    let map = Map::home();
    // Every tile in an interior map has the same zone
    assert_eq!(map.zone_name(1, 1), "Home (Inside)");
    assert_eq!(map.zone_name(5, 3), "Home (Inside)");
}

#[test]
fn dream_inherits_zones() {
    let map = Map::dream();
    // Dream is a visual variant of overworld — zones should match
    assert_eq!(map.zone_name(5, 7), "Home");
    assert_eq!(map.zone_name(17, 12), "Pond");
}
```

## Future: non-rectangular zones

`paint_zone` is rectangular. If a zone needs irregular boundaries (e.g., a winding river), add `paint_zone_tiles`:

```rust
fn paint_zone_tiles(&mut self, zone: u8, tiles: &[(usize, usize)]) {
    for &(x, y) in tiles {
        if x < self.width && y < self.height {
            self.cells[y][x].zone = zone;
        }
    }
}
```

Or `paint_zone_flood` for flood-fill from a seed tile through matching tile types. But rectangular zones cover every current map. Don't build what you don't need yet.
