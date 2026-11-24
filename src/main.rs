//! `horizon-server`: single-executable supervisor and (eventually) headless engine.
//!
//! Current implementation status: CLI, configuration and EULA handling only. The game engine is not
//! linked yet; see `PROGRESS.md`. This binary must never be presented as a working server until the
//! engine and bootstrap milestones are complete.

mod bootstrap;
mod config;
mod datapack;
mod engine;
mod eula;
mod player_lists;
mod prepared;
mod server_dir;
mod supervisor;

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
       horizon-server bootstrap                 Verify/fetch pinned game resources\n\
       horizon-server prepare                   Accept a verified prepared-data folder\n\
       horizon-server datapacks                 List valid vanilla datapacks in the world\n\
       horizon-server --help                    Show this help without network access\n\
       horizon-server --version                 Print server and Minecraft versions\n\
     \n\
     Options:\n\
       --dir <path>          Server directory (default: current working directory)\n\
       --accept-eula         Explicitly accept the EULA for non-interactive setup\n\
       --offline             Require already-verified cached resources (no network)\n\
       --prepared-folder <path>  Verified prepared-data folder (interim source)\n\
       --prepared-bundle <path>  CI-generated prepared-data ZIP archive\n\
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

fn option_value(args: &[String], name: &str) -> Option<String> {
    let mut iterator = args.iter();
    while let Some(argument) = iterator.next() {
        if argument == name {
            return iterator.next().cloned();
        }
        if let Some(value) = argument.strip_prefix(&format!("{name}=")) {
            return Some(value.to_owned());
        }
    }
    None
}

fn prepare_directory(dir: &Path, args: &[String], injected_answer: Option<&str>) -> Result<(), String> {
    fs_create_dir(dir)?;
    let properties = config::load_or_create(dir).map_err(|error| format!("server.properties: {error}"))?;
    println!("Loaded server.properties ({} bytes).", properties.len());

    let level_name = config::read_value(dir, "level-name")
        .ok()
        .flatten()
        .unwrap_or_else(|| "world".to_owned());
    server_dir::ensure_layout(dir, &level_name)
        .map_err(|error| format!("server directory: {error}"))?;

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

fn is_bootstrap(args: &[String]) -> bool {
    args.iter().any(|argument| argument == "bootstrap")
}

fn run_bootstrap(dir: &Path, args: &[String]) -> Result<(), String> {
    let offline = args.iter().any(|argument| argument == "--offline");
    let bootstrap = bootstrap::Bootstrap::new(dir, offline).map_err(|error| error.to_string())?;
    let (client, server) = bootstrap.artifacts().map_err(|error| error.to_string())?;
    println!(
        "Verified pinned Minecraft {} metadata (client SHA-1 {}, server SHA-1 {}).",
        bootstrap::VERSION, client.sha1, server.sha1
    );
    // The proven Android path uses the client JAR plus asset objects; the server JAR is verified
    // here but not downloaded until the headless engine audit confirms it is required.
    let path = bootstrap
        .artifact("client.jar", &client, offline)
        .map_err(|error| error.to_string())?;
    println!("Client JAR verified and cached at {}", path.display());
    let staged = bootstrap
        .stage_client_resources(&path)
        .map_err(|error| error.to_string())?;
    println!("Engine resources staged at {}", staged.display());
    Ok(())
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

    if args.iter().any(|argument| argument == "datapacks") {
        let level = config::read_value(&dir, "level-name")
            .ok()
            .flatten()
            .unwrap_or_else(|| "world".to_owned());
        match datapack::discover(&dir.join(level)) {
            Ok(packs) => {
                for pack in packs {
                    println!("{} (format {})", pack.name, pack.format);
                }
                return 0;
            }
            Err(error) => {
                eprintln!("horizon-server: datapacks: {error}");
                return 2;
            }
        }
    }

    if let Err(error) = prepare_directory(&dir, args, injected_answer) {
        eprintln!("horizon-server: {error}");
        return 2;
    }

    if args.iter().any(|argument| argument == "prepare") {
        let bundle = option_value(args, "--prepared-bundle")
            .or_else(|| env::var("HORIZON_PREPARED_BUNDLE").ok());
        let folder = option_value(args, "--prepared-folder")
            .or_else(|| env::var("HORIZON_PREPARED_FOLDER").ok());
        let result = match (bundle, folder) {
            (Some(bundle), _) => {
                prepared::accept_bundle(&dir, Path::new(&bundle), bootstrap::VERSION)
            }
            (None, Some(folder)) => {
                prepared::accept_folder(&dir, Path::new(&folder), bootstrap::VERSION)
            }
            (None, None) => {
                eprintln!(
                    "horizon-server: prepare requires --prepared-bundle <archive> or --prepared-folder <path>"
                );
                return 2;
            }
        };
        match result {
            Ok(data) => {
                println!("Accepted prepared data at {}", data.root.display());
                return 0;
            }
            Err(error) => {
                eprintln!("horizon-server: prepared data: {error}");
                return 2;
            }
        }
    }

    if is_init(args) {
        println!("First-run setup complete for {}.", dir.display());
        return 0;
    }

    if let Err(error) = run_bootstrap(&dir, args) {
        eprintln!("horizon-server: bootstrap: {error}");
        return 2;
    }
    if is_bootstrap(args) {
        println!("Resource bootstrap verified.");
        return 0;
    }

    match supervisor::run_engine(&dir) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("horizon-server: supervisor: {error}");
            2
        }
    }
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
