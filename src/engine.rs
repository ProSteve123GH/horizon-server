//! Internal engine-mode boundary.
//!
//! The supervisor re-executes `current_exe()` with a private flag. This module detects that mode
//! *before* any setup, bootstrap or CLI dispatch, which makes recursive supervisor startup
//! impossible. With the `engine` feature enabled, internal mode enters the real dedicated game loop
//! directly through `horizon_client::entry` with an empty (no-mod) runtime. Until that seam is
//! type-verified in CI the feature is off, and this placeholder fails loudly rather than pretending
//! to be a running server.

pub const INTERNAL_FLAG: &str = "--__horizon-engine-internal";

pub fn is_internal_invocation(args: &[String]) -> bool {
    args.first().is_some_and(|argument| argument == INTERNAL_FLAG)
}

#[cfg(feature = "engine")]
fn run_engine_seam(_args: &[String]) -> Result<(), String> {
    // Direct dedicated-server entry, deliberately NOT horizon_game::main(): that function
    // unconditionally starts the mod host (wasmtime), which this product forbids.
    let startup = horizon_client::entry::prepare();
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
    Ok(())
}

#[cfg(not(feature = "engine"))]
fn run_engine_seam(_args: &[String]) -> Result<(), String> {
    Err(concat!(
        "headless engine is not integrated yet: supervisor/engine single-binary dispatch is ",
        "reserved by this internal mode, but no game simulation is linked"
    )
    .to_owned())
}

pub fn run_internal(args: &[String]) -> Result<(), String> {
    run_engine_seam(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_flag_is_detected_only_as_first_argument() {
        assert!(is_internal_invocation(&[INTERNAL_FLAG.to_owned()]));
        assert!(!is_internal_invocation(&["start".to_owned(), INTERNAL_FLAG.to_owned()]));
        assert!(!is_internal_invocation(&[]));
    }

    #[test]
    fn placeholder_does_not_claim_to_run_an_engine() {
        assert!(run_internal(&[]).unwrap_err().contains("not integrated yet"));
    }
}
