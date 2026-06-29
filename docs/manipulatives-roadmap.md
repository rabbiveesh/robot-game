# Manipulatives Roadmap — the full host (+ experimental sandbox)

The build backlog of math representations/manipulatives for the RPG, organized by maturity tier. Each is framed as a **game mechanic**, not a popup — every entry must pass the Broccoli Test (would it still be a real interaction with the math removed? if yes, the math isn't the mechanic — cut or redesign).

Grounded in `docs/cra-progression-research.md`. The **Experimental tier is a deliberate sandbox** for out-there ideas; a bleeding-edge research pass is running to enrich it (see "Pending enrichment" at the bottom).

## How to read this

Each entry: **RPG mechanic** · *teaches* (skill/operation) · **stage** (C/R/A) · **age** · **maturity** (proven / promising / speculative) · **priority** (P0 build-first … P3 someday).

## Cross-cutting design rules (from the CRA research)

These apply to *every* manipulative we build:

1. **Concreteness fading is the transition engine, not a stage flip.** Within one operation, fade the *same* mechanic: real manipulative → drawing of it → faint/ghost drawing → numerals, with an explicit "the 3 stones *become* the numeral 3" linking beat. This is how a kid moves C→R→A.
2. **Number PATH before number LINE** for the youngest (discrete countable boxes before continuous length) — avoids tick-mark/off-by-one errors.
3. **Schematic, not lavish.** Low perceptual richness while a thing is doing math (a counter is a plain disc, not a detailed character) — rich toys get treated as toys, not symbols.
4. **Guided + immediate in-world feedback.** The bags fill, the bridge wobbles, the merchant frowns. Manipulatives help *most* with guidance, *least* as unguided sandbox.
5. **Per-operation, reversible staging.** Readiness = rolling accuracy at the current representation for *that* operation; drop back a level on a frustration signal.

---

## Tier 1 — Core / proven (P0–P1, build first)

| Manipulative | RPG mechanic | Teaches | Stage | Age | Maturity | Priority |
|---|---|---|---|---|---|---|
| **Number path** | Stepping-stones the avatar hops, one stone per number; count-on/back = hops | counting, 1-to-1, +/− within 20 | C→R | 3–6 | proven (practitioner) | **P0** (the 4yo headline) |
| **Ten-frame** | A 10-cell battery / rocket-fuel rack / egg carton you fill | subitizing, five/ten anchors, bonds-to-10 | C→R | 4–7 | proven | **P0** |
| **Subitizing flash** | A gem/star cluster flashes; grab the matching count fast | instant quantity, fact-fluency foundation | R | 3–6 | proven | **P1** |
| **Counters / set model** | Collect & sort items into the right count/bin | counting, cardinality, +/−, sharing→÷ | C | 3–7 | proven | **P1** |
| **Part-part-whole / number bonds** | Split a treasure pile into two bags; a *covered* bag = missing addend | decomposition, +/− inverse, fact families | R | 5–9 | proven | **P1** |
| **Rekenrek** | A 2×10 bead gate (5 red/5 white) you slide to make a number to pass | number sense via 5/10 structure, doubles, bonds | C | 5–8 | proven | **P1** |
| **Number line (true)** | A measured track; frog/jump to land on a target (bridge planks) | magnitude, jumps of +n/−n, later fractions/negatives | R→A | 6+ | proven | **P1** (after path) |
| **Base-ten blocks** | Trade 10 coins → 1 gem (regrouping); stack rods/flats | place value, multi-digit +/− w/ regrouping | C→R | 6–10 | **proven (CRA's strongest win)** | **P1** |
| **Array / area model** | Tile a floor / plant an orchard grid (rows × cols) | multiplication, division, distributive | C→R | 7–10 | proven | **P2** |
| **Bar / tape model** | Compare two ropes/bridges — which longer, by how much | word problems, comparison, multi-step | R | 7–10+ | proven | **P2** |
| **Cuisenaire rods** | Colored logs you combine to span a gap of a given length | length-magnitude, part-whole, bridge to number line | C | 5–10 | proven | **P2** |

---

## Tier 2 — Emerging / promising (P2–P3)

| Manipulative | RPG mechanic | Teaches | Stage | Age | Maturity | Priority |
|---|---|---|---|---|---|---|
| **Numicon-style shapes** | Pegged hole-shapes you fit into a slot that sums to a target | number-as-shape, pattern, bonds | C→R | 4–8 | promising | P2 |
| **Slavonic abacus / bead string** | 100-bead grid colored in 5s; slide to a quantity | counting to 100, place value, structure | C | 5–9 | promising | P3 |
| **Hundred chart / Gattegno** | A town grid navigated by +10/+1 moves | place value, skip-count, number relationships | R | 6–9 | promising | P2 |
| **Fraction circles / strips** | Share a pie/pizza; fit fraction strips to a whole | fractions, equivalence, part-whole | C→R | 7–10 | promising | P3 |
| **Double number line / ratio table** | Scale a recipe; trade rates at the market | ratio, proportion, scaling | R→A | 9–10+ | promising | P3 |
| **Dot patterns (dice/dominoes)** | Pattern-match gates; build a domino bridge | subitizing, addition patterns | C→R | 3–7 | promising | P2 |
| **Number-sense *routines*** (Splat!, Which-One-Doesn't-Belong, Estimation jars, Number Talks) | "Sparky wonders…" micro-prompts; estimation jars; odd-one-out sorting | flexible reasoning, estimation, justification | R/A | 5–10 | promising (routines, not manipulatives) | P2 |

> Routines are *prompt patterns*, not objects — cheap to build, generative, and great for the "no wrong answer" feel. Strong fit for atypical thinkers who reason before they compute.

---

## Tier 3 — Experimental / frontier (the sandbox)

Higher-risk, higher-upside. Now **grounded** in `docs/manipulatives-frontier-research.md` (5 research agents, evidence-rated, hype-flagged). Ordered by the brief's ranked shortlist (evidence × RPG fit × fits-our-architecture).

> **The lesson the whole frontier sweep converged on:** the manipulative is *necessary but not sufficient* — the embedded structure/guidance/progression is the active ingredient (Cuisenaire needs Gattegno's pedagogy; sandboxes fail novices per Kirschner/Sweller; DragonBox engages but doesn't transfer; Catch-Up's gains were the 1:1 *attention*, not the method). **Our adaptive scaffolding / CRA staging / frustration dials ARE that active ingredient** — that's our edge. Build the progression, not just the toy. And don't over-credit a mechanic for what Sparky's attention may be doing.

| # | Idea | RPG mechanic | Evidence | Notes |
|---|---|---|---|---|
| 1 | **Embodied number line** (FIRST SPIKE) | The **avatar walks the line** — a quest path *is* a number line; distance walked = magnitude | **proven-ish** — number-line estimation is THE strongest math correlate (r=.443, N>10k); walk-the-line shows child-age transfer; congruency (motion=magnitude) survives a flat screen | Nearly free given movement; doubles as stealth assessment; scales 4–10 |
| 2 | **Quantity creatures** (2nd spike) | Companions/enemies whose **body = N unit cubes** (no numeral needed); combine/split = add/subtract/factor | **promising** — representational *consistency* (can't misrepresent the quantity); principles well-evidenced, show itself unstudied | Composes with follower system; **keep cubes schematic** (richness hurts transfer) |
| 3 | **Adaptive skill-tree engine** | (under the mechanics) Bayesian learner-model routes stay/regress/advance over a skill graph | **promising→proven** (Calcularis RCTs, 3-mo retention) | Maps onto our reducer/band/profile arch; personalizes without labels |
| 4 | **Perceptual-learning fluency drill** | "Scanner tuning": snap-match dots ↔ numeral ↔ ten-frame ↔ number-line | **promising→proven** (Kellman PLM: large fluency gains, retained) | The C↔R↔A *fluency* bridge; un-timed-**feeling** (no visible clock) |
| 5 | **Desirable-difficulties scheduler** | Facts re-surface at expanding intervals; operation types interleave across the world | **proven** (Bjork; most mature evidence here) | Infrastructure not content; bolt onto encounters/bands; cheap |
| 6 | **Productive-failure loop** | Try-first → tinker/fail safely → an NPC reveal that *lands* | **proven** w/ boundary cond. (Kapur meta) — consolidation reveal must be strong | Broccoli-Test-perfect; author ladders via **variation theory** |
| 7 | **Cuisenaire–Gattegno rods** | Numeral-free rod creatures; match/combine to a target bar (built-in equation) | **proven (best-evidenced manipulative, d≈0.55)** — fidelity-dependent | **Ship the progression, not just rods** |
| 8 | **Draggable notation** | Drag glyphs across a shimmering `=` barrier (auto-sign-flip); zero-pair annihilation | **promising** (Graspable Math RCT g≈0.135; only notation-as-object tool with an RCT) | Scale to commutativity/integers for our ages; **keep symbols visible** (avoid DragonBox trap) |
| 9 | **Wordless struggle rooms** | A wrong move animates a graceful in-world consequence; retry freely, no text | **pattern proven, product oversold** (ST-Math's strongest RCT nulled) | Serves pre-readers/atypical; build the pattern, keep an invisible hint ramp |
| 10 | **WODB / Splat! flavor puzzles** | "Which gem is cursed?" (many right answers); covered-treasure part-whole | **promising/practitioner** (not RCT-proven) | Flavor not driver; reasoning is the play → Broccoli-safe |

**Deliberately NOT building on** (hype flags from the brief): non-symbolic ANS dot-comparison as an assessment/progression backbone (originating lab failed to replicate transfer — keep "which swarm is bigger?" as fun *onboarding* only); DragonBox-style notation-hiding; finger-gnosis training (failed to beat active control); mental abacus (no broad transfer); Osmo/AR tangibles (needs hardware, off-table for WASM).

---

## How a manipulative becomes a mechanic (the test)

For each candidate, before building, answer: **"If the kid already knew the answer, would this still be an interesting action?"**
- Tiling an orchard, balancing a bridge's weight, sharing loot fairly → **yes** (strategic/satisfying) → math is the mechanic. Build it.
- Tapping the right numeral on a flashcard skinned as a chest → **no** → it's broccoli. Redesign.

## Suggested build sequence

1. **P0 — number path + ten-frame**, with **concreteness fading** wired as the C→R transition (this is also the CRA-engine spike). Targets the 4yo immediately.
2. **P1 — subitizing, counters, part-part-whole, rekenrek, true number line, base-ten** — the early-number core.
3. **First experimental spike — the embodied number line** (Tier 3 #1). The research settles it: number-line estimation is the single strongest correlate of math achievement, the embodied "walk-the-line" variant has direct child-age evidence, the congruency (motion = magnitude) survives a flat screen, it doubles as stealth assessment, and it's *nearly free* given our movement system. **Second spike: quantity creatures** (#2, composes with followers).
4. **P2/P3** as operations come online (arrays/area + bar model when multiplication/division matter).

## Open questions

- One generic "manipulative panel" component vs. bespoke per mechanic? (The CRA-Integrated layout argues for a shared 3-pane fade component; the frontier "build 2–3 deeply-integrated reusable manipulatives, not a library" finding agrees.)
- How much free-play sandbox vs. guided — where's the line for our youngest? (Frontier answer: *constrained microworlds with embedded goals*, never an open canvas — discovery fails novices.)
- Validating attribution: heed the Catch-Up warning — before crediting any mechanic, sanity-check it isn't just Sparky's 1:1 attention doing the work.

## Frontier findings

The bleeding-edge pass is **done** — see `docs/manipulatives-frontier-research.md` for the full evidence-rated brief (number line as load-bearing mechanic; embodiment-on-a-flat-screen; representational consistency; draggable notation's transfer gap; perceptual-richness caution; pedagogy mechanics to gamify; the contested ANS-transfer debate; and what *not* to lean on). Its ranked shortlist of 10 experimental goodies is folded into Tier 3 above.
