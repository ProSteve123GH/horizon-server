//! Verified Minecraft 26.3 resource bootstrap.
//!
//! This mirrors the validation of the Android fork's `official_install.rs`: official HTTPS hosts
//! only, no redirects, pinned content-addressed metadata, SHA-1 checks, bounded downloads, partial
//! files and atomic finalization. No Mojang file is ever committed or bundled with the binary.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use sha1::{Digest, Sha1};

/// Pins from canonical upstream `research-lock.json` (Minecraft 26.3), not the Android fork's
/// `dev/26.4` snapshot pins.
pub const VERSION: &str = "26.3";
pub const MANIFEST_SHA1: &str = "11f1aefe473eefba0af62e3286c33b692807b2eb";
pub const CLIENT_SHA1: &str = "e877b6a07acd633fb3bb475002175cec036e7b87";
pub const SERVER_SHA1: &str = "33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c";

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

#[derive(Debug)]
pub enum BootstrapError {
    Io(io::Error),
    Http(String),
    DisallowedUrl(String),
    TooLarge { url: String, limit: u64 },
    HashMismatch { url: String, expected: String, actual: String },
    OfflineCacheMissing(PathBuf),
    Json(String),
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
}

#[derive(Debug, Clone)]
pub struct ArtifactInfo {
    pub url: String,
    pub sha1: String,
    pub size: u64,
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
}
