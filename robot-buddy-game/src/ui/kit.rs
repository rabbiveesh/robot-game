//! Bevy UI plumbing every panel shares: turning views into nodes, taps into
//! [`UiAction`]s, art slots into offscreen cameras, and text that has to fit.
//!
//! ```text
//!  Update                           PostUpdate (Bevy UI layout)        next PreUpdate
//!  ──────                           ───────────────────────────        ──────────────
//!  UiViews ──► reconcile::<P> ──►  nodes laid out + drawn by Bevy ──► Pointer<Press> ──► Pressed
//!   (from Game)  (respawn a panel   shrink_text: fonts step down        art_targets: slot sizes
//!                 when its view      until the text fits                 ──► FrameInput.art
//!                 changes)
//! ```
//!
//! Panels are pure functions from a view to a bundle; nothing here knows any
//! particular panel.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget, ScalingMode};
use bevy::picking::events::{Pointer, Press};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::text::{LineBreak, LineHeight, TextLayoutInfo};
use bevy::ui::UiSystems;
use bevy::window::WindowResized;

use super::views::UiViews;
use super::{ArtId, UiAction};
use crate::draw;

// ─── Panels ─────────────────────────────────────────────

/// One panel: how to find its view in [`UiViews`] and how to build it.
/// Building is pure — a bundle from a view — so tests can spawn a panel into
/// a bare `World` and look for its buttons.
pub trait Panel: Send + Sync + 'static {
    type View: PartialEq + Clone + Send + Sync + 'static;
    /// Stacking order among panels (higher draws on top).
    const Z: i32;
    fn view(views: &UiViews) -> Option<&Self::View>;
    fn bundle(view: &Self::View) -> impl Bundle;
    /// Systems for the panel's per-frame bits (a typewriter…), if any.
    fn setup(_app: &mut App) {}
}

/// The root of a spawned panel, holding the view it was built from.
#[derive(Component)]
pub struct PanelRoot<P: Panel>(pub P::View);

/// Keep panel `P` in step with its view: spawn it when it appears, despawn
/// it when it goes, rebuild it whenever the view changes. Views only change
/// on a game event (a new question, a tap), never per frame.
pub fn reconcile<P: Panel>(
    mut commands: Commands,
    views: Res<UiViews>,
    roots: Query<(Entity, &PanelRoot<P>)>,
) {
    let want = P::view(&views);
    let mut have = roots.iter();
    match (have.next(), want) {
        (Some((_, root)), Some(v)) if &root.0 == v => {}
        (old, v) => {
            if let Some((e, _)) = old {
                commands.entity(e).despawn();
            }
            if let Some(v) = v {
                commands.spawn((PanelRoot::<P>(v.clone()), GlobalZIndex(P::Z), P::bundle(v)));
            }
        }
    }
}

pub trait AddPanel {
    fn add_panel<P: Panel>(&mut self) -> &mut Self;
}

impl AddPanel for App {
    fn add_panel<P: Panel>(&mut self) -> &mut Self {
        P::setup(self);
        self.add_systems(Update, reconcile::<P>.in_set(PanelSystems))
    }
}

/// Every panel's reconcile system (runs after the game steps).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PanelSystems;

// ─── Taps ───────────────────────────────────────────────

/// What a tap on this node means to the game.
#[derive(Component, Clone, Copy, Debug)]
#[require(Button)]
pub struct Action(pub UiAction);

/// The tap the next `FrameInput` carries.
#[derive(Resource, Default)]
pub struct Pressed {
    pub action: Option<UiAction>,
    /// Any panel node was under the press.
    pub on_ui: bool,
}

/// The innermost node with an [`Action`] under a press wins: a press on a
/// button's label bubbles up to the button.
fn on_press(
    mut press: On<Pointer<Press>>,
    actions: Query<&Action>,
    nodes: Query<(), With<Node>>,
    mut pressed: ResMut<Pressed>,
) {
    if press.event.button != PointerButton::Primary {
        return;
    }
    let target = press.event_target();
    if nodes.contains(target) {
        pressed.on_ui = true;
    }
    if let Ok(a) = actions.get(target) {
        pressed.action.get_or_insert(a.0);
        press.propagate(false);
    }
}

/// Background colours for a button at rest and under the pointer.
#[derive(Component, Clone, Copy)]
pub struct Tint {
    pub idle: Color,
    pub hot: Color,
}

fn tint(mut q: Query<(&Interaction, &Tint, &mut BackgroundColor), Changed<Interaction>>) {
    for (i, t, mut bg) in &mut q {
        bg.0 = if *i == Interaction::None { t.idle } else { t.hot };
    }
}

// ─── Text that fits ─────────────────────────────────────

/// Text that steps its font down (to `min`) until it fits. Bevy wraps text
/// but never shrinks it, and a kid's answer button can't wrap "1,000".
///
/// Across: a one-line text shrinks to its box's width; with `wrap`, once at
/// `min` it wraps instead. Down: a `squeeze` text also shrinks while the box
/// holding it spills out of the bottom (past its max height) — a long
/// speech in the dialogue box, a word problem over the visual.
#[derive(Component, Clone, Copy, Debug)]
pub struct Shrink {
    pub max: f32,
    pub min: f32,
    /// With `wrap`: a line too wide at this size wraps rather than shrinking
    /// further (only `squeeze` takes it on down to `min`).
    pub wrap_at: f32,
    pub wrap: bool,
    /// Wraps from the start, rather than shrinking a line first.
    pub starts_wrapped: bool,
    pub squeeze: bool,
}

/// Fonts only go down while a box is too small; a resize starts every
/// shrinking text over from full size. A text's spans (the typewriter's)
/// shrink with it.
fn shrink_text(
    mut resized: MessageReader<WindowResized>,
    mut q: Query<
        (&Shrink, &mut TextFont, &mut TextLayout, &mut Node, &TextLayoutInfo, &ComputedNode, &mut Visibility, Option<&Children>, &ChildOf),
        Without<TextSpan>,
    >,
    mut spans: Query<&mut TextFont, With<TextSpan>>,
    boxes: Query<&ComputedNode, Without<Shrink>>,
) {
    let reset = resized.read().count() > 0;
    for (s, mut font, mut layout, mut style, info, node, mut vis, children, parent) in &mut q {
        let mut set = |font: &mut TextFont, size: f32| {
            font.font_size = FontSize::Px(size);
            for &c in children.into_iter().flatten() {
                if let Ok(mut f) = spans.get_mut(c) {
                    f.font_size = FontSize::Px(size);
                }
            }
        };
        if reset {
            set(&mut font, s.max);
            if !s.starts_wrapped {
                layout.linebreak = LineBreak::NoWrap;
                style.width = Val::Auto;
            }
            continue;
        }
        let FontSize::Px(size) = font.font_size else { continue };
        let (need, have) = (info.size, node.size());
        if have.x <= 0.0 {
            vis.set_if_neq(Visibility::Inherited);
            continue;
        }
        // Across: a line too wide for its box.
        if layout.linebreak == LineBreak::NoWrap && need.x > have.x + 0.5 {
            let floor = if s.wrap { s.wrap_at } else { s.min };
            if size > floor {
                set(&mut font, (size * have.x / need.x).floor().max(floor));
                continue;
            }
            if s.wrap {
                // Wrapped text takes the whole width it's given (left to
                // itself it would shrink to its longest word).
                layout.linebreak = LineBreak::WordBoundary;
                style.width = percent(100);
                continue;
            }
        }
        // Down: the box holding it spills.
        if s.squeeze && size > s.min {
            let over = boxes.get(parent.parent()).map_or(0.0, |b| b.content_size().y - b.size().y);
            if over > 0.5 {
                // Area goes with the square of the size.
                let fit = ((need.y - over).max(1.0) / need.y.max(1.0)).sqrt();
                set(&mut font, (size * fit).floor().clamp(s.min, size - 1.0));
                continue;
            }
        }
        vis.set_if_neq(Visibility::Inherited);
    }
}

/// Blinks at `hz` (a "SPACE >" prompt waiting for the kid).
#[derive(Component, Clone, Copy)]
pub struct Blink(pub f32);

fn blink(time: Res<Time>, mut q: Query<(&Blink, &mut Visibility)>) {
    let t = time.elapsed_secs();
    for (b, mut vis) in &mut q {
        vis.set_if_neq(if (t * b.0).sin() > 0.0 { Visibility::Inherited } else { Visibility::Hidden });
    }
}

// ─── Art slots ──────────────────────────────────────────

/// A node `Game::render` paints procedural art into.
#[derive(Component, Clone, Copy, Debug)]
pub struct Art(pub ArtId);

/// An art slot whose height follows its width: `h(w)` logical pixels tall
/// at width `w` (the challenge visual is as tall as its dots need at the
/// width the panel gives it). Settles a frame after the width does.
#[derive(Component, Clone)]
pub struct HeightFor(pub std::sync::Arc<dyn Fn(f32) -> f32 + Send + Sync>);

impl HeightFor {
    pub fn new(h: impl Fn(f32) -> f32 + Send + Sync + 'static) -> Self {
        HeightFor(std::sync::Arc::new(h))
    }
}

fn height_for(mut q: Query<(&HeightFor, &ComputedNode, &mut Node)>) {
    for (f, computed, mut node) in &mut q {
        let w = computed.size().x * computed.inverse_scale_factor();
        let h = (f.0)(w).max(0.0);
        if node.height != px(h) && (node.height == Val::Auto || (computed.size().y * computed.inverse_scale_factor() - h).abs() > 0.5) {
            node.height = px(h);
        }
    }
}

/// The camera and texture behind one art slot.
#[derive(Component)]
struct ArtTarget {
    camera: Entity,
    size: UVec2,
}

/// The image node showing a slot's texture.
#[derive(Component)]
struct ArtImage;

/// Present when nothing will be drawn (a headless probe): art slots are
/// measured but get no texture.
#[derive(Resource)]
pub struct NoArtTextures;

/// Art slots on screen, logical sizes — read into the next `FrameInput`.
#[derive(Resource, Default)]
pub struct ArtSizes(pub Vec<(ArtId, f32, f32)>);

/// Give every art slot an offscreen camera on its own render layer, drawing
/// into a texture the slot shows. A slot's texture follows its laid-out size.
fn art_targets(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    slots: Query<(Entity, &Art, &ComputedNode, Option<&ArtTarget>, Option<&Children>)>,
    mut shown: Query<&mut ImageNode, With<ArtImage>>,
    mut cameras: Query<(&mut RenderTarget, &mut Projection)>,
    mut sizes: ResMut<ArtSizes>,
    headless: Option<Res<NoArtTextures>>,
) {
    sizes.0.clear();
    for (e, art, node, target, children) in &slots {
        let px = node.size().round().as_uvec2().max(UVec2::ONE);
        let logical = node.size() * node.inverse_scale_factor();
        sizes.0.push((art.0, logical.x, logical.y));
        if headless.is_some() || target.is_some_and(|t| t.size == px) {
            continue;
        }
        let mut image = Image::new_target_texture(px.x, px.y, TextureFormat::Rgba8UnormSrgb, None);
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let handle = images.add(image);
        let projection = Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed { width: logical.x, height: logical.y },
            ..OrthographicProjection::default_2d()
        });
        let render_target = RenderTarget::Image(ImageRenderTarget::from(handle.clone()));
        match target {
            Some(t) => {
                if let Ok((mut rt, mut proj)) = cameras.get_mut(t.camera) {
                    *rt = render_target;
                    *proj = projection;
                }
                for &c in children.into_iter().flatten() {
                    if let Ok(mut img) = shown.get_mut(c) {
                        img.image = handle.clone();
                    }
                }
                commands.entity(e).insert(ArtTarget { camera: t.camera, size: px });
            }
            None => {
                let camera = commands
                    .spawn((
                        Camera2d,
                        Camera {
                            order: -1,
                            clear_color: ClearColorConfig::Custom(Color::NONE),
                            ..default()
                        },
                        render_target,
                        projection,
                        RenderLayers::layer(art.0.layer() as usize),
                    ))
                    .id();
                commands.entity(e).insert(ArtTarget { camera, size: px }).with_child((
                    ArtImage,
                    ImageNode::new(handle),
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
        }
    }
}

/// A slot going away takes its camera with it.
fn drop_art_camera(remove: On<Remove, ArtTarget>, targets: Query<&ArtTarget>, mut commands: Commands) {
    if let Ok(t) = targets.get(remove.entity) {
        commands.entity(t.camera).despawn();
    }
}

// ─── Plugin ─────────────────────────────────────────────

pub struct UiKitPlugin;

impl Plugin for UiKitPlugin {
    fn build(&self, app: &mut App) {
        // DefaultPlugins bring it; a headless app may not.
        if !app.is_plugin_added::<bevy::ui_widgets::ScrollAreaPlugin>() {
            app.add_plugins(bevy::ui_widgets::ScrollAreaPlugin);
        }
        app.init_resource::<UiViews>()
            .init_resource::<Pressed>()
            .init_resource::<ArtSizes>()
            .init_resource::<ScrollMemory>()
            .add_observer(on_press)
            .add_observer(restore_scroll)
            .add_observer(drop_art_camera)
            .add_observer(drag_to_scroll)
            .add_systems(Update, (tint, blink))
            .add_systems(PostUpdate, remember_scroll.before(UiSystems::Prepare))
            .add_systems(PostUpdate, (shrink_text, height_for, art_targets).after(UiSystems::PostLayout));
    }
}

/// A scrolling list that keeps its place: the panel it's in is rebuilt
/// whenever its view changes (a tap on a row does that), and the kid
/// shouldn't be thrown back to the top each time. `key` names the list.
pub fn scroll_list(key: &'static str, gap: f32) -> impl Bundle {
    let list = Node { flex_grow: 1.0, min_height: px(0), row_gap: px(gap), overflow: Overflow::scroll_y(), ..col() };
    (list, bevy::ui_widgets::ScrollArea, ScrollKey(key))
}

#[derive(Component, Clone, Copy)]
pub struct ScrollKey(pub &'static str);

/// Where each open list was scrolled to.
#[derive(Resource, Default)]
struct ScrollMemory(std::collections::HashMap<&'static str, Vec2>);

/// Note where every list is; forget lists that closed (a reopened shop
/// starts at the top).
fn remember_scroll(lists: Query<(&ScrollKey, &ScrollPosition)>, mut mem: ResMut<ScrollMemory>) {
    mem.0.retain(|k, _| lists.iter().any(|(key, _)| key.0 == *k));
    for (key, pos) in &lists {
        mem.0.insert(key.0, pos.0);
    }
}

/// A rebuilt list picks up where the old one was.
fn restore_scroll(add: On<Add, ScrollKey>, mut lists: Query<(&ScrollKey, &mut ScrollPosition)>, mem: Res<ScrollMemory>) {
    if let Ok((key, mut pos)) = lists.get_mut(add.entity) {
        if let Some(&at) = mem.0.get(key.0) {
            pos.0 = at;
        }
    }
}

/// A finger dragging a scroll area scrolls it (the mouse wheel is handled by
/// `ScrollArea` itself).
fn drag_to_scroll(
    mut drag: On<Pointer<bevy::picking::events::Drag>>,
    mut areas: Query<(&ComputedNode, &mut ScrollPosition), With<bevy::ui_widgets::ScrollArea>>,
) {
    if let Ok((node, mut pos)) = areas.get_mut(drag.event_target()) {
        drag.propagate(false);
        let max = ((node.content_size() - node.size()) * node.inverse_scale_factor()).max(Vec2::ZERO);
        pos.y = (pos.y - drag.event.delta.y).clamp(0.0, max.y);
    }
}

// ─── Building blocks ────────────────────────────────────

/// Our colour as Bevy's.
pub fn color(c: draw::Color) -> Color {
    Color::srgba(c.r, c.g, c.b, c.a)
}

/// A text node at `size` px. Text wraps at word boundaries (Bevy's default).
pub fn text(s: impl Into<String>, size: f32, c: draw::Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont::from_font_size(size),
        TextColor(color(c)),
        LineHeight::RelativeToFont(1.15),
        Pickable::IGNORE,
    )
}

/// Centred text that wraps.
pub fn text_centered(s: impl Into<String>, size: f32, c: draw::Color) -> impl Bundle {
    (text(s, size, c), TextLayout::new(Justify::Center, LineBreak::WordBoundary))
}

/// One line that shrinks (to `min`) to fit its box rather than overflow.
/// `wrap`: once at `min`, wrap instead.
pub fn fitted(s: impl Into<String>, size: f32, min: f32, wrap: bool, c: draw::Color, justify: Justify) -> impl Bundle {
    fitted_by(s, c, justify, Shrink { max: size, min, wrap_at: min, wrap, starts_wrapped: false, squeeze: false })
}

fn fitted_by(s: impl Into<String>, c: draw::Color, justify: Justify, shrink: Shrink) -> impl Bundle {
    // A text box hugs its text, so justifying the lines doesn't place the
    // box: in a column, centre (or right-align) the box too.
    let align_self = match justify {
        Justify::Center => AlignSelf::Center,
        Justify::Right | Justify::End => AlignSelf::FlexEnd,
        _ => AlignSelf::Auto,
    };
    (
        Text::new(s),
        TextFont::from_font_size(shrink.max),
        TextColor(color(c)),
        LineHeight::RelativeToFont(1.15),
        TextLayout::new(justify, LineBreak::NoWrap),
        shrink,
        // Hidden while it settles (a frame or two), so the kid never sees it
        // overflow first.
        // At most as wide as its parent, so "too wide" is measurable.
        Node { max_width: percent(100), min_width: px(0), align_self, ..default() },
        Visibility::Hidden,
        Pickable::IGNORE,
    )
}

/// Like [`fitted`] with `wrap`, and it also shrinks while the box holding
/// it spills out of the bottom (give that box a max height). A short line
/// ("125 + 378 = ?") shrinks a little to stay on one line; a word problem
/// wraps at a readable size first and only shrinks if it then spills.
pub fn fitted_squeezed(s: impl Into<String>, size: f32, min: f32, c: draw::Color, justify: Justify) -> impl Bundle {
    let wrap_at = (size * 0.6).round().max(min);
    fitted_by(s, c, justify, Shrink { max: size, min, wrap_at, wrap: true, starts_wrapped: false, squeeze: true })
}

/// Wrapped text that shrinks (to `min`) until the box holding it stops
/// overflowing (give that box a max height).
pub fn filled(s: impl Into<String>, size: f32, min: f32, c: draw::Color, justify: Justify) -> impl Bundle {
    (
        Text::new(s),
        TextFont::from_font_size(size),
        TextColor(color(c)),
        LineHeight::RelativeToFont(1.15),
        TextLayout::new(justify, LineBreak::WordBoundary),
        Shrink { max: size, min, wrap_at: min, wrap: true, starts_wrapped: true, squeeze: true },
        Node { width: percent(100), min_width: px(0), ..default() },
        Visibility::Hidden,
        Pickable::IGNORE,
    )
}

/// The whole screen, `alpha`-dimmed, with its one child centred inside a
/// `margin`. Every modal panel sits in one.
pub fn backdrop(alpha: f32, margin: f32) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(margin)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, alpha)),
    )
}

/// A tappable box carrying `action`: `node` (size, padding, radius…) with a
/// fill and a 2px outline (unless `node` sets its own border).
pub fn button(action: UiAction, mut node: Node, fill: draw::Color, outline: draw::Color) -> impl Bundle {
    if node.border == UiRect::DEFAULT {
        node.border = UiRect::all(px(2));
    }
    (Action(action), node, BackgroundColor(color(fill)), BorderColor::all(color(outline)))
}

/// A filled, outlined box (a panel or a row): `node` plus its colours.
pub fn boxed(mut node: Node, fill: draw::Color, border: f32, outline: draw::Color) -> impl Bundle {
    node.border = UiRect::all(px(border));
    (node, BackgroundColor(color(fill)), BorderColor::all(color(outline)))
}

/// A column (Bevy's `Node` defaults to a row).
pub fn col() -> Node {
    Node { flex_direction: FlexDirection::Column, ..default() }
}

/// A box whose one child is centred both ways (a button's label).
pub fn centered() -> Node {
    Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }
}
