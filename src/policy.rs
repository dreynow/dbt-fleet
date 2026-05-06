//! Governance policies — what dbt-fleet checks, and how violations look.
//!
//! Each policy implements `Policy` and is run against the set of tier-1 models.
//! A policy returns a list of violations (empty = passing). The orchestrator
//! aggregates results and decides the exit code.

use crate::manifest::Node;

/// Minimum acceptable column description length, after trimming whitespace
/// and excluding common no-op placeholders ("TBD", "TODO", "?", etc).
const MIN_DESCRIPTION_LEN: usize = 10;
const PLACEHOLDER_DESCRIPTIONS: &[&str] = &["tbd", "todo", "?", "n/a", "na", "-", "fixme"];

#[derive(Debug, Clone, serde::Serialize)]
pub struct Violation {
    pub model: String,
    pub file: String,
    /// Column name if column-scoped; None if model-scoped.
    pub column: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PolicyResult {
    pub name: &'static str,
    pub description: &'static str,
    pub violations: Vec<Violation>,
}

impl PolicyResult {
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }
}

pub trait Policy {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn check(&self, models: &[&Node]) -> Vec<Violation>;
    fn run(&self, models: &[&Node]) -> PolicyResult {
        PolicyResult {
            name: self.name(),
            description: self.description(),
            violations: self.check(models),
        }
    }
}

// ---------------------------------------------------------------------------
// Policy 1: tier-1 models must have an owner
// ---------------------------------------------------------------------------

pub struct Tier1HasOwner;

impl Policy for Tier1HasOwner {
    fn name(&self) -> &'static str {
        "tier_1_has_owner"
    }

    fn description(&self) -> &'static str {
        "Every tier-1 model must declare an owner via meta.owner."
    }

    fn check(&self, models: &[&Node]) -> Vec<Violation> {
        models
            .iter()
            .filter(|m| m.owner().is_none())
            .map(|m| Violation {
                model: m.name.clone(),
                file: m.original_file_path.clone(),
                column: None,
                message: "meta.owner is missing or empty".into(),
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Policy 2: tier-1 model columns must have descriptions
// ---------------------------------------------------------------------------

pub struct Tier1ColumnsDescribed;

impl Policy for Tier1ColumnsDescribed {
    fn name(&self) -> &'static str {
        "tier_1_columns_described"
    }

    fn description(&self) -> &'static str {
        "Every column of a tier-1 model must have a description \u{2265} 10 chars (no TBD/TODO placeholders)."
    }

    fn check(&self, models: &[&Node]) -> Vec<Violation> {
        let mut violations = Vec::new();
        for model in models {
            // Models with no columns declared in schema.yml at all → one
            // violation per model rather than spamming.
            if model.columns.is_empty() {
                violations.push(Violation {
                    model: model.name.clone(),
                    file: model.original_file_path.clone(),
                    column: None,
                    message: "no columns declared in schema.yml — add column definitions with descriptions".into(),
                });
                continue;
            }
            for column in model.columns.values() {
                if let Some(reason) = description_violation(&column.description) {
                    violations.push(Violation {
                        model: model.name.clone(),
                        file: model.original_file_path.clone(),
                        column: Some(column.name.clone()),
                        message: reason,
                    });
                }
            }
        }
        violations
    }
}

fn description_violation(desc: &str) -> Option<String> {
    let trimmed = desc.trim();
    if trimmed.is_empty() {
        return Some("description is empty".into());
    }
    if PLACEHOLDER_DESCRIPTIONS.contains(&trimmed.to_lowercase().as_str()) {
        return Some(format!("description is a placeholder (\"{}\")", trimmed));
    }
    if trimmed.len() < MIN_DESCRIPTION_LEN {
        return Some(format!(
            "description too short ({} chars, minimum {})",
            trimmed.len(),
            MIN_DESCRIPTION_LEN
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Column, NodeConfig};
    use std::collections::HashMap;

    fn model_with_owner(name: &str, owner: Option<&str>) -> Node {
        let mut meta = HashMap::new();
        if let Some(o) = owner {
            meta.insert("owner".into(), serde_json::json!(o));
        }
        Node {
            name: name.into(),
            resource_type: "model".into(),
            package_name: "p".into(),
            path: format!("marts/{}.sql", name),
            original_file_path: format!("models/marts/{}.sql", name),
            meta,
            config: NodeConfig::default(),
            columns: HashMap::new(),
        }
    }

    fn model_with_columns(name: &str, columns: Vec<(&str, &str)>) -> Node {
        let mut node = model_with_owner(name, Some("alice"));
        for (col_name, desc) in columns {
            node.columns.insert(
                col_name.into(),
                Column {
                    name: col_name.into(),
                    description: desc.into(),
                },
            );
        }
        node
    }

    #[test]
    fn owner_policy_flags_missing_and_empty() {
        let m1 = model_with_owner("a", Some("data-team"));
        let m2 = model_with_owner("b", None);
        let m3 = model_with_owner("c", Some(""));
        let result = Tier1HasOwner.run(&[&m1, &m2, &m3]);
        assert_eq!(result.violations.len(), 2);
        assert!(result.violations.iter().any(|v| v.model == "b"));
        assert!(result.violations.iter().any(|v| v.model == "c"));
    }

    #[test]
    fn description_policy_flags_short_empty_placeholder() {
        let model = model_with_columns(
            "fct_revenue",
            vec![
                ("invoice_id", "Primary key for the invoice"), // OK
                ("revenue_total", ""),                         // empty
                ("status", "TBD"),                             // placeholder
                ("country", "short"),                          // too short (5 < 10)
            ],
        );
        let result = Tier1ColumnsDescribed.run(&[&model]);
        let names: Vec<_> = result
            .violations
            .iter()
            .filter_map(|v| v.column.clone())
            .collect();
        assert_eq!(result.violations.len(), 3);
        assert!(names.contains(&"revenue_total".into()));
        assert!(names.contains(&"status".into()));
        assert!(names.contains(&"country".into()));
    }

    #[test]
    fn model_with_no_columns_at_all_flags_once() {
        let model = model_with_owner("undocumented", Some("alice"));
        let result = Tier1ColumnsDescribed.run(&[&model]);
        assert_eq!(result.violations.len(), 1);
        assert!(result.violations[0].message.contains("no columns declared"));
    }
}
