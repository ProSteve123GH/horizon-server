# Progress, evidence and blockers

Integration lead's running checklist for the standalone Horizon Dedicated Server. This is a living
document: each claim below is backed by a command output, file path or test result in this repository
or the session transcript. **Do not mark an item done on a successful compile alone** — `TASK.md` §8
requires runtime and parity evidence.

## Current phase

- [x] Repository initialized (`server` branch, remote, tracked docs, ignored `DEV.md`, clean commit).
- [ ] Phase 0 — environment/toolchain/device feasibility and source mapping (in progress).
- [ ] Phase 1 — Android ARM64 (`aarch64-linux-android`) native Termux CLI, milestone-one acceptance.
- [ ] Phase 2 — remaining Android ABIs.
- [ ] Phase 3 — Linux targets.
- [ ] Phase 4 — Windows targets.
- [ ] Phase 5 — macOS targets.

No server functionality has been implemented, compiled or run yet. No milestone from `TASK.md` has
been started beyond repository setup and this investigation.

## Environment facts (observed 2026-11-24)

| Item | Evidence | Status |
|---|---|---|
| Local repo | `git status -sb` → `## server...origin/server`; `git log -1` → `8d8b1c9` | OK |
| GitHub remote/auth | `gh api user --jq .login` → `ProSteve123GH`; remote default branch `server` | OK |
| Host resources | `free -m` → ~32 GB RAM; `df -h /` → ~321 GB free; `nproc` → 8 | OK |
| Rust toolchain | `command -v cargo rustc rustup` → all MISSING | BLOCKER |
| Android NDK / clang | `command -v ndk-build clang` → MISSING; no Android env vars | BLOCKER |
| Tablet access | Research agent: no `adb`, no `/dev/bus/usb`, no Android mounts, ports 5037/5555/8022 closed, no Termux host/credentials in docs/scripts/history. Gateway `:22` is a Linux SSH host, not Android. | BLOCKED — owner must supply access |
| Android build checkout | `/root/horizon-android-build` absent; owner clarified `/root/horizon-android-old` is the local source **and** build/tooling tree | RESOLVED |
| Mojang files in repo | `git ls-files` → only `.gitignore`, `AGENTS.md`, `README.md`, `TASK.md` | OK |

The inherited `/root/horizon-android-old/LOCAL_ENVIRONMENT.md` describes the *old* agent workspace
(700 MB RAM under proot, no local builds). The machine observed here is different (~32 GB RAM,
8 CPUs); that note's no-local-build rule is retained as a safety default until a local toolchain and
validation policy are established in local-only `DEV.md`.

## Hard blockers requiring owner decisions (do not mark as resolved)

1. **No Android tablet access path exists in this environment.** Evidence: no `adb`, no
   `/dev/bus/usb`, no Android mounts, TCP 5037/5555/8022 closed, no Termux SSH host/IP/credentials
   in docs/scripts/history. Owner must provide a device connection or run the acceptance steps on
   the tablet and return logs. Until then M1.3–M1.8 cannot be validated and must stay unchecked.
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
  `sh.rustup.rs` all HTTP 200), ~321 GB disk and 32 GB RAM available. Fork pins Rust 1.95,
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
