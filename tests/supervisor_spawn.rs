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
fn prepare_subcommand_accepts_a_synthetic_verified_folder() {
    let dir = scratch("prepare");
    let source = dir.join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("data.bin"), b"abc").unwrap();
    fs::write(
        source.join("prepared-manifest.json"),
        r#"{"version":"26.3","files":[{"path":"data.bin","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","size":3}]}"#,
    )
    .unwrap();

    let server = dir.join("server");
    let output = Command::new(binary())
        .args(["prepare", "--dir"])
        .arg(&server)
        .args(["--accept-eula", "--prepared-folder"])
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(server.join(".horizon/prepared/26.3/data.bin").exists());
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
