//! Self-contained HTML report. No external assets, no JS framework, no web
//! fonts — the output is one file you can drop into a PR comment, attach to
//! Slack, host on GH Pages, or save locally.
//!
//! Visual quality is intentional: this is the artifact people share. Looks
//! cheap → adoption fails. We use a system font stack, restrained palette,
//! and generous whitespace.

use crate::check::CheckReport;
use crate::policy::Violation;

const STYLES: &str = r#"
:root {
  --bg: #fafaf9;
  --surface: #ffffff;
  --border: #e7e5e4;
  --border-strong: #d6d3d1;
  --text: #1c1917;
  --text-muted: #57534e;
  --text-subtle: #78716c;
  --accent: #0c0a09;
  --pass: #15803d;
  --pass-bg: #f0fdf4;
  --fail: #b91c1c;
  --fail-bg: #fef2f2;
  --warn-bg: #fffbeb;
}

* { box-sizing: border-box; }

html, body {
  margin: 0;
  padding: 0;
  background: var(--bg);
  color: var(--text);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen,
    Ubuntu, Cantarell, "Helvetica Neue", sans-serif;
  font-size: 15px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.page {
  max-width: 880px;
  margin: 0 auto;
  padding: 48px 24px 96px;
}

header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  border-bottom: 1px solid var(--border);
  padding-bottom: 24px;
  margin-bottom: 32px;
}

.brand {
  font-weight: 600;
  letter-spacing: -0.01em;
  font-size: 18px;
}

.brand .version {
  color: var(--text-subtle);
  font-weight: 400;
  font-size: 13px;
  margin-left: 8px;
}

.meta {
  font-size: 13px;
  color: var(--text-muted);
}

.summary {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 28px 32px;
  margin-bottom: 32px;
}

.summary-row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 14px;
  padding: 6px 12px;
  border-radius: 999px;
  letter-spacing: 0.01em;
}

.status-badge.pass { background: var(--pass-bg); color: var(--pass); }
.status-badge.fail { background: var(--fail-bg); color: var(--fail); }

.status-badge::before {
  content: "";
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: currentColor;
}

.summary-stats {
  margin-top: 20px;
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 24px;
}

.stat .label {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-subtle);
  margin-bottom: 4px;
}

.stat .value {
  font-size: 28px;
  font-weight: 600;
  letter-spacing: -0.02em;
  color: var(--text);
}

h2 {
  font-size: 14px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-subtle);
  font-weight: 600;
  margin: 40px 0 16px;
}

.policy {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  margin-bottom: 16px;
  overflow: hidden;
}

.policy-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 20px 24px;
  gap: 16px;
}

.policy.fail .policy-header {
  background: var(--fail-bg);
  border-bottom: 1px solid var(--border);
}

.policy.pass .policy-header {
  background: var(--pass-bg);
}

.policy-name {
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.policy-description {
  font-size: 13px;
  color: var(--text-muted);
  margin-top: 4px;
}

.policy-status {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  flex-shrink: 0;
}

.policy.pass .policy-status { color: var(--pass); }
.policy.fail .policy-status { color: var(--fail); }

.violations {
  list-style: none;
  margin: 0;
  padding: 0;
}

.violations li {
  padding: 12px 24px;
  border-top: 1px solid var(--border);
  font-size: 13px;
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 16px;
  align-items: baseline;
}

.violations li:first-child {
  border-top: none;
}

.violation-location {
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  color: var(--text);
  word-break: break-all;
}

.violation-location .col {
  color: var(--text-muted);
}

.violation-message {
  color: var(--text-muted);
  text-align: right;
  font-style: italic;
}

footer {
  margin-top: 64px;
  padding-top: 24px;
  border-top: 1px solid var(--border);
  font-size: 12px;
  color: var(--text-subtle);
  text-align: center;
}

footer a {
  color: var(--text-muted);
  text-decoration: none;
}

footer a:hover {
  text-decoration: underline;
}
"#;

/// Render a CheckReport as a self-contained HTML document.
pub fn render(report: &CheckReport) -> String {
    let timestamp = current_timestamp();
    let status_class = if report.passed() { "pass" } else { "fail" };
    let status_label = if report.passed() {
        "All policies passing"
    } else {
        "Policy violations"
    };
    let failing_policies = report.policies.iter().filter(|p| !p.passed()).count();

    let mut policy_html = String::new();
    for p in &report.policies {
        let p_class = if p.passed() { "pass" } else { "fail" };
        let status = if p.passed() {
            "Passing".to_string()
        } else {
            format!(
                "{} violation{}",
                p.violations.len(),
                if p.violations.len() == 1 { "" } else { "s" }
            )
        };

        policy_html.push_str(&format!(
            r#"<section class="policy {p_class}">
  <div class="policy-header">
    <div>
      <div class="policy-name">{name}</div>
      <div class="policy-description">{desc}</div>
    </div>
    <div class="policy-status">{status}</div>
  </div>"#,
            p_class = p_class,
            name = escape(p.name),
            desc = escape(p.description),
            status = status,
        ));

        if !p.violations.is_empty() {
            policy_html.push_str(r#"<ul class="violations">"#);
            for v in &p.violations {
                policy_html.push_str(&render_violation(v));
            }
            policy_html.push_str("</ul>");
        }

        policy_html.push_str("</section>");
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>dbt-fleet report — {date}</title>
  <style>{styles}</style>
</head>
<body>
  <div class="page">
    <header>
      <div class="brand">dbt-fleet<span class="version">v{version}</span></div>
      <div class="meta">{date}</div>
    </header>

    <div class="summary">
      <div class="summary-row">
        <span class="status-badge {status_class}">{status_label}</span>
      </div>
      <div class="summary-stats">
        <div class="stat">
          <div class="label">Tier-1 models</div>
          <div class="value">{tier1}</div>
        </div>
        <div class="stat">
          <div class="label">Policies</div>
          <div class="value">{policies}</div>
        </div>
        <div class="stat">
          <div class="label">Violations</div>
          <div class="value">{violations}</div>
        </div>
      </div>
    </div>

    <h2>Policy results{failing_note}</h2>
    {policy_html}

    <footer>
      Generated by <a href="https://github.com/dreynow/dbt-fleet">dbt-fleet</a> v{version}.
    </footer>
  </div>
</body>
</html>
"#,
        date = timestamp,
        styles = STYLES,
        version = report.version,
        status_class = status_class,
        status_label = status_label,
        tier1 = report.tier1_models,
        policies = report.policies.len(),
        violations = report.total_violations,
        failing_note = if failing_policies > 0 {
            format!(" ({} failing)", failing_policies)
        } else {
            String::new()
        },
        policy_html = policy_html,
    )
}

fn render_violation(v: &Violation) -> String {
    let location = match &v.column {
        Some(col) => format!(
            r#"<span class="violation-location">{file}<span class="col">:{col}</span></span>"#,
            file = escape(&v.file),
            col = escape(col)
        ),
        None => format!(
            r#"<span class="violation-location">{file}</span>"#,
            file = escape(&v.file)
        ),
    };
    format!(
        r#"<li>{location}<span class="violation-message">{message}</span></li>"#,
        location = location,
        message = escape(&v.message),
    )
}

/// Minimal HTML escaping for the fields we emit. Keep this simple — nothing
/// in a dbt manifest should contain script tags, but better to be safe with
/// model names like `weird&model<v2>` if someone has them.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Current UTC timestamp in `YYYY-MM-DD HH:MM UTC` format. We keep our own
/// formatter to avoid pulling in a date crate — overkill for one string.
fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_unix_utc(secs)
}

/// Convert a Unix timestamp to `YYYY-MM-DD HH:MM UTC`. Hand-rolled because
/// chrono is 200KB and we render one string per run.
fn format_unix_utc(mut secs: i64) -> String {
    let mut days_since_epoch = secs.div_euclid(86_400);
    secs = secs.rem_euclid(86_400);
    let hours = (secs / 3600) as u32;
    let minutes = ((secs % 3600) / 60) as u32;

    // Day-of-week not needed; compute year/month/day from days_since_epoch.
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
        "{:04}-{:02}-{:02} {:02}:{:02} UTC",
        year,
        month + 1,
        day,
        hours,
        minutes
    )
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::PolicyResult;

    fn sample_report() -> CheckReport {
        CheckReport {
            version: "0.0.2",
            tier1_models: 8,
            policies: vec![
                PolicyResult {
                    name: "tier_1_has_owner",
                    description: "Every tier-1 model must declare an owner.",
                    violations: vec![Violation {
                        model: "fct_revenue".into(),
                        file: "models/marts/fct_revenue.sql".into(),
                        column: None,
                        message: "meta.owner is missing or empty".into(),
                    }],
                },
                PolicyResult {
                    name: "tier_1_columns_described",
                    description: "Columns need descriptions.",
                    violations: vec![],
                },
            ],
            total_violations: 1,
        }
    }

    #[test]
    fn renders_complete_html_document() {
        let html = render(&sample_report());
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<title>"));
        assert!(html.ends_with("</html>\n"));
    }

    #[test]
    fn includes_summary_stats() {
        let html = render(&sample_report());
        assert!(html.contains("Tier-1 models"));
        assert!(html.contains(">8<")); // model count
        assert!(html.contains(">2<")); // policy count
        assert!(html.contains(">1<")); // violation count
    }

    #[test]
    fn renders_passing_and_failing_policies() {
        let html = render(&sample_report());
        assert!(html.contains("policy fail"));
        assert!(html.contains("policy pass"));
        assert!(html.contains("Passing"));
        assert!(html.contains("1 violation"));
    }

    #[test]
    fn includes_violation_locations() {
        let html = render(&sample_report());
        assert!(html.contains("models/marts/fct_revenue.sql"));
        assert!(html.contains("meta.owner is missing"));
    }

    #[test]
    fn escapes_html_special_chars() {
        let mut report = sample_report();
        report.policies[0].violations[0].file = "models/<weird>.sql".into();
        let html = render(&report);
        assert!(!html.contains("models/<weird>.sql"));
        assert!(html.contains("models/&lt;weird&gt;.sql"));
    }

    #[test]
    fn timestamp_format_is_correct() {
        // 2026-05-06 09:00 UTC → 1778058000 seconds since epoch
        let ts = format_unix_utc(1_778_058_000);
        assert_eq!(ts, "2026-05-06 09:00 UTC");
    }

    #[test]
    fn timestamp_handles_leap_year() {
        // 2024 is a leap year — feb 29 exists.
        // 2024-02-29 00:00 UTC = 1709164800
        let ts = format_unix_utc(1_709_164_800);
        assert_eq!(ts, "2024-02-29 00:00 UTC");
    }
}
