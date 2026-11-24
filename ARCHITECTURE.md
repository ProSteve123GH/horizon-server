# Architecture note (integration lead)

Provisional design record required by `TASK.md` §5. This is a *planning* document, not evidence of
implementation. Sections marked **TBD** will be filled from the source-mapping research before any
shared interfaces or file ownership are fixed.

## Goal and hard constraints

One `horizon-server` executable per target, containing both:

1. **Supervisor/host** — CLI, first-run setup/EULA, resource bootstrap, configuration, logs,
   backup/restart handling and optional management API.
2. **Headless engine** — the existing Minecraft 26.3 dedicated simulation, protocol, world, commands,
   persistence and datapacks.

Supervisor starts the same `current_exe()` in a private internal engine mode; that mode is never a
second installed binary. No GUI/window/WGPU/audio/controller/JNI/launcher/mod-runtime initialization
may be reachable from the headless path. No Java/JDK, Python, Docker, launcher, `horizon-engine`,
`horizon-client`, `horizon-prepare` or helper executable at runtime. Mojang inputs are downloaded
and verified by the binary at runtime and stored outside it; nothing Mojang-derived is distributed.

## Current upstream baseline (from `horizon-rust/horizon` docs, observed 2026-11-24)

- The dedicated server today is the game itself run headless:
  `horizon-client --resources <prepared> --dedicated [--nogui] [--port N] [--world NAME] [--universe DIR] [--initSettings]`
  (`docs/server.md`). It reads/writes `server.properties` in vanilla syntax, refuses to start until
  `eula.txt` says `eula=true`, loads vanilla player lists, opens the level with the same `Game` as
  single player, ticks at 20 TPS, uses the LAN play hub for networking and writes `logs/latest.log`.
- `crates/horizon-server` is currently a **supervisor** crate: `server.toml`, CLI, management API,
  backups and a child-engine lookup (`docs/api/server.md`, crate manifest).
- Upstream pins Minecraft `26.3` in `research-lock.json` (protocol 777, data version 5023, Java 25).
  Its own `docs/hosting.md` states it currently reads prepared game files supplied by the operator —
  the new product must add automatic verified bootstrap from official Mojang endpoints.
- Version pins, vanilla parity and layer direction (`docs/layers.md`) come from upstream; the
  Android fork contributes its Rust download/import and table reconstruction code.

## Proposed binary modes

| Mode | Invocation | Responsibility |
|---|---|---|
| Supervisor (default) | `horizon-server [options]` | CLI dispatch, EULA/first run, bootstrap, config, spawn engine, logs, shutdown/restart. |
| Internal engine | `horizon-server <private internal flag> …` | Headless game loop only. Detected before normal setup so recursion is impossible. Sensitive data passed by inherited pipe/socket/protected file, not command line. |
| Utility commands | `--help`, `--version`, `init`, `status`, … | Must work without network or resource downloads where specified. |

## Configuration ownership

- `server.properties`: canonical vanilla gameplay settings, read/written with vanilla Java-properties
  semantics; unknown keys preserved; changes applied at vanilla lifecycle points.
- `eula.txt`: explicit operator acknowledgement only; never written silently, including in
  non-interactive mode (use an explicit flag/env instead).
- Player lists, world, `logs/`: standard vanilla names and formats.
- Horizon-only settings: separate, optional, namespaced file/section; existing `server.toml`
  directories migrate without losing worlds/settings. No duplicated source of truth.

## Resource bootstrap flow

1. Pin 26.3 from `research-lock.json`; allowlist official Mojang HTTPS hosts.
2. Fetch/validate metadata and artifact hashes/sizes; choose smallest sufficient input (server JAR
   vs client JAR path **TBD** from the Android fork's proven flow). Never execute either JAR.
3. Reconstruct required registries/tags/commands/worldgen tables in pure Rust, reusing the
   platform-independent parts of the Android fork's `android_resources.rs`; exclude Android UI/JNI.
4. Write a versioned, integrity-checked cache under the server data directory (e.g.
   `.horizon/resources/26.3`), download to partial files and atomically finalize; verify before
   reuse; support `--offline` and safe repair.
5. Headless engine starts only after the manifest verifies; client-only assets are not fetched
   unless the engine demonstrably reads them.

## Datapack flow

Vanilla `<world>/datapacks/` discovery for directory and ZIP packs; validate `pack.mcmeta` and
format; honour `level.dat` enabled/disabled lists and priority; load before worldgen/reload;
implement `/datapack list|enable|disable` and `/reload` atomically. Test with real 26.3-format
packs and a vanilla client. No mod/code-mod/WASM/plugin runtime anywhere in the build graph.

## Dependency boundaries (TBD)

Pending source-mapping research: exact crate list and feature edges for the headless graph; the
smallest reusable engine boundary; how to avoid copying gameplay logic; Android NDK cross-compile
configuration; whether the existing dedicated path can be compiled without WGPU/winit/audio/JNI.

## Target staging

Android ARM64 (`aarch64-linux-android`, Termux, real tablet) first; then remaining Android ABIs;
then Linux, Windows and macOS. Cross-compilation is never reported as runtime validation.

## Ownership model for parallel work

Integration lead owns this document, `TASK.md`, `AGENTS.md`, `PROGRESS.md`, `DEV.md` policy, the
final executable dispatch and shared CLI/engine protocol. Research agents are strictly read-only.
Implementation agents will receive exclusive file/path ownership before editing; no two agents edit
the same file concurrently, and their output is independently re-verified by the lead.
