// SPDX-License-Identifier: MIT
fn statsplus_tick(game: &mut crate::game::Game<'_>) {
    crate::game::native_hooks::client_tick_post::statsplus::close_if_asked(game);
    if !game.native_window() {
        return;
    }
    crate::game::native_hooks::client_tick_post::statsplus::ensure_ready(game);
    if crate::game::native_hooks::client_tick_post::statsplus::claimed() {
        crate::game::native_hooks::client_tick_post::statsplus::refresh_stats(game);
    }
}
