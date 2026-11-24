//! Vanilla-style `server.properties` handling.
//!
//! The operator's file is the canonical source of gameplay settings. Unknown keys are preserved
//! byte-for-byte on their own line; only keys we explicitly manage are rewritten. This module is
//! deliberately dependency-free so the same code builds for Android, Linux, Windows and macOS.

use std::fs;
use std::io;
use std::path::Path;

/// Defaults for the small, unambiguous subset of vanilla 26.3 properties we own today.
/// Additional keys are appended only when absent, so operator edits always win.
pub const MANAGED_DEFAULTS: &[(&str, &str)] = &[
    ("motd", "A Horizon Server"),
    ("server-port", "25565"),
    ("max-players", "20"),
    ("online-mode", "true"),
    ("level-name", "world"),
    ("view-distance", "10"),
    ("simulation-distance", "10"),
    ("gamemode", "survival"),
    ("difficulty", "easy"),
    ("pvp", "true"),
    ("spawn-protection", "16"),
];

#[derive(Clone, Debug)]
struct Properties {
    lines: Vec<String>,
}

impl Properties {
    /// Parse a properties file. Lines that are blank or start with `#` or `!` are kept verbatim;
    /// property lines are indexed by their key so values can be replaced without discarding
    /// unknown settings.
    fn parse(text: &str) -> Self {
        Self {
            lines: text.lines().map(str::to_owned).collect(),
        }
    }

    fn separator(line: &str) -> Option<usize> {
        let mut escaped = false;
        for (index, character) in line.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match character {
                '\\' => escaped = true,
                '=' | ':' => return Some(index),
                _ => {}
            }
        }
        None
    }

    fn is_property(line: &str) -> bool {
        let trimmed = line.trim_start();
        !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('!')
    }

    fn key_of(line: &str) -> Option<String> {
        if !Self::is_property(line) {
            return None;
        }
        match Self::separator(line) {
            Some(index) => {
                let key = line[..index].trim();
                (!key.is_empty()).then(|| key.to_owned())
            }
            None => {
                // Java properties also allow whitespace as a separator.
                let key = line.split_whitespace().next().unwrap_or_default();
                (!key.is_empty()).then(|| key.to_owned())
            }
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        for line in &self.lines {
            if Self::key_of(line).as_deref() == Some(key) {
                if let Some(index) = Self::separator(line) {
                    return Some(line[index + 1..].trim());
                }
                return line.split_whitespace().nth(1);
            }
        }
        None
    }

    fn set(&mut self, key: &str, value: &str) {
        for line in &mut self.lines {
            if Self::key_of(line).as_deref() == Some(key) {
                if let Some(index) = Self::separator(line) {
                    *line = format!("{}{}", &line[..=index], value);
                } else {
                    *line = format!("{key}={value}");
                }
                return;
            }
        }
        self.lines.push(format!("{key}={value}"));
    }

    fn ensure_defaults(&mut self) {
        for (key, value) in MANAGED_DEFAULTS {
            if self.get(key).is_none() {
                self.set(key, value);
            }
        }
    }

    fn render(&self) -> String {
        let mut text = self.lines.join("\n");
        text.push('\n');
        text
    }
}

fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    let temporary = path.with_extension("properties.tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}

/// Load `server.properties`, fill in managed defaults without overwriting operator values,
/// and write the result back. A missing file is created with defaults.
pub fn load_or_create(dir: &Path) -> io::Result<String> {
    let path = dir.join("server.properties");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let mut properties = Properties::parse(&text);
    properties.ensure_defaults();
    let rendered = properties.render();
    write_atomically(&path, &rendered)?;
    Ok(rendered)
}

/// Read a property without creating or rewriting the file (used by `--help`-style paths).
#[allow(dead_code)]
pub fn read_value(dir: &Path, key: &str) -> io::Result<Option<String>> {
    let text = fs::read_to_string(dir.join("server.properties"))?;
    Ok(Properties::parse(&text).get(key).map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_key_value_and_preserves_unknown_lines() {
        let source = "# a comment\nmotd=Old MOTD\n! another comment\nunknown-key = keep me\n";
        let mut properties = Properties::parse(source);
        assert_eq!(properties.get("motd"), Some("Old MOTD"));
        assert_eq!(properties.get("unknown-key"), Some("keep me"));
        properties.set("motd", "New MOTD");
        let rendered = properties.render();
        assert!(rendered.contains("motd=New MOTD"));
        assert!(rendered.contains("unknown-key = keep me"));
        assert!(rendered.contains("! another comment"));
    }

    #[test]
    fn defaults_do_not_overwrite_operator_values() {
        let mut properties = Properties::parse("server-port=30000\nmy-own-setting=1\n");
        properties.ensure_defaults();
        assert_eq!(properties.get("server-port"), Some("30000"));
        assert_eq!(properties.get("my-own-setting"), Some("1"));
        assert_eq!(properties.get("motd"), Some("A Horizon Server"));
    }

    #[test]
    fn load_or_create_round_trips_on_disk() {
        let dir = std::env::temp_dir().join(format!("horizon-config-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("server.properties"), "motd=Custom\nfuture-key=kept\n").unwrap();
        let rendered = load_or_create(&dir).unwrap();
        assert!(rendered.contains("motd=Custom"));
        assert!(rendered.contains("future-key=kept"));
        assert!(read_value(&dir, "server-port").unwrap().is_some());
        fs::remove_dir_all(&dir).unwrap();
    }
}
