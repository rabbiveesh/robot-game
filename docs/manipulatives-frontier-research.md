# Frontier Manipulatives — Research Brief

Bleeding-edge / experimental math representations and interaction mechanics for the RPG (ages 4–10, incl. dyscalculia/neurodivergent), to populate the **Experimental tier** of `docs/manipulatives-roadmap.md`. Companion to `docs/cra-progression-research.md`.

> **Provenance.** Synthesized from 5 parallel topic-focused research agents (each ran live WebSearch + WebFetch over real sources). Evidence is rated **proven / promising / speculative** and hype is flagged. (The deep-research workflow harness failed twice on this query — synthesis stub, then a scope-agent retry-cap abort — so this was gathered with direct research agents instead.)

---

## 0. The one lesson everything converged on

Five independent searches hit the **same wall**: **the manipulative is necessary but not sufficient — the embedded structure / guidance / progression is the active ingredient.**

- Cuisenaire rods get a medium effect (d≈0.55) **only at high fidelity to Gattegno's pedagogy**; the meta-analysis's own headline is "the rods got adopted, the pedagogy didn't."
- Polypad (6M+ users) and Numberblocks (BAFTA-nominated) have **near-zero published efficacy data** on the tools themselves — only borrowed credibility from the principles.
- **Kirschner, Sweller & Clark (2006):** pure-discovery / minimal-guidance instruction is *worse* for novices (which all our kids are) — free exploration overloads working memory and breeds misconceptions.
- DragonBox produces the biggest engagement but **doesn't transfer** — kids master the game, not the notation.
- **Catch Up Numeracy** (EEF RCT, n=1,181): its gains came from the *dose of 1:1 attention*, not the method — a same-time generic-tutoring arm did *better*. A second trial found no impact.

**Implication for us — and it's a real edge:** our domain already has the thing the evidence says is the active ingredient — **adaptive scaffolding dials, per-operation CRA staging, frustration detection, invisible assessment.** The manipulatives are the visible surface; the *invisible guidance* is what makes them work. Build the progression, not just the toy. And heed the Catch Up warning: **Sparky's engaged 1:1 attention may itself be doing much of the work — don't over-attribute gains to any specific math content** (validate against an equal-engagement comparison before believing a mechanic "teaches").

---

## 1. Number-line estimation is the load-bearing mechanic (strongest evidence in the whole sweep)

Across the number-sense agents, one finding dominates:

- **Number-line estimation is the single strongest correlate of math achievement** — Schneider et al. (2018, *Child Development*) meta-analysis, 263 effects, N=10,576, **r=.443**, rising with age (esp. fractions). [siegler.tc.columbia.edu](https://siegler.tc.columbia.edu/wp-content/uploads/2019/12/4027Reading-Schneider-etal-2018.pdf)
- **Symbolic** magnitude beats **non-symbolic**: Schneider et al. (2017, *Dev. Sci.*) symbolic comparison r=.302 vs non-symbolic r=.241.
- **Calcularis/Dybuster** (ETH Zürich; number-line-centered adaptive dyscalculia trainer) is the best-evidenced intervention found — a true RCT *with an active control* (Kohn/Rauscher 2016, n=138) beat both controls on subtraction & 0–10 number line; gains **stable at 3-month follow-up** (Rauscher/von Aster 2020, diagnosed DD). **Promising, leaning proven for narrow near-transfer targets.** [PMC4921479](https://pmc.ncbi.nlm.nih.gov/articles/PMC4921479/)
- **Semideus** (Kiili/Ninaus/Moeller) — number-line game; n=95 4th-graders gained conceptual rational-number knowledge, **low-prior-knowledge kids most**, and **in-game number-line metrics validly predicted the paper posttest** (stealth assessment). [ScienceDirect](https://www.sciencedirect.com/science/article/pii/S0360131518300125)

This reinforces our existing P1 number-line build AND the embodied version below. It also doubles as **invisible assessment**, matching our invariant.

---

## 2. Embodiment survives a flat 2D screen — *if* the motion is congruent

The make-or-break question for us (we're a flat top-down game, no whole-body):

- **Walk-the-number-line** (first-graders physically walk to a number's position): embodied group beat control with **partial transfer to untrained skills**, and **life-size walking beat the tablet version** for symbolic estimation. [ScienceDirect](https://www.sciencedirect.com/science/article/abs/pii/S2211949313000197)
- **Johnson-Glenberg** (the key guide for screens): embodiment value = sensorimotor engagement × **gesture–content congruency** × immersion — and **congruency matters more than magnitude.** A finger swipe teaches if the motion *maps to the concept*; a big arm wave teaches nothing if it doesn't. [Frontiers](https://www.frontiersin.org/journals/robotics-and-ai/articles/10.3389/frobt.2018.00081/full)
- **Goldin-Meadow:** gesture grows math understanding — and **simultaneous** speech+gesture drives transfer (speech-then-gesture doesn't). **Proven, lab-robust.** [Psych Science 2009](https://journals.sagepub.com/doi/abs/10.1111/j.1467-9280.2009.02297.x)
- **Nathan (GEMC):** "directed action biases cognition" — but action is **insufficient without coordinated language** to formalize. [Springer](https://cognitiveresearchjournal.springeropen.com/articles/10.1186/s41235-016-0040-5)
- **Abrahamson's Mathematics Imagery Trainer:** move to keep the screen green → discover a ratio rule motorically before symbols; ported to iOS. Caveat: re-description to formal math "required heavy-handed intervention" — discovery alone isn't enough. [PMC5256464](https://pmc.ncbi.nlm.nih.gov/articles/PMC5256464/)

**Our payoff:** a top-down RPG is *already a body moving on a 2D plane*. **The avatar walking a quest-path-as-number-line is the embodied number line** — congruency is perfect (movement = magnitude), it converts the one embodied intervention with direct child-age evidence into movement we already have, and it's nearly free.

---

## 3. Representational consistency — "a representation that can't lie about the quantity"

- **Numberblocks:** each character's body IS its quantity (N stacked unit cubes + a floating symbol). Part-whole, cardinality, subitizing, factors, square/triangular numbers become *intrinsic and self-evident* — you cannot draw a "wrong" Five. **The cleanest popular embodiment of "the math IS the body."** Evidence-light on the show itself (no RCT found), but the underlying principles are well-evidenced; composes beautifully with our follower system. Keep cubes **schematic** (see §5). Scales poorly past ~20 / no fractions. [Wikipedia](https://en.wikipedia.org/wiki/Numberblocks) · [NCETM](https://www.ncetm.org.uk/classroom-resources/ey-numberblocks-support-materials/)

---

## 4. Dynamic / draggable notation — promising but transfer is the silent failure

- **Graspable Math / "From Here to There!"** — drag terms across the `=` (auto-negates), drag factors together to combine; the UI physics *enforces* legal algebra. **The only notation-as-object tool with a real RCT** (IES, n=1,850 grade-7): beat control, **Hedge's g≈0.135** (small, near-transfer only). Gesture-as-operation. [PMC10125888](https://pmc.ncbi.nlm.nih.gov/articles/PMC10125888/)
- **DragonBox** — "isolate the box," cards fade to numerals. Bigger effect in that same RCT (g≈0.269) **but** the canonical "engagement-not-learning" case: independent studies find no algebra gain, and the documented failure mode is **hiding the real notation so kids master the game, not the math.** [EdSurge](https://www.edsurge.com/news/2016-03-13-enter-the-dragonbox-can-a-game-really-teach-third-graders-algebra)
- **Algebra tiles** — area/zero-pair model; older, smaller studies favor it **especially for negatives and for students with LD**. Not a silver bullet without explicit symbol links. [SAGE 2025](https://journals.sagepub.com/doi/10.1177/01626434241263055)

For our age band, only the **arithmetic-level** slice is in scope (drag-to-combine / commutativity; zero-pair annihilation for early integers). **Lesson: keep the real symbols visible from the start** — don't repeat DragonBox's broccoli-hiding inversion.

---

## 5. Perceptual richness: schematic beats lavish (proven, and it constrains all the above)

- **Kaminski & Sloutsky / Uttal / Willingham:** perceptually rich, "realistic" materials grab attention but **draw it away from the math and hurt transfer**; bland/schematic representations often transfer better. The "looks like a toy" dual-representation trap. **Proven, decades, multiple labs.** [Willingham/AFT](https://www.aft.org/ae/fall2017/willingham) · [Kaminski & Sloutsky](https://cpb-us-w2.wpmucdn.com/u.osu.edu/dist/1/56827/files/2018/06/Kaminski__Sloutsky-_Rep_Math-final-2bdsrfj.pdf)
- Nuance: a meta-analysis finds richness can *aid transfer* in some cases — it's a tradeoff dial, not a one-way rule. Pair with **concreteness fading** (our CRA engine).

**Design rule:** make the math-relevant feature (length, count) the most salient thing; strip decorative detail while a thing is doing math. (Directly tensions with "cute Numberblocks creatures" — resolve by keeping the *cubes* schematic even if the face is friendly.)

---

## 6. Pedagogy mechanics worth gamifying

- **Productive Failure (Kapur)** — try-before-taught; meta-analysis Sinha & Kapur (2021), ~50 studies, positive for **conceptual understanding + transfer** without hurting procedural. **Proven, with a boundary condition: the consolidation/reveal phase must be strong.** Passes the Broccoli Test beautifully — the struggle + "aha" reveal IS the gameplay. Most evidence is older kids. [RER 2021](https://journals.sagepub.com/doi/full/10.3102/00346543211019105)
- **Desirable Difficulties (Bjork): spacing / interleaving / retrieval / generation** — the **most mature evidence** in the whole sweep, and it's *infrastructure not content*: bolt a scheduler onto our band/profile/encounter system so facts re-surface at expanding intervals and operation types interleave across the world. Avoids drill-and-kill by construction. [Bjork & Bjork 2011](https://bjorklab.psych.ucla.edu/wp-content/uploads/sites/13/2016/04/EBjork_RBjork_2011.pdf)
- **Perceptual Learning Modules (Kellman)** — fast, low-verbal "match the representations" trials build automatic structure recognition; large fluency gains with retention/transfer (e.g., equation-solving 28s→12s, held at 2 weeks). **Almost literally a matching minigame already** and a perfect C↔R↔A *fluency* bridge. [PMC6124488](https://pmc.ncbi.nlm.nih.gov/articles/PMC6124488/)
- **Variation Theory / Intelligent Practice (Marton; Barton)** — an *authoring discipline*, not a player mechanic: vary one dimension at a time so attention lands on what changed. Use it as the generator constraint in `challenge/` and `quest/` micro-quest gen. Theory-strong, thin RCT base. [variationtheory.com](https://variationtheory.com/)
- **Number-sense routines (WODB, Splat!, Estimation180, Number Talks, Notice&Wonder)** — popular and sensible but **mostly not rigorously evidence-backed as named interventions** (Number Talks/Boaler is actively contested). **WODB** ("which gem is cursed?" — multiple defensible answers, the *reasoning* is the play) and **Splat!** (covered-treasure part-whole) are the two worth stealing; treat as flavor, not proven drivers. [Hechinger on Boaler](https://hechingerreport.org/proof-points-stanfords-jo-boaler-book-math-ish-critics/)

---

## 7. Wordless / spatial productive-struggle (great pattern, oversold product)

- **ST Math / JiJi** — radically wordless; a wrong move animates a graceful in-world consequence the child reads without text, then retries. This is *exactly our Invariant #7 (fail gracefully) at industrial scale*. **But** the strongest independent test (Rutherford, RCT, 52 schools, 2 years) found **no significant effect**, and the US DoE WWC lists **no** computer-assisted elementary math intervention as having positive effects. **Build the mechanic pattern; don't promise test-score lift.** [Nautilus](https://nautil.us/does-a-cartoon-penguin-make-math-education-great-again-236360)
- **Tanton's "Math Without Words"** — infer the rule from the visual alone; built-in difficulty spread (5-year-olds to grownups). Speculative as efficacy; the *wordless* principle helps pre-readers/EAL. Keep an invisible hint ramp so "wordless" never tips into "opaque." [jamestanton.com](https://www.jamestanton.com/?p=1605)

---

## 8. What to deliberately NOT lean on (hype flags)

- **Non-symbolic ANS dot-comparison transfer** — the originating lab **failed to replicate its own result** (Szkudlarek, Park & Brannon 2021, Bayesian ~7× favoring null); symbolic > non-symbolic across meta-analyses. Keep "which swarm is bigger?" as fun onboarding/duels — **never as the assessment or progression backbone.** [PMC7805575](https://pmc.ncbi.nlm.nih.gov/articles/PMC7805575/)
- **DragonBox-style notation-hiding** — engagement ≠ learning; keep symbols visible.
- **Finger-gnosis training** — predicts numeracy but *training* **failed to beat an active control**; researchers want kids to *abandon* finger counting. Scaffolding-to-fade at most. [PMC7105809](https://pmc.ncbi.nlm.nih.gov/articles/PMC7105809/)
- **Mental abacus / Soroban** — real arithmetic gains **but no broad cognitive transfer**; domain-specific, needs sustained training. [Barner 2015](https://langcog.stanford.edu/papers_new/barner-2015-childdev.pdf)
- **Osmo / AR tangibles** — needs hardware, off-table for a pure-WASM build; benefit often the *feedback*, not the tangibility.
- **Trachtenberg / generative-LLM autonomous tutoring** — fun/promising for authoring, unproven for autonomous child tutoring.

---

## 9. Ranked shortlist — experimental goodies to prototype

Ranked by **evidence × RPG fit × fits-our-architecture**. Top of list = first spikes.

1. **Embodied number line — the avatar walks the line.** Best evidence (number-line estimation is THE correlate; walk-the-line has child-age transfer) × best RPG fit (nearly free given movement) × doubles as stealth assessment × scales 4–10 (whole numbers → fractions). **The clear first spike.**
2. **Numberblocks-style "body = quantity" creatures.** Representational consistency; composes with the follower system; builds the most-valued early-number skills. Keep cubes schematic. **Strong second spike.**
3. **Calcularis-style adaptive skill-tree as the engine *under* the mechanics.** Best-evaluated adaptive numeracy system; maps onto our reducer/band/profile architecture; personalizes without labels.
4. **Perceptual-Learning representation-matching drill** ("scanner tuning": dots ↔ numeral ↔ ten-frame ↔ number-line). Real fluency evidence; literally a minigame; the C↔R↔A fluency bridge; un-timed-*feeling* (no visible clock).
5. **Desirable-difficulties scheduler** (spacing + interleaving + retrieval) bolted onto encounters/bands. Most mature evidence; infrastructure not content; cheap, high payoff.
6. **Productive-Failure puzzle loop + Variation-Theory authoring.** Try-first → tinker/fail safely → reveal-that-lands. Evidence-backed; Broccoli-Test-perfect; variation theory generates the ladders.
7. **Cuisenaire–Gattegno rod-matching microworld** (no numerals, match/combine to a target bar). Best-*evidenced* manipulative here (d≈0.55) — but **ship the structured progression, not just rods.**
8. **Graspable-Math "drag across the equals barrier"** (scaled to commutativity/combining; zero-pair annihilation for early integers). Only notation-as-object mechanic with an RCT; gesture-as-operation; **keep symbols visible** (avoid the DragonBox trap).
9. **Wordless productive-struggle rooms** (ST-Math/Tanton pattern): wrong move animates a consequence, retry freely; serves pre-readers/atypical natively. Build the pattern; don't promise score lift; keep an invisible hint ramp.
10. **WODB / Splat! flavor puzzles** ("which gem is cursed?", covered-treasure part-whole). Cheap, reasoning-rewarding, kid-safe; flavor not driver.

---

## 10. Key sources

Number sense / number line / dyscalculia: Schneider et al. 2018 (number-line meta) · Schneider et al. 2017 (symbolic>non-symbolic) · Kohn/Rauscher 2016 & Rauscher/von Aster 2020 (Calcularis RCTs) · Kiili/Moeller/Ninaus 2018 (Semideus) · Szkudlarek/Park/Brannon 2021 (ANS non-replication) · Szűcs & Myers 2017.
Embodiment: Walk-the-number-line (Learning & Instruction) · Johnson-Glenberg 2018 · Goldin-Meadow 2009 · Nathan GEMC 2016 · Abrahamson MIT-P (PMC5256464).
Notation: Ottmar/Landy FH2T RCT (PMC10125888) · DragonBox critiques (EdSurge/Cosulich) · Algebra tiles (SAGE 2025).
Representation/consistency & richness: Numberblocks (NCETM) · Willingham (AFT) · Kaminski & Sloutsky.
Pedagogy: Sinha & Kapur 2021 (productive failure) · Bjork & Bjork 2011 (desirable difficulties) · Kellman PLM (PMC6124488) · Marton/Barton variation theory · Kirschner/Sweller/Clark 2006.
Sandboxes: Mathigon Polypad/Amplify · Cuisenaire–Gattegno meta-analysis (Frontiers 2022) · Papert constructionism.
Cautionary: Catch Up Numeracy (EEF) · ST Math Rutherford RCT / WWC (Nautilus).
