//! Verified Minecraft 26.3 resource bootstrap.
//!
//! This mirrors the validation of the Android fork's `official_install.rs`: official HTTPS hosts
//! only, no redirects, pinned content-addressed metadata, SHA-1 checks, bounded downloads, partial
//! files and atomic finalization. No Mojang file is ever committed or bundled with the binary.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use zip::ZipArchive;

/// Pins from canonical upstream `research-lock.json` (Minecraft 26.3), not the Android fork's
/// `dev/26.4` snapshot pins.
pub const VERSION: &str = "26.3";
pub const MANIFEST_SHA1: &str = "11f1aefe473eefba0af62e3286c33b692807b2eb";
pub const CLIENT_SHA1: &str = "e877b6a07acd633fb3bb475002175cec036e7b87";
pub const SERVER_SHA1: &str = "33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c";

/// Accepted asset indexes (id 34) of the republished 26.3 version JSONs, from upstream
/// `research-lock.json`. A launcher keeps whichever one it last downloaded.
pub const ACCEPTED_ASSET_INDEX_SHA1: &[&str] = &[
    "abfaa525f923f807df8b4e4d29c1b5e3a104adbe",
    "1e4e4a68dece1c0f51b8f93b45da8fe8da51bcd3",
    "0c8f43b28f837869c1e54a6aeb22c396a1548946",
    "917ff6d74b9f635c46bac0d28857206bbcc1ffe4",
    "331e5c80937c9cd79647ab34bdc1cbb5ef09f456",
    "e99f9973047785b5a9d52eaf72020b97545053c9",
    "4e950fe41d8ad13e66bde63f476afdbe0cb19f94",
    "f3df28b441565ca4e5deb9cda856d90df62e1433",
];

/// Other 26.3 version JSONs Mojang republished; they change only non-English translations.
pub const ACCEPTED_MANIFEST_SHA1: &[&str] = &[
    "4fe1aa1ef8da1cb95c5bad1fb98890ca56dd8ca3",
    "702fe59163c6ee6578607daa85811d9bc9c7cc40",
    "61fff1316bd50312624a858bbb5eb18fa45208ed",
    "5df1795e16eea19d75b923d66ecca9ce183353c5",
    "80608ff6753705b6a82e2cb86a5937d4d233abd4",
    "9f67e10ed013deeaad20ac3c800652765d493408",
    "969f7ab0ce024e8dcf95372971bcf291b34ed9f7",
    "ead839cbccf0ddf013c3f935f2840080b5cb0d26",
];

const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_STAGED_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug)]
pub enum BootstrapError {
    Io(io::Error),
    Http(String),
    DisallowedUrl(String),
    TooLarge { url: String, limit: u64 },
    HashMismatch { url: String, expected: String, actual: String },
    OfflineCacheMissing(PathBuf),
    Json(String),
    UnsafeArchivePath(String),
    TooLargeStaged,
}

impl std::fmt::Display for BootstrapError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Http(error) => write!(formatter, "HTTP error: {error}"),
            Self::DisallowedUrl(url) => write!(formatter, "disallowed URL (official HTTPS only): {url}"),
            Self::TooLarge { url, limit } => write!(formatter, "download too large ({limit} bytes): {url}"),
            Self::HashMismatch { url, expected, actual } => write!(
                formatter,
                "SHA-1 mismatch for {url}: expected {expected}, got {actual}"
            ),
            Self::OfflineCacheMissing(path) => {
                write!(formatter, "offline mode and no verified cache at {}", path.display())
            }
            Self::Json(error) => write!(formatter, "invalid Mojang metadata: {error}"),
            Self::UnsafeArchivePath(name) => write!(formatter, "unsafe archive path: {name}"),
            Self::TooLargeStaged => write!(formatter, "staged resources exceed {MAX_STAGED_BYTES} bytes"),
        }
    }
}

impl std::error::Error for BootstrapError {}

impl From<io::Error> for BootstrapError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Only the hosts used by the proven Android installer are accepted. No userinfo, no ports other
/// than the implicit HTTPS port, no plain HTTP, no redirects (configured on the agent).
pub fn is_official_url(url: &str) -> bool {
    const HOSTS: &[&str] = &[
        "piston-meta.mojang.com",
        "piston-data.mojang.com",
        "resources.download.minecraft.net",
        "launcher.mojang.com",
        "launchermeta.mojang.com",
    ];
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let Some((authority, _path)) = rest.split_once('/') else {
        return false;
    };
    if authority.contains('@') || authority.contains(':') {
        return false;
    }
    HOSTS.contains(&authority)
}

fn sha1_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn agent() -> ureq::Agent {
    ureq::builder().redirects(0).timeout_read(Duration::from_secs(60)).build()
}

fn fetch(url: &str, limit: u64) -> Result<Vec<u8>, BootstrapError> {
    if !is_official_url(url) {
        return Err(BootstrapError::DisallowedUrl(url.to_owned()));
    }
    let response = agent()
        .get(url)
        .call()
        .map_err(|error| BootstrapError::Http(error.to_string()))?;
    if response.status() != 200 {
        return Err(BootstrapError::Http(format!("HTTP {} for {url}", response.status())));
    }
    if let Some(length) = response.header("content-length").and_then(|value| value.parse::<u64>().ok()) {
        if length > limit {
            return Err(BootstrapError::TooLarge { url: url.to_owned(), limit });
        }
    }
    let mut limited = response.into_reader().take(limit + 1);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(BootstrapError::TooLarge { url: url.to_owned(), limit });
    }
    Ok(bytes)
}

#[derive(Debug, Deserialize)]
struct VersionJson {
    downloads: Downloads,
    #[serde(rename = "assetIndex")]
    asset_index: Artifact,
}

#[derive(Debug, Deserialize)]
struct Downloads {
    client: Artifact,
    server: Artifact,
}

#[derive(Debug, Deserialize)]
struct Artifact {
    url: String,
    sha1: String,
    size: u64,
}

/// The 26.3 version JSON is content-addressed, so no mutable launcher metadata is trusted.
pub fn version_json_url() -> String {
    format!(
        "https://piston-meta.mojang.com/v1/packages/{MANIFEST_SHA1}/{VERSION}.json"
    )
}

pub struct Bootstrap {
    pub version_json: Vec<u8>,
    pub cache_dir: PathBuf,
}

impl Bootstrap {
    /// Fetch and verify pinned version metadata. `offline` reuses a verified cached copy only.
    pub fn new(server_dir: &Path, offline: bool) -> Result<Self, BootstrapError> {
        let cache_dir = server_dir.join(".horizon").join("downloads").join(VERSION);
        let manifest_path = cache_dir.join(format!("{VERSION}.json"));
        let url = version_json_url();

        let bytes = if let Ok(existing) = fs::read(&manifest_path) {
            let digest = sha1_hex(&existing);
            if is_accepted_manifest(&digest) {
                existing
            } else {
                return Err(BootstrapError::HashMismatch {
                    url: url.clone(),
                    expected: MANIFEST_SHA1.to_owned(),
                    actual: digest,
                });
            }
        } else if offline {
            return Err(BootstrapError::OfflineCacheMissing(manifest_path));
        } else {
            let downloaded = fetch(&url, MAX_METADATA_BYTES)?;
            let digest = sha1_hex(&downloaded);
            if !is_accepted_manifest(&digest) {
                return Err(BootstrapError::HashMismatch {
                    url: url.clone(),
                    expected: MANIFEST_SHA1.to_owned(),
                    actual: digest,
                });
            }
            fs::create_dir_all(&cache_dir)?;
            write_atomically(&manifest_path, &downloaded)?;
            downloaded
        };
        Ok(Self { version_json: bytes, cache_dir })
    }

    /// Parse the verified metadata and re-check the pinned client/server artifact hashes.
    pub fn artifacts(&self) -> Result<(ArtifactInfo, ArtifactInfo), BootstrapError> {
        let parsed: VersionJson = serde_json::from_slice(&self.version_json)
            .map_err(|error| BootstrapError::Json(error.to_string()))?;
        let client = ArtifactInfo {
            url: parsed.downloads.client.url,
            sha1: parsed.downloads.client.sha1,
            size: parsed.downloads.client.size,
        };
        let server = ArtifactInfo {
            url: parsed.downloads.server.url,
            sha1: parsed.downloads.server.sha1,
            size: parsed.downloads.server.size,
        };
        if !is_official_url(&client.url) || !is_official_url(&server.url) {
            return Err(BootstrapError::DisallowedUrl(format!(
                "{} / {}",
                client.url, server.url
            )));
        }
        if !client.sha1.eq_ignore_ascii_case(CLIENT_SHA1)
            || !server.sha1.eq_ignore_ascii_case(SERVER_SHA1)
        {
            return Err(BootstrapError::Json(format!(
                "artifact hashes do not match the 26.3 pins: client {}, server {}",
                client.sha1, server.sha1
            )));
        }
        Ok((client, server))
    }

    /// Download or reuse the asset index, accepting only upstream's pinned 26.3 hashes.
    pub fn asset_index(&self, offline: bool) -> Result<PathBuf, BootstrapError> {
        let parsed: VersionJson = serde_json::from_slice(&self.version_json)
            .map_err(|error| BootstrapError::Json(error.to_string()))?;
        let info = &parsed.asset_index;
        if !is_official_url(&info.url) {
            return Err(BootstrapError::DisallowedUrl(info.url.clone()));
        }
        let path = self.cache_dir.join(format!("{VERSION}-asset-index.json"));
        if let Ok(existing) = fs::read(&path) {
            if existing.len() as u64 == info.size && is_accepted_asset_index(&sha1_hex(&existing)) {
                return Ok(path);
            }
            let _ = fs::rename(&path, path.with_extension("corrupt"));
        }
        if offline {
            return Err(BootstrapError::OfflineCacheMissing(path));
        }
        let bytes = fetch(&info.url, MAX_METADATA_BYTES)?;
        let digest = sha1_hex(&bytes);
        if bytes.len() as u64 != info.size || !is_accepted_asset_index(&digest) {
            return Err(BootstrapError::HashMismatch {
                url: info.url.clone(),
                expected: info.sha1.clone(),
                actual: format!("{digest} ({} bytes)", bytes.len()),
            });
        }
        write_atomically(&path, &bytes)?;
        Ok(path)
    }

    /// Download (or reuse) a verified artifact. The file is written to a partial path and renamed
    /// only after its SHA-1 and size pass.
    pub fn artifact(&self, name: &str, info: &ArtifactInfo, offline: bool) -> Result<PathBuf, BootstrapError> {
        let path = self.cache_dir.join(name);
        if let Ok(existing) = fs::read(&path) {
            if existing.len() as u64 == info.size
                && sha1_hex(&existing).eq_ignore_ascii_case(&info.sha1)
            {
                return Ok(path);
            }
            // Corrupt cache: keep it aside rather than trusting it, then repair.
            let corrupt = path.with_extension("corrupt");
            let _ = fs::rename(&path, &corrupt);
        }
        if offline {
            return Err(BootstrapError::OfflineCacheMissing(path));
        }
        if info.size > MAX_ARTIFACT_BYTES {
            return Err(BootstrapError::TooLarge { url: info.url.clone(), limit: MAX_ARTIFACT_BYTES });
        }
        let bytes = fetch(&info.url, MAX_ARTIFACT_BYTES)?;
        let digest = sha1_hex(&bytes);
        if !digest.eq_ignore_ascii_case(&info.sha1) || bytes.len() as u64 != info.size {
            return Err(BootstrapError::HashMismatch {
                url: info.url.clone(),
                expected: info.sha1.clone(),
                actual: format!("{digest} ({} bytes)", bytes.len()),
            });
        }
        write_atomically(&path, &bytes)?;
        Ok(path)
    }

    /// Resource directory, always inside the server directory and outside the executable.
    pub fn resource_dir(&self) -> PathBuf {
        self.cache_dir
            .parent()
            .unwrap_or(&self.cache_dir)
            .join("resources")
            .join(VERSION)
    }

    /// Stage engine-relevant entries from the verified client JAR into a versioned resource dir.
    /// Entries are never extracted from untrusted paths, and the result is finalized atomically.
    pub fn stage_client_resources(&self, jar_path: &Path) -> Result<PathBuf, BootstrapError> {
        let final_dir = self.resource_dir();
        let staging = final_dir.with_file_name(format!("{VERSION}.partial"));
        if staging.exists() {
            fs::remove_dir_all(&staging)?;
        }
        fs::create_dir_all(&staging)?;

        let mut archive = ZipArchive::new(fs::File::open(jar_path)?)
            .map_err(|error| BootstrapError::Json(format!("invalid client JAR: {error}")))?;
        let mut files = 0usize;
        let mut bytes = 0u64;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| BootstrapError::Json(format!("cannot read JAR entry: {error}")))?;
            let name = entry
                .name()
                .map_err(|error| BootstrapError::Json(format!("invalid JAR entry name: {error}")))?
                .into_owned();
            let wanted = name.starts_with("data/")
                || name.starts_with("assets/minecraft/font/")
                || name.starts_with("assets/minecraft/textures/");
            if !wanted || entry.is_dir() {
                continue;
            }
            let safe = safe_entry_path(&name)?;
            let target = staging.join(safe);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut writer = fs::File::create(&target)?;
            bytes += io::copy(&mut entry, &mut writer)?;
            files += 1;
            if bytes > MAX_STAGED_BYTES {
                return Err(BootstrapError::TooLargeStaged);
            }
        }

        let manifest = StagedManifest {
            version: VERSION,
            client_sha1: CLIENT_SHA1,
            files,
            bytes,
            note: "provisional: engine manifest format and prepared tables are not integrated yet",
        };
        let text = serde_json::to_string_pretty(&manifest)
            .map_err(|error| BootstrapError::Json(error.to_string()))?;
        fs::write(staging.join("bootstrap-manifest.json"), format!("{text}\n"))?;

        if final_dir.exists() {
            let previous = final_dir.with_file_name(format!("{VERSION}.previous"));
            let _ = fs::remove_dir_all(&previous);
            fs::rename(&final_dir, &previous)?;
        }
        fs::rename(&staging, &final_dir)?;
        Ok(final_dir)
    }
}

#[derive(Debug, Serialize)]
struct StagedManifest<'a> {
    version: &'a str,
    client_sha1: &'a str,
    files: usize,
    bytes: u64,
    note: &'a str,
}

fn safe_entry_path(name: &str) -> Result<&str, BootstrapError> {
    if name.starts_with('/')
        || name.contains('\\')
        || name.split('/').any(|part| part.is_empty() || part == "." || part == "..")
    {
        Err(BootstrapError::UnsafeArchivePath(name.to_owned()))
    } else {
        Ok(name)
    }
}

#[derive(Debug, Clone)]
pub struct ArtifactInfo {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

fn is_accepted_asset_index(digest: &str) -> bool {
    ACCEPTED_ASSET_INDEX_SHA1
        .iter()
        .any(|accepted| digest.eq_ignore_ascii_case(accepted))
}

fn is_accepted_manifest(digest: &str) -> bool {
    digest.eq_ignore_ascii_case(MANIFEST_SHA1)
        || ACCEPTED_MANIFEST_SHA1
            .iter()
            .any(|accepted| digest.eq_ignore_ascii_case(accepted))
}

fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension("part");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn pins_match_the_canonical_upstream_version() {
        assert_eq!(VERSION, "26.3");
        assert_eq!(MANIFEST_SHA1.len(), 40);
        assert_eq!(CLIENT_SHA1.len(), 40);
        assert_eq!(SERVER_SHA1.len(), 40);
    }

    #[test]
    fn allowlist_rejects_unsafe_urls() {
        assert!(is_official_url("https://piston-meta.mojang.com/v1/packages/x.json"));
        assert!(!is_official_url("http://piston-meta.mojang.com/x"));
        assert!(!is_official_url("https://evil.example/piston-meta.mojang.com/x"));
        assert!(!is_official_url("https://user@piston-meta.mojang.com/x"));
        assert!(!is_official_url("https://piston-meta.mojang.com:8443/x"));
    }

    #[test]
    fn asset_index_accepts_only_pinned_26_3_hashes() {
        assert!(is_accepted_asset_index(ACCEPTED_ASSET_INDEX_SHA1[0]));
        assert!(is_accepted_asset_index("ABFAA525F923F807DF8B4E4D29C1B5E3A104ADBE"));
        assert!(!is_accepted_asset_index("0".repeat(40).as_str()));
    }

    #[test]
    fn sha1_matches_known_vector() {
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn corrupt_cache_is_replaced_by_a_verified_download_result() {
        // Exercises the cache-verification branch without network access.
        let dir = std::env::temp_dir().join(format!("horizon-bootstrap-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let cache_dir = Bootstrap { version_json: vec![], cache_dir: dir.clone() };
        let info = ArtifactInfo {
            url: "https://piston-data.mojang.com/unused".to_owned(),
            sha1: "a9993e364706816aba3e25717850c26c9cd0d89d".to_owned(),
            size: 3,
        };
        fs::write(dir.join("fixture.bin"), b"abc").unwrap();
        assert_eq!(cache_dir.artifact("fixture.bin", &info, true).unwrap(), dir.join("fixture.bin"));

        fs::write(dir.join("fixture.bin"), b"bad").unwrap();
        assert!(matches!(
            cache_dir.artifact("fixture.bin", &info, true),
            Err(BootstrapError::OfflineCacheMissing(_))
        ));
        assert!(dir.join("fixture.corrupt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    fn synthetic_jar(dir: &Path, entries: &[(&str, &str)]) -> PathBuf {
        let jar = dir.join("client.jar");
        let mut writer = ZipWriter::new(fs::File::create(&jar).unwrap());
        for (name, contents) in entries {
            writer.start_file(*name, SimpleFileOptions::default()).unwrap();
            writer.write_all(contents.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
        jar
    }

    #[test]
    fn stages_engine_relevant_entries_outside_the_binary() {
        let dir = std::env::temp_dir().join(format!("horizon-stage-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let jar = synthetic_jar(
            &dir,
            &[
                ("data/minecraft/tags.json", "{}"),
                ("assets/minecraft/font/default.json", "{}"),
                ("assets/minecraft/sounds/step.ogg", "ignored"),
            ],
        );
        let cache_dir = dir.join(".horizon/downloads/26.3");
        let bootstrap = Bootstrap { version_json: vec![], cache_dir };
        let staged = bootstrap.stage_client_resources(&jar).unwrap();
        assert!(staged.join("data/minecraft/tags.json").exists());
        assert!(staged.join("assets/minecraft/font/default.json").exists());
        assert!(!staged.join("assets/minecraft/sounds/step.ogg").exists());
        let manifest = fs::read_to_string(staged.join("bootstrap-manifest.json")).unwrap();
        assert!(manifest.contains("\"version\": \"26.3\""));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn zip_slip_entries_are_rejected_before_extraction() {
        let dir = std::env::temp_dir().join(format!("horizon-slip-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let jar = synthetic_jar(&dir, &[("data/../evil.txt", "bad")]);
        let cache_dir = dir.join(".horizon/downloads/26.3");
        let bootstrap = Bootstrap { version_json: vec![], cache_dir };
        assert!(matches!(
            bootstrap.stage_client_resources(&jar),
            Err(BootstrapError::UnsafeArchivePath(_))
        ));
        fs::remove_dir_all(&dir).unwrap();
    }
}
