// SPDX-License-Identifier: MIT
// Dragging on Statistics+'s settings card: its sliders and scrollbar.
fn statsplus_card_options_drag(game: &mut crate::game::Game<'_>) -> ::core::ops::ControlFlow<Result<bool>> {
    if !crate::game::statsplus_card::active(game) {
        return ::core::ops::ControlFlow::Continue(());
    }
    ::core::ops::ControlFlow::Break(crate::game::statsplus_card::drag(game).map(|()| true))
}
