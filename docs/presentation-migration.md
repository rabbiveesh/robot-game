# Presentation Layer Migration — Tech Debt Tracker

## Current State

The domain is Rust (compiled to WASM). The presentation layer is legacy JS files with global mutable state. The adapter (`adapter.js`) bridges WASM domain ↔ presentation.

The bridge was burned for challenges (challenge lifecycle goes through the Rust state machine, not legacy globals). But the rest of the presentation (dialogue, movement, maps, rendering) is still legacy.

### Legacy files and their debt

| File | Debt |
|------|------|
| `dialogue.js` | Still handles dialogue box rendering, NPC trigger functions, voice input UI, TTS. Challenge rendering delegates to QuizRenderer but the dialogue rendering is inline. |
| `game.js` | Game loop, state machine, input dispatch. Input handlers are per-file (game.js, index.html, adapter.js) — caused the ESC freeze bug. |
| `world.js` | Map data, portals, camera, NPC definitions, glitch renderer. Map data should be data files, not code. |
| `sprites.js` | Programmatic sprites. Clean — pure drawing functions. |
| `characters.js` | Player movement, robot AI. Movement logic mixed with rendering. |
| `adapter.js` | WASM bridge. Handles events, save/load, intake. Shrinking as features move to Rust. |

### What's already migrated

- Challenge lifecycle → Rust state machine (burn-the-bridge)
- QuizRenderer extracted from dialogue.js
- Base-10 blocks visual in `src/presentation/renderers/visuals/`
- Visual registry pattern for Dev Zone
- Show-me / Tell-me buttons through renderer interface

## Migration Triggers

### Input Dispatcher (URGENT)

Multiple files register key listeners. Caused the settings freeze bug. Consolidate to one dispatcher in game.js with state-based routing:

```js
window.addEventListener('keydown', (e) => {
  switch (GAME.state) {
    case 'INTERACTION_MENU': handleMenuInput(e); break;
    case 'SETTINGS':         handleSettingsInput(e); break;
    case 'CHALLENGE':        handleChallengeInput(e); break;
    case 'PLAYING':          handlePlayingInput(e); break;
    case 'DIALOGUE':         handleDialogueInput(e); break;
  }
});
```

### More Visualization Renderers

Each new visual method (ten-frames, arrays, number lines, bar models) is a new file in `src/presentation/renderers/visuals/`. Registers via the visual registry. No legacy code changes needed — the QuizRenderer picks from the registry.

### Quest System (future)

Requires: quest state service, NPC refactoring, interaction orchestration rewrite. The biggest remaining migration.

### Dum Dum Shop (future)

Phase 2 economy: shop UI with embedded math. Needs the interaction model (CRA + answer mode + scaffold) wired into a ShopRenderer.

### Full Rust Renderer (optional future)

Port canvas rendering to Rust via Macroquad. Replaces all JS rendering files. See `docs/rust-wasm-migration-spec.md` Phase 2.

## Files to Delete When Legacy Dies

When the full migration is complete:
- `dialogue.js` → split into `src/presentation/ui/dialogue-box.js` + already-extracted renderers
- `game.js` → `src/presentation/game-loop.js` + `src/presentation/input/dispatcher.js`
- `characters.js` → player/robot movement stays JS (or goes Rust with Macroquad)
- `world.js` → map data as JSON files, camera/portal logic in JS or Rust
- `sprites.js` → moved to `src/presentation/sprites/` (mostly unchanged)
- `adapter.js` → dies when Rust domain is called directly (Macroquad path) or shrinks to a thin WASM loader
