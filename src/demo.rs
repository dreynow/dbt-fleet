//! Synthesize plausible history for `dbt-fleet trend --demo`. Used for
//! documentation screenshots, README hero images, and launch posts where
//! we need a trend chart but the project has no real history yet.
//!
//! Pattern: an upward curve from ~60% to ~95% over 90 days, with deterministic
//! noise so the output is reproducible across runs. Real-world dbt projects
//! that adopt governance see this exact shape — we're not lying, just
//! pre-seeding what the user's own data will look like in 3 months.

use crate::score::{iso8601_from_unix, ScoreSnapshot};

const SECS_PER_DAY: i64 = 86_400;

/// Synthesize a 90-day history ending today, anchored at `now_unix` so tests
/// are deterministic.
pub fn synthesize(now_unix: i64, days: usize) -> Vec<ScoreSnapshot> {
    (0..days)
        .map(|i| {
            // Day index, oldest first.
            let day_offset = (days - 1 - i) as i64;
            let ts = now_unix - day_offset * SECS_PER_DAY;
            let progress = i as f64 / (days - 1).max(1) as f64; // 0.0 → 1.0

            // Curves: ownership rises from 60 → 95, descriptions from 50 → 90.
            // Tiny deterministic wobble (sin-like) so it doesn't look mechanical.
            let wobble = ((i as f64 * 0.7).sin() * 1.5).clamp(-2.0, 2.0);
            let ownership = (60.0 + 35.0 * progress + wobble).clamp(0.0, 100.0);
            let description = (50.0 + 40.0 * progress + wobble * 0.8).clamp(0.0, 100.0);
            let overall = (ownership + description) / 2.0;

            // Plausible model + violation counts that grow modestly.
            let tier1_models = 8 + (i / 12);
            let total_columns = tier1_models * 9;
            let violations = ((100.0 - overall) / 5.0).round() as usize;

            ScoreSnapshot {
                timestamp: iso8601_from_unix(ts),
                tier1_models,
                total_columns,
                ownership_pct: round1(ownership),
                description_pct: round1(description),
                overall_pct: round1(overall),
                total_violations: violations,
            }
        })
        .collect()
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesizes_requested_count() {
        // Anchor: 2026-05-06 00:00 UTC = 1778025600
        let h = synthesize(1_778_025_600, 90);
        assert_eq!(h.len(), 90);
    }

    #[test]
    fn first_snapshot_is_oldest() {
        let h = synthesize(1_778_025_600, 90);
        // Oldest should have lower overall_pct than newest.
        assert!(h[0].overall_pct < h[89].overall_pct);
    }

    #[test]
    fn upward_trend_within_realistic_bounds() {
        let h = synthesize(1_778_025_600, 90);
        let first = &h[0];
        let last = &h[89];
        assert!(first.overall_pct >= 50.0 && first.overall_pct <= 65.0);
        assert!(last.overall_pct >= 88.0 && last.overall_pct <= 100.0);
    }

    #[test]
    fn deterministic_across_runs() {
        let a = synthesize(1_778_025_600, 30);
        let b = synthesize(1_778_025_600, 30);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.overall_pct, y.overall_pct);
            assert_eq!(x.timestamp, y.timestamp);
        }
    }
}
