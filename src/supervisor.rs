//! Same-executable supervisor/engine boundary.
//!
//! The supervisor re-executes `current_exe()` with a private flag. The flag is detected before any
//! setup or bootstrap in `main`, so recursive supervisor startup is impossible. The engine itself is
//! not linked yet; spawning currently produces a loud placeholder error, which is intentional.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::engine::INTERNAL_ENV;

/// Arguments for the hidden internal engine mode. They must be valid upstream game arguments:
/// `entry::prepare()` parses every argv entry strictly, so our private handoff uses an env var.
/// No secret is passed on the command line.
pub fn engine_args(dir: &Path) -> Vec<String> {
    let resources = dir.join(".horizon/resources/26.3");
    vec![
        "--dedicated".to_owned(),
        "--nogui".to_owned(),
        "--resources".to_owned(),
        resources.to_string_lossy().into_owned(),
        "--universe".to_owned(),
        dir.to_string_lossy().into_owned(),
    ]
}

/// Build the child command from this exact executable, never from a second installed binary.
pub fn engine_command(dir: &Path) -> Result<Command, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("cannot resolve current_exe(): {error}"))?;
    let mut command = Command::new(executable);
    command
        .args(engine_args(dir))
        .current_dir(dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env(INTERNAL_ENV, "1")
        .env("HORIZON_NO_AUDIO", "1")
        .env("HORIZON_NO_DATA_FILE", "1");
    Ok(command)
}

/// Run the internal engine mode as a child and report its exit status.
pub fn run_engine(dir: &Path) -> Result<i32, String> {
    let mut command = engine_command(dir)?;
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot spawn internal engine mode: {error}"))?;
    let status = child
        .wait()
        .map_err(|error| format!("cannot wait for internal engine mode: {error}"))?;
    Ok(status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_argv_is_valid_for_upstream_entry() {
        let args = engine_args(Path::new("/srv/horizon"));
        assert_eq!(args[0], "--dedicated");
        assert!(!args.iter().any(|arg| arg == crate::engine::INTERNAL_FLAG));
        assert!(args.contains(&"--nogui".to_owned()));
        assert!(args.contains(&"/srv/horizon/.horizon/resources/26.3".to_owned()));
        assert!(args.contains(&"/srv/horizon".to_owned()));
    }

    #[test]
    fn command_targets_this_executable_only() {
        let command = engine_command(Path::new(".")).unwrap();
        assert_eq!(command.get_program(), std::env::current_exe().unwrap().as_os_str());
        assert_eq!(
            command.get_envs().find(|(key, _)| *key == INTERNAL_ENV).and_then(|(_, value)| value),
            Some(std::ffi::OsStr::new("1"))
        );
    }
}
