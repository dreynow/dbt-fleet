# dbt-fleet

> Governance scoring and trends for dbt projects. CI-native, milliseconds, MIT.

`dbt-fleet` is a single Rust binary that reads your dbt project's `manifest.json` and answers three questions:

1. **Are the models that matter documented and owned?**
2. **Does this PR break anything downstream?**
3. **Are we getting better or worse over time?**

Output: a self-contained HTML report, a JSON blob for CI, or an ASCII trend chart you can post anywhere.

```text
  Overall:
    Feb 06 █████████████░░░░░░░░░░░  55.0%
    Feb 26 ████████████████░░░░░░░░  64.8%
    Mar 18 █████████████████░░░░░░░  72.2%
    Apr 06 ███████████████████░░░░░  79.3%
    Apr 26 █████████████████████░░░  87.0%
    May 06 ██████████████████████░░  91.8%

Overall: ↑ 55.0% → 91.8% (+36.8pp)
```

No database. No SaaS account. No sign-up. One YAML file of config, max.

---

## Why

| Existing tool | What it does | The gap dbt-fleet fills |
|---------------|--------------|--------------------------|
| Elementary | Test observability + anomaly detection | No governance scoring; needs a database |
| dbt-project-evaluator | dbt Labs' static audit | One-shot report, no trending, runs as dbt models (slow) |
| dbt-checkpoint | Pre-commit hooks | Pre-commit only, no scoring, no report |
| Atlan / Castor / Select Star | Full enterprise catalog | £50K–£500K/year |
| dbt Cloud Explorer | Lineage + governance for dbt Cloud users | Cloud-only |

`dbt-fleet` is for the open-source / cost-conscious / pre-enterprise-tooling-budget tier of the data platform stack.

## Install

**pip (recommended — wraps the Rust binary):**

```bash
pip install dbt-fleet
dbt-fleet --version
```

The Python package is a thin launcher: on first run it downloads the right binary for your OS/arch from the matching GitHub Release and caches it under `~/.cache/dbt-fleet/`. No Python at runtime.

**Cargo:**

```bash
cargo install dbt-fleet
```

**Direct download:** grab the archive for your platform from the [latest release](https://github.com/dreynow/dbt-fleet/releases/latest), unpack, and put `dbt-fleet` on your PATH.

A Homebrew tap and one-line shell installer are planned for v0.2.

## Quick start

Run from a dbt project root that has `target/manifest.json` (i.e., `dbt parse` or `dbt compile` has already run):

```bash
# Check governance policies
dbt-fleet check

# Record a score snapshot
dbt-fleet score

# After a few snapshots, see the trend
dbt-fleet trend

# Want a screenshot for LinkedIn before you have real history?
dbt-fleet trend --demo
```

## What it checks (v0.1)

Two policies, run against **tier-1 models** (default: anything under `models/marts/**`):

1. **`tier_1_has_owner`** — every tier-1 model must declare `meta.owner` (top-level or under `config.meta`), non-empty.
2. **`tier_1_columns_described`** — every column in a tier-1 model must have a description ≥ 10 characters, with placeholder rejection (`TBD`, `TODO`, `?`, `n/a`, `fixme`).

A third policy (breaking-change detection across two manifests) lands in v0.2.

## Configuration

Override the tier-1 definition at `<project>/.dbt-fleet/tiers.yaml`:

```yaml
tier_1:
  paths:
    - "models/marts/**"
    - "models/exposed/**"
  meta_match:
    critical: true
```

A model is tier-1 if **either** any `paths` glob matches **or** all `meta_match` key/value pairs are present in its meta.

## CI integration

Drop this into your dbt repo's `.github/workflows/dbt-fleet.yml`:

```yaml
name: dbt-fleet
on: [pull_request]

jobs:
  governance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # Replace with however your project produces target/manifest.json
      - name: dbt parse
        run: |
          pip install dbt-postgres
          dbt parse

      - uses: dreynow/dbt-fleet@v0.1.1
        with:
          project: '.'
          format: 'html'
          output: 'dbt-fleet-report.html'

      - uses: actions/upload-artifact@v4
        with:
          name: dbt-fleet-report
          path: dbt-fleet-report.html
```

Action inputs:

| Input | Default | Description |
|-------|---------|-------------|
| `project` | `.` | dbt project root |
| `format` | `html` | `human`, `json`, or `html` |
| `output` | `dbt-fleet-report.html` | Path to write the report |
| `version` | `latest` | Pin to a release like `v0.1.0` |
| `fail-on-violation` | `true` | Set `false` to report without failing the build |

## Output formats

| Format | When | Notes |
|--------|------|-------|
| `human` | Local dev, terminal | Default. Coloured-ish status markers. |
| `json` | Custom CI, dashboards | Stable, documented schema. |
| `html` | PR comments, S3, GH Pages | Self-contained, ~6 KB, no external assets. |

Exit code: `0` clean, `1` policy violations, `2` error.

## Status

**v0.1.0** — first version with the trending feature. Two policies, three output formats, GitHub Action template, MIT licensed.

Roadmap (not yet shipped):

- v0.2 — breaking-change detection (compare two manifests), per-model inventory table in the HTML report, sparkline section
- v0.3 — opinionated GitHub Action that posts the report as a PR comment
- v0.4 — schema snapshot to detect deletions/type changes without dual manifests

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Scope-disciplined: dbt-fleet does **governance + scoring + trends**, nothing more. PRs that expand scope will be politely declined.

## License

[MIT](LICENSE).

## Author

Built by [@dreynow](https://github.com/dreynow) — Lead Analytics Engineer building the tools I want to use.
