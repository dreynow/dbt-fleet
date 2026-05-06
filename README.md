# dbt-fleet

> Governance scoring and trends for dbt projects. CI-native, milliseconds, MIT.

`dbt-fleet` is a single-binary CLI that reads your dbt `manifest.json` and answers three questions:

1. **Are the models that matter documented and owned?** — tier-1 models without owners, descriptions, or test coverage are flagged.
2. **Does this PR break anything downstream?** — schema changes that affect tier-1 models fail the CI check.
3. **Are we getting better or worse over time?** — score history (ownership %, doc %, test %) tracked across runs.

Output: a single self-contained HTML report you can drop in a PR comment, post to Slack, or ship to GitHub Pages. No database. No SaaS account. No config beyond one YAML file.

## Status

**Pre-alpha — under active development.** Watch this repo for the v0.1 release. ETA ~6 weekends from May 2026.

If you're a Lead/Senior Analytics Engineer managing 500+ dbt models and the operational chaos sounds familiar, ⭐ this repo and check back in July.

## Why

| Existing tool | What it does | The gap dbt-fleet fills |
|---------------|--------------|--------------------------|
| Elementary | Test observability, anomaly detection | No governance scoring; needs a database |
| dbt-project-evaluator | dbt Labs' static audit | One-shot report, no trending, runs as dbt models (slow) |
| dbt-checkpoint | Pre-commit hooks | Pre-commit only; no scoring; no report |
| Atlan / Castor / Select Star | Full enterprise catalog | £50K–£500K/year |
| dbt Cloud Explorer | Lineage + governance for dbt Cloud users | Cloud-only |

`dbt-fleet` is for the open-source / cost-conscious / pre-enterprise-tooling-budget tier of the data platform stack.

## Install (coming soon)

```bash
# Homebrew
brew install dreynow/tap/dbt-fleet

# Cargo
cargo install dbt-fleet

# Direct download
curl -sSL https://github.com/dreynow/dbt-fleet/releases/latest/download/install.sh | sh
```

## License

MIT.

## Author

Built by [@dreynow](https://github.com/dreynow) — Lead Analytics Engineer building the tools I want to use.
