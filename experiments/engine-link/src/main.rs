//! Feasibility probe: reach the dedicated engine's startup/entry APIs directly,
//! with an empty (no-mod) runtime, and WITHOUT `horizon-mod-host` / wasmtime.
//!
//! See README.md for the API survey and results. `cargo check` is enough here:
//! this file is not meant to be run.

fn main() {
    // (1) Smallest observable call into the engine: upstream's first startup
    // step, which validates the launch arguments/options and returns a Startup
    // lease. It names no mod host and no wasm runtime.
    let startup = horizon_client::entry::prepare();

    // (2) Drive the full headless engine entry with an empty native-mod inventory
    // and no-op code/loader hooks. `Runtime::default()` is the vanilla-empty
    // inventory upstream documents; the closure returns no loaded mod ids and no
    // host state. This reaches `game::run_with(options, native)` — the real
    // dedicated-server loop — without ever calling `horizon_mod_host::main_with`.
    horizon_client::entry::start(
        horizon_mod_package::runtime::Runtime::default(),
        |_native: &horizon_mod_package::runtime::Runtime,
         _launch: &horizon_mod_package::selection::LaunchArguments|
         -> (Vec<&'static str>, ()) { (Vec::new(), ()) },
        horizon_client::entry::CodeRuntime {
            initialize: |_inventory| {},
            refusal: |_id| None,
            startup_refused: |_id| false,
        },
        startup,
    );
}
