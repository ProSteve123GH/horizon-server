//! Same-executable supervisor/engine boundary.
//!
//! The supervisor re-executes `current_exe()` with a private flag. The flag is detected before any
//! setup or bootstrap in `main`, so recursive supervisor startup is impossible. The engine itself is
//! not linked yet; spawning currently produces a loud placeholder error, which is intentional.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::engine::INTERNAL_FLAG;

/// Arguments for the hidden internal engine mode. No secret is passed on the command line.
pub fn engine_args(dir: &Path) -> Vec<String> {
    vec![
        INTERNAL_FLAG.to_owned(),
        "--dir".to_owned(),
        dir.to_string_lossy().into_owned(),
        "--dedicated".to_owned(),
        "--nogui".to_owned(),
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
    fn internal_mode_is_always_the_first_argument() {
        let args = engine_args(Path::new("/srv/horizon"));
        assert_eq!(args[0], INTERNAL_FLAG);
        assert!(args.contains(&"--dedicated".to_owned()));
        assert!(args.contains(&"--nogui".to_owned()));
        assert!(args.contains(&"/srv/horizon".to_owned()));
    }

    #[test]
    fn command_targets_this_executable_only() {
        let command = engine_command(Path::new(".")).unwrap();
        assert_eq!(command.get_program(), std::env::current_exe().unwrap().as_os_str());
    }
}
