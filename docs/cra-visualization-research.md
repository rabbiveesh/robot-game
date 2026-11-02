# Let the child do the math, not watch it

The main problem with today's "Show me" is not how it looks. It is a static picture that does the work for the child, and that makes it a bottom-out hint at the Representational stage, even though it is filed under Concrete. The research here points to four fixes, which together would make it help the child learn rather than let them skip the thinking.

1. **Concrete should mean the child acts on the quantity.** Each drag, tap or group move should equal one mathematical step. An on-screen object is not enough to count as Concrete.
2. **Representational pictures should be structured, not loose dots.** Groups of 5 and 10, ten-frames, two hands, and arrays for multiplication.
3. **The picture should match the story.** Taking away, comparing, filling up to a target, dealing out and making groups each need a different picture.
4. **Speech replaces written labels.** Sparky speaks every word. Only numerals and operation signs appear on screen, each tied to its quantity by colour and by a pointing beam.

The best direct evidence is modest but consistent:

- A small kindergarten RCT: structured linear materials nearly tripled count-on, while random arrays did nothing ([Schiffman & Laski 2018](https://pmc.ncbi.nlm.nih.gov/articles/PMC6312299/)).
- Three replicated RCTs, about 450 children: kindergartners taught to show one addend on each hand went from 37% to 77–90% accuracy ([Poletti et al. 2025](https://pmc.ncbi.nlm.nih.gov/articles/PMC11693818/)).
- Congruent drag beat tapping for 5–6-year-olds ([Wu et al. 2024](https://pubmed.ncbi.nlm.nih.gov/38889478/)).
- Countable decorative units stopped 6–8-year-olds from learning the structure ([Kaminski & Sloutsky 2013](https://eric.ed.gov/?id=EJ1007940)).

Two findings argue against quick fixes. Animation on its own shows no measurable benefit for mathematics (g = 0.07, n.s.) ([Berney & Bétrancourt 2016](https://access.archive-ouverte.unige.ch/access/metadata/75bf6876-cb91-4e2c-8ac6-9d0767449449/download)). A virtual manipulative used for a single day has essentially zero effect (0.003) ([Moyer-Packenham & Westenskow 2013](https://blogs.sd38.bc.ca/sd38mathandscience/wp-content/uploads/sites/14/2020/06/VMMetaAnalysisPaper.MoyerPackenhamWestenskow.pdf)). So the fix is a small number of familiar tools the child uses again and again, not moving pictures. The biggest gap is that no study directly tests "the child builds it" against "the child watches it" against "a static picture" on arithmetic for ages 4–7. The game's own event log is the best place to answer that.

**How to read the evidence tags.** Each claim is tagged by the kind of study behind it:

| Tag | Kind of study |
|---|---|
| **[MA]** | Meta-analysis |
| **[RCT]** | Randomized or controlled experiment |
| **[QE]** | Quasi-experimental, longitudinal or correlational |
| **[QUAL]** | Design research, case study or qualitative |
| **[PRAC]** | Practitioner consensus |

"Adult-derived" means the principle was established mostly with older learners.

## What today's "Show me" gets wrong

Here is what the current renderer in `robot-buddy-game/src/ui/visuals.rs` actually draws:

- **Addition:** every dot of both addends, in one unstructured run that wraps at 10.
- **Subtraction:** only the remainder left uncrossed, then asks the child to "count the blue ones!"
- **Multiplication:** all a×b dots, in groups whose colours alternate blue and yellow by position.
- **Division:** the answer already dealt out. It draws *b* boxes, each holding *a/b* dots.

In every case the answer can be counted straight off the picture. The help literature calls this a **bottom-out hint**, the last hint level, which reveals the answer. It has two well-documented problems:

- **Children game it.** "Gaming the system" is defined as exploiting the system rather than learning. It includes drilling through hints to reach the answer ([LearnLab](https://learnlab.org/wiki/index.php?title=Gaming_the_system)) **[QE]**.
- **Gaming corrupts the learner model.** Knowledge-tracing models assume the answers reflect what the student knows; gamed answers don't ([arXiv 2512.18659](https://arxiv.org/html/2512.18659v3)) **[QE]**.

A 2026 learning-analytics paper found that unproductive hint use predicts worse learning across several tutoring-system datasets. It concluded that the standard multi-level hint button, which ends in the answer, needs "critical re-examination" ([LAK26](https://dl.acm.org/doi/10.1145/3785022.3785040)) **[QE, abstract only]**. In a fractions puzzle game, adding hints actually *lowered* performance ([O'Rourke et al. 2014](https://dl.acm.org/doi/10.1145/2556325.2566248)) **[RCT; I could not read the full text]**.

Kids who grind for Dum Dums on purpose make the risk concrete here: a motivated grinder will learn that tapping "Show me" and counting is the cheapest route to a reward.

**Coaching children to use help better doesn't fix this.** Aleven et al.'s "Help Tutor" gave students feedback on how they sought help. It improved how they used help, but it did not improve what they learned ([Aleven et al. 2016](https://link.springer.com/article/10.1007/s40593-015-0089-1)) **[RCT, older students]**. A "you're using help too much" message would also break the no-labels invariant. The fix has to be in the design of the help itself.

**The same picture also fails as a Concrete stage, on the research's own definition.** The prior brief (`docs/cra-progression-research.md` §1) defines Concrete as the enactive mode: "the operation is an action you *do*, not a symbol you read." The new notes back that definition up with evidence:

- **Martin & Schwartz:** 9–10-year-olds solved fraction problems better when they physically moved pie pieces than when they only looked at them ([Martin & Schwartz 2005](https://pubmed.ncbi.nlm.nih.gov/21702786/)) **[RCT]**.
- **Manches et al.:** with 4–8-year-olds, *how* the interface lets the child move blocks changed the strategies they found. One block at a time produced counting and compensation strategies. Moving blocks as a group allowed part-whole strategies ([Manches et al. 2010](https://www.research.ed.ac.uk/en/publications/the-role-of-physical-representations-in-solving-number-problems-a/)) **[RCT, three studies]**.
- **Sarama & Clements** argue that "concrete" means meaningful action on objects, not physical objects. My notes flagged this reading as recalled from memory, not re-read this session ([Sarama & Clements 2009](https://onlinelibrary.wiley.com/doi/abs/10.1111/j.1750-8606.2009.00095.x)) **[theory]**.

A picture the child cannot act on is iconic, which means Representational.

**This contradicts `docs/visualization-methods-spec.md`.** That spec's `METHOD_CRA` table labels dots, ten-frames, base-10 blocks and arrays as `concrete`, whether they are drawn or manipulated. The research implies CRA stage is a property of **what the child does with the model**, not of which model is used:

- A draggable ten-frame is Concrete.
- The same ten-frame, drawn and static, is Representational.
- A bare numeral is Abstract.

The spec's central idea still holds: CRA stage and visualization method are two separate axes. It just needs a third axis, **who acts**.

**Some of the fix already exists.** `logic::manipulate_concrete` already models tap-to-count, drag-to-group, build-a-tower and take-away as pure reducers. `logic::base_ten` models trading ten ones for a ten, and breaking a ten back into ones. ADR-003 lists all of these as "debroccoli" mechanics. Concrete-stage "Show me" is mostly a matter of wiring these up, not building from scratch.

## Structure beats counting at every stage

Once a picture is shown, how the dots are arranged matters more than how many there are.

**Structured quantities predict later addition; unstructured ones don't.** Kreilinger et al. followed 116 children from preschool into grade 1. Finger and dice patterns were read faster and more accurately than random sets. Only the ability to read *structured* sets predicted later addition ([Kreilinger et al. 2021](https://pubmed.ncbi.nlm.nih.gov/33021730/)) **[QE, longitudinal]**. The benefit of grouping ("groupitizing") gets stronger with each grade ([Starkey & McCandliss 2014](https://www.sciencedirect.com/science/article/pii/S0022096514000630)) **[experimental, abstract only]**. Clements separates two skills ([Clements 1999](https://eric.ed.gov/?id=EJ588575)) **[theory]**:

- *Perceptual* subitizing: seeing how many at a glance, up to about 4.
- *Conceptual* subitizing: seeing 8 as "4 and 4". This has to be taught, and part-whole reasoning rests on it.

A run of 7 evenly spaced dots can only be counted one by one, so it builds neither skill.

**The key RCT shows why structure matters.** Schiffman & Laski randomly assigned 29 low-income kindergartners to practise sums to 10 with one of two materials ([PMC6312299](https://pmc.ncbi.nlm.nih.gov/articles/PMC6312299/)) **[RCT, small, four sessions]**:

| Outcome | Linear, unit-segmented sticks | Random arrays of stars |
|---|---|---|
| Count-on | 16% → 44% | 11% → 13% |
| Advanced strategies | 19% → 52% (g = 0.81) | 14% → 19% |
| Absolute error | 2.53 → 1.51 | 3.04 → 3.26 |

**Better strategies mediated the accuracy gains.** In other words, the materials worked by changing *how* children solved the problems. This is the most direct evidence that the current wrap-at-10 dot row is the wrong default.

Other evidence points the same way:

- **PASMAP**, a kindergarten program that teaches pattern and structure, raised structural awareness, and the gains held into grade 1 ([Mulligan & Mitchelmore](https://files.eric.ed.gov/fulltext/ED521029.pdf)) **[QE]**.
- A **1,017-student grade-1 study** found that teaching non-counting strategies alongside conceptual subitizing beat the control group. It also noted that many children "continue to rely on counting strategies in later years" ([JMD 2024](https://link.springer.com/article/10.1007/s13138-024-00236-6)) **[QE, large N, abstract only]**.

The current wrap-at-10 row breaks the line at 10 but has no break at 5. It also shows no empty slots, so the child can't see how many more make 10.

**Fingers are the surprising winner for the youngest.** On-screen hands carry this structure naturally, because a full hand *is* five.

- **Finger *gnosis* training does not transfer.** This is training to sense and tell apart individual fingers. A 102-child RCT with an active control found no advantage ([Schild et al. 2020](https://www.frontiersin.org/journals/psychology/articles/10.3389/fpsyg.2020.00529/full)) **[RCT, null]**.
- **Finger *counting* does.** In three randomized experiments (about 450 kindergartners), six 10-minute sessions taught "first addend on one hand, second on the other, count all". Children who had not been using their fingers went from 37% to 77% accuracy (controls: 40% to 48%). Accuracy reached about 90% against an active control, and the gains lasted six weeks ([Poletti et al. 2025](https://pmc.ncbi.nlm.nih.gov/articles/PMC11693818/)) **[RCT, replicated ×3]**.
- **Early finger users leave fingers earlier.** Children who start using fingers early move sooner to accurate mental strategies. The best performers at 6.5 were *former* finger users ([Thevenot group 2025](https://www.apa.org/pubs/journals/releases/dev-dev0002099.pdf)) **[QE, longitudinal]**.

So fingers are a scaffold to fade out, not a trap. The caveat: every one of these studies used the children's real fingers. Whether *pictured* hands (Sparky's robot hands) work the same way is untested.

**Counting itself is not the enemy.** The enemy is a visual that allows *only* counting all the dots from 1. Siegler's "overlapping waves" model says counting all, counting on, breaking numbers apart and recall coexist and shift gradually ([Siegler](https://earlymath.erikson.edu/wp-content/uploads/2016/10/Siegler-draft-10_25_16.pdf)) **[theory + microgenetic]**. The design job is to make the next strategy available, for example by hiding the first addend so the child has to count on from it. Banning counting is not the goal.

**Dots should be uniform in size and spacing.** Numerosity research controls dot size and density because children use them as cues for "more" ([Gebuis & Reynvoet](https://www.researchgate.net/publication/51069992_Generating_non-symbolic_number_stimuli)) **[methodological]**. Frame slots enforce this for free.

**How this relates to the prior brief.** It already recommended ten-frames and rekenreks as "cheap, schematic, high-yield" (§8.4) and put them at Concrete→Representational. The new evidence confirms the recommendation and upgrades its basis from practitioner consensus to an RCT plus longitudinal data. It adds two things the brief lacked: the finger evidence, and the specific failure mechanism of unstructured arrays (strategies stay flat).

## Picture the story, not the operator

Children solve problems by **directly modelling the action or structure in the story**. The kind of story, not the operator symbol, sets both the difficulty and the strategy the child uses. This is the finding behind CGI (Cognitively Guided Instruction), a first-grade teacher-training program. In an RCT with 40 first-grade teachers, those trained in CGI's problem types and children's strategies had students who outperformed controls in both problem solving and number facts ([Carpenter et al. 1989](https://journals.sagepub.com/doi/abs/10.3102/00028312026004499)) **[RCT, teacher-level]**.

CGI's problem types, roughly from easiest to hardest ([DREME](https://prek-math-te.stanford.edu/operations/engaging-participants-around-problem-types)) **[PRAC]**:

- **Join** and **Separate**, when the result is unknown, are easiest, because the action can be acted out.
- **Part-whole** has no action at all.
- **Join-change-unknown** (the missing addend) is harder.
- **Compare** is hardest, because it is static and the language is tricky.

**Subtraction currently has one picture for several different stories.** "Cross out, then count the blue ones" models only take-away, and it invites the child to count everything that remains. Selter et al. distinguish two models of subtraction ([Selter et al. 2012](https://wwwold.mathematik.tu-dortmund.de/~prediger/veroeff/12-Selter-et-al_ESM_Subtraction-Web_version.pdf)) **[theory + review]**:

- **Taking away:** efficient when the number removed is small (12 − 3).
- **Finding the difference, by counting up:** efficient when the numbers are close (9 − 7).

Children hardly ever use the second model on their own.

**Compare problems need a one-to-one picture.** Hudson's classic study is the key evidence. Children failed "How many more birds than worms?" but did much better on "How many birds won't get a worm?", which frames the question as one-to-one matching ([via Shanahan](https://www.shanahanonliteracy.com/blog/can-reading-instruction-improve-math-learning-in-the-primary-grades)) **[experimental, secondary source only]**.

**Division has two models, and children find sharing more intuitive.**

- **Partitive division (sharing):** "12 pearls among 3 buddies". This is the intuitive "primitive" model, and that finding has been replicated ([Maffia et al. replication](https://ccpp.leeds.ac.uk/wp-content/uploads/sites/21/2021/05/012-MaffiaEtAl_NumSense_REV.pdf)) **[replication]**.
- **Quotative division (making groups):** "each chest holds 3, how many chests?" This comes later.
- **Children add models rather than replace them**, dealing out by ones or building up groups repeatedly ([Mulligan & Mitchelmore 1997](https://www2.merga.net.au/documents/Roche_RP09.pdf)) **[QE, longitudinal]**.

The current renderer labels division "split into b groups" (sharing) but draws the answer already dealt into boxes, so the child never does the sharing. (The trade desk's "how many groups of `rate` are in this pile?" in `game.rs` is a separate grouping puzzle, and it is the right model for that story.)

**Arrays suit multiplication beyond "groups of".** Their row-by-column structure shows that multiplication is commutative and distributive, and supports the move from "groups of" to multiplicative thinking ([Barmby et al. 2009](https://durham-repository.worktribe.com/output/1530338/)) **[QUAL]**.

**Why this matters for the game.** The Broccoli Test and CGI point in the same direction. In this RPG, the story actions (Sparky giving away bolts, sharing pearls among buddies, matching one buddy to one item) *are* the problem types. So the visual can simply be the story action, drawn schematically. The honest caveat: no controlled study directly measures harm from a *mismatched* visual, such as a take-away picture on a compare problem. That claim rests on CGI's direct-modelling findings, plus Hudson's result on how problems are worded.

**Base-10 blocks come with a warning.** The prior brief calls base-10 blocks CRA's "strongest documented win" for regrouping. That still holds. But "quick tens" (a stick drawn for ten) should come only after the child has made "ten ones become a ten" with their own hands (the next section covers why, via Novack). That is exactly the enactive trade that `logic::base_ten` already models.

## Speak it, point at it, keep the charm outside

**Written labels teach nothing to a 4-year-old.** "3 groups of 4" and "count the blue ones!" carry no meaning for a child who can't read. They are just more visual clutter.

**Clutter has a cost at these ages.**

- **Decorative units:** 6–8-year-olds learning to read bar graphs learned substantially less when the bars were built from stacks of countable objects. The children counted the objects instead of reading the scale, and the interference was worse the younger the child ([Kaminski & Sloutsky 2013](https://eric.ed.gov/?id=EJ1007940)) **[RCT]**.
- **Decorated rooms:** kindergartners in a heavily decorated classroom were more off-task and learned less ([Fisher et al. 2014](https://journals.sagepub.com/doi/abs/10.1177/0956797614533801)) **[RCT, within-subject]**. There is an unresolved critique that part of this effect is novelty ([PMC commentary](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4274873/)).
- **Seductive details:** age did not reduce how much seductive details hurt learning ([2025 meta-analysis](https://www.sciencedirect.com/science/article/pii/S1747938X25000673)) **[MA, secondary report]**.

**Charm belongs around the diagram, not inside it.** The same 2025 source found that "emotional design" (appealing visuals) had much *larger* benefits for elementary students than for older learners. The way to reconcile the two findings is to put the charm in Sparky and the scene, and keep the math diagram schematic. This confirms and sharpens the prior brief's point about dual representation and perceptual richness. Its advice to use "counters as plain discs" now has a specific mechanism: decorated units get counted *instead of* the structure.

**The multi-colour group problem.** Alternating blue and yellow by group position in the × visual is mild decoration: the colours encode which group comes first, not a mathematical role. Spacing or enclosures are enough to separate groups. The colours can then be saved for roles, such as number of groups versus size of each group.

**Narration is the only verbal channel a pre-reader has, and narrated animation works for young children.** A meta-analysis of 43 studies with 2,147 young children found ([Takacs et al. 2015](https://eric.ed.gov/?id=EJ1081714)) **[MA, preschool/early primary]**:

- Technology-enhanced stories gave small gains in comprehension (g = 0.17) and vocabulary (g = 0.20).
- Multimedia features (animation, music, sound) helped.
- *Interactive* extras (hotspots, games) distracted.
- Both effects were stronger for children from less stimulating homes.

The adult "modality" finding (narration beats on-screen text) is large in Mayer's own tests. Broader meta-analyses find it smaller, about d = 0.24 ([Cambridge Handbook](https://www.cambridge.org/core/books/abs/multimedia-learning/modality-principle/E5CD6E01CEA0B568CE260F66A3CD0D1F)) **[MA, adult-derived]**. For a child who can't read, the question is simply settled: speech has to carry the language. Pedagogical agents like Sparky show small learning benefits, larger for K-12 than for adults ([Schroeder et al. 2013](https://journals.sagepub.com/doi/10.2190/EC.49.1.a)) **[MA]**. The risk is an agent animating constantly next to the math. Sparky's motion during an explanation should be deictic: pointing at the math.

**The numeral is the one written symbol worth showing.** In a longitudinal study of 206 children aged 3–5, numeral knowledge fully mediated the step from informal to formal math a year later. That held only when numeral knowledge included both recognising numerals *and* mapping them to quantities ([Purpura, Baroody & Lonigan 2013](https://experts.illinois.edu/en/publications/the-transition-from-informal-to-formal-mathematical-knowledge-med)) **[QE, longitudinal]**. So every quantity should appear with three things at the same moment:

- its numeral;
- Sparky saying the number word;
- a visible link between the numeral and the set.

**The link itself has to be visible.**

- **Ainsworth:** multiple representations help only when learners are given support to translate between them ([Ainsworth 2006](https://eric.ed.gov/?id=EJ737858)) **[theory]**.
- **Richland:** teachers in higher-achieving regions used *linking gestures* that point back and forth between representations. When source and target were visibly aligned, students transferred more ([Richland & Begolli 2016](https://journals.sagepub.com/doi/abs/10.1177/2372732216629795)) **[QE video analysis + RCT, older students]**.
- **Richter et al.:** signals such as colour coding that tie words to parts of a picture have a small-to-medium effect, mostly for learners with low prior knowledge ([Richter et al. 2016](https://www.sciencedirect.com/science/article/abs/pii/S1747938X15000664)) **[MA, mostly older]**. Signals become redundant as expertise grows ([Instructional Science 2019](https://link.springer.com/article/10.1007/s11251-019-09492-3)) **[RCT]**.

Together these argue for heavy linking the first few times an operation's picture appears, then less, keyed to the per-operation CRA stage.

**This tips the balance in an open question from the prior brief.** Its §3 left CRA-Integrated (all three representations shown together) versus sequential presentation as "genuinely contested". The linking evidence doesn't settle it. It does say integration only works *with* visible links. Showing things side by side without links is exactly the failure Ainsworth describes.

## Interaction helps only when every touch is math

Interactivity can go wrong, and the ways it goes wrong are documented as clearly as the benefits.

**Plain tapping trains more tapping.** In the "All Tapped Out" study (170 children aged 2–4, learning words, not math), children in the tap and drag conditions made about 38–44 taps while the narration played, against about 10 in the watch-only condition ([Russo-Johnson et al. 2017](https://pmc.ncbi.nlm.nih.gov/articles/PMC5388766/)) **[RCT]**. The same study found:

- low-SES children learned more by dragging than by tapping;
- boys learned better by watching.

**Congruent gestures work better.** When 5–6-year-olds estimated on a number line, *dragging*, which matches a continuous change in magnitude, beat discrete tapping on hard items. It was more accurate and faster ([Wu et al. 2024](https://pubmed.ncbi.nlm.nih.gov/38889478/)) **[RCT, N = 70]**. Design research points the same way:

- TouchCounts (one object per touch, with a spoken count) moved children from counting towards conceptual subitizing ([Sinclair & Pimm 2015](https://link.springer.com/article/10.1007/s13394-015-0154-y)) **[QUAL]**.
- Fingu (showing quantities with finger configurations) showed pre/post gains with 5–7-year-olds ([Holgersson et al. 2016](https://www.researchgate.net/publication/304336337_Fingu-A_Game_to_Support_Children's_Development_of_Arithmetic_Competence_Theory_Design_and_Empirical_Research)) **[QE, no reported control]**.

The rule these suggest:

- one touch per unit for counting;
- a continuous drag for magnitude and hops;
- a group drag for tens and sets;
- never "tap to reveal", which matches no mathematical idea at all.

**Toys versus tools.** In an RPG everything looks like a toy. Second and third graders who were told a set of manipulatives were *math tools* learned more, transferred more and understood more than children told the same objects were for a game ([J. Cog. & Dev. 2021](https://www.tandfonline.com/doi/full/10.1080/15248372.2021.1890602)) **[RCT]**. Framing matters: this should be "Sparky's counting tray", with snapping slots and no flingable physics.

**Constraint has limits.** Martin & Schwartz found that *less* pre-structured environments transferred better ([2005](https://pubmed.ncbi.nlm.nih.gov/21702786/)). The reconciliation is to constrain *what the child can do*, not *what solution they reach*: every move is a count or a group move, but the child chooses which dots go where.

**Animation, when used at all, should show the change in steps.** Animation beats static pictures on average in the meta-analyses (Höffler & Leutner d = 0.37; Berney & Bétrancourt g = 0.23) ([Höffler & Leutner 2007](https://www.sciencedirect.com/science/article/abs/pii/S0959475207001077); [Berney & Bétrancourt 2016](https://access.archive-ouverte.unige.ch/access/metadata/75bf6876-cb91-4e2c-8ac6-9d0767449449/download)) **[MA]**. The details matter more than the averages:

- the mathematics estimate is null;
- the benefit held only when the system set the pace;
- adding written text wiped out the benefit.

Tversky et al. argue that many "animation wins" are really wins for *extra information or interactivity*. They add that continuous events are better shown as discrete steps ([Tversky et al. 2002](https://hci.stanford.edu/courses/cs448b/papers/Tversky_AnimationFacilitate_IJHCS02.pdf)) **[theory]**. Segmenting an animation helps, with small-to-medium effects ([Rey et al. 2019](https://link.springer.com/article/10.1007/s10648-018-9456-4)) **[MA]**.

The child's own action is a natural way to pace the steps. Each step happens when the child does it, so the display is segmented without any timer. That fits the no-time-pressure invariant.

**Generalising needs a move away from objects.** Novack et al. taught third graders a math strategy in one of three ways: acting on objects, miming the action, or an abstract gesture. All three helped on the problems they were trained on, but **only gesture generalised** ([Novack et al. 2014](https://pmc.ncbi.nlm.nih.gov/articles/PMC4672635/)) **[RCT; a commentary disputes how far this generalises]**. This backs the prior brief's concreteness-fading recommendation from a new angle. Dragging dots is the on-ramp; staying there forever is a mistake.

**One-off use doesn't work.** The fact that one-day use of a virtual manipulative has essentially zero effect means one reused, familiar tool per operation beats a new visual each time. This also argues against the spec's "Show me differently" method picker as the default, at least for pre-readers: a menu of text-labelled methods is both unreadable and a novelty cost.

**A note on who these studies involved.** The evidence here is mostly for ages 5–10. Relevant for Sparky, a study of toddlers learning words found that asking them to *tap the named object* helped the youngest, but slightly hurt slightly older toddlers who could already learn by watching ([Kirkorian et al. 2016](https://pubmed.ncbi.nlm.nih.gov/27018327/)) **[RCT]**. So "tap the group of 3" prompts should be gated by stage.

## Prioritized design implications

The recommendations below are ordered by leverage times strength of evidence, and they map onto the current visuals. Throughout:

- **C** means the child acts on the quantity. It is built on `logic::manipulate_concrete` and `logic::base_ten`.
- **R** means a structured picture, shown with narration and linking.

Rules that apply to every row:

- **Speech carries all the words.** For pre-readers, the only glyphs are numerals and + − × ÷. Sentence labels are removed. Readers aged about 7+ may get a short label that matches the narration.
- **Colour encodes one role consistently** across C, R and the equation. For example, blue = first addend everywhere, including the "3" in "3 + 4".
- **Uniform dots in frame slots.**
- **No red marks.** The current subtraction draws a red "X" through removed dots. That echoes "wrong", against invariant 7, so use fading or sliding away instead.
- **No timers.** Any quick-look reveal is hidden when the child taps, never on a countdown.

| # | Current visual | Change for **C** | Change for **R** | Evidence strength |
|---|---|---|---|---|
| 1 | **"Show me" as a static, countable answer picture (all operations)** | Replace it with a manipulative the child finishes. Help should never be a picture of the answer. | Graduated help. Level 1: the addends only, structured, with no total. Level 2: Sparky narrates and links. Bottom-out: the child completes the manipulative. Offer help proactively after an error, in-world ("want to look together?"), not only from a button. | Moderate (help gaming and ITS literature, mostly older students; Outhwaite's review ties explanatory feedback to better app outcomes ([BJET 2023](https://bera-journals.onlinelibrary.wiley.com/doi/full/10.1111/bjet.13339))) |
| 2 | **+ : unstructured blue/yellow row wrapping at 10** | Bands 1–2: two hands (Sparky's), first addend on one hand, second on the other. Later: the child drags yellow counters into a ten-frame that already holds blue, one drag per counter with a spoken count. Group drags come later. | A ten-frame (or double frame) filled blue then yellow, with a visible 5-break and empty slots showing "how many to 10". Next step: the first addend is covered by a chest with its numeral, so the child counts on. Then a quick look at the frame, hidden on tap. | Strong-moderate (Schiffman & Laski RCT; Poletti RCT ×3; Kreilinger longitudinal; pictured hands untested) |
| 3 | **− : last b dots crossed out, "count the blue ones!"** | Choose the model from the story. **Separate:** the child slides b counters out of a 5-structured frame (10 − 3 leaves "5 and 2"). **Compare:** two rows matched one-to-one, and the child taps the unmatched extras. **Close numbers (9 − 7):** count up. | The same three pictures, static and structured. Remove the "count the blue ones" prompt and the red X. | Moderate (CGI RCT, Hudson, Selter; no direct test of harm from mismatched visuals) |
| 4 | **Missing addend: part-whole bond** | Fill to a target. The frame shows the whole as slots, and the child adds counters until it is full. | Ages 4–6: the bond drawn with visible units in its parts. Ages 6–7: units that fuse into bars. 8+: unit-free bars. | Moderate for grade 1 schema-based diagrams (Fuchs RCT); thin for under-6 bar models |
| 5 | **× : row of groups colour-alternated, "3 groups of 4"** | The child builds equal groups by dragging a group-sized scoop, not single dots. | Early: equal groups separated by spacing or enclosures, with uniform colour inside groups and role colours on the numerals. Times-table bands: an array that can rotate to show 3×4 = 4×3. Remove the text; Sparky says "three groups of four". | Moderate-low (Barmby QUAL; Kaminski & Sloutsky on unit decoration) |
| 6 | **÷ : answer pre-dealt into b boxes, labelled "split into"** | **Sharing (start here):** the child deals pearls round-robin into buddy slots. **Grouping:** the child scoops groups of b into chests. Match the model to which quantity is unknown. | Show the dealing or grouping *action* in discrete narrated steps, not the finished result. Make the model agree with the story. | Moderate (Fischbein replication; Mulligan & Mitchelmore) |
| 7 | **Base-10 blocks, bands 5+** | The child trades ten ones for a rod ("clunk"), or breaks a rod into ones, using `logic::base_ten`. | Quick tens (stick = 10) only after the child has done the trade with their own hands. Keep the start state ghosted after regrouping. | Moderate (CRA evidence base in the prior brief; Novack for fading; Tversky on transience) |
| 8 | **Linking across C → R → A** | Every drag updates the numeral live, and Sparky says the count. | Sparky's pointer beam sweeps from set to numeral while he speaks. Link heavily at first, then less as the stage rises. | Moderate (Purpura; Richland; Richter MA, mostly older) |
| 9 | **Instrumentation** | Log which affordance was used and whether success followed. | Log fast "Show me" taps and correct answers that came only after help. Use them to **discount the accuracy and CRA credit** for that item, silently. | Moderate (knowledge-tracing contamination; Moyer-Packenham 2016: the same affordance helps some children and hinders others) |

## Where the evidence runs out

**The contrast we most need is untested.** No study I found compares, for ages 4–7 and arithmetic:

- the child builds the quantity;
- the child watches it being built;
- a static picture.

The closest evidence is word learning in toddlers, or fractions at ages 9–10.

**Much of the relevant evidence has a narrow base.**

- **Mostly older learners:** help-seeking and gaming, signalling, linking gestures, and modality/segmenting. All are extrapolated down to 4-year-olds.
- **Real fingers only:** the finger RCTs all used real fingers.
- **Small or practitioner-level:** the ten-frame and rekenrek evidence rests on one small RCT, plus longitudinal and quasi-experimental studies, with no rekenrek RCT at all. Bar models below age 6 have no controlled evidence.
- **Low-quality or inferred:** evidence for semantic matching comes from teacher-level RCTs and practitioner summaries of how hard each problem type is. Harm from a mismatched visual is inferred, not measured.
- **Uncertain for this age:** worked-example fading (Renkl and Atkinson) was not retrieved and is probably from older students, so "Sparky does one, then half" is a plausible transfer, not an established one.
- **Vendor-heavy app evidence:** app outcome evidence is weighted towards developers. Math Shelf's d = 0.94 comes from the developer and a researcher-built measure. Kim et al. find researcher-developed outcomes run about 0.26 SD higher than standardized ones ([Kim et al. 2021](https://journals.sagepub.com/doi/full/10.1177/23328584211004183)) **[MA]**. So in-game accuracy will overstate learning, and a separate check that learning transfers outside the game is needed.

**Two tensions remain open.**

- **Who sets the pace of an animation.** Animation benefits appeared only when the system set the pace. Tversky and the tapping study favour giving the child control. The child's own action as the pacer is a reasoned bridge, not a tested one.
- **Watching versus doing.** Boys learning better by watching is a single-study finding. It justifies keeping a "watch Sparky do it" fallback, not designing around it.

## Conclusion

Two findings change how the prior design documents should be read.

First, CRA stage is a property of **who acts on the model**, not which model is drawn. That turns the current "Concrete" hints into Representational ones, and changes the method-as-dial spec from a single axis to two.

Second, the research makes "Show me" a learning-model problem as well as a pedagogy problem. A countable answer picture is both a shortcut for a motivated grinder and a source of false mastery signals for the adaptive system. Fixing the visual and discounting help-assisted answers are the same fix.

The practical consequence is a better investment, not a bigger one. A few recurring, structured, narrated tools that the child's own hands drive fit every project invariant:

- two hands and a ten-frame for + and −;
- a dealing tray for ÷;
- an array for ×;
- a trading table for base-10.

Each passes the Broccoli Test because the manipulation *is* the story action. And since the key causal question remains open in the literature, the game's event log is in a good position to answer it: A/B "child builds" against "Sparky shows", measure count-on and response-time shifts per child, and let the adaptive system learn which scaffold actually moves each kid.
