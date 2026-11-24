# Owner-run Android ARM64 tablet acceptance plan

This container has no adb, no device nodes and no Termux connection; the owner confirmed tablet
validation is performed on their side. **No result in this repository claims any of these tests
passed.** The checklist items in `PROGRESS.md` stay unchecked until real device logs are returned.

> Current status: the ARM64 artifact from CI run 45333795229 contains the dedicated engine linked
> through the no-mod entry seam (single ELF, 91,568,072 bytes, SHA-256
> `f973fa5884983ae26dbbdb7a76ac760577e94d8785bf0bb302990eeb7bba0d7d`, archive SHA-256
> `d6f3a0c0de888d9dc65dc5f5fe3af941ce2034f5c0918553458942620f281548`). It has **not been run on a
> tablet**, and no world/client/parity result exists yet. Use this plan as the acceptance procedure
> and return the evidence listed at the end; steps that fail because prepared tables or runtime
> behavior are incomplete must be reported as failures, not silently skipped.

## Prerequisites returned with the build

- One `horizon-server` ELF (`aarch64-linux-android`, arm64-v8a) and its SHA-256 checksum.
- Rust/NDK/toolchain versions from CI logs; commit SHA of this repository.
- Confirmation that no `.jar`, `.so`, sidecar executor or Mojang content is in the release archive.

## Procedure (run in Termux on the ARM64 tablet)

Use a fresh scratch directory each time; never point a test at a real world or player data.

1. **Binary sanity**
   ```sh
   sha256sum -c horizon-server.sha256
   ./horizon-server --version
   ./horizon-server --help
   ```

2. **First startup, EULA refusal**
   ```sh
   mkdir -p /sdcard/horizon-test/first && cd /sdcard/horizon-test/first
   /path/to/horizon-server
   ```
   Expected: explains the EULA and refuses to start; `eula.txt` is absent or not silently accepted.
   With `--accept-eula`, `eula.txt` must contain `eula=true`.

3. **Prepared data (owner-approved interim source)**
   ```sh
   ./horizon-server prepare --dir <server-dir> --prepared-folder <folder-with-prepared-manifest.json>
   ```
   The folder must contain `prepared-manifest.json` (`version: 26.3`, per-file SHA-256 and size).
   Verify a tampered file or path is rejected, and that the accepted copy lands in
   `<server-dir>/.horizon/prepared/26.3` with the manifest intact.

4. **Offline startup and cache**
   After a successful bootstrap, rerun with `--offline`; expected: verified cache reused, no network
   required. Delete `.horizon/downloads` and confirm `--offline` fails with an actionable message.

6. **Configuration and directory**
   Check `server.properties` defaults, edit `motd`/`server-port`, restart and verify the value was
   preserved and applied. Check `logs/`, player JSON files and `<level-name>/datapacks/` exist.

7. **World lifecycle**
   Create a world, stop gracefully, start again and confirm the same world is reopened (`level.dat`,
   region files). Verify no world was overwritten by first-run setup.

8. **Vanilla client**
   From an unmodified Minecraft Java 26.3 client, query status and join. Move, load chunks, chat,
   save/restart, travel to Nether and End. Record exact client version and results.

9. **Administration**
   `stop`, `save-all`, `list`, `op`, `deop`, `whitelist`, `ban`, `pardon`, `kick`, `say`; check the
   vanilla player JSON files change as expected.

10. **Datapacks**
   Place one directory pack and one ZIP pack with `pack.mcmeta` format 121 in
   `<world>/datapacks/`. Verify `horizon-server datapacks` lists them, that wrong formats are
   rejected, then verify server-side `/datapack list|enable|disable` and `/reload` once integrated.

11. **Shutdown**
   Send SIGINT/SIGTERM (or Ctrl-C); confirm clean save, closed sockets and a final log line.

## Evidence to return

For each step: exact command, exit code, stdout/stderr, relevant file contents (`eula.txt`,
`server.properties`, `logs/latest.log`), `uname -m`, `file horizon-server` on the device, vanilla
client version, and whether a real client connected. Cross-compile, emulator or host-test results are
not substitutes and must be labelled separately.
