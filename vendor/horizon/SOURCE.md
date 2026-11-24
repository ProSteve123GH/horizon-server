# Source snapshot provenance — `vendor/horizon/`

This directory is a **source-only reference snapshot** of canonical upstream
[`horizon-rust/horizon`](https://github.com/horizon-rust/horizon) at the pinned commit recorded
below. It is kept in this standalone `horizon-server` product repository purely as an
architecture/protocol/parity reference. Nothing here was built, tested, run or modified by the
snapshot process, and no game content is bundled.

## 1. Exact provenance

| Field | Value |
| --- | --- |
| Repository URL | `https://github.com/horizon-rust/horizon` |
| Git transport used | HTTPS (`https://github.com/horizon-rust/horizon.git`) |
| Pinned commit | `a02b2838d34f4c99726cca349ec0b4d62c5c8fa5` |
| Commit subject | `tooling(parity): add 8 movement scenarios and phased teleport comparison to wire_stream (#19275)` |
| Commit author | Nikita Maximilian Grimm `<github@nikitagrimm.de>` |
| Commit date | `2026-11-21T00:35:49Z` (`Sat Nov 21 01:35:49 2026 +0100`) |
| Default branch at retrieval | `main`; remote `HEAD` was `2743ed5b7eeb9f062637224634178525d3bbb5ed` |
| License | LGPL-3.0 — preserved verbatim in `LICENSE` and `THIRD_PARTY_NOTICES.md` |

The local reference checkout `/root/horizon-upstream` named in the repository instructions no longer
exists, so the snapshot was fetched fresh from GitHub.

## 2. Retrieval commands (exact)

The canonical codeload tarball endpoint was attempted first and failed with `curl: (28) SSL
connection timeout` after repeated retries:

```sh
curl -fL --retry 3 --connect-timeout 20 \
  -o /root/scratch/horizon-upstream-a02b2838.tar.gz \
  "https://codeload.github.com/horizon-rust/horizon/tar.gz/a02b2838d34f4c99726cca349ec0b4d62c5c8fa5"
```

Retrieval therefore used the GitHub HTTP git transport with a blobless partial clone and sparse
checkout (only ~150 MiB of working tree instead of the full ~871 MiB repository):

```sh
mkdir -p /root/scratch/upstream
cd /root/scratch/upstream
git init -q
git remote add origin https://github.com/horizon-rust/horizon.git
git fetch --depth 1 --filter=blob:none origin \
  a02b2838d34f4c99726cca349ec0b4d62c5c8fa5
git rev-parse FETCH_HEAD   # -> a02b2838d34f4c99726cca349ec0b4d62c5c8fa5

git sparse-checkout init --no-cone
cat > .git/info/sparse-checkout <<'EOF'
/.cargo/**
/crates/**
/third_party/**
/tools/**
/docs/layers.md
/docs/server.md
/docs/hosting.md
/docs/api/server.md
/docs/api/**
EOF
git checkout FETCH_HEAD

# Keep only the four requested docs files (the initial broad /docs/api/** pattern
# was narrowed after checkout).
find docs -type f \
  ! -path docs/layers.md ! -path docs/server.md \
  ! -path docs/hosting.md ! -path docs/api/server.md -delete
find docs -type d -empty -delete

# Root files requested by the snapshot specification.
for f in LICENSE THIRD_PARTY_NOTICES.md Cargo.toml Cargo.lock research-lock.json; do
  git show "FETCH_HEAD:$f" > "$f"
done
```

Copy into this repository, followed by binary-artifact pruning (all files containing NUL bytes
were removed except the UTF-16 XML source file
`third_party/libmimalloc-sys/c_src/mimalloc/v3/src/prim/windows/etw.man`, which is text):

```sh
cd /root/scratch/upstream
cp -a .cargo crates third_party tools docs /root/horizon-server/vendor/horizon/
cp -a LICENSE THIRD_PARTY_NOTICES.md Cargo.toml Cargo.lock research-lock.json \
  /root/horizon-server/vendor/horizon/
```

No `cargo`, `rustc`, build, test or clippy command was run. No commit was made; the files are left
for the integration lead to review.

## 3. Included paths

- `LICENSE`, `THIRD_PARTY_NOTICES.md` (upstream attribution and licence text)
- `Cargo.toml`, `Cargo.lock`
- `.cargo/config.toml`
- `research-lock.json`
- all of `crates/`
- all of `third_party/`
- all of `tools/` (source only)
- `docs/layers.md`, `docs/server.md`, `docs/hosting.md`, `docs/api/server.md`
- this `SOURCE.md`

## 4. Excluded paths and artifacts

The following were never copied or were removed from the snapshot:

- `.git/`, `.local/`, `target/`, `.claude/`, CI workflow metadata and other repository meta
  outside the include list
- `*.jar`, `*.zip` (including `tools/ci/vendor/pyyaml-6.0.3.zip`), `*.bin`, `*.class`, `*.so`,
  `*.dll`, `*.exe`
- Mojang / game content, generated resource folders, prepared data (`horizon-data.bin`),
  `engine-data.zip`, real player or world data
- all binary assets/fixtures: `.png`, `.ttf`, `.nbt`, `.hzperf`, `.tar.xz`, `.hmod`, `.hpack`,
  `.hshader` — 93 files removed in total. Binary test fixtures are intentionally absent; this is
  a source-only reference, not a buildable test tree.

`third_party/libmimalloc-sys/c_src/mimalloc/v3/src/prim/windows/etw.man` is retained: it is a
UTF-16 encoded XML source file, not a binary artifact.

## 5. Version pins recorded by this snapshot

- **Minecraft version: `26.3`** — `research-lock.json` (`minecraft.version`) and
  `crates/horizon-core/src/version.rs` (`VERSION_ID`, `VANILLA_VERSION`).
- **Protocol version: `777`** — `crates/horizon-core/src/version.rs:9`
  (`pub const PROTOCOL_VERSION: i32 = 777;`).
- **Data pack format: `121`** — `crates/horizon-core/src/version.rs:15`
  (`pub const DATA_PACK_FORMAT: (i64, i64) = (121, 0);`).
- Related pins from the same source: `DATA_VERSION = 5023`, `RESOURCE_PACK_FORMAT = (97, 1)`,
  `JAVA_VERSION = 25`, `STABLE = true`. `research-lock.json` additionally pins the 26.3 manifest
  SHA-1, accepted manifest/asset-index SHA-1 lists, client/server JAR SHA-1 values and Java 25
  runtime manifests (metadata only — no JARs or runtimes are included).

## 6. No Mojang content

**No Mojang content is included in this snapshot.** There are no game or server JARs, no assets,
no decompiled source, no generated resource folders, no `.local/` directory, no prepared data
(`horizon-data.bin`), no `engine-data.zip`, and no real player or world data. `research-lock.json`
contains only official endpoint metadata and checksums; the `horizon-server` product is required
to download and verify its inputs from official endpoints at runtime.

## 7. Attribution

All copied source remains the work of the upstream `horizon-rust/horizon` contributors and is
licensed under LGPL-3.0. Upstream `LICENSE`, `THIRD_PARTY_NOTICES.md`, in-file copyright headers
and `LICENSES/`-style attribution were preserved where present in the included paths. This file
is the only file added by the snapshot process.
