# Dev Zone

A small in-game showroom for sanity-checking sprites, chests, NPC interactions, and TTS without playing through the full game. Implemented in `robot-buddy-game/src/main.rs` + the `"dev"` map in `tilemap.rs` + the `"dev"` NPC list in `npc.rs`.

## Access

Name a save file `justinbailey` (case-insensitive, whitespace ignored). The new-game flow detects this and routes into the dev map instead of the normal save flow.

```rust
fn is_dev_zone_code(name: &str) -> bool {
    let normalized: String = name.chars().filter(|c| !c.is_whitespace()).collect();
    normalized.eq_ignore_ascii_case("justinbailey")
}
```

The dev session:
- Skips the intake quiz (`intake_completed = true`).
- Skips writing to a save slot (no persistence — the room resets each entry).
- Pre-sets `math_band = 5`, `dum_dums = 20` so you can immediately give and trigger challenges.
- Player name is forced to `"Dev"`.
- ESC exits to the title screen.

## The map

A 16×12 indoor room (wood floor, stone walls). Single screen, no scrolling.

```
WWWWWWWWWWWWWWWW
W..............W
WB...T....T...BW
W..............W
W..............W
W..............W
W......C.C.....W
W..............W
W..............W
W.....rrr......W
W.....r.r......W
WWWWWWWWWWWWWWWW
```

`B` bookshelf · `T` table · `C` chest · `r` rug · `.` wood floor · `W` wall

Player spawns at (7, 10) facing up. Sparky spawns at (8, 10).

## NPC line-up

Every NPC sprite, one of each, lined up at y=3. `can_receive_gifts: false` and `never_challenge: true` on all of them — talking is purely a TTS / dialogue check, not a math interaction.

| Tile (x, 3)      | NPC                    |
|------------------|------------------------|
| 2                | Mommy                  |
| 4                | Professor Gizmo (sage) |
| 6                | Bolt the Shopkeeper    |
| 8                | Tali (kid_1)           |
| 10               | Noa (kid_2)            |
| 12               | B0RK.exe (glitch_dog)  |
| 13               | Old Oak (grove_spirit) |

## What it lets you check

- **Sprite rendering** — every NPC sprite plus Sparky and the player visible at once.
- **TTS voices per speaker** — walk up to each NPC, talk, hear their voice config.
- **Chest interaction** — two chests in the middle of the room.
- **Tile rendering** — wood floor, walls, tables, bookshelves, rugs all on one screen.
- **Give mechanic** — 20 Dum Dums in pocket, but every NPC has `can_receive_gifts: false` so the give option is hidden (sanity check that the menu correctly omits unavailable options).

## What it doesn't do (yet)

- No visualization gallery (dots / ten-frame / base-10 blocks side-by-side at the same problem).
- No challenge renderer gallery across phases.
- No live profile inspector with dial sliders.
- No tile gallery showing every tile type — only the ones placed in this map.

If those become valuable, they'd live as additional dev-only screens reachable from the dev map (e.g., interact with a specific chest to open the visual playground). Today the dev zone is just a curated room.

## Sparky's intro line

```
"BEEP BOOP! Dev zone! Walk around, talk to everyone, open chests. ESC to exit!"
```
