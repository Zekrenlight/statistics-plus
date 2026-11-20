// SPDX-License-Identifier: MIT
fn statsplus_release(game: &crate::game::Game<'_>, screen: &mod_api::ScreenView<'_>, button: i32, canceled: &mut bool) {
    if crate::game::native_hooks::client_tick_post::statsplus_panel::release(game, screen, button) {
        *canceled = true;
    }
}
