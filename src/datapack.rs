//! Vanilla datapack discovery and metadata validation for Minecraft 26.3.
//!
//! This covers directory packs and ZIP packs under `<world>/datapacks/`. It deliberately does
//! **not** implement loading, priorities, `/reload` or worldgen registration — those belong to the
//! headless engine and remain unimplemented. No Horizon mod package is accepted here.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use zip::ZipArchive;

/// Pinned by upstream 26.3 `crates/horizon-core/src/version.rs` at commit a02b2838.
pub const DATA_PACK_FORMAT: i64 = 121;

#[derive(Debug)]
pub enum DatapackError {
    Io(io::Error),
    Json(String),
    InvalidMeta(String),
    UnsupportedFormat { name: String, format: i64 },
}

impl std::fmt::Display for DatapackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "invalid pack.mcmeta: {error}"),
            Self::InvalidMeta(reason) => write!(formatter, "invalid pack.mcmeta: {reason}"),
            Self::UnsupportedFormat { name, format } => write!(
                formatter,
                "datapack {name} uses format {format}; Minecraft 26.3 requires {DATA_PACK_FORMAT}"
            ),
        }
    }
}

impl std::error::Error for DatapackError {}

impl From<io::Error> for DatapackError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Deserialize)]
struct PackMeta {
    pack: PackSection,
}

#[derive(Debug, Deserialize)]
struct PackSection {
    pack_format: i64,
    #[serde(default)]
    supported_formats: Option<SupportedFormats>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SupportedFormats {
    Single(i64),
    Range { min_inclusive: i64, max_inclusive: Option<i64> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackSource {
    Directory(PathBuf),
    Zip(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredPack {
    pub name: String,
    pub format: i64,
    pub source: PackSource,
}

fn validate(name: &str, bytes: &[u8]) -> Result<i64, DatapackError> {
    let meta: PackMeta =
        serde_json::from_slice(bytes).map_err(|error| DatapackError::Json(error.to_string()))?;
    let format = meta.pack.pack_format;
    let supported = match meta.pack.supported_formats {
        None => format == DATA_PACK_FORMAT,
        Some(SupportedFormats::Single(value)) => value == DATA_PACK_FORMAT,
        Some(SupportedFormats::Range { min_inclusive, max_inclusive }) => {
            DATA_PACK_FORMAT >= min_inclusive
                && max_inclusive.is_none_or(|maximum| DATA_PACK_FORMAT <= maximum)
        }
    };
    if !supported {
        return Err(DatapackError::UnsupportedFormat { name: name.to_owned(), format });
    }
    Ok(format)
}

fn from_directory(path: &Path) -> Result<Option<DiscoveredPack>, DatapackError> {
    let meta_path = path.join("pack.mcmeta");
    if !meta_path.is_file() {
        return Ok(None);
    }
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let format = validate(&name, &fs::read(&meta_path)?)?;
    Ok(Some(DiscoveredPack { name, format, source: PackSource::Directory(path.to_path_buf()) }))
}

fn from_zip(path: &Path) -> Result<Option<DiscoveredPack>, DatapackError> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("zip") {
        return Ok(None);
    }
    let mut archive = ZipArchive::new(fs::File::open(path)?)
        .map_err(|error| DatapackError::InvalidMeta(format!("invalid zip archive: {error}")))?;
    // Only the root pack.mcmeta is read; entries are never extracted, so zip-slip paths are inert.
    let mut file = match archive.by_name("pack.mcmeta") {
        Ok(file) => file,
        Err(_) => return Ok(None),
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let format = validate(&name, &bytes)?;
    Ok(Some(DiscoveredPack { name, format, source: PackSource::Zip(path.to_path_buf()) }))
}

/// List valid vanilla datapacks in a world. Missing `datapacks/` is an empty list.
pub fn discover(world_dir: &Path) -> Result<Vec<DiscoveredPack>, DatapackError> {
    let root = world_dir.join("datapacks");
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(DatapackError::Io(error)),
    };
    let mut packs = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let found = if path.is_dir() {
            from_directory(&path)?
        } else if path.is_file() {
            from_zip(&path)?
        } else {
            None
        };
        if let Some(pack) = found {
            packs.push(pack);
        }
    }
    packs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(packs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("horizon-datapack-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir.join("world/datapacks")).unwrap();
        dir
    }

    fn meta(format: i64) -> String {
        format!(r#"{{"pack":{{"pack_format":{format},"description":"synthetic test"}}}}"#)
    }

    #[test]
    fn discovers_directory_and_zip_packs_in_name_order() {
        let dir = scratch("both");
        let packs = dir.join("world/datapacks");
        fs::create_dir_all(packs.join("b_dir")).unwrap();
        fs::write(packs.join("b_dir/pack.mcmeta"), meta(DATA_PACK_FORMAT)).unwrap();

        let zip_path = packs.join("a_zip.zip");
        let mut writer = ZipWriter::new(fs::File::create(&zip_path).unwrap());
        writer
            .start_file("pack.mcmeta", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(meta(DATA_PACK_FORMAT).as_bytes()).unwrap();
        writer.finish().unwrap();

        let found = discover(&dir.join("world")).unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "a_zip.zip");
        assert!(matches!(found[0].source, PackSource::Zip(_)));
        assert_eq!(found[1].name, "b_dir");
        assert!(matches!(found[1].source, PackSource::Directory(_)));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_wrong_format_and_ignores_non_pack_entries() {
        let dir = scratch("invalid");
        let packs = dir.join("world/datapacks");
        fs::create_dir_all(packs.join("old")).unwrap();
        fs::write(packs.join("old/pack.mcmeta"), meta(10)).unwrap();
        fs::write(packs.join("notes.txt"), "not a pack").unwrap();

        let error = discover(&dir.join("world")).unwrap_err();
        assert!(error.to_string().contains("26.3 requires 121"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn accepts_supported_format_range_and_missing_directory() {
        let dir = scratch("range");
        assert!(discover(&dir.join("world")).unwrap().is_empty());
        let packs = dir.join("world/datapacks");
        fs::create_dir_all(packs.join("range")).unwrap();
        fs::write(
            packs.join("range/pack.mcmeta"),
            r#"{"pack":{"pack_format":48,"supported_formats":{"min_inclusive":48,"max_inclusive":121},"description":"x"}}"#,
        )
        .unwrap();
        assert_eq!(discover(&dir.join("world")).unwrap().len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}
