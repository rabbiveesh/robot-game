//! Panels and HUD.
//!
//! Panels that are Bevy UI (dialogue, challenge, shop, swag, quest, settings,
//! leap, descent, the shooter's footer) are written in two halves: a pure
//! *view* the game builds from its state (`Game::ui_views`), and a *bundle*
//! function turning that view into Bevy UI nodes. Bevy lays them out, draws
//! them, and turns taps into [`UiAction`]s that `Game::step` reads from
//! `FrameInput`. Art inside a panel (the challenge visual, the kid in the
//! shop mirror…) is drawn by `Game::render` into an [`ArtId`] slot. See
//! `docs/adr/006-bevy-ui-panels.md`.
//!
//! The rest (HUD, title screen, the logic-puzzle boards) still draw by hand
//! through `crate::draw`.

pub mod kit;
pub mod probe;
pub mod views;
pub mod metrics;
pub mod rect;
pub mod dialogue;
pub mod challenge;
pub mod balance;
pub mod kenken;
pub mod leap;
pub mod patterns;
pub mod quest;
pub mod shooter;
pub mod shop;
pub mod sudoku;
pub mod descent;
pub mod swag;
pub mod swatches;
pub mod visuals;
pub mod title_screen;
pub mod hud;
pub mod interaction_menu;
pub mod settings_overlay;

/// A tap on a panel button, as `Game::step` reads it. Each panel names its
/// own buttons; the tap carries the name, never coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAction {
    Challenge(challenge::ChallengeId),
    Shop(shop::ShopId),
    Swag(swag::SwagId),
    Quest(quest::QuestId),
    Settings(settings_overlay::SettingsId),
    Shooter(shooter::FooterId),
    Leap(leap::LeapId),
    Descent(descent::DescentId),
}

/// A slot inside a panel that `Game::render` paints procedural art into.
/// Each has its own render layer (and offscreen camera), so art always sits
/// on its own panel and under anything stacked above it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArtId {
    /// The CRA visual (dots, number line, base-ten blocks…).
    ChallengeVisual,
    /// Stars circling the praise.
    ChallengeStars,
    /// The kid in Bolt's mirror while they pick an outfit colour.
    ShopPreview,
    /// The pile of pearls at the trade desk, in rows of the rate.
    ShopPile,
    /// The buddy being dressed up.
    SwagPreview,
    /// The dive shaft: a vertical number line with the diver on it.
    DescentShaft,
}

impl ArtId {
    pub const ALL: [ArtId; 6] = [
        ArtId::ChallengeVisual,
        ArtId::ChallengeStars,
        ArtId::ShopPreview,
        ArtId::ShopPile,
        ArtId::SwagPreview,
        ArtId::DescentShaft,
    ];

    /// The render layer this slot's art is drawn on (0 is the screen).
    pub fn layer(self) -> u8 {
        ArtId::ALL.iter().position(|&a| a == self).unwrap() as u8 + 1
    }
}

/// Every Bevy UI panel, plus the kit they're built with. The game adds it;
/// so does [`probe::UiProbe`], so tests lay out exactly the game's panels.
pub struct PanelsPlugin;

impl bevy::app::Plugin for PanelsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        use kit::AddPanel;
        app.add_plugins(kit::UiKitPlugin)
            .add_panel::<dialogue::DialoguePanel>()
            .add_panel::<challenge::ChallengePanel>()
            .add_panel::<settings_overlay::SettingsPanel>()
            .add_panel::<quest::QuestPanel>()
            .add_panel::<shop::ShopPanel>()
            .add_panel::<swag::SwagPanel>()
            .add_panel::<leap::LeapPanel>()
            .add_panel::<descent::DescentPanel>()
            .add_panel::<shooter::FooterPanel>();
    }
}
