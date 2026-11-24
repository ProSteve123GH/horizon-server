#!/usr/bin/env bash
# CI-only generator for the Minecraft 26.3 prepared-data bundle.
#
# This script is the owner's PC substitute: it runs where Java at BUILD time is
# allowed (GitHub Actions), downloads only Mojang's official, hash-pinned inputs,
# and runs the vendored upstream `horizon-prepare` in a clean scratch copy under
# $RUNNER_TEMP. Nothing is written into this repository, and nothing Mojang
# publishes (jars, classes, assets) is shipped in the final archive.
#
# The shipped `horizon-server` never needs Java: it verifies and installs this
# archive with src/prepared.rs (`horizon-server prepare --prepared-bundle ...`).
#
# Output: a single deterministic ZIP containing `prepared-manifest.json`
# ({version, files:[{path, sha256, size}]}) and exactly those listed files.
# ZIP metadata (timestamps, order, compression) is fixed; the manifest records
# the actual prepared bytes, because upstream's Java exporters are not required
# to be byte-identical across independent runs.
set -euo pipefail

log() { printf '[prepare-data] %s\n' "$*"; }
die() { printf '::error::[prepare-data] %s\n' "$*" >&2; exit 1; }

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
VENDOR="$REPO_ROOT/vendor/horizon"
VERSION="${VERSION:-26.3}"
RUST_TOOLCHAIN="${RUST_TOOLCHAIN:-1.95.0}"
BUNDLE_NAME="horizon-prepared-$VERSION.zip"

[ -f "$VENDOR/research-lock.json" ] || die "missing $VENDOR/research-lock.json"
lock_version="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["minecraft"]["version"])' "$VENDOR/research-lock.json")"
[ "$lock_version" = "$VERSION" ] || die "research-lock.json pins $lock_version, expected $VERSION"

# Always use a scratch root outside the checkout. A locally invoked script gets
# its own private temp root and removes it on exit.
CREATED_TEMP=""
if [ -z "${RUNNER_TEMP:-}" ]; then
  RUNNER_TEMP="$(mktemp -d)"
  CREATED_TEMP="$RUNNER_TEMP"
fi
mkdir -p "$RUNNER_TEMP"
WORK="$RUNNER_TEMP/horizon-prepared-work"
OUT_DIR="${OUT_DIR:-$RUNNER_TEMP/horizon-prepared-out}"
mkdir -p "$OUT_DIR"
rm -rf "$WORK"
mkdir -p "$WORK"
cleanup() {
  rm -rf "$WORK"
  [ -z "$CREATED_TEMP" ] || rm -rf "$CREATED_TEMP"
}
trap cleanup EXIT
log "scratch checkout: $WORK"

# A pristine copy of the pinned upstream snapshot; the repo itself is never built in.
cp -a "$VENDOR/." "$WORK/"
rm -rf "$WORK/target" "$WORK/.git"

command -v python3 >/dev/null || die "python3 is required"
command -v cargo >/dev/null || die "cargo is required (install pinned Rust $RUST_TOOLCHAIN first)"
rustc_version="$(rustc --version)"
case "$rustc_version" in
  "rustc $RUST_TOOLCHAIN"*) log "$rustc_version" ;;
  *) die "expected rustc $RUST_TOOLCHAIN, found: $rustc_version" ;;
esac
export CARGO_TARGET_DIR="$WORK/target"
export CARGO_INCREMENTAL=0

# Upstream's rule: every run of a preparer/game gets fresh, isolated homes.
ISOLATED="$WORK/isolated"
mkdir -p "$ISOLATED/home" "$ISOLATED/launcher_home" "$ISOLATED/xdg_data" "$ISOLATED/xdg_config"
export HORIZON_HOME="$ISOLATED/home"
export HORIZON_LAUNCHER_HOME="$ISOLATED/launcher_home"
export XDG_DATA_HOME="$ISOLATED/xdg_data"
export XDG_CONFIG_HOME="$ISOLATED/xdg_config"
export HORIZON_LAUNCHER_NO_SELF_UPDATE=1
export HORIZON_NO_AUDIO=1

INPUTS="$WORK/build-inputs"
mkdir -p "$INPUTS"

# Adapted from the Android fork's tools/android_prepare.py (download_game and
# download_jdk), using the vendored upstream tools/lab.py for verified fetches.
# It writes only into the scratch inputs and prints the chosen JDK home to a file.
cat > "$WORK/tools/ci_fetch_inputs.py" <<'PY'
#!/usr/bin/env python3
"""Fetch hash-pinned Mojang inputs and a Java 25 JDK for upstream horizon-prepare.

Adapted from the Android fork's tools/android_prepare.py; the download helpers
come from this checkout's tools/lab.py, so every file is verified before use.
"""
import argparse
import json
import os
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import re
import subprocess
import sys
import time
import urllib.request

import lab

RUNTIMES = ("https://launchermeta.mojang.com/v1/products/java-runtime/"
            "2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json")


def say(message):
    print(f"[mojang-inputs] {message}", file=sys.stderr, flush=True)


def fetch_json(url):
    with urllib.request.urlopen(url, timeout=120) as response:
        return json.load(response)


def relative(base, value):
    """Resolve value under base, refusing a path that escapes it."""
    path = (base / value).resolve()
    if not path.is_relative_to(base.resolve()):
        raise ValueError(f"Path escapes download directory: {value}")
    return path


def parallel_download(files):
    def one(item):
        url, path, checksum = item
        for attempt in range(4):
            try:
                lab.download(url, path, checksum)
                return
            except (OSError, ValueError):
                if attempt == 3:
                    raise
                time.sleep(1 + attempt)

    with ThreadPoolExecutor(max_workers=8) as pool:
        for _ in pool.map(one, files):
            pass


def download_game(version, directory, pins):
    """Mojang's own pinned version JSON, client/server jars, libraries and assets."""
    versions = directory / "versions" / version
    manifest_path = versions / f"{version}.json"
    manifest_url = (f"https://piston-meta.mojang.com/v1/packages/"
                    f"{pins['manifest_sha1']}/{version}.json")
    lab.download(manifest_url, manifest_path, pins["manifest_sha1"])
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("id") != version:
        raise ValueError(f"version JSON declares {manifest.get('id')!r}, not {version!r}")

    files = []
    for side in ("client", "server"):
        artifact = manifest["downloads"][side]
        if artifact["sha1"] != pins[f"{side}_sha1"]:
            raise ValueError(f"{side} jar differs from the engine pin")
        name = f"{version}-server.jar" if side == "server" else f"{version}.jar"
        files.append((artifact["url"], versions / name, artifact["sha1"]))
    for artifact in lab.client_library_artifacts(manifest):
        files.append((artifact["url"], relative(directory / "libraries", artifact["path"]),
                      artifact["sha1"]))

    index = manifest["assetIndex"]
    accepted = pins.get("accepted_asset_index_sha1", [])
    if accepted and index["sha1"] not in accepted:
        raise ValueError(f"asset index {index['sha1']} is not in the engine's accepted list")
    index_path = relative(directory / "assets/indexes", f"{index['id']}.json")
    lab.download(index["url"], index_path, index["sha1"])
    objects = json.loads(index_path.read_text(encoding="utf-8"))["objects"]
    for value in objects.values():
        checksum = value["hash"]
        name = f"{checksum[:2]}/{checksum}"
        files.append((f"https://resources.download.minecraft.net/{name}",
                      directory / "assets/objects" / name, checksum))

    say(f"downloading Minecraft {version}: {len(files)} verified files")
    parallel_download(files)
    return manifest, versions / f"{version}-server.jar"


def download_jdk(manifest, directory):
    """Mojang's own Java runtime for this version (the Android fork's flow)."""
    component = manifest["javaVersion"]["component"]
    entry = fetch_json(RUNTIMES)["linux"][component][0]["manifest"]
    metadata = directory / "runtime.json"
    lab.download(entry["url"], metadata, entry["sha1"])
    files = json.loads(metadata.read_text(encoding="utf-8"))["files"]
    home = directory / component
    downloads = []
    for name, info in files.items():
        path = relative(home, name)
        if info["type"] == "file":
            artifact = info["downloads"]["raw"]
            downloads.append((artifact["url"], path, artifact["sha1"]))
    say(f"downloading Mojang JDK: {component} ({len(downloads)} files)")
    parallel_download(downloads)
    for name, info in files.items():
        path = relative(home, name)
        if info["type"] == "file" and info.get("executable"):
            path.chmod(path.stat().st_mode | 0o111)
        elif info["type"] == "link" and not path.exists():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.symlink_to(info["target"])
    return home


def tool_major(home, tool):
    """The Java major a home's tool reports, or None when it is absent/broken."""
    executable = home / "bin" / tool
    if not executable.is_file() or not os.access(executable, os.X_OK):
        return None
    try:
        output = subprocess.run([str(executable), "-version"], stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, text=True, timeout=60,
                                check=False).stdout
    except (OSError, subprocess.SubprocessError):
        return None
    match = re.search(r"\b(\d+)(?:\.\d+)*\b", output.splitlines()[0] if output else "")
    return int(match.group(1)) if match else None


def choose_java_home(downloaded, pinned_major):
    candidates = []
    if downloaded is not None:
        candidates.append(("Mojang runtime", downloaded))
    fallback = os.environ.get("JAVA_HOME")
    if fallback:
        candidates.append(("JAVA_HOME fallback", Path(fallback)))
    for label, home in candidates:
        if tool_major(home, "javac") == pinned_major and tool_major(home, "java") == pinned_major:
            say(f"using Java {pinned_major} from {label}: {home}")
            return home.resolve()
    raise SystemExit(
        f"no JDK {pinned_major} with javac: checked "
        + ", ".join(f"{label} ({home})" for label, home in candidates)
        + ". Install the pinned JDK with actions/setup-java (Temurin)."
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--inputs", type=Path, required=True)
    parser.add_argument("--java-home-out", type=Path, required=True)
    args = parser.parse_args()

    pins = json.loads((args.root / "research-lock.json").read_text(encoding="utf-8"))["minecraft"]
    version = pins["version"]
    if version != "26.3":
        raise SystemExit(f"this bundle generator is pinned to 26.3, not {version}")
    manifest, _server = download_game(version, args.inputs / "minecraft", pins)

    downloaded = None
    try:
        downloaded = download_jdk(manifest, args.inputs / "jdk")
    except Exception as error:  # fall back to the JDK the workflow installed
        say(f"Mojang JDK download failed ({error!r}); using JAVA_HOME if it is Java {pins['java_major']}")
    home = choose_java_home(downloaded, pins["java_major"])
    args.java_home_out.write_text(str(home) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
PY

log "fetching Mojang-pinned inputs for $VERSION"
python3 "$WORK/tools/ci_fetch_inputs.py" \
  --root "$WORK" --inputs "$INPUTS" \
  --java-home-out "$RUNNER_TEMP/horizon-java-home.txt"

JAVA_HOME="$(cat "$RUNNER_TEMP/horizon-java-home.txt")"
[ -x "$JAVA_HOME/bin/javac" ] && [ -x "$JAVA_HOME/bin/java" ] || die "chosen JAVA_HOME has no javac/java: $JAVA_HOME"
export JAVA_HOME
log "JAVA_HOME=$JAVA_HOME"

log "running vendored upstream horizon-prepare --prepare"
(
  cd "$WORK"
  cargo run --locked --release -p horizon-prepare -- \
    --root "$WORK" --prepare \
    --minecraft-dir "$INPUTS/minecraft" \
    --server-jar "$INPUTS/minecraft/versions/$VERSION/$VERSION-server.jar" \
    --java-home "$JAVA_HOME"
)

PREPARED="$WORK/.local/horizon-client/$VERSION"
[ -f "$PREPARED/horizon-resources.json" ] || die "preparer did not create $PREPARED/horizon-resources.json"

# Build the one deterministic archive and audit it in the same pass.
ARCHIVE="$OUT_DIR/$BUNDLE_NAME"
python3 - "$PREPARED" "$ARCHIVE" "$VERSION" <<'PY'
import hashlib
import json
import shutil
import sys
import zipfile
from pathlib import Path

source, archive, version = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
MANIFEST = "prepared-manifest.json"
FIXED_TIME = (1980, 1, 1, 0, 0, 0)
BANNED_SUFFIXES = (".jar", ".class", ".zip")


def fail(message):
    raise SystemExit(f"[prepare-data] archive audit failed: {message}")


if (source / MANIFEST).exists():
    fail(f"prepared folder already contains {MANIFEST}")

files = []
for path in sorted(source.rglob("*")):
    if path.is_symlink():
        fail(f"symlink in prepared folder: {path.relative_to(source).as_posix()}")
    if not path.is_file():
        continue
    relative = path.relative_to(source).as_posix()
    parts = relative.split("/")
    if relative.startswith("/") or "\\" in relative or "//" in relative or any(
        part in ("", ".", "..") for part in parts
    ):
        fail(f"unsafe prepared path: {relative!r}")
    low = relative.lower()
    if low.endswith(BANNED_SUFFIXES) or low.startswith(".local/") or "/.local/" in low:
        fail(f"forbidden payload in prepared folder: {relative}")
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    files.append({"path": relative, "sha256": digest.hexdigest(), "size": path.stat().st_size})

if not files:
    fail("prepared folder is empty")

manifest_bytes = (json.dumps({"version": version, "files": files},
                             indent=2, sort_keys=True) + "\n").encode("utf-8")
archive.parent.mkdir(parents=True, exist_ok=True)
partial = archive.with_suffix(archive.suffix + ".partial")
with zipfile.ZipFile(partial, "w", compression=zipfile.ZIP_DEFLATED,
                     compresslevel=6, strict_timestamps=True) as bundle:
    for name, payload in ((MANIFEST, manifest_bytes),):
        info = zipfile.ZipInfo(name, date_time=FIXED_TIME)
        info.compress_type = zipfile.ZIP_DEFLATED
        info.create_system = 3
        info.external_attr = 0o100644 << 16
        with bundle.open(info, "w") as output:
            output.write(payload)
    for entry in files:
        info = zipfile.ZipInfo(entry["path"], date_time=FIXED_TIME)
        info.compress_type = zipfile.ZIP_DEFLATED
        info.create_system = 3
        info.external_attr = 0o100644 << 16
        with (source / entry["path"]).open("rb") as stream, bundle.open(info, "w") as output:
            shutil.copyfileobj(stream, output, 1024 * 1024)
partial.replace(archive)

# Independent re-read of the shipped archive: names, mode, size and digest.
expected = {MANIFEST} | {entry["path"] for entry in files}
with zipfile.ZipFile(archive) as bundle:
    names = bundle.namelist()
    if len(names) != len(expected) or set(names) != expected:
        fail("archive entries do not match the manifest file list")
    for info in bundle.infolist():
        if info.is_dir() or ((info.external_attr >> 16) & 0o170000) == 0o120000:
            fail(f"directory or symlink entry: {info.filename}")
        low = info.filename.lower()
        if low.endswith(BANNED_SUFFIXES) or low.startswith(".local/") or "/.local/" in low:
            fail(f"forbidden archive entry: {info.filename}")
    reread = json.loads(bundle.read(MANIFEST))
    if reread != {"version": version, "files": files}:
        fail("manifest changed while writing the archive")
    for entry in files:
        digest = hashlib.sha256()
        with bundle.open(entry["path"]) as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
        if digest.hexdigest() != entry["sha256"]:
            fail(f"digest mismatch after packaging: {entry['path']}")

print(f"[prepare-data] packaged {len(files)} files from {source}", flush=True)
PY

archive_sha="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
archive_size="$(stat -c%s "$ARCHIVE")"
log "archive: $ARCHIVE"
log "SHA-256: $archive_sha"
log "size: $archive_size bytes"
