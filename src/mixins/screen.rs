// SPDX-License-Identifier: MIT
fn statsplus_screen(
    game: &crate::game::Game<'_>,
    batch: &mut sunrise_client::hud::HudBatch,
    screen: &mod_api::ScreenView<'_>,
) {
    crate::game::native_hooks::client_tick_post::statsplus_panel::paint(game, batch, screen);
}
