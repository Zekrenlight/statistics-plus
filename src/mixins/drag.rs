// SPDX-License-Identifier: MIT
fn statsplus_drag(game: &crate::game::Game<'_>, screen: &mod_api::ScreenView<'_>, button: i32, canceled: &mut bool) {
    if crate::game::native_hooks::client_tick_post::statsplus_panel::drag(game, screen, button) {
        *canceled = true;
    }
}
