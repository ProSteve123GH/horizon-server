//! Owner-approved interim "prepared data" source.
//!
//! A prepared-data folder is any local directory containing a `prepared-manifest.json` at its root.
//! The manifest lists the files that make up the prepared dataset together with their exact byte
//! size and SHA-256 digest. This module never downloads anything, never invokes Java or any other
//! executable, and never follows symlinks named by a manifest.
//!
//! Acceptance is a two-step, crash-safe hand-off:
//!   1. [`verify_folder`] validates a source folder in place.
//!   2. [`accept_folder`] copies *only the manifest-listed files* into a staging directory
//!      (`<server_dir>/.horizon/prepared/.partial`), re-verifies the copy, then atomically renames
//!      it to `<server_dir>/.horizon/prepared/<version>`, keeping the old version in `.previous`.
//!
//! [`load_accepted`] re-verifies the stored files on every access, so a mutated or partially
//! written dataset is never trusted.

use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use zip::ZipArchive;

/// Manifest file that must sit at the root of a prepared-data folder.
pub const MANIFEST_FILE_NAME: &str = "prepared-manifest.json";

const PREPARED_DIR: &str = "prepared";
const HORIZON_DIR: &str = ".horizon";
const STAGING_DIR: &str = ".partial";
const PREVIOUS_DIR: &str = ".previous";

/// Top-level contents of `prepared-manifest.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedManifest {
    pub version: String,
    pub files: Vec<PreparedFile>,
}

/// One verified file entry in a [`PreparedManifest`]. `path` is a forward-slash relative path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

/// An accepted, verified prepared dataset and the directory that holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedData {
    pub root: PathBuf,
    pub manifest: PreparedManifest,
}

/// All failures produced by this module. I/O errors preserve their source for diagnostics.
#[derive(Debug)]
pub enum PreparedError {
    Io(io::Error),
    Json(String),
    VersionMismatch { expected: String, found: String },
    InvalidVersion(String),
    InvalidPath { path: String, reason: String },
    DuplicatePath(String),
    NotDirectory(String),
    Symlink(String),
    MissingFile(String),
    NotAFile(String),
    SizeMismatch { path: String, expected: u64, found: u64 },
    InvalidHash { path: String, hash: String },
    HashMismatch { path: String, expected: String, found: String },
}

impl fmt::Display for PreparedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "invalid {MANIFEST_FILE_NAME}: {error}"),
            Self::VersionMismatch { expected, found } => write!(
                formatter,
                "prepared data version mismatch: expected {expected}, manifest declares {found}"
            ),
            Self::InvalidVersion(version) => {
                write!(formatter, "unsafe prepared-data version identifier: {version:?}")
            }
            Self::InvalidPath { path, reason } => {
                write!(formatter, "unsafe manifest path {path:?}: {reason}")
            }
            Self::DuplicatePath(path) => {
                write!(formatter, "manifest lists {path:?} more than once")
            }
            Self::NotDirectory(path) => write!(formatter, "not a directory: {path}"),
            Self::Symlink(path) => write!(formatter, "refusing symlink in prepared path: {path}"),
            Self::MissingFile(path) => write!(formatter, "missing prepared file: {path}"),
            Self::NotAFile(path) => write!(formatter, "prepared path is not a regular file: {path}"),
            Self::SizeMismatch { path, expected, found } => write!(
                formatter,
                "size mismatch for {path}: expected {expected} bytes, found {found}"
            ),
            Self::InvalidHash { path, hash } => {
                write!(formatter, "invalid SHA-256 for {path:?}: {hash:?}")
            }
            Self::HashMismatch { path, expected, found } => write!(
                formatter,
                "SHA-256 mismatch for {path}: expected {expected}, found {found}"
            ),
        }
    }
}

impl std::error::Error for PreparedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for PreparedError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

// ---------------------------------------------------------------------------
// Path / version validation
// ---------------------------------------------------------------------------

/// Validate that `version` is a single, harmless path component.
fn sanitize_version(version: &str) -> Result<(), PreparedError> {
    if version.is_empty()
        || version == "."
        || version == ".."
        || version.contains('/')
        || version.contains('\\')
        || version.contains('\0')
        || version.contains("//")
        || version.starts_with('/')
        || Path::new(version).is_absolute()
    {
        return Err(PreparedError::InvalidVersion(version.to_owned()));
    }
    Ok(())
}

/// Validate a manifest path and return its normalized forward-slash form.
///
/// Rejects empty paths, `..`, absolute paths (including Windows-style drive prefixes),
/// backslashes, NUL bytes, redundant separators and `.` components. The normalized form must be
/// identical to the raw string, which also rules out duplicate-path aliases.
fn validate_relative_path(raw: &str) -> Result<String, PreparedError> {
    let invalid = |reason: &str| PreparedError::InvalidPath {
        path: raw.to_owned(),
        reason: reason.to_owned(),
    };

    if raw.is_empty() {
        return Err(invalid("path is empty"));
    }
    if raw.contains('\0') {
        return Err(invalid("NUL byte in path"));
    }
    if raw.contains('\\') {
        return Err(invalid("backslashes are not allowed (use '/')"));
    }
    if raw.starts_with('/') || Path::new(raw).is_absolute() {
        return Err(invalid("absolute paths are not allowed"));
    }
    if raw.starts_with("//") || raw.contains("//") {
        return Err(invalid("redundant separators are not allowed"));
    }
    if raw.ends_with('/') {
        return Err(invalid("path must name a file, not a directory"));
    }
    // Windows drive-letter or UNC-style prefix, even when parsed on a Unix host.
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Err(invalid("drive-letter prefixes are not allowed"));
    }

    let mut normalized = Vec::new();
    for component in Path::new(raw).components() {
        match component {
            Component::Normal(part) => normalized.push(part.to_string_lossy().into_owned()),
            Component::ParentDir => return Err(invalid("'..' components are not allowed")),
            Component::CurDir => return Err(invalid("'.' components are not allowed")),
            Component::RootDir | Component::Prefix(_) => {
                return Err(invalid("absolute paths are not allowed"));
            }
        }
    }
    let normalized = normalized.join("/");
    if normalized != raw {
        return Err(invalid("path is not in canonical relative form"));
    }
    Ok(normalized)
}

/// Reject symlinks anywhere along `root/<relative>` and require intermediate components to be
/// directories. This prevents a manifest entry from escaping the folder via a linked ancestor.
fn ensure_no_symlinks(root: &Path, relative: &str) -> Result<(), PreparedError> {
    let mut current = root.to_path_buf();
    for component in Path::new(relative).components() {
        current.push(component);
        let metadata = fs::symlink_metadata(&current)?;
        if metadata.file_type().is_symlink() {
            return Err(PreparedError::Symlink(current.to_string_lossy().into_owned()));
        }
    }
    Ok(())
}

/// Parse and structurally validate a manifest, returning its canonicalized file list.
fn read_manifest(folder: &Path) -> Result<(PreparedManifest, Vec<String>), PreparedError> {
    let manifest_path = folder.join(MANIFEST_FILE_NAME);
    let manifest_meta = fs::symlink_metadata(&manifest_path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            PreparedError::MissingFile(MANIFEST_FILE_NAME.to_owned())
        } else {
            PreparedError::Io(error)
        }
    })?;
    if manifest_meta.file_type().is_symlink() {
        return Err(PreparedError::Symlink(manifest_path.to_string_lossy().into_owned()));
    }
    if !manifest_meta.is_file() {
        return Err(PreparedError::NotAFile(MANIFEST_FILE_NAME.to_owned()));
    }

    let bytes = fs::read(&manifest_path)?;
    let manifest: PreparedManifest =
        serde_json::from_slice(&bytes).map_err(|error| PreparedError::Json(error.to_string()))?;

    let paths = manifest_paths(&manifest)?;
    Ok((manifest, paths))
}

/// Validate every manifest path once, returning canonical relative paths.
fn manifest_paths(manifest: &PreparedManifest) -> Result<Vec<String>, PreparedError> {
    let mut paths = Vec::with_capacity(manifest.files.len());
    for file in &manifest.files {
        let canonical = validate_relative_path(&file.path)?;
        if paths.iter().any(|seen| seen == &canonical) {
            return Err(PreparedError::DuplicatePath(canonical));
        }
        paths.push(canonical);
    }
    Ok(paths)
}

/// Reject malformed digests up front so a bad manifest cannot silently bypass verification.
fn validate_hash_shape(path: &str, hash: &str) -> Result<(), PreparedError> {
    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(PreparedError::InvalidHash {
            path: path.to_owned(),
            hash: hash.to_owned(),
        })
    }
}

// ---------------------------------------------------------------------------
// Verification
// ---------------------------------------------------------------------------

/// Verify a prepared-data folder without modifying it.
///
/// Checks the manifest version, path safety, file existence, exact byte size and SHA-256 digest of
/// every listed file. Symlinks (including symlinked parent components) are always rejected.
pub fn verify_folder(source: &Path, version: &str) -> Result<PreparedManifest, PreparedError> {
    sanitize_version(version)?;

    let root_meta = fs::symlink_metadata(source).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            PreparedError::NotDirectory(source.to_string_lossy().into_owned())
        } else {
            PreparedError::Io(error)
        }
    })?;
    if root_meta.file_type().is_symlink() {
        return Err(PreparedError::Symlink(source.to_string_lossy().into_owned()));
    }
    if !root_meta.is_dir() {
        return Err(PreparedError::NotDirectory(source.to_string_lossy().into_owned()));
    }

    let (manifest, paths) = read_manifest(source)?;
    if manifest.version != version {
        return Err(PreparedError::VersionMismatch {
            expected: version.to_owned(),
            found: manifest.version.clone(),
        });
    }

    for (file, relative) in manifest.files.iter().zip(paths.iter()) {
        ensure_no_symlinks(source, relative)?;
        let path = source.join(relative);
        let metadata = fs::metadata(&path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                PreparedError::MissingFile(relative.clone())
            } else {
                PreparedError::Io(error)
            }
        })?;
        if !metadata.is_file() {
            return Err(PreparedError::NotAFile(relative.clone()));
        }
        if metadata.len() != file.size {
            return Err(PreparedError::SizeMismatch {
                path: relative.clone(),
                expected: file.size,
                found: metadata.len(),
            });
        }
        validate_hash_shape(relative, &file.sha256)?;
        let actual = sha256_file(&path)?;
        if !actual.eq_ignore_ascii_case(&file.sha256) {
            return Err(PreparedError::HashMismatch {
                path: relative.clone(),
                expected: file.sha256.clone(),
                found: actual,
            });
        }
    }

    Ok(manifest)
}

// ---------------------------------------------------------------------------
// Acceptance / loading
// ---------------------------------------------------------------------------

fn prepared_root(server_dir: &Path) -> PathBuf {
    server_dir.join(HORIZON_DIR).join(PREPARED_DIR)
}

/// Copy one verified file into the staging tree, creating parent directories as needed.
fn copy_listed_file(source: &Path, destination: &Path) -> Result<(), PreparedError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

/// Atomically install the staged tree at `target`, preserving any previous tree in `.previous`.
fn commit_staged(root: &Path, stage: &Path, target: &Path) -> Result<(), PreparedError> {
    let previous = root.join(PREVIOUS_DIR);

    if previous.exists() {
        fs::remove_dir_all(&previous)?;
    }

    let moved_old = if target.is_dir() {
        fs::rename(target, &previous)?;
        true
    } else {
        if target.exists() {
            fs::remove_file(target)?;
        }
        false
    };

    if let Err(error) = fs::rename(stage, target) {
        // Roll back the previous version so acceptance never destroys a working dataset.
        if moved_old {
            let _ = fs::rename(&previous, target);
        }
        return Err(PreparedError::Io(error));
    }
    Ok(())
}

/// Verify and atomically install a local prepared-data folder for `version`.
///
/// Only files named by the manifest are copied; the staging directory is re-verified before the
/// atomic rename so a torn copy can never become the accepted dataset.
pub fn accept_folder(
    server_dir: &Path,
    source: &Path,
    version: &str,
) -> Result<PreparedData, PreparedError> {
    sanitize_version(version)?;
    let manifest = verify_folder(source, version)?;
    let (_, paths) = read_manifest(source)?;

    let root = prepared_root(server_dir);
    fs::create_dir_all(&root)?;
    let target = root.join(version);
    let stage = root.join(STAGING_DIR);

    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    if let Err(error) = (|| -> Result<(), PreparedError> {
        fs::create_dir_all(&stage)?;
        fs::copy(
            source.join(MANIFEST_FILE_NAME),
            stage.join(MANIFEST_FILE_NAME),
        )?;
        for relative in &paths {
            copy_listed_file(&source.join(relative), &stage.join(relative))?;
        }
        // Re-verify the copy byte-for-byte before it is published.
        verify_folder(&stage, version)?;
        Ok(())
    })() {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }

    if let Err(error) = commit_staged(&root, &stage, &target) {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }

    Ok(PreparedData { root: target, manifest })
}

/// Install a prepared dataset from a CI-generated ZIP archive.
///
/// The archive must contain `prepared-manifest.json` and every listed file at its exact relative
/// path. Only listed files are extracted, unsafe paths and symlink entries are rejected, every file
/// is hash/size checked, and the staged tree is re-verified before atomic publication.
pub fn accept_bundle(
    server_dir: &Path,
    archive: &Path,
    version: &str,
) -> Result<PreparedData, PreparedError> {
    sanitize_version(version)?;
    let root = prepared_root(server_dir);
    fs::create_dir_all(&root)?;
    let target = root.join(version);
    let stage = root.join(STAGING_DIR);
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }

    let manifest = (|| -> Result<PreparedManifest, PreparedError> {
        fs::create_dir_all(&stage)?;
        let file = fs::File::open(archive)?;
        let mut zip = ZipArchive::new(file)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        let mut manifest_entry = zip
            .by_name(MANIFEST_FILE_NAME)
            .map_err(|_| PreparedError::MissingFile(MANIFEST_FILE_NAME.to_owned()))?;
        if manifest_entry.is_dir() || is_symlink_entry(&manifest_entry) {
            return Err(PreparedError::Symlink(MANIFEST_FILE_NAME.to_owned()));
        }
        let mut bytes = Vec::new();
        manifest_entry.read_to_end(&mut bytes)?;
        let manifest: PreparedManifest =
            serde_json::from_slice(&bytes).map_err(|error| PreparedError::Json(error.to_string()))?;
        if manifest.version != version {
            return Err(PreparedError::VersionMismatch {
                expected: version.to_owned(),
                found: manifest.version.clone(),
            });
        }
        let paths = manifest_paths(&manifest)?;
        fs::write(stage.join(MANIFEST_FILE_NAME), &bytes)?;
        drop(manifest_entry);

        for (file, relative) in manifest.files.iter().zip(paths.iter()) {
            let mut entry = zip
                .by_name(relative)
                .map_err(|_| PreparedError::MissingFile(relative.clone()))?;
            if entry.is_dir() || is_symlink_entry(&entry) {
                return Err(PreparedError::Symlink(relative.clone()));
            }
            validate_hash_shape(relative, &file.sha256)?;
            let destination = stage.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut written = fs::File::create(&destination)?;
            let copied = io::copy(&mut entry, &mut written)?;
            if copied != file.size {
                return Err(PreparedError::SizeMismatch {
                    path: relative.clone(),
                    expected: file.size,
                    found: copied,
                });
            }
            if !sha256_file(&destination)?.eq_ignore_ascii_case(&file.sha256) {
                return Err(PreparedError::HashMismatch {
                    path: relative.clone(),
                    expected: file.sha256.clone(),
                    found: sha256_file(&destination)?,
                });
            }
        }
        Ok(manifest)
    })();

    let manifest = match manifest {
        Ok(manifest) => manifest,
        Err(error) => {
            let _ = fs::remove_dir_all(&stage);
            return Err(error);
        }
    };
    if let Err(error) = verify_folder(&stage, version).and_then(|_| commit_staged(&root, &stage, &target)) {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }
    Ok(PreparedData { root: target, manifest })
}

/// Reject ZIP entries whose Unix mode marks them as symlinks (zip-slip via symlink).
fn is_symlink_entry<R: Read + ?Sized>(entry: &zip::read::ZipFile<'_, R>) -> bool {
    entry
        .unix_mode()
        .is_some_and(|mode| mode & 0o170000 == 0o120000)
}

/// Load the accepted dataset for `version`, re-verifying every file before returning it.
///
/// Returns `Ok(None)` when no dataset has been accepted for that version.
pub fn load_accepted(
    server_dir: &Path,
    version: &str,
) -> Result<Option<PreparedData>, PreparedError> {
    sanitize_version(version)?;
    let target = prepared_root(server_dir).join(version);
    if !target.exists() {
        return Ok(None);
    }
    let manifest = verify_folder(&target, version)?;
    Ok(Some(PreparedData { root: target, manifest }))
}

// ---------------------------------------------------------------------------
// Self-contained SHA-256 (no new crate dependency)
// ---------------------------------------------------------------------------
//
// `sha2` is not a direct dependency and is not present in Cargo.lock, so a compact, dependency-
// free implementation lives here. It streams data and is validated against published vectors in
// the tests below.

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    total_len: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
                0x1f83d9ab, 0x5be0cd19,
            ],
            buffer: [0; 64],
            buffer_len: 0,
            total_len: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.total_len = self.total_len.wrapping_add(data.len() as u64);
        if self.buffer_len > 0 {
            let take = (64 - self.buffer_len).min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&data[..take]);
            self.buffer_len += take;
            data = &data[take..];
            if self.buffer_len == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
            }
        }
        while data.len() >= 64 {
            let block: [u8; 64] = data[..64].try_into().expect("slice of exactly 64 bytes");
            self.compress(&block);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buffer[..data.len()].copy_from_slice(data);
            self.buffer_len = data.len();
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut words = [0u32; 64];
        for (index, chunk) in block.chunks_exact(4).enumerate() {
            words[index] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for index in 0..64 {
            let sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sigma1)
                .wrapping_add(choice)
                .wrapping_add(SHA256_K[index])
                .wrapping_add(words[index]);
            let sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sigma0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }

    fn finalize(mut self) -> [u8; 32] {
        let bit_length = self.total_len.wrapping_mul(8);
        self.update(&[0x80]);
        while self.buffer_len != 56 {
            self.update(&[0u8]);
        }
        self.update(&bit_length.to_be_bytes());

        let mut digest = [0u8; 32];
        for (index, word) in self.state.iter().enumerate() {
            digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        digest
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 0x0f) as usize] as char);
    }
    text
}

fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex_encode(&hasher.finalize())
}

fn sha256_file(path: &Path) -> Result<String, PreparedError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => hasher.update(&chunk[..count]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(PreparedError::Io(error)),
        }
    }
    Ok(hex_encode(&hasher.finalize()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn scratch(label: &str) -> PathBuf {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "horizon-prepared-test-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_synthetic_source(base: &Path, files: &[(&str, &str)], version: &str) -> PathBuf {
        let source = base.join("source");
        let mut listed = Vec::new();
        for (relative, contents) in files {
            let path = source.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, contents).unwrap();
            listed.push(PreparedFile {
                path: (*relative).to_owned(),
                sha256: sha256_bytes(contents.as_bytes()),
                size: contents.len() as u64,
            });
        }
        let manifest = PreparedManifest {
            version: version.to_owned(),
            files: listed,
        };
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join(MANIFEST_FILE_NAME),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        source
    }

    #[test]
    fn sha256_matches_published_vectors() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // 56-byte input exercises the two-block padding path.
        assert_eq!(
            sha256_bytes(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn verify_folder_accepts_a_valid_manifest() {
        let base = scratch("valid");
        let source = write_synthetic_source(
            &base,
            &[("top.txt", "hello"), ("nested/data.bin", "binary-ish")],
            "26.3",
        );
        let manifest = verify_folder(&source, "26.3").unwrap();
        assert_eq!(manifest.version, "26.3");
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(manifest.files[0].path, "top.txt");
        assert_eq!(manifest.files[1].path, "nested/data.bin");
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn verify_folder_rejects_bad_hash_size_and_version() {
        let base = scratch("bad");
        let source = write_synthetic_source(&base, &[("a.txt", "abc")], "26.3");
        let manifest_path = source.join(MANIFEST_FILE_NAME);

        assert!(matches!(
            verify_folder(&source, "26.4"),
            Err(PreparedError::VersionMismatch { .. })
        ));

        let mut manifest: PreparedManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest.files[0].sha256 = "00".repeat(32);
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(matches!(
            verify_folder(&source, "26.3"),
            Err(PreparedError::HashMismatch { .. })
        ));

        manifest.files[0].sha256 = "not-a-hex-digest".to_owned();
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(matches!(
            verify_folder(&source, "26.3"),
            Err(PreparedError::InvalidHash { .. })
        ));

        manifest.files[0].sha256 = sha256_bytes(b"abc");
        manifest.files[0].size += 1;
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(matches!(
            verify_folder(&source, "26.3"),
            Err(PreparedError::SizeMismatch { .. })
        ));
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn verify_folder_rejects_unsafe_paths() {
        let base = scratch("paths");
        for (index, bad) in [
            "../escape.txt",
            "/etc/passwd",
            "a\\b.txt",
            "a/../b.txt",
            "a//b.txt",
            "./a.txt",
            "C:/secret.txt",
            "",
        ]
        .into_iter()
        .enumerate()
        {
            let name = format!("case-{index}");
            let source = write_synthetic_source(&base.join(name), &[], "26.3");
            let manifest = PreparedManifest {
                version: "26.3".to_owned(),
                files: vec![PreparedFile {
                    path: bad.to_owned(),
                    sha256: sha256_bytes(b""),
                    size: 0,
                }],
            };
            fs::write(
                source.join(MANIFEST_FILE_NAME),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            assert!(
                verify_folder(&source, "26.3").is_err(),
                "path {bad:?} should have been rejected"
            );
        }
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn verify_folder_rejects_duplicate_paths() {
        let base = scratch("dupe");
        let source = write_synthetic_source(&base, &[("a.txt", "x")], "26.3");
        let manifest_path = source.join(MANIFEST_FILE_NAME);
        let mut manifest: PreparedManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        let first = manifest.files[0].clone();
        manifest.files.push(first);
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(matches!(
            verify_folder(&source, "26.3"),
            Err(PreparedError::DuplicatePath(_))
        ));
        fs::remove_dir_all(&base).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn verify_folder_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let base = scratch("symlink");
        let source = write_synthetic_source(&base, &[("real.txt", "secret")], "26.3");
        let outside = base.join("outside.txt");
        fs::write(&outside, "secret").unwrap();

        // Manifest names a symlink directly.
        let link = source.join("link.txt");
        symlink(&outside, &link).unwrap();
        let manifest_path = source.join(MANIFEST_FILE_NAME);
        let mut manifest: PreparedManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest.files.push(PreparedFile {
            path: "link.txt".to_owned(),
            sha256: sha256_bytes(b"secret"),
            size: 6,
        });
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(matches!(
            verify_folder(&source, "26.3"),
            Err(PreparedError::Symlink(_))
        ));

        // Manifest path traverses a symlinked parent directory.
        fs::remove_file(&link).unwrap();
        let linked_dir = source.join("linked");
        symlink(&base, &linked_dir).unwrap();
        manifest.files.pop();
        manifest.files.push(PreparedFile {
            path: "linked/outside.txt".to_owned(),
            sha256: sha256_bytes(b"secret"),
            size: 6,
        });
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(matches!(
            verify_folder(&source, "26.3"),
            Err(PreparedError::Symlink(_))
        ));
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn accept_then_load_round_trip() {
        let base = scratch("roundtrip");
        let server = base.join("server");
        let source = write_synthetic_source(
            &base,
            &[("a.txt", "alpha"), ("dir/b.txt", "beta")],
            "26.3",
        );

        let accepted = accept_folder(&server, &source, "26.3").unwrap();
        assert_eq!(accepted.root, server.join(".horizon/prepared/26.3"));
        assert_eq!(accepted.manifest.files.len(), 2);
        assert!(accepted.root.join("dir/b.txt").is_file());
        assert!(!server.join(".horizon/prepared/.partial").exists());

        let loaded = load_accepted(&server, "26.3").unwrap().unwrap();
        assert_eq!(loaded, accepted);
        assert_eq!(
            fs::read_to_string(loaded.root.join("a.txt")).unwrap(),
            "alpha"
        );
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn reaccepting_keeps_previous_version() {
        let base = scratch("previous");
        let server = base.join("server");
        let first = write_synthetic_source(&base.join("v1"), &[("a.txt", "old")], "26.3");
        let second = write_synthetic_source(&base.join("v2"), &[("a.txt", "new")], "26.3");

        accept_folder(&server, &first, "26.3").unwrap();
        accept_folder(&server, &second, "26.3").unwrap();

        let root = server.join(".horizon/prepared");
        assert_eq!(fs::read_to_string(root.join("26.3/a.txt")).unwrap(), "new");
        assert_eq!(
            fs::read_to_string(root.join(".previous/a.txt")).unwrap(),
            "old"
        );
        assert!(!root.join(".partial").exists());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn load_accepted_reverifies_and_returns_none_when_missing() {
        let base = scratch("reverify");
        let server = base.join("server");
        assert!(load_accepted(&server, "26.3").unwrap().is_none());

        let source = write_synthetic_source(&base, &[("a.txt", "original")], "26.3");
        accept_folder(&server, &source, "26.3").unwrap();
        let stored = server.join(".horizon/prepared/26.3/a.txt");
        fs::write(&stored, "tampered").unwrap();
        assert!(matches!(
            load_accepted(&server, "26.3"),
            Err(PreparedError::HashMismatch { .. })
        ));
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn rejects_unsafe_version_identifiers() {
        let base = scratch("versions");
        let server = base.join("server");
        let source = write_synthetic_source(&base, &[("a.txt", "x")], "26.3");
        for bad in ["../escape", "a/b", "a\\b", "", ".", ".."] {
            assert!(
                accept_folder(&server, &source, bad).is_err(),
                "version {bad:?} should have been rejected"
            );
            assert!(
                load_accepted(&server, bad).is_err(),
                "version {bad:?} should have been rejected on load"
            );
        }
        assert!(!server.join(".horizon/prepared/escape").exists());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn accepts_a_signed_style_bundle_with_verified_entries() {
        let base = scratch("bundle");
        let server = base.join("server");
        let archive = base.join("prepared.zip");
        let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let manifest = format!(
            r#"{{"version":"26.3","files":[{{"path":"data.bin","sha256":"{hash}","size":3}}]}}"#
        );
        let mut writer = ZipWriter::new(fs::File::create(&archive).unwrap());
        writer
            .start_file(MANIFEST_FILE_NAME, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(manifest.as_bytes()).unwrap();
        writer
            .start_file("data.bin", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"abc").unwrap();
        writer.finish().unwrap();

        let accepted = accept_bundle(&server, &archive, "26.3").unwrap();
        assert_eq!(accepted.manifest.version, "26.3");
        assert!(accepted.root.join("data.bin").exists());
        assert!(load_accepted(&server, "26.3").unwrap().is_some());
        fs::remove_dir_all(&base).unwrap();
    }
}
