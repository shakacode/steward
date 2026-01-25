# Changelog

### UNRELEASED
- [BREAKING] Simplify `Output`: now a struct instead of enum. Use `Output::bytes()` (was `unwrap()`) and `Output::as_string()` (was `unwrap_string()`). Interrupted and killed processes now return `Err(Error::Interrupted)` and `Err(Error::Killed)` instead of `Ok` variants.
- Add process group support via `SpawnOptions::group`.

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
