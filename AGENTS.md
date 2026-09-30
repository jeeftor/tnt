# Repository Guidelines

## Project Structure & Module Organization

`tnt` is a Rust terminal monitor for TrueNAS SCALE. `src/main.rs` owns the command-line interface, sampling loop, and Ratatui display. `src/sensors.rs` parses `sensors -j` JSON into temperature and fan readings. Unit tests live beside the code they cover in `#[cfg(test)]` modules. `Cargo.toml` defines dependencies; `Cargo.lock` pins them. `dist/` contains a prebuilt Linux x86_64 binary and `SHA256SUMS`. Cargo build output belongs in the ignored `target/` directory.

## Build, Test, and Development Commands

- `cargo fmt --check` checks Rust formatting; run `cargo fmt` to apply it.
- `cargo test` runs the parser and terminal-rendering unit tests.
- `cargo clippy --all-targets -- -D warnings` checks for Rust lint warnings.
- `cargo build --release` builds `target/release/tnt`. Build release binaries on Linux x86_64 for the documented TrueNAS deployment target.
- `cargo run -- --interval 2` runs the monitor with a two-second sample interval on a Linux host with `sensors` in `PATH`. The default is five seconds.

## Coding Style & Naming Conventions

Use Rust 2024 syntax and `rustfmt` defaults (four-space indentation). Follow existing Rust naming: `snake_case` for functions and modules, `PascalCase` for types and enum variants, and uppercase names for constants. Keep JSON parsing in `sensors.rs` and terminal behavior in `main.rs`. Preserve the monitor's read-only behavior; it must not control fans or write sensor settings.

## Testing Guidelines

Use Rust's built-in `#[test]` framework. Name tests for observable behavior, as in `parses_truenas_fans_and_temperatures`. Add representative `sensors -j` JSON cases when changing parsing, and use Ratatui's `TestBackend` for display changes. Run `cargo test` before submitting. There is no stated coverage percentage requirement.

## Commits & Pull Requests

This repository has no commit history yet, so no established message convention can be inferred. Use short, imperative subjects that identify the change. In pull requests, describe the user-visible effect, list verification commands and results, and note any TrueNAS or Linux testing that remains. Include a terminal screenshot when the display changes; link a related issue when one exists.
