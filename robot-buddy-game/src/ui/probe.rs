//! Lay the panels out headlessly and see where everything landed.
//!
//! A [`UiProbe`] is a windowless Bevy app with the game's panels
//! ([`super::PanelsPlugin`]) and the bundled font: hand it a [`UiViews`] and
//! it runs Bevy's own layout and returns a [`Snapshot`] of every button and
//! line of text, in screen pixels. Tests tap buttons by name through it (so a
//! tap asserts the button is really on screen) and sweep panels for the few
//! ways a layout goes wrong ([`Snapshot::problems`]).

use bevy::camera::{ComputedCameraValues, RenderTargetInfo, Viewport};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, OverflowAxis, UiGlobalTransform};
use bevy::window::WindowResolution;

use super::kit::{Action, Art, Shrink};
use super::views::UiViews;
use super::{ArtId, UiAction};

/// A screen rectangle, logical pixels, top-left origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
    fn inside(&self, o: &Rect) -> bool {
        const EPS: f32 = 0.5;
        self.x >= o.x - EPS && self.y >= o.y - EPS && self.x + self.w <= o.x + o.w + EPS && self.y + self.h <= o.y + o.h + EPS
    }
    fn inside_x(&self, o: &Rect) -> bool {
        Rect { y: o.y, h: o.h, ..*self }.inside(o)
    }
    /// The part of `self` inside `o` (zero-sized when they don't meet).
    fn clip(&self, o: &Rect) -> Rect {
        let (x0, y0) = (self.x.max(o.x), self.y.max(o.y));
        let (x1, y1) = ((self.x + self.w).min(o.x + o.w), (self.y + self.h).min(o.y + o.h));
        Rect { x: x0, y: y0, w: (x1 - x0).max(0.0), h: (y1 - y0).max(0.0) }
    }
    fn overlaps(&self, o: &Rect) -> bool {
        const EPS: f32 = 0.5;
        self.w > EPS && self.h > EPS && o.w > EPS && o.h > EPS
            && self.x + EPS < o.x + o.w && o.x + EPS < self.x + self.w && self.y + EPS < o.y + o.h && o.y + EPS < self.y + self.h
    }
}

/// A button where Bevy put it.
#[derive(Clone, Debug)]
pub struct Placed {
    pub action: UiAction,
    pub rect: Rect,
    /// The scroll area it sits in, if any: it may be scrolled out of sight.
    pub scroll: Option<Rect>,
    entity: Entity,
}

impl Placed {
    /// The part a finger can reach: inside its scroll area, if it has one.
    pub fn visible(&self) -> Rect {
        self.scroll.map_or(self.rect, |area| self.rect.clip(&area))
    }
}

/// A line (or block) of text where Bevy put it.
#[derive(Clone, Debug)]
pub struct PlacedText {
    pub text: String,
    /// The text's own box.
    pub rect: Rect,
    /// How far right the glyphs actually run, from the box's left edge.
    pub ink_w: f32,
    pub scroll: Option<Rect>,
    /// It's a button's label.
    pub on_button: bool,
}

/// Everything on screen after a layout.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub screen: Rect,
    pub buttons: Vec<Placed>,
    pub texts: Vec<PlacedText>,
    /// Art slots: what `Game::render` gets to paint into.
    pub arts: Vec<(ArtId, Rect)>,
    /// Boxes a kid can see (a fill or an outline) whose children run out
    /// of the bottom: the box's rect, and by how much. (Across, wrapped
    /// text counts the space it wrapped at, so that's left to the text
    /// and button checks.)
    pub spills: Vec<(Rect, f32)>,
}

impl Snapshot {
    pub fn button(&self, action: UiAction) -> Option<&Placed> {
        self.buttons.iter().find(|b| b.action == action)
    }

    pub fn art(&self, id: ArtId) -> Option<Rect> {
        self.arts.iter().find(|(a, _)| *a == id).map(|(_, r)| *r)
    }

    /// Everything a kid would see as broken: a button off screen or on top
    /// of another, text running out of its box or off screen. Empty when
    /// the layout is sound.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        for b in &self.buttons {
            let ok = match b.scroll {
                // In a scroll list: across, it must fit the list; down, the
                // kid scrolls to it. The list itself must be on screen.
                Some(area) => b.rect.inside_x(&area) && area.inside(&self.screen),
                None => b.rect.inside(&self.screen),
            };
            if !ok {
                out.push(format!("button {:?} at {:?} is off screen", b.action, b.rect));
            }
            if b.rect.w < 1.0 || b.rect.h < 1.0 {
                out.push(format!("button {:?} squashed to {:?}", b.action, b.rect));
            }
        }
        for (i, a) in self.buttons.iter().enumerate() {
            for b in &self.buttons[i + 1..] {
                let (a_r, b_r) = (a.visible(), b.visible());
                if a_r.overlaps(&b_r) && !a_r.inside(&b_r) && !b_r.inside(&a_r) {
                    out.push(format!("buttons {:?} and {:?} overlap ({a_r:?}, {b_r:?})", a.action, b.action));
                }
            }
        }
        for (r, y) in &self.spills {
            out.push(format!("box at {r:?} spills its content {y:.0}px out of the bottom"));
        }
        for (id, r) in &self.arts {
            if !r.inside(&self.screen) {
                out.push(format!("art {id:?} at {r:?} is off screen"));
            }
        }
        for t in &self.texts {
            if t.text.trim().is_empty() {
                continue;
            }
            if t.ink_w > t.rect.w + 1.0 {
                out.push(format!("text {:?} runs {:.0}px past its box {:?}", t.text, t.ink_w - t.rect.w, t.rect));
            }
            let ok = match t.scroll {
                Some(area) => t.rect.inside_x(&area),
                None => t.rect.inside(&self.screen),
            };
            if !ok {
                out.push(format!("text {:?} at {:?} is off screen", t.text, t.rect));
            }
            if !t.on_button {
                let seen = t.scroll.map_or(t.rect, |area| t.rect.clip(&area));
                for b in self.buttons.iter().filter(|b| b.visible().overlaps(&seen)) {
                    out.push(format!("text {:?} runs over button {:?}", t.text, b.action));
                }
            }
        }
        out
    }
}

/// A headless Bevy app with the game's panels, at one screen size.
pub struct UiProbe {
    app: App,
    screen: (f32, f32),
}

impl UiProbe {
    pub fn new(screen: (f32, f32)) -> Self {
        let (w, h) = (screen.0 as u32, screen.1 as u32);
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::window::WindowPlugin {
                primary_window: Some(Window { resolution: WindowResolution::new(w, h), ..default() }),
                ..default()
            },
            bevy::transform::TransformPlugin,
            bevy::input::InputPlugin,
            bevy::camera::CameraPlugin,
            bevy::image::ImagePlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::TextureAtlasPlugin,
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin,
            bevy::picking::DefaultPickingPlugins,
        ));
        app.world_mut()
            .resource_mut::<Assets<Font>>()
            .insert(AssetId::default(), Font::from_bytes(crate::text::FONT_BYTES.to_vec()))
            .expect("default font slot");
        app.add_plugins(super::PanelsPlugin).insert_resource(super::kit::NoArtTextures);
        // No window to render to, so the camera is told its size outright.
        let size = UVec2::new(w, h);
        app.world_mut().spawn((
            Camera2d,
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo { physical_size: size, scale_factor: 1.0 }),
                    ..default()
                },
                viewport: Some(Viewport { physical_size: size, ..default() }),
                ..default()
            },
        ));
        UiProbe { app, screen }
    }

    /// Show `views` and lay them out (until shrinking text has settled).
    pub fn show(&mut self, views: UiViews) -> Snapshot {
        *self.app.world_mut().resource_mut::<UiViews>() = views;
        for _ in 0..16 {
            self.app.update();
            let world = self.app.world_mut();
            let mut q = world.query_filtered::<&Visibility, With<Shrink>>();
            if q.iter(world).all(|v| *v != Visibility::Hidden) {
                break;
            }
        }
        // One more pass so anything sized off settled text (HeightFor) lands.
        self.app.update();
        self.snapshot()
    }

    /// Show `views`, scrolling `action`'s list (if it's in one) so the
    /// button is in sight, as a kid would before tapping it.
    pub fn show_for(&mut self, views: UiViews, action: UiAction) -> Snapshot {
        let snap = self.show(views);
        let Some(b) = snap.button(action) else { return snap };
        let Some(area) = b.scroll.filter(|a| !b.rect.inside(a)) else { return snap };
        let entity = b.entity;
        // Scroll so the button sits at the top of the list (or as far as it goes).
        let dy = b.rect.y - area.y;
        let world = self.app.world_mut();
        let mut parents = world.query::<&ChildOf>();
        let mut e = entity;
        let list = loop {
            let Ok(p) = parents.get(world, e) else { return snap };
            e = p.parent();
            if world.get::<Node>(e).is_some_and(|n| n.overflow.y == OverflowAxis::Scroll) {
                break e;
            }
        };
        if let Some(mut pos) = world.get_mut::<ScrollPosition>(list) {
            pos.y += dy;
        }
        self.app.update();
        self.app.update();
        self.snapshot()
    }

    /// The probe's Bevy world, to look at anything a snapshot leaves out.
    pub fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }

    fn snapshot(&mut self) -> Snapshot {
        let world = self.app.world_mut();
        let rect_of = |n: &ComputedNode, t: &UiGlobalTransform| {
            let s = n.size() * n.inverse_scale_factor();
            let c = t.translation * n.inverse_scale_factor();
            Rect { x: c.x - s.x / 2.0, y: c.y - s.y / 2.0, w: s.x, h: s.y }
        };
        // The scroll area above an entity, if any.
        let mut nodes = world.query::<(&Node, &ComputedNode, &UiGlobalTransform, Option<&ChildOf>)>();
        let mut scroll_of = |world: &World, mut e: Entity| -> Option<Rect> {
            loop {
                let (node, cn, t, parent) = nodes.get(world, e).ok()?;
                if node.overflow.y == OverflowAxis::Scroll {
                    return Some(rect_of(cn, t));
                }
                e = parent?.parent();
            }
        };
        let on_button = |world: &World, mut e: Entity| -> bool {
            while let Some(p) = world.get::<ChildOf>(e) {
                e = p.parent();
                if world.get::<Action>(e).is_some() {
                    return true;
                }
            }
            false
        };
        let mut found = Vec::new();
        let mut q = world.query::<(Entity, &Action, &ComputedNode, &UiGlobalTransform, &InheritedVisibility)>();
        for (e, a, n, t, vis) in q.iter(world) {
            if vis.get() {
                found.push((e, a.0, rect_of(n, t)));
            }
        }
        let mut lines = Vec::new();
        let mut q = world.query::<(Entity, &Text, &bevy::text::TextLayoutInfo, &ComputedNode, &UiGlobalTransform, &InheritedVisibility)>();
        for (e, text, info, n, t, vis) in q.iter(world) {
            if vis.get() {
                // Where the glyphs end (Bevy's own width counts the space a line
                // wrapped at, which nobody sees).
                let ink = info.glyphs.iter().map(|g| g.position.x + g.atlas_info.rect.width() / 2.0).fold(0.0, f32::max);
                lines.push((e, text.0.clone(), rect_of(n, t), ink * n.inverse_scale_factor()));
            }
        }
        let mut q = world.query::<(&Art, &ComputedNode, &UiGlobalTransform, &InheritedVisibility)>();
        let arts = q.iter(world).filter(|(.., v)| v.get()).map(|(a, n, t, _)| (a.0, rect_of(n, t))).collect();
        let mut q = world.query::<(
            &Node,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&BackgroundColor>,
            Option<&BorderColor>,
            Option<&bevy::ui::BackgroundGradient>,
        )>();
        let spills = q
            .iter(world)
            .filter(|(node, _, _, v, bg, border, gradient)| {
                let seen = bg.is_some_and(|c| c.0.alpha() > 0.0)
                    || (node.border != UiRect::ZERO && border.is_some_and(|b| b.top.alpha() > 0.0))
                    || gradient.is_some_and(|g| !g.0.is_empty());
                v.get() && seen && node.overflow.y != OverflowAxis::Scroll
            })
            .filter_map(|(_, n, t, ..)| {
                let over = (n.content_size().y - n.size().y) * n.inverse_scale_factor();
                (over > 1.0).then(|| (rect_of(n, t), over))
            })
            .collect();
        let mut buttons: Vec<Placed> = found
            .into_iter()
            .map(|(e, action, rect)| Placed { action, rect, scroll: scroll_of(world, e), entity: e })
            .collect();
        let texts = lines
            .into_iter()
            .map(|(e, text, rect, ink_w)| PlacedText { text, rect, ink_w, scroll: scroll_of(world, e), on_button: on_button(world, e) })
            .collect();
        buttons.sort_by(|a, b| (a.rect.y, a.rect.x).partial_cmp(&(b.rect.y, b.rect.x)).unwrap());
        let (w, h) = self.screen;
        Snapshot { screen: Rect { x: 0.0, y: 0.0, w, h }, buttons, texts, arts, spills }
    }
}
