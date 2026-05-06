//! Tier classification — which models are tier-1 (production-critical)?
//!
//! Default: anything matching `models/marts/**`. The convention is that dbt
//! projects already separate `staging/` (intermediate, low-stakes) from
//! `marts/` (consumed by dashboards, finance, exec) — we lean on it.
//!
//! Override via `.dbt-fleet/tiers.yaml` at the project root:
//!
//! ```yaml
//! tier_1:
//!   paths:
//!     - "models/marts/**"
//!     - "models/exposed/**"
//!   meta_match:
//!     critical: true
//! ```
//!
//! A node is tier-1 if EITHER any path glob matches OR all meta_match key/value
//! pairs are present in the node's meta. Most projects only need `paths`.

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::manifest::Node;

/// Resolved tier configuration.
pub struct TierConfig {
    paths: GlobSet,
    meta_match: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Default)]
struct TierFile {
    tier_1: Tier1Spec,
}

#[derive(Debug, Deserialize, Default)]
struct Tier1Spec {
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    meta_match: HashMap<String, serde_json::Value>,
}

impl TierConfig {
    /// Load tier config from `<project>/.dbt-fleet/tiers.yaml`. If the file is
    /// missing, fall back to the default (`models/marts/**`).
    pub fn load(project_dir: &Path) -> Result<Self> {
        let path = project_dir.join(".dbt-fleet").join("tiers.yaml");
        if path.exists() {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let spec: TierFile = serde_yaml::from_str(&text)
                .with_context(|| format!("Failed to parse {}", path.display()))?;
            Self::from_spec(spec.tier_1)
        } else {
            Self::default_marts()
        }
    }

    /// Build the default config: tier-1 is any model under `models/marts/`.
    pub fn default_marts() -> Result<Self> {
        Self::from_spec(Tier1Spec {
            paths: vec!["models/marts/**".into()],
            meta_match: HashMap::new(),
        })
    }

    fn from_spec(spec: Tier1Spec) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();
        for pattern in &spec.paths {
            builder.add(
                Glob::new(pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?,
            );
        }
        Ok(Self {
            paths: builder.build()?,
            meta_match: spec.meta_match,
        })
    }

    /// Is this node tier-1?
    pub fn is_tier_1(&self, node: &Node) -> bool {
        // Path match: globset checks against original_file_path (the
        // project-root-relative path like "models/marts/fct_revenue.sql").
        if self.paths.is_match(&node.original_file_path) {
            return true;
        }
        // Meta match: ALL configured key/value pairs must be present in
        // the node's meta (top-level or config.meta).
        if !self.meta_match.is_empty() {
            let merged = node
                .meta
                .iter()
                .chain(node.config.meta.iter())
                .collect::<HashMap<_, _>>();
            if self
                .meta_match
                .iter()
                .all(|(k, v)| merged.get(k).map(|got| *got == v).unwrap_or(false))
            {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Node, NodeConfig};

    fn node(path: &str, meta: HashMap<String, serde_json::Value>) -> Node {
        Node {
            name: "x".into(),
            resource_type: "model".into(),
            package_name: "p".into(),
            path: path.into(),
            original_file_path: path.into(),
            meta,
            config: NodeConfig::default(),
            columns: HashMap::new(),
        }
    }

    #[test]
    fn default_treats_marts_as_tier_1() {
        let cfg = TierConfig::default_marts().unwrap();
        assert!(cfg.is_tier_1(&node("models/marts/fct_revenue.sql", HashMap::new())));
        assert!(!cfg.is_tier_1(&node("models/staging/stg_invoices.sql", HashMap::new())));
    }

    #[test]
    fn meta_match_requires_all_keys() {
        let spec = Tier1Spec {
            paths: vec![],
            meta_match: HashMap::from([("critical".into(), serde_json::json!(true))]),
        };
        let cfg = TierConfig::from_spec(spec).unwrap();
        assert!(cfg.is_tier_1(&node(
            "anywhere.sql",
            HashMap::from([("critical".into(), serde_json::json!(true))])
        )));
        assert!(!cfg.is_tier_1(&node("anywhere.sql", HashMap::new())));
    }

    #[test]
    fn path_or_meta_match_either_qualifies() {
        let spec = Tier1Spec {
            paths: vec!["models/marts/**".into()],
            meta_match: HashMap::from([("tier".into(), serde_json::json!(1))]),
        };
        let cfg = TierConfig::from_spec(spec).unwrap();
        // Matches by path
        assert!(cfg.is_tier_1(&node("models/marts/x.sql", HashMap::new())));
        // Matches by meta
        assert!(cfg.is_tier_1(&node(
            "models/staging/x.sql",
            HashMap::from([("tier".into(), serde_json::json!(1))])
        )));
        // Matches neither
        assert!(!cfg.is_tier_1(&node("models/staging/x.sql", HashMap::new())));
    }
}
