//! dbt-fleet — governance scoring and trends for dbt projects.
//!
//! Library surface is intentionally empty for v0.0.1. The first real types
//! land with the manifest.json parser in v0.0.2.

/// Crate version, sourced from Cargo.toml at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
