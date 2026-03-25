# Architecture Spec — Domain-Driven Design

## Bounded Contexts

```
┌─────────────────────────────────────────────────────┐
│                    PRESENTATION                      │
│  (Canvas, Sprites, UI, Input, TTS)                  │
│  Depends on everything below. Nothing depends on it.│
└──────────────────────┬──────────────────────────────┘
                       │ reads state, receives render commands
┌──────────────────────▼──────────────────────────────┐
│                    APPLICATION                       │
│  (GameSession, InteractionService, QuestService)     │
│  Orchestrates domain objects. Thin glue layer.       │
└───┬──────────┬───────────┬──────────┬───────────────┘
    │          │           │          │
┌───▼────┐ ┌──▼─────┐ ┌───▼────┐ ┌──▼──────────┐
│LEARNING│ │ QUEST  │ │ WORLD  │ │ CHARACTER   │
│ (core) │ │        │ │        │ │             │
└────────┘ └────────┘ └────────┘ └─────────────┘
    ▲          ▲           ▲          ▲
    │          │           │          │
┌───┴──────────┴───────────┴──────────┴───────────────┐
│                  INFRASTRUCTURE                       │
│  (Persistence/SaveManager, Claude API, Speech)       │
└─────────────────────────────────────────────────────┘
```

**The golden rule**: the Learning domain has ZERO dependencies on browser APIs, canvas, DOM, or rendering. It is pure logic that takes events in and produces decisions out. This is what we unit test.

## Core Domain: Learning

This is what makes this game different from any other Zelda clone. Everything else is a supporting or generic subdomain.

### Aggregates

#### LearnerProfile (Aggregate Root)

The central model of "what we know about this kid." Owned entirely by the Learning domain.

```js
LearnerProfile {
  // ── Dials (set by intake, adjusted by ongoing play) ──
  pace:                0.5,    // 0=patient, 1=fast
  scaffolding:         0.5,    // 0=always show aids, 1=minimal
  challengeFreq:       0.5,    // 0=more story, 1=more puzzles
  streakToPromote:     3,      // 2-5
  wrongsBeforeTeach:   2,      // 1-3
  hintVisibility:      0.5,    // 0=always, 1=only after mistakes
  textSpeed:           0.035,  // seconds per char
  framingStyle:        0.5,    // 0=bottom-up (show steps first), 1=top-down (show goal first)

  // ── Per-operation CRA stage ──
  craStages: {
    'add':         'concrete',   // 'concrete' | 'representational' | 'abstract'
    'sub':         'concrete',
    'multiply':    'concrete',
    'divide':      'concrete',
    'number_bond': 'concrete',
  },

  // ── Current band ──
  mathBand:    1,   // 1-10

  // ── Intake state ──
  intakeCompleted: false,

  // Methods
  recordPuzzleAttempt(event: PuzzleAttempted): void
  recordTeachingResult(event: TeachingRetryResult): void
  recordBehavior(event: BehaviorSignal): void
  suggestNextChallenge(): ChallengeSuggestion
  shouldShowHints(operation): boolean
  getRepresentationFor(operation): 'concrete' | 'representational' | 'abstract'
  toJSON(): object   // for persistence
  static fromJSON(data): LearnerProfile
}
```

#### OperationStats (Value Object, owned by LearnerProfile)

```js
OperationStats {
  operation: string,    // 'add', 'sub', 'multiply', 'divide', 'number_bond'
  correct:   number,
  attempts:  number,
  accuracy(): number,   // correct / attempts
  isStrength(): boolean,  // accuracy > 0.75
  isWeakness(): boolean,  // accuracy < 0.5
}
```

#### RollingWindow (Value Object, owned by LearnerProfile)

```js
RollingWindow {
  entries: PuzzleAttempted[],   // last N entries (N = 20)
  maxSize: 20,

  push(entry: PuzzleAttempted): void
  accuracy(): number
  avgResponseTime(): number
  recentFrustrationSignals(): number
  operationMix(): Map<string, number>   // how many of each op type recently
}
```

### Domain Events

Events flow INTO the Learning domain from the application layer. The domain processes them and mutates its own state. Events are also the unit test interface — we feed events in and assert on state changes.

```js
// ── Puzzle events ──
PuzzleAttempted {
  operation: string,      // 'add', 'sub', 'multiply', 'divide', 'number_bond'
  band: number,
  correct: boolean,
  responseTimeMs: number,
  representationShown: 'concrete' | 'representational' | 'abstract' | null,
  attemptNumber: number,  // 1st try, 2nd try, etc.
}

TeachingModeTriggered {
  operation: string,
  band: number,
  representationShown: string,
}

TeachingRetryResult {
  operation: string,
  correct: boolean,
  representationStyle: 'concrete' | 'representational' | 'abstract',
}

// ── Behavioral events ──
BehaviorSignal =
  | { type: 'text_skipped' }                     // mashed space before typewriter done
  | { type: 'idle', durationMs: number }          // no input for a while
  | { type: 'rapid_clicking', responseTimeMs: number }  // <1s answer = mashing
  | { type: 'chose_harder_path' }
  | { type: 'chose_easier_path' }

// ── Band events (emitted BY the domain, consumed by application/presentation) ──
BandChanged {
  oldBand: number,
  newBand: number,
  reason: 'promotion' | 'demotion' | 'intake',
}

FrustrationStateChanged {
  level: 'none' | 'mild' | 'high',
  recommendation: string,   // e.g. 'switch_to_chat', 'drop_band', 'encourage'
}
```

### Domain Services

#### ChallengeGenerator

Produces the next challenge based on the learner profile. Pure function — no side effects, no randomness source dependency (takes a random seed for testability).

```js
ChallengeGenerator {
  generate(profile: LearnerProfile, context: QuestContext, rng: () => number): Challenge

  // Internals:
  // 1. Pick operation (weighted by operationStats: 60% strength, 40% growth)
  // 2. Pick numbers (scaled to band)
  // 3. Pick framing (story template from quest context)
  // 4. Pick representation (from CRA stage for this operation)
  // 5. Generate wrong answers (scaled spread)
}
```

#### IntakeAssessor

Runs the intake quiz logic. Stateless — takes answers in, produces a LearnerProfile.

```js
IntakeAssessor {
  generateIntakeQuestion(currentBand: number, questionIndex: number, rng): Challenge
  processIntakeResults(answers: IntakeAnswer[]): LearnerProfile
}

IntakeAnswer {
  band: number,
  correct: boolean,
  responseTimeMs: number,
  skippedText: boolean,
  representationEngaged: 'concrete' | 'representational' | null,
}
```

#### FrustrationDetector

Analyzes the rolling window and behavioral signals to detect frustration state.

```js
FrustrationDetector {
  assess(window: RollingWindow, recentBehaviors: BehaviorSignal[]): FrustrationState

  // Signals:
  // - 3+ wrong in a row on same band → high
  // - rapid clicking (<1s responses) → high (mashing)
  // - long idle after wrong (>15s) → mild
  // - accuracy drop below 40% in window → high
  // - chose easier path twice in a row → mild
}

FrustrationState {
  level: 'none' | 'mild' | 'high',
  recommendation: 'continue' | 'encourage' | 'switch_to_chat' | 'drop_band' | 'offer_easier_path',
}
```

## Supporting Domain: Quest

### Aggregates

#### Quest (Aggregate Root)

```js
Quest {
  id: string,
  title: string,
  description: string,
  steps: QuestStep[],
  prereqs: string[],
  mathDomain: string[],
  minBand: number,
  maxBand: number,
}
```

#### QuestState (Entity, tracks progress through a quest)

```js
QuestState {
  questId: string,
  status: 'available' | 'active' | 'completed',
  currentStepIndex: number,
  stepResults: StepResult[],   // track what happened at each step

  advance(): void
  isComplete(): boolean
  currentStep(): QuestStep
}
```

#### QuestStep (Value Object)

```js
QuestStep =
  | { type: 'dialogue', speaker: string, text: string }
  | { type: 'travel', targetMap: string, targetX: number, targetY: number }
  | { type: 'puzzle', template: string, operation: string, schema: string }
  | { type: 'choice', options: ChoiceOption[] }
  | { type: 'reward', dumDums: number, items: string[] }
```

### Domain Services

#### MicroQuestGenerator

Produces procedural quests from templates + learner profile.

```js
MicroQuestGenerator {
  generate(profile: LearnerProfile, zone: string, availableNPCs: string[], rng): Quest
}
```

#### PuzzleTemplateEngine

Fills story templates with numbers appropriate to the learner's band.

```js
PuzzleTemplateEngine {
  // Template: "${npc} needs ${quantity} ${item}s. Each costs ${price} gold. How much total?"
  // Fills quantity and price based on band, operation = multiply

  fill(template: PuzzleTemplate, band: number, operation: string, rng): FilledPuzzle
}

FilledPuzzle {
  storyText: string,          // "Bolt needs 6 potions. Each costs 4 gold. How much total?"
  mathExpression: string,     // "6 × 4"
  correctAnswer: number,      // 24
  operation: string,          // 'multiply'
  numbers: { a: number, b: number },
  choices: Choice[],          // for multiple choice fallback
  schema: string,             // 'buying'
}
```

## Generic Subdomain: World

```js
GameMap { id, width, height, tiles[][], renderMode }
Portal { fromMap, fromX, fromY, toMap, toX, toY, dir }
Zone { name, mapId, bounds, mathDomain, bandRange }
Camera { x, y, update(targetX, targetY, canvasW, canvasH) }
```

Pure data + simple logic. No dependencies on rendering.

## Generic Subdomain: Character

```js
Player { tileX, tileY, pixelX, pixelY, dir, gender, moving, move(dir), update(dt) }
Companion { tileX, tileY, followQueue, update(dt) }
NPC { id, name, tileX, tileY, dialogueContext }
Inventory { items: Map<string, number>, add(item, qty), remove(item, qty), has(item, qty) }
```

## Application Layer

Thin orchestration layer. No business logic — just wiring domain objects together and handling cross-cutting concerns.

```js
GameSession {
  learnerProfile: LearnerProfile,
  questStates: Map<string, QuestState>,
  player: Player,
  companion: Companion,
  currentMap: GameMap,
  inventory: Inventory,

  // Lifecycle
  startNew(name, gender, teachingStyle): void
  loadFromSave(saveData): void
  save(): SaveData

  // Core loop
  handleInteraction(target): InteractionResult
  handlePuzzleAnswer(answer, responseTimeMs): PuzzleResult
  handleBehavior(signal: BehaviorSignal): void
  tick(dt): void
}

InteractionService {
  // Decides what happens when player interacts with something
  interact(target, profile, activeQuest): InteractionResult

  // InteractionResult = Dialogue | Puzzle | QuestAdvance | ShopOpen | ...
}

QuestService {
  // Manages quest lifecycle
  availableQuests(profile, completedQuests): Quest[]
  startQuest(questId): QuestState
  advanceQuest(questState, stepResult): QuestState
  generateMicroQuest(profile, zone): Quest
}
```

## Infrastructure Layer

External concerns. Adapters that implement interfaces defined by the domain/application.

```js
SaveManager {
  // Implements persistence via localStorage
  getSaveSlots(): SaveSlot[]
  saveToSlot(index, gameSession): void
  loadFromSlot(index): SaveData
  deleteSlot(index): void
  copyProfile(fromIndex, toIndex): void
}

ClaudeDialogueService {
  // Fetches AI-generated dialogue
  fetchDialogue(context, systemPrompt): Promise<string>
  prefetch(playerName): void
}

SpeechService {
  // Browser TTS
  speak(text, speaker): void
  stop(): void
  enabled: boolean
}
```

## Presentation Layer

Rendering, input, UI. Depends on everything else. Nothing depends on it.

```
presentation/
  renderer/
    canvas-renderer.js    # Main render loop, compositing
    tile-renderer.js      # Draws tiles (normal, dream, glitch modes)
    character-renderer.js # Player, Sparky, NPCs
    effect-renderer.js    # Particles, celebrations
  sprites/
    tile-sprites.js       # Programmatic tile drawing functions
    character-sprites.js  # Player (boy/girl), robot, mommy, sage, dog
    ui-sprites.js         # Dum Dum icon, hearts, etc.
  ui/
    dialogue-box.js       # Typewriter text, speaker name
    hud.js                # Area name, dum dum counter, quest indicator
    quest-journal.js      # Quest log overlay
    puzzle-ui.js          # In-world puzzle interaction
    title-screen.js       # Save slots, new game form
    parent-dashboard.js   # Learner profile viewer
  input/
    keyboard.js           # Key state tracking
    mouse.js              # Click/touch handling
```

## Project Layout

```
robot-game/
├── src/
│   ├── domain/
│   │   ├── learning/
│   │   │   ├── learner-profile.js
│   │   │   ├── operation-stats.js
│   │   │   ├── rolling-window.js
│   │   │   ├── challenge-generator.js
│   │   │   ├── intake-assessor.js
│   │   │   ├── frustration-detector.js
│   │   │   └── index.js              # re-exports
│   │   ├── quest/
│   │   │   ├── quest.js
│   │   │   ├── quest-state.js
│   │   │   ├── micro-quest-generator.js
│   │   │   ├── puzzle-template-engine.js
│   │   │   ├── templates/             # story template data
│   │   │   │   ├── shop-templates.js
│   │   │   │   ├── obstacle-templates.js
│   │   │   │   └── ...
│   │   │   └── index.js
│   │   ├── world/
│   │   │   ├── game-map.js
│   │   │   ├── portal.js
│   │   │   ├── zone.js
│   │   │   └── maps/                  # map data
│   │   │       ├── overworld.js
│   │   │       ├── home.js
│   │   │       ├── dream.js
│   │   │       └── ...
│   │   └── character/
│   │       ├── player.js
│   │       ├── companion.js
│   │       ├── npc.js
│   │       └── inventory.js
│   ├── application/
│   │   ├── game-session.js
│   │   ├── interaction-service.js
│   │   └── quest-service.js
│   ├── infrastructure/
│   │   ├── save-manager.js
│   │   ├── claude-dialogue.js
│   │   └── speech-service.js
│   ├── presentation/
│   │   ├── renderer/
│   │   ├── sprites/
│   │   ├── ui/
│   │   └── input/
│   └── index.js                       # entry point, wires everything
├── test/
│   ├── domain/
│   │   ├── learning/
│   │   │   ├── learner-profile.test.js
│   │   │   ├── challenge-generator.test.js
│   │   │   ├── intake-assessor.test.js
│   │   │   ├── frustration-detector.test.js
│   │   │   └── rolling-window.test.js
│   │   └── quest/
│   │       ├── quest-state.test.js
│   │       ├── micro-quest-generator.test.js
│   │       └── puzzle-template-engine.test.js
│   └── application/
│       └── game-session.test.js
├── docs/
│   ├── adaptive-learning-spec.md
│   ├── rpg-quest-spec.md
│   └── architecture-spec.md
├── index.html
├── package.json
└── vitest.config.js                   # or jest, whatever you prefer
```

## Key Design Decisions

### Why DDD for a kids' game?

The Learning domain IS the product. If the stealth assessment is wrong, the game fails educationally. If the quest generation is broken, the game fails as entertainment. These are complex domains with real business rules that need to be tested in isolation.

The rendering is comparatively trivial — if a sprite draws wrong, we fix it. If the frustration detector miscategorizes a struggling kid as bored, we've lost a learner.

### Dependency injection over globals

The current prototype uses global state everywhere (`GAME`, `PLAYER`, `SKILL`, etc.). The new architecture passes dependencies explicitly:

```js
// Old
function generateMathChallenge() {
  const band = SKILL.math.band;  // global
  ...
}

// New
class ChallengeGenerator {
  generate(profile, context, rng) {
    const band = profile.mathBand;  // injected
    ...
  }
}
```

This means we can test ChallengeGenerator with a fake profile, a controlled rng seed, and assert on the output deterministically.

### RNG injection for deterministic tests

Every function that uses randomness takes an `rng: () => number` parameter instead of calling `Math.random()`. In production, pass `Math.random`. In tests, pass a seeded PRNG. This makes challenge generation, micro-quest generation, and intake question selection fully deterministic and testable.

### Events as the test seam

The primary test pattern for the Learning domain:

```js
// Test: frustration detection after repeated failures
test('detects high frustration after 3 wrong in a row', () => {
  const profile = LearnerProfile.default();
  profile.recordPuzzleAttempt({ operation: 'add', band: 3, correct: false, responseTimeMs: 5000 });
  profile.recordPuzzleAttempt({ operation: 'add', band: 3, correct: false, responseTimeMs: 6000 });
  profile.recordPuzzleAttempt({ operation: 'add', band: 3, correct: false, responseTimeMs: 8000 });

  const frustration = FrustrationDetector.assess(profile.rollingWindow, []);
  expect(frustration.level).toBe('high');
  expect(frustration.recommendation).toBe('drop_band');
});
```

No canvas, no DOM, no browser. Pure logic in, assertions out.

### Module system

ES modules (`import`/`export`) throughout. The HTML loads a single bundled entry point, or we use native ES modules with a simple import map for dev and a bundler (esbuild, rollup) for production.

For testing, `vitest` runs ES modules natively in Node — no build step needed for tests.

## Migration Path

We don't rewrite everything at once. The prototype stays playable while we build the new architecture underneath.

1. **Set up project tooling** — package.json, vitest, ES modules
2. **Extract Learning domain** — pull LearnerProfile, ChallengeGenerator, etc. out of dialogue.js into domain/learning/. Write tests.
3. **Extract Quest domain** — define quest data model, PuzzleTemplateEngine. Write tests.
4. **Extract World/Character domains** — pull map data, player movement, NPC logic into domain objects.
5. **Build Application layer** — GameSession wires domains together.
6. **Migrate Presentation** — canvas rendering reads from domain state instead of globals.
7. **Delete old files** — sprites.js, world.js, characters.js, dialogue.js, game.js → replaced by src/

Each step is independently shippable. The game works at every intermediate state.

## Open Questions

- **Bundler choice**: esbuild (fast, simple) vs rollup (more ecosystem) vs none (native ES modules, simplest but slower in browser)?
- **Test runner**: vitest (fast, ES module native, good DX) vs jest (more established) vs bare node:test?
- **State management**: plain objects with methods (current direction) vs immutable state + reducer pattern? Reducers would make the event flow more explicit and testable, but add verbosity.
- **How much of the presentation layer do we test?** Probably none — it's canvas drawing. But the UI logic (dialogue state machine, quest journal state) could be tested if extracted from rendering.
