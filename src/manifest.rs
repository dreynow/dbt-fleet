//! dbt `manifest.json` parser — only the fields dbt-fleet needs.
//!
//! dbt's full manifest is huge (>50 fields per node, sources, exposures,
//! semantic models, metrics, etc). We deserialize a narrow subset and let
//! `serde` ignore everything else.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Top-level manifest.json shape. We keep `nodes` only — sources, exposures,
/// semantic models etc. aren't policed in v0.0.2.
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub nodes: HashMap<String, Node>,
}

/// A node from `manifest.nodes`. Models are `resource_type == "model"`;
/// tests, seeds, snapshots, analyses also live here and are filtered out
/// downstream.
#[derive(Debug, Deserialize, Clone)]
pub struct Node {
    pub name: String,
    pub resource_type: String,
    pub package_name: String,
    /// Source-relative path, e.g. `marts/fct_revenue.sql`.
    pub path: String,
    /// Project-root-relative path, e.g. `models/marts/fct_revenue.sql`.
    pub original_file_path: String,
    /// dbt's per-node `meta` dict. Owner conventions vary; we look at
    /// `meta.owner` (string) by default.
    #[serde(default)]
    pub meta: HashMap<String, serde_json::Value>,
    /// Parsed model config (post-merge of project + model-level config).
    #[serde(default)]
    pub config: NodeConfig,
    /// Columns declared in `schema.yml` for this model. Missing → no docs.
    #[serde(default)]
    pub columns: HashMap<String, Column>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct NodeConfig {
    /// Some dbt projects put the owner in `config.meta.owner`, others in
    /// the top-level `meta.owner`. We check both.
    #[serde(default)]
    pub meta: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Column {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

impl Manifest {
    /// Parse a manifest.json from disk.
    pub fn from_path(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest at {}", path.display()))?;
        Self::parse(&text)
    }

    /// Parse from a JSON string. Useful for tests.
    /// Named `parse` rather than `from_str` to avoid clashing with the
    /// `FromStr` trait, which would force the same signature.
    pub fn parse(text: &str) -> Result<Self> {
        serde_json::from_str(text).context("Failed to parse manifest.json")
    }

    /// Find the manifest.json for a dbt project rooted at `project_dir`.
    /// Looks at `<project>/target/manifest.json` first; returns an error if
    /// not present (the user has to run `dbt parse` or `dbt compile` first).
    pub fn find(project_dir: &Path) -> Result<Self> {
        let candidate = project_dir.join("target").join("manifest.json");
        if !candidate.exists() {
            anyhow::bail!(
                "No manifest.json found at {}. Run `dbt parse` or `dbt compile` first.",
                candidate.display()
            );
        }
        Self::from_path(&candidate)
    }

    /// Iterator over nodes that are dbt models (excludes tests, seeds, etc.).
    pub fn models(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values().filter(|n| n.resource_type == "model")
    }
}

impl Node {
    /// Resolve the owner field, checking both `meta.owner` and `config.meta.owner`.
    /// Returns the owner string if present and non-empty, else None.
    pub fn owner(&self) -> Option<&str> {
        self.meta
            .get("owner")
            .or_else(|| self.config.meta.get("owner"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_MANIFEST: &str = r#"{
        "nodes": {
            "model.proj.fct_revenue": {
                "name": "fct_revenue",
                "resource_type": "model",
                "package_name": "proj",
                "path": "marts/fct_revenue.sql",
                "original_file_path": "models/marts/fct_revenue.sql",
                "meta": {"owner": "data-team"},
                "config": {"meta": {}},
                "columns": {
                    "invoice_id": {"name": "invoice_id", "description": "Primary key"}
                }
            },
            "test.proj.unique_fct_revenue_invoice_id": {
                "name": "unique_fct_revenue_invoice_id",
                "resource_type": "test",
                "package_name": "proj",
                "path": "...",
                "original_file_path": "..."
            }
        }
    }"#;

    #[test]
    fn parses_minimal_manifest() {
        let m = Manifest::parse(MINIMAL_MANIFEST).unwrap();
        assert_eq!(m.nodes.len(), 2);
        let models: Vec<&Node> = m.models().collect();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "fct_revenue");
    }

    #[test]
    fn owner_resolves_from_top_level_meta() {
        let m = Manifest::parse(MINIMAL_MANIFEST).unwrap();
        let model = m.models().next().unwrap();
        assert_eq!(model.owner(), Some("data-team"));
    }

    #[test]
    fn owner_falls_back_to_config_meta() {
        let json = r#"{"nodes": {"model.p.x": {
            "name": "x", "resource_type": "model", "package_name": "p",
            "path": "x.sql", "original_file_path": "models/x.sql",
            "meta": {},
            "config": {"meta": {"owner": "alice"}}
        }}}"#;
        let m = Manifest::parse(json).unwrap();
        assert_eq!(m.models().next().unwrap().owner(), Some("alice"));
    }

    #[test]
    fn empty_owner_string_is_none() {
        let json = r#"{"nodes": {"model.p.x": {
            "name": "x", "resource_type": "model", "package_name": "p",
            "path": "x.sql", "original_file_path": "models/x.sql",
            "meta": {"owner": "   "},
            "config": {"meta": {}}
        }}}"#;
        let m = Manifest::parse(json).unwrap();
        assert_eq!(m.models().next().unwrap().owner(), None);
    }
}
