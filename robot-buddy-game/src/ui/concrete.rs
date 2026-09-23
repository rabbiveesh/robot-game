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
//!   - Ten is a thing: a filled row of five glows; a full frame pulses and
//!     wears a "10" (and the buddy says so — see `Landed::filled_ten`).
//!   - Drag, not tap: a press-and-release in place slides the counter home.
//!     (Where the platform drops releases — X11 touchscreens — a second press
//!     places the carried counter instead, so it degrades to tap-tap.)
//!   - Every touch is math: only counters move, and only toward their target.
//!     The basket's b spaces are the "how many to take" — no text needed.
//!   - No words on screen: the buddy narrates (`intro_line`); a pre-reader can
//!     use this cold. The "10" badge is a numeral tied to its quantity.
//!
//! The math lives in `logic::manipulate_concrete`; this module is layout
//! (pure, hit-testable), pointer handling, and drawing, plus the purely visual
//! slides and glows that make each move legible.

use ::rand::Rng;
use crate::prelude::*;
use robot_buddy_domain::learning::challenge_generator::Challenge;
use robot_buddy_domain::logic::manipulate_concrete::{
    concrete_reducer, generate_concrete, ConcreteAction, ConcreteKind, ConcretePhase,
    ConcretePuzzle, ConcreteSession,
};
use robot_buddy_domain::types::Operation;

use crate::input::FrameInput;
use crate::ui::layout::UiRect;

/// The workspace's natural size: two blocks of five spaces, the gap between
/// them, and room either side for a frame's "10".
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
const SLOT_GHOST: Color = Color::new(1.0, 1.0, 1.0, 0.12);
const HELD_GAP: Color = Color::new(1.0, 1.0, 1.0, 0.45);
const ARROW: Color = Color::new(1.0, 1.0, 1.0, 0.25);
const GLOW: Color = Color::new(1.0, 0.95, 0.6, 1.0);
const TEN_BADGE: Color = Color::new(0.412, 0.941, 0.682, 1.0);   // #69F0AE

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

/// A counter the kid is carrying. UI-only — the domain session only changes
/// when it lands on target.
#[derive(Clone, Copy, Debug)]
pub struct Drag {
    pub group: u8,
    /// Which space it was lifted from (index into that zone's spaces). It
    /// stays empty while carried.
    pub slot: usize,
    pub pos: (f32, f32),
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
    /// A whole frame of ten just filled.
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
    slides: Vec<Slide>,
    glows: Vec<Glow>,
    clock: f32,
}

/// What a landed counter did, for the game to react to.
pub struct Landed {
    /// Every counter is where it's going: the model is built.
    pub built: bool,
    /// This counter completed a ten-frame.
    pub filled_ten: bool,
}

impl Workspace {
    pub fn new(session: ConcreteSession) -> Self {
        Workspace { session, drag: None, slides: vec![], glows: vec![], clock: 0.0 }
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

    /// Land the carried counter at `pos`: apply the move to the session, then
    /// slide every counter whose space changed from where it was shown.
    fn land(&mut self, action: ConcreteAction, pos: (f32, f32), area: UiRect) -> Landed {
        let before = layout(self, area);
        let shown: Vec<(CounterId, (f32, f32))> =
            before.counters().map(|c| (c.id, self.shown_at(c))).collect();
        let frame_count = |l: &Layout| l.in_frames.len();
        let had = frame_count(&before);

        self.drag = None;
        self.session = concrete_reducer(self.session.clone(), action);
        let after = layout(self, area);

        let mut slides = Vec::new();
        for c in after.counters() {
            let from = shown.iter().find(|(id, _)| *id == c.id).map(|&(_, p)| p).unwrap_or(pos);
            if from != c.center {
                slides.push(Slide { id: c.id, from, age: 0.0 });
            }
        }
        self.slides = slides;

        // Filling a row of five, or a whole ten, is worth a moment.
        let now = frame_count(&after);
        let mut filled_ten = false;
        if now > had {
            if now % 10 == 0 {
                self.glows.push(Glow { kind: GlowKind::Frame(now / 10 - 1), age: 0.0 });
                filled_ten = true;
            } else if now % 5 == 0 {
                self.glows.push(Glow { kind: GlowKind::Row(now / 10, 0), age: 0.0 });
            }
        }
        Landed { built: self.is_built(), filled_ten }
    }

    /// The carried counter missed: it glides back into the space it left.
    fn slide_home(&mut self, pos: (f32, f32), area: UiRect) {
        let Some(drag) = self.drag.take() else { return };
        let after = layout(self, area);
        let home = match after.source_zone() {
            Zone::Frames => after.frame_cells.get(drag.slot).copied(),
            Zone::Side => after.side_slots.get(drag.slot).map(|s| s.0),
        };
        let id = after.counters().find(|c| Some(c.center) == home).map(|c| c.id);
        if let Some(id) = id {
            self.slides = vec![Slide { id, from: pos, age: 0.0 }];
        }
    }
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

/// What the buddy says when a ten-frame fills up.
pub const FULL_TEN_LINE: &str = "A full ten!";

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
}

pub struct Layout {
    /// Ten-frame outlines, top to bottom.
    pub frames: Vec<UiRect>,
    /// Center of every frame cell, in fill order.
    pub frame_cells: Vec<(f32, f32)>,
    /// Counters sitting in the frames.
    pub in_frames: Vec<Counter>,
    /// The tray (put together) or basket (take away).
    pub side: UiRect,
    /// Every tray/basket space, with the group of the counter in it, if any.
    pub side_slots: Vec<((f32, f32), Option<u8>)>,
    /// Counters sitting in the tray/basket.
    pub in_side: Vec<Counter>,
    /// Counters the kid can pick up right now.
    pub grabbable: Vec<Counter>,
    /// Where a carried counter has to land to count.
    pub drop_zone: UiRect,
    /// Releases right of this x land (everything flows left to right).
    pub drop_line: f32,
    /// True when the tray is the source (put together); false when the frames
    /// are (take away).
    pub side_is_source: bool,
    /// The space the carried counter came from, drawn as an empty outline.
    pub held_gap: Option<(f32, f32)>,
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
            Some(Counter { center, group, id: CounterId { zone, group, rank }, slot })
        })
        .collect()
}

/// Pure layout. `area` is the rect the challenge panel reserves for the
/// workspace. A carried counter's space is left empty (see `held_gap`).
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
    } else {
        let taken = (p.a - s.bucket_a) as usize;
        for i in 0..p.b as usize {
            side_slots.push((slot_center(i / COLS, i % COLS), (i < taken).then_some(0)));
        }
    }

    // The carried counter's space stays empty while it's in hand.
    let mut held_gap = None;
    if let Some(d) = ws.drag {
        if side_is_source {
            if let Some(space) = side_slots.get_mut(d.slot) {
                space.1 = None;
                held_gap = Some(space.0);
            }
        } else if let Some(fill) = frame_fill.get_mut(d.slot) {
            *fill = None;
            held_gap = frame_cells.get(d.slot).copied();
        }
    }

    let frame_spaces: Vec<((f32, f32), Option<u8>)> =
        frame_cells.iter().copied().zip(frame_fill.iter().copied()).collect();
    let in_frames = place(Zone::Frames, &frame_spaces);
    let in_side = place(Zone::Side, &side_slots);

    let side_rows = side_slots.len().div_ceil(COLS).max(1);
    let side = UiRect { x: side_x, y: area.y, w: block_w, h: side_rows as f32 * cell };
    let frames_rect = UiRect {
        x: frames_x,
        y: area.y,
        w: block_w,
        h: n_frames as f32 * 2.0 * cell + (n_frames - 1) as f32 * frame_gap,
    };

    let done = s.phase == ConcretePhase::Complete;
    let (grabbable, drop_zone) = if side_is_source {
        (in_side.clone(), frames_rect)
    } else {
        (in_frames.clone(), side)
    };
    let grabbable = if done || ws.drag.is_some() { vec![] } else { grabbable };

    Layout {
        frames, frame_cells, in_frames, side, side_slots, in_side,
        grabbable, drop_zone, drop_line, side_is_source, held_gap,
        cell, counter_r: COUNTER_R * scale,
    }
}

// ─── INPUT ──────────────────────────────────────────────

pub enum Pointer {
    /// The pointer isn't doing anything here — let the rest of the panel have it.
    Idle,
    /// Picked up, carrying, or slid home. Consumed; nothing for the game.
    Busy,
    /// A counter landed on target and the session moved on.
    Landed(Landed),
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
        let (group, pos) = (drag.group, input.mouse_pos);
        let over_target = pos.0 >= layout(ws, area).drop_line
            && pos.1 >= area.y - DROP_Y_SLOP
            && pos.1 <= area.y + area.h + DROP_Y_SLOP;
        if over_target {
            let action = match ws.session.puzzle.kind {
                ConcreteKind::TakeAway => ConcreteAction::Remove { group },
                _ => ConcreteAction::Place { group },
            };
            crate::trace!("drag drop    group={group} at ({:.0},{:.0}) -> {action:?}", pos.0, pos.1);
            return Pointer::Landed(ws.land(action, pos, area));
        }
        crate::trace!("drag miss    group={group} at ({:.0},{:.0}) -> slides home", pos.0, pos.1);
        ws.slide_home(pos, area);
        return Pointer::Busy;
    }

    if input.mouse_clicked {
        let l = layout(ws, area);
        let (mx, my) = input.mouse_pos;
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
            ws.drag = Some(Drag { group: c.group, slot: c.slot, pos: input.mouse_pos });
            ws.slides.retain(|s| s.id != c.id);
            return Pointer::Busy;
        }
        crate::trace!("drag nothing to grab at ({mx:.0},{my:.0})");
    }
    Pointer::Idle
}

// ─── DRAWING ────────────────────────────────────────────

fn group_color(group: u8) -> Color {
    if group == 0 { BLUE } else { YELLOW }
}

fn draw_counter(x: f32, y: f32, r: f32, color: Color) {
    draw_circle(x, y, r, color);
    draw_circle_lines(x, y, r, 2.0, OUTLINE);
}

/// Quick rise, slow fade: 0 → 1 → 0 over a glow's life.
fn glow_strength(age: f32) -> f32 {
    let t = (age / GLOW_S).clamp(0.0, 1.0);
    if t < 0.15 { t / 0.15 } else { 1.0 - (t - 0.15) / 0.85 }
}

pub fn draw(ws: &Workspace, area: UiRect) {
    let l = layout(ws, area);

    for (i, f) in l.frames.iter().enumerate() {
        draw_rectangle(f.x, f.y, f.w, f.h, FRAME_BG);

        for g in &ws.glows {
            let a = glow_strength(g.age);
            match g.kind {
                GlowKind::Row(fi, row) if fi == i => {
                    let y = f.y + row as f32 * l.cell;
                    draw_rectangle(f.x, y, f.w, l.cell, Color { a: 0.35 * a, ..GLOW });
                }
                GlowKind::Frame(fi) if fi == i => {
                    draw_rectangle(f.x, f.y, f.w, f.h, Color { a: 0.3 * a, ..GLOW });
                }
                _ => {}
            }
        }

        for col in 1..COLS {
            let x = f.x + col as f32 * l.cell;
            draw_line(x, f.y, x, f.y + f.h, 1.5, FRAME_LINE);
        }
        draw_line(f.x, f.y + l.cell, f.x + f.w, f.y + l.cell, 1.5, FRAME_LINE);

        // A full frame is a ten: thicker border, and it wears the numeral on
        // its outer side (away from the tray/basket).
        let full = l.in_frames.iter().filter(|c| c.slot / 10 == i).count() == 10;
        let pulse = ws.glows.iter().find(|g| g.kind == GlowKind::Frame(i)).map_or(0.0, |g| glow_strength(g.age));
        let border = if full { 4.0 + 3.0 * pulse } else { 3.0 };
        let border_color = if full { TEN_BADGE } else { FRAME_LINE };
        draw_rectangle_lines(f.x, f.y, f.w, f.h, border, border_color);
        if full {
            let size = (30.0 + 14.0 * pulse) * l.cell / CELL;
            let tw = measure_text("10", None, size as u16, 1.0).width;
            let tx = if l.side_is_source { f.x + f.w + 12.0 } else { f.x - 12.0 - tw };
            draw_text("10", tx, f.y + f.h / 2.0 + size * 0.35, size, TEN_BADGE);
        }
    }

    // Basket outline (take away) — its empty spaces say how many to take.
    if !l.side_is_source {
        let b = l.side;
        draw_rectangle_lines(b.x - 4.0, b.y - 4.0, b.w + 8.0, b.h + 8.0, 3.0, FRAME_LINE);
    }
    for &((x, y), fill) in &l.side_slots {
        if fill.is_none() {
            draw_circle_lines(x, y, l.counter_r, 1.5, SLOT_GHOST);
        }
    }
    if let Some((x, y)) = l.held_gap {
        draw_circle_lines(x, y, l.counter_r, 2.0, HELD_GAP);
    }

    // A quiet chevron: counters travel left to right.
    let mid_y = area.y + l.cell;
    draw_line(l.drop_line - 8.0, mid_y - 10.0, l.drop_line + 6.0, mid_y, 3.0, ARROW);
    draw_line(l.drop_line - 8.0, mid_y + 10.0, l.drop_line + 6.0, mid_y, 3.0, ARROW);

    for c in l.counters() {
        let (x, y) = ws.shown_at(c);
        let color = match (c.id.zone, l.side_is_source) {
            (Zone::Side, false) => BLUE_TAKEN, // in the basket: taken away
            _ => group_color(c.group),
        };
        draw_counter(x, y, l.counter_r, color);
    }
}

/// The carried counter, drawn last so it rides above the whole panel. Lifted
/// — bigger, shadowed, ringed — so it's obvious it's in hand even when the
/// platform can't report the finger moving.
pub fn draw_drag(ws: &Workspace, area: UiRect) {
    if let Some(d) = ws.drag {
        let (x, y) = d.pos;
        let r = COUNTER_R * scale_in(area) * 1.3;
        let ring = r + 6.0 + 3.0 * (ws.clock * 6.0).sin();
        draw_circle(x + 4.0, y + 7.0, r, SHADOW);
        draw_circle_lines(x, y, ring, 3.0, Color { a: 0.8, ..group_color(d.group) });
        draw_counter(x, y - 4.0, r, group_color(d.group));
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
        assert_eq!(l.held_gap, Some(second.center));
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
