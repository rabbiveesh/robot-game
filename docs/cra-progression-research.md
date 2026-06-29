# CRA Progression — Research Brief & Design Goodies

> **Provenance.** This brief was produced by a deep-research pass (5 search angles → 23 sources fetched → 105 claims extracted → top 25 claims put through 3-vote adversarial verification; **25/25 survived, 0 killed**). The automated *synthesis* step returned a stub, so this document was hand-assembled from the verified claims and their verifier corroborations. Confidence tags and sources are carried through. Where the verification surfaced genuine disagreement between sources, it's flagged rather than smoothed over.

Goal: make our CRA system (Concrete → Representational → Abstract, tracked per operation) actually *useful* — concrete stage definitions, evidence-based ways to move a kid from one stage to the next, and a catalog of representations to build (number lines for the 4-year-old called out specifically).

---

## 0. Executive summary

- **CRA = Bruner relabeled.** Concrete/Representational/Abstract maps one-to-one onto Bruner's *enactive / iconic / symbolic* modes; Singapore's "Concrete-Pictorial-Abstract" (CPA) is the same framework under different words. **(high confidence)**
- **CRA works, and its evidence base is strongest exactly where our kids live** — computation/operations for learners who struggle with math, including LD/dyscalculia. It's a formally designated **evidence-based practice**, with a 2025 meta-analysis reporting an enormous effect for the approach. **(high confidence)** — but note the effect sizes come heavily from special-ed single-case designs (see caveats).
- **The single most useful transition technique is "concreteness fading"**: start concrete, then *explicitly and gradually* fade to representational then abstract — which beats both concrete-only and abstract-only for **transfer** to new problems. **Direction matters** (concrete→abstract, not the reverse). **(high confidence)**
- **Per-operation, non-linear CRA is legitimate.** A child can be abstract for addition and concrete for division; theory says the modes overlap rather than running in strict chronological lockstep. Our per-operation CRA tracking is well-aligned with the research. **(high–medium; one genuine tension, see §2)**
- **For a 4-year-old, build a number _path_ before a number _line_.** This is the highest-leverage finding for the youngest: discrete, countable boxes (a path) avoid the classic number-line counting errors. **(high confidence, practitioner-strong)**

---

## 1. The levels themselves

**Concrete (enactive).** The child manipulates real, movable objects — counters, blocks, fingers, beads. The concrete level is repeatedly described as *the most crucial level for developing conceptual understanding*; it's where the operation is an action you *do*, not a symbol you read. **(high)**

**Representational / Pictorial (iconic).** The objects become pictures/drawings of objects — tally marks, dots, a drawn ten-frame, a bar. The quantity is still depicted, but no longer physically handled. **(high)**

**Abstract (symbolic).** Numerals and operators only: `3 + 2 = 5`. The symbols stand in for quantity with nothing depicting it. **(high)**

**Framework lineage.**
- CRA originates in **Bruner & Kenney (1965/66)** "modes of representation": enactive (action), iconic (image), symbolic (notation). **(high)**
- **Singapore CPA** is "directly derived from Bruner's enactive-iconic-symbolic modes, with a one-to-one correspondence … the relabeling was primarily language simplification rather than theory revision." Advocated by Singapore's MOE since the early 1980s (Primary Mathematics Project, materials from 1981). Source: Leong, Ho & Cheng (2015), *The Mathematics Educator* 16(1), NIE Singapore. **(high)**

**A disagreement worth knowing.** The *theoretical* reading (Bruner via Leong et al.) explicitly **debunks the myth that the modes are "distinct and separated chronologically"** — symbolic-mode elements develop *alongside* enactive/iconic ones. But much *special-education CRA pedagogy* prescribes a **strict sequence** ("start at concrete before moving to representational and, finally, abstract" — the ordering treated as a correctness criterion). Both views passed verification because **they're answering different questions**: the strict sequence is an *instructional scaffold* for struggling learners; the overlap claim is about how representation *actually develops*. Design implication: treat strict C→R→A as the *default scaffold*, not a law — and allow simultaneous/linked presentation (see CRA-I, §3). **(this tension is real, not an error)**

---

## 2. Transition criteria — when to move C→R and R→A

Honest finding: **the literature is thinner and vaguer on precise, quantitative readiness thresholds than you'd hope.** What the evidence and practice converge on:

- **Mastery-based gating, not time-based.** Practitioner CRA models advance a learner only after demonstrated proficiency at the current level (commonly framed as a high-accuracy criterion across multiple sessions/problems before fading the support), rather than after a fixed number of lessons. Exact cut scores vary by study and aren't standardized. **(medium — practice consensus, weak standardization)**
- **Readiness is per-skill and per-operation, and can be interleaved.** A study taught **multiplication and division together in an _alternating_ CRA sequence**, leveraging their inverse relationship rather than running each as a separate strictly-linear track — direct support for per-operation / interleaved CRA design. **(medium-high)**
- **CRA is not strictly a linear three-stage staircase.** Verified against the primary CPA survey: the modes overlap. So "concrete for division, abstract for addition" is expected, not a bug. **(high)** This directly validates our existing per-operation CRA stage tracking.
- **Moving too fast vs. too slow.** The strongest *indirect* evidence is the transfer literature (§3): abstract-only instruction (moving "too fast," skipping concrete) underperforms on transfer; but concrete-*only* (never fading, "too slow") also underperforms. The sweet spot is *fade, but only after the concrete grounds the concept*. **(high for the direction of the effect; specific timing is under-specified in the literature)**

**Practical takeaway for our adaptive system:** we already measure accuracy + response time silently. Use **rolling accuracy at the current representation as the readiness signal** (e.g., a sustained high-accuracy streak at Concrete unlocks Representational for *that operation*), and make fades reversible (drop back a level on a frustration signal) — the research supports mastery-gated, per-operation, reversible movement far more than any fixed schedule.

---

## 3. Transition techniques (how to bridge stages)

**Concreteness fading — the headline technique.** Defined (Fyfe, McNeil, Son & Goldstone, 2014, *Educational Psychology Review*, systematic review) as *beginning with concrete materials and then explicitly and gradually fading them toward abstract representations*, rather than choosing concrete-only or abstract-only. A core, repeated benefit is **generalization/transfer to novel problems**. A controlled study (McNeil & Fyfe, 2015, *Learning & Instruction*; 2nd–3rd graders, math equivalence) found **fading concrete→abstract beat both concrete-only and abstract-only on transfer.** **The direction matters** — progressing concrete→abstract is what produces the benefit; the reverse is worse. **(high)**

**This is the mechanic we want for stage transitions:** don't just flip a kid from "blocks" to "numerals." Fade the support *within* a problem type — full manipulative → drawing → faint drawing → symbols — explicitly linking each step to the last.

**CRA-Integrated (CRA-I) vs. sequential.** CRA-I presents **all three representations together from the first lesson** (manipulative + drawing + numeral side by side), then fades the concrete/representational supports. This is concreteness fading operationalized as a layout. **Tension flagged:** one extracted study reported the **non-integrated (sequential) model significantly _outperformed_ the integrated** one; the broader fading literature favors integrated-then-fade. Evidence is **mixed**; both are defensible. Design implication: prototype the **integrated panel with fade** (it composes naturally with a game UI) but keep the sequential path as a fallback. **(medium — genuinely contested)**

**Dual representation / the "symbol grounding" problem.** A manipulative is "simultaneously an object in its own right and a representation of something else" (the dual-representation hypothesis; Uttal/DeLoache lineage). The richer/more *toy-like* the object, the more a young child treats it as a toy and the *less* it functions as a symbol. **Reducing a manipulative's perceptual richness (drawing attention to it as a symbol, not a plaything) improves its representational function.** **(high)** Design implication: our manipulatives should be **clean and schematic**, not lavishly decorated — counters as plain discs, not detailed characters, at the moment they're doing math work.

**Linking, gesture, narration.** Simultaneously linking representations (saying/showing how the blocks *are* the number while pointing) is part of why CRA works; the special-ed protocols lean on explicit teacher verbalization and gesture. In a game, the buddy's narration + an animated "the 3 blocks become the numeral 3" linking beat is the analog. **(medium — supported but entangled with the fading evidence)**

---

## 4. Catalog of representations & manipulatives

Mapped to stage / skills / age / the misconception each cures or causes. **The number-path vs number-line distinction is the most actionable item here.**

### Number PATH vs number LINE — read this before building anything for the 4yo

| | **Number path** | **Number line** |
|---|---|---|
| What | Discrete **rectangles/boxes**, one per number, counted one-by-one | Numbers as **points on a length**; distance between them is the measure |
| Model type | **Counting** model (cardinality, one-to-one) | **Measurement** model (magnitude as length) |
| Best for | **Youngest / earliest counting (≈ ages 3–6)** | Later (≈ 6+), once counting is solid |
| Cures | Reinforces one-to-one correspondence; each box is a thing you can count | Magnitude, relative size, jumps of +n/−n, eventually fractions/negatives |
| **Causes (if used too early)** | — | Young kids **count the tick marks instead of the intervals**, and **start counting at 1 instead of 0** — classic, well-documented errors |

**Verified:** "A number path is a counting model … each rectangle can be counted," whereas "a number line represents numbers as spaces/lengths"; and "number lines cause specific counting errors" in young children (counting marks vs. intervals; off-by-one starts). **(high, practitioner-strong: k5mathspot, Didax, Jillian Starr — multiple independent practitioner sources agreeing; this is consensus among early-math educators even if not a single RCT.)**

> **For your 4-year-old specifically:** build a **number PATH first** — countable stepping-stones the character hops along, one stone per number. Introduce the true **number line (length/jumps)** only once one-to-one counting is automatic. A "hop along the stones" mechanic is concrete (enactive), maps perfectly to a top-down RPG, and sidesteps the number-line error trap.

### The rest of the catalog

| Representation | Stage | Skills / operations | Age band | Misconception it cures / causes |
|---|---|---|---|---|
| **Counters / set models** (discs, bears) | Concrete | Counting, cardinality, +/−, sharing→÷ | 3–7 | Cures one-to-one; *causes* toy-distraction if too richly decorated (dual-representation) |
| **Number path** (see above) | Concrete→Repr. | Counting, count-on/back, +/− within 10–20 | 3–6 | Cures one-to-one; avoids number-line errors |
| **Ten-frame** (2×5 grid) | Concrete→Repr. | **Subitizing**, five/ten anchors, number bonds to 10, +/− | 4–7 | Cures "count-all" (builds instant recognition & ten-structure); structures place value later |
| **Rekenrek** (2 rows × 10 beads, 5 red/5 white) | Concrete | Number sense via **five/ten structure**, not rote counting; +/−, doubles, bonds | 5–8 | Cures rote one-by-one counting; builds "5 and 3 is 8" without counting |
| **Subitizing cards/dots** | Repr. | Instant quantity recognition (foundation of all number sense) | 3–6 | Cures count-all; speeds fact fluency |
| **Part-part-whole / number bonds** | Repr. | Decomposition, +/− as inverse, fact families | 5–9 | Cures "addition and subtraction are unrelated"; grounds missing-addend |
| **Base-ten blocks** | Concrete→Repr. | Place value, multi-digit +/− with regrouping | 6–10 | Cures place-value/regrouping errors (CRA's strongest documented win) |
| **Cuisenaire rods** | Concrete | Magnitude, part-whole, +/−, early ×/fractions | 5–10 | Cures "number = count of objects only" (length model); bridges to number line |
| **Bar / tape model** (Singapore) | Repr. | Word problems, comparison, multi-step, ratio | 7–10+ | Cures "which operation?" by making structure visible |
| **Arrays / area model** | Concrete→Repr. | **Multiplication & division**, distributive property | 7–10 | Cures "× is just repeated addition"; grounds area & factoring |

(Stage assignments follow the standard CRA reading; age bands are typical, not rigid — and per our own model, a kid can sit at different stages per operation.)

---

## 5. Digital / virtual manipulatives & game-based CRA

- **Manipulatives help, modestly, and guidance matters.** Carbonneau, Marley & Selig (2013, *J. Educational Psychology*) — meta-analysis of **55 studies, 7,237 students, K–college** — found teaching with concrete manipulatives produced **statistically significant small-to-moderate effects** vs. abstract symbols alone. Moderators matter: **high perceptual richness can _reduce_ the benefit**, and **instructional guidance increases it.** **(high)** → Clean visuals + scaffolded guidance, not flashy free-play.
- **Virtual manipulatives for kids with math LD: "promise," not proof.** A systematic review of **38 studies** found manipulative interventions (concrete *or* virtual) effective across a range of objectives for children with math learning disabilities, but only a **tentative "promising" conclusion** due to pervasive methodological weakness/heterogeneity. **(medium — honest uncertainty; virtual ≈ concrete in the studies, neither clearly superior)**
- **Adaptive number-sense games work for the at-risk end.** *The Number Race* (Wilson & Dehaene, 2006) — an adaptive game training number sense by **emphasizing the number↔space link** and progressively linking Arabic/verbal/quantity codes — is a direct precedent for a magnitude/number-line mechanic aimed at dyscalculia. **(high that it exists & its design; medium on magnitude of learning gains)**
- **"Productive failure."** Our search angle targeted this but **surfaced no strongly-verified claim** in this run — treat as *under-evidenced here*; don't lean on it as justification. **(evidence thin — flagged)**

Design implications: immediate, in-world feedback (the bridge wobbles, bags fill) is consistent with the manipulatives evidence; keep representations **schematic**; provide **explicit guidance/linking**, not unguided sandbox.

---

## 6. Atypical / neurodivergent learners

- **CRA's home turf is special education.** Its evidence base "is heavily rooted in special education," and it's a **formally designated evidence-based practice** for students with LD on **computation** (addition, subtraction, multiplication — especially regrouping). Source: **Bouck, Satsangi & Park (2018)**, *Remedial and Special Education* 39(4) — a best-evidence synthesis applying Cook et al. quality standards. **(high)**
- **Large meta-analytic effect (with a caveat).** **Ebner, MacDonald, Grekov & Aspiranti (2025)**, *Learning Disabilities Research & Practice* — meta-analysis of **30 single-case studies, 116 participants** — reported an **overall Tau-BC = 0.9965, 95% CI [0.9947, 0.9983], p < .0001.** That number is *huge*; it reflects single-case-design methodology (which tends to produce very high effect statistics) and small N, so read it as "**robustly positive in special-ed settings**," not as a literal real-world effect size. **(high that the finding is real; flagged that SCD effect magnitudes don't translate 1:1 to classroom/group effects.)**
- **Adaptations that recur for LD/dyscalculia/ADHD/ASD:** more time at the concrete stage; **explicit, deliberate linking** between representations; **multisensory** engagement; reduced extraneous load (clean materials, one concept at a time). These align with our existing dials (`scaffolding`, `pace`, CRA stage) and the *concreteness-fading + low-perceptual-richness* findings above. **(medium — consistent across special-ed sources)**

---

## 7. Caveats & contested points (don't oversell these)

1. **Effect-size inflation from single-case designs.** The eye-popping Tau-BC ≈ 0.997 is SCD-typical; group-design effects (Carbonneau) are small-to-moderate. Cite CRA as "evidence-based and reliably positive," not "0.99 effect."
2. **Strict-sequence vs. overlapping-modes** — a real framework disagreement (§1). Use strict C→R→A as the default scaffold but allow integrated/linked presentation.
3. **Integrated (CRA-I) vs. sequential delivery** — mixed evidence (§3). Prototype integrated-then-fade; keep sequential as fallback; don't claim one is settled.
4. **Precise readiness thresholds are under-specified** in the literature (§2). Our adaptive accuracy signal is a reasonable operationalization, but it's *our* design choice, not a research-validated cut score.
5. **Number-path-before-number-line is practitioner consensus**, well-reasoned and widely agreed, but not anchored to a single RCT in this run. High practical confidence, lower "RCT" confidence.
6. **Virtual-manipulative & game-based gains for MLD are "promising," not proven** (§5). And **productive failure was not substantiated** in this pass.

---

## 8. Prioritized "goodies" to build (mapped to our engine)

Ordered by leverage × fit. Our domain already has `logic/number_line` and `logic/base_ten` reducers (per ADR-003) — some of this is *wiring + reframing*, not greenfield.

1. **Number PATH for the 4-year-old (build first).** Countable stepping-stones the character hops, one stone per number; count-on/count-back as hops. Concrete/enactive, RPG-native, dodges the number-line error trap. *Reframe/extend the existing `number_line` module so its earliest mode is a discrete path, with the continuous length-based line as a later mode.* **Highest priority for the youngest.**
2. **Concreteness-fading as the CRA *transition mechanic* itself.** Don't hard-flip stages. Within an operation, fade: real manipulative → drawn manipulative → faint/ghosted drawing → numerals, with an explicit "the blocks *become* the number" linking beat (buddy narrates). This is the research's single best-supported transition technique and becomes our C→R→A engine.
3. **Readiness = rolling accuracy at the current representation, per operation, reversible.** Sustained high accuracy at Concrete unlocks Representational *for that operation*; a frustration signal drops it back. Matches mastery-gated, per-operation, non-linear CRA — and our invisible-assessment invariant.
4. **Ten-frame + rekenrek + subitizing for early number sense.** Cheap, schematic, high-yield for ages 4–7 (subitizing, five/ten anchors, bonds-to-10). Ten-frame first (simplest), rekenrek as the "5-and-some" upgrade.
5. **CRA-Integrated panel (prototype, then A/B).** Show manipulative + drawing + numeral together, then fade — but keep the sequential path given the mixed evidence.
6. **Arrays/area for multiplication & base-ten for regrouping** (older end, 7–10) — these are CRA's most *documented* wins; lean on them when those operations come online.
7. **Keep manipulatives schematic, guided, and immediately responsive** — the cross-cutting design rule from the manipulatives meta-analysis + dual-representation findings (low perceptual richness, explicit guidance, in-world feedback). Bias against decorating a counter into a distracting toy at the moment it's doing math.

---

## 9. Sources (verified pass)

Primary / peer-reviewed:
- Leong, Ho & Cheng (2015), *Concrete-Pictorial-Abstract: Surveying its origins and charting its future*, The Mathematics Educator 16(1), NIE Singapore — https://math.nie.edu.sg/ame/matheduc/tme/tmeV16_1/TME16_1.pdf
- Bouck, Satsangi & Park (2018), *The CRA Approach for Students With Learning Disabilities: An EBP Synthesis*, Remedial and Special Education 39(4) — https://journals.sagepub.com/doi/10.1177/0741932517721712
- Ebner, MacDonald, Grekov & Aspiranti (2025), *A Meta-Analytic Review of the CRA Math Approach*, Learning Disabilities Research & Practice — https://journals.sagepub.com/doi/10.1177/09388982241292299
- Fyfe, McNeil, Son & Goldstone (2014), *Concreteness Fading … A Systematic Review*, Educational Psychology Review — https://link.springer.com/article/10.1007/s10648-014-9249-3
- McNeil & Fyfe (2015), *Concreteness fading promotes transfer*, Learning and Instruction 35 — https://www.sciencedirect.com/science/article/abs/pii/S0959475214000942
- Carbonneau, Marley & Selig (2013), *A Meta-Analysis of the Efficacy of Teaching Mathematics with Concrete Manipulatives*, J. Educational Psychology — https://eric.ed.gov/?id=EJ1007941
- Uttal et al. (2009), dual representation / manipulatives — https://onlinelibrary.wiley.com/doi/10.1111/j.1750-8606.2009.00097.x
- Wilson & Dehaene et al. (2006), *The Number Race* (adaptive dyscalculia game) — https://www.ncbi.nlm.nih.gov/pmc/articles/PMC1523349/
- Multiplication/division alternating-CRA study — https://www.researchgate.net/publication/327078001
- Systematic review, manipulatives for math LD (38 studies) — https://onlinelibrary.wiley.com/doi/10.1155/2019/2142948
- CPA origins (NIE repository) — https://repository.nie.edu.sg/entities/publication/a179146f-ac98-4c32-b576-f5df85253240

Secondary / practitioner (number path vs line; manipulatives how-to):
- PaTTAN, *CRA Methods* — https://www.pattan.net/getmedia/9059e5f0-7edc-4391-8c8e-ebaf8c3c95d6/CRA_Methods0117
- USF MathVIDS, *CRA* — https://fcit.usf.edu/mathvids/strategies/cra.html
- k5mathspot, *Number Paths and Number Lines* — https://k5mathspot.com/models-of-the-count-sequence-number-paths-and-number-lines-in-elementary-math/
- Didax, *Number Paths: a better tool for early math than number lines* — https://www.didax.com/blog/number-paths-a-better-tool-for-early-math-than-number-lines/
- Didax, *How the Rekenrek Supports Number Sense* — https://www.didax.com/blog/how-the-rekenrek-supports-number-sense/
- Jillian Starr, *Number Paths* — https://jillianstarrteaching.com/number-paths/
- Third Space Learning, *Ten Frame* — https://thirdspacelearning.com/us/blog/ten-frame/

---

## 10. Relationship to other specs

- **Adaptive Learning Spec** — owns the dials (`pace`, `scaffolding`, per-operation CRA stage, frustration) this brief recommends wiring readiness/fading into.
- **RPG Quest Spec** — the manipulatives here are the *math mechanics* quest puzzles should embed (number-path hops, ten-frame fills); concreteness fading is how a quest line can deepen a single operation over time.
