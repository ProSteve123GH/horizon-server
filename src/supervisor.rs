//! Same-executable supervisor/engine boundary.
//!
//! The supervisor re-executes `current_exe()` with a private flag. The flag is detected before any
//! setup or bootstrap in `main`, so recursive supervisor startup is impossible. The engine itself is
//! not linked yet; spawning currently produces a loud placeholder error, which is intentional.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::bootstrap;
use crate::engine::INTERNAL_ENV;
use crate::prepared;

/// Arguments for the hidden internal engine mode. They must be valid upstream game arguments:
/// `entry::prepare()` parses every argv entry strictly, so our private handoff uses an env var.
/// No secret is passed on the command line.
pub fn engine_args(dir: &Path) -> Vec<String> {
    // Prefer the operator-supplied (or CI-generated) verified prepared dataset. Only fall back to
    // the client-JAR staging dir when no accepted dataset exists; that fallback cannot start the
    // engine on its own, which is correct and produces the upstream "missing prepared files" error.
    let resources = prepared::load_accepted(dir, bootstrap::VERSION)
        .ok()
        .flatten()
        .map(|data| data.root)
        .unwrap_or_else(|| dir.join(".horizon/resources/26.3"));
    // Upstream's launch parser stops at --dedicated and treats every later argument as a
    // dedicated-server option, so global --resources MUST come first (tablet crash 2026-11-24).
    vec![
        "--resources".to_owned(),
        resources.to_string_lossy().into_owned(),
        "--dedicated".to_owned(),
        "--nogui".to_owned(),
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
    use std::fs;

    fn scratch(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("horizon-supervisor-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn internal_argv_is_valid_for_upstream_entry() {
        let args = engine_args(Path::new("/srv/horizon"));
        assert_eq!(args[0], "--resources");
        let dedicated = args.iter().position(|arg| arg == "--dedicated").unwrap();
        let resources = args.iter().position(|arg| arg == "--resources").unwrap();
        assert!(resources < dedicated, "--resources must precede --dedicated");
        assert!(!args.iter().any(|arg| arg == crate::engine::INTERNAL_FLAG));
        assert!(args.contains(&"--nogui".to_owned()));
        assert!(args.contains(&"/srv/horizon/.horizon/resources/26.3".to_owned()));
        assert!(args.contains(&"/srv/horizon".to_owned()));
    }

    #[test]
    fn accepted_prepared_folder_is_used_as_resources() {
        let dir = scratch("resources");
        let prepared_dir = dir.join(".horizon/prepared/26.3");
        fs::create_dir_all(&prepared_dir).unwrap();
        fs::write(
            prepared_dir.join("prepared-manifest.json"),
            r#"{"version":"26.3","files":[]}"#,
        )
        .unwrap();
        fs::write(prepared_dir.join("horizon-resources.json"), "{}").unwrap();

        let args = engine_args(&dir);
        assert!(args.contains(&prepared_dir.to_string_lossy().into_owned()));
        fs::remove_dir_all(&dir).unwrap();
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
