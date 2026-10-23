//! What every Bevy UI panel shows this frame, as plain data.
//!
//! `Game::ui_views` fills a [`UiViews`] from game state each frame; each
//! panel's reconcile system compares its view with the one its nodes were
//! built from and rebuilds only when it changed. Being data, a `UiViews` can
//! be built headlessly (tests look for a panel's buttons in it) and painted
//! later (the screenshots example).

use bevy::prelude::Resource;

use super::challenge::ChallengeView;
use super::dialogue::DialogueView;
use super::settings_overlay::SettingsView;
use super::quest::QuestPanelView;
use super::shop::ShopPanelView;
use super::swag::SwagPanelView;
use super::descent::DescentView;
use super::leap::LeapView;
use super::ArtId;

#[derive(Resource, Default, Clone)]
pub struct UiViews {
    pub dialogue: Option<DialogueView>,
    /// How many characters of the dialogue line the typewriter has revealed.
    pub dialogue_reveal: usize,
    pub challenge: Option<ChallengeView>,
    pub settings: Option<SettingsView>,
    pub quest: Option<QuestPanelView>,
    pub shop: Option<ShopPanelView>,
    pub swag: Option<SwagPanelView>,
    pub leap: Option<LeapView>,
    pub descent: Option<DescentView>,
    pub shooter_footer: Option<super::shooter::FooterView>,
}

impl UiViews {
    /// Paint art slot `id` (`w`×`h`, the slot's own coordinates), from the
    /// views alone — so anything holding a `UiViews` can paint its art.
    pub fn draw_art(&self, id: ArtId, w: f32, h: f32) {
        let time = crate::draw::get_time() as f32;
        match id {
            ArtId::ChallengeVisual | ArtId::ChallengeStars => {
                if let Some(c) = &self.challenge {
                    super::challenge::draw_art(id, &c.challenge, w, h, time);
                }
            }
            ArtId::ShopPile | ArtId::ShopPreview => {
                if let Some(v) = &self.shop {
                    super::shop::draw_art(id, v, w, h);
                }
            }
            ArtId::SwagPreview => {
                if let Some(v) = &self.swag {
                    super::swag::draw_art(v);
                }
            }
            ArtId::DescentShaft => {
                if let Some(v) = &self.descent {
                    super::descent::draw_art(v, w, h, time);
                }
            }
        }
    }

    /// Record the art for every slot in `slots` (from `FrameInput::art`).
    pub fn draw_all_art(&self, slots: &[(ArtId, f32, f32)]) {
        for &(id, w, h) in slots {
            crate::draw::art(id.layer(), || self.draw_art(id, w, h));
        }
    }
}
