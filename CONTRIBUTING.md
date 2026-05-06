# Contributing

dbt-fleet is pre-alpha. The fastest way to help right now: try v0.0.2+ when it
ships and tell me what breaks against your real dbt project.

## Local development

```bash
git clone git@github.com:dreynow/dbt-fleet.git
cd dbt-fleet
cargo build
cargo test
```

## Before opening a PR

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

CI runs the same three checks. Fail any → blocked.

## Scope discipline

dbt-fleet does **governance scoring + trends**. It is not a catalog, not an
observability tool, not a lineage explorer, not an AI assistant. PRs that
expand scope will be politely declined.

If you want a feature, open an issue first describing the *problem* (not the
solution). The right fix may already be on the roadmap.
