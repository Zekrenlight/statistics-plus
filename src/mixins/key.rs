// SPDX-License-Identifier: MIT
fn statsplus_key(game: &crate::game::Game<'_>, screen: &mod_api::ScreenView<'_>, key: &str, canceled: &mut bool) {
    if crate::game::native_hooks::client_tick_post::statsplus_panel::key(game, screen, key) {
        *canceled = true;
    }
}
