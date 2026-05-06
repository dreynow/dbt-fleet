//! dbt-fleet — governance scoring and trends for dbt projects.

pub mod check;
pub mod demo;
pub mod history;
pub mod manifest;
pub mod policy;
pub mod render;
pub mod score;
pub mod tier;

/// Crate version, sourced from Cargo.toml at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
