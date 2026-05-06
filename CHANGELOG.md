# Changelog

All notable changes to dbt-fleet are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned for v0.2.0
- Third policy: breaking-change detection (compare two manifests)
- Per-model inventory table in HTML report (owner / docs / tests at a glance)
- HTML report sparkline section showing recent trend
- GitHub Action variant that posts the report as a PR comment

## [0.1.2] — 2026-05-06

### Added
- **Python distribution** under `python/` in this repo. `pip install dbt-fleet`
  now installs a thin launcher that downloads the matching Rust binary from
  the GitHub Release on first run, caches it under `~/.cache/dbt-fleet/v<ver>/`,
  verifies the SHA-256, and execs it. Subsequent runs are sub-50ms (cache hit).
- Supported platforms via `pip`: linux x86_64/aarch64, macOS x86_64/aarch64,
  windows x86_64.
- Environment overrides: `DBT_FLEET_BINARY=/path/to/binary` (skip download),
  `DBT_FLEET_CACHE_DIR=/custom/path` (override cache root).
- 19 unit tests covering platform mapping, cache layout, checksum parsing
  (handles directory prefix from our release pipeline), archive extraction
  with path-traversal protection, env override.

## [0.1.1] — 2026-05-06

### Added
- Composite GitHub Action at the repo root (`action.yml`) so users can
  drop `uses: dreynow/dbt-fleet@v0.1.1` into their dbt repo's CI. Auto-detects
  the runner's OS/arch, downloads the matching binary from GitHub Releases,
  runs `dbt-fleet check`, exposes `report-path` and `exit-code` outputs.
- README revamped: trend screenshot front-and-centre, real quick-start,
  comparison table vs Elementary / project-evaluator / Atlan, roadmap
  through v0.4.

## [0.1.0] — 2026-05-06

The first version with the trending feature — the headline differentiator.

### Added
- `dbt-fleet score` — computes a `ScoreSnapshot` from the check report and
  appends it to `<project>/.dbt-fleet/history.json`. Three percentages:
  ownership %, descriptions %, overall (unweighted mean).
- `dbt-fleet trend` — renders the score history as an ASCII bar chart in
  the terminal. Three series (overall / ownership / descriptions) with a
  trend-arrow summary at the bottom (`\u{2191}` 55.0% \u{2192} 91.8% +36.8pp).
- `dbt-fleet trend --demo` — replaces history with 90 days of plausible
  synthesized snapshots. Used for README screenshots and launch posts before
  any real history exists. Deterministic across runs.
- `.dbt-fleet/history.json` is gitignored by default (the user opts in to
  commit it).

## [0.0.2] — 2026-05-06

### Added
- `dbt-fleet check` actually checks. Parses `target/manifest.json` from a dbt
  project root (looks for `<project>/target/manifest.json` by default).
- Tier-1 classification with sane default (`models/marts/**`) and override
  via `.dbt-fleet/tiers.yaml` (path globs and/or meta-key matching).
- Two policies:
  - `tier_1_has_owner` — every tier-1 model must declare `meta.owner`
    (top-level or nested under `config.meta`), non-empty after trim.
  - `tier_1_columns_described` — every column needs a description \u{2265} 10
    chars, with placeholder rejection ("TBD", "TODO", "?", etc).
- Three output formats: `--format human` (default), `--format json`,
  `--format html` (self-contained single-file report, ~6 KB, no external assets).
- `--output <path>` writes to a file instead of stdout.
- Exit code: 0 when all policies pass, 1 on violations, 2 on errors.

### Verified
- Real dbt manifest parsed cleanly (`tutorials/dbt-quickstart` fixture).
- 17 unit tests + 6 CLI tests, all passing on linux/mac/windows.

## [0.0.1] — 2026-05-09

Pre-alpha skeleton. Establishes the binary, CLI surface, cross-platform release
pipeline, and CI gates. No real functionality yet — `dbt-fleet --version`,
`--help`, and stub subcommands only.

### Added
- CLI scaffold with `clap`: `dbt-fleet check`, `dbt-fleet score`, `dbt-fleet trend` (all stubs)
- Release pipeline: builds for linux x86_64, linux aarch64 (cross), macOS x86_64, macOS aarch64, windows x86_64
- CI: `cargo fmt`, `cargo clippy -D warnings`, `cargo test` on linux/macos/windows
- Smoke tests for `--version`, `--help`, and unimplemented-subcommand exit code
