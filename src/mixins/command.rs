// SPDX-License-Identifier: MIT
fn statsplus_command(game: &mut crate::game::Game<'_>, line: &str) {
    crate::game::native_hooks::client_tick_post::statsplus::ensure_ready(game);
    let mut ran = false;
    crate::game::native_hooks::client_tick_post::statsplus::handle_command(game, line, &mut ran);
}
