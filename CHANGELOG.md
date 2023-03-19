# Changelog

### UNRELEASED
- Add dependant processes. See [docs](https://docs.rs/steward/latest/steward/dep/index.html).
- Add [`Cmd::spawn`](https://docs.rs/steward/latest/steward/cmd/struct.Cmd.html#method.spawn) and [`Process::spawn`](https://docs.rs/steward/latest/steward/process/struct.Process.html#method.spawn) methods.
- Expose [`RunningProcess`](https://docs.rs/steward/latest/steward/process/struct.RunningProcess.html). Add [`RunningProcess::stop`](https://docs.rs/steward/latest/steward/process/struct.RunningProcess.html#method.stop) (`unix` only, for now).

### 0.0.3
- Improve process pool output.

### 0.0.2
- Fix Windows build ([#1](https://github.com/alexfedoseev/steward/pull/1)).

### 0.0.1
Initial release.
