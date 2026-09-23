//! Pearl Hop — Shelly's slingshot minigame. Layout, scene geometry, input
//! mapping and the cartoon; the rules live in
//! `robot_buddy_domain::logic::pearl_hop`.
//!
//! ```text
//!  ┌──────────────────────────────────────────────┐
//!  │ (o) 12                               [Leave] │  top row (layout)
//!  │                                              │
//!  │   Shelly        stepping stones        pearl │  Scene region: custom art,
//!  │   [rock] ~~ o ~~ o ~~ o ~~ o ~~ [rock]  ~~~~ │  painted through a Canvas
//!  │ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ │  bound to the region
//!  │           "My pearl is on stone 5!"          │  caption (for grown-ups)
//!  │                  [ Again! ]                  │  only once she has it
//!  └──────────────────────────────────────────────┘
//! ```
//!
//! The kid can't read, so nothing important is only words: the pearl, the
//! stones' numbers, Shelly's count-aloud and her antics carry the game.
//! [`SceneGeom`] is pure (no macroquad) so `Game::step` and the headless
//! tests map a drag to an aim with the exact numbers the drawing uses.

use crate::prelude::*;

use crate::input::FrameInput;
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

/// What the panel shows this frame.
pub struct HopView<'a> {
    pub session: &'a HopSession,
    /// The kid's pearl purse, shown top-left so a win visibly lands somewhere.
    pub pearls: u32,
    /// Shelly's line, for a grown-up reading along (the kid hears it).
    pub caption: &'a str,
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
    let won = view.session.phase == HopPhase::Won;
    let top = row()
        .h(46.0)
        .fixed()
        .gap(8.0)
        .child(region(26.0, 26.0).id(HopId::PearlIcon).fixed())
        .child(text(view.pearls.to_string(), 24, Fit::shrink(14)).id(HopId::Count))
        .child(spacer())
        .child(icon_button(HopId::Leave, HopId::LeaveIcon, HopId::LeaveLabel, "Leave", 22).size(124.0, 44.0).fixed());
    // Reserved even before the win, so the scene never jumps when the Again
    // button turns up.
    let bottom = row()
        .h(58.0)
        .fixed()
        .justify(Justify::Center)
        .maybe(won.then(|| {
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
                .max_w(680.0)
                .align_self(Align::Center)
                .h(56.0)
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
    /// Screen pixels per sub-unit of the round.
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
    scale: u16,
}

impl SceneGeom {
    pub fn new(rect: UiRect, bounds: UiRect, round: &HopRound) -> Self {
        let r = (rect.h * 0.1).min(rect.w * 0.075).clamp(12.0, 34.0);
        let surface_y = rect.y + rect.h * 0.62;
        let x0 = rect.x + r + 16.0;
        let x_end = rect.right() - r - 10.0;
        let unit = ((x_end - x0) / round.span_pos().max(1) as f32).max(0.01);
        let step = unit * round.scale as f32;
        let stone_r = (step * 0.40).min(r * 0.95).max(3.0);
        let home_y = surface_y - r * 1.65; // == home().1
        // A pull has to fit on screen: Shelly sits at the left edge, so the
        // drag goes down and back, and the room below her caps it.
        let max_pull = ((bounds.bottom() - home_y) * 0.8).min(rect.w * 0.34).clamp(60.0, 200.0);
        let dx = ((x0 - bounds.x - 8.0) / max_pull).clamp(0.0, 0.28);
        let drag_dir = (-dx, (1.0 - dx * dx).sqrt());
        SceneGeom {
            rect, surface_y, x0, unit, r, stone_r, max_pull, drag_dir,
            min_aim: round.min_aim, max_aim: round.max_aim, scale: round.scale,
        }
    }

    /// Screen x of a position (sub-units). Past the right edge it pins to the
    /// edge, so a huge overshoot still splashes on screen.
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

    /// Does a press at (x, y) grab Shelly? Generous: small fingers.
    pub fn grabs(&self, x: f32, y: f32) -> bool {
        let (hx, hy) = self.home();
        let reach = (self.r * 2.2).max(44.0);
        (x - hx).powi(2) + (y - hy).powi(2) <= reach * reach
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

    /// The aim for the pointer at `p` while Shelly is being pulled.
    pub fn aim_for_pointer(&self, p: (f32, f32)) -> Option<u16> {
        let (hx, hy) = self.home();
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

    /// Pixels per number on the path.
    pub fn step_px(&self) -> f32 {
        self.unit * self.scale as f32
    }
}

// ─── Input ──────────────────────────────────────────────

pub enum HopInput {
    Leave,
    Again,
    /// Move the aim one step (−1 back, +1 further).
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
/// once she has the pearl, goes again).
pub fn handle_key(input: &FrameInput, session: &HopSession) -> Option<HopInput> {
    if input.pressed(KeyCode::Escape) {
        return Some(HopInput::Leave);
    }
    let go = input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter);
    if session.phase == HopPhase::Won {
        return go.then_some(HopInput::Again);
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
    /// Pointer position while Shelly is being pulled back.
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
const SHELL: Color = Color::new(0.94, 0.67, 0.71, 1.0);
const SHELL_DARK: Color = Color::new(0.78, 0.47, 0.53, 1.0);
const MOUTH: Color = Color::new(0.24, 0.12, 0.18, 1.0);
const PEARL: Color = Color::new(0.95, 0.97, 1.0, 1.0);
const ROCK: Color = Color::new(0.47, 0.42, 0.38, 1.0);
const ROCK_LIGHT: Color = Color::new(0.62, 0.57, 0.50, 1.0);
const STONE: Color = Color::new(0.56, 0.60, 0.58, 1.0);
const STONE_LIT: Color = Color::new(1.0, 0.93, 0.70, 1.0);
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
    /// Degrees.
    spin: f32,
    /// 0 shut … 1 wide open (a yell, a cheer).
    mouth: f32,
    /// Eyes: 0 normal, 1 wide (mid-air panic), -1 squeezed shut (effort).
    eyes: f32,
    /// Little flailing feet.
    flail: bool,
    /// Half under water.
    wet: bool,
}

impl Pose {
    fn at(x: f32, y: f32) -> Pose {
        Pose { x, y, squash: 1.0, spin: 0.0, mouth: 0.15, eyes: 0.0, flail: false, wet: false }
    }
}

pub fn draw(view: &HopView, l: &PearlHopLayout, art: &HopArt, time: f32) {
    let f = &l.frame;
    let g = &l.scene;
    let s = view.session;

    // The whole screen is the lagoon: sky over water, so the buttons sit on
    // the scene rather than on a dark modal.
    let sky = f.bounds;
    paint::vgradient(sky, SKY_TOP, SKY_BOTTOM, 12);
    draw_water(g, f.bounds, time);

    let c = paint::canvas(g.rect);
    draw_path(&c, g, s, time);
    draw_ghost(&c, g, s, art, time);
    draw_counts(&c, g, s);
    draw_hop_bubbles(&c, g, s, time);
    draw_target_bubble(&c, g, s, time);
    let pose = shelly_pose(g, s, art, time);
    draw_splash(&c, g, s);
    draw_band(&c, g, s, art, pose);
    draw_shelly(&c, g, pose, time);
    draw_win(&c, g, s, time);
    if let Some((hx, hy, pressed)) = art.hand {
        draw_hand(&c, hx, hy, pressed, art.hand_alpha);
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
            (_, HopId::Leave) => {
                paint::round_rect(el.rect, 10.0, Color::new(0.26, 0.35, 0.42, 0.92));
            }
            (_, HopId::Again) => {
                let pulse = (time * 3.0).sin() * 0.5 + 0.5;
                paint::round_rect(el.rect, 14.0, BTN);
                paint::outline(el.rect, 2.0 + pulse * 2.0, GOLD);
            }
            (_, HopId::PearlIcon) => draw_pearl(&paint::canvas(el.rect), el.rect.center(), el.rect.w * 0.42, time),
            (_, HopId::AgainIcon) => draw_pearl(&paint::canvas(el.rect), el.rect.center(), el.rect.w * 0.42, time),
            (_, HopId::LeaveIcon) => draw_back_arrow(&paint::canvas(el.rect), el.rect),
            _ => {}
        }
    }
}

/// Water from the surface line to the bottom of the screen, a sandy floor,
/// and a little lazy kelp. Drawn over the full width so the buttons below
/// the scene sit in the sea.
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
        // Wide flat ellipses make seamless bands without hand-made rects.
        let hh = (h * 0.5 + 1.0).min(bottom - y);
        c.line(bounds.x, y + hh, bounds.right(), y + hh, hh * 2.0, col);
    }
    // Surface ripples.
    let mut x = bounds.x;
    let mut i = 0;
    while x < bounds.right() {
        let wob = (time * 2.0 + i as f32 * 0.9).sin() * 2.0;
        c.line(x, top + wob, (x + 18.0).min(bounds.right()), top - wob, 2.0, Color::new(1.0, 1.0, 1.0, 0.55));
        x += 26.0;
        i += 1;
    }
    // Sandy floor.
    let floor = bottom - (bottom - top) * 0.18;
    let mut x = bounds.x;
    while x < bounds.right() + 20.0 {
        c.circle(x.min(bounds.right() - 1.0), bottom + 8.0, (bottom - floor).max(4.0), Color::new(0.86, 0.76, 0.52, 0.55));
        x += 38.0;
    }
    // Rising bubbles.
    for k in 0..7 {
        let bx = bounds.x + bounds.w * ((k as f32 * 0.137 + 0.07) % 1.0);
        let t = (time * 0.25 + k as f32 * 0.31) % 1.0;
        let by = lerp(bottom - 6.0, top + 6.0, t);
        c.circle_lines(bx, by, 2.5 + k as f32 % 3.0, 1.2, Color::new(1.0, 1.0, 1.0, 0.45 * (1.0 - t)));
    }
}

/// The start rock, the stepping stones (numbered), and the pearl rock. Out on
/// the estimation line there are no stones: just 0, the far end, and water.
fn draw_path(c: &paint::Canvas, g: &SceneGeom, s: &HopSession, time: f32) {
    let round = &s.round;
    // Start rock: a taller boulder so Shelly stands above the path.
    let (x0, rock_top) = (g.x0, g.rock_top());
    // Narrow enough on a crowded path that stone 1 stays in the clear.
    let rock_w = match round.stage {
        HopStage::Estimate => g.r * 1.45,
        _ => (g.step_px() - g.stone_r - 4.0).clamp(g.r * 1.05, g.r * 1.45),
    };
    c.ellipse(x0, g.surface_y + g.r * 0.2, rock_w, g.r * 1.3, 0.0, ROCK);
    c.ellipse(x0 - rock_w * 0.2, rock_top + g.r * 0.35, rock_w * 0.62, g.r * 0.45, 0.0, ROCK_LIGHT);
    let label = match round.stage {
        HopStage::Estimate => 20,
        _ => (g.step_px() * 0.5).clamp(12.0, 22.0) as u16,
    };
    c.text_centered("0", x0, g.surface_y + g.r * 1.2 + label as f32, label, WHITE);

    let lit_up_to = lit_position(s);
    match round.stage {
        HopStage::Estimate => {
            // A far rock with the line's far end on it, and faint ticks at
            // the quarter marks so the line reads as a line — not labelled,
            // that would give the estimate away.
            let xe = g.x_of(round.span_pos());
            c.ellipse(xe, g.surface_y + g.r * 0.35, g.r * 1.2, g.r * 1.0, 0.0, ROCK);
            c.text_centered(&round.span.to_string(), xe, g.surface_y + g.r * 1.3 + label as f32, label, WHITE);
            c.line(x0, g.surface_y + 3.0, xe, g.surface_y + 3.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.5));
            for q in 1..4 {
                let x = lerp(x0, xe, q as f32 / 4.0);
                c.line(x, g.surface_y - 4.0, x, g.surface_y + 10.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.5));
            }
        }
        _ => {
            let step = g.step_px();
            let every = if step >= 18.0 { 1 } else { 2 };
            for n in 1..round.pearl {
                let x = g.x_of(n * round.scale);
                let lit = lit_up_to.is_some_and(|p| n * round.scale <= p);
                let bob = (time * 1.3 + n as f32).sin() * 0.8;
                c.ellipse(x, g.surface_y + bob, g.stone_r, g.stone_r * 0.75, 0.0, if lit { STONE_LIT } else { STONE });
                c.ellipse(x - g.stone_r * 0.25, g.stone_top() + bob + g.stone_r * 0.15, g.stone_r * 0.55, g.stone_r * 0.2, 0.0,
                    Color::new(1.0, 1.0, 1.0, 0.35));
                if n % every == 0 {
                    c.text_centered(&n.to_string(), x, g.surface_y + g.stone_r + label as f32 + 2.0, label, WHITE);
                }
            }
            // The pearl rock: bigger, with the pearl glinting on top until
            // it's been won (then it's up in the air, see draw_win).
            let xp = g.x_of(round.pearl_pos());
            c.ellipse(xp, g.surface_y + g.r * 0.1, g.r * 1.05, g.r * 0.85, 0.0, ROCK);
            c.ellipse(xp - g.r * 0.2, g.surface_y - g.r * 0.5, g.r * 0.6, g.r * 0.25, 0.0, ROCK_LIGHT);
            c.text_centered(&round.pearl.to_string(), xp, g.surface_y + g.r * 1.1 + label as f32, label, GOLD);
            if s.phase != HopPhase::Won {
                draw_pearl(c, (xp, g.surface_y - g.r * 0.95), g.r * 0.42, time);
            }
        }
    }
}

/// While flying and after landing, the stones she's already touched glow.
fn lit_position(s: &HopSession) -> Option<u16> {
    let t = s.toss.as_ref()?;
    match s.phase {
        HopPhase::Flying => {
            let (hop, _) = s.flight()?;
            hop.checked_sub(1).map(|i| t.landings[i])
        }
        HopPhase::Landed | HopPhase::Won => Some(t.end()),
        HopPhase::Aiming => None,
    }
}

/// The dotted arc of the first hop the current pull would make, with the
/// stone it lands on ringed — the thing that snaps stone to stone as the kid
/// pulls.
fn draw_ghost(c: &paint::Canvas, g: &SceneGeom, s: &HopSession, art: &HopArt, time: f32) {
    if s.phase != HopPhase::Aiming {
        // Last try's mark stays on the estimation line while she's back on
        // the rock, so the next estimate can be adjusted from it.
        return;
    }
    // On the estimation line, where the last try came down stays marked.
    if s.round.stage == HopStage::Estimate {
        if let Some(t) = s.toss.as_ref() {
            let x = g.x_of(t.end());
            c.circle_lines(x, g.surface_y, g.r * 0.6, 3.0, Color::new(1.0, 1.0, 1.0, 0.8));
            c.line(x, g.surface_y - g.r * 1.2, x, g.surface_y - g.r * 0.6, 2.0, Color::new(1.0, 1.0, 1.0, 0.8));
        }
    }
    let pulling = art.pull_to.is_some_and(|p| g.aim_for_pointer(p).is_some());
    let keyboard_aimed = s.aim != s.round.min_aim || s.toss.is_some();
    if !pulling && !keyboard_aimed {
        return;
    }
    let (hx, hy) = g.home();
    let first = s.round.landings(s.aim).first().copied().unwrap_or(s.aim);
    let x1 = g.x_of(first);
    let water = s.round.stage == HopStage::Estimate || first > s.round.pearl_pos();
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
    c.circle_lines(x1, y1 + g.r * 0.5, g.r * (0.8 + 0.1 * pulse), 3.0, with_alpha(GOLD, 0.9));
    // On the stones, the hop's size rides on the ghost landing — the number
    // Shelly is counting out loud.
    if s.round.stage != HopStage::Estimate {
        let n = first / s.round.scale;
        let size = (g.r * 1.1) as u16;
        c.circle(x1, y1 - g.r * 1.9, g.r * 0.85, Color::new(1.0, 1.0, 1.0, 0.85));
        c.text_centered(&n.to_string(), x1, y1 - g.r * 1.9 + size as f32 * 0.36, size.max(14), INK);
    }
}

fn arc_height(g: &SceneGeom, xa: f32, xb: f32) -> f32 {
    let room = (g.home().1 - g.rect.y - g.r * 1.6).max(10.0);
    ((xb - xa).abs() * 0.45).clamp(30.0, 240.0).min(room)
}

/// Skip counting out loud: every spot she touches down on gets its number
/// popped up over it (2… 4… 6…).
fn draw_counts(c: &paint::Canvas, g: &SceneGeom, s: &HopSession) {
    if s.round.stage == HopStage::Estimate || s.round.stage == HopStage::Count {
        return;
    }
    let Some(t) = s.toss.as_ref() else { return };
    let shown = match s.phase {
        HopPhase::Flying => s.flight().map_or(0, |(hop, _)| hop),
        HopPhase::Landed | HopPhase::Won => t.landings.len(),
        HopPhase::Aiming => 0,
    };
    let size = (g.r * 0.95).clamp(14.0, 26.0) as u16;
    for (i, &p) in t.landings.iter().take(shown).enumerate() {
        let x = g.x_of(p);
        let y = g.surface_y - g.r * 2.7 - if i % 2 == 1 { g.r * 0.7 } else { 0.0 };
        c.circle(x, y - size as f32 * 0.35, size as f32 * 0.8, Color::new(1.0, 1.0, 1.0, 0.8));
        c.text_centered(&(p / s.round.scale).to_string(), x, y, size, INK);
    }
}

/// "Reach the pearl in K hops": K bubbles over Shelly's head, one popping per
/// hop.
fn draw_hop_bubbles(c: &paint::Canvas, g: &SceneGeom, s: &HopSession, time: f32) {
    if s.round.stage != HopStage::Hops {
        return;
    }
    let k = s.round.hops as usize;
    let used = match s.phase {
        HopPhase::Flying => s.flight().map_or(0, |(hop, _)| hop + 1),
        HopPhase::Landed | HopPhase::Won => k,
        HopPhase::Aiming => 0,
    };
    let (hx, hy) = g.home();
    let br = (g.r * 0.5).max(10.0);
    let gap = br * 2.5;
    let y = hy - g.r * 2.4;
    let x_start = (hx - (k as f32 - 1.0) * gap / 2.0).max(g.rect.x + br + 2.0);
    for i in 0..k {
        let x = x_start + i as f32 * gap;
        let bob = (time * 2.2 + i as f32).sin() * 2.0;
        if i < used {
            // Popped: a little ring of spray.
            c.circle_lines(x, y + bob, br * 0.5, 1.5, Color::new(1.0, 1.0, 1.0, 0.35));
        } else {
            c.circle(x, y + bob, br, Color::new(0.75, 0.93, 1.0, 0.95));
            c.circle_lines(x, y + bob, br, 2.5, Color::new(0.10, 0.35, 0.55, 0.9));
            c.circle(x - br * 0.35, y + bob - br * 0.35, br * 0.25, WHITE);
        }
    }
}

/// On the estimation line the pearl is hidden; Shelly's thought bubble says
/// what number it sank at.
fn draw_target_bubble(c: &paint::Canvas, g: &SceneGeom, s: &HopSession, time: f32) {
    if s.round.stage != HopStage::Estimate || s.phase == HopPhase::Won {
        return;
    }
    let (hx, hy) = g.home();
    let bob = (time * 1.8).sin() * 2.0;
    let br = (g.r * 1.15).max(20.0);
    let (bx, by) = (hx + g.r * 1.9, hy - g.r * 2.3 + bob);
    c.circle(hx + g.r * 0.7, hy - g.r * 1.2 + bob, br * 0.18, Color::new(1.0, 1.0, 1.0, 0.9));
    c.circle(hx + g.r * 1.1, hy - g.r * 1.6 + bob, br * 0.28, Color::new(1.0, 1.0, 1.0, 0.9));
    c.circle(bx, by, br, WHITE);
    c.circle_lines(bx, by, br, 2.0, GOLD);
    draw_pearl(c, (bx - br * 0.45, by - br * 0.1), br * 0.22, time);
    let size = (br * 0.85) as u16;
    c.text_centered(&s.round.pearl.to_string(), bx + br * 0.2, by + size as f32 * 0.35, size.max(14), INK);
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
                // Pulled back: she follows the pointer a little way and
                // squishes like a rubber ball, eyes squeezed.
                let (dx, dy) = (ptr.0 - hx, ptr.1 - hy);
                let d = (dx * dx + dy * dy).sqrt().max(0.001);
                let give = d.min(g.r * 1.6);
                let t = (d / g.max_pull).clamp(0.0, 1.0);
                p.x += dx / d * give;
                p.y += dy / d * give;
                p.squash = 1.0 + 0.45 * t;
                p.spin = -12.0 * t;
                p.eyes = if t > 0.15 { -1.0 } else { 0.0 };
                p.mouth = 0.05;
            }
            p
        }
        HopPhase::Flying => {
            let t = s.toss.as_ref().unwrap();
            let (hop, u) = s.flight().unwrap_or((0, 0.0));
            let from = if hop == 0 { 0 } else { t.landings[hop - 1] };
            let to = t.landings[hop];
            let last = hop + 1 == t.landings.len();
            let water_end = last && s.round.stage == HopStage::Estimate
                || to > s.round.pearl_pos() && s.round.stage != HopStage::Estimate;
            let (xa, ya) = if hop == 0 { g.home() } else { g.perch(from, false) };
            let (xb, yb) = g.perch(to, water_end);
            let h = arc_height(g, xa, xb);
            let x = lerp(xa, xb, u);
            let y = lerp(ya, yb, u) - 4.0 * h * u * (1.0 - u);
            let mut p = Pose::at(x, y);
            // Stretch on take-off, spin through the air, flail the whole way.
            p.squash = if u < 0.15 { 0.7 } else { 1.0 };
            p.spin = u * 360.0;
            p.eyes = 1.0;
            p.mouth = 0.7 + 0.3 * (time * 18.0).sin().abs();
            p.flail = true;
            p
        }
        HopPhase::Landed => landed_pose(g, s, time),
        HopPhase::Won => {
            let end = s.toss.as_ref().map_or(0, |t| t.end());
            let water = s.round.stage == HopStage::Estimate;
            let (x, y) = g.perch(end, water);
            let c = s.clock;
            let mut p = Pose::at(x, y);
            if c < 0.25 {
                p.squash = 1.5 - c * 2.0; // BONK onto the pearl
            } else {
                // Happy bouncing.
                p.y -= ((c - 0.25) * 7.0).sin().abs() * g.r * 0.7;
                p.mouth = 1.0;
            }
            p.wet = water && c < 0.25;
            p
        }
    }
}

fn landed_pose(g: &SceneGeom, s: &HopSession, time: f32) -> Pose {
    let t = s.toss.as_ref().unwrap();
    let c = s.clock;
    let end = t.end();
    if s.lands_in_water() {
        // SPLASH. Bob up, spin in place if it was close, paddle home, hop up.
        let (lx, ly) = g.perch(end, true);
        let (hx, hy) = g.home();
        let paddle_end = SPLASH_SECS - 0.5;
        if c < 0.4 {
            let mut p = Pose::at(lx, ly + (0.4 - c) * g.r * 2.0);
            p.wet = true;
            p.eyes = 1.0;
            p.mouth = 0.9;
            return p;
        }
        if c < paddle_end {
            let u = ease((c - 0.4) / (paddle_end - 0.4));
            let mut p = Pose::at(lerp(lx, hx, u), ly + (time * 9.0).sin() * 2.0);
            p.wet = true;
            p.spin = (time * 9.0).sin() * 8.0;
            if t.near && c < 1.0 {
                // So close! A dizzy wobble right by the spot.
                p.spin = (c * 30.0).sin() * 35.0;
                p.x = lx;
            }
            p.flail = true;
            p.mouth = 0.3;
            return p;
        }
        let u = ((c - paddle_end) / 0.5).clamp(0.0, 1.0);
        let x = hx;
        let y = lerp(ly, hy, u) - 4.0 * g.r * 1.2 * u * (1.0 - u);
        let mut p = Pose::at(x, y);
        p.squash = if u > 0.9 { 1.3 } else { 1.0 };
        return p;
    }
    // A plain stone: bonk, wobble, shrug, hop home.
    let (lx, ly) = g.perch(end, false);
    let mut p = Pose::at(lx, ly);
    if c < 0.3 {
        p.squash = 1.6 - c;
        p.eyes = -1.0;
    } else if c < 0.95 {
        let decay = 1.0 - (c - 0.3) / 0.65;
        p.spin = (c * 24.0).sin() * 28.0 * decay;
    } else if c < 1.35 {
        // Shrug: tilt one way, then the other, mouth a little "hm".
        p.spin = ((c - 0.95) * 16.0).sin() * 12.0;
        p.mouth = 0.25;
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
/// being pulled.
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
        let (tx, ty) = if taut { (pose.x, pose.y + pose_half_h(g, pose) * 0.5) } else { (hx, hy + g.r * 0.55) };
        c.line(px, py - g.r * 0.6, tx, ty, if taut { 3.0 } else { 2.0 }, Color::new(0.85, 0.30, 0.25, 0.95));
    }
}

fn pose_half_h(g: &SceneGeom, pose: Pose) -> f32 {
    g.r * 0.75 / pose.squash.max(0.3)
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

fn draw_shelly(c: &paint::Canvas, g: &SceneGeom, p: Pose, time: f32) {
    let r = g.r;
    let (sx, sy) = (p.squash, 1.0 / p.squash.max(0.3));
    let rot = p.spin.to_radians();
    let (sn, cs) = rot.sin_cos();
    // Local shell coordinates → screen, through squash then spin.
    let tf = |lx: f32, ly: f32| -> (f32, f32) {
        let (x, y) = (lx * sx, ly * sy);
        (p.x + x * cs - y * sn, p.y + x * sn + y * cs)
    };

    // Shadow on whatever's underneath (skip mid-air — she's high up).
    if !p.flail && !p.wet {
        c.ellipse(p.x, p.y + r * 0.8 * sy, r * 0.9 * sx, r * 0.18, 0.0, Color::new(0.0, 0.0, 0.0, 0.18));
    }

    // Flailing feet under the shell.
    if p.flail {
        for i in 0..2 {
            let side = if i == 0 { -1.0 } else { 1.0 };
            let wig = (time * 26.0 + i as f32 * 2.0).sin() * r * 0.35;
            let (ax, ay) = tf(side * r * 0.35, r * 0.45);
            let (bx, by) = tf(side * r * 0.55 + wig, r * 0.95);
            c.line(ax, ay, bx, by, (r * 0.16).max(2.0), SHELL_DARK);
        }
    }

    // Bottom shell.
    let (bx, by) = tf(0.0, r * 0.25);
    c.ellipse(bx, by, r * sx, r * 0.5 * sy, p.spin, SHELL);
    // The mouth: a dark gap that yawns open to yell / cheer.
    let open = r * (0.12 + 0.38 * p.mouth.clamp(0.0, 1.0));
    let (mx, my) = tf(0.0, r * 0.02);
    c.ellipse(mx, my, r * 0.82 * sx, open * sy, p.spin, MOUTH);
    if p.mouth > 0.5 {
        let (tx, ty) = tf(0.0, r * 0.1);
        c.ellipse(tx, ty, r * 0.35 * sx, open * 0.45 * sy, p.spin, Color::new(0.93, 0.45, 0.55, 1.0));
    }
    // Top shell, lifted by the open mouth.
    let (ux, uy) = tf(0.0, -r * 0.2 - open * 0.6);
    c.ellipse(ux, uy, r * 0.95 * sx, r * 0.5 * sy, p.spin, SHELL);
    for i in -1..=1 {
        let (ax, ay) = tf(i as f32 * r * 0.45, -r * 0.55 - open * 0.6);
        let (bx2, by2) = tf(i as f32 * r * 0.3, -r * 0.05 - open * 0.6);
        c.line(ax, ay, bx2, by2, (r * 0.08).max(1.5), SHELL_DARK);
    }
    // Eyes on stalks peeking over the lid.
    for side in [-1.0f32, 1.0] {
        let (ex, ey) = tf(side * r * 0.32, -r * 0.78 - open * 0.6);
        let er = r * if p.eyes > 0.5 { 0.3 } else { 0.24 };
        if p.eyes < -0.5 {
            // Squeezed shut: > <
            let d = er * 0.9;
            c.line(ex - d, ey - d * 0.6, ex + d * 0.2, ey, 2.0, INK);
            c.line(ex - d, ey + d * 0.6, ex + d * 0.2, ey, 2.0, INK);
        } else {
            c.circle(ex, ey, er, WHITE);
            c.circle_lines(ex, ey, er, 1.2, INK);
            let look = if p.eyes > 0.5 { 0.0 } else { er * 0.25 };
            c.circle(ex + look, ey, er * (if p.eyes > 0.5 { 0.35 } else { 0.5 }), INK);
        }
    }
    if p.wet {
        // The water line cuts across her: a band of water over her bottom half.
        let wl = g.surface_y;
        c.ellipse(p.x, wl + r * 0.35, r * 1.3, r * 0.45, 0.0, Color::new(0.10, 0.55, 0.70, 0.85));
        c.line(p.x - r * 1.3, wl, p.x + r * 1.3, wl, 2.0, Color::new(1.0, 1.0, 1.0, 0.6));
    }
}

/// The pearl pops: a burst ring, sparkles, and the pearl bobbing up high.
fn draw_win(c: &paint::Canvas, g: &SceneGeom, s: &HopSession, time: f32) {
    if s.phase != HopPhase::Won {
        return;
    }
    let end = s.toss.as_ref().map_or(0, |t| t.end());
    let x = g.x_of(end);
    let k = s.clock;
    let base_y = g.surface_y - g.r * 1.2;
    let rise = ease(k / 0.8);
    let py = lerp(base_y, g.rect.y + g.rect.h * 0.22, rise);
    if k < 0.9 {
        let ring = g.r * (0.5 + k * 4.0);
        c.circle_lines(x, base_y, ring, 4.0, with_alpha(GOLD, 1.0 - k / 0.9));
    }
    for i in 0..10 {
        let a = i as f32 / 10.0 * std::f32::consts::TAU + time * 0.8;
        let d = g.r * (1.2 + 0.4 * (time * 3.0 + i as f32).sin()) * (0.6 + rise);
        let sz = 2.5 + (i % 3) as f32;
        c.circle(x + a.cos() * d, py + a.sin() * d, sz, with_alpha(GOLD, 0.85));
    }
    draw_pearl(c, (x, py), g.r * 0.7, time);
}

/// A pearl with a highlight. Used on the pearl rock, in the purse, on the
/// Again button and in Shelly's thought bubble.
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
    use robot_buddy_domain::logic::pearl_hop::generate_round;
    use ::rand::rngs::SmallRng;
    use ::rand::SeedableRng;

    fn geom(band: u8, screen: (f32, f32)) -> (HopSession, PearlHopLayout) {
        let s = HopSession::new(generate_round(band, &mut SmallRng::seed_from_u64(4)));
        let l = layout(&HopView { session: &s, pearls: 3, caption: "My pearl is on stone 5!" }, screen);
        (s, l)
    }

    #[test]
    fn dragging_to_the_drag_point_sets_exactly_that_aim() {
        for band in [1u8, 2, 3, 4, 6] {
            for &screen in &layout::SWEEP_SCREENS {
                let (s, l) = geom(band, screen);
                for aim in (s.round.min_aim..=s.round.max_aim).step_by(s.round.aim_step() as usize) {
                    let p = l.scene.drag_point_for(aim);
                    assert_eq!(l.scene.aim_for_pointer(p), Some(aim), "band {band} {screen:?} aim {aim}");
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
        for band in [1u8, 2, 3, 4, 6] {
            for &screen in &layout::SWEEP_SCREENS {
                let (s, l) = geom(band, screen);
                let g = &l.scene;
                assert!(g.x_of(s.round.span_pos()) <= g.rect.right(), "band {band} {screen:?}");
                let (x, y) = g.drag_point_for(s.round.max_aim);
                assert!(x >= 0.0 && y <= screen.1, "the biggest pull stays on screen: band {band} {screen:?} ({x}, {y})");
            }
        }
    }
}
