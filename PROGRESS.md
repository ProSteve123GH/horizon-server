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
| Tablet access | `command -v adb` → MISSING; `/dev` has no USB/tty device | BLOCKER (under investigation) |
| Android build checkout | `/root/horizon-android-build` absent; owner clarified `/root/horizon-android-old` is the local source **and** build/tooling tree | RESOLVED |
| Mojang files in repo | `git ls-files` → only `.gitignore`, `AGENTS.md`, `README.md`, `TASK.md` | OK |

The inherited `/root/horizon-android-old/LOCAL_ENVIRONMENT.md` describes the *old* agent workspace
(700 MB RAM under proot, no local builds). The machine observed here is different (~32 GB RAM,
8 CPUs); that note's no-local-build rule is retained as a safety default until a local toolchain and
validation policy are established in local-only `DEV.md`.

## Milestone-one acceptance checklist (TASK.md §0, §8 — Android ARM64 subset)

- [ ] M1.1 `aarch64-linux-android` cross-compilation succeeds with pinned NDK/Rust toolchain; exact
      commands and versions recorded in `DEV.md`.
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
