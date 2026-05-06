//! ASCII trend chart for the terminal. Three bars: ownership %, description
//! %, overall. Each bar shows ~10 sampled snapshots from the history.

use std::fmt::Write as FmtWrite;

use crate::history::History;

const BAR_WIDTH: usize = 24;
const SAMPLE_COUNT: usize = 10;

/// Render the trend as a multi-line string ready for `println!`.
pub fn render(history: &History) -> String {
    let mut out = String::new();
    if history.snapshots.is_empty() {
        out.push_str("No history yet. Run `dbt-fleet score` to record the first snapshot.\n");
        return out;
    }

    if history.snapshots.len() == 1 {
        let s = &history.snapshots[0];
        let _ = writeln!(out, "Only one snapshot recorded so far.");
        let _ = writeln!(out);
        let _ = writeln!(out, "  {}: overall {:.1}%", s.timestamp, s.overall_pct);
        let _ = writeln!(out, "Run `dbt-fleet score` again later to start the trend.");
        return out;
    }

    let samples = sample(&history.snapshots, SAMPLE_COUNT);
    let first = samples.first().unwrap();
    let last = samples.last().unwrap();

    let _ = writeln!(
        out,
        "dbt-fleet trend ({} snapshots, sampled to {})",
        history.snapshots.len(),
        samples.len()
    );
    let _ = writeln!(out);

    write_series(&mut out, "Overall", &samples, |s| s.overall_pct);
    let _ = writeln!(out);
    write_series(&mut out, "Ownership", &samples, |s| s.ownership_pct);
    let _ = writeln!(out);
    write_series(&mut out, "Descriptions", &samples, |s| s.description_pct);

    let _ = writeln!(out);
    let delta = last.overall_pct - first.overall_pct;
    let arrow = if delta > 0.5 {
        "\u{2191}"
    } else if delta < -0.5 {
        "\u{2193}"
    } else {
        "\u{2192}"
    };
    let _ = writeln!(
        out,
        "Overall: {} {:.1}% \u{2192} {:.1}% ({:+.1}pp)",
        arrow, first.overall_pct, last.overall_pct, delta,
    );
    out
}

fn write_series<F>(out: &mut String, label: &str, samples: &[&crate::score::ScoreSnapshot], f: F)
where
    F: Fn(&crate::score::ScoreSnapshot) -> f64,
{
    let _ = writeln!(out, "  {}:", label);
    for s in samples {
        let pct = f(s).clamp(0.0, 100.0);
        let bar_len = ((pct / 100.0) * BAR_WIDTH as f64).round() as usize;
        let bar: String = "\u{2588}".repeat(bar_len) + &"\u{2591}".repeat(BAR_WIDTH - bar_len);
        let date = short_date(&s.timestamp);
        let _ = writeln!(out, "    {} {} {:>5.1}%", date, bar, pct);
    }
}

/// Sample at most N snapshots evenly across the history. Always includes
/// the first and last so the trend's endpoints are honest.
fn sample<T>(items: &[T], n: usize) -> Vec<&T> {
    if items.len() <= n {
        return items.iter().collect();
    }
    let step = (items.len() - 1) as f64 / (n - 1) as f64;
    (0..n)
        .map(|i| &items[(i as f64 * step).round() as usize])
        .collect()
}

/// Extract `MMM DD` from an ISO-8601 timestamp like `2026-05-06T11:30:00Z`.
fn short_date(iso: &str) -> String {
    if iso.len() < 10 {
        return iso.to_string();
    }
    let month: u32 = iso[5..7].parse().unwrap_or(1);
    let day: u32 = iso[8..10].parse().unwrap_or(1);
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{} {:02}",
        months[(month as usize).saturating_sub(1).min(11)],
        day
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::ScoreSnapshot;

    fn snap(ts: &str, pct: f64) -> ScoreSnapshot {
        ScoreSnapshot {
            timestamp: ts.into(),
            tier1_models: 5,
            total_columns: 10,
            ownership_pct: pct,
            description_pct: pct,
            overall_pct: pct,
            total_violations: 0,
        }
    }

    #[test]
    fn empty_history_says_no_history() {
        let h = History { snapshots: vec![] };
        let out = render(&h);
        assert!(out.contains("No history yet"));
    }

    #[test]
    fn single_snapshot_shows_one_off() {
        let h = History {
            snapshots: vec![snap("2026-05-06T00:00:00Z", 75.0)],
        };
        let out = render(&h);
        assert!(out.contains("Only one snapshot"));
        assert!(out.contains("75.0%"));
    }

    #[test]
    fn renders_bars_for_multiple_snapshots() {
        let snapshots = (0..5)
            .map(|i| {
                snap(
                    &format!("2026-0{}-01T00:00:00Z", i + 1),
                    60.0 + i as f64 * 5.0,
                )
            })
            .collect();
        let h = History { snapshots };
        let out = render(&h);
        assert!(out.contains("Overall"));
        assert!(out.contains("Ownership"));
        assert!(out.contains("Descriptions"));
        // Trend summary line should show the +pp delta.
        assert!(out.contains("+20.0pp"));
        // Bar character should appear.
        assert!(out.contains("\u{2588}"));
    }

    #[test]
    fn sample_picks_first_and_last() {
        let items: Vec<i32> = (0..100).collect();
        let sampled = sample(&items, 10);
        assert_eq!(sampled.len(), 10);
        assert_eq!(*sampled[0], 0);
        assert_eq!(*sampled[9], 99);
    }

    #[test]
    fn short_date_extracts_month_day() {
        assert_eq!(short_date("2026-05-06T11:30:00Z"), "May 06");
        assert_eq!(short_date("2024-12-31T23:59:59Z"), "Dec 31");
    }
}
