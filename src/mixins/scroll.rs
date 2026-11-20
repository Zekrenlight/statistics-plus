// SPDX-License-Identifier: MIT
fn statsplus_scroll(game: &crate::game::Game<'_>, screen: &mod_api::ScreenView<'_>, scroll: [f64; 2], canceled: &mut bool) {
    if crate::game::native_hooks::client_tick_post::statsplus_panel::wheel(game, screen, scroll) {
        *canceled = true;
    }
}
