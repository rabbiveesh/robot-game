# Grouping (Unitizing) — Design Spec

## Why this is its own axis

CRA answers **how the math is shown**: objects the kid acts on, pictures, or symbols.
Grouping answers **what the kid sees when they look at a quantity**: a pile of ones,
"a five and some", "a ten and some", or "tens" as things you can count.

The two move independently. A kid can bundle ten real sticks (Concrete, unitizing)
or read `34` as three tens and four ones with no picture at all (Abstract,
unitizing). A kid at the Abstract stage for addition who still counts
everything by ones is common; so is the reverse. Tracking them together would
hide one behind the other.

Research grounding (see `cra-visualization-research.md` §"Structure beats
counting"): structured displays (5/10 groupings) move kindergartners from
counting everything to counting on, where unstructured dots do not
(Schiffman & Laski 2018); skill with patterned sets predicts later addition,
skill with random sets doesn't (Kreilinger et al. 2021). The stage ladder below
follows the early-number trajectories (Clements & Sarama's composition of
number; Cobb & Wheatley on children's "ten"; Fuson's multi-digit stages). Those
three are cited from background knowledge, not re-verified in this repo's
research passes.

## The ladder

| Stage | What 7 looks like | What 14 looks like | Typical age |
|---|---|---|---|
| **G0 Ones** | 1, 2, 3 … 7 | counts all 14 | 3–5 |
| **G1 Five** | "a five and two", no counting | — | 4–6 |
| **G2 Ten** | — | "a ten and four" | 5–6 (K.NBT.1) |
| **G3 Ten is a unit** | — | "1 ten, 4 ones"; counts "10, 20, 30" | 6–7 (1.NBT.2) |
| **G4 Tens are flexible** | — | 14 − 6: "break the ten" | 7–8 |

Ages are typical, not gates. Individual kids — especially twice-exceptional
ones — jump and leave gaps, so **sense the stage, never sequence it**.

**One track for the whole kid, not per operation.** Seeing a ten is number
sense, not an addition skill, and one track settles on less data. (Revisit if
the data shows kids grouping in addition but not subtraction.)

## How the game makes each step felt (never taught)

No labels, no quiz, no "this is called a ten". Each nudge is something that
happens in the world, and each fades once the stage above it is solid.

| Moving toward | Nudge | Status |
|---|---|---|
| G1 | Counters live in rows of five everywhere (tray, frames, basket). A filled row of five glows. | built |
| G1 → G2 | A full frame pulses, turns green, wears a "10"; the buddy says "A full ten!" | built |
| G1 → G2 | **Drag a whole row of five as one piece.** A full row in the tray sits on a stick; grab the stick, move all five. | this branch |
| G2 → G3 | **Tap a full frame and it snaps into a ten-rod**: the ten counters slide into one bar. Tap the rod and it opens back into a frame. | this branch |
| G2 → G3 | Teen problems start with one frame already full and **under a lid marked "10"**, so the fastest route is counting on from ten. | later |
| G3 → G4 | Take-away from a teen number starts with the ten as a rod; to take more ones than are loose, the kid has to **open the rod** — that's borrowing. Joins up with `logic::base_ten` (`TradeUp` / `BreakDown`). | later |
| G3 | **Bags of ten Dum Dums** in the economy: loose Dum Dums bundle into bags, bigger prices read "3 bags and 4", paying with bags gets change. | later (touches economy + save) |

### Row of five, as one piece

Manches et al. (2010) found that how the interface lets children move blocks
changes the strategies they find: one at a time leads to counting; moving a
group allows part-whole strategies. So the workspace offers both, and the
choice is itself the signal.

- A full row of five in the tray sits on a **stick**. Pressing the stick
  picks up the whole row; pressing a counter still picks up one. (Take-away
  doesn't offer row sticks yet.)
- The five land in the next five empty cells, in order, sliding like singles.
- The buddy mentions it **once**, only after the kid has moved five singles
  while a full row was available: "Psst! Grab the stick to slide a whole row at
  once!" After that, stick use is the kid's call.

### Ten-rod

- Tap a full frame: its ten counters slide into a single horizontal bar across
  the frame (colors kept, so 8 + 2 still shows as 8 blue and 2 yellow segments),
  and the "10" sits on it. Tap the bar: it opens back into the frame.
- Purely a change of how the ten is shown; the domain session's counts don't
  change. It's a UI state of the workspace.
- A rod can't take or give singles. In take-away, a counter can't be pulled
  from a rod; the kid has to open it first. That's the G4 action, available early for
  any kid who finds it.

## Sensing the stage (stealth, never a quiz)

All of these are read from play the kid is already doing:

1. **Response time against the numbers.** If response time grows with the
   total, the kid is counting everything; with the smaller addend, counting on;
   flat, the fact is known (Groen & Parkman 1972 — background knowledge, not
   re-verified). Needs a regression over the rolling window per operation; fits
   the existing `response_time_ms` field.
2. **Group moves in the workspace.** Row-stick use vs one-at-a-time when a
   full row is available; rod snaps; rod opens during take-away.
3. **Teen numbers.** 10 + n answered fast vs slow separates G2 from G1.

The stage lives on `LearnerProfile` (event-sourced like everything else), shows
on the parent dashboard only, and drives the fades: once G2 is solid the buddy
stops announcing full tens; once G3 is solid, teen problems start with the lid.

## A note on subtraction

A kid can solve the shop's change questions ("it costs 7, you gave 10, how
much back?") and still not get bare subtraction. That fits the research on
problem types: counting up to a target (a missing-addend story) is much easier
for young kids than take-away, and both are easier in a story than as symbols.
So the subtraction workspace should also offer a **fill-to-target** model: the
frames show the target outlined, the kid starts at the smaller number and fills
up to it, and the answer is how many were added. That's the shop's change
question in ten-frame form, and it's the bridge from counting up to `a − b`.
Not on this branch; noted here because the grouping nudges make it stronger
(filling up to a ten, then past it).

## Build order

1. Row-of-five drag, with the one-time buddy mention. *(this branch)*
2. Tap-to-snap ten-rod, and tap to open. *(this branch)*
3. `GroupingStage` on `LearnerProfile`: a reducer fed by the group-move events
   and a response-time slope over the rolling window; parent-dashboard line;
   nudge fades keyed off it.
4. Fill-to-target subtraction model.
5. Lid on the first ten for teen problems; start teen take-aways as a rod.
6. Bags of ten Dum Dums.

## Invariants check

- **No time pressure:** response times are read silently; nothing is timed.
- **No labels:** no "ten", "unit" or stage names shown to the kid; the "10" is a
  numeral on its quantity.
- **Broccoli Test:** every nudge is a thing that happens to counters the kid
  is already moving (or Dum Dums they already want). Remove the math and there's
  nothing left to do.
- **Reducers only:** the grouping stage is a profile field changed by events;
  row moves are repeated domain `Place`/`Remove` actions; rods are UI state.
