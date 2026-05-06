//! Score computation — turn a CheckReport into per-policy percentages and
//! an overall governance score.
//!
//! Scoring philosophy: percentages are "share of objects passing." For
//! ownership, that's models. For descriptions, it's columns. Overall score
//! is the unweighted mean of the per-policy percentages — simple is more
//! defensible than a hidden weighting scheme.

use serde::{Deserialize, Serialize};

use crate::check::CheckReport;
use crate::manifest::Manifest;
use crate::tier::TierConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreSnapshot {
    /// ISO-8601 UTC timestamp of when the score was computed.
    pub timestamp: String,
    pub tier1_models: usize,
    pub total_columns: usize,
    pub ownership_pct: f64,
    pub description_pct: f64,
    pub overall_pct: f64,
    pub total_violations: usize,
}

impl ScoreSnapshot {
    /// Compute a snapshot from a check report + the manifest used to produce it.
    /// We re-walk the manifest to count columns (the report aggregates violations
    /// but not totals).
    pub fn compute(report: &CheckReport, manifest: &Manifest, tiers: &TierConfig) -> Self {
        let tier_1: Vec<_> = manifest.models().filter(|m| tiers.is_tier_1(m)).collect();
        let total_columns: usize = tier_1.iter().map(|m| m.columns.len()).sum();

        let owner_violations = report
            .policies
            .iter()
            .find(|p| p.name == "tier_1_has_owner")
            .map(|p| p.violations.len())
            .unwrap_or(0);
        let desc_violations = report
            .policies
            .iter()
            .find(|p| p.name == "tier_1_columns_described")
            .map(|p| p.violations.len())
            .unwrap_or(0);

        let ownership_pct = pct(
            report.tier1_models.saturating_sub(owner_violations),
            report.tier1_models,
        );
        // Description violations include both per-column AND per-model
        // (when a model has zero columns declared). For scoring we use
        // total declared columns as the denominator; a no-columns-at-all
        // model contributes 0/0 → treated as 0% via pct().
        let description_pct = pct(total_columns.saturating_sub(desc_violations), total_columns);

        let overall_pct = match (report.tier1_models, total_columns) {
            (0, 0) => 0.0,
            (_, 0) => ownership_pct,
            (0, _) => description_pct,
            _ => (ownership_pct + description_pct) / 2.0,
        };

        Self {
            timestamp: now_iso8601(),
            tier1_models: report.tier1_models,
            total_columns,
            ownership_pct,
            description_pct,
            overall_pct,
            total_violations: report.total_violations,
        }
    }
}

fn pct(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        (numerator as f64 / denominator as f64) * 100.0
    }
}

/// ISO-8601 timestamp like `2026-05-06T11:30:00Z`. Hand-rolled to avoid chrono.
fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    iso8601_from_unix(secs)
}

pub(crate) fn iso8601_from_unix(mut secs: i64) -> String {
    let mut days_since_epoch = secs.div_euclid(86_400);
    secs = secs.rem_euclid(86_400);
    let hours = (secs / 3600) as u32;
    let minutes = ((secs % 3600) / 60) as u32;
    let seconds = (secs % 60) as u32;

    let mut year: i64 = 1970;
    loop {
        let len = if is_leap_year(year) { 366 } else { 365 };
        if days_since_epoch < len {
            break;
        }
        days_since_epoch -= len;
        year += 1;
    }
    let months_in_year: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month: usize = 0;
    while month < 12 && days_since_epoch >= months_in_year[month] {
        days_since_epoch -= months_in_year[month];
        month += 1;
    }
    let day = days_since_epoch + 1;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month + 1,
        day,
        hours,
        minutes,
        seconds
    )
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::CheckReport;
    use crate::policy::{PolicyResult, Violation};

    fn report(tier1: usize, owner_violations: usize, desc_violations: usize) -> CheckReport {
        CheckReport {
            version: "test",
            tier1_models: tier1,
            policies: vec![
                PolicyResult {
                    name: "tier_1_has_owner",
                    description: "x",
                    violations: (0..owner_violations)
                        .map(|i| Violation {
                            model: format!("m{}", i),
                            file: "f".into(),
                            column: None,
                            message: "m".into(),
                        })
                        .collect(),
                },
                PolicyResult {
                    name: "tier_1_columns_described",
                    description: "x",
                    violations: (0..desc_violations)
                        .map(|i| Violation {
                            model: format!("m{}", i),
                            file: "f".into(),
                            column: Some(format!("c{}", i)),
                            message: "m".into(),
                        })
                        .collect(),
                },
            ],
            total_violations: owner_violations + desc_violations,
        }
    }

    #[test]
    fn pct_handles_zero_denominator() {
        assert_eq!(pct(0, 0), 0.0);
        assert_eq!(pct(5, 0), 0.0);
        assert_eq!(pct(7, 10), 70.0);
        assert_eq!(pct(10, 10), 100.0);
    }

    #[test]
    fn iso8601_basic_format() {
        // 2026-05-06 09:00:00 UTC
        assert_eq!(iso8601_from_unix(1_778_058_000), "2026-05-06T09:00:00Z");
    }

    #[test]
    fn iso8601_leap_day() {
        // 2024-02-29 12:30:45 UTC
        assert_eq!(iso8601_from_unix(1_709_209_845), "2024-02-29T12:30:45Z");
    }

    #[test]
    fn snapshot_with_no_violations_is_full_score() {
        // Synthesize a manifest with 5 tier-1 models, 10 columns total.
        use crate::manifest::{Column, Manifest, Node, NodeConfig};
        use std::collections::HashMap;
        let mut nodes = HashMap::new();
        for i in 0..5 {
            let mut cols = HashMap::new();
            for c in 0..2 {
                cols.insert(
                    format!("c{}", c),
                    Column {
                        name: format!("c{}", c),
                        description: "well-documented column".into(),
                    },
                );
            }
            nodes.insert(
                format!("model.p.m{}", i),
                Node {
                    name: format!("m{}", i),
                    resource_type: "model".into(),
                    package_name: "p".into(),
                    path: format!("marts/m{}.sql", i),
                    original_file_path: format!("models/marts/m{}.sql", i),
                    meta: HashMap::from([("owner".into(), serde_json::json!("alice"))]),
                    config: NodeConfig::default(),
                    columns: cols,
                },
            );
        }
        let manifest = Manifest { nodes };
        let tiers = TierConfig::default_marts().unwrap();
        let r = report(5, 0, 0);
        let snap = ScoreSnapshot::compute(&r, &manifest, &tiers);
        assert_eq!(snap.tier1_models, 5);
        assert_eq!(snap.total_columns, 10);
        assert_eq!(snap.ownership_pct, 100.0);
        assert_eq!(snap.description_pct, 100.0);
        assert_eq!(snap.overall_pct, 100.0);
    }
}
