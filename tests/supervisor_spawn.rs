//! Black-box checks against the real `horizon-server` executable.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_horizon-server")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("horizon-spawn-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn internal_mode_skips_setup_and_fails_loudly_until_engine_exists() {
    let dir = scratch("internal");
    let output = Command::new(binary())
        .args(["--__horizon-engine-internal", "--dir"])
        .arg(&dir)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("internal engine mode"), "{stderr}");
    assert!(stderr.contains("not integrated yet"), "{stderr}");
    assert!(!dir.join("server.properties").exists());
    assert!(!dir.join("eula.txt").exists());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn help_and_version_are_network_free_and_exit_zero() {
    for flag in ["--help", "--version"] {
        let output = Command::new(binary()).arg(flag).output().unwrap();
        assert!(output.status.success(), "{flag}: {:?}", output.status);
        assert!(!output.stdout.is_empty());
    }
}
