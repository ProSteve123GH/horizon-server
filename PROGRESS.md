# Progress, evidence and blockers

Integration lead's running checklist for the standalone Horizon Dedicated Server. This is a living
document: each claim below is backed by a command output, file path or test result in this repository
or the session transcript. **Do not mark an item done on a successful compile alone** — `TASK.md` §8
requires runtime and parity evidence.

## Current phase

- [x] Repository initialized (`server` branch, remote, tracked docs, ignored `DEV.md`, clean commit).
- [x] Phase 0 — environment/toolchain/device feasibility and source mapping complete; owner decisions
      recorded below.
- [~] Phase 1 — Android ARM64 (`aarch64-linux-android`) native Termux CLI. CLI/config/bootstrap/
      datapack/self-spawn scaffolding implemented and unit-tested; engine integration and tablet
      validation are still pending.
- [ ] Phase 2 — remaining Android ABIs.
- [ ] Phase 3 — Linux targets.
- [ ] Phase 4 — Windows targets.
- [ ] Phase 5 — macOS targets.

Implemented so far: CLI/config/EULA/bootstrap/datapack discovery/player files/server-dir/self-spawn
scaffolding with host tests. No integrated game engine, no runtime server behavior and no tablet
validation yet. See `DECISIONS_NEEDED.md` for the owner-approved interim approach.

## Owner decisions (2026-11-24)

The owner selected the **pragmatic interim** path and confirmed tablet validation is theirs to run:

1. Headless boundary: graphics crates may be linked but no renderer/window may be initialized in
   headless mode; this remains a **documented deviation** from `TASK.md §2.1` until a full extraction
   is done.
2. Prepared 26.3 tables: an operator-supplied or signed verified prepared-data folder is accepted
   instead of pure-Rust generation; this remains a **documented deviation** from `TASK.md §2.3`.
3. Tablet acceptance: owner runs `TABLET_TEST_PLAN.md`; M1.3–M1.8 stay unchecked until real logs are
   returned. No criterion is deleted or weakened.

## Environment facts (observed 2026-11-24)

| Item | Evidence | Status |
|---|---|---|
| Local repo | `git status -sb` → `## server...origin/server`; `git log -1` → `8d8b1c9` | OK |
| GitHub remote/auth | `gh api user --jq .login` → `ProSteve123GH`; remote default branch `server` | OK |
| Host resources | Owner's GuppyNodes panel: **3 vCPU, 2.5 GiB RAM (~712 MiB baseline used), 10 GiB disk (~4.9 GiB used)**; free tier, do not max it out. `df -h` reports the host overlay and is misleading. | CONSTRAINED — no heavy local builds |
| Rust toolchain | Installed locally (1.95.0) for light verification; heavy builds belong in CI due to container limits | CONSTRAINED |
| Android NDK / clang | Pinned r28c verified locally, then removed to conserve quota; CI workflow installs it | CI-MANAGED |
| Tablet access | No `adb`, no `/dev/bus/usb`, no Android mounts, ports 5037/5555/8022 closed. Owner confirms tablet testing is done on their side. | OWNER-RUN — provide build + test plan, never claim results here |
| Android build checkout | `/root/horizon-android-build` absent; owner clarified `/root/horizon-android-old` is the local source **and** build/tooling tree | RESOLVED |
| Mojang files in repo | `git ls-files` contains no JAR/asset/prepared-data files; `git grep` found no credential strings | OK |

The inherited `/root/horizon-android-old/LOCAL_ENVIRONMENT.md` forbids local builds/tests in that
old workspace; none were ever run there. The owner confirms this container is Linux x86_64 in proot
inside a GuppyNodes/Pterodactyl **free** server: 3 vCPU, 2.5 GiB RAM (about 712 MiB used at idle),
10 GiB disk (about 4.9 GiB used); `df -h` shows the host overlay and is misleading. Heavy compilation
is deliberately avoided to keep from maxing the server; it belongs in CI. Only light, bounded
`cargo test` runs are used for module verification. Tablet validation is explicitly owner-run.

## Hard blockers requiring owner decisions (do not mark as resolved)

1. **Tablet validation is owner-run.** This container has no `adb`, no `/dev/bus/usb`, no Android
   mounts and ports 5037/5555/8022 closed; the owner confirmed the tablet test is theirs to run.
   Our deliverable is a reproducible ARM64 build plus a written acceptance procedure
   (`TABLET_TEST_PLAN.md`). M1.3–M1.8 stay unchecked until real device logs are returned.
2. **A truly headless engine (no wgpu/winit) does not exist upstream.** Evidence: upstream
   `crates/horizon-client/Cargo.toml:63-64`, `horizon-client-render/Cargo.toml:66-67`,
   `horizon-client-lib/Cargo.toml:62-63`, `horizon-client-input/Cargo.toml:14` make them
   unconditional; `docs/server.md:419-428` calls this future work after perf-ticks-2. 97 source
   files use `winit::keyboard::KeyCode`. This is a multi-week refactor, not a feature flag.
   Owner decision needed: fund that refactor, or approve a documented interim deviation.
3. **No pure-Rust generator exists for the prepared tables** (`facts.json`, `registries.json`,
   `commands.json`, etc.). Evidence: Android `official_install.rs:582` extracts a bundled
   `engine-data.zip` made by the JDK preparer; upstream `crates/horizon-prepare/src/lib.rs:1-6`
   runs Java exporters. TASK.md §2.3 requires Rust-only runtime preparation. Owner decision
   needed: authorise a signed generated-table bundle from an owner-controlled endpoint or an
   operator-supplied one-time prepared folder, or fund implementation of the generators.
4. **Android CLI + winit is unproven** because winit's Android backend expects `android_main()` /
   `AndroidApp` (Android fork `third_party/winit/src/event_loop.rs:101`), not a Termux CLI.

None of these blockers is resolved. No acceptance criterion has been weakened.

## Milestone-one acceptance checklist (TASK.md §0, §8 — Android ARM64 subset)

- [~] M1.1 `aarch64-linux-android` cross-compilation succeeds with pinned NDK/Rust toolchain; exact
      commands and versions recorded in `DEV.md`. **Partial:** CLI skeleton cross-compiles (ELF64
      AArch64 verified); the final engine-linked binary does not exist yet.
- [ ] M1.2 Exactly one `horizon-server` ELF executable is produced (no APK, JNI entry, sidecar, JAR,
      helper or Mojang content); architecture verified with `file`/`readelf`.
- [ ] M1.3 Binary starts in an empty Termux directory, explains the EULA and refuses to start without
      explicit affirmative acceptance; non-interactive start never silently accepts.
- [ ] M1.4 Bootstrap downloads/pins/verifies Minecraft 26.3 inputs from official Mojang endpoints,
      writes prepared data outside the binary, and reuses verified cache offline.
- [ ] M1.5 Vanilla-style directory (`server.properties`, `eula.txt`, player lists, `logs/`, world)
      and world create/open/save/restart work on the tablet.
- [ ] M1.6 A real vanilla Java 26.3 client connects, plays, moves, loads chunks, saves/restarts and
      travels dimensions; result labelled with exact validation level.
- [ ] M1.7 Commands, whitelist/bans/ops, datapacks (directory + ZIP), enable/disable persistence and
      `/reload` verified against pinned vanilla behavior.
- [ ] M1.8 Graceful shutdown (SIGINT/SIGTERM where available) saves state and closes cleanly.
- [ ] M1.9 No mod/plugin/code-mod/WASM runtime is linked or reachable in the server build; audited.
- [ ] M1.10 Android client and desktop client behavior are not regressed (no changes made to those
      source repositories during initialization).

## Later-phase checklist

- [ ] Android ARMv7, x86, x86_64 built and run-tested (deferred until M1 stable).
- [ ] Linux x86, x86_64, ARMv7, ARM64 built and validated.
- [ ] Windows x86, x86_64, ARM64 built and validated.
- [ ] macOS x86_64, ARM64 built and validated.
- [ ] CI for all 13 targets, artifact audit, checksums, release metadata.
- [ ] Full `TASK.md` §8 definition of done and §9 final report.

## Evidence log

- 2026-11-24 — repository initialization: branch `server`, commit `8d8b1c9`, four tracked files,
  `DEV.md` ignored by `.gitignore:2:/DEV.md`, remote head equals local HEAD, remote tree contains
  only the four tracked files, remote default branch `server`.
- 2026-11-24 — environment inventory: no `cargo`, `rustc`, `rustup`, `ndk-build`, `clang`, `adb`,
  `cmake`, `ninja` or `make` on PATH; GitHub API reachable; three research agents launched for
  device/toolchain discovery, headless source mapping and Android build pipeline analysis.
- 2026-11-24 — owner clarification: `/root/horizon-android-build` is not a separate checkout; the
  local Android source and build/tooling tree is `/root/horizon-android-old`. Repository docs
  updated accordingly; no second tree will be assumed.
- 2026-11-24 — toolchain/device research: network reachable (`static.rust-lang.org`, `dl.google.com`,
  `sh.rustup.rs` all HTTP 200). `df`/`free` initially reported host values; the owner later corrected
  these to the panel's 2.5 GiB RAM / 10 GiB disk quota. Fork pins Rust 1.95,
  `aarch64-linux-android`, NDK 28.2.13676358, android-36. No tablet access path exists in this
  environment, so all real-device acceptance items remain blocked.
- 2026-11-24 — build pipeline research: fork's native server build recipe identified
  (`cargo build -p horizon-client --no-default-features` + `-p horizon-server`, Dockerfile), NDK env
  vars and Rust-only resource pipeline (`official_install.rs`, `android_resources.rs::rebuild`).
  Version discrepancy found: fork branch `dev/26.4` pins `26.4-snapshot-10` while canonical upstream
  and `TASK.md` require `26.3`; upstream 26.3 is authoritative for this product.
- 2026-11-24 — toolchain provisioning started in background: rustup 1.95.0 with
  `aarch64-linux-android` target, NDK r28b zip download. Not yet complete/verified.
- 2026-11-24 — first vertical slice implemented (CLI, `server.properties` default/preservation,
  explicit EULA acceptance, internal-mode guard) with **12/12 host unit tests passing**
  (`cargo test` output: `test result: ok. 12 passed; 0 failed`).
- 2026-11-24 — cross-compile verified: `cargo build --release --target aarch64-linux-android`
  with NDK clang/linker produced a 342 KB ELF; `llvm-readelf -h` → `Class: ELF64`, `Machine:
  AArch64`. This is the CLI skeleton only, not an integrated server binary.
- 2026-11-24 — NDK revision correction: the `android-ndk-r28b-linux.zip` package reports
  `Pkg.Revision = 28.1.13356709`; the fork's pinned `28.2.13676358` is `r28c`
  (`android-ndk-r28c-linux.zip`, verified from `source.properties`). CI workflow corrected to r28c.
- 2026-11-24 — source-mapping research completed: no existing headless server crate; dedicated
  logic lives in `game/interface/dedicated/mod.rs` (~3,272 lines upstream) and needs extraction;
  no pure-Rust prepared-table generator exists. Details in `ARCHITECTURE.md` and blockers above.
- 2026-11-24 — resource-bootstrap core added (pinned 26.3 metadata, official-host allowlist,
  SHA-1/size checks, atomic cache, offline reuse). Host test run at that point: **15 passed, 1
  failed** (`corrupt_cache...` asserted the wrong temporary filename); the assertion was corrected
  to `fixture.corrupt` but **not re-run** after the owner asked to stop testing due to a storage
  incident. Do not claim 16/16.
- 2026-11-24 — workspace restart removed the extracted NDK and rustup toolchain; the large NDK
  zips and build directories created here were deleted to free storage. No builds/tests are being
  run while the storage incident is unresolved.
- 2026-11-24 — `/root/horizon-android-old/LOCAL_ENVIRONMENT.md` respected: no build, test, game or
  launcher was ever run in that workspace; all compilation happened in the new repo with a freshly
  installed toolchain. No emulator, cross-compile or host result is claimed as tablet validation.
- 2026-11-24 — bootstrap assertion fix re-verified: `cargo test` → **16 passed, 0 failed**.
- 2026-11-24 — vanilla server-directory layout and player-list modules added (lazy creation, no
  overwrite, path-traversal guard, unknown JSON fields preserved): `cargo test` → **23 passed,
  0 failed**. These are module-level tests only, not server runtime validation.
- 2026-11-24 — workspace restart removed `/root/horizon-upstream`; the pinned 26.3 data-pack format
  was re-verified from the upstream commit via raw GitHub content:
  `DATA_PACK_FORMAT: (i64, i64) = (121, 0)`, `DATA_VERSION = 5023`. Datapack discovery/validation
  module added against that pin: directory + ZIP packs, format/range checks, deterministic ordering.
- 2026-11-24 — full local suite after datapack addition: `cargo test` → **26 passed, 0 failed**
  (includes directory/ZIP datapack, player-list, server-dir and EULA tests). Module-level evidence
  only; no runtime/tablet claims.
- 2026-11-24 — owner clarified the real environment: Linux x86_64 proot inside a GuppyNodes free
  Pterodactyl server (3 vCPU, 2.5 GiB RAM, 10 GiB disk; `df -h` misleading), no adb, tablet testing
  is owner-run. `TABLET_TEST_PLAN.md` added as the handoff procedure; local heavy builds stopped.
- 2026-11-24 — same-executable self-spawn boundary implemented (`src/supervisor.rs`): child command
  always uses `current_exe()`, internal flag is first, no setup in internal mode. Full suite:
  **28 unit tests + 2 black-box integration tests passed, 0 failed**. The child still reports the
  engine placeholder error — no simulation is claimed.
- 2026-11-24 — `DECISIONS_NEEDED.md` created with the three unresolved choices (headless boundary,
  prepared-table source, tablet evidence) so the owner can unblock the remaining implementation.
- 2026-11-24 — CI build caching added per owner request: `Swatinem/rust-cache@v2` for compiled crates
  and `actions/cache@v4` for the pinned NDK, so repeat runs only rebuild this repository's code.
  Local `target/` is retained (never `cargo clean`ed) for the same reason.
- 2026-11-24 — resource staging added: verified client JAR entries under `data/`,
  `assets/minecraft/font/` and `assets/minecraft/textures/` are copied into a versioned directory
  outside the binary with zip-slip rejection and atomic finalization. Manifest is explicitly
  provisional because the engine manifest format is not integrated. Full suite after the change:
  **30 unit + 2 black-box integration tests passed, 0 failed**.
- 2026-11-24 — `server.properties` defaults expanded to 50 keys verified directly from upstream
  `horizon-server-files/src/properties.rs` at commit a02b2838 (e.g. `motd=A Minecraft Server`,
  `level-type=minecraft:normal`, `white-list=true`, `max-world-size=29999984`); removed the invalid
  `pvp` server-property key. Local test run in progress.
- 2026-11-24 — prepared-data verifier integrated as `horizon-server prepare --prepared-folder <path>`.
  Independently reviewed (pure Rust SHA-256, no new dependency, no `unsafe`, path/symlink hardening)
  and verified end-to-end with a synthetic manifest. Full suite: **40 unit + 3 black-box integration
  tests passed, 0 failed**.
- 2026-11-24 — CI artifact handoff verified: downloaded run 45320154503 artifact
  `horizon-server-aarch64-linux-android.tar.gz` (SHA-256
  `392c37cbb74fdeaa80d4e11e79249ef133a2176b84858d5ac1d08385fc6db8cd`); archive contains exactly
  `horizon-server` and `horizon-server.sha256`; ELF header bytes are ELF64 / machine 183 (AArch64).
  This is the CLI skeleton build only — not a playable server.
- 2026-11-24 — vendored upstream tree completed: all 11,691 tracked files at pinned commit restored
  (11,692 files incl. `SOURCE.md`, 203 MB); forbidden-file scan empty; `cargo metadata --no-deps`
  parses **77 workspace members**. Committed in `3aba9e5`.
- 2026-11-24 — bundle-capable ARM64 build: CI runs 45359218116 and **45361395498 (final) GREEN**
  (host tests, smoke test, no-mod audit, packaging). Final artifact independently verified: ELF64/
  AArch64. Archive SHA-256 `9be4d4cdd2e33ce08bb58f494863fd52df091f857199f72de2c427d263435837`;
  binary SHA-256 `39248702cadd9ab9d085bb24f7795e552384d424fa5242e6d524ad9e1facf358`. (Binary hashes
  differ between CI runs; builds are not byte-reproducible yet.) Combined with the prepared bundle
  below, this is the complete no-PC handoff for the tablet.
- 2026-11-24 — no-PC prepared data: CI workflow `prepare-data.yml` + `scripts/generate_prepared_bundle.sh`
  generated a verified 26.3 dataset from official Mojang inputs with build-time-only Java 25
  (5,214 files verified, 23,218 prepared files). Run 45360507408 **green**. Artifact independently
  downloaded: 44,872,319 bytes, SHA-256
  `e749897938efa6fd645c293ca5b6082417c0cdbc14ef45fa0c66ed310ab5fa16`, 23,219 ZIP entries (manifest
  + 23,218 listed files), includes facts/block-tags/registries/commands/horizon-resources.json, no
  JARs/engine-data.zip/horizon-data.bin. Binary-side `accept_bundle` verifier was exercised in that
  CI run. This satisfies the approved interim prepared-data source; Java is never needed at runtime.
- 2026-11-24 — **second real tablet runtime evidence (owner)**: with the argv fix, the binary
  downloaded and SHA-1-verified the 26.3 client JAR, staged resources, reached the engine, printed
  `Horizon 0.1.0`, and then stopped at `no horizon-resources.json in .../.horizon/resources/26.3`
  (crash `crash-2026-11-24_16.32.29-server.txt`). Two defects found: (1) staging wrote to
  `.horizon/downloads/resources/26.3` instead of `.horizon/resources/26.3` — fixed in
  `Bootstrap::resource_dir`; (2) `try_lock() not supported` on Termux log file — fixed by skipping
  advisory locking on Android in vendored `logging.rs` (documented in `vendor/horizon/SOURCE.md`).
  Remaining blocker: a verified prepared-data folder (facts.json, block-tags.json, registries.json,
  commands.json + `horizon-resources.json`) must be supplied via `horizon-server prepare`.
- 2026-11-24 — rebuild after the tablet-crash fix: CI run 45344696712 **green** (all steps incl.
  host smoke test, no-mod audit and packaging). New artifact independently verified: two archive
  entries only, ELF64/AArch64, 91,568,712 bytes. Archive SHA-256
  `a8db77448d231b2317265d6c9f7f0c695ac0b285735e334c52396837860b9e16`, binary SHA-256
  `32b9a4e4bb9be73927baaf6767fa503b0538bf41e71579ef417a50bf7397c8fb`. Awaiting owner retest.
- 2026-11-24 — **first real tablet runtime evidence (owner)**: engine started on Android aarch64
  (crash report `crash-2026-11-24_14.57.04-server.txt`, log lines `Horizon 0.1.0` then
  `Horizon: resources is not a recognized option`). Root cause: upstream's launch parser stops at
  `--dedicated` and routes later arguments to the dedicated-only parser, which rejects `--resources`.
  Fix: pass global `--resources <path>` **before** `--dedicated`; unit test asserts the ordering.
  This is the first device result — it proves native startup, not a working server yet.
- 2026-11-24 — **final engine-enabled ARM64 run 45333795229 GREEN**: host tests, host smoke test
  (`cargo run --locked --features engine -- --version`), cross-compile, no-mod graph audit, artifact
  audit/package and upload all passed (33m31s). Final artifact independently verified: exactly two
  archive entries, ELF64/AArch64, 91,568,072 bytes. Archive SHA-256
  `d6f3a0c0de888d9dc65dc5f5fe3af941ce2034f5c0918553458942620f281548`; binary SHA-256
  `f973fa5884983ae26dbbdb7a76ac760577e94d8785bf0bb302990eeb7bba0d7d`. Build/audit evidence only —
  no runtime or tablet result.
- 2026-11-24 — startup correctness fix: internal engine mode is now selected by the private env var
  `HORIZON_SERVER_INTERNAL_ENGINE=1` instead of a private argv flag, and the child is launched with
  only upstream-valid arguments (`--dedicated --nogui --resources … --universe …`). Without this,
  upstream's strict `entry::prepare()` would have rejected our `--dir`/internal flag. Local
  `cargo test --locked` → **41 unit + 3 integration passed**. Not yet runtime-verified on a device.
- 2026-11-24 — **engine-enabled ARM64 CI run 45330665004 GREEN**: host tests, cross-compile with
  `--features engine`, no-mod graph audit, artifact audit/package and upload all passed (23m28s).
  Downloaded artifact independently verified: archive contains only `horizon-server` and
  `horizon-server.sha256`; ELF64/AArch64 (machine 183), **91,568,072 bytes**; archive SHA-256
  `c97e653060eda6f5bfa57874ec463eaabb609e1aee6826c828864f4f73cc1cdf`, binary SHA-256
  `7965bbd03167cac28cb5cad28f9b08d2ff7be4dd52b489c60fde76b645fde3fd`. This is a build/audit result
  only — no runtime or tablet validation has occurred.
- 2026-11-24 — sixth engine-enabled ARM64 run 45329349790: host tests green, cross-compile failed
  with E0609 `no field key on KeyEvent` at the patched window handler. Corrected the Android fallback
  to use `event.logical_key.clone()` (winit 0.30 field name). Rebuild pending.
- 2026-11-24 — fifth engine-enabled ARM64 run 45327640706: host tests green, cross-compile failed
  with E0432/E0599 (`winit::platform::modifier_supplement` / `key_without_modifiers` absent on
  Android) at vendored `window_handler.rs`. Fourth documented patch gates them off on Android with
  `event.key.clone()` fallback; desktop behavior unchanged. Rebuild pending.
- 2026-11-24 — fourth engine-enabled ARM64 run 45326517856: host tests green, cross-compile failed
  with E0432/E0599 (`winit::platform::scancode` / `to_scancode` absent on Android) at vendored
  `horizon-client-input/src/keybinds.rs`. Minimal documented patch gates the scancode helper off on
  Android with a 0 fallback; desktop behavior unchanged. Rebuild pending.
- 2026-11-24 — third engine-enabled ARM64 run 45325742272: host tests green, cross-compile failed with
  E0425 `cannot find value uid` at vendored `local_control/unix.rs:193` — peer-credential branch was
  Linux/macOS-only. Minimal documented patch extends the Linux `SO_PEERCRED` branch to Android.
  Rebuild pending.
- 2026-11-24 — second engine-enabled ARM64 run 45325420328 failed earlier in **host tests**: two
  bootstrap staging tests shared `/tmp/resources/26.3` because their synthetic `cache_dir` was the
  temp dir itself rather than a production-like `.horizon/downloads/26.3` path. Fixed the fixtures;
  local `cargo test --locked` → **41 unit + 3 integration passed, 0 failed**. This was a real
  test-isolation bug found only by CI, not a flake.
- 2026-11-24 — first engine-enabled ARM64 CI run 45324523961 **failed** (exit 101) while compiling
  `horizon-mod-package` for Android: E0433 `cannot find holder in crate` at
  `crates/horizon-mod-package/src/runtime_leases.rs:431`, because upstream gates `holder`/
  `local_control` to linux/macos/windows while Android still compiles the referencing module.
  Minimal documented vendor patch adds `target_os = "android"` to those two cfg blocks; no mod
  runtime is added. Rebuild pending.
- 2026-11-24 — engine-link CI run 45323508643 **success**: both "Type-check the no-mod engine entry"
  and "Audit that no mod runtime entered the graph" passed on hosted CI. The direct entry seam is
  now type-verified (host toolchain). ARM64 workflow updated to build with `--features engine` and
  to repeat the no-mod graph audit for the shipped target; requires a fresh run to verify.
- 2026-11-24 — opt-in `engine` feature prepared in the root crate: optional path deps on vendored
  `horizon-client`/`horizon-mod-package` plus the direct `entry::prepare`/`entry::start` seam in
  internal mode. Feature is **off** and not type-verified; default `cargo test` still passes
  (**41 unit + 3 integration, 0 failed**) after lockfile resolution (5,108 lines). CI engine-link
  run 45323508643 remains in progress.
- 2026-11-24 — engine-link feasibility spike result: a standalone crate can call
  `horizon_client::entry::prepare()` + `entry::start()` with an empty `Runtime::default()` and no-op
  code hooks, reaching the real dedicated loop without `horizon_game::main()`. `cargo tree` on the
  generated lockfile (497 packages) showed **no wasmtime, horizon-mod-host or horizon-mod-wasm**;
  wgpu/winit/naga remain unconditional (owner-approved interim deviation). The local check exited 124
  at the 10-minute timeout while compiling 496 dependencies, with no errors and before type-checking
  the probe, so the API calls are **not yet type-verified**. CI workflow `engine-link.yml` added with
  a 45-minute timeout to verify them on a hosted runner.
- 2026-11-24 — asset-index verification added: 8 accepted 26.3 hashes from upstream
  `research-lock.json`, official-host check, size/hash verification, corrupt-cache repair and offline
  reuse. Full suite: **41 unit + 3 integration tests passed, 0 failed**.
- 2026-11-24 — pinned upstream source snapshot vendored into `vendor/horizon/` (148 MB, 9,767 files)
  from commit `a02b2838d34f4c99726cca349ec0b4d62c5c8fa5`; independently verified: no `.jar`,
  `horizon-data.bin`, `engine-data.zip`, `.zip`, `.git`, `.local` or `target` files; version pins
  confirmed (26.3 / protocol 777 / data-pack format 121). Provenance in `vendor/horizon/SOURCE.md`.
  Binary source fixtures were pruned, so the snapshot is a reference tree, not yet a build tree.
- 2026-11-24 — CI watcher/fixer fixed the red workflow: the NDK env vars were only written to
  `$GITHUB_ENV` and were not visible to the separate verification step; they are now set inside the
  cross-build step itself. Verified with `gh run list`: runs 45320021389 and 45320154503 both
  `success` (host tests, ARM64 cross-build, audit/package, artifact upload).
- 2026-11-24 — CI watcher/fixer subagent launched with sole ownership of `.github/workflows/**` after
  all `server-arm64` runs failed at "Verify NDK revision" (missing `source.properties` path).
- 2026-11-24 — owner-approved interim path recorded (`79a29b4`). Parallel work launched with
  exclusive ownership: vendoring pinned upstream source (`vendor/**`), prepared-data verifier
  (`src/prepared.rs`), and artifact audit/package scripts (`scripts/**`). Integration lead retains
  `Cargo.toml`, `src/main.rs`, workflows and documentation.
