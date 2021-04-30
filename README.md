# Steward

[![crates.io](https://meritbadge.herokuapp.com/steward)](https://crates.io/crates/steward)
[![API docs](https://docs.rs/steward/badge.svg)](https://docs.rs/steward)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Task runner and process manager for Rust.

If you're not happy managing your infrastructure with a pile of bash scripts, this crate might be helpful. It provides base building blocks for defining and running various kinds of tasks. It's like [`foreman`](https://github.com/ddollar/foreman) but more low-level, with Rust API and more flexibility & features.

Works great with [`clap`](https://github.com/clap-rs/clap)!

## Documentation
See [docs.rs/steward](https://docs.rs/steward).

## Examples
See [`examples`](./examples).

## Limitations
### Windows support
It wouldn't compile on Windows yet. No fundamental problems, I'm just not a user of this OS and can't properly test the implementation. There is only one platform-specific place in the crate related to a process termination. If you're a Windows user and willing to help — PRs are totally welcome.

### Async runtimes
Tokio only.

## License
MIT.
