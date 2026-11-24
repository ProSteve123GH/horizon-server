#!/usr/bin/env python3
"""Audit a release directory for the one-binary-per-target rule.

Usage:
    python3 audit_artifact.py <release-directory> [--target <triple>]
                              [--allow-dir <name> ...]

The release directory must contain exactly one regular, executable file named
``horizon-server`` (or ``horizon-server.exe``). Everything else is either a
benign, non-executable metadata file (README, LICENSE, *.sha256, ...) or a
violation. Directories are rejected unless explicitly allowed with
``--allow-dir`` (no directories are allowed by default).

Rejected: extra executables, .jar/.zip archives, horizon-data.bin, shared
libraries (*.so, *.dll, *.dylib, including versioned .so.N), the sidecar names
horizon-engine / horizon-client / horizon-prepare / launcher (with or without
.exe), symlinks, special files, and unapproved directories.

A JSON report is printed to stdout with counts and the SHA-256 of the accepted
executable. Exit status: 0 = clean, 1 = violation, 2 = bad input/IO error.
``--target`` is report-only and never affects validation.
"""

import argparse
import hashlib
import json
import os
import re
import stat
import sys

ALLOWED_EXECUTABLES = ("horizon-server", "horizon-server.exe")

# Case-insensitive exact/prefix names that must never ship beside the server.
SIDECAR_BASES = frozenset(
    {"horizon-engine", "horizon-client", "horizon-prepare", "launcher"}
)
BANNED_SUFFIXES = (".jar", ".zip", ".so", ".dll", ".dylib")
BANNED_EXACT_NAMES = frozenset({"horizon-data.bin"})
VERSIONED_SO_RE = re.compile(r"\.so\.\d+(?:\.\d+)*$")

# No metadata directories are trusted out of the box; --allow-dir extends this.
DEFAULT_ALLOWED_METADATA_DIRS = frozenset()

_HASH_CHUNK = 1024 * 1024
_EXEC_BITS = stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH


def empty_counts():
    """All counters start at zero so reports have a stable shape."""
    return {
        "entries": 0,
        "files": 0,
        "directories": 0,
        "allowed_metadata_dirs": 0,
        "metadata_files": 0,
        "executables": 0,
        "allowed_name_files": 0,
        "allowed_executables": 0,
        "extra_executables": 0,
        "banned_files": 0,
        "sidecar_files": 0,
        "symlinks": 0,
        "special_files": 0,
        "unreadable": 0,
    }


def violation(kind, path, detail):
    return {"kind": kind, "path": path, "detail": detail}


def is_banned_name(name):
    low = name.lower()
    return (
        low in BANNED_EXACT_NAMES
        or low.endswith(BANNED_SUFFIXES)
        or bool(VERSIONED_SO_RE.search(low))
    )


def is_sidecar_name(name):
    low = name.lower()
    base = low[:-4] if low.endswith(".exe") else low
    for sidecar in SIDECAR_BASES:
        if base == sidecar or base.startswith((sidecar + "-", sidecar + "_", sidecar + ".")):
            return True
    return False


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(_HASH_CHUNK), b""):
            digest.update(chunk)
    return digest.hexdigest()


def audit_release_dir(release_dir, target, allowed_dirs):
    """Return (report, exit_code). Never raises for per-entry problems."""
    counts = empty_counts()
    violations = []
    executable = None

    def add(kind, path, detail):
        violations.append(violation(kind, path, detail))

    try:
        with os.scandir(release_dir) as entries:
            for entry in entries:
                counts["entries"] += 1
                name = entry.name
                try:
                    info = entry.stat(follow_symlinks=False)
                except OSError as exc:
                    counts["unreadable"] += 1
                    add("unreadable", entry.path, "cannot stat: %s" % exc)
                    continue
                mode = info.st_mode

                if entry.is_symlink():
                    counts["symlinks"] += 1
                    add("symlink", entry.path, "symlinks are not allowed in a release directory")
                    continue

                if stat.S_ISDIR(mode):
                    counts["directories"] += 1
                    if name in allowed_dirs:
                        counts["allowed_metadata_dirs"] += 1
                    else:
                        add("directory", entry.path, "unapproved directory (use --allow-dir to permit)")
                    continue

                if not stat.S_ISREG(mode):
                    counts["special_files"] += 1
                    add("special-file", entry.path, "only regular files are allowed")
                    continue

                counts["files"] += 1

                if is_banned_name(name):
                    counts["banned_files"] += 1
                    add("banned-file", entry.path, "forbidden game/archive/library payload")
                    continue

                if is_sidecar_name(name):
                    counts["sidecar_files"] += 1
                    add("sidecar", entry.path, "Horizon sidecar launcher/engine/helper is not allowed")
                    continue

                is_executable = bool(mode & _EXEC_BITS)
                if is_executable:
                    counts["executables"] += 1

                if name in ALLOWED_EXECUTABLES:
                    counts["allowed_name_files"] += 1
                    if not is_executable:
                        add("not-executable", entry.path, "expected a regular executable file")
                        continue
                    if executable is not None:
                        counts["extra_executables"] += 1
                        add("multiple-executables", entry.path, "more than one server executable")
                        continue
                    try:
                        digest = sha256_file(entry.path)
                    except OSError as exc:
                        counts["unreadable"] += 1
                        add("unreadable", entry.path, "cannot hash: %s" % exc)
                        continue
                    counts["allowed_executables"] += 1
                    executable = {
                        "name": name,
                        "path": entry.path,
                        "size": info.st_size,
                        "sha256": digest,
                    }
                    continue

                if is_executable:
                    counts["extra_executables"] += 1
                    add("extra-executable", entry.path, "unexpected executable file")
                    continue

                counts["metadata_files"] += 1
    except OSError as exc:
        add("io-error", release_dir, "cannot scan directory: %s" % exc)

    if executable is None:
        add(
            "missing-executable",
            release_dir,
            "expected exactly one executable named %s" % " or ".join(ALLOWED_EXECUTABLES),
        )

    ok = not violations
    report = {
        "target": target,
        "release_dir": release_dir,
        "ok": ok,
        "violations": violations,
        "counts": counts,
        "executable": executable,
        "allowed_metadata_dirs": sorted(allowed_dirs),
    }
    return report, (0 if ok else 1)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("release_dir", help="directory containing the release artifact")
    parser.add_argument("--target", default=None, help="target triple, report-only")
    parser.add_argument(
        "--allow-dir",
        action="append",
        default=[],
        metavar="NAME",
        help="permit an additional metadata directory (repeatable)",
    )
    args = parser.parse_args(argv)

    allowed_dirs = set(DEFAULT_ALLOWED_METADATA_DIRS)
    for item in args.allow_dir:
        allowed_dirs.add(item.strip("/\\"))

    if not os.path.isdir(args.release_dir):
        report = {
            "target": args.target,
            "release_dir": args.release_dir,
            "ok": False,
            "violations": [
                violation("input-error", args.release_dir, "not a readable directory")
            ],
            "counts": empty_counts(),
            "executable": None,
            "allowed_metadata_dirs": sorted(allowed_dirs),
        }
        print(json.dumps(report, indent=2, sort_keys=True))
        return 2

    report, code = audit_release_dir(args.release_dir, args.target, allowed_dirs)
    print(json.dumps(report, indent=2, sort_keys=True))
    return code


if __name__ == "__main__":
    sys.exit(main())
