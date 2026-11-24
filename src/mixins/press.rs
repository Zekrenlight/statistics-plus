// SPDX-License-Identifier: MIT
fn statsplus_press(game: &crate::game::Game<'_>, screen: &mod_api::ScreenView<'_>, button: i32, double_click: bool, canceled: &mut bool) {
    let _ = double_click;
    if crate::game::native_hooks::client_tick_post::statsplus_panel::click(game, screen, button) {
        *canceled = true;
    }
}
