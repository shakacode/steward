# Steward

[![crates.io](https://meritbadge.herokuapp.com/steward)](https://crates.io/crates/steward)
[![API docs](https://docs.rs/steward/badge.svg)](https://docs.rs/steward)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

Task runner and process manager for Rust.

![steward](./examples/output.png)

If you're not happy managing your infrastructure with a pile of bash scripts, this crate might be helpful. It provides base building blocks for defining and running various kinds of tasks. It's like [`foreman`](https://github.com/ddollar/foreman) but more low-level, with Rust API and more flexibility & features.

Works great with [`clap`](https://github.com/clap-rs/clap)!

## Documentation
See [docs.rs/steward](https://docs.rs/steward).

## Examples
See [`examples`](./examples).

## Limitations
### Windows
Apparently, Windows build is broken on recent versions of Rust due to [`winapi`](https://github.com/retep998/winapi-rs) being unmaintained. We need to migrate to [`windows-rs`](https://github.com/microsoft/windows-rs), but I don't know anything about Windows, so help is very welcome!

### Async runtimes
Tokio only.

## License
MIT.

> ### ShakaCode
> If you are looking for help with the development and optimization of your project, [ShakaCode](https://www.shakacode.com) can help you to take the reliability and performance of your app to the next level.
>
> If you are a developer interested in working on Rust / ReScript / TypeScript / Ruby on Rails projects, [we're hiring](https://www.shakacode.com/career/)!
