// SPDX-License-Identifier: MIT
fn statsplus_draw_stats(
    game: &mut crate::game::Game<'_>,
    batch: &mut sunrise_client::hud::HudBatch,
) -> core::ops::ControlFlow<()> {
    crate::game::native_hooks::client_tick_post::statsplus::take_over_stats(game);
    sunrise_client::widgets::begin_screen(|| {
        sunrise_client::widgets::ScreenInfo::new(
            "stats",
            crate::game::screen_info::tr("gui.stats", &[]),
        )
    });
    let (w, h) = (batch.width, batch.height);
    sunrise_client::ui::menu_background(batch, &game.gui.atlas, 0.0, 0.0, w, h, true);
    core::ops::ControlFlow::Break(())
}
