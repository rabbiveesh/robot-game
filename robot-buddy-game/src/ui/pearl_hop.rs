//! Pearl Hop — Shelly's slingshot minigame. Layout, scene geometry, input
//! mapping, the scene model and the cartoon; the rules live in
//! `robot_buddy_domain::logic::pearl_hop`.
//!
//! ```text
//!  ┌──────────────────────────────────────────────┐
//!  │ (o) 12                               [Leave] │  top row (layout)
//!  │   (• • •)                                    │
//!  │   Shelly        stepping stones              │  Scene region: custom art,
//!  │   [rock] ~ o ~ o ~ o ~ o ~ o ~ o ~ o ~ o ~~~ │  painted through a Canvas
//!  │ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ │  bound to the region
//!  │               "Find my pearl!"               │  caption (for grown-ups)
//!  │                  [ Again! ]                  │  once the win has played
//!  └──────────────────────────────────────────────┘
//! ```
//!
//! The kid can't read, so nothing important is only words: dots, stones,
//! Shelly's count-aloud and her antics carry the game. [`scene_model`] says
//! what the scene shows (and, at the counting stage, what it must NOT show:
//! where the pearl is) without drawing; [`SceneGeom`] maps a drag or a tap to
//! an aim with the exact numbers the drawing uses. Both are pure, so
//! `Game::step` and the headless tests use them too.

use crate::prelude::*;
use std::collections::BTreeSet;

use crate::input::FrameInput;
use crate::sprites::dressed::{self, Body, Outfit, Posture};
use crate::sprites::npcs::ClamFace;
use crate::sprites::swag::SwagFit;
use crate::sprites::Dir;
use crate::ui::layout::{self, col, paint, region, row, spacer, text, Align, Fit, Frame, Justify, Kind, Node, UiRect};
use robot_buddy_domain::logic::pearl_hop::{HopPhase, HopRound, HopSession, HopStage, SPLASH_SECS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HopId {
    PearlIcon,
    Count,
    Leave,
    LeaveIcon,
    LeaveLabel,
    Scene,
    Caption,
    Again,
    AgainIcon,
    AgainLabel,
}

/// What Shelly is wearing (from the wardrobe, same as in the world).
pub struct ShellyOutfit<'a> {
    pub worn: &'a BTreeSet<String>,
    pub color: &'a str,
}

/// What the panel shows this frame.
pub struct HopView<'a> {
    pub session: &'a HopSession,
    /// The kid's pearl purse as shown — the won pearl only lands in it when
    /// it arrives from the stone.
    pub pearls: u32,
    /// A few words for a grown-up reading along. The kid hears Shelly.
    pub caption: &'a str,
    pub outfit: ShellyOutfit<'a>,
}

pub struct PearlHopLayout {
    pub frame: Frame<HopId>,
    pub scene: SceneGeom,
}

impl PearlHopLayout {
    pub fn leave(&self) -> Option<UiRect> {
        self.frame.rect(HopId::Leave)
    }
    pub fn again(&self) -> Option<UiRect> {
        self.frame.rect(HopId::Again)
    }
}

fn icon_button(id: HopId, icon: HopId, label_id: HopId, label: &str, size: u16) -> Node<HopId> {
    row()
        .id(id)
        .hit()
        .justify(Justify::Center)
        .gap(8.0)
        .child(region(24.0, 24.0).id(icon).fixed())
        .child(text(label, size, Fit::shrink(14)).id(label_id))
}

pub fn layout(view: &HopView, screen: (f32, f32)) -> PearlHopLayout {
    let again = view.session.win_done();
    let top = row()
        .h(46.0)
        .fixed()
        .gap(8.0)
        .child(region(26.0, 26.0).id(HopId::PearlIcon).fixed())
        .child(text(view.pearls.to_string(), 24, Fit::shrink(14)).id(HopId::Count))
        .child(spacer())
        .child(icon_button(HopId::Leave, HopId::LeaveIcon, HopId::LeaveLabel, "Leave", 22).size(124.0, 44.0).fixed());
    // Reserved even before the win, so the scene never jumps when Again!
    // turns up; padded so it never sits against the caption.
    let bottom = row()
        .h(68.0)
        .fixed()
        .pad_edges(0.0, 10.0, 0.0, 4.0)
        .justify(Justify::Center)
        .maybe(again.then(|| {
            icon_button(HopId::Again, HopId::AgainIcon, HopId::AgainLabel, "Again!", 26).size(180.0, 54.0).fixed()
        }));
    let root = col()
        .pad(10.0)
        .gap(6.0)
        .child(top)
        .child(region(0.0, 0.0).id(HopId::Scene).w_pct(1.0).grow(1.0).min_h(0.0))
        .child(
            text(view.caption, 22, Fit::shrink_then_wrap(12, 2))
                .id(HopId::Caption)
                .center_text()
                .pad_xy(14.0, 4.0)
                .w_pct(1.0)
                .max_w(520.0)
                .align_self(Align::Center)
                .h(50.0)
                .fixed(),
        )
        .child(bottom);
    let bounds = layout::screen_rect(screen);
    let frame = layout::layout(&root, bounds);
    let scene_rect = frame.rect(HopId::Scene).unwrap_or(bounds);
    let scene = SceneGeom::new(scene_rect, bounds, &view.session.round);
    PearlHopLayout { frame, scene }
}

// ─── Scene geometry (pure) ──────────────────────────────

/// Pulls shorter than this are a tap, not a toss.
pub const DEAD_PULL: f32 = 12.0;

/// Where everything in the scene sits, derived from the Scene region alone.
#[derive(Debug, Clone, Copy)]
pub struct SceneGeom {
    pub rect: UiRect,
    /// The water line. Stones poke up out of it; misses splash into it.
    pub surface_y: f32,
    /// Centre x of the start rock (position 0).
    pub x0: f32,
    /// Screen pixels per stone.
    pub unit: f32,
    /// Shelly's radius.
    pub r: f32,
    pub stone_r: f32,
    /// The pull that sets the biggest aim.
    pub max_pull: f32,
    /// Unit vector the demo and tests drag along: back and down, but never
    /// off the left edge of the screen.
    drag_dir: (f32, f32),
    min_aim: u16,
    max_aim: u16,
}

impl SceneGeom {
    pub fn new(rect: UiRect, bounds: UiRect, round: &HopRound) -> Self {
        let r = (rect.h * 0.1).min(rect.w * 0.075).clamp(12.0, 34.0);
        let surface_y = rect.y + rect.h * 0.62;
        let x0 = rect.x + r + 16.0;
        let x_end = rect.right() - r - 10.0;
        let unit = ((x_end - x0) / round.span.max(1) as f32).max(0.01);
        let stone_r = (unit * 0.40).min(r * 0.95).max(2.5);
        let home_y = surface_y - r * 1.65; // == home().1
        // A pull has to fit on screen: Shelly sits at the left edge, so the
        // drag goes down and back, and the room below her caps it.
        let max_pull = ((bounds.bottom() - home_y) * 0.8).min(rect.w * 0.34).clamp(60.0, 200.0);
        let dx = ((x0 - bounds.x - 8.0) / max_pull).clamp(0.0, 0.28);
        let drag_dir = (-dx, (1.0 - dx * dx).sqrt());
        SceneGeom { rect, surface_y, x0, unit, r, stone_r, max_pull, drag_dir, min_aim: round.min_aim, max_aim: round.max_aim }
    }

    /// Screen x of a position. Past the right edge it pins to the edge, so a
    /// huge overshoot still splashes on screen.
    pub fn x_of(&self, pos: u16) -> f32 {
        (self.x0 + pos as f32 * self.unit).min(self.rect.right() - self.r - 2.0)
    }

    /// Top of the start rock (it stands taller than the stepping stones).
    pub fn rock_top(&self) -> f32 {
        self.surface_y - self.r * 1.1
    }

    pub fn stone_top(&self) -> f32 {
        self.surface_y - self.stone_r * 0.8
    }

    /// Shelly's centre when she's sitting on the start rock.
    pub fn home(&self) -> (f32, f32) {
        (self.x0, self.rock_top() - self.r * 0.55)
    }

    /// Shelly's centre sitting on position `pos`: the rock, a stone, or
    /// (when `in_water`) bobbing at the surface.
    pub fn perch(&self, pos: u16, in_water: bool) -> (f32, f32) {
        let x = self.x_of(pos);
        if pos == 0 {
            self.home()
        } else if in_water {
            (x, self.surface_y - self.r * 0.2)
        } else {
            (x, self.stone_top() - self.r * 0.55)
        }
    }

    /// How far from Shelly a press still grabs her: finger-sized.
    fn reach(&self) -> f32 {
        (self.r * 2.2).max(44.0)
    }

    /// Does a press at (x, y) grab Shelly?
    pub fn grabs(&self, x: f32, y: f32) -> bool {
        let (hx, hy) = self.home();
        (x - hx).powi(2) + (y - hy).powi(2) <= self.reach().powi(2)
    }

    /// Pull pixels per aim step: the pull range split evenly among the aims.
    fn bucket(&self) -> f32 {
        let n = (self.max_aim - self.min_aim) as f32 + 1.0;
        (self.max_pull - DEAD_PULL) / n
    }

    /// The aim a pull of `pull` pixels sets, or `None` for a pull too small
    /// to be a toss. Every aim gets an equal slice of the pull.
    pub fn aim_for_pull(&self, pull: f32) -> Option<u16> {
        if pull < DEAD_PULL {
            return None;
        }
        let i = ((pull - DEAD_PULL) / self.bucket()).floor().max(0.0) as u32;
        Some((self.min_aim as u32 + i).min(self.max_aim as u32) as u16)
    }

    /// The aim for the pointer at `p` while Shelly is held. Two gestures:
    ///
    /// * **Slingshot** — pointer behind her (back and down): the pull's
    ///   length sets the aim.
    /// * **Point at it** — pointer out over the path: the first hop lands on
    ///   the stone under it. This is also how a native Linux touchscreen
    ///   plays, where a held finger reports no motion: press Shelly, then tap
    ///   where she should land.
    pub fn aim_for_pointer(&self, p: (f32, f32)) -> Option<u16> {
        let (hx, hy) = self.home();
        if p.0 > hx + self.reach() {
            let stone = ((p.0 - self.x0) / self.unit).round().max(0.0) as u32;
            return Some(stone.clamp(self.min_aim as u32, self.max_aim as u32) as u16);
        }
        self.aim_for_pull(((p.0 - hx).powi(2) + (p.1 - hy).powi(2)).sqrt())
    }

    /// Inverse of [`aim_for_pull`]: the pull (pixels) in the middle of
    /// `aim`'s slice.
    pub fn pull_for_aim(&self, aim: u16) -> f32 {
        let i = (aim.clamp(self.min_aim, self.max_aim) - self.min_aim) as f32;
        DEAD_PULL + (i + 0.5) * self.bucket()
    }

    /// Where to drag the pointer to set `aim`: back and down from Shelly, the
    /// way a slingshot is drawn. Tests and the demo use this.
    pub fn drag_point_for(&self, aim: u16) -> (f32, f32) {
        let (hx, hy) = self.home();
        let pull = self.pull_for_aim(aim);
        (hx + self.drag_dir.0 * pull, hy + self.drag_dir.1 * pull)
    }

    /// Where to tap to point the first hop at stone `aim`. (Only meaningful
    /// for stones out past Shelly's reach.)
    pub fn tap_point_for(&self, aim: u16) -> (f32, f32) {
        (self.x0 + aim as f32 * self.unit, self.stone_top())
    }
}

// ─── Scene model (pure) ─────────────────────────────────

/// How a stone is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lit {
    Dark,
    /// Counted out after a landing, within the target (a dot ticked off).
    Counted,
    /// Counted out past the last dot: she went further than the target.
    PastTarget,
    /// Touched down on mid-toss (the skip-counting stages).
    Touched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoneView {
    pub pos: u16,
    /// The number painted under it, if this stage numbers its stones.
    pub label: Option<u16>,
    pub lit: Lit,
}

/// What Shelly's bubble over the start rock shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetView {
    None,
    /// The counting stage: N dots (no numeral), `ticked` of them counted off.
    Dots { n: u16, ticked: u16 },
    /// "In K hops": K bubbles, `used` of them popped.
    Hops { k: u8, used: u8 },
}

/// Everything in the scene that carries information, decided without
/// drawing anything — so a test can check that the counting stage never
/// shows where the pearl is before Shelly lands on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneModel {
    pub stones: Vec<StoneView>,
    /// A distinct pearl rock at this position (stages that name the target).
    pub pearl_rock: Option<u16>,
    /// A pearl drawn at this position.
    pub pearl_shown: Option<u16>,
    /// Whether positions have numbers at all (the "0" on the start rock).
    pub numbered: bool,
    /// Whether the aim ghost carries the hop size as a number.
    pub ghost_label: bool,
    pub target: TargetView,
    /// Numbers popped over touch-downs (skip counting out loud).
    pub hop_counts: Vec<(u16, u16)>,
    /// What Shelly has on.
    pub shelly_wears: Vec<String>,
}

pub fn scene_model(view: &HopView) -> SceneModel {
    let s = view.session;
    let r = &s.round;
    let toss = s.toss.as_ref();
    let shelly_wears: Vec<String> = view.outfit.worn.iter().cloned().collect();
    let pearl_leaving = s.win_clock().is_some_and(|c| c >= WIN_PEARL_LEAVES);
    match r.stage {
        HopStage::Count => {
            // A row of identical, unnumbered stones; the pearl is under one
            // of them and nothing says which. After a landing the stones she
            // covered are counted out one by one against the dots.
            let tallied = s.tallied();
            let stones = (1..=r.span)
                .map(|n| {
                    let lit = match n <= tallied {
                        true if n <= r.pearl => Lit::Counted,
                        true => Lit::PastTarget,
                        false => Lit::Dark,
                    };
                    StoneView { pos: n, label: None, lit }
                })
                .collect();
            let revealed = s.win_clock().is_some() && !pearl_leaving;
            SceneModel {
                stones,
                pearl_rock: None,
                pearl_shown: revealed.then_some(r.pearl),
                numbered: false,
                ghost_label: false,
                target: TargetView::Dots { n: r.pearl, ticked: tallied.min(r.pearl) },
                hop_counts: Vec::new(),
                shelly_wears,
            }
        }
        HopStage::SkipCount | HopStage::Hops => {
            let touched = match (s.phase, toss) {
                (HopPhase::Flying, Some(t)) => s.flight().map_or(0, |(hop, _)| hop).min(t.landings.len()),
                (HopPhase::Landed | HopPhase::Won, Some(t)) => t.landings.len(),
                _ => 0,
            };
            let hits: Vec<u16> = toss.map_or(Vec::new(), |t| t.landings[..touched].to_vec());
            let stones = (1..r.pearl)
                .map(|n| StoneView { pos: n, label: Some(n), lit: if hits.contains(&n) { Lit::Touched } else { Lit::Dark } })
                .collect();
            let target = match r.stage {
                HopStage::Hops => {
                    let used = match s.phase {
                        HopPhase::Flying => s.flight().map_or(0, |(hop, _)| hop + 1),
                        HopPhase::Landed | HopPhase::Won => r.hops as usize,
                        HopPhase::Aiming => 0,
                    };
                    TargetView::Hops { k: r.hops, used: used as u8 }
                }
                _ => TargetView::None,
            };
            SceneModel {
                stones,
                pearl_rock: Some(r.pearl),
                pearl_shown: (!pearl_leaving).then_some(r.pearl),
                numbered: true,
                ghost_label: true,
                target,
                // (position, number) — they're the same on the stones.
                hop_counts: hits.iter().map(|&p| (p, p)).collect(),
                shelly_wears,
            }
        }
    }
}

// ─── The win's timeline (seconds into `HopSession::win_clock`) ──────

/// The pearl leaves its stone and starts flying to the purse.
pub const WIN_PEARL_LEAVES: f32 = 0.45;
/// ...and lands in the purse; the count ticks up.
pub const WIN_PEARL_ARRIVES: f32 = 1.2;

// ─── Input ──────────────────────────────────────────────

pub enum HopInput {
    Leave,
    Again,
    /// Move the aim one stone (−1 back, +1 further).
    Nudge(i32),
    Toss,
}

/// Taps on the buttons.
pub fn handle_click(mx: f32, my: f32, l: &PearlHopLayout) -> Option<HopInput> {
    match l.frame.hit_at(mx, my)? {
        HopId::Leave => Some(HopInput::Leave),
        HopId::Again => Some(HopInput::Again),
        _ => None,
    }
}

/// Keys: ESC leaves; arrows move the aim a stone at a time; Space tosses (or,
/// once the win has played, goes again).
pub fn handle_key(input: &FrameInput, session: &HopSession) -> Option<HopInput> {
    if input.pressed(KeyCode::Escape) {
        return Some(HopInput::Leave);
    }
    let go = input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter);
    if session.phase == HopPhase::Won {
        return (go && session.win_done()).then_some(HopInput::Again);
    }
    if input.pressed(KeyCode::Right) || input.pressed(KeyCode::D) {
        return Some(HopInput::Nudge(1));
    }
    if input.pressed(KeyCode::Left) || input.pressed(KeyCode::A) {
        return Some(HopInput::Nudge(-1));
    }
    go.then_some(HopInput::Toss)
}

// ─── Drawing ────────────────────────────────────────────

/// Render-only extras the game hands the painter: the live pull, and the
/// demo's teaching hand.
#[derive(Default, Clone, Copy)]
pub struct HopArt {
    /// Pointer position while Shelly is held.
    pub pull_to: Option<(f32, f32)>,
    /// The demo's hand: position, and whether it's pressing.
    pub hand: Option<(f32, f32, bool)>,
    /// 0..1 fade for the hand.
    pub hand_alpha: f32,
}

const SKY_TOP: Color = Color::new(0.55, 0.86, 0.93, 1.0);
const SKY_BOTTOM: Color = Color::new(0.80, 0.95, 0.96, 1.0);
const WATER_TOP: Color = Color::new(0.10, 0.55, 0.70, 1.0);
const WATER_DEEP: Color = Color::new(0.04, 0.22, 0.38, 1.0);
const GOLD: Color = Color::new(1.0, 0.835, 0.310, 1.0);
const INK: Color = Color::new(0.10, 0.13, 0.20, 1.0);
const PEARL: Color = Color::new(0.95, 0.97, 1.0, 1.0);
const ROCK: Color = Color::new(0.47, 0.42, 0.38, 1.0);
const ROCK_LIGHT: Color = Color::new(0.62, 0.57, 0.50, 1.0);
const STONE: Color = Color::new(0.56, 0.60, 0.58, 1.0);
const STONE_LIT: Color = Color::new(1.0, 0.93, 0.70, 1.0);
/// Counted out past the target: a different glow, so "too far" is visible.
const STONE_PAST: Color = Color::new(1.0, 0.62, 0.55, 1.0);
/// An uncounted target dot.
const DOT: Color = Color::new(0.20, 0.45, 0.70, 1.0);
const BTN: Color = Color::new(0.10, 0.36, 0.55, 1.0);

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a)
}

/// How Shelly looks this frame.
#[derive(Clone, Copy)]
struct Pose {
    x: f32,
    y: f32,
    /// Squash: >1 wide/flat, <1 tall/thin.
    squash: f32,
    /// Degrees, clockwise.
    spin: f32,
    /// Extra size on top of her radius: 1.3 while she's held ("in hand").
    lift: f32,
    face: ClamFace,
    /// Half under water.
    wet: bool,
}

impl Pose {
    fn at(x: f32, y: f32) -> Pose {
        Pose { x, y, squash: 1.0, spin: 0.0, lift: 1.0, face: ClamFace { mouth: 0.15, eyes: 0.0, flail: false }, wet: false }
    }
}

pub fn draw(view: &HopView, l: &PearlHopLayout, art: &HopArt, time: f32) {
    let f = &l.frame;
    let g = &l.scene;
    let s = view.session;

    // The whole screen is the lagoon: sky over water, so the buttons sit on
    // the scene rather than on a dark modal.
    paint::vgradient(f.bounds, SKY_TOP, SKY_BOTTOM, 12);
    draw_water(g, f.bounds, time);

    let m = scene_model(view);
    let c = paint::canvas(g.rect);
    draw_path(&c, g, s, &m, time);
    draw_ghost(&c, g, s, &m, art, time);
    draw_counts(&c, g, &m);
    draw_target(&c, g, &m, time);
    let pose = shelly_pose(g, s, art, time);
    draw_splash(&c, g, s);
    // Shelly, the band and her "in hand" ring can leave the scene while she's
    // pulled, so they paint on the whole screen (ADR-004).
    let screen = paint::canvas(f.bounds);
    draw_band(&screen, g, s, art, pose);
    draw_win_burst(&c, g, s, time);
    draw_shelly(&screen, g, pose, &view.outfit, art, time);
    draw_won_pearl(&screen, g, s, &m, time);
    if let Some((hx, hy, pressed)) = art.hand {
        draw_hand(&screen, hx, hy, pressed, art.hand_alpha);
    }

    for el in f.elements() {
        let Some(id) = el.id else { continue };
        match (&el.kind, id) {
            (Kind::Text(t), HopId::Caption) => {
                paint::round_rect(el.rect, 14.0, Color::new(0.93, 0.97, 0.98, 1.0));
                paint::text(t, INK)
            }
            (Kind::Text(t), HopId::Count) => paint::text(t, INK),
            (Kind::Text(t), _) => paint::text(t, WHITE),
            (_, HopId::Leave) => paint::round_rect(el.rect, 10.0, Color::new(0.26, 0.35, 0.42, 0.92)),
            (_, HopId::Again) => {
                // A breathing gold rim round a rounded button.
                let pulse = (time * 3.0).sin() * 0.5 + 0.5;
                paint::round_rect(el.rect.expand(2.0 + pulse * 2.0), 16.0, GOLD);
                paint::round_rect(el.rect, 14.0, BTN);
            }
            (_, HopId::PearlIcon) => {
                // The purse swells as the won pearl drops in.
                let pop = s.win_clock().map_or(0.0, |c| {
                    let u = (c - WIN_PEARL_ARRIVES) / 0.35;
                    if (0.0..1.0).contains(&u) { (u * std::f32::consts::PI).sin() } else { 0.0 }
                });
                draw_pearl(&paint::canvas(el.rect.expand(8.0)), el.rect.center(), el.rect.w * (0.42 + 0.2 * pop), time)
            }
            (_, HopId::AgainIcon) => draw_pearl(&paint::canvas(el.rect), el.rect.center(), el.rect.w * 0.42, time),
            (_, HopId::LeaveIcon) => draw_back_arrow(&paint::canvas(el.rect), el.rect),
            _ => {}
        }
    }
    draw_pearl_flight(f, g, s, time);
}

/// Water from the surface line to the bottom of the screen, a sandy floor,
/// and rising bubbles. Full width, so the buttons below the scene sit in it.
fn draw_water(g: &SceneGeom, bounds: UiRect, time: f32) {
    let c = paint::canvas(bounds);
    let top = g.surface_y;
    let bottom = bounds.bottom();
    let bands = 10;
    let h = (bottom - top) / bands as f32;
    for i in 0..bands {
        let t = i as f32 / (bands - 1) as f32;
        let col = Color::new(
            lerp(WATER_TOP.r, WATER_DEEP.r, t),
            lerp(WATER_TOP.g, WATER_DEEP.g, t),
            lerp(WATER_TOP.b, WATER_DEEP.b, t),
            1.0,
        );
        let y = top + h * i as f32;
        // Thick lines make seamless bands without hand-made rects.
        let hh = (h * 0.5 + 1.0).min(bottom - y);
        c.line(bounds.x, y + hh, bounds.right(), y + hh, hh * 2.0, col);
    }
    let mut x = bounds.x;
    let mut i = 0;
    while x < bounds.right() {
        let wob = (time * 2.0 + i as f32 * 0.9).sin() * 2.0;
        c.line(x, top + wob, (x + 18.0).min(bounds.right()), top - wob, 2.0, Color::new(1.0, 1.0, 1.0, 0.55));
        x += 26.0;
        i += 1;
    }
    let floor = bottom - (bottom - top) * 0.18;
    let mut x = bounds.x;
    while x < bounds.right() + 20.0 {
        c.circle(x.min(bounds.right() - 1.0), bottom + 8.0, (bottom - floor).max(4.0), Color::new(0.86, 0.76, 0.52, 0.55));
        x += 38.0;
    }
    for k in 0..7 {
        let bx = bounds.x + bounds.w * ((k as f32 * 0.137 + 0.07) % 1.0);
        let t = (time * 0.25 + k as f32 * 0.31) % 1.0;
        let by = lerp(bottom - 6.0, top + 6.0, t);
        c.circle_lines(bx, by, 2.5 + k as f32 % 3.0, 1.2, Color::new(1.0, 1.0, 1.0, 0.45 * (1.0 - t)));
    }
}

/// The start rock, the stepping stones and (where the stage names it) the
/// pearl rock. A long path keeps a base-ten rhythm: every fifth stone a bit
/// bigger, a little post at every ten, so big numbers stay countable.
fn draw_path(c: &paint::Canvas, g: &SceneGeom, s: &HopSession, m: &SceneModel, time: f32) {
    let (x0, rock_top) = (g.x0, g.rock_top());
    // Narrow enough on a crowded path that stone 1 stays in the clear.
    let rock_w = (g.unit - g.stone_r - 4.0).clamp(g.r * 1.05, g.r * 1.45);
    c.ellipse(x0, g.surface_y + g.r * 0.2, rock_w, g.r * 1.3, 0.0, ROCK);
    c.ellipse(x0 - rock_w * 0.2, rock_top + g.r * 0.35, rock_w * 0.62, g.r * 0.45, 0.0, ROCK_LIGHT);
    let label = (g.unit * 0.5).clamp(12.0, 22.0) as u16;
    let label_y = g.surface_y + g.r * 1.2 + label as f32;
    if m.numbered {
        c.text_centered("0", x0, label_y, label, WHITE);
    }

    // A long path (past 20) is grouped: a bigger stone every five, a post
    // every ten, and only the fives (or, when crowded, the tens) written —
    // so 48 stays countable as four tens and eight.
    let long = s.round.span > 20;
    let every = if !long && g.unit >= 18.0 { 1 } else if g.unit * 5.0 >= 28.0 { 5 } else { 10 };
    let grouped = long;
    for st in &m.stones {
        let x = g.x_of(st.pos);
        let bob = (time * 1.3 + st.pos as f32).sin() * 0.8;
        let (fill, glow) = match st.lit {
            Lit::Dark => (STONE, false),
            Lit::Counted => (STONE_LIT, true),
            Lit::PastTarget => (STONE_PAST, true),
            Lit::Touched => (STONE_LIT, false),
        };
        let big = if grouped && st.pos % 5 == 0 { 1.35 } else { 1.0 };
        let sr = g.stone_r * big;
        if glow {
            c.circle(x, g.surface_y + bob - sr * 0.1, sr * 1.4, with_alpha(fill, 0.4));
        }
        if grouped && st.pos % 10 == 0 {
            // A ten-post: a little marker pole standing behind the stone.
            c.line(x, g.surface_y - g.r * 1.1, x, g.surface_y, 2.0, Color::new(1.0, 1.0, 1.0, 0.75));
            c.circle(x, g.surface_y - g.r * 1.1, 3.0, Color::new(1.0, 1.0, 1.0, 0.9));
        }
        c.ellipse(x, g.surface_y + bob, sr, sr * 0.75, 0.0, fill);
        c.ellipse(x - sr * 0.25, g.surface_y - sr * 0.65 + bob, sr * 0.55, sr * 0.2, 0.0, Color::new(1.0, 1.0, 1.0, 0.35));
        // No number squeezed under the pearl rock's shoulder.
        let under_rock = m.pearl_rock.is_some_and(|pr| g.x_of(pr) - x < pearl_rock_w(g) + label as f32);
        if let Some(n) = st.label.filter(|n| n % every == 0 && !under_rock) {
            c.text_centered(&n.to_string(), x, g.surface_y + g.stone_r + label as f32 + 2.0, label, WHITE);
        }
    }

    if let Some(pr) = m.pearl_rock {
        // The pearl rock, numbered in gold: at these stages the target is
        // given; the maths is choosing the hop that reaches it.
        let xp = g.x_of(pr);
        // Never so wide it swallows the stone before it.
        let rw = pearl_rock_w(g);
        c.ellipse(xp, g.surface_y + g.r * 0.1, rw, g.r * 0.85, 0.0, ROCK);
        c.ellipse(xp - rw * 0.2, g.surface_y - g.r * 0.5, rw * 0.6, g.r * 0.25, 0.0, ROCK_LIGHT);
        c.text_centered(&pr.to_string(), xp, label_y, label.max(16), GOLD);
    }
    if let (Some(pp), None) = (m.pearl_shown, s.win_clock()) {
        draw_pearl(c, (g.x_of(pp), g.surface_y - g.r * 0.95), g.r * 0.42, time);
    }
}

/// Half-width of the pearl rock: never so wide it swallows the stones before
/// it, never so thin it reads as a post.
fn pearl_rock_w(g: &SceneGeom) -> f32 {
    (g.unit * 1.3).max(g.r * 0.7).min(g.r * 1.05)
}

/// Where the won pearl hangs while it glitters, before it flies to the purse:
/// popped up beside Shelly's flip, not behind her.
fn won_pearl_at(g: &SceneGeom, s: &HopSession) -> (f32, f32) {
    let end = s.toss.as_ref().map_or(0, |t| t.end());
    let k = s.win_clock().unwrap_or(0.0);
    let u = ease(k / 0.3);
    (g.x_of(end) + g.r * 1.6 * u, g.surface_y - g.r * (0.9 + 1.9 * u))
}

/// The revealed pearl, big and on top of everything in the scene.
fn draw_won_pearl(c: &paint::Canvas, g: &SceneGeom, s: &HopSession, m: &SceneModel, time: f32) {
    if m.pearl_shown.is_none() {
        return;
    }
    let Some(k) = s.win_clock() else { return };
    let grow = 1.0 + 1.1 * ease(k / 0.3);
    draw_pearl(c, won_pearl_at(g, s), g.r * 0.42 * grow, time);
}

/// The dotted arc of the first hop the current aim would make, with the
/// stone it lands on ringed — it snaps stone to stone as the kid pulls.
fn draw_ghost(c: &paint::Canvas, g: &SceneGeom, s: &HopSession, m: &SceneModel, art: &HopArt, time: f32) {
    if s.phase != HopPhase::Aiming {
        return;
    }
    let pulling = art.pull_to.is_some_and(|p| g.aim_for_pointer(p).is_some());
    let keyboard_aimed = s.aim != s.round.min_aim || s.toss.is_some();
    if !pulling && !keyboard_aimed {
        return;
    }
    let (hx, hy) = g.home();
    let first = s.round.landings(s.aim).first().copied().unwrap_or(s.aim);
    let x1 = g.x_of(first);
    let water = first > s.round.last_stone();
    let (_, y1) = g.perch(first, water);
    let h = arc_height(g, hx, x1);
    let dots = 14;
    for i in 1..dots {
        let t = i as f32 / dots as f32;
        let x = lerp(hx, x1, t);
        let y = lerp(hy, y1, t) - 4.0 * h * t * (1.0 - t);
        let a = 0.35 + 0.4 * ((time * 6.0 - t * 8.0).sin() * 0.5 + 0.5);
        c.circle(x, y, 3.0, Color::new(1.0, 1.0, 1.0, a));
    }
    let pulse = (time * 5.0).sin() * 0.5 + 0.5;
    c.circle_lines(x1, g.surface_y - g.stone_r * 0.1, (g.stone_r * 1.15).max(8.0) * (1.0 + 0.08 * pulse), 3.0, with_alpha(GOLD, 0.95));
    // At the skip-counting stages the hop's size rides on the ghost landing —
    // the number Shelly is saying. Not at the counting stage: there the kid
    // counts the stones, and nothing reads them out.
    if m.ghost_label {
        let size = (g.r * 1.1) as u16;
        c.circle(x1, y1 - g.r * 1.9, g.r * 0.85, Color::new(1.0, 1.0, 1.0, 0.85));
        c.text_centered(&first.to_string(), x1, y1 - g.r * 1.9 + size as f32 * 0.36, size.max(14), INK);
    }
}

fn arc_height(g: &SceneGeom, xa: f32, xb: f32) -> f32 {
    let room = (g.home().1 - g.rect.y - g.r * 1.6).max(10.0);
    ((xb - xa).abs() * 0.45).clamp(30.0, 240.0).min(room)
}

/// Skip counting out loud: every touch-down gets its number popped up over it
/// (6… 12… 18… 24) — the same at the "in X hops" stage, so dividing and
/// skip counting look like one thing.
fn draw_counts(c: &paint::Canvas, g: &SceneGeom, m: &SceneModel) {
    let size = (g.r * 0.95).clamp(14.0, 26.0) as u16;
    for (i, &(pos, n)) in m.hop_counts.iter().enumerate() {
        let x = g.x_of(pos);
        let y = g.surface_y - g.r * 2.7 - if i % 2 == 1 { g.r * 0.7 } else { 0.0 };
        c.circle(x, y - size as f32 * 0.35, size as f32 * 0.8, Color::new(1.0, 1.0, 1.0, 0.85));
        c.text_centered(&n.to_string(), x, y, size, INK);
    }
}

/// Shelly's bubble over the start rock: the target as dots (counting) or as
/// K hop-bubbles.
fn draw_target(c: &paint::Canvas, g: &SceneGeom, m: &SceneModel, time: f32) {
    let (hx, hy) = g.home();
    let bob = (time * 1.8).sin() * 2.0;
    match m.target {
        TargetView::None => {}
        TargetView::Hops { k, used } => {
            let k = k as usize;
            let br = (g.r * 0.5).max(10.0);
            let gap = br * 2.5;
            let y = hy - g.r * 2.4;
            let x_start = (hx - (k as f32 - 1.0) * gap / 2.0).max(g.rect.x + br + 2.0);
            for i in 0..k {
                let x = x_start + i as f32 * gap;
                let bob = (time * 2.2 + i as f32).sin() * 2.0;
                if i < used as usize {
                    c.circle_lines(x, y + bob, br * 0.5, 1.5, Color::new(1.0, 1.0, 1.0, 0.35));
                } else {
                    c.circle(x, y + bob, br, Color::new(0.75, 0.93, 1.0, 0.95));
                    c.circle_lines(x, y + bob, br, 2.5, Color::new(0.10, 0.35, 0.55, 0.9));
                    c.circle(x - br * 0.35, y + bob - br * 0.35, br * 0.25, WHITE);
                }
            }
        }
        TargetView::Dots { n, ticked } => {
            // Dots like a die face: one row up to three, two rows past that.
            let dr = (g.r * 0.26).max(6.0);
            let rows = if n > 3 { 2.0 } else { 1.0 };
            let gap = dr * 2.7;
            let br = (n.min(3) as f32 * gap * 0.5 + dr * 1.4).max(g.r * 1.1);
            let (bx, by) = (hx + g.r * 0.9 + br, hy - g.r * 1.7 - br * 0.6 + bob);
            c.circle(hx + g.r * 0.7, hy - g.r * 1.2 + bob, (br * 0.16).max(3.0), Color::new(1.0, 1.0, 1.0, 0.9));
            c.circle(hx + g.r * 1.0, hy - g.r * 1.55 + bob, (br * 0.24).max(4.0), Color::new(1.0, 1.0, 1.0, 0.9));
            c.circle(bx, by, br, WHITE);
            c.circle_lines(bx, by, br, 2.5, GOLD);
            for i in 0..n {
                let (row, col) = if n > 3 { (i / 3, i % 3) } else { (0, i) };
                let in_row = if n > 3 && row == 1 { n - 3 } else { n.min(3) };
                let x = bx + (col as f32 - (in_row as f32 - 1.0) / 2.0) * gap;
                let y = by + (row as f32 - (rows - 1.0) / 2.0) * gap;
                if i < ticked {
                    // Counted off: a gold dot with a ring.
                    c.circle(x, y, dr * 1.15, GOLD);
                    c.circle_lines(x, y, dr * 1.15, 2.0, Color::new(0.65, 0.45, 0.05, 1.0));
                } else {
                    c.circle(x, y, dr, DOT);
                }
            }
        }
    }
}

/// Where Shelly is and what she's doing this frame. All of it is a function
/// of the session's phase clock (and the live pull) — nothing here decides
/// anything.
fn shelly_pose(g: &SceneGeom, s: &HopSession, art: &HopArt, time: f32) -> Pose {
    let (hx, hy) = g.home();
    match s.phase {
        HopPhase::Aiming => {
            let mut p = Pose::at(hx, hy + (time * 2.0).sin() * 1.2);
            if let Some(ptr) = art.pull_to {
                // Held: lifted (bigger, ringed), and on a slingshot pull she
                // follows the pointer a little and squishes, eyes squeezed.
                p.lift = 1.3;
                let (dx, dy) = (ptr.0 - hx, ptr.1 - hy);
                let d = (dx * dx + dy * dy).sqrt().max(0.001);
                let behind = ptr.0 <= hx + g.reach();
                if behind {
                    let give = d.min(g.r * 1.6);
                    let t = (d / g.max_pull).clamp(0.0, 1.0);
                    p.x += dx / d * give;
                    p.y += dy / d * give;
                    p.squash = 1.0 + 0.45 * t;
                    p.spin = -12.0 * t;
                    p.face.eyes = if t > 0.15 { -1.0 } else { 0.0 };
                    p.face.mouth = 0.05;
                }
            }
            p
        }
        HopPhase::Flying => {
            let t = s.toss.as_ref().unwrap();
            let (hop, u) = s.flight().unwrap_or((0, 0.0));
            let from = if hop == 0 { 0 } else { t.landings[hop - 1] };
            let to = t.landings[hop];
            let (xa, ya) = if hop == 0 { g.home() } else { g.perch(from, false) };
            let (xb, yb) = g.perch(to, to > s.round.last_stone());
            let h = arc_height(g, xa, xb);
            let mut p = Pose::at(lerp(xa, xb, u), lerp(ya, yb, u) - 4.0 * h * u * (1.0 - u));
            // Stretch on take-off, spin through the air, flail the whole way.
            p.squash = if u < 0.15 { 0.7 } else { 1.0 };
            p.spin = u * 360.0;
            p.face = ClamFace { mouth: 0.7 + 0.3 * (time * 18.0).sin().abs(), eyes: 1.0, flail: true };
            p
        }
        HopPhase::Landed => landed_pose(g, s, time),
        HopPhase::Won => {
            let end = s.toss.as_ref().map_or(0, |t| t.end());
            let (x, y) = g.perch(end, false);
            let mut p = Pose::at(x, y);
            let Some(c) = s.win_clock() else {
                // Still counting the stones out: sitting tight on her stone.
                p.squash = if s.clock < 0.2 { 1.4 } else { 1.0 };
                return p;
            };
            if c < 0.15 {
                p.squash = 1.5 - c * 3.0; // crouch...
            } else if c < 0.95 {
                // ...and a big happy backflip, mouth wide open.
                let u = (c - 0.15) / 0.8;
                p.y -= (u * std::f32::consts::PI).sin() * g.r * 3.2;
                p.spin = -360.0 * ease(u);
                p.face = ClamFace { mouth: 1.0, eyes: 1.0, flail: true };
            } else {
                // Stuck the landing: a few bouncy cheers.
                let u = c - 0.95;
                let fade = 1.0 - (u / 0.9).min(1.0);
                p.y -= (u * 9.0).sin().abs() * g.r * 0.5 * fade;
                p.squash = 1.0 + 0.15 * (u * 9.0).cos().abs() * fade;
                p.face.mouth = 1.0;
            }
            p
        }
    }
}

fn landed_pose(g: &SceneGeom, s: &HopSession, time: f32) -> Pose {
    let t = s.toss.as_ref().unwrap();
    let end = t.end();
    let water = s.lands_in_water();
    // Counting stage: she sits where she landed while the stones are counted.
    let c = s.clock - s.tally_secs();
    if c < 0.0 {
        let (lx, ly) = g.perch(end, water);
        let mut p = Pose::at(lx, ly + if water { (time * 4.0).sin() * 2.0 } else { 0.0 });
        p.wet = water;
        p.squash = if s.clock < 0.2 { 1.4 } else { 1.0 };
        return p;
    }
    if water {
        // SPLASH. Bob up, paddle home, hop up.
        let (lx, ly) = g.perch(end, true);
        let (hx, hy) = g.home();
        let paddle_end = SPLASH_SECS - 0.5;
        if c < 0.4 {
            let mut p = Pose::at(lx, ly + (0.4 - c) * g.r * 2.0);
            p.wet = true;
            p.face.eyes = 1.0;
            p.face.mouth = 0.9;
            return p;
        }
        if c < paddle_end {
            let u = ease((c - 0.4) / (paddle_end - 0.4));
            let mut p = Pose::at(lerp(lx, hx, u), ly + (time * 9.0).sin() * 2.0);
            p.wet = true;
            p.spin = (time * 9.0).sin() * 8.0;
            p.face.flail = true;
            p.face.mouth = 0.3;
            return p;
        }
        let u = ((c - paddle_end) / 0.5).clamp(0.0, 1.0);
        let mut p = Pose::at(hx, lerp(ly, hy, u) - 4.0 * g.r * 1.2 * u * (1.0 - u));
        p.squash = if u > 0.9 { 1.3 } else { 1.0 };
        return p;
    }
    // A plain stone: bonk, wobble, shrug, glide home.
    let (lx, ly) = g.perch(end, false);
    let mut p = Pose::at(lx, ly);
    if c < 0.3 {
        p.squash = 1.6 - c;
        p.face.eyes = -1.0;
    } else if c < 0.95 {
        let decay = 1.0 - (c - 0.3) / 0.65;
        p.spin = (c * 24.0).sin() * 28.0 * decay;
    } else if c < 1.35 {
        // Shrug: tilt one way, then the other, mouth a little "hm".
        p.spin = ((c - 0.95) * 16.0).sin() * 12.0;
        p.face.mouth = 0.25;
    } else {
        let u = ((c - 1.35) / 0.45).clamp(0.0, 1.0);
        let (hx, hy) = g.home();
        let h = arc_height(g, lx, hx) * 0.6;
        p.x = lerp(lx, hx, u);
        p.y = lerp(ly, hy, u) - 4.0 * h * u * (1.0 - u);
        p.spin = -u * 360.0;
    }
    p
}

/// The rubber band: two strands from the rock's posts to Shelly while she's
/// pulled back.
fn draw_band(c: &paint::Canvas, g: &SceneGeom, s: &HopSession, art: &HopArt, pose: Pose) {
    if s.phase != HopPhase::Aiming {
        return;
    }
    let (hx, hy) = g.home();
    let posts = [(hx - g.r * 1.1, hy + g.r * 0.1), (hx + g.r * 1.1, hy + g.r * 0.1)];
    for &(px, py) in &posts {
        c.line(px, py - g.r * 0.6, px, g.rock_top() + g.r * 0.4, 4.0, Color::new(0.45, 0.30, 0.18, 1.0));
    }
    let taut = art.pull_to.is_some();
    for &(px, py) in &posts {
        let (tx, ty) = if taut { (pose.x, pose.y + g.r * 0.4 / pose.squash.max(0.3)) } else { (hx, hy + g.r * 0.55) };
        c.line(px, py - g.r * 0.6, tx, ty, if taut { 3.0 } else { 2.0 }, Color::new(0.85, 0.30, 0.25, 0.95));
    }
}

/// A splash of droplets where she hit the water.
fn draw_splash(c: &paint::Canvas, g: &SceneGeom, s: &HopSession) {
    if s.phase != HopPhase::Landed || !s.lands_in_water() {
        return;
    }
    let Some(t) = s.toss.as_ref() else { return };
    let k = s.clock;
    if k > 0.9 {
        return;
    }
    let x = g.x_of(t.end());
    let y = g.surface_y;
    let fade = 1.0 - k / 0.9;
    for i in 0..9 {
        let a = std::f32::consts::PI * (0.12 + 0.76 * i as f32 / 8.0);
        let v = g.r * (2.2 + (i % 3) as f32 * 0.6);
        let dx = -a.cos() * v * k * 1.4;
        let dy = -a.sin() * v * k * 2.2 + 9.0 * g.r * k * k;
        c.circle(x + dx, (y + dy).min(y), 3.0 + (i % 2) as f32 * 2.0, Color::new(0.85, 0.96, 1.0, 0.9 * fade));
    }
    c.circle_lines(x, y, g.r * (0.6 + k * 2.5), 2.5, Color::new(1.0, 1.0, 1.0, 0.7 * fade));
}

/// Shelly, wearing her swag, through the shared dressed-character helper:
/// the squish, spin and flip move her crown with her.
fn draw_shelly(c: &paint::Canvas, g: &SceneGeom, p: Pose, outfit: &ShellyOutfit, art: &HopArt, time: f32) {
    let r = g.r * p.lift;
    if art.pull_to.is_some() {
        // In hand: a soft shadow below and a pulsing ring, so she reads as
        // picked up even when the platform reports no motion.
        let pulse = (time * 6.0).sin() * 0.5 + 0.5;
        c.ellipse(p.x, g.rock_top() + g.r * 0.2, r * 0.9, r * 0.2, 0.0, Color::new(0.0, 0.0, 0.0, 0.22));
        c.circle_lines(p.x, p.y, r * (1.25 + 0.1 * pulse), 3.0, with_alpha(GOLD, 0.8));
    } else if !p.face.flail && !p.wet {
        c.ellipse(p.x, p.y + g.r * 0.8, g.r * 0.9 * p.squash, g.r * 0.18, 0.0, Color::new(0.0, 0.0, 0.0, 0.18));
    }
    // The cartoon clam is drawn at radius 14 in tile space.
    let posture = Posture { x: p.x, y: p.y, scale: r / 14.0, squash: p.squash, spin: p.spin };
    let fit = SwagFit::CLAM_CARTOON;
    dressed::draw_dressed(posture, Body::Clam(p.face), Dir::Down, &Outfit { worn: outfit.worn, color: outfit.color, fit }, time);
    if p.wet {
        // The water line cuts across her: a band of water over her bottom half.
        let wl = g.surface_y;
        c.ellipse(p.x, wl + g.r * 0.35, g.r * 1.3, g.r * 0.45, 0.0, Color::new(0.10, 0.55, 0.70, 0.85));
        c.line(p.x - g.r * 1.3, wl, p.x + g.r * 1.3, wl, 2.0, Color::new(1.0, 1.0, 1.0, 0.6));
    }
}

/// The win's burst where the pearl pops: a flash, a ring, and rays of glint.
fn draw_win_burst(c: &paint::Canvas, g: &SceneGeom, s: &HopSession, time: f32) {
    let Some(k) = s.win_clock() else { return };
    let (x, y) = won_pearl_at(g, s);
    if k < 0.6 {
        // A white flash and a gold shock ring.
        let u = k / 0.6;
        c.circle(x, y, g.r * (1.2 + 2.6 * u), Color::new(1.0, 1.0, 0.92, 0.75 * (1.0 - u)));
        c.circle_lines(x, y, g.r * (1.0 + 4.5 * u), 7.0 * (1.0 - u) + 2.0, with_alpha(GOLD, 1.0 - u));
    }
    if k < 1.4 {
        // Glint rays, turning, long and short, gold with a white core.
        let fade = 1.0 - (k / 1.4);
        let reach = 0.6 + ease(k / 0.25) * 0.7;
        for i in 0..12 {
            let a = i as f32 / 12.0 * std::f32::consts::TAU + time * 1.2;
            let (r0, r1) = (g.r * 1.2, g.r * (2.2 + 1.6 * ((i % 2) as f32)) * reach);
            let (cs, sn) = (a.cos(), a.sin());
            c.line(x + cs * r0, y + sn * r0, x + cs * r1, y + sn * r1, 6.0, with_alpha(GOLD, fade));
            c.line(x + cs * r0, y + sn * r0, x + cs * r1, y + sn * r1, 2.0, Color::new(1.0, 1.0, 1.0, fade));
        }
        // Confetti bubbles bursting outward and drifting down.
        let colors = [GOLD, Color::new(1.0, 0.45, 0.65, 1.0), Color::new(0.45, 0.85, 1.0, 1.0), Color::new(0.6, 0.95, 0.5, 1.0)];
        for i in 0..18 {
            let a = i as f32 * 2.399; // golden angle: an even spray
            let v = g.r * (2.2 + (i % 4) as f32 * 0.7);
            let (px, py) = (x + a.cos() * v * ease(k / 0.5), y + a.sin() * v * ease(k / 0.5) + g.r * 1.5 * k * k);
            c.circle(px, py, 3.0 + (i % 3) as f32, with_alpha(colors[i % 4], fade));
        }
    }
}

/// The won pearl flies from the stone up into the purse in the top row,
/// trailing sparkles. Drawn over the whole screen, above the buttons.
fn draw_pearl_flight(f: &Frame<HopId>, g: &SceneGeom, s: &HopSession, time: f32) {
    let Some(k) = s.win_clock() else { return };
    if !(WIN_PEARL_LEAVES..WIN_PEARL_ARRIVES).contains(&k) {
        return;
    }
    let Some(purse) = f.rect(HopId::PearlIcon) else { return };
    let from = won_pearl_at(g, s);
    let to = purse.center();
    let u = ease((k - WIN_PEARL_LEAVES) / (WIN_PEARL_ARRIVES - WIN_PEARL_LEAVES));
    let arc = g.r * 3.0;
    let at = |u: f32| (lerp(from.0, to.0, u), lerp(from.1, to.1, u) - 4.0 * arc * u * (1.0 - u));
    let c = paint::canvas(f.bounds);
    for i in 1..6 {
        let (tx, ty) = at((u - i as f32 * 0.04).max(0.0));
        c.circle(tx, ty, (g.r * 0.35 - i as f32 * 1.5).max(2.0), with_alpha(GOLD, 0.7 - i as f32 * 0.1));
    }
    let (px, py) = at(u);
    draw_pearl(&c, (px, py), g.r * lerp(0.75, 0.45, u), time);
}

/// A pearl with a highlight: on the pearl rock, in the purse, on Again!.
fn draw_pearl(c: &paint::Canvas, (x, y): (f32, f32), r: f32, time: f32) {
    let glint = (time * 3.0).sin() * 0.5 + 0.5;
    c.circle(x, y, r * (1.0 + 0.15 * glint), Color::new(1.0, 0.95, 0.75, 0.35));
    c.circle(x, y, r, PEARL);
    c.circle_lines(x, y, r, 1.2, Color::new(0.70, 0.74, 0.85, 1.0));
    c.circle(x - r * 0.35, y - r * 0.35, r * 0.3, WHITE);
}

fn draw_back_arrow(c: &paint::Canvas, r: UiRect) {
    let (cx, cy) = r.center();
    let s = r.w * 0.4;
    c.triangle((cx - s, cy), (cx - s * 0.1, cy - s * 0.8), (cx - s * 0.1, cy + s * 0.8), WHITE);
    c.line(cx - s * 0.2, cy, cx + s, cy, s * 0.55, WHITE);
}

/// A cartoon hand: palm, pointing finger, a press ring when it's pressing.
fn draw_hand(c: &paint::Canvas, x: f32, y: f32, pressed: bool, alpha: f32) {
    let skin = Color::new(1.0, 0.86, 0.72, alpha);
    let edge = Color::new(0.45, 0.30, 0.22, alpha);
    // The fingertip is at (x, y); the palm trails below-right.
    let (px, py) = (x + 18.0, y + 32.0);
    c.circle(px, py, 19.0, edge);
    c.circle(px, py, 17.0, skin);
    c.line(x, y, px - 4.0, py - 12.0, 14.0, edge);
    c.line(x, y, px - 4.0, py - 12.0, 11.0, skin);
    c.circle(x, y, 6.5, skin);
    c.circle_lines(x, y, 6.5, 2.0, edge);
    for i in 0..3 {
        c.circle(px + 6.0 + i as f32 * 5.0, py - 14.0 + i as f32 * 3.0, 5.5, skin);
        c.circle_lines(px + 6.0 + i as f32 * 5.0, py - 14.0 + i as f32 * 3.0, 5.5, 1.5, edge);
    }
    if pressed {
        c.circle_lines(x, y, 15.0, 3.0, Color::new(1.0, 1.0, 1.0, 0.85 * alpha));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::rand::rngs::SmallRng;
    use ::rand::SeedableRng;
    use robot_buddy_domain::logic::pearl_hop::{generate_round, hop_reducer, HopAction, Landing};

    const NOTHING: &BTreeSet<String> = &BTreeSet::new();

    fn view(s: &HopSession) -> HopView<'_> {
        HopView { session: s, pearls: 3, caption: "Find my pearl!", outfit: ShellyOutfit { worn: NOTHING, color: "red" } }
    }

    fn geom(band: u8, screen: (f32, f32)) -> (HopSession, PearlHopLayout) {
        let s = HopSession::new(generate_round(band, &mut SmallRng::seed_from_u64(4)));
        let l = layout(&view(&s), screen);
        (s, l)
    }

    #[test]
    fn dragging_to_the_drag_point_sets_exactly_that_aim() {
        for band in [1u8, 2, 3, 4, 6] {
            for &screen in &layout::SWEEP_SCREENS {
                let (s, l) = geom(band, screen);
                for aim in s.round.min_aim..=s.round.max_aim {
                    let p = l.scene.drag_point_for(aim);
                    assert_eq!(l.scene.aim_for_pointer(p), Some(aim), "band {band} {screen:?} aim {aim}");
                }
            }
        }
    }

    #[test]
    fn pointing_at_a_stone_aims_the_first_hop_there() {
        for band in [1u8, 3, 6] {
            for &screen in &layout::SWEEP_SCREENS {
                let (s, l) = geom(band, screen);
                let g = &l.scene;
                for aim in s.round.min_aim..=s.round.max_aim {
                    let p = g.tap_point_for(aim);
                    if p.0 > g.home().0 + g.reach() && p.0 < g.rect.right() {
                        assert_eq!(g.aim_for_pointer(p), Some(aim), "band {band} {screen:?} aim {aim}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_tap_on_shelly_is_not_a_toss() {
        let (_, l) = geom(1, (960.0, 720.0));
        let (hx, hy) = l.scene.home();
        assert!(l.scene.grabs(hx, hy));
        assert_eq!(l.scene.aim_for_pointer((hx + 3.0, hy + 2.0)), None);
    }

    #[test]
    fn the_whole_path_and_the_pull_fit_on_screen() {
        for band in [1u8, 2, 3, 4, 6, 9] {
            for &screen in &layout::SWEEP_SCREENS {
                let (s, l) = geom(band, screen);
                let g = &l.scene;
                assert!(g.x_of(s.round.span) <= g.rect.right(), "band {band} {screen:?}");
                let (x, y) = g.drag_point_for(s.round.max_aim);
                assert!(x >= 0.0 && y <= screen.1, "the biggest pull stays on screen: band {band} {screen:?} ({x}, {y})");
            }
        }
    }

    /// The counting stage's whole point: nothing on screen says where the
    /// pearl is until Shelly lands on it. Not a distinct rock, not a pearl,
    /// not a number, not a lit stone.
    #[test]
    fn the_counting_stage_never_shows_the_pearl_before_she_lands_on_it() {
        for seed in 0..30u64 {
            let round = generate_round(1, &mut SmallRng::seed_from_u64(seed));
            let fresh = HopSession::new(round.clone());
            let check = |s: &HopSession, when: &str| {
                let m = scene_model(&view(s));
                assert_eq!(m.pearl_rock, None, "{when}: no pearl rock");
                assert_eq!(m.pearl_shown, None, "{when}: no pearl drawn");
                assert!(!m.numbered && !m.ghost_label, "{when}: no numbers");
                assert!(m.stones.iter().all(|st| st.label.is_none()), "{when}: unnumbered stones");
                assert!(m.stones.last().unwrap().pos > round.pearl, "{when}: the row runs on past the pearl");
                assert_eq!(m.target, TargetView::Dots { n: round.pearl, ticked: 0 }, "{when}: the target is dots");
            };
            check(&fresh, "aiming");
            assert!(scene_model(&view(&fresh)).stones.iter().all(|st| st.lit == Lit::Dark), "every stone looks the same");
            // Aimed right at it, mid-air: still hidden.
            let mut s = hop_reducer(hop_reducer(fresh.clone(), HopAction::Aim { at: round.pearl }), HopAction::Toss);
            s = hop_reducer(s, HopAction::Tick { dt: 0.2 });
            check(&s, "mid-air");
            // A miss, counted out: dots tick, no pearl.
            let short = round.pearl - 1;
            let mut m = hop_reducer(hop_reducer(fresh.clone(), HopAction::Aim { at: short }), HopAction::Toss);
            while m.phase == HopPhase::Flying {
                m = hop_reducer(m, HopAction::Tick { dt: 0.1 });
            }
            assert_eq!(m.toss.as_ref().unwrap().landing, Landing::Short);
            // Let the count finish (and no further: then she shrugs home).
            while m.clock < m.tally_secs() + 0.1 {
                m = hop_reducer(m, HopAction::Tick { dt: 0.05 });
            }
            let model = scene_model(&view(&m));
            assert_eq!(model.pearl_shown, None, "a short toss never reveals it");
            assert_eq!(model.target, TargetView::Dots { n: round.pearl, ticked: short }, "dots left over");
        }
    }

    #[test]
    fn too_far_is_counted_past_the_last_dot() {
        let round = generate_round(1, &mut SmallRng::seed_from_u64(2));
        let mut s = hop_reducer(hop_reducer(HopSession::new(round.clone()), HopAction::Aim { at: round.pearl + 2 }), HopAction::Toss);
        for _ in 0..80 {
            s = hop_reducer(s, HopAction::Tick { dt: 0.05 });
        }
        let m = scene_model(&view(&s));
        let past = m.stones.iter().filter(|st| st.lit == Lit::PastTarget).count();
        assert_eq!(past, 2, "two stones past the dots");
        assert_eq!(m.pearl_shown, None);
    }

    #[test]
    fn x_hops_echo_the_skip_count() {
        let round = generate_round(6, &mut SmallRng::seed_from_u64(9));
        let size = round.pearl / round.hops as u16;
        let mut s = hop_reducer(hop_reducer(HopSession::new(round.clone()), HopAction::Aim { at: size }), HopAction::Toss);
        for _ in 0..400 {
            s = hop_reducer(s, HopAction::Tick { dt: 0.05 });
        }
        let m = scene_model(&view(&s));
        let counts: Vec<u16> = m.hop_counts.iter().map(|&(_, n)| n).collect();
        let want: Vec<u16> = (1..=round.hops as u16).map(|i| i * size).collect();
        assert_eq!(counts, want, "{} in {} hops reads as {size}, {}, …", round.pearl, round.hops, size * 2);
    }
}
