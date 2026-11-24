# `engine-link` — feasibility probe (no commits, no root/workspace changes)

Question: can a standalone binary call into the **dedicated engine** directly, without
`horizon-game::main()` and therefore without `horizon-mod-host` / wasmtime, which TASK.md forbids?

This directory is self-contained: its own `Cargo.toml`, `src/main.rs`, lockfile and `target/`.
It is **not** a member of the root server workspace and does not modify any repo files outside
`experiments/**`.

## What the vendored upstream actually does

- `vendor/horizon/crates/horizon-game/src/lib.rs` defines `main()` as:
  `horizon_guest_stack::prepare_main(); horizon_mod_host::exit_if_compile_child();
   let startup = horizon_client::entry::prepare(); ... horizon_mod_host::main_with(..., startup);`
  The mod host call is unconditional, so using `horizon-game::main()` as-is is disallowed.
- `horizon-mod-host` depends on `horizon-mod-wasm`, and **`horizon-mod-wasm` is the only crate in
  the whole vendored tree that declares `wasmtime`** (`Cargo.toml:22`). Avoiding `horizon-mod-host`
  is therefore sufficient to keep wasmtime out of the graph.
- The dedicated engine itself is `horizon-client` built with `--no-default-features` — upstream's
  CI releases it as `horizon-engine` (`.github/workflows/game-build.yml:479`,
  `nix/horizon-engine.nix`, `docs/server.md`). There is no separate engine crate.
- `horizon-mod-package::runtime::Runtime` derives `Default` and is documented as the
  vanilla-empty mod inventory, so the engine entry can be called with an empty runtime.

## APIs tried (exact calls in `src/main.rs`)

1. **`horizon_client::entry::prepare()`** — the smallest observable call into the engine.
   It is `#[doc(hidden)] pub mod entry` in `horizon-client/src/lib.rs`, so it is reachable from an
   external crate. `prepare()` reads the launch arguments/options and returns
   `entry::Startup`. It names neither the mod host nor any wasm runtime. Adopted.
2. **`horizon_client::entry::start(...)`** — the full headless engine entry, called with:
   - `horizon_mod_package::runtime::Runtime::default()` (empty inventory, no built-in mods),
   - a `load` closure returning `(Vec::new(), ())` (no loaded mod ids, no host state),
   - a no-op `entry::CodeRuntime { initialize, refusal, startup_refused }`,
   - the `Startup` from (1).

   This reaches `game::run_with(options, native)` — the real dedicated-server loop — without
   ever calling `horizon_mod_host::main_with`. Adopted as the representative "headless entry".
3. **Rejected: `horizon_game::main()`** — unconditionally calls `horizon_mod_host::main_with`,
   which pulls `horizon-mod-wasm` and wasmtime. Not used, not depended on.

Features: `horizon-client` is declared `default-features = false`, matching the upstream server
build (drops `audio`, `controllers`, `friends`). `horizon-mod-package` is a direct dependency only
so `main.rs` can name `Runtime` / `LaunchArguments` for call (2); it is already a transitive dep of
`horizon-client`, and both manifests point at the same path so the types are identical.

`[patch.crates-io]` carries the three upstream entries (`wgpu-hal`, `naga`, `libmimalloc-sys`),
rebased to `../../vendor/horizon/third_party/...`.

## Verification

Cheap preflight (allowed diagnostic, not a build):
`cargo metadata --no-deps --format-version 1 --manifest-path experiments/engine-link/Cargo.toml`
→ exit 0, resolved this experiment as its own workspace member; path deps and patch paths valid.

Single allowed check (run once, no retry):

```
cd /root/horizon-server
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=1
timeout 600 cargo check --manifest-path experiments/engine-link/Cargo.toml \
    --target-dir experiments/engine-link/target
```

**Exit code 124** — the external 10-minute timeout elapsed. It was still compiling the 496-crate
dependency set (`454` Compiling/Checking lines), had produced **no errors**, and had not yet reached
`engine-link-probe` itself, so this run did not type-check `main.rs`. Memory/timeout, not a code
error, was the blocker. Per instructions the check was **not retried** and no `cargo build/test/clippy`
was run.

Last 30 lines of output:

```
   Compiling prettyplease v0.3.0
   Compiling thiserror v1.0.69
   Compiling foldhash v0.1.5
    Checking xxhash-rust v0.8.21
   Compiling io-extras v0.18.4
    Checking subtle v2.6.1
   Compiling hashbrown v0.15.5
    Checking png v0.18.1
   Compiling parking_lot v0.12.5
   Compiling logos v0.15.1
    Checking idna v1.1.0
   Compiling logos v0.16.1
   Compiling horizon-mod-vocab v0.1.0 (/root/horizon-server/vendor/horizon/crates/horizon-mod-vocab)
   Compiling encoding_rs v0.8.44
   Compiling thiserror-impl v0.1.69
   Compiling khronos-egl v6.0.0
    Checking allocator-api2 v0.2.21
   Compiling fastrand v2.5.0
   Compiling getrandom v0.3.4
    Checking linux-raw-sys v0.4.15
   Compiling fixedbitset v0.5.9
    Checking static_assertions v1.1.0
   Compiling cap-primitives v3.4.6
    Checking glam v0.30.10
    Checking wgpu-types v30.0.2
   Compiling petgraph v0.8.3
   Compiling tempfile v3.27.0
    Checking hashbrown v0.16.1
   Compiling protox-parse v0.9.0
```

Dependency graph (read-only `cargo tree` against the lockfile the check produced, 497 packages):

- `cargo tree -i wasmtime` → `error: package ID specification 'wasmtime' did not match any packages`
  (exit 101). **wasmtime is not in the graph.**
- `cargo tree -i horizon-mod-host` → same "did not match any packages" (exit 101).
  **mod-host is not in the graph.**
- `cargo tree -i horizon-mod-wasm` → same "did not match any packages" (exit 101).
  **mod-wasm is not in the graph.**
- `grep 'name = "..."' Cargo.lock` confirms none of the three appears.
- `wasmparser v0.261` *is* present, but only as a file-format validator via `horizon-mod-package`
  (and `wit-parser` via the API generator). That is a parser, not a wasm execution/runtime; it does
  not violate the "no mod/plugin runtime" rule, but it is worth flagging.
- Unlike audio/controllers (absent with `--no-default-features`), the render stack is **unconditional**:
  `wgpu v30.0.2`, `winit v0.30.13`, `naga` and `wgpu-hal` are all in the host build graph through
  `horizon-client`, `horizon-client-lib`, `horizon-client-render`, `horizon-hdr` and
  `horizon-client-input`. `cargo tree -i wgpu` / `-i winit` confirm it. This is orthogonal to the
  mod-runtime question but contradicts AGENTS.md's "headless graph excludes wgpu/winit" rule —
  see recommendation.
- `jni` appears in the lockfile only for non-host targets (`cargo tree -i jni` → "nothing to print"
  on the host); not part of this host graph.

## Recommendation for the real integration

1. **Way in is viable and clean for the mod-runtime constraint.** A binary can depend on
   `horizon-client` (path, `default-features = false`) plus `horizon-mod-package` and call
   `entry::prepare()` and `entry::start()` with `Runtime::default()` and no-op hooks. Neither
   `horizon-mod-host`, `horizon-mod-wasm` nor wasmtime enters the graph. Do **not** call
   `horizon_game::main()`.
2. **Caveat — this check did not finish**, so treat the compile of `main.rs` as unverified. Before
   relying on it, rerun the same `cargo check` in CI (or with a larger memory limit / longer
   timeout) and confirm the two entry calls type-check. The signatures were verified by source
   inspection of `entry.rs`, `mod-package/src/runtime.rs` and `mod-package/src/selection.rs`.
3. **Caveat — the layering rule is not met by this path alone.** `wgpu`/`winit` are unconditional
   deps of `horizon-client`; `--no-default-features` does not remove them. To satisfy AGENTS.md's
   "headless graph excludes wgpu/winit", upstream must first split the engine so render/input are
   feature-gated or moved behind a crate that the server does not depend on. Until then, a single
   binary built from `horizon-client` will link the graphics stack even in dedicated mode. That is
   a source-level upstream change, not something this experiment can fix from the outside.
4. **Practical build note:** the 496-package resolution + compile did not complete in 10 min at
   `CARGO_BUILD_JOBS=1` on this container (2.95 GiB cgroup limit). For the real integration, warm
   the cache in CI and budget more time, or vendor/pre-build dependencies. The experiment left a
   populated `Cargo.lock` and ~1.3 GB of artifacts in `experiments/engine-link/target` (gitignored
   local evidence only; not for commit).
