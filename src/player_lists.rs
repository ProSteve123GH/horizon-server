//! Vanilla-style player administration files (`ops.json`, `whitelist.json`, banned lists and
//! `usercache.json`). All data here is synthetic operator data; no real account info is bundled.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerEntry {
    pub uuid: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl PlayerEntry {
    pub fn new(uuid: impl Into<String>, name: impl Into<String>) -> Self {
        Self { uuid: uuid.into(), name: name.into(), level: None, extra: BTreeMap::new() }
    }

    pub fn op(uuid: impl Into<String>, name: impl Into<String>, level: i32) -> Self {
        Self { level: Some(level), ..Self::new(uuid, name) }
    }

    pub fn matches(&self, uuid_or_name: &str) -> bool {
        self.uuid.eq_ignore_ascii_case(uuid_or_name)
            || self.name.eq_ignore_ascii_case(uuid_or_name)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpBanEntry {
    pub ip: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub source: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedUser {
    pub name: String,
    pub uuid: String,
    #[serde(default)]
    pub expires_on: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn atomic_write(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, format!("{text}\n"))?;
    fs::rename(&temporary, path)
}

/// Load a vanilla player list. A missing file is an empty list; vanilla creates these lazily.
pub fn load_players(path: &Path) -> io::Result<Vec<PlayerEntry>> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

pub fn save_players(path: &Path, entries: &[PlayerEntry]) -> io::Result<()> {
    atomic_write(path, &entries)
}

pub fn load_ip_bans(path: &Path) -> io::Result<Vec<IpBanEntry>> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

pub fn save_ip_bans(path: &Path, entries: &[IpBanEntry]) -> io::Result<()> {
    atomic_write(path, &entries)
}

pub fn load_user_cache(path: &Path) -> io::Result<Vec<CachedUser>> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

pub fn save_user_cache(path: &Path, entries: &[CachedUser]) -> io::Result<()> {
    atomic_write(path, &entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("horizon-players-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_lists_are_empty_and_round_trip_vanilla_shape() {
        let dir = scratch("roundtrip");
        let path = dir.join("ops.json");
        assert!(load_players(&path).unwrap().is_empty());
        let ops = vec![
            PlayerEntry::op("00000000-0000-0000-0000-000000000001", "Alice", 4),
            PlayerEntry::new("00000000-0000-0000-0000-000000000002", "Bob"),
        ];
        save_players(&path, &ops).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"level\": 4"));
        assert_eq!(load_players(&path).unwrap(), ops);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_json_fields_are_preserved() {
        let dir = scratch("unknown");
        let path = dir.join("whitelist.json");
        fs::write(
            &path,
            r#"[{"uuid":"abc","name":"Alice","futureField":42}]"#,
        )
        .unwrap();
        let entries = load_players(&path).unwrap();
        assert_eq!(entries[0].extra.get("futureField"), Some(&Value::from(42)));
        save_players(&path, &entries).unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains("futureField"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ip_bans_and_user_cache_use_vanilla_field_names() {
        let dir = scratch("ipcache");
        let bans = dir.join("banned-ips.json");
        save_ip_bans(
            &bans,
            &[IpBanEntry {
                ip: "203.0.113.9".to_owned(),
                reason: "synthetic test".to_owned(),
                created: "2026-11-24 00:00:00 +0000".to_owned(),
                source: "console".to_owned(),
                extra: BTreeMap::new(),
            }],
        )
        .unwrap();
        assert!(fs::read_to_string(&bans).unwrap().contains("\"ip\""));
        assert_eq!(load_ip_bans(&bans).unwrap()[0].ip, "203.0.113.9");

        let cache = dir.join("usercache.json");
        save_user_cache(
            &cache,
            &[CachedUser {
                name: "Alice".to_owned(),
                uuid: "00000000-0000-0000-0000-000000000001".to_owned(),
                expires_on: "2026-12-24 00:00:00 +0000".to_owned(),
                extra: BTreeMap::new(),
            }],
        )
        .unwrap();
        let loaded = load_user_cache(&cache).unwrap();
        assert_eq!(loaded[0].name, "Alice");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn matching_is_case_insensitive_for_names_and_uuids() {
        let entry = PlayerEntry::new("AbC", "Alice");
        assert!(entry.matches("alice"));
        assert!(entry.matches("abc"));
        assert!(!entry.matches("Bob"));
    }
}
