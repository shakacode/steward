# Changelog

### Unreleased

**Windows support**
- [BREAKING] Migrated from unmaintained `winapi` to `windows-sys`. Windows build now works on recent Rust versions.
- Added `RunningProcess::stop()` for Windows using `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT)`.
- Added process group support on Windows via `CREATE_NEW_PROCESS_GROUP`.
- Unified Unix/Windows `Cmd::spawn()` into a single implementation.

**Dependencies**
- Upgraded `hyper` 0.14 → 1.x (with `hyper-util`, `http-body-util`, `http` 1.0).
- Upgraded `thiserror` 1 → 2, `rand` 0.8 → 0.9, `nix` 0.20 → 0.29.
- Dropped `once_cell` in favor of `std::sync::LazyLock` (MSRV now 1.85).
- Added optional `tracing` feature for structured logging.

**Performance**
- `Cmd::shelled()` returns `[&str; 2]` (stack) instead of `Vec<&str>` (heap). Measured 21x faster.
- `Cmd::spawn()` passes `&Env` to `command.envs()` instead of cloning the entire HashMap. Measured 30-80x faster for env iteration.
- `ProcessPool` uses `Arc<str>` for tag strings (cheap clones across spawned tasks).
- Output streaming uses reusable `read_line()` buffer instead of `lines()` (no per-line allocation).
- `processes_list` uses `join()` (O(n)) instead of `fold` (O(n²)).

**Reliability**
- Removed panics from hot paths: pool spawn failure now logs and skips, `HttpService::build_req()` returns `Result`.
- Changed `AtomicUsize` ordering from `Relaxed` to `SeqCst` for process pool exit counter.
- Fixed busy-wait loop in Ctrl+C exit checker (added `yield_now()`).
- Fixed `PATH::get()` case-sensitivity on Windows.
- Fixed `colors::make()` `todo!()` panic — now handles any pool size.
- Added `#[non_exhaustive]` to `Error` enum for forward compatibility.
- Added `#[must_use]` to `Cmd::run/silent/output` and `Env::insert/extend` methods.
- Added `Debug` derives for all key public types.

**Testing**
- Added 69 tests: 30 unit, 39 integration (cmd, process, pool, dependency, stress).
- Added criterion benchmark suite (env, cmd, spawn overhead).
- Enabled runnable doc tests for `Env::one` and `KillTimeout::new`.

**Infrastructure**
- Set MSRV to 1.85 (`rust-version` in Cargo.toml).
- Modernized CI: actions v4, Windows added to matrix, `cargo audit`, MSRV verification job.
- Documented security model (shell injection, trust model).

### 0.0.10
- [BREAKING] Add `join` method to `Location` trait.

### 0.0.9
- Fix `loc!` macro documentation.

### 0.0.8
- [BREAKING] Simplify `Output`: now a struct instead of enum. Use `Output::bytes()` (was `unwrap()`) and `Output::as_string()` (was `unwrap_string()`). Interrupted and killed processes now return `Err(Error::Interrupted)` and `Err(Error::Killed)` instead of `Ok` variants.
- Add process group support via `SpawnOptions::group`.
- Add borrowed `IntoIterator` impl for `Env`.
- Add [`loc!`](https://docs.rs/steward/latest/steward/macro.loc.html) macro for defining project structure/locations.

### 0.0.7
- Add [`print`](https://docs.rs/steward/latest/steward/fn.print.html) function.

### 0.0.6
- Allow unlabeled command:

```rust
cmd! {
    "cargo build",
    env: Env::empty(),
    pwd: Loc::root(),
    msg: "Building a server",
}
```

### 0.0.5
- Fix non-TLS build.

### 0.0.4
- Switch to 2021 Rust edition.
- Add dependant processes. See [docs](https://docs.rs/steward/latest/steward/dep/index.html).
- Add [`Cmd::spawn`](https://docs.rs/steward/latest/steward/cmd/struct.Cmd.html#method.spawn) and [`Process::spawn`](https://docs.rs/steward/latest/steward/process/struct.Process.html#method.spawn) methods.
- Expose [`RunningProcess`](https://docs.rs/steward/latest/steward/process/struct.RunningProcess.html). Add [`RunningProcess::stop`](https://docs.rs/steward/latest/steward/process/struct.RunningProcess.html#method.stop) (`unix` only, for now).
- Add [`run`](https://docs.rs/steward/latest/steward/fn.run.html), [`run_mut`](https://docs.rs/steward/latest/steward/fn.run_mut.html) and [`run_once`](https://docs.rs/steward/latest/steward/fn.run_once.html) functions.

### 0.0.3
- Improve process pool output.

### 0.0.2
- Fix Windows build ([#1](https://github.com/alexfedoseev/steward/pull/1)).

### 0.0.1
Initial release.
