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
3. Prepared-data source (owner-approved interim deviation): accept an operator-supplied or signed
   verified prepared-data folder, checking a versioned manifest and hashes, rather than generating
   `facts.json`/`registries.json`/`commands.json` in pure Rust. Reuse the platform-independent
   `android_resources.rs::rebuild` logic where it applies; exclude Android UI/JNI. This deviation is
   recorded in `PROGRESS.md` and is **not** a claim that `TASK.md §2.3` is satisfied.
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

## Dependency boundaries (source-mapping findings, 2026-11-24)

- There is **no existing headless server crate** to reuse. The dedicated server is the game run with
  `--dedicated`: upstream `crates/horizon-client/src/game/interface/dedicated/mod.rs` (~3,272 lines)
  plus seven siblings; the Android fork has a structurally different `game/dedicated.rs` (~3,043
  lines, ~846-line diff). Do not blindly merge them — upstream 26.3 is authoritative.
- `crates/horizon-server` is only a supervisor: it spawns `horizon-engine`/`horizon-client` as a
  separate process (`horizon-server-launch/src/lib.rs:19`). Same-executable dispatch is new work.
- **wgpu/winit are unconditional** in `horizon-client`, `horizon-client-render`,
  `horizon-client-lib` and `horizon-client-input`; `--no-default-features` removes only
  audio/controllers/extras. Upstream `docs/server.md:419-428` explicitly defers a true headless
  engine. The Android fork additionally gates the mod/WASM runtime behind `mods` (off by default),
  which is the pattern to adopt for the no-mods guarantee.
- The smallest portable crate set is `horizon-core`, `horizon-world`, `horizon-protocol`,
  `horizon-items`, `horizon-command`, `horizon-client-core`, `horizon-assets`, `horizon-threads`,
  `horizon-guest-stack`, `horizon-java-properties`, `horizon-server-files` and
  `horizon-server-launch`. `horizon-gameplay` still reaches `horizon-client-lib` (and therefore
  wgpu/winit), so extracting the dedicated loop is a large porting task, not a build flag.
- No pure-Rust generator exists for the prepared data tables; both trees rely on JDK exporters or
  a bundled `engine-data.zip`. The owner approved an interim deviation: link the upstream engine
  with graphics crates present, guarantee no GUI/window/audio/JNI initialization in headless mode,
  and accept a verified prepared-data folder. Both deviations stay visible in `PROGRESS.md`; a full
  headless extraction and pure-Rust generators remain mandatory for strict `TASK.md` completion.
- Build tooling pinned by the fork: Rust 1.95.0, NDK revision `28.2.13676358` (package **r28c**,
  verified — r28b is 28.1.13356709), minSdk 28, target `aarch64-linux-android`, clang/linker env
  vars recorded in local `DEV.md`. APK/Gradle/JNI paths are excluded for the server CLI.

## Target staging

Android ARM64 (`aarch64-linux-android`, Termux, real tablet) first; then remaining Android ABIs;
then Linux, Windows and macOS. Cross-compilation is never reported as runtime validation.

## Ownership model for parallel work

Integration lead owns this document, `TASK.md`, `AGENTS.md`, `PROGRESS.md`, `DEV.md` policy, the
final executable dispatch and shared CLI/engine protocol. Research agents are strictly read-only.
Implementation agents will receive exclusive file/path ownership before editing; no two agents edit
the same file concurrently, and their output is independently re-verified by the lead.
