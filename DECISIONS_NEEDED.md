# Decisions needed from the owner — RESOLVED 2026-11-24

Owner choice: **pragmatic interim** — option **B** for item 1 (graphics crates linked, no GUI
initialization in headless mode), option **B** for item 2 (verified prepared-data folder instead of
pure-Rust generators), and owner-run tablet validation for item 3. Both deviations stay documented
in `PROGRESS.md` and are not claimed as `TASK.md` compliance. The original options are preserved
below for traceability.

## 1. Headless engine dependency boundary

`TASK.md §2.1` requires the headless dependency graph to exclude `wgpu`, `winit`, audio, controllers,
JNI and client-mod runtimes. Upstream does not currently have such a build: those dependencies are
unconditional in `horizon-client`, `horizon-client-render`, `horizon-client-lib` and
`horizon-client-input`, and `docs/server.md:419-428` defers headless work. Removing them means a
multi-thousand-line extraction of the dedicated loop (97 files use `winit::keyboard::KeyCode`).

- **A (spec-faithful):** fund/authorise the full headless extraction refactor. No interim binary
  until it is complete.
- **B (interim deviation):** link the graphics crates but guarantee no renderer/window is
  initialised in headless mode, with a dependency audit and a documented deviation in `PROGRESS.md`.
  This violates the letter of §2.1 and needs explicit approval.
- **C:** owner proposes another boundary.

## 2. Prepared 26.3 data tables without Java

`TASK.md §2.3/§2.4` requires the binary to generate `facts.json`, `registries.json`, `commands.json`
and the rest in pure Rust at runtime. Neither reference tree has such a generator: the Android fork
extracts a JDK-produced `engine-data.zip`, and upstream `horizon-prepare` invokes Java exporters.

- **A (spec-faithful):** implement the Rust generators (large, separate effort).
- **B (interim deviation):** fetch a signed generated-table bundle from an owner-controlled
  endpoint, pinned and verified exactly like Mojang inputs; still no Java at runtime, but an extra
  Horizon-owned download. Needs approval and endpoint details.
- **C (interim deviation):** operator supplies a one-time prepared folder; binary verifies it and
  works offline thereafter. Needs approval and changes the first-run UX.

## 3. Tablet acceptance

No adb/device exists in this container and the owner confirmed tablet testing is theirs. The handoff
procedure is `TABLET_TEST_PLAN.md`. Once an integrated ARM64 build exists, return the evidence listed
there; until then M1.3–M1.8 remain unchecked. No other acceptance criterion will be weakened.

## 4. Build environment

The free GuppyNodes/Pterodactyl container (3 vCPU, 2.5 GiB RAM, 10 GiB disk) is not for heavy builds.
Compilation and release artifacts are produced by `.github/workflows/server-arm64.yml`; only light
module tests run locally. If you want a locally produced binary instead, provide a larger builder or
approve a one-off CI download of the artifact.
