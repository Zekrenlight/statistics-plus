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

use mod_api::{ClientHost, Command, Hooks, RegisterClientCommandsEvent};

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

    fn register_client_commands(
        &self,
        _host: &mut dyn ClientHost,
        event: &mut RegisterClientCommandsEvent<'_>,
    ) {
        // The real work runs in the `run_client_native_command` mixin, which
        // has `&mut Game` (needed to open the screen). These registrations
        // put the command in the client dispatcher so it is not sent to the server.
        event.commands.push(Command::new("statsplus [sub:text]").executes_client(|_, _| Some(1)));
    }

}
