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

Higher-risk, higher-upside. Some are shipping tools we'd adapt; some are research ideas. **Evidence varies — flagged honestly.** This tier is being expanded by the running bleeding-edge research pass.

| Idea | RPG mechanic | What's novel | Maturity | Notes |
|---|---|---|---|---|
| **Embodied number line** | The **player's own avatar walks the number line** — the world *is* the line; movement *is* arithmetic | makes the number line a place you inhabit, not a UI; embodied-cognition leverage | speculative-but-grounded | Maximally RPG-native; pair w/ path-first rule |
| **Numberblocks-style quantity creatures** | Companions/enemies whose **body visibly = their value** (a "7" is 7 blocks); split a 7 into 3+4 | representational *consistency* — the character can't lie about its number | promising (huge engagement evidence, learning evidence thinner) | Composes with our follower system |
| **Draggable notation (Graspable Math / DragonBox)** | Glowing runes/crystals that **are** the equation; drag to combine, balance two sides to free a creature | symbols become physical objects you manipulate | promising (DragonBox engagement strong; transfer contested) | "Isolate the box" = solve for unknown |
| **Magnitude-comparison / ANS duels** | "Which treasure pile is bigger?" snap-judgment duels that adapt to your acuity | trains the approximate number system | contested (ANS→symbolic transfer is disputed — flag) | Fast, wordless, good for pre-readers |
| **Spatial-temporal / wordless puzzles (ST Math / JiJi)** | Language-free visual puzzle rooms; manipulate to make it work | zero reading load; pure visual reasoning | promising (ST Math has efficacy studies) | Ideal for atypical/pre-literate kids |
| **Open manipulative sandbox (Mathigon Polypad)** | A "tinker bench" room with free manipulatives, low-floor/high-ceiling | unstructured exploration; kid-led | promising | Risk: violates "guided" rule — gate w/ light goals |
| **Perceptual-learning fluency (Kellman PLM)** | Fast match-the-representations rounds (array ↔ numeral ↔ ten-frame) | builds automaticity in mapping representations | promising (PLM has strong lab evidence) | A great *fading* / R↔A bridge drill |
| **Variation-theory sequences** | Puzzle chains that vary exactly one dimension at a time | conceptual contrast as the teacher | promising (theory-strong) | A way to *author* puzzle progressions |

---

## How a manipulative becomes a mechanic (the test)

For each candidate, before building, answer: **"If the kid already knew the answer, would this still be an interesting action?"**
- Tiling an orchard, balancing a bridge's weight, sharing loot fairly → **yes** (strategic/satisfying) → math is the mechanic. Build it.
- Tapping the right numeral on a flashcard skinned as a chest → **no** → it's broccoli. Redesign.

## Suggested build sequence

1. **P0 — number path + ten-frame**, with **concreteness fading** wired as the C→R transition (this is also the CRA-engine spike). Targets the 4yo immediately.
2. **P1 — subitizing, counters, part-part-whole, rekenrek, true number line, base-ten** — the early-number core.
3. **Pick ONE Tier-3 spike** to prototype early as the "experimental ground" proof — strongest candidates today: **embodied number line** (RPG-native), **Numberblocks-style quantity creatures** (composes with followers), or **wordless ST-Math-style rooms** (atypical-friendly). The research pass will sharpen this pick.
4. **P2/P3** as operations come online (arrays/area + bar model when multiplication/division matter).

## Open questions

- One generic "manipulative panel" component vs. bespoke per mechanic? (The CRA-Integrated layout argues for a shared 3-pane fade component.)
- How much free-play sandbox vs. guided — where's the line for our youngest?
- Which Tier-3 idea is the first experimental spike? (await research pass)

## Pending enrichment

A bleeding-edge research pass (frontier manipulatives, dynamic notation, ANS trainers, embodied math, wordless/spatial systems, recent 2018–2025 edtech/HCI) is running. Its ranked shortlist of 8–12 "experimental goodies" will be folded into Tier 3 with evidence ratings and criticisms, and will settle the first-spike pick above.
