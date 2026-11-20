// SPDX-License-Identifier: MIT
//! Statistics+ for Sunrise: a clearer Statistics screen.
//!
//! This crate holds the pure rules (`logic`: sorting, grouping, units,
//! filtering, listing math and snapshot files). The game side, which
//! replaces the Statistics screen, is the support modules in `src/native/`
//! and the HEAD mixins in `src/mixins/`, listed in `sunrise-mod.toml`.

pub mod logic;
#[cfg(test)]
mod rules_tests;

use mod_api::Hooks;

#[derive(Default)]
struct StatsPlus;

#[mod_api::sunrise_mod]
impl Hooks for StatsPlus {
    fn register(&self, registrar: &mut dyn mod_api::Registrar, _cx: &mod_api::RegistrationContext) {
        // The accent colour of statsplus's cards (Sunrise orange by default).
        let _ = registrar.register_setting(
            mod_api::SettingSpec::color("accent", "statsplus.setting.accent", 0xF9801D)
                .tooltip("statsplus.setting.accent.tooltip"),
        );
    }
}
