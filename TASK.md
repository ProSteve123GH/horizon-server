# TASK: Build a Standalone, Vanilla-Compatible Horizon Dedicated Server

## 0. New repository, source repositories, and implementation priority

This product must be developed in a **new, separate repository**. Do not implement the standalone server inside `horizon-android-old`, `horizon-android-build`, or the upstream client repository. The new repository is intended to contain the standalone server product and its build/release infrastructure only.

- New repository name: `horizon-server` (under `ProSteve123GH`, unless the owner chooses another name).
- Default branch: **`server`**. Initialize the new repository directly with `server` as its initial/default branch; do not create or use `main` as the default branch.
- Android source fork: `https://github.com/ProSteve123GH/horizon-android-old`.
- Local Android source checkout: `/root/horizon-android-old`.
- Local Android build/tooling checkout: `/root/horizon-android-build`.
- Upstream reference: `https://github.com/horizon-rust/horizon`.

Treat the upstream `horizon-rust/horizon` repository as the canonical reference for current architecture, behavior, protocol implementation, vanilla parity, and upstream changes. Treat `horizon-android-old` as the reference for the Android port, Android-compatible Rust implementations, the 26.3 download/import flow, prepared data tables, and existing Android-specific work. Treat `horizon-android-build` as a local build/tooling/environment reference only. Do not assume these repositories are interchangeable. Inspect their current branches, commits, status, and relevant files before relying on them.

### Local environment and instructions

Before doing any implementation work, read and obey:

- `/root/horizon-android-old/AGENTS.md`
- `/root/horizon-android-old/LOCAL_ENVIRONMENT.md`

Also inspect relevant build scripts and documentation in `/root/horizon-android-build`. These files describe the available local tablet/build environment and constraints. The new repository must have its own concise `AGENTS.md` (or equivalent repository instructions) that points contributors to the product task and records the new repository's own rules. Do not copy machine-specific secrets or private values into tracked files.

Create a local-only `DEV.md` in the new repository for machine-specific paths, local checkout relationships, build commands, device details, and temporary development notes. Add `DEV.md` to `.gitignore` before creating it; verify it is ignored and never stage or commit it. The tracked `TASK.md` must contain the durable project requirements and explicitly point developers to `DEV.md` for local-only environment notes and to the two source instruction files above for inherited environment constraints. Do not put secrets, credentials, tokens, signing material, or private user data in either file.

### Initial implementation priority: Android ARM64

The first milestone is **Android ARM64 only**, built and tested locally on the available Android tablet. Prioritize a native Android ARM64 command-line executable that can run in the tablet's Termux environment. Make the smallest clean architecture choices that keep the other targets possible, but do not spend the first milestone implementing or debugging every OS/architecture combination.

The initial target is Rust `aarch64-linux-android` (Android ARM64 / `arm64-v8a`) and must be tested on the actual tablet, not merely cross-compiled. Use `/root/horizon-android-build` for its available local build tooling and `/root/horizon-android-old` for Android implementation reference. Record exact build commands, required NDK/toolchain details, runtime limitations, and test outcomes in local-only `DEV.md`.

After Android ARM64 is stable, expand to the remaining Android ABIs and then Linux, Windows, and macOS targets. Keep the full 13-target matrix as the eventual release goal, but make the roadmap explicitly staged:

1. Android ARM64: first working standalone executable, resource bootstrap, vanilla configuration/datapacks, real tablet validation.
2. Remaining Android ABIs: ARMv7, x86, x86_64.
3. Linux: x86, x86_64, ARMv7, ARM64.
4. Windows: x86, x86_64, ARM64.
5. macOS: x86_64, ARM64.

Do not claim a platform is supported until its executable has been built and run-tested on that platform or a clearly documented equivalent environment. Cross-compilation alone is not runtime validation.

### Repository initialization is a separate prerequisite

The initial repository-setup operation must create the new repository, initialize Git with branch `server`, add the supplied `TASK.md`, create `.gitignore`, create the ignored local `DEV.md`, and add a tracked repository `AGENTS.md` with contributor guidance. It may create a minimal README describing the project and staged roadmap. **Repository initialization must not begin implementing the server.** Do not modify engine code, add functional server implementation, port modules, or start the implementation milestones until the owner explicitly starts the task after reviewing the initialized repository.

---

## Mission

Implement a **standalone Horizon Dedicated Server** from the existing Horizon upstream codebase and this Android fork (`ProSteve123GH/horizon-android-old`). The result must be a native executable named `horizon-server` that an operator can place in an empty directory and run directly:

```sh
./horizon-server
```

On first launch, the executable performs first-run setup, obtains and verifies the required Minecraft 26.3 inputs directly from official Mojang endpoints, prepares the runtime data it needs using Horizon's existing Android-style data-table/resource pipeline, creates or opens a vanilla-compatible server directory, and starts the dedicated server. It must not require a separate Horizon launcher, desktop client installation, `horizon-engine`, `horizon-prepare`, Java/JDK, Python, Docker, or a second helper executable at runtime.

The server must behave as closely as Horizon's implemented Minecraft 26.3 engine permits to a **vanilla Minecraft Java Edition dedicated server**. It must support ordinary vanilla server administration, `server.properties`, standard world layout and saves, commands, and datapacks. **Do not add mod support.** Datapacks are vanilla content and are required; native mods, Horizon code mods, WASM mods, Fabric, Forge, NeoForge, Mixin, plugin loaders, and mod-loading APIs are explicitly out of scope for this server product.

The release target is one self-contained executable **per OS/architecture combination** (13 target combinations below). “One binary” means no separate engine, launcher, preparer, JVM, or sidecar executable. Runtime data downloaded from Mojang is stored outside the executable in the server data directory. Do not bundle Mojang game files or assets into releases.

---

## 1. Mandatory repository orientation

Before changing code in the new repository, read its own `AGENTS.md`, this `TASK.md`, and local-only `DEV.md` if available. Also read `/root/horizon-android-old/AGENTS.md` and `/root/horizon-android-old/LOCAL_ENVIRONMENT.md` in full, then inspect the upstream `horizon-rust/horizon` repository and the local `/root/horizon-android-build` checkout. Read the relevant design documents in full:

- `AGENTS.md`
- `CONTRIBUTING.md`
- `CONTRIBUTING_ANDROID.md`
- `docs/layers.md`
- `docs/server.md`
- `docs/hosting.md`
- `docs/api/server.md`
- `docs/android.md`
- `docs/android-game-files.md` (if present; otherwise locate the current resource-install documentation)
- `docs/datapack-upgrade.md`
- `research-lock.json`

Also inspect the current source, not just these instructions:

- `crates/horizon-server/` — current supervisor, CLI, API, configuration, resource discovery and engine process management.
- `crates/horizon-client/src/game/dedicated.rs` — actual dedicated game implementation.
- `crates/horizon-client/src/entry.rs` and `crates/horizon-client/src/main.rs` — current executable entry points and `--dedicated` dispatch.
- `crates/horizon-client/Cargo.toml` — current unconditional GUI/render/client dependencies and feature gates.
- `crates/horizon-android/` and `crates/horizon-client/src/android_main.rs` — Android-specific cdylib/JNI entry point; do not reuse this entry point for a CLI server.
- `crates/horizon-client/src/official_install.rs` — official Mojang metadata/download/hash validation.
- `crates/horizon-client/src/minecraft_import.rs` and `crates/horizon-client/src/android/minecraft_import.rs` — install/import/repair behavior.
- `crates/horizon-client/src/android_resources.rs` — existing Rust data-table reconstruction; identify which parts are platform-independent and which are Android-only.
- `crates/horizon-core/src/prepared_files.rs` — `horizon-data.bin` prepared resource container.
- `crates/horizon-server/src/engine.rs` — current child-engine lookup and prepared-resource checks.
- `tools/android_prepare.py`, `tools/android_build.py`, `.github/workflows/android-source.yml`, and `.github/workflows/android-dev.yml` — existing Android build and preparation pipeline.
- `tools/test_dedicated_server_parity.py` and other dedicated-server parity tests.
- Datapack implementation and tests in `crates/horizon-client/`, `crates/horizon-world/`, `crates/horizon-port-pack/`, `launcher/src/datapacks.rs`, and `tools/fixtures/datapacks/`.

The Android fork currently has a Rust download/install path that validates Minecraft 26.3 against the repository's pinned metadata/hashes, downloads the client JAR and asset objects, reconstructs generated world/data tables, and uses a prepared-data bundle. Reuse the proven pieces where appropriate. Do not copy Android UI, JNI, SAF, touch-control, Bedrock-control, or APK packaging concerns into the server.

### Repository constraints that remain mandatory

- Follow `AGENTS.md` exactly, including its testing, attribution, layer and repository hygiene rules.
- Never commit or publish Mojang JARs, assets, decompiled source, generated resource folders, `.local/`, personal game data, account data, worlds, tokens, signing keys, or other private build artifacts.
- Keep the engine/platform/Horizon dependency direction in `docs/layers.md`. Do not introduce a dependency from the vanilla engine into the Horizon layer.
- Preserve upstream attribution/history and the existing Android fork. Avoid unrelated refactors and avoid breaking the Android client.
- Inspect `git status` before work and preserve all pre-existing uncommitted changes. Do not reset, clean, force-push, or overwrite unrelated work.
- Do not invent test results. Separate “compiled”, “ran on target”, “connected with a vanilla client”, and “parity-tested” claims.
- Do not merge experimental branches or make unrelated upstream sync changes.

---

## 2. Product requirements

### 2.1 Single executable; no separate engine installation

The published artifact for a target is named `horizon-server` (`horizon-server.exe` on Windows). It contains both:

1. **Supervisor/server host:** first-run setup, configuration discovery, resource bootstrap, logs, CLI, shutdown/restart handling, backups, optional management API/panel where already supported.
2. **Headless game engine:** the existing dedicated Minecraft simulation, protocol/networking, world loading/saving, chunk generation, ticking, player handling and vanilla commands.

The supervisor and engine may run as separate OS processes for crash isolation, but they must be modes of the **same executable**. The supervisor should spawn its own `current_exe()` with a private, versioned internal argument or equivalent IPC handoff. The internal mode must never be presented as a second user-installed binary. Preserve the useful supervisor/engine isolation: a game-engine crash should not automatically destroy management state, logs or recovery capability.

Remove runtime dependence on looking up `horizon-engine` or `horizon-client` from beside the executable or from `PATH`. Do not ship either as a required second program. Any engine path option may remain only as a development/testing escape hatch; it must not be needed for normal releases and must not undermine the one-binary default.

No GUI/window/display initialization may occur in headless mode. The server binary must not initialize WGPU, Winit, audio, controllers, Android JNI, the desktop launcher, or client-only rendering paths. Do not merely pass `--no-default-features` and assume this is enough: audit the complete target dependency graph and split crates/modules/features as necessary.

Prefer extracting the common gameplay/server runtime into reusable engine crates or a carefully gated headless entry path rather than cloning or maintaining a second copy of the Minecraft server logic. The normal desktop and Android client must continue to work.

### 2.2 First-run behavior and immediate start

In an empty working directory, running `./horizon-server` must be sufficient to begin setup and start the server. A reasonable first-run sequence is:

1. Detect that this is a new server directory.
2. Explain the Minecraft EULA and ask for an explicit affirmative response in an interactive terminal, or require an explicit flag/environment setting for a non-interactive start. Never silently write `eula=true` or accept the EULA on the operator's behalf.
3. Create vanilla-style defaults and a default world directory/configuration.
4. Bootstrap and verify the minimum required Minecraft 26.3 game inputs and assets directly from Mojang.
5. Build/verify Horizon's prepared data tables using Rust code; do not invoke a JDK or Java exporter at runtime.
6. Start the dedicated server in the foreground with logs on stdout and in the standard log file.

Subsequent invocations in an initialized directory should start the server directly, without repeating setup or downloading already-verified data. If the resources are already present and valid, offline startup must work. If network access is unavailable and required files are absent, fail with a precise actionable error and retain safe, verified partial downloads for retry.

Support conventional command-line use as well, at minimum:

```text
horizon-server [options]                 Start in foreground (default)
horizon-server --help                    Show help without downloading resources
horizon-server --version                 Print server and Minecraft versions
horizon-server init                      Optional explicit first-run setup
horizon-server start                     Optional background start, if retained
horizon-server stop                      Graceful stop, if retained
horizon-server status                    Show status, if retained
horizon-server console                   Attach to the running server, if retained
horizon-server backup / backups / restore Optional management commands, if retained
```

Exact subcommand UX may follow the existing `crates/horizon-server` CLI, but the default invocation must be simple. Do not require an `init` command, separate preparation command, or user-supplied engine path just to run a fresh server.

### 2.3 Resource bootstrap: reuse the Android pipeline

Implement a server-specific resource bootstrapper inside the shipped executable. It should reuse and, where necessary, refactor the existing Android installation code into shared platform-independent Rust APIs. Do not create a second downloader with weaker validation.

Requirements:

- Pin the supported Minecraft game version to the repository's current engine version (currently 26.3 as recorded by the source/research lock). Do not silently float to a newer version.
- Use Mojang's official HTTPS metadata/download endpoints only. Maintain an explicit host allowlist equivalent to the existing installer.
- Validate official metadata and the expected client/server artifact hashes against the repository's pinned records in `research-lock.json` (or the designated canonical pin source). Validate downloaded content with the available authoritative hashes and sizes before accepting it.
- Choose the smallest legal and technically sufficient inputs. Compare the 26.3 server JAR and client JAR paths and document the choice. The Android workflow already demonstrates a client-JAR + asset-object installation path; it may be reused if the server JAR does not contain all required resources. The server must not execute either JAR.
- Download only the resources required by the headless engine. Do not fetch sounds, client-only textures/models, touch controls, Bedrock sample controls, launcher content or other irrelevant files unless the engine demonstrably reads them.
- Generate the required registries, tags, commands, worldgen tables and any other prepared inputs using the existing Android-style Rust table reconstruction where possible. Audit `android_resources.rs` carefully: reuse platform-independent reconstruction, but exclude Android-specific controls/paths.
- Preserve or generate `horizon-resources.json` and the correct `horizon-data.bin` format/version if the engine requires them. The manifest must identify the exact pinned game version and integrity of the prepared files.
- Store resources in a clearly defined server data location (for example `<server-dir>/.horizon/resources/26.3` or a documented `game-files/` layout). Do not place them in the executable directory if that would break read-only installs.
- Verify every cached file before reuse. Detect corrupt, incomplete, stale-version or mismatched resources and repair them safely.
- Download into temporary/partial files and atomically rename only after successful integrity checks. Protect against path traversal, unsafe archive paths, symlink redirection and oversized metadata/archives.
- Keep partial valid cache data after interrupted downloads; never replace a working installation with an incomplete preparation.
- Provide clear progress on stderr/stdout without corrupting the interactive server console. Provide `--quiet`/JSON progress only if it fits the existing CLI conventions.
- Support an explicit `--offline` mode or equivalent behavior when the resources are already cached.
- Do not distribute or bundle Mojang's JARs/assets/resources in Git, GitHub release artifacts, Docker images, tests, or generated source. The end user's executable downloads them directly from official sources.

The bootstrap must be shared by Linux, Windows, macOS and Android. Use OS-appropriate cache/data directories only where needed; the default server working directory should remain easy to understand and compatible with normal Minecraft server folders.

### 2.4 Vanilla-style server directory and configuration

Make the server directory familiar to Minecraft Java Edition operators. `server.properties` must be the canonical operator-facing source of vanilla gameplay/server settings, with defaults and accepted values matching the pinned Minecraft 26.3 server wherever implemented.

A new server directory should resemble a normal vanilla server folder and create/use the applicable files:

```text
server-directory/
├── horizon-server[.exe]       # optional when the operator keeps the binary here
├── eula.txt
├── server.properties
├── server-icon.png            # optional, user supplied
├── ops.json
├── whitelist.json
├── banned-players.json
├── banned-ips.json
├── usercache.json
├── logs/
│   └── latest.log
├── world/                     # or the configured level-name
│   ├── level.dat
│   ├── region/
│   ├── DIM-1/ / DIM1/          # actual format/layout used by this version
│   └── datapacks/
└── .horizon/                   # Horizon-owned cache, runtime metadata and internal state
```

Do not force all listed files to exist before they are needed if vanilla creates them lazily. Preserve the standard filenames, encodings, data formats and paths whenever applicable.

Requirements:

- Read and write `server.properties` directly using vanilla Java-properties syntax and behavior. Preserve unknown/unsupported properties where vanilla would preserve them, and do not silently discard user settings on load/store.
- Support the standard 26.3 server properties that the engine supports, with vanilla defaults. Audit every property from the pinned server version; document any intentionally unsupported property and fail clearly rather than silently pretending it works.
- Cover at least the normal operator-facing settings: `motd`, `server-port`, `server-ip`, `max-players`, `online-mode`, `white-list`, `enforce-whitelist`, `view-distance`, `simulation-distance`, `level-name`, `level-seed`, `gamemode`, `difficulty`, `hardcore`, `allow-nether`, `spawn-protection`, `max-world-size`, `generate-structures`, `pvp`, `allow-flight`, `player-idle-timeout`, `pause-when-empty-seconds`, `max-tick-time`, `network-compression-threshold`, `enable-command-block`, `function-permission-level`, `op-permission-level`, `force-gamemode`, `prevent-proxy-connections`, `enable-status`, `enable-query`, `enable-rcon`, `broadcast-console-to-ops`, `broadcast-rcon-to-ops`, `hide-online-players`, `initial-enabled-packs`, `initial-disabled-packs`, `accepts-transfers`, `resource-pack` fields, `require-resource-pack`, `enforce-secure-profile`, `server-name` / management protocol keys where supported, and other keys present in the actual 26.3 server implementation. Confirm every key against the pinned 26.3 server; do not assume an option exists just because it appeared in another Minecraft release.
- `server.properties` changes should take effect at the same lifecycle point as vanilla (normally next restart unless the property is explicitly live-reloadable). Do not create conflicting duplicated values in mandatory `server.toml` gameplay sections.
- The existing `server.toml` may be retained for Horizon-only supervisor features such as backups, update policy, API/panel binding, crash restart and management integration. It must be optional and must not be required for a vanilla-like server to start. Its precedence relative to `server.properties` must be explicit and must never silently overwrite operator edits to vanilla properties. Prefer keeping Horizon-specific values in a separate namespaced file/section.
- Existing users with a `server.toml` server directory must not lose worlds, operator lists or settings. Provide a safe migration/compatibility path, preserve the original files, and test it.
- Support existing vanilla server folders: when pointed at a folder with `server.properties`, `eula.txt`, a world and player lists, use them in place rather than requiring an import/export conversion.
- Support `--dir <path>` and/or a documented server-directory environment variable. The default should be the current working directory, like vanilla's server JAR.

### 2.5 Vanilla world and player administration

Preserve and complete the current dedicated implementation so vanilla clients can connect and play without needing Horizon clients.

Required behavior includes:

- Standard level/world naming, seed handling, world creation and reopening; never overwrite an existing world as a side effect of first-run setup.
- Vanilla-compatible `level.dat`, region/chunk storage and dimension layout.
- Normal Overworld, Nether and End operation, dimension travel and player visibility/state transitions.
- 20 TPS target, autosave and clean save/stop behavior. Respect configured view and simulation distances and other relevant vanilla tick/world settings.
- Status ping, legacy ping where applicable, protocol/version handshake, compression, login/configuration/play states, secure chat, online-mode account/session verification and offline-mode behavior consistent with vanilla 26.3.
- Standard player capacity, MOTD, server icon, whitelist and ban checks, operator permissions and server list/status behavior.
- Standard player files and data: `ops.json`, `whitelist.json`, `banned-players.json`, `banned-ips.json`, `usercache.json`; preserve vanilla JSON structure, UUID/name semantics and persistence.
- Standard console input/output and ordinary vanilla server commands including (at minimum) `stop`, `save-all`, `save-on`, `save-off`, `list`, `op`, `deop`, `whitelist`, `ban`, `ban-ip`, `pardon`, `pardon-ip`, `banlist`, `kick`, `say`, `help`, and all implemented vanilla Brigadier commands.
- Correct permission levels, command feedback, command source behavior and operator broadcast settings.
- Graceful Ctrl-C/SIGINT/SIGTERM handling where available. Save player/world state and close sockets/files cleanly.
- Proper logs to stdout/stderr and `logs/latest.log`; keep crash reports/diagnostics in the server directory. Do not log tokens or private session credentials.
- Vanilla management protocols/features that already exist in the engine (RCON, GS4 query, vanilla management WebSocket/HTTP, etc.) must be preserved where configured and tested. Do not advertise a protocol as supported until it is implemented and validated.

Run and extend the existing dedicated-server parity harnesses against the exact pinned vanilla 26.3 server. Use fresh isolated test directories and synthetic/test accounts; do not touch personal worlds or real player data.

### 2.6 Vanilla datapack support — required, not optional

Datapacks are an essential vanilla server feature and are **not** considered mods. Support the standard Minecraft Java datapack location and lifecycle:

```text
<world-folder>/datapacks/
```

Required behavior:

- Discover datapacks in the world `datapacks/` directory using vanilla-compatible directory-pack and `.zip`-pack conventions for Minecraft 26.3.
- Validate `pack.mcmeta`, pack format/minimum format metadata as applicable to 26.3, archive paths, namespaces and required directory structure. Give useful errors for invalid or incompatible packs.
- Respect vanilla `level.dat` `DataPacks` enabled/disabled lists and pack ordering/priority. Persist enable/disable decisions across restart.
- Load datapacks before world creation/worldgen when they affect world generation, and load/reload them at the correct vanilla lifecycle stage.
- Support `/datapack list`, `/datapack enable`, `/datapack disable`, and `/reload` with vanilla-compatible command parsing, feedback, failure handling and permissions to the extent implemented by vanilla 26.3.
- Ensure datapack reload is atomic/safe: a failed reload must not leave the running server in a partially replaced registry/function/recipe/tag state. Match vanilla behavior where it is observable.
- Support datapack content types implemented by vanilla 26.3 and supported by the Horizon engine, including relevant tags, recipes, loot tables, advancements, functions, predicates/item modifiers and world-generation data (biomes, noise, density functions, configured/placed features, structures, dimensions and dimension types). Do not claim support for a content type until it is actually parsed, registered and used correctly.
- Inventory every vanilla 26.3 datapack registry/content type. Compare that inventory against Horizon's current loaders and tests. Implement missing server-relevant behavior required for practical vanilla datapacks, or clearly record a narrowly scoped limitation with a regression test and a follow-up issue.
- Test both a directory datapack and a ZIP datapack, multiple datapacks with priority conflicts, enabled/disabled persistence, invalid/mismatched `pack.mcmeta`, `/reload`, and a datapack that changes world-generation data on a newly generated world.
- Test with ordinary datapacks made for the exact 26.3 pack format, not Horizon-specific mod packages.

Do not confuse datapacks with resource packs. A resource pack is client-side presentation; the server may offer/require one using standard `server.properties` settings, but must not require clients to install Horizon. Datapack behavior must be exercised with a vanilla Java client and the pinned vanilla server as parity reference.

### 2.7 Explicitly no mods or mod loaders

The dedicated server release must not expose or load:

- Fabric, Forge, NeoForge, Quilt, Mixin or Bukkit/Paper/Spigot plugin systems;
- Horizon native code mods, `.hmod` runtime, Rust dynamic libraries or downloaded executable extensions;
- Horizon WASM/component mods or mod registries/runtime;
- modpack import, mod marketplace, launcher integration or code-mod development commands.

Disable these features in the server build at the feature/dependency level, not merely through a UI switch. Do not remove mod support from the existing client or unrelated packages. Vanilla datapacks, vanilla commands, server resource-pack configuration and ordinary datapack ZIPs must remain supported.

If a dependency is pulled into the headless build only for a client/mod feature, remove it from that build graph. Keep the server's built-in security/anticheat or supervisor features only where they already exist and do not alter vanilla gameplay by default; document Horizon-only behavior separately.

---

## 3. Build and release target matrix

Produce one `horizon-server` executable for every target below. Keep per-target build flags/toolchains explicit and reproducible.

| OS | Architecture | Rust target triple | Notes |
|---|---|---|---|
| Linux | x86 (32-bit) | `i686-unknown-linux-musl` preferred; otherwise documented glibc target | Prefer a portable musl build if all dependencies support it. |
| Linux | x86_64 | `x86_64-unknown-linux-musl` preferred | Primary Linux release target. |
| Linux | ARM32 | `armv7-unknown-linux-musleabihf` preferred; otherwise documented hard-float glibc target | Must not assume AArch64. |
| Linux | ARM64 | `aarch64-unknown-linux-musl` preferred | Include common ARM64 Linux VPS/single-board targets. |
| Android | x86 | `i686-linux-android` | Standalone CLI executable; no APK/JNI entry point. |
| Android | x86_64 | `x86_64-linux-android` | Standalone CLI executable; no APK/JNI entry point. |
| Android | ARM32 | `armv7-linux-androideabi` | Respect a documented minimum API level and ABI. |
| Android | ARM64 | `aarch64-linux-android` | Primary Android/Termux target. |
| macOS | x86_64 | `x86_64-apple-darwin` | Intel Macs. |
| macOS | ARM64 | `aarch64-apple-darwin` | Apple Silicon. |
| Windows 10/11 | x86 | `i686-pc-windows-msvc` | 32-bit executable on supported Windows versions. |
| Windows 10/11 | x86_64 | `x86_64-pc-windows-msvc` | Primary Windows target. |
| Windows 10/11 | ARM64 | `aarch64-pc-windows-msvc` | Native Windows on ARM, not x64 emulation only. |

Notes:

- These are 13 separate target artifacts, not a universal cross-architecture binary.
- “Static binary” must be interpreted per OS. Prefer fully static musl for Linux when feasible. Android uses its platform/Bionic ABI; Windows and macOS may use their standard OS runtime/system libraries. Do not claim literal static linking where the platform does not support it or where system frameworks are required. The product requirement is one self-contained executable file with no Horizon sidecar executables or language runtime.
- Do not bundle native DLLs, dylibs or `.so` files just to make the server start if they can be eliminated by compiling a headless dependency graph. If an OS library is genuinely required, document it, minimize it and add an automated dependency audit.
- Android must use NDK cross-compilation appropriate to each ABI, but must not build an Android APK. Confirm the resulting ELF executable can be invoked from Termux or a normal Android shell with the documented storage/permission caveats.
- Linux ARM32 and x86-32 targets may expose dependency or upstream architecture limitations. Investigate them rather than silently dropping them. If a target is genuinely impossible with the present dependency set, report the exact blocker and a smallest actionable remediation; do not mark it complete based only on a successful cargo check.
- Build macOS artifacts on supported macOS runners or through a verified cross-build setup. Do not assume Linux CI can produce a release-ready macOS binary without testing it.

### 3.1 CI expectations

Add a dedicated server workflow rather than overloading the Android APK workflow. It should:

- Build the standalone `horizon-server` binary for all 13 targets.
- Use locked toolchain/dependency versions and reproducible build inputs.
- Cache Cargo dependencies/targets safely without caching or publishing Mojang game content as a release artifact.
- Run formatting, linting, unit tests and server parity checks once per appropriate host/target grouping.
- Run target-native `--help`/`--version` smoke tests where runners are available. For cross-compiled targets that cannot be executed on the runner, run an appropriate emulator/VM smoke test where practical and clearly mark build-only validation.
- Audit artifacts to ensure there is exactly one server executable per target and no JAR, asset archive, client binary, engine binary, launcher, helper executable, debug symbols or unrelated payload accidentally shipped. Debug symbols may be retained as separate non-release CI artifacts only if the workflow requires them.
- Publish checksums and release metadata. Do not publish Mojang-derived game assets or prepared game directories.
- Keep signing/release secrets out of untrusted PR workflows.

---

## 4. Proposed architecture and implementation guidance

The exact crate split is for the architecture lead to decide after auditing current dependency boundaries. The final design must satisfy the behavior below.

### 4.1 Headless engine boundary

- Separate server/gameplay runtime from client rendering/window/input initialization.
- Reuse the same `Game`/world/network/gameplay implementation used by the current dedicated server.
- Keep the server path free of `wgpu`, `winit`, `naga` renderer backends, audio, controllers, JNI and desktop UI dependencies wherever possible.
- Compile only required server-side registries, world data, protocol, command, authentication, persistence and data-pack loaders.
- Avoid duplicating code or drifting gameplay behavior between the desktop client's integrated server and dedicated server.
- Add a single headless entry point callable from the shared executable's internal engine mode.

### 4.2 Self-spawn and process lifecycle

- Supervisor resolves `std::env::current_exe()` and starts the same executable in a hidden internal-engine mode.
- Pass arguments/data using a stable internal contract. Avoid leaking secrets in command-line arguments; use inherited pipes, a private local socket or protected temporary file for sensitive tokens/config where appropriate.
- Keep stdin/stdout/stderr semantics explicit: server console commands reach the engine; logs are forwarded to stdout and the standard log file; Ctrl-C and termination are propagated once and gracefully.
- Avoid recursive supervisor startup loops. The internal-engine mode must be detected before normal setup/bootstrap/CLI dispatch.
- Validate child startup and detect early exit. Keep crash diagnostics and restart/backoff behavior if the existing supervisor provides it.
- On Windows, use correct process creation, console control and graceful termination semantics. On Unix-like systems, handle signals and process groups deliberately.

### 4.3 Configuration ownership

- `server.properties` is the vanilla gameplay contract and is read/written by the game as vanilla would.
- `eula.txt` remains an explicit operator acknowledgement.
- Horizon-only supervisor config stays separate and optional. The current `server.toml` can be migrated/retained for management-only settings with a clear compatibility layer.
- Avoid two sources of truth. In particular, a `server.toml` generated value must not overwrite a manually edited `server.properties` unexpectedly on every startup.
- Document standard file names, supported property values/defaults, restart-required settings, and any Horizon-only options separately.

### 4.4 Resource bootstrap crate/API

- Refactor the existing official installer/downloader and Android table reconstruction into reusable shared APIs rather than copying implementations.
- Use a small, testable server resource manifest that identifies each required file, source endpoint, pinned checksum/size and destination.
- Keep Mojang download logic separate from the game engine's resource readers.
- Reuse the verified cache and atomic-finalization behavior.
- Ensure the headless engine can run with its own minimum server resource set, without requiring a client install or client asset folders it never reads.
- Audit and document all files opened at server startup and during gameplay before deciding that a resource is optional.

---

## 5. Multi-agent execution plan

Use multiple subagents to parallelize independent investigations and implementation. The coordinator/integration agent owns architecture decisions, shared APIs, final integration and acceptance. Agents must communicate before modifying shared files.

### Agent 0 — Coordinator / architecture lead (required)

Owns:

- Repository audit and current-worktree safety.
- Reading `AGENTS.md` and required design docs.
- Defining the crate/feature boundaries and internal self-spawn protocol.
- Maintaining an interface/ownership matrix so agents do not edit the same central files concurrently.
- Reviewing each agent's findings and integrating changes.
- Ensuring no client/Android regressions and that all requested requirements are actually covered.

Must produce a short architecture note before large implementation work: current dependency graph, proposed binary modes, config ownership, resource bootstrap flow, datapack flow, and target matrix.

### Agent 1 — Headless engine / crate separation

Owns the engine dependency audit and headless compilation path. Investigate unconditional dependencies in `horizon-client`, gameplay entry points, and server-only feature gates. Produce a headless executable mode that reuses existing dedicated gameplay and excludes rendering/window/audio/controller/JNI/launcher/mod-runtime dependencies. Coordinate all `Cargo.toml` and shared entry-point changes with Agent 0 before editing.

### Agent 2 — Mojang bootstrap / resource preparation

Owns the official-download and prepared-resource pipeline. Reuse `official_install.rs`, the relevant parts of `minecraft_import.rs`, `android_resources.rs`, `prepared_files.rs`, and the `research-lock.json` pins. Determine whether `server.jar` or `client.jar` is the smallest sufficient source, download only the server-required assets, generate the prepared tables in pure Rust, and store them safely outside the executable. No Mojang files may be committed or bundled. Coordinate shared crate/API changes with Agent 0.

### Agent 3 — Vanilla configuration, world and datapacks

Owns the vanilla compatibility audit. Compare Horizon's dedicated behavior to the pinned 26.3 server for `server.properties`, EULA, player list files, world layout, console commands, datapack discovery/priority/reload/commands and worldgen loading. Extend the existing engine and parity harnesses. Do not add mod support. Report every datapack content registry and its current support status.

### Agent 4 — Single-binary supervisor and CLI

Owns `crates/horizon-server` supervisor/CLI integration. Replace the separate-engine lookup with same-executable internal engine mode, implement default first-run bootstrap and foreground start, preserve existing management functionality where practical, and ensure restart/crash lifecycle works. Coordinate entry-point changes and the internal command/IPC contract with Agent 0 and Agent 1.

### Agent 5 — Cross-platform / Android / release engineering

Owns the target matrix, toolchains, platform-specific startup and release workflow. Build the CLI binary—not an APK—for Android. Add Linux/macOS/Windows/Android target jobs, smoke tests, dependency/artifact audits, checksums and release packaging. Do not change core game behavior without Agent 0 approval.

### Agent 6 — Integration testing / documentation (can be folded into Agent 0 if subagents are limited)

Owns end-to-end test design, vanilla parity scripts, a clean-room startup fixture, datapack fixtures, migration tests and user-facing documentation. Confirm the exact binary can bootstrap a fresh server directory and accept a connection from an ordinary vanilla Java client.

### Parallel-work rules

- Agents may investigate in parallel immediately.
- Before implementation, Agent 0 assigns exclusive file/path ownership and publishes shared API contracts.
- Only Agent 0 owns top-level `Cargo.toml`, central crate feature graphs, the final executable dispatch and shared CLI/engine protocol unless ownership is explicitly delegated.
- Agent 2 owns resource-installer implementation files; Agent 3 owns datapack/server compatibility files; Agent 5 owns CI/packaging files. Shared files require an explicit handoff, not simultaneous edits.
- Keep patches focused and small. Avoid multiple agents refactoring the same large gameplay file.
- Each agent must report changed files, API assumptions, commands run, test results and unresolved blockers to the coordinator.
- The coordinator must review for duplicated functionality, overlapping changes, layer violations, added dependencies, unsupported claims and Android/desktop regressions before integration.

---

## 6. Required testing and validation

### 6.1 Unit/integration tests

Add or extend tests for:

- Same-executable supervisor-to-engine dispatch, including prevention of recursive dispatch.
- `--help` and `--version` working without network access or resource downloads.
- First-run EULA flow: decline blocks startup; affirmative interactive/explicit acceptance writes `eula=true`; non-interactive mode never silently accepts.
- Fresh resource bootstrap from official metadata; correct version/hash validation; invalid hash rejection; invalid URL/host rejection; truncated download retry; corrupt cache repair; interrupted download recovery; atomic prepared-resource replacement; offline cache reuse.
- Minimal server resource completeness: all files actually read during server startup are present; client-only files are not required.
- `server.properties` parsing/writing/defaults/unknown properties, existing folder import, settings preservation and migration from existing `server.toml` installations.
- World create/open/save/restart, seed persistence, dimensions and graceful shutdown.
- Player lists, whitelist, bans, operators, permissions and command execution.
- Directory and ZIP datapacks, pack metadata validation, enabled/disabled state, priority order, invalid archive safety, commands, reload and world-generation integration.
- No mod runtime/loader is activated in the server build.
- CLI behavior in non-interactive/CI environments.

Tests must use synthetic fixtures and isolated temporary directories. Never point a test at the user's real game folder, world, account data or server directory.

### 6.2 Vanilla parity

Use the pinned vanilla Minecraft 26.3 server as the behavioral reference. Extend/reuse existing scripts such as:

- `tools/test_dedicated_server_parity.py`
- `tools/test_user_cache_parity.py`
- `tools/test_server_dimension_players_parity.py`
- `tools/test_folder_datapack_metadata_parity.py`
- `tools/test_datapack_worldgen_parity.py`
- Existing command, networking, online-mode, secure-chat, worldgen and datapack parity suites.

At minimum, demonstrate:

1. A vanilla client can query server status and join.
2. A vanilla client can survive movement, chunk loading, save/restart and dimension travel.
3. Online-mode authentication and secure chat behave correctly for the supported 26.3 flow; offline mode behaves as documented.
4. `server.properties` settings change the same observable behavior as vanilla.
5. A directory datapack and ZIP datapack load, are listed, can be enabled/disabled, survive restart, and reload.
6. A datapack affecting world generation changes a newly created world as expected.
7. Standard operator/whitelist/ban commands and player JSON files behave like vanilla.

Do not use an Horizon-only client as the sole compatibility test. At least one complete integration scenario must use an unmodified vanilla Minecraft Java client of the pinned version.

### 6.3 Target validation

For every one of the 13 targets, record:

- Rust target triple and linker/toolchain versions.
- Build command and whether it completed.
- Output file type/architecture (`file`, `readelf`, `otool`, `dumpbin` or platform equivalent).
- Dynamic dependency audit result.
- `--help` and `--version` smoke-test result, where executable on available runners/emulators.
- Actual first-run/start/stop smoke-test result on native hardware/VM/emulator where practical.

A successful `cargo check` is not a release build. A successful cross-compile is not a runtime test. Clearly label each level.

---

## 7. Documentation and release deliverables

Update/add documentation for:

- Installation and the one-executable model.
- First startup, the EULA flow, automatic resource bootstrap and offline behavior.
- Supported Minecraft version and official resource download/hash verification.
- Vanilla `server.properties` and supported 26.3 properties/defaults.
- EULA, world directory layout, datapacks and `/reload` workflow.
- Standard console commands, player-list files, online/offline mode and secure chat.
- Optional Horizon-only management settings and how they differ from vanilla settings.
- OS/architecture support, Android/Termux usage and platform-specific caveats.
- Backups, logs, graceful shutdown and troubleshooting.
- Clear statement: no Java/JDK, launcher, separate engine, mod loaders, plugin system or code-mod runtime is included or required.
- Clear statement: Mojang files are downloaded directly by each operator and are not distributed by Horizon.

Release deliverables:

- One `horizon-server` executable per supported target.
- SHA-256 checksum manifest and release notes listing exact supported target combinations.
- CI logs or a validation report showing build, architecture inspection, dependency audit and smoke/parity coverage.
- No Mojang JARs/assets/prepared resources or extra Horizon executables in release archives.

---

## 8. Definition of done

The task is complete only when all applicable points below are satisfied:

- [ ] A clean server directory can be initialized and started by running `./horizon-server` (or `horizon-server.exe`) alone.
- [ ] No separate `horizon-engine`, `horizon-client`, `horizon-prepare`, launcher, Java/JDK, Python or helper executable is needed at runtime.
- [ ] The supervisor and headless engine are modes of the same executable.
- [ ] The server downloads/verifies the pinned 26.3 resources itself from official Mojang endpoints and can reuse its local cache.
- [ ] The runtime bootstrap uses Rust and does not invoke Java or a desktop launcher.
- [ ] Mojang JARs, assets and prepared game resources are not committed or distributed in release artifacts.
- [ ] Headless server dependency graph excludes GUI/render/window/audio/controller/Android-JNI/client-mod runtime requirements.
- [ ] `eula.txt` requires explicit operator agreement.
- [ ] `server.properties` provides the vanilla-style gameplay configuration and existing vanilla server directories remain usable.
- [ ] Standard worlds, player files, commands, logs, save/restart and networking are verified against vanilla 26.3 to the extent the engine supports them.
- [ ] Vanilla datapacks work from `<world>/datapacks` in directory and ZIP form, including metadata, priorities, enabled/disabled state, commands/reload and supported content registries.
- [ ] No mod/plugin/mod-loader/code-mod support is included in the server binary.
- [ ] Linux x86/x86_64/ARM32/ARM64, Android x86/x86_64/ARM32/ARM64, macOS x86_64/ARM64 and Windows x86/x86_64/ARM64 build jobs exist.
- [ ] Every claimed target has a recorded successful build and architecture audit; runtime validation levels are reported honestly.
- [ ] Vanilla client connectivity and datapack behavior have been exercised in an integration test.
- [ ] The Android client and existing desktop client build/behavior have not regressed.
- [ ] Documentation, checksums, test report and release artifact audit are complete.

If a target or vanilla feature is blocked by a concrete upstream/dependency limitation, do not silently mark the task done. Record the exact reason, evidence, impact and smallest viable next step, and keep the remaining deliverables working.

---

## 9. Final report format

When implementation is complete, report:

1. **Architecture:** how the single executable dispatches supervisor and engine modes; the resulting crate/feature boundary.
2. **Bootstrap:** which Mojang inputs are downloaded, how they are pinned/verified, which resources are prepared, and where cached files live.
3. **Vanilla compatibility:** `server.properties` coverage, world/player files, commands, networking and datapack content types supported.
4. **No-mod guarantee:** which mod/plugin runtimes and dependencies are excluded from the server build.
5. **Target matrix:** all 13 targets, their exact build status, architecture audit and runtime validation level.
6. **Tests:** exact commands run and pass/fail results, including vanilla-client integration and datapack parity.
7. **Artifacts:** exact output paths and sizes for each executable, checksum manifest and confirmation that no Mojang content or extra binaries were packaged.
8. **Limitations:** anything incomplete, unsupported or only cross-compiled.

Do not say “all platforms supported” unless every listed architecture has a successful binary and the level of validation is stated. Do not say “vanilla compatible” without specifying which parity suites and real-client scenarios passed.
