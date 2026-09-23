//! The generic layout sweep on the default engine. The bodies live in
//! `tests/sweep/` so `tests/layout_taffy.rs` can run the same sweep with
//! every layout cross-checked against taffy.

mod sweep;

macro_rules! sweep_tests {
    ($($name:ident => $body:path),* $(,)?) => {
        $(#[test] fn $name() { $body() })*
    };
}

sweep_tests! {
    bolt_catalog_is_sane_everywhere => sweep::bolt_catalog_is_sane_everywhere,
    hermie_catalog_is_sane_everywhere => sweep::hermie_catalog_is_sane_everywhere,
    hermies_full_shelf_fits_one_page_at_the_default_window => sweep::hermies_full_shelf_fits_one_page_at_the_default_window,
    buying_and_trading_are_sane_everywhere => sweep::buying_and_trading_are_sane_everywhere,
    bolts_shelf_works_with_bolts_catalog_too => sweep::bolts_shelf_works_with_bolts_catalog_too,
    swag_picker_is_sane_with_a_full_wardrobe => sweep::swag_picker_is_sane_with_a_full_wardrobe,
    a_tall_screen_shows_the_whole_wardrobe_on_one_page => sweep::a_tall_screen_shows_the_whole_wardrobe_on_one_page,
    empty_swag_picker_is_sane => sweep::empty_swag_picker_is_sane,
    every_challenge_phase_is_sane_everywhere => sweep::challenge_sweep::every_challenge_phase_is_sane_everywhere,
    a_wrong_answer_does_not_move_the_answer_buttons => sweep::challenge_sweep::a_wrong_answer_does_not_move_the_answer_buttons,
    tapping_a_drawn_button_answers_it_even_under_a_wrapped_question =>
        sweep::challenge_sweep::tapping_a_drawn_button_answers_it_even_under_a_wrapped_question,
    dialogue_lines_are_sane_everywhere => sweep::dialogue_lines_are_sane_everywhere,
    settings_overlay_is_sane_everywhere => sweep::settings_overlay_is_sane_everywhere,
    quest_beats_are_sane_everywhere => sweep::quest_beats_are_sane_everywhere,
}
