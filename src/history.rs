//! Persisted score history. Lives at `<project>/.dbt-fleet/history.json` by
//! default. Gitignored unless the user opts in.
//!
//! Format: a flat JSON array of `ScoreSnapshot` objects, oldest first. We
//! never rewrite history — `score` appends, `trend` reads. Manual edits are
//! the user's prerogative.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::score::ScoreSnapshot;

const HISTORY_DIR: &str = ".dbt-fleet";
const HISTORY_FILE: &str = "history.json";

pub struct History {
    pub snapshots: Vec<ScoreSnapshot>,
}

impl History {
    /// Load history from `<project>/.dbt-fleet/history.json`. Returns an
    /// empty history if the file doesn't exist (first run).
    pub fn load(project_dir: &Path) -> Result<Self> {
        let path = Self::path(project_dir);
        if !path.exists() {
            return Ok(Self { snapshots: vec![] });
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let snapshots: Vec<ScoreSnapshot> = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(Self { snapshots })
    }

    /// Append a snapshot and save. Creates `.dbt-fleet/` directory if missing.
    pub fn append(project_dir: &Path, snapshot: ScoreSnapshot) -> Result<Self> {
        let mut history = Self::load(project_dir)?;
        history.snapshots.push(snapshot);
        history.save(project_dir)?;
        Ok(history)
    }

    /// Persist the history to disk.
    pub fn save(&self, project_dir: &Path) -> Result<()> {
        let dir = project_dir.join(HISTORY_DIR);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create {}", dir.display()))?;
        let path = Self::path(project_dir);
        let json = serde_json::to_string_pretty(&self.snapshots)?;
        std::fs::write(&path, json)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }

    /// Replace the history with a new set of snapshots and persist.
    /// Used by `--demo` mode to seed synthetic history.
    pub fn replace(project_dir: &Path, snapshots: Vec<ScoreSnapshot>) -> Result<Self> {
        let history = Self { snapshots };
        history.save(project_dir)?;
        Ok(history)
    }

    fn path(project_dir: &Path) -> PathBuf {
        project_dir.join(HISTORY_DIR).join(HISTORY_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn empty_history_for_fresh_project() {
        let dir = tempfile::tempdir().unwrap();
        let h = History::load(dir.path()).unwrap();
        assert_eq!(h.snapshots.len(), 0);
    }

    #[test]
    fn append_and_reload_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let _ = History::append(dir.path(), snap("2026-04-01T00:00:00Z", 70.0)).unwrap();
        let _ = History::append(dir.path(), snap("2026-05-01T00:00:00Z", 85.0)).unwrap();
        let reloaded = History::load(dir.path()).unwrap();
        assert_eq!(reloaded.snapshots.len(), 2);
        assert_eq!(reloaded.snapshots[0].overall_pct, 70.0);
        assert_eq!(reloaded.snapshots[1].overall_pct, 85.0);
    }

    #[test]
    fn replace_overwrites_history() {
        let dir = tempfile::tempdir().unwrap();
        let _ = History::append(dir.path(), snap("2026-04-01T00:00:00Z", 70.0)).unwrap();
        History::replace(dir.path(), vec![snap("2026-01-01T00:00:00Z", 50.0)]).unwrap();
        let reloaded = History::load(dir.path()).unwrap();
        assert_eq!(reloaded.snapshots.len(), 1);
        assert_eq!(reloaded.snapshots[0].overall_pct, 50.0);
    }
}
