//! Sockbot's Sock Shop, as a Bevy UI panel: the order line, the shop scene
//! (an [`ArtId::SockShop`] art slot: the customers' feet, the counter with
//! the rolled-up pairs, Sockbot behind it), Sockbot's line, and the drawer
//! buttons. The rules live in `robot_buddy_domain::logic::socks`.
//!
//! The pairs on the counter sit in rows of five, so "how many?" can be read
//! at a glance at any size of order. Socks fly onto feet one at a time when
//! an order is sent; an odd order's lonely sock flies to Sockbot.

use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::{default, AlignItems, AlignSelf, Bundle, ChildSpawner, Children, JustifyContent, Node, SpawnRelated, UiRect};
use bevy::text::Justify;
use bevy::ui::{percent, px, BackgroundGradient, BorderColor, ColorStop, LinearGradient};

use crate::prelude::*;
use robot_buddy_domain::logic::socks::{Fit, SockPhase, SockRound, SockSession, SockStage, Walker};

use super::kit::{self, backdrop, button, col, fitted, Art, Panel};
use super::views::UiViews;
use super::{ArtId, UiAction};
use crate::input::FrameInput;

/// What the panel's nodes are built from. Changes only when a button, a
/// count or a line changes, so the panel isn't rebuilt every frame.
#[derive(Clone, PartialEq, Debug)]
pub struct SockPanelView {
    /// "3 tripods need socks!"
    pub order: String,
    /// Sockbot's line, for a grown-up reading along. The kid hears it.
    pub caption: String,
    /// The pairs on the counter.
    pub pairs: u16,
    /// The step buttons the drawer offers (1, and 10 for big orders).
    pub steps: Vec<u16>,
    /// The order's been socked and the dance is over: "Next!" is up.
    pub next: bool,
}

/// What Sock Shop shows this frame: the panel's view, plus the session the
/// scene animates from.
#[derive(Clone, Debug)]
pub struct SockView {
    pub panel: SockPanelView,
    pub session: SockSession,
}

/// The panel only rebuilds when its nodes would change; the scene is art,
/// redrawn every frame from `session` regardless.
impl PartialEq for SockView {
    fn eq(&self, other: &Self) -> bool {
        self.panel == other.panel
    }
}

impl SockView {
    pub fn new(session: &SockSession, caption: &str) -> SockView {
        SockView {
            panel: SockPanelView {
                order: order_line(&session.round),
                caption: caption.to_string(),
                pairs: session.pairs,
                steps: session.round.steps().to_vec(),
                next: session.win_done(),
            },
            session: session.clone(),
        }
    }
}

/// The order, in words: "A tripod needs socks!", "3 dog-bots need socks!".
pub fn order_line(round: &SockRound) -> String {
    let name = round.walker.name(round.robots);
    if round.robots == 1 {
        let a = if name.starts_with(['a', 'e', 'i', 'o', 'u']) { "An" } else { "A" };
        format!("{a} {name} needs socks!")
    } else {
        format!("{} {name} need socks!", round.robots)
    }
}

/// "1 pair", "6 pairs".
pub fn pairs_label(n: u16) -> String {
    if n == 1 { "1 pair".into() } else { format!("{n} pairs") }
}

/// The Sock Shop's buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SockId {
    /// Take `n` more pairs out of the drawer.
    More(u16),
    /// Put `n` pairs back.
    Fewer(u16),
    /// Hand the order over.
    Send,
    /// The next customer.
    Next,
    Leave,
}

const WALL_TOP: Color = Color::new(0.98, 0.93, 0.80, 1.0);
const WALL_BOTTOM: Color = Color::new(0.80, 0.88, 0.92, 1.0);
const TEAL: Color = Color::new(0.235, 0.745, 0.706, 1.0);
const INK: Color = Color::new(0.13, 0.13, 0.16, 1.0);
const EDGE: Color = Color::new(1.0, 1.0, 1.0, 0.35);

pub struct SockPanel;

impl Panel for SockPanel {
    type View = SockView;
    const Z: i32 = 40;
    fn view(views: &UiViews) -> Option<&SockView> {
        views.socks.as_ref()
    }
    fn bundle(v: &SockView) -> impl Bundle {
        let v = v.panel.clone();
        (backdrop(0.55, 16.0), Children::spawn(SpawnWith(move |p: &mut ChildSpawner| spawn_panel(p, &v))))
    }
}

fn spawn_panel(p: &mut ChildSpawner, v: &SockPanelView) {
    let panel = Node {
        width: percent(100),
        max_width: px(760),
        height: percent(100),
        max_height: px(680),
        padding: UiRect::new(px(14), px(14), px(10), px(12)),
        row_gap: px(6),
        border: UiRect::all(px(4)),
        ..col()
    };
    let wall = LinearGradient::to_bottom(vec![ColorStop::auto(kit::color(WALL_TOP)), ColorStop::auto(kit::color(WALL_BOTTOM))]);
    p.spawn((panel, BackgroundGradient::from(wall), BorderColor::all(kit::color(TEAL)))).with_children(|p| {
        let line = |p: &mut ChildSpawner, s: String, size: f32, c: Color| {
            p.spawn(Node { flex_shrink: 0.0, ..col() }).with_child(fitted(s, size, 12.0, true, c, Justify::Center));
        };
        // Leave up top, out of the way of the drawer buttons.
        let top = Node { justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, flex_shrink: 0.0, column_gap: px(8), ..default() };
        p.spawn(top).with_children(|p| {
            p.spawn(Node { flex_grow: 1.0, flex_shrink: 1.0, min_width: px(0), ..col() })
                .with_child(fitted(v.order.clone(), 26.0, 14.0, true, INK, Justify::Left));
            let leave = Node { width: px(100), height: px(40), flex_shrink: 0.0, ..kit::centered() };
            p.spawn(button(UiAction::Socks(SockId::Leave), leave, Color::new(0.329, 0.431, 0.478, 1.0), EDGE))
                .with_child(fitted("Bye!", 22.0, 14.0, false, WHITE, Justify::Center));
        });
        // The shop takes whatever height is left.
        p.spawn((Art(ArtId::SockShop), Node { flex_grow: 1.0, min_height: px(0), width: percent(100), ..default() }));
        line(p, v.caption.clone(), 22.0, INK);
        // On a narrow phone with tens buttons, "Here you go!" wraps under.
        let row = Node {
            column_gap: px(8),
            row_gap: px(6),
            flex_wrap: bevy::ui::FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            min_height: px(56),
            ..default()
        };
        p.spawn(row).with_children(|p| {
            if v.next {
                let next = Node { width: px(220), height: px(56), ..kit::centered() };
                p.spawn(button(UiAction::Socks(SockId::Next), next, Color::from_rgba(46, 160, 90, 255), EDGE))
                    .with_child(fitted("Next robot!", 26.0, 14.0, false, WHITE, Justify::Center));
                return;
            }
            for &n in v.steps.iter().rev() {
                step(p, SockId::Fewer(n), format!("-{n}"));
            }
            let count = Node { width: px(118), height: px(56), flex_shrink: 1.0, min_width: px(70), ..kit::centered() };
            p.spawn(kit::boxed(count, WHITE, 2.0, TEAL))
                .with_child(fitted(pairs_label(v.pairs), 24.0, 13.0, false, INK, Justify::Center));
            for &n in &v.steps {
                step(p, SockId::More(n), format!("+{n}"));
            }
            let send = Node { width: px(150), height: px(56), flex_shrink: 1.0, min_width: px(84), ..kit::centered() };
            p.spawn(button(UiAction::Socks(SockId::Send), send, Color::from_rgba(240, 120, 40, 255), EDGE))
                .with_child(fitted("Here you go!", 22.0, 12.0, false, WHITE, Justify::Center));
        });
    });
}

fn step(p: &mut ChildSpawner, id: SockId, label: String) {
    let node = Node { width: px(62), height: px(56), min_width: px(44), flex_shrink: 1.0, align_self: AlignSelf::Center, ..kit::centered() };
    p.spawn(button(UiAction::Socks(id), node, TEAL, EDGE))
        .with_child(fitted(label, 26.0, 14.0, false, WHITE, Justify::Center));
}

/// What the kid asked for this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SockInput {
    Add(i32),
    Send,
    Next,
    Leave,
}

pub fn handle_click(input: &FrameInput) -> Option<SockInput> {
    let Some(UiAction::Socks(id)) = input.ui_press else { return None };
    Some(match id {
        SockId::More(n) => SockInput::Add(n as i32),
        SockId::Fewer(n) => SockInput::Add(-(n as i32)),
        SockId::Send => SockInput::Send,
        SockId::Next => SockInput::Next,
        SockId::Leave => SockInput::Leave,
    })
}

/// Arrows add and take back a pair (Shift: ten, where the drawer has tens);
/// Space or Enter hands the order over, or calls the next robot once the
/// dance is done; Escape leaves.
pub fn handle_key(input: &FrameInput, session: &SockSession) -> Option<SockInput> {
    if input.pressed(KeyCode::Escape) {
        return Some(SockInput::Leave);
    }
    if input.pressed(KeyCode::Space) || input.pressed(KeyCode::Enter) {
        return Some(if session.win_done() { SockInput::Next } else { SockInput::Send });
    }
    let big = *session.round.steps().last().unwrap_or(&1) as i32;
    let n = if input.down(KeyCode::LeftShift) || input.down(KeyCode::RightShift) { big } else { 1 };
    if input.pressed(KeyCode::Up) || input.pressed(KeyCode::Right) {
        return Some(SockInput::Add(n));
    }
    if input.pressed(KeyCode::Down) || input.pressed(KeyCode::Left) {
        return Some(SockInput::Add(-n));
    }
    None
}

// ─── The shop (art slot) ────────────────────────────────

/// Pair colours: both socks of pair k are `SOCKS[k % len]`.
const SOCKS: [Color; 6] = [
    Color::new(0.94, 0.35, 0.63, 1.0),
    Color::new(1.0, 0.82, 0.24, 1.0),
    Color::new(0.47, 0.55, 1.0, 1.0),
    Color::new(0.36, 0.80, 0.45, 1.0),
    Color::new(1.0, 0.55, 0.25, 1.0),
    Color::new(0.68, 0.45, 0.90, 1.0),
];

fn sock_color(sock: u16) -> Color {
    SOCKS[(sock / 2) as usize % SOCKS.len()]
}

/// Where everything in the shop goes, in the slot's own coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct ShopLayout {
    /// The floor line the customers stand on.
    pub floor: f32,
    /// One entry per robot drawn: its centre x and the scale it's drawn at.
    pub robots: Vec<(f32, f32)>,
    /// Every drawn foot, robot by robot, left to right: where its sock goes.
    pub feet: Vec<(f32, f32)>,
    /// The counter top: x, y, width.
    pub counter: (f32, f32, f32),
    /// Sockbot's hand, where the lonely sock ends up.
    pub hand: (f32, f32),
    /// The ball radius on the counter.
    pub ball: f32,
}

/// How many robots the scene draws: everyone at the first two stages, the
/// one who stands in for the order slip at the third.
pub fn robots_drawn(round: &SockRound) -> u16 {
    match round.stage {
        SockStage::Order => 1,
        _ => round.robots,
    }
}

/// A walker's leg span at scale 1.
fn span(w: Walker) -> f32 {
    26.0 + 13.0 * w.feet() as f32
}

pub fn shop_layout(round: &SockRound, w: f32, h: f32) -> ShopLayout {
    let floor = h - 14.0;
    // The counter takes the right ~38%; Sockbot stands behind it.
    let cw = (w * 0.38).clamp(110.0, 300.0);
    let cx0 = w - cw - 8.0;
    let counter_y = l_counter_y(h);
    // His left hand, up in the air (see the transform in `draw_art`).
    let sb = sockbot_scale(cw);
    let hand = (cx0 + cw * 0.62 - 17.0 * sb, counter_y - 19.0 * sb);
    let ball = (cw / 23.0).clamp(4.0, 12.0);

    // Customers fill the left, all the same size, shrunk to fit.
    let n = robots_drawn(round);
    let area = (cx0 - 16.0).max(60.0);
    let gap = 14.0;
    let total = span(round.walker) * n as f32 + gap * (n.saturating_sub(1)) as f32;
    let tall = (floor - 28.0).max(40.0);
    let scale = (area / total).min(tall / 120.0).min(2.4).max(0.3);
    let used = total * scale;
    let mut x = 8.0 + (area - used) / 2.0;
    let mut robots = Vec::new();
    let mut feet = Vec::new();
    for _ in 0..n {
        let s = span(round.walker) * scale;
        let cx = x + s / 2.0;
        robots.push((cx, scale));
        let f = round.walker.feet();
        for i in 0..f {
            let fx = cx - s / 2.0 + s * (i as f32 + 0.5) / f as f32;
            feet.push((fx, floor - 4.0 * scale));
        }
        x += s + gap * scale;
    }
    ShopLayout { floor, robots, feet, counter: (cx0, counter_y, cw), hand, ball }
}

/// How big Sockbot is drawn behind a counter `cw` wide.
fn sockbot_scale(cw: f32) -> f32 {
    (cw / 70.0).clamp(2.0, 3.6)
}

/// The counter top: low enough that Sockbot stands tall behind it.
fn l_counter_y(h: f32) -> f32 {
    h * 0.72
}

/// Where the counter's `k`th ball sits: rows of ten, with a gap after five.
fn ball_at(l: &ShopLayout, k: u16) -> (f32, f32) {
    let (x0, y0, cw) = l.counter;
    let step = l.ball * 2.3;
    let row_w = step * 10.0 + l.ball;
    let left = x0 + (cw - row_w) / 2.0 + l.ball;
    let (row, col) = (k / 10, k % 10);
    let x = left + col as f32 * step + if col >= 5 { l.ball } else { 0.0 };
    (x, y0 - l.ball - 2.0 - row as f32 * step)
}

/// Paint the shop into its `w`×`h` slot.
pub fn draw_art(v: &SockView, w: f32, h: f32, time: f32) {
    let s = &v.session;
    let round = &s.round;
    let l = shop_layout(round, w, h);

    // Floor and wall.
    draw_rectangle(0.0, l.floor, w, h - l.floor, Color::new(0.62, 0.66, 0.72, 1.0));
    draw_line(0.0, l.floor, w, l.floor, 2.0, Color::new(0.40, 0.44, 0.50, 1.0));
    // A washing line strung across the top, with a few socks drying on it.
    draw_line(10.0, 18.0, w - 10.0, 26.0, 1.5, Color::new(0.4, 0.4, 0.45, 1.0));
    for i in 0..5 {
        let x = 30.0 + i as f32 * (w - 60.0) / 4.0;
        let y = 18.0 + 8.0 * (x / w) + 2.0;
        let sway = (time * 1.4 + i as f32).sin() * 0.12;
        draw_sock(x, y + 10.0, 1.5, SOCKS[(i * 2) % SOCKS.len()], sway, false);
    }

    // Sockbot behind his counter, big.
    let (cx0, cy, cw) = l.counter;
    // Behind the counter: his wheel is hidden, his juggling shows.
    let sb = sockbot_scale(cw);
    let sx = cx0 + cw * 0.62;
    crate::draw::with_transform(Affine2::from_scale_angle_translation(vec2(sb, sb), 0.0, vec2(sx - 24.0 * sb, cy - 34.0 * sb)), || {
        crate::sprites::robots::draw_sock_bot(0.0, 0.0, time);
    });
    // The counter.
    draw_rectangle(cx0, cy, cw, (l.floor - cy).max(10.0), Color::new(0.55, 0.36, 0.22, 1.0));
    draw_rectangle(cx0 - 6.0, cy - 6.0, cw + 12.0, 8.0, Color::new(0.70, 0.48, 0.30, 1.0));
    draw_rectangle_lines(cx0, cy, cw, (l.floor - cy).max(10.0), 2.0, INK);

    // The customers, bare feet and all.
    for &(rx, sc) in &l.robots {
        draw_walker(rx, l.floor, sc, round.walker, time, s.phase == SockPhase::Won);
    }
    // At the order stage one robot stands in for the lot.
    if round.stage == SockStage::Order {
        if let Some(&(rx, sc)) = l.robots.first() {
            let label = format!("x {}", round.robots);
            let size = (34.0 * sc.max(0.7)).min(44.0);
            let tx = rx + span(round.walker) * sc / 2.0 + 10.0;
            let ty = l.floor - 60.0 * sc;
            draw_rectangle(tx - 6.0, ty - size, measure_text(&label, None, size as u16, 1.0).width + 12.0, size + 10.0, WHITE);
            draw_rectangle_lines(tx - 6.0, ty - size, measure_text(&label, None, size as u16, 1.0).width + 12.0, size + 10.0, 2.0, INK);
            draw_text(&label, tx, ty, size, INK);
        }
    }

    // Socks on feet. At the order stage the drawn robot's feet stand for
    // every robot's: sock j goes on foot j modulo the drawn feet.
    let drawn = l.feet.len().max(1) as u16;
    let socked = s.socked();
    let shiver = s.phase == SockPhase::Reacting && matches!(s.sent, Some(Fit::Short { .. }));
    for j in 0..socked.min(drawn) {
        let (fx, fy) = l.feet[j as usize];
        draw_sock(fx, fy - 12.0, robots_scale(&l), sock_color(j), 0.0, true);
    }
    if shiver {
        // The first bare foot shivers: "brrr".
        if let Some(&(fx, fy)) = l.feet.get(socked.min(drawn - 1) as usize).filter(|_| socked < drawn) {
            let j = (time * 40.0).sin() * 2.0;
            draw_line(fx - 10.0 + j, fy - 18.0, fx - 4.0 + j, fy - 24.0, 2.0, Color::new(0.3, 0.6, 1.0, 0.8));
            draw_line(fx + 10.0 - j, fy - 18.0, fx + 4.0 - j, fy - 24.0, 2.0, Color::new(0.3, 0.6, 1.0, 0.8));
        }
    }

    // The sock in the air while dressing.
    if s.phase == SockPhase::Dressing {
        let per = s.sock_secs();
        let j = (s.clock / per).floor() as u16;
        if j < s.socks_flying() {
            let t = (s.clock - j as f32 * per) / per;
            let from = ball_at(&l, s.pairs.saturating_sub(j / 2 + 1));
            let to = l.feet[(j % drawn) as usize];
            let (x, y) = (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - 12.0 - from.1) * t - (t * std::f32::consts::PI).sin() * 60.0);
            draw_sock(x, y, robots_scale(&l), sock_color(j), t * 6.0, true);
        }
    }

    // The rolled-up pairs still on the counter.
    let on_counter = match s.phase {
        SockPhase::Choosing if round.stage.live() => s.pairs.saturating_sub(round.pairs_needed()),
        SockPhase::Choosing => s.pairs,
        SockPhase::Dressing => s.pairs.saturating_sub(((s.socked() + 1) / 2).min(s.pairs)),
        // The spares bounce back on; everything else went out.
        SockPhase::Reacting | SockPhase::Won => match s.sent {
            Some(Fit::Spare { pairs }) => pairs,
            _ => 0,
        },
    };
    let bounce = s.phase == SockPhase::Reacting && matches!(s.sent, Some(Fit::Spare { .. }));
    let first = s.pairs - on_counter;
    for k in 0..on_counter {
        let (bx, by) = ball_at(&l, k);
        let hop = if bounce { ((time * 7.0 + k as f32).sin()).abs() * 10.0 } else { 0.0 };
        draw_ball(bx, by - hop, l.ball, sock_color((first + k) * 2));
    }

    // The lonely sock, in Sockbot's hand, waving.
    if s.shows_lonely_sock() {
        let wave = (time * 5.0).sin() * 0.3;
        draw_sock(l.hand.0, l.hand.1, 1.6, sock_color(round.feet() - 1), wave + 3.14, false);
    }
}

fn robots_scale(l: &ShopLayout) -> f32 {
    l.robots.first().map_or(1.0, |r| r.1)
}

/// A rolled-up pair: a ball with a fold line.
fn draw_ball(x: f32, y: f32, r: f32, c: Color) {
    draw_circle(x, y, r + 1.0, INK);
    draw_circle(x, y, r, c);
    draw_line(x - r * 0.7, y - r * 0.2, x + r * 0.7, y + r * 0.3, 1.0, Color::new(1.0, 1.0, 1.0, 0.6));
}

/// One sock: a cuffed tube with a foot, `scale` big, turned `angle` (rad).
/// `on_foot` draws it pointing down onto a foot; otherwise hanging.
fn draw_sock(x: f32, y: f32, scale: f32, c: Color, angle: f32, on_foot: bool) {
    let xf = Affine2::from_scale_angle_translation(vec2(scale, scale), angle, vec2(x, y));
    crate::draw::with_transform(xf, || {
        let (lx, lw, lh) = (-4.0, 8.0, 12.0);
        draw_rectangle(lx - 1.0, -1.0, lw + 2.0, lh + 2.0, INK);
        draw_rectangle(lx, 0.0, lw, lh, c);
        // The foot points away from the leg.
        let toe = if on_foot { 5.0 } else { 6.0 };
        draw_rectangle(lx - 1.0, lh - 5.0, lw + toe + 2.0, 7.0, INK);
        draw_rectangle(lx, lh - 4.0, lw + toe, 5.0, c);
        // White cuff with a stripe.
        draw_rectangle(lx, 0.0, lw, 3.0, WHITE);
        draw_rectangle(lx, 5.0, lw, 1.5, Color::new(1.0, 1.0, 1.0, 0.7));
    });
}

/// A customer: a robot body over a row of legs, one per foot. `party` once
/// they're all socked: they bounce.
fn draw_walker(cx: f32, floor: f32, scale: f32, walker: Walker, time: f32, party: bool) {
    let s = scale;
    let feet = walker.feet();
    let w = span(walker) * s;
    let hop = if party { (time * 8.0).sin().abs() * 6.0 * s } else { 0.0 };
    let leg_top = floor - 28.0 * s - hop;
    let body_h = 34.0 * s;
    let body_w = (w - 8.0 * s).max(30.0 * s);
    let body_y = leg_top - body_h;
    let shell = match walker {
        Walker::Biped => Color::new(0.55, 0.62, 0.72, 1.0),
        Walker::Tripod => Color::new(0.85, 0.55, 0.30, 1.0),
        Walker::DogBot => Color::new(0.70, 0.55, 0.40, 1.0),
        Walker::StarBot => Color::new(0.95, 0.75, 0.30, 1.0),
        Walker::BugBot => Color::new(0.40, 0.70, 0.45, 1.0),
        Walker::SpiderBot => Color::new(0.45, 0.42, 0.60, 1.0),
        Walker::CentiBot => Color::new(0.80, 0.40, 0.50, 1.0),
    };
    // Shadow.
    draw_ellipse(cx, floor + 2.0, w * 0.55, 4.0 * s, 0.0, Color::new(0.0, 0.0, 0.0, 0.18));
    // Legs and bare feet.
    for i in 0..feet {
        let fx = cx - w / 2.0 + w * (i as f32 + 0.5) / feet as f32;
        let wiggle = (time * 3.0 + i as f32 * 0.9).sin() * 1.5 * s;
        draw_line(fx, leg_top, fx + wiggle, floor - 6.0 * s, 3.0 * s.max(0.6), INK);
        draw_ellipse(fx + 2.0 * s + wiggle, floor - 4.0 * s, 6.0 * s, 3.5 * s, 0.0, Color::new(0.75, 0.78, 0.82, 1.0));
    }
    // Body.
    draw_rectangle(cx - body_w / 2.0 - 1.5, body_y - 1.5, body_w + 3.0, body_h + 3.0, INK);
    draw_rectangle(cx - body_w / 2.0, body_y, body_w, body_h, shell);
    draw_rectangle(cx - body_w / 2.0 + 3.0 * s, body_y + 3.0 * s, body_w - 6.0 * s, 4.0 * s, Color::new(1.0, 1.0, 1.0, 0.3));
    // Face: two big eyes and a hopeful smile.
    let ey = body_y + body_h * 0.45;
    for dx in [-1.0, 1.0] {
        draw_circle(cx + dx * 8.0 * s, ey, 5.0 * s, WHITE);
        draw_circle(cx + dx * 8.0 * s + 1.0 * s, ey + 1.0 * s, 2.4 * s, INK);
    }
    draw_line(cx - 5.0 * s, ey + 9.0 * s, cx + 5.0 * s, ey + 9.0 * s, 1.5 * s.max(0.6), INK);
    // Antenna with a blinking light.
    draw_line(cx, body_y, cx, body_y - 10.0 * s, 1.5 * s.max(0.6), INK);
    let blink = if (time * 2.0 + cx * 0.01).sin() > 0.0 { Color::new(1.0, 0.3, 0.3, 1.0) } else { Color::new(0.6, 0.2, 0.2, 1.0) };
    draw_circle(cx, body_y - 11.0 * s, 3.0 * s, blink);
}

#[cfg(test)]
mod tests {
    use super::*;
    use robot_buddy_domain::logic::socks::SockSession;

    fn round(stage: SockStage, walker: Walker, robots: u16) -> SockRound {
        SockRound { stage, walker, robots, max_pairs: 20 }
    }

    #[test]
    fn the_order_reads_naturally() {
        assert_eq!(order_line(&round(SockStage::Feet, Walker::Tripod, 1)), "A tripod needs socks!");
        assert_eq!(order_line(&round(SockStage::Group, Walker::DogBot, 3)), "3 dog-bots need socks!");
        assert_eq!(pairs_label(1), "1 pair");
        assert_eq!(pairs_label(0), "0 pairs");
    }

    #[test]
    fn every_foot_has_a_spot_in_the_scene() {
        for (stage, walker, n) in [(SockStage::Feet, Walker::StarBot, 1), (SockStage::Group, Walker::BugBot, 3), (SockStage::Order, Walker::CentiBot, 9)] {
            let r = round(stage, walker, n);
            let l = shop_layout(&r, 420.0, 260.0);
            let want = robots_drawn(&r) * walker.feet();
            assert_eq!(l.feet.len() as u16, want);
            assert!(l.feet.iter().all(|&(x, _)| x > 0.0 && x < l.counter.0), "feet stay left of the counter: {l:?}");
        }
    }

    #[test]
    fn keys_drive_the_drawer() {
        let s = SockSession::new(round(SockStage::Group, Walker::DogBot, 2));
        assert_eq!(handle_key(&FrameInput::empty().with_key_pressed(KeyCode::Up), &s), Some(SockInput::Add(1)));
        assert_eq!(handle_key(&FrameInput::empty().with_key_pressed(KeyCode::Left), &s), Some(SockInput::Add(-1)));
        assert_eq!(handle_key(&FrameInput::empty().with_key_pressed(KeyCode::Enter), &s), Some(SockInput::Send));
        assert_eq!(handle_key(&FrameInput::empty().with_key_pressed(KeyCode::Escape), &s), Some(SockInput::Leave));
    }
}
