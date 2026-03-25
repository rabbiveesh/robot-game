# Presentation Layer Migration — Tech Debt Tracker

## Current State

The presentation layer is the original prototype: 5 flat files with global mutable state, loaded via `<script>` tags. The domain layer (`src/domain/learning/`) is clean and tested, but it connects to the game via a monkey-patching adapter (`adapter.js`). This works today but will break when we need to replace (not just wrap) presentation behavior.

### Legacy files and their debt

| File | LOC (approx) | Debt |
|------|-------------|------|
| `dialogue.js` | ~600 | Does too much: challenge UI, dialogue box, TTS, AI providers (2), Dum Dum counter, skill badges. Has been gutted (phonics removed), patched (hints disabled), and monkey-patched (adapter wraps 4 functions). Next feature (interaction model) requires replacing the challenge UI entirely — can't be patched. |
| `game.js` | ~450 | Game loop, state machine, input handling, save system, render orchestration. The state machine (TITLE/PLAYING/DIALOGUE/CHALLENGE) is fragile — adding states for voice input, shop UI, quest journal will make it worse. |
| `world.js` | ~400 | Map data, portals, camera, NPC defs, glitch renderer. Relatively clean but map data should be separated from rendering logic. |
| `sprites.js` | ~700 | All programmatic sprites. Fine as-is — pure drawing functions, no state. Can be moved to `src/presentation/sprites/` wholesale. |
| `characters.js` | ~250 | Player movement, robot follow AI, NPC interaction detection. Movement logic is domain (should be in `src/domain/character/`), rendering is presentation. Currently mixed. |
| `adapter.js` | ~400 | The bridge between domain and legacy. Monkey-patches `generateMathChallenge`, `startChallenge`, `selectChallengeChoice`, `advanceDialogue`, `handleChallengeClick`, `gatherSaveData`, `loadFromSlot`, `initGame`, `render`. Every new domain feature adds more patches. |

### What triggers the migration

The presentation migration is NOT a standalone project. Each trigger below is a feature that forces us to migrate a specific part:

## Migration Triggers

### Trigger 1: Interaction Model (CRA visuals + show-me/tell-me)
**Spec:** `docs/interaction-model-spec.md`
**Forces migration of:** Challenge UI

The current challenge UI is a single `renderChallenge()` function in `dialogue.js` that draws a pop-up panel with buttons. The interaction model requires:
- CRA visual renderers (concrete dots, representational number line/blocks, abstract text) swappable per question
- Show-me / tell-me buttons that dynamically change the visual
- Answer mode switching (choice → eliminate → free input → build)
- Self-selected answer mode picker

This cannot be patched onto `renderChallenge()`. It's a full replacement.

**Migrate to:** `src/presentation/ui/challenge-ui.js` — reads challenge state from domain, renders the appropriate CRA visual + answer mode + scaffold buttons. No globals.

### Trigger 2: Voice Input
**Spec:** `docs/voice-input-impl-spec.md`
**Forces migration of:** Input handling

The current input is keydown/keyup listeners set up in `initInput()` in `game.js`. Adding voice input means the input layer needs to handle:
- Keyboard (existing)
- Mouse/touch (existing)
- Voice recognition (new)
- Input mode switching (which is active)

This is manageable without migration (add voice handlers alongside existing ones), but the state machine in `game.js` (TITLE/PLAYING/DIALOGUE/CHALLENGE) will need a new state for voice-listening, or voice needs to work within CHALLENGE state. Keep an eye on complexity.

**Migrate when:** input handling in `game.js` exceeds 3 input modalities or the state machine exceeds 6 states. Until then, additive patches are fine.

### Trigger 3: Quest System
**Spec:** `docs/rpg-quest-spec.md`
**Forces migration of:** Interaction orchestration, NPC system, game state

The current interaction flow is: `handleInteract()` → `triggerInteraction()` → `startDialogue()/startChallenge()`. Quests add: quest state tracking, step sequencing, travel objectives, branching choices, rewards. This replaces the entire `triggerInteraction` function and requires persistent quest state across save/load.

**Migrate to:**
- `src/application/quest-service.js` — quest lifecycle, step sequencing
- `src/presentation/ui/quest-journal.js` — quest log overlay
- NPC interaction refactored to check active quest before defaulting to random dialogue

**This is the big one.** Quests touch everything: NPCs, dialogue, challenges, map zones, save data. Plan for this to be a multi-day migration.

### Trigger 4: Dum Dum Economy
**Spec:** `docs/dum-dum-economy-spec.md`
**Forces migration of:** Inventory, shop UI

Phase 1 (give button + Sparky reactions) can be patched onto existing code. Phase 2 (shop with math-embedded purchases) requires a shop UI that uses the interaction model for purchase math. This depends on Trigger 1 being done.

**Migrate when:** phase 2 starts. Phase 1 is fine as a patch.

### Trigger 5: Parent Dashboard
**Spec:** `docs/adaptive-learning-spec.md` (section on parent visibility)
**Forces migration of:** UI overlay system

The current debug overlay (P key) is a function that draws directly on the canvas. A real parent dashboard needs: scrollable content, tap/click targets, dial overrides, session replay, profile export. Canvas-only won't cut it — this needs DOM-based UI overlaid on the game canvas.

**Migrate to:** `src/presentation/ui/parent-dashboard.js` — HTML/CSS overlay on top of the canvas, reads from domain state and event log. Could be a separate HTML page that loads the save data.

**Migrate when:** we want to show this to parents beyond the developer. The P-key debug overlay is fine for now.

### Trigger 6: De-Broccoli (Interactive CRA Mini-Games)
**Spec:** `docs/debroccoli-spec.md`
**Forces migration of:** The entire challenge rendering system

Interactive CRA (drag objects, tap number line, build answers) requires a mini-game framework where each game type is a pluggable component with its own input handling, rendering, and state. The current `renderChallenge()` is a single monolithic function.

**Migrate to:** `src/presentation/minigames/` — each mini-game is a module exporting `render(ctx, state, time)`, `handleInput(event)`, and `getResult()`. A coordinator selects which mini-game to use based on the CRA stage and answer mode.

**This is the final form of the presentation layer.** Everything before this is incremental. This is where the legacy challenge code dies completely.

## Migration Order (recommended)

```
1. Challenge UI (forced by interaction model)     ← FIRST
   ├── Extract renderChallenge into a module
   ├── Add CRA visual renderers
   ├── Add show-me/tell-me/answer-mode
   └── Adapter simplified (fewer patches)

2. Voice input (parallel, independent)
   └── New module, additive, no migration needed

3. Dum Dum economy phase 1 (parallel, additive)
   └── Give button + reactions, patch onto existing

4. Quest system (big, after challenge UI is clean)
   ├── Extract interaction orchestration
   ├── Add quest service
   └── Refactor NPC system

5. Dum Dum economy phase 2 (after quest system)
   └── Shop uses interaction model + quest rewards

6. Parent dashboard (whenever, independent)
   └── DOM overlay, reads save data

7. Interactive CRA mini-games (last, after everything)
   └── Replace challenge UI with mini-game framework
   └── Legacy dialogue.js challenge code deleted
```

## Files to Delete When Done

When the full migration is complete, these files are replaced:
- `dialogue.js` → split into `src/presentation/ui/dialogue-box.js`, `src/presentation/ui/challenge-ui.js`, `src/infrastructure/dialogue-cache.js`, `src/infrastructure/speech-service.js`
- `game.js` → split into `src/application/game-session.js`, `src/presentation/renderer/canvas-renderer.js`, `src/presentation/input/keyboard.js`
- `characters.js` → split into `src/domain/character/player.js`, `src/domain/character/companion.js`, `src/presentation/renderer/character-renderer.js`
- `world.js` → split into `src/domain/world/game-map.js`, `src/domain/world/maps/*.js`, `src/presentation/renderer/tile-renderer.js`
- `sprites.js` → moved to `src/presentation/sprites/` (mostly unchanged, just re-exported)
- `adapter.js` → deleted entirely (domain wires in directly)
