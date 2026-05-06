//! `dbt-fleet check` orchestration — load manifest + tier config, run all
//! policies, render results.

use anyhow::Result;
use serde::Serialize;
use std::fmt::Write;
use std::path::Path;

use crate::manifest::Manifest;
use crate::policy::{Policy, PolicyResult, Tier1ColumnsDescribed, Tier1HasOwner};
use crate::tier::TierConfig;

#[derive(Debug, Serialize)]
pub struct CheckReport {
    pub version: &'static str,
    pub tier1_models: usize,
    pub policies: Vec<PolicyResult>,
    pub total_violations: usize,
}

impl CheckReport {
    pub fn passed(&self) -> bool {
        self.total_violations == 0
    }
}

pub fn run(project_dir: &Path) -> Result<CheckReport> {
    let manifest = Manifest::find(project_dir)?;
    let tiers = TierConfig::load(project_dir)?;

    let tier_1_models: Vec<_> = manifest.models().filter(|m| tiers.is_tier_1(m)).collect();

    let policies: Vec<Box<dyn Policy>> =
        vec![Box::new(Tier1HasOwner), Box::new(Tier1ColumnsDescribed)];

    let results: Vec<PolicyResult> = policies.iter().map(|p| p.run(&tier_1_models)).collect();

    let total_violations = results.iter().map(|r| r.violations.len()).sum();

    Ok(CheckReport {
        version: env!("CARGO_PKG_VERSION"),
        tier1_models: tier_1_models.len(),
        policies: results,
        total_violations,
    })
}

/// Pretty-print a CheckReport to stdout.
pub fn print_human(report: &CheckReport) {
    let mut buf = String::new();
    let _ = write_human(report, &mut buf);
    print!("{}", buf);
}

/// Write the human-readable report into any `Write`-able buffer. Used both
/// by `print_human` and by `--output <path>` mode.
pub fn write_human(report: &CheckReport, w: &mut String) -> std::fmt::Result {
    writeln!(w, "dbt-fleet {}", report.version)?;
    writeln!(w)?;
    writeln!(w, "Tier-1 models: {}", report.tier1_models)?;
    writeln!(w, "Policies checked: {}", report.policies.len())?;
    writeln!(w)?;

    for policy in &report.policies {
        let marker = if policy.passed() {
            "\u{2713}"
        } else {
            "\u{2717}"
        };
        let count = policy.violations.len();
        if count == 0 {
            writeln!(w, "  {} {}", marker, policy.name)?;
        } else {
            writeln!(
                w,
                "  {} {} \u{2014} {} violation{}",
                marker,
                policy.name,
                count,
                if count == 1 { "" } else { "s" }
            )?;
            for v in &policy.violations {
                let location = match &v.column {
                    Some(col) => format!("{}:{}", v.file, col),
                    None => v.file.clone(),
                };
                writeln!(w, "      \u{2022} {} \u{2014} {}", location, v.message)?;
            }
        }
    }

    writeln!(w)?;
    if report.passed() {
        writeln!(w, "Passed: 0 violations.")?;
    } else {
        let failing = report.policies.iter().filter(|p| !p.passed()).count();
        writeln!(
            w,
            "Failed: {} violation{} across {} polic{}.",
            report.total_violations,
            if report.total_violations == 1 {
                ""
            } else {
                "s"
            },
            failing,
            if failing == 1 { "y" } else { "ies" },
        )?;
    }
    Ok(())
}

/// JSON-serialize a CheckReport to stdout.
pub fn print_json(report: &CheckReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}
