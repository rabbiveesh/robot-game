//! Concrete-stage "Show me": the kid builds the problem with their hands.
//!
//! A static picture of dots is a Representational aid — and a countable one
//! gives the answer away. At the Concrete stage "Show me" opens this workspace
//! instead: the kid DRAGS counters, and each drag is one step of the math.
//!   - Put together (a + b): a blue and b yellow counters wait in a tray; the
//!     kid slides them into ten-frames.
//!   - Take away (a − b): the ten-frames start with a counters; the kid slides
//!     them out into a basket with exactly b spaces.
//! The multiple-choice quiz still decides the answer — building the model never
//! resolves the challenge (that's why the old auto-completing manipulative was
//! retired). See `docs/cra-visualization-research.md`.
//!
//! Design rules from the research, and where they live here:
//!   - Structured, not scattered: counters snap into 2×5 ten-frames, filled in
//!     order. Pull one from the middle and the rest slide over, so the frame is
//!     always "full up to here" and 8 + 5 shows up as a full ten and 3 more.
//!   - Ten is a thing (`docs/grouping-axis-spec.md`): a filled row of five
//!     glows; a full frame pulses and wears a "10" (the buddy says so — see
//!     `Landed::filled_ten`). A full row in the tray sits on a stick that moves
//!     all five at once; tapping a full frame's "10" snaps its ten counters
//!     into one rod, and tapping the rod opens it again.
//!   - Drag, not tap: a press-and-release in place slides the counter home.
//!     (Where the platform drops releases — X11 touchscreens — a second press
//!     places the carried counter instead, so it degrades to tap-tap.)
//!   - Every touch is math: only counters move, and only toward their target.
//!     The basket's b spaces are the "how many to take" — no text needed.
//!   - No words on screen: the buddy narrates (`intro_line`); a pre-reader can
//!     use this cold. The "10" badge is a numeral tied to its quantity.
//!
//! The math lives in `logic::manipulate_concrete` (a row move is five `Place`s;
//! a rod is only a way of showing a full frame). This module is layout (pure,
//! hit-testable), pointer handling, and drawing, plus the purely visual slides
//! and glows that make each move legible.

use ::rand::Rng;
use crate::prelude::*;
use robot_buddy_domain::learning::challenge_generator::Challenge;
use robot_buddy_domain::logic::manipulate_concrete::{
    concrete_reducer, generate_concrete, ConcreteAction, ConcreteKind, ConcretePhase,
    ConcretePuzzle, ConcreteSession,
};
use robot_buddy_domain::types::Operation;

use crate::input::FrameInput;
use crate::ui::layout::paint::{self, Canvas};
use crate::ui::layout::{FontMetrics, TextMetrics, UiRect};

/// The workspace's natural size: two blocks of five spaces, the gap between
/// them, and room either side (the row sticks' handles on the left of the
/// tray; a frame's "10" on the frames' outer side).
const NATURAL_H: f32 = 184.0;
const BADGE_ROOM: f32 = 46.0;
const NATURAL_W: f32 = 2.0 * COLS as f32 * CELL + MID_GAP + 2.0 * BADGE_ROOM;

/// Finger-sized: ~44px is the usual minimum touch target.
const CELL: f32 = 42.0;
const COUNTER_R: f32 = 17.0;
const COLS: usize = 5;
const FRAME_GAP: f32 = 12.0;
const MID_GAP: f32 = 70.0;
const MAX_TRAY_ROWS: usize = 4;
/// How far past the workspace's top/bottom a release still counts.
const DROP_Y_SLOP: f32 = 48.0;

/// Slide duration (seconds) for a counter moving between spaces.
const SLIDE_S: f32 = 0.22;
const GLOW_S: f32 = 0.9;

const BLUE: Color = Color::new(0.259, 0.647, 0.961, 1.0);        // #42A5F5
const YELLOW: Color = Color::new(1.0, 0.835, 0.310, 1.0);        // #FFD54F
const BLUE_TAKEN: Color = Color::new(0.259, 0.647, 0.961, 0.45);
const OUTLINE: Color = Color::new(0.0, 0.0, 0.0, 0.35);
const SHADOW: Color = Color::new(0.0, 0.0, 0.0, 0.35);
const FRAME_BG: Color = Color::new(1.0, 1.0, 1.0, 0.06);
const FRAME_LINE: Color = Color::new(0.690, 0.745, 0.773, 0.9);  // #B0BEC5
const FRAME_LINE_FAINT: Color = Color::new(0.690, 0.745, 0.773, 0.25);
const SLOT_GHOST: Color = Color::new(1.0, 1.0, 1.0, 0.12);
const HELD_GAP: Color = Color::new(1.0, 1.0, 1.0, 0.45);
const ARROW: Color = Color::new(1.0, 1.0, 1.0, 0.25);
const GLOW: Color = Color::new(1.0, 0.95, 0.6, 1.0);
const TEN_BADGE: Color = Color::new(0.412, 0.941, 0.682, 1.0);   // #69F0AE
const STICK: Color = Color::new(0.635, 0.490, 0.353, 1.0);       // wood
const STICK_GRIP: Color = Color::new(0.0, 0.0, 0.0, 0.3);

/// The two places counters live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Frames,
    /// The tray (put together) or basket (take away).
    Side,
}

/// Which counter is which, for animation: the n-th counter of a group within
/// a zone, in reading order. Identical counters don't need more identity than
/// that — a slide is "the 3rd blue one moved".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CounterId {
    pub zone: Zone,
    pub group: u8,
    pub rank: usize,
}

/// What the kid is carrying: one counter, or a whole row of five. UI-only —
/// the domain session only changes when it lands on target.
#[derive(Clone, Debug)]
pub struct Drag {
    pub group: u8,
    /// The spaces it was lifted from (indices into the source zone's spaces).
    /// They stay empty while carried.
    pub slots: Vec<usize>,
    pub pos: (f32, f32),
    /// Picked up one by one while a whole row was there for the taking.
    single_past_a_row: bool,
}

impl Drag {
    pub fn is_row(&self) -> bool {
        self.slots.len() > 1
    }
}

/// A counter gliding from where it was drawn to its new space.
#[derive(Clone, Copy, Debug)]
struct Slide {
    id: CounterId,
    from: (f32, f32),
    age: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum GlowKind {
    /// A row of five just filled (frame index, row 0/1).
    Row(usize, usize),
    /// A whole frame of ten just filled, or became a rod.
    Frame(usize),
}

#[derive(Clone, Copy, Debug)]
struct Glow {
    kind: GlowKind,
    age: f32,
}

pub struct Workspace {
    pub session: ConcreteSession,
    pub drag: Option<Drag>,
    /// Which frames the kid has snapped into a ten-rod. Only honored while the
    /// frame is full.
    rods: [bool; 2],
    /// Singles moved while a whole row was available (the row-stick nudge
    /// waits for a few of these).
    singles_past_a_row: usize,
    slides: Vec<Slide>,
    glows: Vec<Glow>,
    clock: f32,
}

/// What a landing did, for the game to react to.
pub struct Landed {
    /// Every counter is where it's going: the model is built.
    pub built: bool,
    /// This move completed a ten-frame.
    pub filled_ten: bool,
    /// It was a whole row of five, moved as one.
    pub row: bool,
    /// How many singles the kid has moved while a row was available, so far.
    pub singles_past_a_row: usize,
}

impl Workspace {
    pub fn new(session: ConcreteSession) -> Self {
        Workspace {
            session,
            drag: None,
            rods: [false; 2],
            singles_past_a_row: 0,
            slides: vec![],
            glows: vec![],
            clock: 0.0,
        }
    }

    /// A workspace for this challenge, or `None` if it doesn't fit on the
    /// ten-frames (the caller falls back to the static picture).
    pub fn for_challenge(challenge: &Challenge, rng: &mut impl Rng) -> Option<Workspace> {
        Some(Workspace::new(ConcreteSession::new(puzzle_for(challenge, rng)?)))
    }

    pub fn is_built(&self) -> bool {
        self.session.phase == ConcretePhase::Complete
    }

    /// Anything still moving? (Tests wait on this; the game doesn't care.)
    pub fn is_settled(&self) -> bool {
        self.slides.is_empty()
    }

    /// Advance the slides and glows.
    pub fn tick(&mut self, dt: f32) {
        self.clock += dt;
        for s in &mut self.slides {
            s.age += dt;
        }
        self.slides.retain(|s| s.age < SLIDE_S);
        for g in &mut self.glows {
            g.age += dt;
        }
        self.glows.retain(|g| g.age < GLOW_S);
    }

    /// Where a counter is on screen right now: its space, or partway there.
    fn shown_at(&self, c: &Counter) -> (f32, f32) {
        match self.slides.iter().find(|s| s.id == c.id) {
            Some(s) => {
                let t = (s.age / SLIDE_S).clamp(0.0, 1.0);
                let e = 1.0 - (1.0 - t).powi(3); // ease out
                (s.from.0 + (c.center.0 - s.from.0) * e, s.from.1 + (c.center.1 - s.from.1) * e)
            }
            None => c.center,
        }
    }

    /// Where every counter is drawn right now, before a change.
    fn snapshot(&self, area: UiRect) -> Vec<(CounterId, (f32, f32))> {
        layout(self, area).counters().map(|c| (c.id, self.shown_at(c))).collect()
    }

    /// After a change: every counter whose space moved slides there from where
    /// it was drawn. Counters that didn't exist before come from `arrivals`,
    /// in order (where the kid let go).
    fn slide_from(&mut self, before: &[(CounterId, (f32, f32))], arrivals: &[(f32, f32)], area: UiRect) {
        let after = layout(self, area);
        let mut arrivals = arrivals.iter();
        let mut slides = Vec::new();
        for c in after.counters() {
            let from = match before.iter().find(|(id, _)| *id == c.id) {
                Some(&(_, p)) => p,
                None => arrivals.next().copied().unwrap_or(c.center),
            };
            if from != c.center {
                slides.push(Slide { id: c.id, from, age: 0.0 });
            }
        }
        self.slides = slides;
    }

    /// Land what's carried at `pos`: apply the move to the session (once per
    /// counter), then slide everything whose space changed.
    fn land(&mut self, pos: (f32, f32), area: UiRect) -> Landed {
        let Some(drag) = self.drag.clone() else {
            return Landed { built: self.is_built(), filled_ten: false, row: false, singles_past_a_row: self.singles_past_a_row };
        };
        let before = self.snapshot(area);
        let had = layout(self, area).in_frames.len();
        let cell = layout(self, area).cell;

        let action = match self.session.puzzle.kind {
            ConcreteKind::TakeAway => ConcreteAction::Remove { group: drag.group },
            _ => ConcreteAction::Place { group: drag.group },
        };
        self.drag = None;
        for _ in &drag.slots {
            self.session = concrete_reducer(self.session.clone(), action);
        }
        if drag.single_past_a_row {
            self.singles_past_a_row += 1;
        }
        // A rod only stands while its frame is full.
        let full = frames_full(&layout(self, area));
        for (i, rod) in self.rods.iter_mut().enumerate() {
            *rod &= full.get(i).copied().unwrap_or(false);
        }

        self.slide_from(&before, &carried_positions(pos, drag.slots.len(), cell), area);

        // Filling a row of five, or a whole ten, is worth a moment — for every
        // one this move completed (a row of five can finish both).
        let now = layout(self, area).in_frames.len();
        let mut filled_ten = false;
        for k in (had + 1)..=now {
            if k % 10 == 0 {
                self.glows.push(Glow { kind: GlowKind::Frame(k / 10 - 1), age: 0.0 });
                filled_ten = true;
            } else if k % 5 == 0 {
                self.glows.push(Glow { kind: GlowKind::Row(k / 10, 0), age: 0.0 });
            }
        }
        Landed { built: self.is_built(), filled_ten, row: drag.is_row(), singles_past_a_row: self.singles_past_a_row }
    }

    /// What's carried missed: it glides back into the spaces it left.
    fn slide_home(&mut self, pos: (f32, f32), area: UiRect) {
        let Some(drag) = self.drag.clone() else { return };
        let cell = layout(self, area).cell;
        self.drag = None;
        // The returning counters are the ones whose spaces were gaps.
        let after = layout(self, area);
        let homes: Vec<(f32, f32)> = drag
            .slots
            .iter()
            .filter_map(|&slot| match after.source_zone() {
                Zone::Frames => after.frame_cells.get(slot).copied(),
                Zone::Side => after.side_slots.get(slot).map(|s| s.0),
            })
            .collect();
        let from = carried_positions(pos, drag.slots.len(), cell);
        self.slides = after
            .counters()
            .filter_map(|c| {
                let k = homes.iter().position(|&h| h == c.center)?;
                Some(Slide { id: c.id, from: from[k], age: 0.0 })
            })
            .collect();
    }

    /// Snap a full frame into a ten-rod, or open a rod back into a frame.
    fn toggle_rod(&mut self, frame: usize, area: UiRect) -> bool {
        let before = self.snapshot(area);
        self.rods[frame] = !self.rods[frame];
        self.slide_from(&before, &[], area);
        if self.rods[frame] {
            self.glows.push(Glow { kind: GlowKind::Frame(frame), age: 0.0 });
        }
        self.rods[frame]
    }
}

/// Where carried counters sit around the pointer: one under the finger, or a
/// row of five centered on it.
fn carried_positions(pos: (f32, f32), n: usize, cell: f32) -> Vec<(f32, f32)> {
    let mid = (n as f32 - 1.0) / 2.0;
    (0..n).map(|k| (pos.0 + (k as f32 - mid) * cell, pos.1)).collect()
}

/// The manipulatives on the control room's bench, in the order it cycles
/// them: put together, then take away. Add new manipulatives here to make them
/// reachable from the dev zone.
pub const BENCH: &[Operation] = &[Operation::Add, Operation::Sub];

/// Which problems the ten-frame workspace can hold: standard-format + and −
/// that fit in two frames (and, for addition, a tray of at most four rows).
pub fn puzzle_for(challenge: &Challenge, rng: &mut impl Rng) -> Option<ConcretePuzzle> {
    let n = &challenge.numbers;
    if n.format != "standard" || n.a < 1 || n.b < 1 {
        return None;
    }
    match n.op.as_str() {
        "+" if n.a + n.b <= 20 && tray_rows(n.a as usize, n.b as usize) <= MAX_TRAY_ROWS => {
            Some(generate_concrete(ConcreteKind::AddGroups, n.a as u8, n.b as u8, rng))
        }
        "-" if n.a <= 20 && n.b <= 10 && n.b < n.a => {
            Some(generate_concrete(ConcreteKind::TakeAway, n.a as u8, n.b as u8, rng))
        }
        _ => None,
    }
}

/// What the buddy says when the workspace opens — the only instructions a
/// pre-reader gets, so it names the action, never the answer.
pub fn intro_line(ws: &Workspace) -> &'static str {
    match ws.session.puzzle.kind {
        ConcreteKind::TakeAway => "Let's build it! Slide them into the basket until it's full.",
        _ => "Let's build it! Slide the blue ones into the boxes, then the yellow ones.",
    }
}

/// What the buddy says when a ten-frame fills up.
pub const FULL_TEN_LINE: &str = "A full ten!";

/// The once-only nudge toward moving a row as one piece, after the kid moves
/// `ROW_NUDGE_AFTER` singles while a whole row sat there.
pub const ROW_NUDGE_LINE: &str = "Psst! Grab the stick to slide a whole row at once!";
pub const ROW_NUDGE_AFTER: usize = 5;

/// The least of its natural height the workspace will give up on a short
/// screen (the panel lays it out between this and natural).
pub const MIN_HEIGHT_SHARE: f32 = 0.45;

/// How much to shrink everything to fit `area` (never grows past natural).
fn scale_in(area: UiRect) -> f32 {
    (area.w / NATURAL_W).min(area.h / NATURAL_H).min(1.0)
}

/// The room the workspace wants when it may be at most `max_w` wide. The
/// panel may hand it less height; `layout` scales to whatever it gets.
pub fn extent(_ws: &Workspace, max_w: f32) -> (f32, f32) {
    let s = (max_w / NATURAL_W).min(1.0);
    (NATURAL_W * s, NATURAL_H * s)
}

fn rows_for(n: usize) -> usize {
    n.div_ceil(COLS)
}

fn tray_rows(a: usize, b: usize) -> usize {
    rows_for(a) + rows_for(b)
}

// ─── LAYOUT (testable) ─────────────────────────────────

/// A counter sitting in a space.
#[derive(Clone, Copy, Debug)]
pub struct Counter {
    pub center: (f32, f32),
    pub group: u8,
    pub id: CounterId,
    /// Index of its space within its zone (what a `Drag` records).
    pub slot: usize,
    /// Part of a ten-rod (drawn as a segment of the bar, can't be picked up).
    pub in_rod: bool,
}

/// A full row of five in the tray, on a stick that carries all five.
#[derive(Clone, Debug)]
pub struct Stick {
    /// The knob to grab, left of the row.
    pub handle: UiRect,
    /// The row itself (the stick runs behind it).
    pub row: UiRect,
    pub slots: Vec<usize>,
    pub group: u8,
}

pub struct Layout {
    /// Ten-frame outlines, top to bottom.
    pub frames: Vec<UiRect>,
    /// Center of every frame cell, in fill order.
    pub frame_cells: Vec<(f32, f32)>,
    /// Which frames are showing as a ten-rod.
    pub rods: Vec<bool>,
    /// Where each full frame's "10" sits (tap it to snap / open the rod).
    pub badges: Vec<Option<UiRect>>,
    /// Counters sitting in the frames.
    pub in_frames: Vec<Counter>,
    /// The tray (put together) or basket (take away).
    pub side: UiRect,
    /// Every tray/basket space, with the group of the counter in it, if any.
    pub side_slots: Vec<((f32, f32), Option<u8>)>,
    /// Counters sitting in the tray/basket.
    pub in_side: Vec<Counter>,
    /// Counters the kid can pick up one at a time right now.
    pub grabbable: Vec<Counter>,
    /// Full rows the kid can pick up whole right now.
    pub sticks: Vec<Stick>,
    /// Where a carried counter has to land to count.
    pub drop_zone: UiRect,
    /// Releases right of this x land (everything flows left to right).
    pub drop_line: f32,
    /// True when the tray is the source (put together); false when the frames
    /// are (take away).
    pub side_is_source: bool,
    /// The spaces the carried counters came from, drawn as empty outlines.
    pub held_gaps: Vec<(f32, f32)>,
    /// Space size and counter radius at this scale.
    pub cell: f32,
    pub counter_r: f32,
}

impl Layout {
    pub fn counters(&self) -> impl Iterator<Item = &Counter> {
        self.in_frames.iter().chain(self.in_side.iter())
    }

    fn source_zone(&self) -> Zone {
        if self.side_is_source { Zone::Side } else { Zone::Frames }
    }
}

/// Which frames hold all ten.
fn frames_full(l: &Layout) -> Vec<bool> {
    (0..l.frames.len())
        .map(|i| l.in_frames.iter().filter(|c| c.slot / 10 == i).count() == 10)
        .collect()
}

/// Give each occupied space a counter, numbering each group in reading order.
fn place(zone: Zone, spaces: &[((f32, f32), Option<u8>)]) -> Vec<Counter> {
    let mut seen = [0usize; 2];
    spaces
        .iter()
        .enumerate()
        .filter_map(|(slot, &(center, fill))| {
            let group = fill?;
            let rank = seen[group as usize];
            seen[group as usize] += 1;
            Some(Counter { center, group, id: CounterId { zone, group, rank }, slot, in_rod: false })
        })
        .collect()
}

/// Pure layout. `area` is the rect the challenge panel reserves for the
/// workspace. A carried counter's space is left empty (see `held_gaps`).
pub fn layout(ws: &Workspace, area: UiRect) -> Layout {
    let s = &ws.session;
    let p = &s.puzzle;
    let scale = scale_in(area);
    let (cell, frame_gap, mid_gap) = (CELL * scale, FRAME_GAP * scale, MID_GAP * scale);

    let block_w = COLS as f32 * cell;
    let start_x = area.x + (area.w - (block_w * 2.0 + mid_gap)) / 2.0;
    let (left_x, right_x) = (start_x, start_x + block_w + mid_gap);
    let drop_line = left_x + block_w + mid_gap / 2.0;

    let side_is_source = p.kind != ConcreteKind::TakeAway;
    let (frames_x, side_x) = if side_is_source { (right_x, left_x) } else { (left_x, right_x) };

    // Ten-frames: one, or two when the numbers pass ten.
    let biggest = if side_is_source { p.target } else { p.a };
    let n_frames = if biggest > 10 { 2 } else { 1 };
    let frames: Vec<UiRect> = (0..n_frames)
        .map(|i| UiRect {
            x: frames_x,
            y: area.y + i as f32 * (2.0 * cell + frame_gap),
            w: block_w,
            h: 2.0 * cell,
        })
        .collect();
    let frame_cells: Vec<(f32, f32)> = frames
        .iter()
        .flat_map(|f| {
            (0..10).map(move |k| {
                let (col, row) = (k % COLS, k / COLS);
                (f.x + col as f32 * cell + cell / 2.0, f.y + row as f32 * cell + cell / 2.0)
            })
        })
        .collect();

    // Frame contents: group A fills first, then group B — "a, then b more"
    // reads left-to-right, top-to-bottom, across the ten.
    let mut frame_fill: Vec<Option<u8>> = vec![None; frame_cells.len()];
    let groups = std::iter::repeat(0).take(s.bucket_a as usize)
        .chain(std::iter::repeat(1).take(if side_is_source { s.bucket_b as usize } else { 0 }));
    for (cell, g) in frame_fill.iter_mut().zip(groups) {
        *cell = Some(g);
    }

    // Tray / basket spaces, rows of five.
    let slot_center = |row: usize, col: usize| {
        (side_x + col as f32 * cell + cell / 2.0, area.y + row as f32 * cell + cell / 2.0)
    };
    let mut side_slots = Vec::new();
    // For the tray: where each group's rows start (in rows) and its spaces begin.
    let mut tray_rows_of: Vec<(u8, usize, usize, usize)> = Vec::new(); // (group, first slot, count, first row)
    if side_is_source {
        // Blue rows first, yellow starts on a fresh row: two visible groups.
        let left_a = (p.a - s.bucket_a) as usize;
        let left_b = (p.b - s.bucket_b) as usize;
        for i in 0..p.a as usize {
            side_slots.push((slot_center(i / COLS, i % COLS), (i < left_a).then_some(0)));
        }
        let b_row = rows_for(p.a as usize);
        for i in 0..p.b as usize {
            side_slots.push((slot_center(b_row + i / COLS, i % COLS), (i < left_b).then_some(1)));
        }
        tray_rows_of.push((0, 0, p.a as usize, 0));
        tray_rows_of.push((1, p.a as usize, p.b as usize, b_row));
    } else {
        let taken = (p.a - s.bucket_a) as usize;
        for i in 0..p.b as usize {
            side_slots.push((slot_center(i / COLS, i % COLS), (i < taken).then_some(0)));
        }
    }

    // The carried counters' spaces stay empty while they're in hand.
    let mut held_gaps = Vec::new();
    if let Some(d) = &ws.drag {
        for &slot in &d.slots {
            if side_is_source {
                if let Some(space) = side_slots.get_mut(slot) {
                    space.1 = None;
                    held_gaps.push(space.0);
                }
            } else if let Some(fill) = frame_fill.get_mut(slot) {
                *fill = None;
                if let Some(&c) = frame_cells.get(slot) {
                    held_gaps.push(c);
                }
            }
        }
    }

    let frame_spaces: Vec<((f32, f32), Option<u8>)> =
        frame_cells.iter().copied().zip(frame_fill.iter().copied()).collect();
    let mut in_frames = place(Zone::Frames, &frame_spaces);
    let in_side = place(Zone::Side, &side_slots);

    // Rods: a full frame the kid has snapped. Its counters line up as the ten
    // segments of one bar across the middle of the frame.
    let full: Vec<bool> = (0..n_frames)
        .map(|i| in_frames.iter().filter(|c| c.slot / 10 == i).count() == 10)
        .collect();
    let rods: Vec<bool> = (0..n_frames).map(|i| full[i] && ws.rods[i]).collect();
    for c in &mut in_frames {
        let fi = c.slot / 10;
        if rods[fi] {
            let f = frames[fi];
            let k = c.slot % 10;
            c.center = (f.x + (k as f32 + 0.5) * f.w / 10.0, f.y + f.h / 2.0);
            c.in_rod = true;
        }
    }
    let badge_w = (BADGE_ROOM * scale - 6.0).max(12.0);
    let badges: Vec<Option<UiRect>> = frames
        .iter()
        .zip(&full)
        .map(|(f, &full)| {
            full.then(|| {
                let x = if side_is_source { f.x + f.w + 4.0 } else { f.x - 4.0 - badge_w };
                UiRect { x, y: f.y, w: badge_w, h: f.h }
            })
        })
        .collect();

    let side_rows = side_slots.len().div_ceil(COLS).max(1);
    let side = UiRect { x: side_x, y: area.y, w: block_w, h: side_rows as f32 * cell };
    let frames_rect = UiRect {
        x: frames_x,
        y: area.y,
        w: block_w,
        h: n_frames as f32 * 2.0 * cell + (n_frames - 1) as f32 * frame_gap,
    };

    let done = s.phase == ConcretePhase::Complete;
    let carrying = ws.drag.is_some();
    let (grabbable, drop_zone) = if side_is_source {
        (in_side.clone(), frames_rect)
    } else {
        (in_frames.iter().filter(|c| !c.in_rod).copied().collect(), side)
    };
    let grabbable = if done || carrying { vec![] } else { grabbable };

    // Sticks: every full row of five in the source — the tray, or (taking
    // away) the frames, when the basket has room for all five. They stay drawn
    // while something's carried (a row with a gap in it isn't full, so it
    // loses its stick); pickup is already blocked mid-carry.
    let mut sticks = Vec::new();
    let basket_room = side_slots.iter().filter(|s| s.1.is_none()).count();
    if !side_is_source && !done && basket_room >= COLS {
        // The handle sits at the row's right end, pointing at the basket (the
        // "10" has the frames' left side).
        let handle_w = (mid_gap * 0.4 - 4.0).max(12.0);
        for (fi, f) in frames.iter().enumerate() {
            if rods[fi] {
                continue; // a rod is one ten; open it to break off a five
            }
            for r in 0..2 {
                let first = fi * 10 + r * COLS;
                let slots: Vec<usize> = (first..first + COLS).collect();
                if slots.iter().all(|&i| frame_fill.get(i).copied().flatten() == Some(0)) {
                    let y = f.y + r as f32 * cell;
                    sticks.push(Stick {
                        handle: UiRect { x: f.x + f.w + 4.0, y, w: handle_w, h: cell },
                        row: UiRect { x: f.x, y, w: f.w, h: cell },
                        slots,
                        group: 0,
                    });
                }
            }
        }
    }
    if side_is_source && !done {
        for &(group, first, count, first_row) in &tray_rows_of {
            for r in 0..count / COLS {
                let slots: Vec<usize> = (0..COLS).map(|c| first + r * COLS + c).collect();
                if slots.iter().all(|&i| side_slots.get(i).is_some_and(|s| s.1 == Some(group))) {
                    let y = area.y + (first_row + r) as f32 * cell;
                    let handle_w = (BADGE_ROOM * scale - 8.0).max(12.0);
                    sticks.push(Stick {
                        handle: UiRect { x: side_x - handle_w - 4.0, y, w: handle_w, h: cell },
                        row: UiRect { x: side_x, y, w: block_w, h: cell },
                        slots,
                        group,
                    });
                }
            }
        }
    }

    Layout {
        frames, frame_cells, rods, badges, in_frames, side, side_slots, in_side,
        grabbable, sticks, drop_zone, drop_line, side_is_source, held_gaps,
        cell, counter_r: COUNTER_R * scale,
    }
}

// ─── INPUT ──────────────────────────────────────────────

pub enum Pointer {
    /// The pointer isn't doing anything here — let the rest of the panel have it.
    Idle,
    /// Picked up, carrying, or slid home. Consumed; nothing for the game.
    Busy,
    /// Counters landed on target and the session moved on.
    Landed(Landed),
    /// A full frame snapped into a ten-rod (`bundled`), or a rod opened.
    Rod { frame: usize, bundled: bool },
}

/// Advance the drag by one frame of input.
pub fn handle_pointer(ws: &mut Workspace, input: &FrameInput, area: UiRect) -> Pointer {
    if let Some(drag) = ws.drag.as_mut() {
        drag.pos = input.mouse_pos;
        // A fresh press while carrying means the release got lost (X11
        // touchscreens never send one): treat the press as the drop.
        let let_go = input.mouse_released || !input.mouse_down || input.mouse_clicked;
        if !let_go {
            return Pointer::Busy;
        }
        let (group, n, pos) = (drag.group, drag.slots.len(), input.mouse_pos);
        let over_target = pos.0 >= layout(ws, area).drop_line
            && pos.1 >= area.y - DROP_Y_SLOP
            && pos.1 <= area.y + area.h + DROP_Y_SLOP;
        if over_target {
            crate::trace!("drag drop    group={group} x{n} at ({:.0},{:.0})", pos.0, pos.1);
            return Pointer::Landed(ws.land(pos, area));
        }
        crate::trace!("drag miss    group={group} x{n} at ({:.0},{:.0}) -> slides home", pos.0, pos.1);
        ws.slide_home(pos, area);
        return Pointer::Busy;
    }

    if !input.mouse_clicked {
        return Pointer::Idle;
    }
    let l = layout(ws, area);
    let (mx, my) = input.mouse_pos;

    // The "10" on a full frame, or the rod itself: snap / open.
    for (i, badge) in l.badges.iter().enumerate() {
        let on_badge = badge.is_some_and(|b| b.contains(mx, my));
        let on_rod = l.rods[i] && l.frames[i].contains(mx, my);
        // Putting together, the frames aren't a source — the whole full frame
        // is a fair place to tap.
        let on_full_frame = badge.is_some() && l.side_is_source && l.frames[i].contains(mx, my);
        if on_badge || on_rod || on_full_frame {
            let bundled = ws.toggle_rod(i, area);
            crate::trace!("rod {} frame={i}", if bundled { "snap" } else { "open" });
            return Pointer::Rod { frame: i, bundled };
        }
    }

    // A stick's handle picks up the whole row.
    if let Some(stick) = l.sticks.iter().find(|s| s.handle.expand(6.0).contains(mx, my)) {
        crate::trace!("drag grab    row of {} group={}", stick.slots.len(), stick.group);
        ws.drag = Some(Drag { group: stick.group, slots: stick.slots.clone(), pos: input.mouse_pos, single_past_a_row: false });
        return Pointer::Busy;
    }

    // Anywhere in a counter's space grabs it — fingers are fat.
    let reach = l.cell * 0.6;
    let nearest = l
        .grabbable
        .iter()
        .map(|c| (c, (c.center.0 - mx).hypot(c.center.1 - my)))
        .filter(|&(_, d)| d <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((c, d)) = nearest {
        crate::trace!("drag grab    group={} slot={} at ({mx:.0},{my:.0}), {d:.0}px from center", c.group, c.slot);
        let single_past_a_row = !l.sticks.is_empty();
        ws.drag = Some(Drag { group: c.group, slots: vec![c.slot], pos: input.mouse_pos, single_past_a_row });
        ws.slides.retain(|s| s.id != c.id);
        return Pointer::Busy;
    }
    crate::trace!("drag nothing to grab at ({mx:.0},{my:.0})");
    Pointer::Idle
}

// ─── DRAWING ────────────────────────────────────────────
//
// Everything paints through `paint::Canvas` (ADR-004): the workspace's own art
// on a canvas bound to its layout region, and what moves across the panel —
// counters sliding in from a drop, the one in hand — on a canvas bound to the
// whole frame.

fn group_color(group: u8) -> Color {
    if group == 0 { BLUE } else { YELLOW }
}

fn paint_counter(c: &Canvas, x: f32, y: f32, r: f32, color: Color) {
    c.circle(x, y, r, color);
    c.circle_lines(x, y, r, 2.0, OUTLINE);
}

/// One segment of a ten-rod, centered on (x, y).
fn paint_segment(c: &Canvas, x: f32, y: f32, w: f32, h: f32, color: Color) {
    let seg = UiRect::new(x - w / 2.0, y - h / 2.0, w, h);
    c.rect(seg, color);
    c.rect_lines(seg, 1.5, OUTLINE);
}

/// A stick running behind a row, with a knob to grab at one end.
fn paint_stick(c: &Canvas, handle: UiRect, row: UiRect, cell: f32, lift: f32) {
    let bar_h = cell * 0.3;
    let y = row.y + row.h / 2.0 - bar_h / 2.0 - lift;
    let knob_x = handle.x + handle.w / 2.0;
    let (x0, x1) = (knob_x.min(row.x), knob_x.max(row.x + row.w));
    c.rect(UiRect::new(x0, y, x1 - x0, bar_h), STICK);
    let knob = UiRect::new(handle.x, handle.y + handle.h * 0.15 - lift, handle.w, handle.h * 0.7);
    c.rect(knob, STICK);
    c.rect_lines(knob, 2.0, OUTLINE);
    for k in 1..4 {
        let gy = knob.y + knob.h * k as f32 / 4.0;
        c.line(knob.x + 4.0, gy, knob.x + knob.w - 4.0, gy, 1.5, STICK_GRIP);
    }
}

/// Quick rise, slow fade: 0 → 1 → 0 over a glow's life.
fn glow_strength(age: f32) -> f32 {
    let t = (age / GLOW_S).clamp(0.0, 1.0);
    if t < 0.15 { t / 0.15 } else { 1.0 - (t - 0.15) / 0.85 }
}

/// Paint the workspace into `area` (its layout region). `bounds` is the whole
/// frame, for counters partway through a slide.
pub fn draw(ws: &Workspace, area: UiRect, bounds: UiRect) {
    let l = layout(ws, area);
    let c = paint::canvas(area);
    let seg_w = l.cell * 0.95;
    let seg_h = l.cell * 0.8;

    for (i, f) in l.frames.iter().enumerate() {
        c.rect(*f, FRAME_BG);

        for g in &ws.glows {
            let a = glow_strength(g.age);
            match g.kind {
                GlowKind::Row(fi, row) if fi == i => {
                    let band = UiRect::new(f.x, f.y + row as f32 * l.cell, f.w, l.cell);
                    c.rect(band, Color { a: 0.35 * a, ..GLOW });
                }
                GlowKind::Frame(fi) if fi == i => c.rect(*f, Color { a: 0.3 * a, ..GLOW }),
                _ => {}
            }
        }

        // As a rod the grid fades back: the ten is one thing now.
        let grid = if l.rods[i] { FRAME_LINE_FAINT } else { FRAME_LINE };
        for col in 1..COLS {
            let x = f.x + col as f32 * l.cell;
            c.line(x, f.y, x, f.y + f.h, 1.5, grid);
        }
        c.line(f.x, f.y + l.cell, f.x + f.w, f.y + l.cell, 1.5, grid);

        // A full frame is a ten: thicker border, and it wears the numeral on
        // its outer side (away from the tray/basket). Tap it to make a rod.
        let pulse = ws.glows.iter().find(|g| g.kind == GlowKind::Frame(i)).map_or(0.0, |g| glow_strength(g.age));
        let full = l.badges[i].is_some();
        let border = if full { 4.0 + 3.0 * pulse } else { 3.0 };
        c.rect_lines(*f, border, if full { TEN_BADGE } else { FRAME_LINE });
        if let Some(b) = l.badges[i] {
            let size = (((30.0 + 14.0 * pulse) * l.cell / CELL) as u16).max(10);
            let tw = FontMetrics::bundled().width("10", size);
            c.text("10", b.x + (b.w - tw) / 2.0, b.y + b.h / 2.0 + size as f32 * 0.35, size, TEN_BADGE);
        }
    }

    // Sticks behind the source's full rows.
    for stick in &l.sticks {
        paint_stick(&c, stick.handle, stick.row, l.cell, 0.0);
    }

    // Basket outline (take away) — its empty spaces say how many to take.
    if !l.side_is_source {
        let b = l.side;
        c.rect_lines(UiRect::new(b.x - 4.0, b.y - 4.0, b.w + 8.0, b.h + 8.0), 3.0, FRAME_LINE);
    }
    for &((x, y), fill) in &l.side_slots {
        if fill.is_none() {
            c.circle_lines(x, y, l.counter_r, 1.5, SLOT_GHOST);
        }
    }
    for &(x, y) in &l.held_gaps {
        c.circle_lines(x, y, l.counter_r, 2.0, HELD_GAP);
    }

    // A quiet chevron: counters travel left to right (stepped clear of the
    // take-away knobs).
    let mid_y = area.y + l.cell;
    let cx = l.drop_line + if l.side_is_source { 0.0 } else { 8.0 };
    c.line(cx - 8.0, mid_y - 10.0, cx + 6.0, mid_y, 3.0, ARROW);
    c.line(cx - 8.0, mid_y + 10.0, cx + 6.0, mid_y, 3.0, ARROW);

    // Counters, some mid-slide from wherever they were let go.
    let moving = paint::canvas(bounds);
    for counter in l.counters() {
        let (x, y) = ws.shown_at(counter);
        let color = match (counter.id.zone, l.side_is_source) {
            (Zone::Side, false) => BLUE_TAKEN, // in the basket: taken away
            _ => group_color(counter.group),
        };
        if counter.in_rod {
            paint_segment(&moving, x, y, seg_w / 2.0, seg_h, color);
        } else {
            paint_counter(&moving, x, y, l.counter_r, color);
        }
    }
}

/// What's carried, drawn last so it rides above the whole panel. Lifted —
/// bigger, shadowed, ringed — so it's obvious it's in hand even when the
/// platform can't report the finger moving. A row travels on its stick.
/// `area` is the workspace (for scale); `bounds` the whole frame.
pub fn draw_drag(ws: &Workspace, area: UiRect, bounds: UiRect) {
    let Some(d) = &ws.drag else { return };
    let c = paint::canvas(bounds);
    let scale = scale_in(area);
    let cell = CELL * scale;
    let r = COUNTER_R * scale * if d.is_row() { 1.1 } else { 1.3 };
    let color = group_color(d.group);
    let spots = carried_positions(d.pos, d.slots.len(), cell);
    if d.is_row() {
        let first = spots[0];
        let row = UiRect::new(first.0 - cell / 2.0, first.1 - cell / 2.0, cell * COLS as f32, cell);
        let handle = if ws.session.puzzle.kind == ConcreteKind::TakeAway {
            let handle_w = (MID_GAP * scale * 0.4 - 4.0).max(12.0);
            UiRect::new(row.x + row.w + 4.0, row.y, handle_w, cell)
        } else {
            let handle_w = (BADGE_ROOM * scale - 8.0).max(12.0);
            UiRect::new(row.x - handle_w - 4.0, row.y, handle_w, cell)
        };
        paint_stick(&c, handle, row, cell, 4.0);
    }
    for &(x, y) in &spots {
        c.circle(x + 4.0, y + 7.0, r, SHADOW);
    }
    if !d.is_row() {
        let ring = r + 6.0 + 3.0 * (ws.clock * 6.0).sin();
        c.circle_lines(d.pos.0, d.pos.1, ring, 3.0, Color { a: 0.8, ..color });
    }
    for &(x, y) in &spots {
        paint_counter(&c, x, y - 4.0, r, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::rand::SeedableRng;
    use ::rand::rngs::SmallRng;
    use robot_buddy_domain::learning::challenge_generator::generate_challenge_at;

    const AREA: UiRect = UiRect { x: 100.0, y: 100.0, w: NATURAL_W, h: NATURAL_H };

    fn ws(kind: ConcreteKind, a: u8, b: u8) -> Workspace {
        Workspace::new(ConcreteSession::new(generate_concrete(kind, a, b, &mut SmallRng::seed_from_u64(1))))
    }

    fn press(p: (f32, f32)) -> FrameInput {
        FrameInput::empty().with_mouse_click(p.0, p.1)
    }

    fn release(p: (f32, f32)) -> FrameInput {
        FrameInput::empty().with_mouse_release(p.0, p.1)
    }

    fn drag(w: &mut Workspace, from: (f32, f32), to: (f32, f32)) -> Pointer {
        handle_pointer(w, &press(from), AREA);
        handle_pointer(w, &release(to), AREA)
    }

    fn tap(w: &mut Workspace, at: (f32, f32)) -> Pointer {
        handle_pointer(w, &press(at), AREA)
    }

    #[test]
    fn put_together_fills_blue_first_then_yellow_across_the_ten() {
        let mut w = ws(ConcreteKind::AddGroups, 8, 5);
        w.session.bucket_a = 8;
        w.session.bucket_b = 5;
        let l = layout(&w, AREA);
        assert_eq!(l.frames.len(), 2, "13 needs a second frame");
        let groups: Vec<u8> = l.in_frames.iter().map(|c| c.group).collect();
        assert_eq!(groups, [vec![0; 8], vec![1; 5]].concat());
    }

    #[test]
    fn the_space_you_lift_from_is_the_one_left_empty() {
        let mut w = ws(ConcreteKind::AddGroups, 4, 2);
        let second = layout(&w, AREA).in_side[1];
        handle_pointer(&mut w, &press(second.center), AREA);
        let l = layout(&w, AREA);
        assert_eq!(l.held_gaps, vec![second.center]);
        assert!(l.in_side.iter().all(|c| c.center != second.center));
        assert_eq!(l.in_side.len(), 5, "the others stay put while it's carried");
    }

    #[test]
    fn landing_one_from_the_middle_slides_the_rest_over() {
        let mut w = ws(ConcreteKind::AddGroups, 4, 2);
        let second = layout(&w, AREA).in_side[1].center;
        let target = layout(&w, AREA).drop_zone.center();
        assert!(matches!(drag(&mut w, second, target), Pointer::Landed(_)));

        let l = layout(&w, AREA);
        let blues: Vec<_> = l.in_side.iter().filter(|c| c.group == 0).collect();
        assert_eq!(blues.len(), 3);
        assert!(blues.iter().enumerate().all(|(i, c)| c.slot == i), "tray closes up from the left");
        assert!(!w.is_settled(), "the shift is animated, not a jump");
        w.tick(1.0);
        assert!(w.is_settled());
    }

    #[test]
    fn a_miss_glides_home_and_changes_nothing() {
        let mut w = ws(ConcreteKind::AddGroups, 3, 2);
        let from = layout(&w, AREA).in_side[0].center;
        assert!(matches!(drag(&mut w, from, (AREA.x, AREA.y)), Pointer::Busy));
        assert_eq!(w.session.total(), 0);
        assert!(w.drag.is_none());
        assert!(!w.is_settled(), "it slides back rather than teleporting");
    }

    #[test]
    fn a_second_press_places_a_counter_whose_release_was_lost() {
        // X11 touchscreens: press, (no motion, no release), press again.
        let mut w = ws(ConcreteKind::AddGroups, 3, 2);
        let from = layout(&w, AREA).in_side[0].center;
        let target = layout(&w, AREA).drop_zone.center();
        handle_pointer(&mut w, &press(from), AREA);
        handle_pointer(&mut w, &FrameInput::empty().with_mouse_held(from.0, from.1), AREA);
        assert!(matches!(handle_pointer(&mut w, &press(target), AREA), Pointer::Landed(_)));
        assert_eq!(w.session.total(), 1);
    }

    #[test]
    fn filling_a_frame_is_announced() {
        let mut w = ws(ConcreteKind::AddGroups, 9, 3);
        w.session.bucket_a = 9;
        let yellow = layout(&w, AREA).in_side.iter().find(|c| c.group == 1).unwrap().center;
        let target = layout(&w, AREA).drop_zone.center();
        match drag(&mut w, yellow, target) {
            Pointer::Landed(l) => assert!(l.filled_ten, "the tenth counter completes the frame"),
            _ => panic!("should have landed"),
        }
    }

    #[test]
    fn a_full_row_rides_a_stick_and_moves_as_one() {
        let mut w = ws(ConcreteKind::AddGroups, 7, 5); // blue: one full row + 2; yellow: one full row
        let l = layout(&w, AREA);
        assert_eq!(l.sticks.len(), 2, "one stick per full row");
        let yellow_row = l.sticks.iter().find(|s| s.group == 1).unwrap().handle.center();
        let target = l.drop_zone.center();
        match drag(&mut w, yellow_row, target) {
            Pointer::Landed(landed) => assert!(landed.row),
            _ => panic!("the row should land"),
        }
        assert_eq!(w.session.bucket_b, 5, "all five in one move");
    }

    #[test]
    fn a_partial_row_has_no_stick() {
        let w = ws(ConcreteKind::AddGroups, 4, 3);
        assert!(layout(&w, AREA).sticks.is_empty());
    }

    #[test]
    fn a_row_landing_across_the_ten_fills_the_frame() {
        // 8 + 5: the five finish the first ten and spill 3 into the next.
        let mut w = ws(ConcreteKind::AddGroups, 8, 5);
        w.session.bucket_a = 8;
        let l = layout(&w, AREA);
        let stick = l.sticks.iter().find(|s| s.group == 1).unwrap().handle.center();
        match drag(&mut w, stick, l.drop_zone.center()) {
            Pointer::Landed(landed) => assert!(landed.filled_ten && landed.built),
            _ => panic!("should land"),
        }
    }

    #[test]
    fn singles_moved_past_a_whole_row_are_counted() {
        let mut w = ws(ConcreteKind::AddGroups, 5, 1);
        for _ in 0..2 {
            let one = layout(&w, AREA).grabbable.iter().find(|c| c.group == 0).unwrap().center;
            let target = layout(&w, AREA).drop_zone.center();
            drag(&mut w, one, target);
        }
        // Only the first was taken with a full row still there.
        assert_eq!(w.singles_past_a_row, 1);
    }

    #[test]
    fn tapping_the_ten_snaps_the_frame_into_a_rod_and_back() {
        let mut w = ws(ConcreteKind::TakeAway, 13, 2);
        let l = layout(&w, AREA);
        let badge = l.badges[0].expect("the first frame starts full").center();
        assert!(matches!(tap(&mut w, badge), Pointer::Rod { frame: 0, bundled: true }));

        let l = layout(&w, AREA);
        assert!(l.rods[0]);
        assert_eq!(l.in_frames.iter().filter(|c| c.in_rod).count(), 10);
        assert_eq!(l.grabbable.len(), 3, "a rod's ones can't be picked off it");

        w.tick(1.0);
        let rod = l.frames[0].center();
        assert!(matches!(tap(&mut w, rod), Pointer::Rod { frame: 0, bundled: false }));
        assert!(!layout(&w, AREA).rods[0]);
    }

    #[test]
    fn a_rod_only_stands_while_its_frame_is_full() {
        let mut w = ws(ConcreteKind::TakeAway, 11, 5);
        let badge = layout(&w, AREA).badges[0].unwrap().center();
        tap(&mut w, badge);
        // Take the one loose counter; the rod still holds ten.
        let loose = layout(&w, AREA).grabbable[0].center;
        let basket = layout(&w, AREA).drop_zone.center();
        drag(&mut w, loose, basket);
        assert!(layout(&w, AREA).rods[0], "ten is still ten");
        assert!(layout(&w, AREA).grabbable.is_empty(), "to take more, the ten has to be opened");
    }

    #[test]
    fn taking_away_five_can_be_one_move() {
        // 12 − 5: the first frame's rows ride sticks; one row fills the basket.
        let mut w = ws(ConcreteKind::TakeAway, 12, 5);
        let l = layout(&w, AREA);
        assert_eq!(l.sticks.len(), 2, "both rows of the full frame");
        let knob = l.sticks[1].handle;
        assert!(knob.x > l.frames[0].x + l.frames[0].w, "the knob points at the basket");
        match drag(&mut w, knob.center(), l.drop_zone.center()) {
            Pointer::Landed(landed) => assert!(landed.row && landed.built),
            _ => panic!("the row should land in the basket"),
        }
        let l = layout(&w, AREA);
        assert_eq!(l.in_frames.len(), 7);
        assert!(l.in_frames.iter().enumerate().all(|(i, c)| c.slot == i), "the frames close up");
    }

    #[test]
    fn no_take_away_stick_without_room_for_five() {
        let w = ws(ConcreteKind::TakeAway, 12, 4);
        assert!(layout(&w, AREA).sticks.is_empty(), "the basket only holds four");
    }

    #[test]
    fn a_rod_has_no_sticks_until_it_is_opened() {
        let mut w = ws(ConcreteKind::TakeAway, 12, 6);
        let badge = layout(&w, AREA).badges[0].unwrap().center();
        tap(&mut w, badge);
        assert!(layout(&w, AREA).sticks.is_empty(), "a ten is one thing until it's opened");
    }

    #[test]
    fn take_away_basket_has_one_space_per_counter_to_take() {
        let w = ws(ConcreteKind::TakeAway, 12, 4);
        let l = layout(&w, AREA);
        assert_eq!(l.side_slots.len(), 4);
        assert_eq!(l.in_frames.len(), 12);
        assert_eq!(l.grabbable.len(), 12, "frame counters are the ones to take");
    }

    #[test]
    fn taking_from_the_middle_keeps_the_frame_filled_in_order() {
        let mut w = ws(ConcreteKind::TakeAway, 7, 2);
        let third = layout(&w, AREA).in_frames[2].center;
        let basket = layout(&w, AREA).drop_zone.center();
        drag(&mut w, third, basket);
        let l = layout(&w, AREA);
        assert_eq!(l.in_frames.len(), 6);
        assert!(l.in_frames.iter().enumerate().all(|(i, c)| c.slot == i));
        assert_eq!(l.in_side.len(), 1, "one in the basket");
    }

    #[test]
    fn workspace_only_takes_problems_that_fit_the_frames() {
        let mut rng = SmallRng::seed_from_u64(3);
        for seed in 0..40 {
            let mut r = SmallRng::seed_from_u64(seed);
            for op in [Operation::Add, Operation::Sub] {
                let ch = generate_challenge_at(2, op, &mut r);
                let p = puzzle_for(&ch, &mut rng).expect("band 2 +/− should fit");
                assert!(p.target <= 20);
            }
        }
        let mul = generate_challenge_at(5, Operation::Multiply, &mut rng);
        assert!(puzzle_for(&mul, &mut rng).is_none(), "× isn't a ten-frame job yet");
    }
}
