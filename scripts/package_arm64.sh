#!/usr/bin/env bash
# Package one Android ARM64 (aarch64-linux-android) server binary for handoff.
#
# Usage:
#   scripts/package_arm64.sh <binary-path> <output-directory>
#
# Verifies the input with `llvm-readelf -h` (ELF64 + AArch64), stages a single
# executable named `horizon-server`, audits that stage with audit_artifact.py,
# writes <name>.sha256, and emits a .tar.gz containing ONLY the binary and its
# checksum. No Mojang files, sidecars, libraries or extra executables are staged.
# $TMPDIR is honoured; /tmp/opencode is preferred when $TMPDIR is unset.

set -euo pipefail

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

if [[ $# -ne 2 ]]; then
  printf 'usage: %s <binary-path> <output-directory>\n' "$0" >&2
  exit 2
fi

binary=$1
out_dir=$2
target=aarch64-linux-android
name=horizon-server

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
audit_script="$script_dir/audit_artifact.py"

[[ -f "$binary" ]] || die "binary not found: $binary"
[[ -r "$binary" ]] || die "binary is not readable: $binary"
[[ -f "$audit_script" ]] || die "audit script not found: $audit_script"
command -v llvm-readelf >/dev/null 2>&1 || die "llvm-readelf is required but not on PATH"
command -v sha256sum >/dev/null 2>&1 || die "sha256sum is required but not on PATH"
command -v tar >/dev/null 2>&1 || die "tar is required but not on PATH"

# Confirm we are packaging a native ARM64 ELF, not a script or wrong-arch build.
header=$(llvm-readelf -h "$binary" 2>&1) || die "llvm-readelf -h failed for $binary"
printf '%s\n' "$header" | grep -Eq 'Class:[[:space:]]*ELF64' \
  || die "$binary is not an ELF64 binary"
printf '%s\n' "$header" | grep -Eiq 'Machine:[[:space:]]*(AArch64|ARM64)([^[:alnum:]]|$)' \
  || die "$binary is not an AArch64/ARM64 binary"

# Prefer /tmp/opencode when the caller did not override the temporary location.
if [[ -z "${TMPDIR:-}" ]]; then
  if [[ -d /tmp/opencode && -w /tmp/opencode ]]; then
    TMPDIR=/tmp/opencode
  else
    TMPDIR=/tmp
  fi
fi
export TMPDIR

work=$(mktemp -d "${TMPDIR%/}/horizon-package-arm64.XXXXXX") \
  || die "cannot create a temporary directory under $TMPDIR"
trap 'rm -rf "$work"' EXIT

stage="$work/stage"
mkdir -p "$stage"

# Stage under the canonical release name; chmod only the copy, never the input.
install -m 0755 "$binary" "$stage/$name"

# Audit must pass before anything is published (prints its JSON report).
python3 "$audit_script" "$stage" --target "$target"

( cd "$stage" && sha256sum "$name" > "$name.sha256" )

mkdir -p "$out_dir"
out_dir=$(cd "$out_dir" && pwd)
checksum="$out_dir/$name.sha256"
archive="$out_dir/$name-$target.tar.gz"

cp "$stage/$name.sha256" "$checksum"
tar -czf "$archive" -C "$stage" "$name" "$name.sha256"

# Belt-and-braces: the archive must hold exactly those two entries.
mapfile -t archive_entries < <(tar -tzf "$archive")
[[ ${#archive_entries[@]} -eq 2 ]] || die "unexpected tar entries in $archive"
[[ "${archive_entries[0]}" == "$name" && "${archive_entries[1]}" == "$name.sha256" ]] \
  || die "unexpected tar contents in $archive"

printf 'packaged %s\n  archive:  %s\n  checksum: %s\n' "$name" "$archive" "$checksum"
