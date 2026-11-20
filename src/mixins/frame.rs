// SPDX-License-Identifier: MIT
fn statsplus_frame(game: &mut crate::game::Game<'_>) {
    crate::game::native_hooks::client_tick_post::statsplus::on_frame(game);
}
