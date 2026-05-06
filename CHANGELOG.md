# Changelog

All notable changes to dbt-fleet are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned for v0.0.2
- `dbt-fleet check` parses `manifest.json`
- First three policies: tier-1 ownership, tier-1 column descriptions, breaking-change detection
- JSON output + non-zero exit on violation
- Tier configuration via `.dbt-fleet/tiers.yaml`

## [0.0.1] — 2026-05-09

Pre-alpha skeleton. Establishes the binary, CLI surface, cross-platform release
pipeline, and CI gates. No real functionality yet — `dbt-fleet --version`,
`--help`, and stub subcommands only.

### Added
- CLI scaffold with `clap`: `dbt-fleet check`, `dbt-fleet score`, `dbt-fleet trend` (all stubs)
- Release pipeline: builds for linux x86_64, linux aarch64 (cross), macOS x86_64, macOS aarch64, windows x86_64
- CI: `cargo fmt`, `cargo clippy -D warnings`, `cargo test` on linux/macos/windows
- Smoke tests for `--version`, `--help`, and unimplemented-subcommand exit code
