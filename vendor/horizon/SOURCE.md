# Source snapshot provenance — `vendor/horizon/`

This directory is a **source-only reference snapshot** of canonical upstream
[`horizon-rust/horizon`](https://github.com/horizon-rust/horizon) at the pinned commit recorded
below. It is kept in this standalone `horizon-server` product repository purely as an
architecture/protocol/parity reference. Nothing here was built, tested, run or modified by the
snapshot process, and no game content is bundled.

> **Completeness status (updated):** this tree is now a **complete mirror of every git-tracked
> file** in the pinned upstream commit — all **11,691** tracked paths are present at their exact
> upstream-relative paths. The `cargo metadata --no-deps` workspace resolves successfully (77
> workspace members). It still contains **no Mojang content**. See **section 9** for the exact
> restore commands and verification results. Sections 2–4 below record the *initial* subset
> snapshot and are superseded where they describe included/excluded paths.

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

*Initial snapshot only — superseded by section 9, which made the tree complete.*

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

*Initial snapshot only — superseded by section 9. The only artifacts still excluded are those
listed below that are not part of upstream's tracked tree (`.git/`, `.local/`, `target/`, `*.jar`,
`horizon-data.bin`, `engine-data.zip`); all other upstream-tracked paths are now present.*

The following were never copied or were removed from the snapshot:

- `.git/`, `.local/`, `target/`, `.claude/`, CI workflow metadata and other repository meta
  outside the include list
- `*.jar`, `*.bin`, `*.class`, `*.so`, `*.dll`, `*.exe`
- Mojang / game content, generated resource folders, prepared data (`horizon-data.bin`),
  `engine-data.zip`, real player or world data
- all binary assets/fixtures were initially removed (`.png`, `.ttf`, `.nbt`, `.hzperf`, `.tar.xz`,
  `.hmod`, `.hpack`, `.hshader`, `.zip`, `.jpg`, `.webp`, `.ico`, `.icns`). The upstream-source-
  controlled subset was subsequently restored for buildability — see section 8.

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
and `LICENSES/`-style attribution were preserved where present in the included paths. `SOURCE.md`
is the only file added by the snapshot process.

## 8. Restored git-tracked binary fixtures (buildability)

The initial snapshot pruned every git-tracked binary file, which left it a reference tree rather
than a build tree: vendored Rust sources pull in fonts, sprites, SDK archives and test fixtures
with `include_bytes!`. To make the snapshot buildable, **212 git-tracked binary files** were
restored byte-for-byte from the pinned commit's git object store (blobless partial clone at
`/root/scratch/upstream`, `FETCH_HEAD` = `a02b2838d34f4c99726cca349ec0b4d62c5c8fa5`) into their
exact upstream-relative paths. Every restored blob was verified against the upstream tree OID;
all 212 matched. Categories restored:

- fonts: 14 `.ttf` files under `crates/horizon-theme/fonts/`
- UI/controller/icon sprites and documentation media: 158 `.png`, 16 `.webp`, 7 `.jpg`,
  `.ico`/`.icns` under `crates/`, `docs/media/`, `launcher/` and `packaging/`
- test data: 5 `.nbt`, 2 `.hzperf`, `.hmod`/`.hpack`/`.hshader` under `crates/` and `tests/`
- SDK/tooling archives: 4 `.tar.xz` under `crates/horizon-mod-package/src/sdk/` and
  `tools/ci/vendor/pyyaml-6.0.3.zip`

**No Mojang content is included.** No `*.jar`, `horizon-data.bin`, `engine-data.zip`, `.local/`,
`target/`, `.git/` or downloaded game resource was restored — none of these exist in the upstream
commit's tracked tree. All restored files are upstream's own source-controlled fixtures (licensed
under the project's LGPL-3.0 / third-party notices). The two `include_str!` text fixtures
(`docs/modpacks/atm10-mods.csv` and `docs/parity/random-chance-seeded.jsonl`) previously noted as
out of scope were subsequently restored with the full tracked tree in section 9.

## 9. Full tracked-tree restore (completeness)

The earlier snapshot copied only selected directories, so upstream's root `Cargo.toml` listed
workspace members (e.g. `docs/modding/examples`, `benchmarks`, `launcher`, `packaging`, `tests`,
`xtask`, `mods`) whose manifests were missing. `cargo metadata --no-deps --manifest-path
vendor/horizon/Cargo.toml` therefore failed with `failed to load manifest for workspace member
.../docs/modding/examples`. The tree was completed by restoring **every git-tracked file** from
the pinned commit.

Source of truth: the existing blobless partial clone at `/root/scratch/upstream`, verified as
`HEAD` = `FETCH_HEAD` = `a02b2838d34f4c99726cca349ec0b4d62c5c8fa5`.

```sh
git -C /root/scratch/upstream rev-parse HEAD FETCH_HEAD   # both a02b2838...
git -C /root/scratch/upstream ls-tree -r --name-only FETCH_HEAD > /tmp/upstream_tracked.txt
# 11,691 tracked paths
```

Every tracked path was checked against `vendor/horizon/`; 1,713 were absent. All 1,713 missing
blobs were already present locally in the partial clone's object store (`GIT_NO_LAZY_FETCH=1
git cat-file --batch-check` reported 0 missing), so no network fetch was needed. Missing files
were materialised byte-for-byte from `FETCH_HEAD:<path>` with a single `git cat-file --batch`
process, using `mkdir -p` for parent directories and refusing to overwrite any path that already
existed. The 1,713 restored files were all upstream text/source files (`.rs`, `.java`, `.py`,
`.md`, `.json`, `.yml`, `.toml`, `.svg`, `.nix`, shell scripts, etc.) — no binary blob was
missing. Categories: `launcher/` (420), `mods/` (417), `docs/` (396), `tests/` (344),
`benchmarks/` (42), `.github/` (35), `packaging/` (24), `nix/` (8), `xtask/` (5), `roadmap/` (4),
`LICENSES/` (2), `.githooks/` (2), and root/meta files (`AGENTS.md`, `CLAUDE.md`,
`CONTRIBUTING.md`, `GOVERNANCE.md`, `README.md`, `ROADMAP.md`, `clippy.toml`, `flake.lock`,
`flake.nix`, `.gitignore`, `.gitattributes`, `.dockerignore`, `.coderabbit.yaml`, `.claude/`).

Verification results (exact commands and outputs):

| Check | Result |
| --- | --- |
| `git -C /root/scratch/upstream ls-tree -r --name-only FETCH_HEAD \| wc -l` | 11,691 |
| Upstream tracked paths missing on disk after restore | 0 |
| `find vendor/horizon -type f \| wc -l` | 11,692 (11,691 tracked + this `SOURCE.md`) |
| `du -sh vendor/horizon` | 203M |
| `find vendor/horizon \( -name '*.jar' -o -name 'horizon-data.bin' -o -name 'engine-data.zip' \) -print` | empty |
| `.local/` / `target/` directories under `vendor/horizon` | none |
| `cargo metadata --no-deps --format-version 1 --manifest-path vendor/horizon/Cargo.toml` | exit 0 — 77 workspace members, root `/root/horizon-server/vendor/horizon` |

The 1,713 newly restored files join the 212 binaries restored in section 8, giving a complete,
parseable, build-oriented reference tree. **No Mojang content is included:** no game/server JARs,
assets, decompiled source, generated resource folders, `.local/`, prepared data
(`horizon-data.bin`), `engine-data.zip`, or real player/world data — none of these exist in the
upstream commit's tracked tree, and the exclusion check above is empty. All content remains
upstream's LGPL-3.0 source-controlled material. No build/test/clippy was run; no commit was made.

## 10. Downstream patch for the Android ARM64 CLI (horizon-server)

`horizon-server` re-execs itself with a private flag and enters the dedicated game loop through
`horizon_client::entry::prepare()`/`start()`. On `aarch64-linux-android`, upstream's
`horizon-mod-package` compiled `runtime_leases.rs`, which references `crate::holder`, while
`holder`/`local_control` were gated to `linux`, `macos` and `windows` only. That produced E0433 and
E0277 (`could not compile horizon-mod-package`) in CI run 45324523961.

Minimal downstream patch applied here: add `target_os = "android"` to the two `cfg(all(...))` blocks
in `crates/horizon-mod-package/src/lib.rs` that gate `holder` and `local_control`. No mod runtime is
added — `horizon-mod-host`/wasmtime are still absent from the dependency graph. The patch is
documented inline beside each change. Upstream's original file is otherwise unchanged.
