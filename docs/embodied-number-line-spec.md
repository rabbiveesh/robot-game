# Embodied Number Line + Reef Descent — Spec

The first build spike from the manipulatives roadmap. Grounded in `docs/cra-progression-research.md` and `docs/manipulatives-frontier-research.md`. Domain-first per ADR-003.

## The idea

The number line stops being a popup and becomes **terrain the avatar walks**. The kid IS the token; walking tiles IS counting/adding/subtracting; distance walked = magnitude. The reef biome is its showcase: **depth = the number line**, so it's the reef's spine and stays light elsewhere (the lane discipline in `manipulatives-roadmap.md`).

This replaces the retired popup number-line manipulative (removed in #27). The pure domain `number_line` reducer is **reused** — driven by world movement instead of UI buttons.

## What's reused vs. new

- **Reused as-is:** `robot_buddy_domain::logic::number_line` — `NumberLineSession { position, target, jumps, phase }`, `number_line_reducer` (JumpForward/Backward, clamps, completes on target, ignores moves after Complete). A unit tile-step maps to `JumpForward { n: 1 }` / `JumpBackward { n: 1 }`.
- **Reused patterns:** gate guardian (`gating(id)`, `opening_gate`, `satisfied_gates`, `GateOpened`), portals (`all_portals()`), the rideable mount (Chompy), companion swap/walk-home, chest reward flow.
- **New (domain):** a raw "go to depth N" puzzle constructor (the dive gauge isn't an a+b operation — it's a target on a 0..max line) + a count-on/assessment helper.
- **New (game):** the in-world **number-line track** feature (terrain + a position synced to the avatar), the **dive-gate** (creature-guards-a-gauge), a **bail** affordance, the new deeper reef zone(s), and the **pearls** currency.

## CRA staging → clarity (the fade is the water)

The same track renders differently by the learner's per-operation CRA stage (`profile.cra_stages`), which is *macro-scale concreteness fading*:

| Stage | Render | Reef water |
|---|---|---|
| **Concrete** (the 4yo) | discrete **numbered stepping-stones** — a number *path*; each step lights + dings | crystal clear |
| **Representational** | continuous **marked track** with a 0 anchor; jumps of n | light murk |
| **Abstract** | marks **faded** — estimate position | murky (estimate by feel) |

Number PATH before number LINE (research-backed for the youngest): the Concrete render is discrete countable stones, not a continuous length, to dodge the tick-mark/off-by-one error.

## Two surfaces (the cadence)

1. **Ambient stepping-stones** — terrain within a reef zone you hop to get around. Ungated, frequent, satisfying, asks almost nothing. Drives the `number_line` reducer for silent assessment (where you aim vs. land, count-on vs. recount) but never blocks.
2. **Dive-gate** — a **creature guards a gauge** at a chokepoint. Interacting starts a number-line target task ("dive to mark N"); landing on N opens the descent to the next zone. This is the deliberate, assessed challenge.

## Reef descent

The reef becomes a chain of progressively deeper zones (separate maps, per the biome/portal architecture + themed-map hooks):

- **3 zones to start** (existing reef → one new deeper zone is spike-1; full chain: Shallows → Coral Gardens → Trench), built so a **4th drops in** via hooks.
- Each descent is gated by a **creature-guards-a-gauge** dive-gate (number-line task at that zone's band). Solve → the creature steps aside (existing gate pattern) → descend.
- **Ride Chompy to descend** — the rideable shark is the descent mount (reuses the riding mechanic). Mount-only for spike-1; other buddy abilities (dolphin leap, octopus murk-reveal) are later increments.
- **Bail** — every gate attempt has a clear "swim back / not yet" that exits cleanly to the world: no penalty, the creature stays, encouraging line, return anytime. Bailable mid-attempt. Spec'd as a **general challenge affordance**, not reef-only.
- **Pearls** — a reef-local currency. Zone chests pay pearls; later a reef trader sells **gear** (snorkel, flippers, lantern, pearl-pouch) — pearls' sink. Dum Dums stay the global currency (recruit buddies, overworld shop, tolls).

## Invariants (hard rules)

- **No oxygen/air timer.** Depth is *spatial* progress, never timed (would violate "never time-pressure a child"). No breath/clock gear.
- **Fail gracefully.** Overshoot a target → walk back (that's subtraction); a wrong gauge read → "let's recount," never a red X. Bail is always available.
- **No labels.** Depth self-limits by difficulty (a not-ready kid can't read the deep gauge yet) — no "Band 3", no stage badge.
- **Seeded RNG, domain-pure.** New domain logic takes `&mut impl Rng`; headless-tested.
- **Lane, not spine** (except here). The number line is the reef's spine *because depth fits the lane*; elsewhere it stays ambient/occasional.

## Build order (PRs, TDD, merge on green CI)

1. **Spec + domain reducer** — this doc + the raw-target puzzle constructor & count-on/assessment helper for `number_line`, tests first. *(this PR)*
2. **Ambient stepping-stones** in the existing reef — terrain + avatar-synced track + silent assessment. Headless tests for the walk→land→signal flow.
3. **Reef descent** — one new deeper zone + a dive-gate (creature-guards-a-gauge) + ride-to-descend + bail + pearls. Headless tests for descend/solve/bail/reward.

Later increments: reef trader + gear; Zone 3 (Trench) + quest-payoff treasure; octopus murk-reveal buddy (proves "buddies change play"); logbook; 4th zone via hooks.
