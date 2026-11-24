# Horizon Server — repository instructions

This is a **new, standalone product repository** for a single, self-contained, vanilla-compatible
Minecraft Java Edition 26.3 dedicated server executable named `horizon-server`. It is not a fork of
`horizon-rust/horizon` and not a subfolder of the Android fork. Source repositories are references
only; do not implement, port, or build server functionality inside them.

**Current status: repository initialization only.** No server implementation, porting, or build has
been started. Do not begin the `TASK.md` milestones until the owner explicitly starts the task after
reviewing this initialized repository.

## Read first

- [`TASK.md`](TASK.md) — the durable product specification, full target matrix and definition of done.
  It is the authority for requirements and the staged roadmap.
- This file — this repository's own rules.
- `DEV.md` — **local-only**, gitignored machine-specific paths, environment, build commands and device
  notes. If absent, recreate it locally; never commit it.
- `/root/horizon-android-old/AGENTS.md` and `/root/horizon-android-old/LOCAL_ENVIRONMENT.md` —
  inherited environment/contributor constraints from the Android fork, read in full.
- The upstream repository `https://github.com/horizon-rust/horizon` (local reference checkout
  `/root/horizon-upstream`) and the local Android reference `/root/horizon-android-old`. Inspect their
  current branches, commits, status and files before relying on them; do not assume they are
  interchangeable or that a local checkout is current.

## Source repository relationships

- `https://github.com/horizon-rust/horizon` — **canonical upstream reference** for current
  architecture, behavior, protocol (777), vanilla parity, `research-lock.json` version pins and
  upstream changes. Default branch `main`, LGPL-3.0. The local reference checkout
  `/root/horizon-upstream` was observed on `main` at commit `a02b2838d34f4c99726cca349ec0b4d62c5c8fa5`
  (2026-11-21) and may lag the live repository. Trust the live repository for current state.
- `https://github.com/ProSteve123GH/horizon-android-old` — the owner's **Android port fork** of
  upstream, local checkout `/root/horizon-android-old`. Observed on branch `dev/26.4` at commit
  `6f2e86c3a85e831035f60242d18a80435fd1d3c2`; its remote HEAD branch is `android`. Reference for the
  Android-compatible Rust implementations, the 26.3 download/import flow, prepared data tables and
  existing Android work. Its `CONTRIBUTING_ANDROID.md`, `docs/android.md`,
  `docs/android-game-files.md`, `docs/datapack-upgrade.md` and `AGENTS.md` are the Android-side
  references. Do not copy Android UI, JNI, SAF, touch/Bedrock control or APK packaging concerns here.
- `/root/horizon-android-build` — intended **local Android build/tooling checkout**. It was **not
  present** when this repository was initialized; do not invent its contents. Inspect it if/when it
  appears and record what is actually there in `DEV.md`.
- `/root/horizon-android-mod-builds` — a *different* repository
  (`ProSteve123GH/horizon-android-mod-builds`, a mod build service with `worker/`, `scripts/` and
  `fixtures/`). It is not the expected build-tooling checkout and must not be treated as such.
- This repository — the **standalone server product** and its build/release infrastructure only.
  Preserve the upstream and Android forks; avoid unrelated refactors or changes that break them.

Milestone one is deliberately narrow: a native `aarch64-linux-android` (ARM64 / `arm64-v8a`) CLI
executable that runs in Termux on the real tablet. Keep other targets possible, but defer ARMv7,
x86 and x86_64 Android and all Linux/Windows/macOS targets until ARM64 is stable and validated on
the device. Cross-compilation is not runtime validation; do not claim platform support without a
run test.

## Rules

- **Initialization only for now.** Do not add engine code, functional server implementation, ports,
  Cargo workspaces, CI workflows or build scripts until the owner starts the task. Documentation and
  repository hygiene only.
- **Never commit Mojang's files**: game/server JARs, assets, decompiled source, generated resource
  folders, `.local/`, prepared data (`horizon-data.bin`), or real player/world data. The product will
  download and verify inputs directly from official endpoints at runtime; no game content is bundled.
- **No secrets in tracked files or local notes**: no tokens, credentials, signing material, private
  user data or account data. `DEV.md` stays local and gitignored.
- **Keep runs away from real player data**: any future game/launcher/server run uses fresh scratch
  directories (logs, crash reports, telemetry consent, recordings); never write into real saves,
  `~/.local/share/...` or `~/.local/bin`.
- **Inherited environment constraint:** the old Android workspace (`/root/horizon-android-old`)
  forbids local building/testing because of tight RAM/disk under proot; all verification there is
  CI's job. Do not run `cargo build`/`check`/`test`/`clippy`, harnesses or emulators there. The new
  repository's own build/validation policy is recorded in local-only `DEV.md`; do not assume a local
  build is available here either.
- **Mark what an agent posts.** A PR body starts with a line naming the person whose agent opened it
  and the tool: `> 🤖 Opened by <login>'s AI agent (<tool, model>), not by <login> personally.`,
  where `<login>` is the GitHub account the PR is opened from (`gh api user --jq .login`). Issues say
  "Filed by", reviews "Review by". Every comment starts with `🤖 Agent:`. Write only as the person you
  work for; the lines in the upstream `AGENTS.md` naming specific maintainers are for their own agents.
- **Attribution.** Commits end with a `Co-Authored-By:` line for the agent that wrote them; PR bodies
  carry that agent's "Generated with" line. Follow the upstream example's format, substituting your
  own tool/model and account. Preserve upstream attribution and LGPL-3.0 history when code is
  eventually reused; keep third-party notices accurate.
- **Stay in the vanilla engine → platform → Horizon layer direction** from upstream
  `docs/layers.md`. Never introduce a dependency from the vanilla engine into the Horizon layer.
  Split/gate crates so the headless server graph excludes `wgpu`, `winit`, audio, controllers, JNI,
  launcher and client-mod runtimes — verify the actual dependency graph, not just feature flags.
- **Vanilla parity is the spec.** Use the pinned 26.3 records in upstream `research-lock.json` and the
  parity harnesses as references. Separate "compiled", "ran on target", "connected with a vanilla
  client" and "parity-tested" claims; never invent test results.
- **No mods.** Fabric/Forge/NeoForge/Quilt/Mixin/plugins, `.hmod`, WASM/component mods, modpack import
  and code-mod runtime are out of scope for this server. Vanilla datapacks are required and are not
  mods.
- **Write tests with signal** when implementation starts: observable behavior, independent expected
  values or a vanilla oracle, one test per changed behaviour, synthetic fixtures and isolated
  temporary directories. Never delete parity harnesses that pin vanilla behavior.
- **Work Safely.** Inspect `git status` before work; preserve pre-existing uncommitted changes. Do
  not reset, clean, force-push or overwrite unrelated work. Keep PRs small. The default branch is
  `server` — never create or switch the default to `main`.
- **Eventual target matrix** (13 targets) is staged in `TASK.md` §3; Android ARM64 comes first and
  every later target waits for ARM64 stability. Build/release infrastructure belongs in this new
  repository and must audit artifacts so exactly one server executable per target is released.

## Repository layout (initialization)

```text
AGENTS.md   # this file (tracked)
TASK.md     # supplied durable specification (tracked verbatim)
README.md   # minimal project overview and staged roadmap (tracked)
.gitignore  # ignores DEV.md and local/Mojang/build artifacts (tracked)
DEV.md      # local-only environment notes (gitignored, never committed)
```
