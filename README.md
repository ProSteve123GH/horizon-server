# horizon-server

A standalone, self-contained, vanilla-compatible Minecraft Java Edition 26.3 dedicated server
built from the Horizon engine. The release artifact is a single native executable named
`horizon-server` (`horizon-server.exe` on Windows): no separate engine, launcher, preparer, JDK,
Python, Docker or helper binary at runtime. Each operator's executable downloads and verifies the
required Mojang inputs directly from official endpoints; no Mojang game files are bundled.

**Status: implementation underway, milestone one (Android ARM64).** The CLI/config/EULA/bootstrap/
datapack/self-spawn scaffolding is unit-tested (41 unit + 3 integration tests passing), and a direct
no-mod engine entry seam has been type-verified on hosted CI. An engine-enabled ARM64 cross-build is
in progress; no playable server or tablet validation exists yet. See [`PROGRESS.md`](PROGRESS.md)
for the live acceptance checklist, evidence and blockers, and [`TABLET_TEST_PLAN.md`](TABLET_TEST_PLAN.md)
for the owner-run acceptance procedure. [`TASK.md`](TASK.md) holds the full requirements, target
matrix and definition of done; [`AGENTS.md`](AGENTS.md) holds contributor rules.

## Scope

- A clean directory plus `./horizon-server` should perform first-run setup, require explicit EULA
  acceptance, bootstrap and verify pinned 26.3 resources, then start a vanilla-style server.
- Vanilla `server.properties`, worlds/saves, commands, player files, networking and datapacks are in
  scope. Mods, mod loaders and plugin systems are explicitly out of scope.
- The supervisor and headless engine are modes of the same executable; the headless dependency graph
  must exclude GUI/render/window/audio/controller/JNI/client-mod runtimes.

## Source relationships

- Canonical reference: [`horizon-rust/horizon`](https://github.com/horizon-rust/horizon) — architecture,
  behavior, protocol, vanilla parity and version pins.
- Android reference: [`ProSteve123GH/horizon-android-old`](https://github.com/ProSteve123GH/horizon-android-old)
  — Android-compatible Rust implementations, 26.3 download/import flow and prepared data tables.
- This is a **new, separate product repository**, not a fork and not an implementation location
  inside either reference repository.

## Staged roadmap

1. **Android ARM64 first** (`aarch64-linux-android` / `arm64-v8a`): native Termux CLI executable,
   resource bootstrap, vanilla configuration/datapacks and real tablet validation.
2. Remaining Android ABIs: ARMv7, x86, x86_64.
3. Linux: x86, x86_64, ARMv7, ARM64.
4. Windows: x86, x86_64, ARM64.
5. macOS: x86_64, ARM64.

All 13 targets are the eventual release goal. No later target begins until ARM64 is stable and
run-tested on the tablet; cross-compilation alone is not validation.

## Local development

Machine-specific paths, environment, build commands and device notes belong in local-only `DEV.md`,
which is gitignored and must never be committed. Inherited workspace constraints are documented in
`/root/horizon-android-old/AGENTS.md` and `/root/horizon-android-old/LOCAL_ENVIRONMENT.md`.
