# wtop - Quality Pass, Responsive Titles, CI/CD + Self-Update

Plan: C:\Users\Gabs\.claude\plans\sorted-watching-hippo.md

## Workstream A - Code quality + responsive truncation [DONE]
- [x] `src/ui/text.rs` - shared `truncate`, `draw_str`, `draw_str_with` helpers
- [x] Register `pub mod text;` in `src/ui/mod.rs`
- [x] `header.rs` - use text helpers for logo/uptime/badges
- [x] `cpu_panel.rs` - content-aware title truncation, use text helpers (fixes `lx` counter lint)
- [x] `mem_disk_panel.rs` - use text helpers, `.enumerate()` for partitions (fixes `cur_y` counter lint)
- [x] `net_gpu_panel.rs` - content-aware GPU name + NET iface truncation (THE bug fix), use text helpers
- [x] `proc_panel.rs` - `ProcPanelView` struct (fixes too_many_arguments), use text helpers, delete local `truncate_str`
- [x] `help_modal.rs` - use text helpers, add `u` shortcut row
- [x] `layout.rs` - update proc panel call site to use `ProcPanelView`
- [x] `app.rs:68` - collapsible_match fix
- [x] `collectors/cpu.rs`, `collectors/gpu.rs` - c"" string literals (+ `.cast()`)
- [x] `config.rs` - let-chain collapses in load()/save()
- [x] `braille.rs` - saturating_sub fix
- [x] `README.md` - add `u` keybinding row
- [x] whole-project `cargo fmt` (codebase wasn't fmt-clean before this work either; needed so CI's fmt gate is meaningful from day one)
- [x] `cargo clippy --release --all-targets` -> 0 warnings (confirmed)
- [x] `cargo fmt -- --check` -> clean (confirmed)
- [x] `cargo test` passes (13/13)
- [x] `cargo build --release` succeeds

## Workstream B - Self-update [DONE]
- [x] B.0 risk spike: ureq+rustls links fine against MSYS2 UCRT64 GCC, no native-tls fallback needed
- [x] `src/updater.rs` - ReleaseInfo, UpdateStatus, spawn_check, is_newer, spawn_apply_update, cleanup_previous_update
- [x] `mod updater;` in main.rs
- [x] `app.rs` - AppState fields (update_status, pending_update_confirm, update_confirmed), `u` keybinding, confirm block
- [x] `main.rs` - Arc<Mutex<UpdateStatus>>, cleanup + spawn_check at startup, per-frame sync, apply/relaunch routing
- [x] `header.rs` - update badge states (Idle/Error render nothing)
- [x] `src/ui/update_modal.rs` - confirm dialog
- [x] `layout.rs` - render update modal
- [x] Unit tests: `text::truncate`, `updater::is_newer`, PE-verify, rename-dance in temp_dir, net_gpu_panel regression test proving throughput never gets cut off
- [x] `[profile.release]` (lto, strip, codegen-units=1, panic=abort) - binary is 2.24MB, SMALLER than pre-feature 2.6MB baseline
- [x] `cargo build --release` + `cargo clippy -D warnings` + `cargo fmt --check` + `cargo test` all clean (19/19 tests)
- [x] Binary smoke-tested (starts, renders, updater thread doesn't block/crash, no lingering process)
- [ ] Manual resize test in a real terminal - NOT done (no interactive TTY available in this environment); covered instead by the automated net_gpu_panel regression test + cpu_title unit tests, which assert the same invariant more rigorously than eyeballing

## Workstream C - CI/CD [DONE]
- [x] `.github/workflows/ci.yml` (fmt, clippy -D warnings, build, test on push/PR to master)
- [x] `.github/workflows/release.yml` (tag v*.*.* trigger, version guard, GNU/MSYS2 setup, bare wtop.exe + zip asset, softprops/action-gh-release)
- [x] YAML syntax validated (python yaml.safe_load on both files)
- [x] Exact rustup command from workflows dry-verified locally
- [ ] Not pushed/triggered - deliberately left for the user to trigger (tag push / release is a shared, irreversible action)

## Review section

All three workstreams complete. Final state: `cargo build --release --all-targets`,
`cargo clippy --release --all-targets -- -D warnings`, `cargo fmt -- --check`, and
`cargo test --release` (19/19) all clean. Binary smoke-tested (starts, renders, updater
thread doesn't block/crash, no lingering process). Release binary is 2.24MB — smaller
than the pre-work 2.6MB baseline, despite adding the full self-update stack (ureq +
rustls), thanks to `[profile.release]` lto/strip/codegen-units=1/panic=abort.

Deviations from the original plan, both improvements made along the way:
1. The whole codebase wasn't `rustfmt`-clean before this work (pre-existing, unrelated
   to these changes). Ran a project-wide `cargo fmt` so the new CI `fmt --check` gate
   is meaningful from day one, not immediately red on files nobody touched.
2. Added `[profile.release]` (not in the original plan) once the self-update
   dependencies visibly grew the binary from 2.6MB to 6.4MB — this was a legitimate
   "minimal footprint" concern worth addressing given the project's stated value prop,
   and it more than recovered the size.

Not done, deliberately: no tag was pushed and no GitHub Release was triggered — that's
a shared/irreversible action left for the user. The self-updater's full happy path
(real GitHub API response, badge, download, swap, relaunch) is inert until a real
release exists and can only be exercised once the first tag is published.

## Workstream D - ARM64 support [DONE, unverified in real CI]
- [x] `src/updater.rs` - `ASSET_NAME` is now `cfg(target_arch)`-gated: `wtop.exe` on
      x86_64, `wtop-arm64.exe` on aarch64 — picked by the binary's own compiled
      target, not the host OS, so an x64 build under ARM64 emulation still updates
      itself as x64
- [x] `.github/workflows/release.yml` restructured: `check-version` (shared tag/version
      guard) -> `build-x64` (unchanged GNU/MSYS2 path) + `build-arm64` (new: MSVC-host
      toolchain, `rustup target add aarch64-pc-windows-msvc`, `ilammy/msvc-dev-cmd`
      for the ARM64 cross-linker environment) -> `publish` (downloads both artifacts,
      single `softprops/action-gh-release` call, avoids a two-job race on release
      creation)
- [x] `README.md` - new "Piattaforme Supportate" section documenting both assets
- [x] YAML syntax validated (python yaml.safe_load)
- [x] `cargo fmt --check` / `cargo clippy -D warnings` / `cargo test --release` (19/19)
      / `cargo build --release` all clean after the updater.rs change; x86_64 binary
      size unaffected (still 2.24MB)
- [ ] **Not verified**: the actual `aarch64-pc-windows-msvc` cross-build has never run
      (no ARM64/MSVC cross toolchain available locally). Main risk: rustls' `ring`
      crypto backend needs working aarch64-pc-windows-msvc assembly support — if the
      `build-arm64` job fails on `ring`, documented fallback is switching ureq's `rustls`
      feature to `native-tls` in `Cargo.toml`. This will only be provable on the first
      real tag push.
