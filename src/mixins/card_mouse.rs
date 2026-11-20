// SPDX-License-Identifier: MIT
// The mouse buttons on Statistics+'s settings card.
fn statsplus_card_options_mouse(
    game: &mut crate::game::Game<'_>,
    pressed: bool,
    right: bool,
) -> ::core::ops::ControlFlow<Result<()>> {
    if !crate::game::statsplus_card::active(game) {
        return ::core::ops::ControlFlow::Continue(());
    }
    ::core::ops::ControlFlow::Break(crate::game::statsplus_card::mouse(game, pressed, right))
}
