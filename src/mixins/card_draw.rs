// SPDX-License-Identifier: MIT
// Statistics+'s settings page: its card is drawn instead of Sunrise's generic page.
fn statsplus_card_options_draw(
    game: &mut crate::game::Game<'_>,
    batch: &mut sunrise_client::hud::HudBatch,
) -> ::core::ops::ControlFlow<()> {
    if !crate::game::statsplus_card::active(game) {
        return ::core::ops::ControlFlow::Continue(());
    }
    crate::game::statsplus_card::draw(game, batch);
    ::core::ops::ControlFlow::Break(())
}
