//! Vanilla-style server directory layout. Files are created lazily and never overwritten.

use std::fs;
use std::io;
use std::path::Path;

const PLAYER_FILES: &[&str] = &[
    "ops.json",
    "whitelist.json",
    "banned-players.json",
    "banned-ips.json",
    "usercache.json",
];

/// Reject separators and parent references so a configured level name cannot escape the server dir.
pub fn sanitize_level_name(name: &str) -> Option<&str> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('\0')
    {
        None
    } else {
        Some(trimmed)
    }
}

/// Create the minimum familiar vanilla directory shape without touching existing data.
pub fn ensure_layout(dir: &Path, level_name: &str) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::create_dir_all(dir.join("logs"))?;
    fs::create_dir_all(dir.join(".horizon"))?;

    let level = sanitize_level_name(level_name).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "invalid level-name for a directory")
    })?;
    fs::create_dir_all(dir.join(level).join("datapacks"))?;

    for name in PLAYER_FILES {
        let path = dir.join(name);
        if !path.exists() {
            fs::write(&path, "[]\n")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("horizon-layout-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn creates_expected_layout_and_empty_player_lists() {
        let dir = scratch("layout");
        ensure_layout(&dir, "world").unwrap();
        for path in ["logs", ".horizon", "world/datapacks"] {
            assert!(dir.join(path).is_dir(), "{path} missing");
        }
        for name in PLAYER_FILES {
            assert_eq!(fs::read_to_string(dir.join(name)).unwrap(), "[]\n");
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn never_overwrites_existing_operator_data() {
        let dir = scratch("preserve");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("ops.json"), "[{\"uuid\":\"x\",\"name\":\"Alice\"}]").unwrap();
        ensure_layout(&dir, "custom-world").unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("ops.json")).unwrap(),
            "[{\"uuid\":\"x\",\"name\":\"Alice\"}]"
        );
        assert!(dir.join("custom-world/datapacks").is_dir());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_path_traversal_in_level_name() {
        assert_eq!(sanitize_level_name("world"), Some("world"));
        assert_eq!(sanitize_level_name("../escape"), None);
        assert_eq!(sanitize_level_name("a/b"), None);
        assert_eq!(sanitize_level_name(".."), None);
    }
}
