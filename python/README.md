# dbt-fleet (Python distribution)

This package is a thin wrapper around the [dbt-fleet](https://github.com/dreynow/dbt-fleet) Rust binary.

```bash
pip install dbt-fleet
dbt-fleet check
```

On first run, the wrapper downloads the right binary for your platform from the matching GitHub Release, caches it under `~/.cache/dbt-fleet/`, and execs it. All subsequent runs hit the cache.

For the real documentation, the trend screenshot, configuration, CI integration, and roadmap, see the [main README](https://github.com/dreynow/dbt-fleet#readme).

## Supported platforms

- Linux x86_64 (`x86_64-unknown-linux-gnu`)
- Linux aarch64 (`aarch64-unknown-linux-gnu`)
- macOS x86_64 (`x86_64-apple-darwin`)
- macOS aarch64 (`aarch64-apple-darwin`)
- Windows x86_64 (`x86_64-pc-windows-msvc`)

## Overrides

- `DBT_FLEET_BINARY=/path/to/dbt-fleet` — bypass download; use a binary you already have. Useful for local Rust development.
- `DBT_FLEET_CACHE_DIR=...` — override cache location. Defaults to `~/.cache/dbt-fleet/`.

## License

[MIT](https://github.com/dreynow/dbt-fleet/blob/main/LICENSE).
