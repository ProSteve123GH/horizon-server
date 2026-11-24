# Horizon Server

**A standalone, vanilla-compatible Minecraft Java Edition 26.3 dedicated server built from the Horizon engine.**

Horizon Server is a separate server product designed to run from a single native executable. The long-term goal is to let an operator place `horizon-server` in an empty directory, complete first-run setup, and launch a vanilla-style dedicated server without installing a separate Horizon client, launcher, JDK, Python runtime, or helper executable.

**First release:** [`v0.0.1`](https://github.com/ProSteve123GH/horizon-server/releases/tag/v0.0.1) — Android ARM64 executable plus a verified `horizon-prepared-26.3.zip`. Download both assets from the release page; no Mojang files are bundled.

> [!WARNING]
> **This project is still in active development and is not ready for production worlds.** Milestone one has now been run on a real ARM64 tablet — the server started, loaded a world, accepted a vanilla 26.3 client and shut down cleanly. Several acceptance items remain unverified (fresh EULA refusal, datapack reload, dimension travel with a client, offline cache reuse). Check [`PROGRESS.md`](PROGRESS.md) for exact evidence before using real worlds.

## Project goals

- **One executable per platform target.** The supervisor and dedicated game engine are intended to run from the same `horizon-server` executable (`horizon-server.exe` on Windows).
- **Vanilla compatibility.** Target Minecraft Java Edition 26.3 behavior, including ordinary server configuration, worlds, saves, commands, player administration, networking, and datapacks.
- **No Java runtime requirement.** The server should not require a JDK, a separate launcher/preparer, Docker, Python, or a second engine executable at runtime.
- **Verified runtime resources.** Required Mojang inputs are obtained from official endpoints, checked against pinned metadata, and stored outside the executable. Mojang game files are not bundled in releases.
- **No mod loader or plugin runtime.** Fabric, Forge, NeoForge, Quilt, Mixin, native mods, Horizon code mods, WASM mods, and server plugins are out of scope. Vanilla datapacks are in scope.

These are product goals, not a claim that every feature is already implemented. See [`TASK.md`](TASK.md) for the detailed requirements and definition of done.

## Current development status

The first implementation milestone targets Android ARM64. The repository currently contains work on:

- CLI help/version handling and first-run setup scaffolding;
- explicit EULA handling and vanilla-style `server.properties` / server-directory setup;
- pinned Minecraft 26.3 resource metadata and download/cache verification;
- validation and acceptance of a prepared-data folder or bundle (the current interim data-preparation path);
- vanilla datapack discovery;
- a supervisor that re-launches the same executable through a private internal engine entry point;
- host-side tests and an Android ARM64 CI cross-build/package workflow.

**Demonstrated on the tablet:** pinned 26.3 bootstrap verification, resource loading, world creation/loading, all three dimensions saved on `stop`, console commands, a graceful shutdown, and a connection from an unmodified Minecraft Java 26.3 client that joined and moved.

**Still not demonstrated:** a fresh first-run EULA refusal, `--offline` cache reuse, datapack enable/disable persistence and `/reload`, dimension travel with a connected client, and full vanilla command parity. Cross-compilation and CI smoke tests are not substitutes for these device checks.

For the exact checklist and supporting evidence, see [`PROGRESS.md`](PROGRESS.md).

## Platform targets and test environments

The eventual release matrix contains 13 OS/architecture combinations. Development is staged, but Windows and macOS CI work is intended to begin early rather than wait for access to physical machines. Hosted CI can provide native Windows and macOS build/smoke-test environments; community testing is still valuable for real-world hardware and operating-system coverage.

| Platform target | Intended test environment | Current status |
|---|---|---|
| **Android ARM64** (`aarch64-linux-android`) | Termux on an ARM64 Android device | **First milestone, runtime validated.** Server started, saved worlds, accepted a vanilla 26.3 client, shut down cleanly. Some parity checks remain; see `PROGRESS.md`. |
| **Linux x86_64** | An unprivileged x86_64 Linux proot/container environment (not Android), or a Debian 12 Docker container that can run ordinary Linux binaries | Near-term target. Build and runtime validation are not yet claimed. The test environment should not assume root access. |
| **Linux ARM64** | ARM64 GNU/Linux userspace, including Linux distributions running under proot where applicable | Planned target. This is a GNU/Linux binary and is distinct from the Android/Termux executable. Runtime validation is pending. |
| **Windows x86 / x86_64 / ARM64** | Windows CI runners, followed by community or other native runtime testing | Planned for early CI enablement. No Windows runtime support is claimed yet. |
| **macOS x86_64 / ARM64** | macOS CI runners, followed by community or other native runtime testing | Planned for early CI enablement. No macOS runtime support is claimed yet. |
| **Remaining Android / Linux architectures** (Android ARMv7 / x86 / x86_64; Linux x86 / ARMv7) | Suitable native devices, VMs, or hosted runners | Part of the eventual 13-target matrix; staged after the initial targets. |

### Important: Android and Linux binaries are different

An Android ARM64 build targets Android's native userspace and is intended to run directly in **Termux**. A Linux ARM64 build targets a GNU/Linux userspace and is intended to run inside a Linux distribution or container. They are separate targets and are not interchangeable, even when the underlying device CPU is the same.

Likewise, Linux x86_64 testing in a non-Android proot/container environment does not establish support for every Linux distribution. Debian 12 is the initial reference environment; root privileges should not be required for ordinary server operation.

**A successful cross-compile is not platform support.** A target is only considered supported after its artifact has been run and validated in the corresponding environment. Windows/macOS builds may be added to CI before local hardware is available, but their status will remain clearly labelled until runtime checks exist.

## CI and building

The current workflow is [`server-arm64.yml`](.github/workflows/server-arm64.yml). It uses:

- Rust `1.95.0`;
- Android NDK `r28c` (`28.2.13676358`);
- Rust target `aarch64-linux-android`;
- locked host tests, an engine-enabled host smoke test, an ARM64 cross-build, and release-artifact audits.

The Android workflow currently produces an Actions artifact containing the packaged ARM64 executable and checksum. It is not a declaration of a stable release, and it does not replace runtime acceptance testing.

For local machine-specific paths and commands, use the ignored, local-only `DEV.md` if you have one. Do not commit that file. Repository-wide contributor rules are in [`AGENTS.md`](AGENTS.md).

## Intended command-line interface

The CLI is evolving. The current help output includes the following commands and options:

```text
horizon-server [options]       Start in the foreground (default)
horizon-server init             Run first-run setup
horizon-server bootstrap        Verify/fetch pinned game resources
horizon-server prepare          Accept a verified prepared-data folder or bundle
horizon-server datapacks         List valid vanilla datapacks
horizon-server --help            Show help
horizon-server --version         Show server and Minecraft versions
```

Common options include:

```text
--dir <path>                    Server directory (defaults to current directory)
--accept-eula                   Explicitly accept the EULA during non-interactive setup
--offline                       Require already-verified cached resources
--prepared-folder <path>        Use a verified prepared-data folder
--prepared-bundle <path>        Use a verified prepared-data ZIP bundle
```

The interface and behavior may change before the first usable release. In particular, do not assume that the default start command currently creates and runs a fully playable server.

## Source references

This repository is the standalone server product. The following projects are references, not places where this server is implemented:

- [horizon-rust/horizon](https://github.com/horizon-rust/horizon) — canonical upstream reference for engine architecture, protocol behavior, version pins, and vanilla parity.
- [ProSteve123GH/horizon-android-old](https://github.com/ProSteve123GH/horizon-android-old) — Android implementation and resource/bootstrap reference.

The server remains a separate repository so its executable, headless integration, target builds, and release pipeline can evolve independently.

## Contributing and reporting results

Contributions, issue reports, and platform test results are welcome. When reporting a test, include the OS, architecture, environment (for example Termux, proot, or Debian 12), exact command, exit code, and relevant output.

Please keep these distinctions explicit:

1. **Compiled** — the target artifact was produced.
2. **Started** — the executable ran on the target environment.
3. **Client-tested** — an unmodified Minecraft Java 26.3 client connected successfully.
4. **Parity-tested** — the behavior was checked against the pinned vanilla behavior.

Do not report a higher validation level based only on a lower one. Do not commit Mojang JARs/assets, generated game resources, prepared-data bundles, player/world data, credentials, or private build artifacts. Preserve upstream attribution and follow [`AGENTS.md`](AGENTS.md).

## Project documents

- [`TASK.md`](TASK.md) — product specification, target matrix, and definition of done.
- [`PROGRESS.md`](PROGRESS.md) — current implementation checklist, evidence, and blockers.
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — architecture and integration notes.
- [`AGENTS.md`](AGENTS.md) — repository contribution and safety rules.
