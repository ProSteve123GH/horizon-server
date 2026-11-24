//! `horizon-server`: single-executable supervisor and (eventually) headless engine.
//!
//! Current implementation status: CLI, configuration and EULA handling only. The game engine is not
//! linked yet; see `PROGRESS.md`. This binary must never be presented as a working server until the
//! engine and bootstrap milestones are complete.

mod config;
mod eula;
mod engine;

use std::env;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

const MINECRAFT_VERSION: &str = "26.3";
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn help_text() -> &'static str {
    "horizon-server — standalone vanilla-compatible Minecraft 26.3 dedicated server\n\
     \n\
     Usage:\n\
       horizon-server [options]                 Start in the foreground (default)\n\
       horizon-server init                      Explicit first-run setup only\n\
       horizon-server --help                    Show this help without network access\n\
       horizon-server --version                 Print server and Minecraft versions\n\
     \n\
     Options:\n\
       --dir <path>          Server directory (default: current working directory)\n\
       --accept-eula         Explicitly accept the EULA for non-interactive setup\n\
     \n\
     The Minecraft EULA must be accepted explicitly: answer the terminal prompt or set\n\
     --accept-eula after reading https://aka.ms/MinecraftEULA. No game files are bundled."
}

fn explicit_acceptance(args: &[String]) -> bool {
    if args.iter().any(|argument| argument == "--accept-eula") {
        return true;
    }
    matches!(
        env::var("HORIZON_SERVER_ACCEPT_EULA").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

fn server_directory(args: &[String]) -> Result<PathBuf, String> {
    let mut iterator = args.iter();
    while let Some(argument) = iterator.next() {
        if argument == "--dir" {
            let value = iterator.next().ok_or("--dir requires a path")?;
            return Ok(PathBuf::from(value));
        }
        if let Some(value) = argument.strip_prefix("--dir=") {
            return Ok(PathBuf::from(value));
        }
    }
    env::current_dir().map_err(|error| format!("cannot read working directory: {error}"))
}

fn prepare_directory(dir: &Path, args: &[String], injected_answer: Option<&str>) -> Result<(), String> {
    fs_create_dir(dir)?;
    let properties = config::load_or_create(dir).map_err(|error| format!("server.properties: {error}"))?;
    println!("Loaded server.properties ({} bytes).", properties.len());

    let state = eula::ensure_eula(
        dir,
        eula::EulaOptions {
            explicit_accept: explicit_acceptance(args),
            interactive: injected_answer.is_some() || io::stdin().is_terminal(),
            injected_answer,
        },
    )
    .map_err(|error| error.to_string())?;
    match state {
        eula::EulaState::AlreadyAccepted => println!("EULA already accepted."),
        eula::EulaState::AcceptedNow => println!("EULA accepted and recorded in eula.txt."),
    }
    Ok(())
}

fn fs_create_dir(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| format!("cannot create {}: {error}", dir.display()))
}

fn is_init(args: &[String]) -> bool {
    args.iter().any(|argument| argument == "init")
}

/// Returns a process exit code. Kept separate from `main` so the CLI can be exercised in tests.
fn run(args: &[String], injected_answer: Option<&str>) -> i32 {
    if engine::is_internal_invocation(args) {
        return match engine::run_internal(&args[1..]) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("horizon-server: internal engine mode: {error}");
                2
            }
        };
    }

    if args.iter().any(|argument| argument == "--help" || argument == "-h") {
        println!("{}", help_text());
        return 0;
    }
    if args.iter().any(|argument| argument == "--version" || argument == "-V") {
        println!("horizon-server {VERSION} (Minecraft {MINECRAFT_VERSION})");
        return 0;
    }

    let dir = match server_directory(args) {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("horizon-server: {error}");
            return 2;
        }
    };

    if let Err(error) = prepare_directory(&dir, args, injected_answer) {
        eprintln!("horizon-server: {error}");
        return 2;
    }

    if is_init(args) {
        println!("First-run setup complete for {}.", dir.display());
        return 0;
    }

    eprintln!(
        "horizon-server: resource bootstrap and headless engine are not integrated yet; \
         refusing to claim the server is running"
    );
    2
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    std::process::exit(run(&args, None));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("horizon-cli-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn help_and_version_work_without_a_server_directory() {
        assert_eq!(run(&["--help".to_owned()], None), 0);
        assert_eq!(run(&["--version".to_owned()], None), 0);
    }

    #[test]
    fn internal_mode_does_not_run_setup() {
        let dir = scratch("internal");
        let args = vec![
            engine::INTERNAL_FLAG.to_owned(),
            "--dir".to_owned(),
            dir.to_string_lossy().into_owned(),
        ];
        assert_eq!(run(&args, None), 2);
        assert!(!dir.join("server.properties").exists());
        assert!(!dir.join("eula.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn init_requires_explicit_eula_and_creates_expected_files() {
        let dir = scratch("init");
        let path = dir.to_string_lossy().into_owned();

        let refused = run(&["init".to_owned(), "--dir".to_owned(), path.clone()], None);
        assert_eq!(refused, 2);
        assert!(dir.join("server.properties").exists());
        assert!(!dir.join("eula.txt").exists());

        let accepted = run(
            &[
                "init".to_owned(),
                "--dir".to_owned(),
                path,
                "--accept-eula".to_owned(),
            ],
            None,
        );
        assert_eq!(accepted, 0);
        assert!(fs::read_to_string(dir.join("eula.txt"))
            .unwrap()
            .contains("eula=true"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
