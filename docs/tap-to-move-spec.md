# Tap-to-Move & Mobile Input Spec

Click/tap anywhere on the map to move there. Tap an NPC to walk up and interact. All existing keyboard controls remain. Target: mobile-first but works identically with mouse.

Schedule: after step 11 (delete JS). No legacy dependencies.

## Why

A 4-year-old on an iPad can't use arrow keys. Touch is the native input for kids this age. Click-to-move also benefits mouse users who prefer point-and-click over WASD.

## Input Model

Macroquad 0.4 auto-maps touch to mouse events. `mouse_position()` and `is_mouse_button_pressed(MouseButton::Left)` capture both mouse clicks and finger taps. No separate touch handling needed.

### Click/Tap on Map Tile

1. Player taps a walkable tile on the map.
2. Pathfinder computes shortest path from player's current tile to target tile.
3. Player follows the path one tile at a time (using existing `Entity::start_move`).
4. Each step uses the same smooth interpolation as keyboard movement (200 px/sec, ~240ms per tile).
5. Tapping a new destination mid-path cancels the current path and recomputes from the player's current tile.
6. Sparky follows normally via `record_player_pos` — no changes to Sparky's follow system.

### Click/Tap on NPC

1. Player taps an NPC sprite.
2. Pathfinder targets a tile **adjacent** to the NPC, not the NPC's tile (NPCs are solid).
3. Pick the adjacent tile that's closest to the player. If multiple are equidistant, prefer the tile that faces the NPC (so the player arrives facing them).
4. Player walks the path. On arrival, auto-trigger interaction (equivalent to pressing Space while facing the NPC).
5. Same for Sparky: tap Sparky to walk adjacent and interact.

### Click/Tap on Solid Tile

No-op. No path generated. Optional: brief visual feedback (small X or flash) so the kid knows they can't go there.

### Click/Tap During Dialogue

Tap advances the dialogue (equivalent to Space). The full dialogue box is the touch target — don't require tapping a tiny button.

### Click/Tap During Challenge

Already works. Choice buttons, scaffold buttons, and dismiss are all clickable. No changes needed.

### Screen-to-Tile Conversion

The game uses Camera2D. Mouse position is in screen coordinates. Conversion:

```rust
fn screen_to_tile(mx: f32, my: f32, camera: &GameCamera) -> (usize, usize) {
    let world_x = mx + camera.x;
    let world_y = my + camera.y;
    let tile_x = (world_x / TILE_SIZE).floor() as usize;
    let tile_y = (world_y / TILE_SIZE).floor() as usize;
    (tile_x, tile_y)
}
```

Note: this needs to account for the Camera2D zoom/target transform. The camera currently uses `zoom: vec2(2.0 / screen_width(), 2.0 / screen_height())` which maps world coordinates 1:1 to screen pixels when the window matches GAME_W/GAME_H. If the window is resized, the conversion must apply the inverse camera transform.

## Pathfinding

### Algorithm: BFS

BFS on the tile grid. Not A* — the grid is small (max 30x25 = 750 tiles), BFS is simpler, and the difference is unmeasurable at this scale.

### Walkability

A tile is walkable for pathfinding if:
- `!map.is_solid(x, y)` — uses existing collision check (includes secret walkable overrides)
- No NPC occupies the tile — `!npcs.iter().any(|n| n.tile_x == x && n.tile_y == y)`
- Sparky's tile is walkable for pathfinding (player can walk through Sparky)

Sparky is always pathable — the player should be able to pathfind through Sparky's tile. The existing long-press walk-through applies to keyboard; for click-to-move, Sparky just lets you through since the intent is clear (you tapped a tile behind him).

### Implementation

```rust
// In a new file: robot-buddy-game/src/pathfinding.rs

/// BFS pathfind on the tile grid. Returns the path as a list of (x, y) tiles
/// from start (exclusive) to goal (inclusive). Returns None if no path exists.
pub fn find_path(
    start: (usize, usize),
    goal: (usize, usize),
    map: &Map,
    npcs: &[Npc],
    sparky_pos: (usize, usize),
) -> Option<Vec<(usize, usize)>> {
    // BFS with visited set
    // Neighbors: 4-directional (up, down, left, right)
    // Walkable: !map.is_solid(x, y) && no NPC at (x, y)
    // Sparky's tile is always walkable for pathfinding
    // Goal tile itself must be walkable (or be the NPC-adjacent target)
    // Reconstruct path from came_from map
}
```

The path is stored as a `Vec<(usize, usize)>` on the player (or in a `ClickPath` struct). Each frame, if the player has a path and isn't currently moving, pop the next tile and call `start_move`. The existing `move_toward_target` handles the smooth interpolation.

### Path Cancellation

- New click: replace path
- Keyboard movement: clear path immediately (keyboard takes priority)
- Portal arrival: clear path (new map, old path is invalid)
- Dialogue/challenge start: clear path

### Visual Path Indicator (Optional)

Draw subtle dots or faded tiles along the computed path so the kid can see where they're going. Helps a 4-year-old understand "I tapped there and my character is walking there." Draw under the entity layer, over the tile layer.

```rust
fn draw_path_indicator(path: &[(usize, usize)], time: f32) {
    for (i, &(tx, ty)) in path.iter().enumerate() {
        let alpha = 0.3 - (i as f32 * 0.02).min(0.25);
        let x = tx as f32 * TILE_SIZE + TILE_SIZE / 2.0;
        let y = ty as f32 * TILE_SIZE + TILE_SIZE / 2.0;
        draw_circle(x, y, 3.0, Color::new(1.0, 1.0, 1.0, alpha));
    }
}
```

## NPC Tap Target

NPCs are 48x48 pixel sprites, which is fine for mouse but tight for a small finger on a phone screen. Expand the tap target to 64x64 (8px padding on each side) centered on the NPC's tile. This doesn't change the NPC's collision tile — just the clickable area.

```rust
fn find_tapped_npc(world_x: f32, world_y: f32, npcs: &[Npc]) -> Option<usize> {
    let padding = 8.0;
    for (i, npc) in npcs.iter().enumerate() {
        let nx = npc.tile_x as f32 * TILE_SIZE;
        let ny = npc.tile_y as f32 * TILE_SIZE;
        if world_x >= nx - padding && world_x <= nx + TILE_SIZE + padding
            && world_y >= ny - padding && world_y <= ny + TILE_SIZE + padding
        {
            return Some(i);
        }
    }
    None
}
```

Same for Sparky.

## Mobile UI Adjustments

### Dialogue Box

Already at the bottom of the screen, full width. For mobile, make the entire box a tap target to advance (not just the "SPACE >" indicator). The typewriter skip + advance logic stays the same — first tap shows full text, second tap advances.

### Challenge Panel

Already clickable. Button sizes are 160x70 (choices) and 90x30 (scaffold). The choices are fine for touch. The scaffold buttons are small — bump to 110x40 minimum on mobile. Detect mobile via screen size or aspect ratio:

```rust
let is_mobile = screen_width() < 600.0 || screen_height() < 500.0;
let scaff_btn_w = if is_mobile { 110.0 } else { 90.0 };
let scaff_btn_h = if is_mobile { 40.0 } else { 30.0 };
```

### Title Screen

Already clickable — buttons for NEW/LOAD/DELETE. Keyboard shortcuts (1/2/3) stay. Name input needs the on-screen keyboard which macroquad's WASM target triggers automatically via `get_char_pressed()` on mobile browsers (the canvas gets focus, browser shows keyboard).

### Virtual D-Pad (Optional, Defer)

Some kids like d-pad controls even on touch. A semi-transparent d-pad overlay in the bottom-left corner. This is a nice-to-have, not a must-have — tap-to-move is the primary input. Defer unless playtesting shows kids want it.

## State Changes to main.rs

```rust
// New state in the game loop
struct ClickTarget {
    path: Vec<(usize, usize)>,
    interact_npc: Option<usize>,  // auto-interact on arrival
    interact_sparky: bool,
}

// In the Playing state input section:
// 1. Check keyboard input first (clears click path)
// 2. If no keyboard input, check for click
// 3. If click, convert to tile, pathfind, store path
// 4. In update section, if path exists and player idle, pop next tile and start_move
// 5. On arrival at final tile, if interact_npc is set, trigger interaction
```

## File Layout

```
robot-buddy-game/src/
  pathfinding.rs          # BFS, find_path(), find_adjacent_to_npc()
  main.rs                 # ClickTarget struct, input handling changes
```

## Testing

Pathfinding is pure logic — no macroquad deps. Fully testable with `cargo test`.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn simple_map() -> Map {
        // 5x5 map with a wall in the middle
        // 0 0 0 0 0
        // 0 0 X 0 0
        // 0 0 X 0 0
        // 0 0 0 0 0
        // 0 0 0 0 0
    }

    #[test]
    fn direct_path() {
        let map = simple_map();
        let path = find_path((0, 0), (4, 0), &map, &[], (99, 99));
        assert_eq!(path.unwrap().len(), 4); // 4 steps
    }

    #[test]
    fn path_around_wall() {
        let map = simple_map();
        let path = find_path((0, 1), (4, 1), &map, &[], (99, 99));
        assert!(path.is_some());
        // Path should go around the wall, not through it
        for &(x, y) in path.as_ref().unwrap() {
            assert!(!map.is_solid(x, y));
        }
    }

    #[test]
    fn no_path_to_solid() {
        let map = simple_map();
        let path = find_path((0, 0), (2, 1), &map, &[], (99, 99)); // wall tile
        assert!(path.is_none());
    }

    #[test]
    fn npc_adjacent_target() {
        let npc_pos = (3, 3);
        let player_pos = (0, 3);
        let target = find_adjacent_to_npc(player_pos, npc_pos, &simple_map(), &[]);
        // Should pick (2, 3) — adjacent to NPC, closest to player
        assert_eq!(target, Some((2, 3)));
    }

    #[test]
    fn path_through_sparky() {
        let map = simple_map();
        let sparky = (2, 0);
        let path = find_path((0, 0), (4, 0), &map, &[], sparky);
        assert!(path.is_some());
        // Sparky's tile should be in the path (passable)
        assert!(path.unwrap().contains(&(2, 0)));
    }
}
```

## Migration Note

This feature has zero legacy dependencies. It reads `Map`, `Npc`, and `Entity` — all Rust structs. It writes to `Entity::start_move` — existing API. It adds one new file (`pathfinding.rs`) and modifies the input section of `main.rs`. No JS, no bridge, no boundary.
