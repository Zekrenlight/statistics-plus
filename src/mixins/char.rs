// SPDX-License-Identifier: MIT
fn statsplus_char(game: &crate::game::Game<'_>, screen: &mod_api::ScreenView<'_>, c: char, canceled: &mut bool) {
    if crate::game::native_hooks::client_tick_post::statsplus_panel::typed(game, screen, c) {
        *canceled = true;
    }
}
