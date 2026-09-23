//! The two shop counters (Bolt's Dum Dum shop, Hermie's pearl shop and trade
//! desk), the purses they draw on, and the Give-Swag panel for handing shop
//! swag to a buddy.

use super::*;

impl Game {
    /// Everything the shop panel shows, borrowed from the live session.
    pub fn shop_model(&self) -> Option<ui::shop::ShopModel<'_>> {
        let ash = self.active_shop.as_ref()?;
        Some(ui::shop::ShopModel {
            shop: ash.shop,
            catalog: &ash.catalog,
            owned: &ash.owned,
            balance: self.balance_for(ash.shop.currency()),
            view: shop_view(ash, self.outfit_color(wardrobe::PLAYER)),
            message: ash.message.as_deref(),
            page: ash.page,
        })
    }

    /// The shop panel as laid out on `screen` — what's drawn and what's
    /// tappable (tests click through this, same as `step`).
    pub fn shop_layout(&self, screen: (f32, f32)) -> Option<ui::shop::ShopLayout> {
        self.shop_model().map(|m| ui::shop::layout(&m, screen))
    }

    pub(super) fn step_shop(&mut self, input: &FrameInput, screen: (f32, f32)) {
        let intent = {
            let Some(model) = self.shop_model() else { return };
            let layout = ui::shop::layout(&model, screen);
            if input.mouse_clicked {
                let (mx, my) = input.mouse_pos;
                ui::shop::handle_click(mx, my, &layout, &model.view)
            } else {
                ui::shop::handle_key(input, &model.view)
            }
        };
        let Some(intent) = intent else { return };

        match intent {
            ui::shop::ShopInput::Page(page) => {
                if let Some(ash) = self.active_shop.as_mut() {
                    ash.page = page;
                }
            }
            ui::shop::ShopInput::Close => {
                // "Done" dismisses the nearest thing: the color picker if it's
                // up, otherwise the whole shop.
                let ash = self.active_shop.as_mut().unwrap();
                if ash.picking_color {
                    ash.picking_color = false;
                    ash.message = None;
                    return;
                }
                // Purchases went straight into the wardrobe and upgrades as
                // they settled, so closing has nothing to hand back.
                self.active_shop = None;
                self.set_state(GameState::Playing);
            }
            ui::shop::ShopInput::SelectItem(i) => {
                // Balances read before the session borrow so the purchase
                // branch below can use them without fighting the borrowck.
                let purse = {
                    let shop = self.active_shop.as_ref().unwrap().shop;
                    self.balance_for(shop.currency())
                };
                let trade_purse = match self.active_shop.as_ref().unwrap().catalog[i].kind {
                    ItemKind::Trade { .. } => purse,
                    _ => 0,
                };
                let ash = self.active_shop.as_mut().unwrap();
                if ash.selected.is_some() || ash.picking_color {
                    return; // already solving a purchase or picking a color
                }
                let item = ash.catalog[i].clone();
                // An owned Color Change re-opens the picker — buying it once
                // means you get to change colors whenever you like.
                if item.id == domain_shop::COLOR_CHANGE && ash.owned.contains(&item.id) {
                    ash.picking_color = true;
                    ash.message = None;
                    return;
                }
                // The trade desk isn't a purchase, it's a conversion: hand
                // over the pile and work out what it's worth.
                if let ItemKind::Trade { rate, into } = item.kind {
                    let quote = domain_shop::quote_trade(trade_purse, item.currency, rate, into);
                    if quote.gain == 0 {
                        ash.message = Some(format!(
                            "Not enough for a {} yet — you need {} more {}!",
                            into.singular(), quote.short_by(), item.currency.noun(quote.short_by()),
                        ));
                        return;
                    }
                    ash.selected = Some(i);
                    ash.answer = quote.gain;
                    ash.message = None;
                    ash.choices = division_choices(quote.gain, quote.offered, &mut self.rng);
                    ash.trading = Some(quote);
                    return;
                }
                let balance = purse;
                match domain_shop::process_purchase(balance, &item.id, &ash.owned) {
                    domain_shop::PurchaseOutcome::Bought { result } => {
                        ash.selected = Some(i);
                        ash.cost = result.spent;
                        ash.answer = result.new_balance;
                        ash.balance_before = balance;
                        ash.message = None;
                        let choices = subtraction_choices(balance, result.spent, &mut self.rng);
                        ash.choices = choices;
                    }
                    domain_shop::PurchaseOutcome::CantAfford { shortfall } => {
                        ash.message = Some(format!(
                            "You need {shortfall} more {}!", item.currency.noun(shortfall),
                        ));
                    }
                    domain_shop::PurchaseOutcome::AlreadyOwned => {
                        ash.message = Some(match item.kind {
                            // A perk is carried, not worn — say it's working.
                            ItemKind::Upgrade => format!("The {} is already yours!", item.name),
                            // You can only wear one of each — but give it to a
                            // buddy and the counter will happily sell another.
                            _ => "You're already wearing that one!".into(),
                        });
                    }
                    domain_shop::PurchaseOutcome::UnknownItem => {}
                }
            }
            ui::shop::ShopInput::Answer(v) => {
                // Resolve the guess on the shop session, then drop that borrow
                // before touching `self` (balances, events, save).
                enum Settled {
                    Bought { item: ShopItem, spent: u32, left: u32 },
                    Traded(domain_shop::TradeQuote),
                }
                let settled = {
                    let ash = self.active_shop.as_mut().unwrap();
                    let Some(i) = ash.selected else { return };
                    if v != ash.answer {
                        // Natural consequence, not punishment — recount and retry.
                        ash.message = Some("Hmm, let me count again...".into());
                        None
                    } else if let Some(quote) = ash.trading.take() {
                        ash.selected = None;
                        ash.choices.clear();
                        ash.message = Some(if quote.left_over > 0 {
                            format!(
                                "{}, and {} back in your pocket!",
                                quote.into.count(quote.gain), quote.from.count(quote.left_over),
                            )
                        } else {
                            format!("{}, spot on!", quote.into.count(quote.gain))
                        });
                        Some(Settled::Traded(quote))
                    } else {
                        let item = ash.catalog[i].clone();
                        ash.selected = None;
                        ash.choices.clear();
                        if item.id == domain_shop::COLOR_CHANGE {
                            // The fun part of Color Change is choosing — go
                            // straight to the swatches.
                            ash.picking_color = true;
                            ash.message = Some("You got it! Pick your color!".into());
                        } else if matches!(item.kind, ItemKind::Upgrade) {
                            // Say what it DOES, not just that it's bought — a
                            // perk with no visible effect is a mystery.
                            ash.message = Some(if item.blurb.is_empty() {
                                format!("The {} is yours for keeps!", item.name)
                            } else {
                                format!("{} — {}!", item.name, item.blurb)
                            });
                        } else {
                            ash.message = Some(format!("You look GREAT in the {}!", item.name));
                        }
                        let spent = ash.cost;
                        let left = ash.answer;
                        Some(Settled::Bought { item, spent, left })
                    }
                };

                let Some(settled) = settled else { return };
                match settled {
                    Settled::Bought { item, spent, left } => {
                        debug_assert_eq!(self.balance_for(item.currency), left + spent);
                        self.spend(item.currency, spent, item.id.clone());
                        // Swag goes on the kid; upgrades are banked rather than
                        // worn — they're perks, not outfits, and can't be
                        // handed to a buddy.
                        match item.kind {
                            ItemKind::Swag => {
                                self.dress(wardrobe::WardrobeAction::put_on(wardrobe::PLAYER, &item.id));
                            }
                            ItemKind::Upgrade => {
                                self.upgrades.insert(item.id.clone());
                            }
                            ItemKind::Trade { .. } => {}
                        }
                        // The shelf's "owned" marks are read from the real
                        // wardrobe, never kept as a second copy.
                        let owned = self.active_shop.as_ref().map(|ash| self.shop_owned_for(ash.shop));
                        if let (Some(ash), Some(owned)) = (self.active_shop.as_mut(), owned) {
                            ash.owned = owned;
                        }
                    }
                    Settled::Traded(quote) => {
                        let from = self.purse_mut(quote.from);
                        *from = from.saturating_sub(quote.spent);
                        let into = self.purse_mut(quote.into);
                        *into = into.saturating_add(quote.gain);
                        self.flash_purse(quote.from);
                        self.flash_purse(quote.into);
                        self.events.push(GameEvent::PearlsTraded {
                            pearls: quote.spent,
                            dum_dums: quote.gain,
                            left_over: quote.left_over,
                        });
                    }
                }

                // Persist immediately so the purchase (and the spent currency)
                // survive a reload even if the kid quits right now.
                self.persist();
            }
            ui::shop::ShopInput::PickColor(i) => {
                let Some((id, _)) = sprites::player::OUTFIT_COLORS.get(i) else { return };
                self.dress(wardrobe::WardrobeAction::set_color(wardrobe::PLAYER, id));
                self.events.push(GameEvent::OutfitColorPicked {
                    wearer: wardrobe::PLAYER.into(),
                    color: id.to_string(),
                });
                let ash = self.active_shop.as_mut().unwrap();
                ash.message = Some("Looking good!".into());
                // Persist right away, same as a purchase — the new outfit
                // should survive a reload even if the kid quits now.
                self.persist();
            }
        }
    }

    /// What a counter treats as already-bought: swag is "what the kid is
    /// wearing" (hand it to a buddy and it's for sale again), upgrades are
    /// "what they've bought" (permanent). Hermie sells both, so his shelf
    /// checks the union.
    pub(super) fn shop_owned_for(&self, shop: ShopKind) -> std::collections::BTreeSet<String> {
        let mut owned = self.player_swag().clone();
        if shop == ShopKind::Hermie {
            owned.extend(self.upgrades.iter().cloned());
        }
        owned
    }

    /// The purse a counter spends from.
    pub(super) fn balance_for(&self, currency: Currency) -> u32 {
        match currency {
            Currency::DumDums => self.dum_dums,
            Currency::Pearls => self.pearls,
        }
    }

    /// The one place a currency maps onto its field — every debit and credit
    /// goes through here, so nothing pays in pearls and debits Dum Dums.
    pub(super) fn purse_mut(&mut self, currency: Currency) -> &mut u32 {
        match currency {
            Currency::DumDums => &mut self.dum_dums,
            Currency::Pearls => &mut self.pearls,
        }
    }

    pub(super) fn flash_purse(&mut self, currency: Currency) {
        match currency {
            Currency::DumDums => self.dum_dum_hud.flash(),
            Currency::Pearls => self.pearl_hud.flash(),
        }
    }

    /// Pay `amount` for `item`: debit, flash the counter, log the spend.
    pub(super) fn spend(&mut self, currency: Currency, amount: u32, item: String) {
        let purse = self.purse_mut(currency);
        *purse = purse.saturating_sub(amount);
        self.flash_purse(currency);
        self.events.push(match currency {
            Currency::DumDums => GameEvent::DumDumsSpent { amount, item },
            Currency::Pearls => GameEvent::PearlsSpent { amount, item },
        });
    }

    /// Shop overlay, plus the live outfit preview while picking a color.
    pub(super) fn render_shop_overlay(&self, screen: (f32, f32)) {
        if let (Some(ash), Some(model)) = (self.active_shop.as_ref(), self.shop_model()) {
            let layout = ui::shop::layout(&model, screen);
            ui::shop::draw_shop(&model, &layout);

            // While picking an outfit color, show a live preview of the kid in
            // the panel's top-right so tapping swatches visibly recolors them.
            if let Some(r) = layout.preview() {
                let (px, py) = (r.x + 4.0, r.y + 14.0);
                match self.player_gender {
                    Gender::Boy => sprites::player::draw_player_boy(px, py, Dir::Down, 0, self.game_time),
                    Gender::Girl => sprites::player::draw_player_girl(px, py, Dir::Down, 0, self.game_time),
                }
                sprites::player::draw_player_cosmetics(px, py, Dir::Down, 0, &ash.owned,
                    self.outfit_color(wardrobe::PLAYER));
            }
        }
    }

    /// Open the Give-Swag panel for whoever the menu is talking to: to the
    /// list of what the kid is wearing, or (`recolor_only`, from a buddy's "New
    /// colour?") straight to that buddy's Color Change swatches.
    pub(super) fn open_swag(&mut self, recolor_only: bool) {
        // Sparky is a robot rather than a roster NPC, hence the sprite-less
        // preview; everyone else previews as themselves.
        let sprite = self.npcs.iter()
            .chain(self.companion.iter())
            .find(|n| n.id_str() == self.menu_target_id)
            .map(|n| n.sprite);
        self.active_swag = Some(ActiveSwag {
            recipient_id: self.menu_target_id.clone(),
            recipient_name: self.menu_target_name.clone(),
            recipient_sprite: sprite,
            items: self.swag_catalog_for(wardrobe::PLAYER),
            message: None,
            page: 0,
            picking_color: recolor_only,
            recolor_only,
        });
        self.set_state(GameState::Swag);
    }

    /// Everything the "Give Swag" panel shows, borrowed from the session.
    /// Handing a piece over moves it off the kid, so the list shrinks as they
    /// dress their buddy up — and Bolt is free to sell them another one.
    pub fn swag_model(&self) -> Option<ui::swag::SwagModel<'_>> {
        let asw = self.active_swag.as_ref()?;
        let picking = asw.picking_color.then(|| {
            let current = self.outfit_color(&asw.recipient_id);
            ui::swag::ColorPick {
                colors: sprites::player::OUTFIT_COLORS,
                current: sprites::player::OUTFIT_COLORS.iter().position(|(id, _)| *id == current).unwrap_or(0),
            }
        });
        Some(ui::swag::SwagModel {
            recipient: &asw.recipient_name,
            items: &asw.items,
            taken: self.wardrobe.worn_by(&asw.recipient_id),
            message: asw.message.as_deref(),
            page: asw.page,
            picking,
        })
    }

    /// The swag picker as laid out on `screen`.
    pub fn swag_layout(&self, screen: (f32, f32)) -> Option<ui::swag::SwagLayout> {
        self.swag_model().map(|m| ui::swag::layout(&m, screen))
    }

    pub(super) fn step_swag(&mut self, input: &FrameInput, screen: (f32, f32)) {
        let intent = {
            let Some(model) = self.swag_model() else { return };
            if input.mouse_clicked {
                let (mx, my) = input.mouse_pos;
                ui::swag::handle_click(mx, my, &ui::swag::layout(&model, screen))
            } else {
                ui::swag::handle_key(input, &model)
            }
        };
        let Some(intent) = intent else { return };
        let Some(asw) = self.active_swag.as_ref() else { return };

        match intent {
            ui::swag::SwagInput::Page(page) => {
                if let Some(asw) = self.active_swag.as_mut() {
                    asw.page = page;
                }
            }
            ui::swag::SwagInput::Close => {
                // Done backs out of the swatches to the list they came from;
                // opened for a recolour, there's no list to go back to.
                if asw.picking_color && !asw.recolor_only {
                    if let Some(asw) = self.active_swag.as_mut() {
                        asw.picking_color = false;
                        asw.message = None;
                    }
                    return;
                }
                self.active_swag = None;
                self.set_state(GameState::Playing);
            }
            ui::swag::SwagInput::PickColor(i) => {
                let Some((color, _)) = sprites::player::OUTFIT_COLORS.get(i) else { return };
                let (who, name) = (asw.recipient_id.clone(), asw.recipient_name.clone());
                self.dress(wardrobe::WardrobeAction::set_color(&who, color));
                self.events.push(GameEvent::OutfitColorPicked { wearer: who, color: color.to_string() });
                if let Some(asw) = self.active_swag.as_mut() {
                    asw.message = Some(format!("{name} looks great!"));
                }
                self.persist();
            }
            ui::swag::SwagInput::Give(i) => {
                let Some(item) = asw.items.get(i).cloned() else { return };
                let to = asw.recipient_id.clone();
                let name = asw.recipient_name.clone();
                let outcome = self.dress(wardrobe::WardrobeAction::hand_over(wardrobe::PLAYER, &to, &item.id));
                let message = match outcome {
                    HandOver::Given => {
                        self.events.push(GameEvent::SwagGiven {
                            item: item.id.clone(),
                            recipient: to.clone(),
                        });
                        audio::tts::speak(&name, &format!("Ooh! A {}! Thank you!", item.name));
                        // The fun of Color Change is choosing — so the buddy
                        // gets to choose too, starting from the colour it was.
                        if item.id == domain_shop::COLOR_CHANGE {
                            if let Some(asw) = self.active_swag.as_mut() {
                                asw.picking_color = true;
                            }
                        }
                        Some(format!("{name} puts on the {}!", item.name))
                    }
                    // Never a scolding — just a fact about their buddy.
                    HandOver::AlreadyWearing => Some(format!("{name} already has a {}!", item.name)),
                    HandOver::NotWorn => None,
                };
                let remaining = self.swag_catalog_for(wardrobe::PLAYER);
                if let Some(asw) = self.active_swag.as_mut() {
                    asw.items = remaining;
                    asw.message = message;
                }
                if outcome == HandOver::Given {
                    self.persist();
                }
            }
        }
    }

    /// Catalog entries for everything `who` is wearing, in catalog order so the
    /// picker rows are stable between openings.
    pub(super) fn swag_catalog_for(&self, who: &str) -> Vec<ShopItem> {
        let worn = self.wardrobe.worn_by(who);
        domain_shop::swag_items().into_iter().filter(|i| worn.contains(&i.id)).collect()
    }

    /// Give-Swag overlay, with the buddy previewed in their current outfit.
    pub(super) fn render_swag_overlay(&self, screen: (f32, f32)) {
        if let (Some(asw), Some(model)) = (self.active_swag.as_ref(), self.swag_model()) {
            let layout = ui::swag::layout(&model, screen);
            let taken = model.taken;
            ui::swag::draw(&model, &layout);
            // Live preview of the buddy in their current outfit, so handing
            // something over visibly lands on them.
            if let Some((px, py)) = layout.preview() {
                match asw.recipient_sprite {
                    Some(sprite) => {
                        sprite.draw_sprite(px, py, Dir::Down, self.game_time, false);
                        sprites::swag::draw_swag(px, py, Dir::Down, 0.0, taken,
                            self.outfit_color(&asw.recipient_id), sprite.swag_fit());
                    }
                    None => {
                        sprites::robot::draw_robot(px, py, Dir::Down, 0, self.game_time);
                        sprites::swag::draw_swag(px, py, Dir::Down, 0.0, taken,
                            self.outfit_color(&asw.recipient_id), sprites::swag::SwagFit::ROBOT);
                    }
                }
            }
        }
    }
}

/// Build the shop's current view (browsing, solving a purchase subtraction,
/// or picking an outfit color) from the active session. Borrows the session
/// so the layout/draw can read it.
fn shop_view<'a>(ash: &'a ActiveShop, color_choice: &str) -> ui::shop::ShopView<'a> {
    if ash.picking_color {
        let current = sprites::player::OUTFIT_COLORS
            .iter()
            .position(|(id, _)| *id == color_choice)
            .unwrap_or(0);
        return ui::shop::ShopView::PickingColor { colors: sprites::player::OUTFIT_COLORS, current };
    }
    if let Some(ref quote) = ash.trading {
        return ui::shop::ShopView::Trading { quote, choices: &ash.choices };
    }
    match ash.selected {
        Some(i) => ui::shop::ShopView::Buying {
            item: &ash.catalog[i],
            balance: ash.balance_before,
            cost: ash.cost,
            choices: &ash.choices,
        },
        None => ui::shop::ShopView::Browsing,
    }
}

/// Answer tiles for "balance − cost = ?": the correct remainder plus plausible
/// near-miss distractors (forgot to subtract, off-by-one), shuffled, all > 0
/// where possible. Always includes the right answer.
fn subtraction_choices(balance: u32, cost: u32, rng: &mut SmallRng) -> Vec<u32> {
    let answer = balance.saturating_sub(cost);
    let mut out = vec![answer];
    // Common slip-ups make the best distractors.
    for cand in [balance, answer + 1, answer.saturating_sub(1), answer + 2] {
        if out.len() >= 3 {
            break;
        }
        if !out.contains(&cand) {
            out.push(cand);
        }
    }
    out.shuffle(rng);
    out
}

/// Answer tiles for "how many groups of `rate` are in this pile?" — the right
/// quotient plus the near-misses a kid actually makes (one group out, or the
/// whole pile counted as singles).
fn division_choices(answer: u32, offered: u32, rng: &mut SmallRng) -> Vec<u32> {
    let mut out = vec![answer];
    for cand in [answer + 1, answer.saturating_sub(1), offered, answer + 2] {
        if out.len() >= 3 {
            break;
        }
        if !out.contains(&cand) {
            out.push(cand);
        }
    }
    out.shuffle(rng);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::InMemoryBackend;

    fn game() -> Game {
        Game::with_backend(7, Box::new(InMemoryBackend::default()))
    }

    // ── Handing swag over puts it back on Bolt's shelf ──
    #[test]
    fn giving_swag_away_lets_bolt_sell_another_one() {
        let mut g = game();
        g.dress(wardrobe::WardrobeAction::put_on(wardrobe::PLAYER, "hat"));
        assert_eq!(
            domain_shop::process_purchase(20, "hat", g.player_swag()),
            domain_shop::PurchaseOutcome::AlreadyOwned,
            "no buying a second hat while you're wearing one",
        );

        g.dress(wardrobe::WardrobeAction::hand_over(wardrobe::PLAYER, "kid_1", "hat"));
        assert!(
            matches!(domain_shop::process_purchase(20, "hat", g.player_swag()),
                domain_shop::PurchaseOutcome::Bought { .. }),
            "once the hat is Tali's, the kid can buy themselves another",
        );
    }

    // ── Color Change comes with a color picker ──

    const SCREEN: (f32, f32) = (960.0, 720.0);

    /// Open Bolt's shop directly (skipping the walk-and-talk).
    fn open_shop(g: &mut Game) {
        g.active_shop = Some(ActiveShop {
            shop: ShopKind::Bolt,
            trading: None,
            catalog: domain_shop::shop_catalog(),
            owned: g.player_swag().clone(),
            selected: None,
            choices: Vec::new(),
            answer: 0,
            cost: 0,
            balance_before: 0,
            message: None,
            source_npc: "shopkeeper".into(),
            picking_color: false,
            page: 0,
        });
        g.set_state(GameState::Shop);
    }

    /// Click whatever shop element sits at the center of `rect`.
    fn click_shop(g: &mut Game, rect: ui::shop::UiRect) {
        let click = crate::input::FrameInput::empty()
            .with_mouse_click(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        g.step(&click, 1.0 / 60.0, SCREEN);
    }

    fn shop_layout(g: &Game) -> ui::shop::ShopLayout {
        g.shop_layout(SCREEN).expect("shop should be open")
    }

    /// The catalog row for `id`.
    fn shop_row(g: &Game, id: &str) -> ui::shop::UiRect {
        let i = g.active_shop.as_ref().unwrap().catalog.iter().position(|it| it.id == id).expect("item in catalog");
        shop_layout(g).item(i).expect("row on screen")
    }

    #[test]
    fn buying_color_change_opens_the_picker_and_picking_sticks() {
        let mut g = game();
        g.dum_dums = 20;
        open_shop(&mut g);

        // Tap the Color Change row, then answer the purchase subtraction.
        let row = shop_row(&g, "color_change");
        click_shop(&mut g, row);
        let answer = g.active_shop.as_ref().unwrap().answer;
        let tile = {
            let ash = g.active_shop.as_ref().unwrap();
            shop_layout(&g).answer(&shop_view(ash, g.outfit_color(wardrobe::PLAYER)), answer).expect("correct answer tile")
        };
        click_shop(&mut g, tile);

        let ash = g.active_shop.as_ref().unwrap();
        assert!(ash.owned.contains("color_change"));
        assert!(ash.picking_color, "buying Color Change should open the picker");

        // Pick the second swatch; the kid's outfit color should change.
        let swatch = shop_layout(&g).swatch(1).unwrap();
        click_shop(&mut g, swatch);
        assert_eq!(g.outfit_color(wardrobe::PLAYER), sprites::player::OUTFIT_COLORS[1].0);

        // Done dismisses the picker but keeps the shop open.
        let close = shop_layout(&g).done().unwrap();
        click_shop(&mut g, close);
        let ash = g.active_shop.as_ref().unwrap();
        assert!(!ash.picking_color, "Done should close the picker first");
        assert!(g.active_shop.is_some(), "the shop itself should stay open");
    }

    #[test]
    fn changing_color_back_and_forth_sticks_each_time() {
        let mut g = game();
        g.dress(wardrobe::WardrobeAction::put_on(wardrobe::PLAYER, "color_change"));
        open_shop(&mut g);

        // Reopen the picker from the owned Color Change row.
        let row = shop_row(&g, "color_change");
        click_shop(&mut g, row);
        assert!(g.active_shop.as_ref().unwrap().picking_color);

        // Pick a sequence with repeats and back-tracking. Each pick must stick,
        // the picker must stay open, and the highlighted swatch must follow.
        for &i in &[1usize, 3, 6, 3, 1, 0, 6, 0] {
            let swatch = shop_layout(&g).swatch(i).unwrap();
            click_shop(&mut g, swatch);
            assert_eq!(g.outfit_color(wardrobe::PLAYER), sprites::player::OUTFIT_COLORS[i].0,
                "picking swatch {i} should set the kid's colour to {}", sprites::player::OUTFIT_COLORS[i].0);
            assert!(g.active_shop.as_ref().unwrap().picking_color,
                "picker should stay open so the kid can keep changing colors");
            match shop_view(g.active_shop.as_ref().unwrap(), g.outfit_color(wardrobe::PLAYER)) {
                ui::shop::ShopView::PickingColor { current, .. } =>
                    assert_eq!(current, i, "the highlighted swatch should track the latest pick"),
                _ => panic!("expected the PickingColor view while picking"),
            }
        }
    }

    #[test]
    fn owned_color_change_row_reopens_the_picker() {
        let mut g = game();
        g.dress(wardrobe::WardrobeAction::put_on(wardrobe::PLAYER, "color_change"));
        open_shop(&mut g);
        let row = shop_row(&g, "color_change");
        click_shop(&mut g, row);
        assert!(
            g.active_shop.as_ref().unwrap().picking_color,
            "tapping an owned Color Change should reopen the picker, not refuse the sale"
        );
    }
}
