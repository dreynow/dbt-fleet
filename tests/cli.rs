//! Smoke tests — verify the binary runs and basic flags work on every
//! supported platform. Real behaviour tests land alongside each feature.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn prints_version_with_v_flag() {
    Command::cargo_bin("dbt-fleet")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn prints_help_with_h_flag() {
    // `--help` shows the long_about; `-h` shows the short `about`.
    // Assert on text present in the long form so we exercise the real flag.
    Command::cargo_bin("dbt-fleet")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("manifest.json"))
        .stdout(predicate::str::contains("Usage: dbt-fleet"));
}

#[test]
fn prints_short_about_with_short_h_flag() {
    Command::cargo_bin("dbt-fleet")
        .unwrap()
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("Governance scoring"));
}

#[test]
fn no_args_prints_pitch() {
    Command::cargo_bin("dbt-fleet")
        .unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Governance scoring"));
}

#[test]
fn check_without_manifest_emits_helpful_error() {
    let tempdir = tempfile::tempdir().unwrap();
    Command::cargo_bin("dbt-fleet")
        .unwrap()
        .arg("check")
        .arg("--project")
        .arg(tempdir.path())
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("No manifest.json found"))
        .stderr(predicate::str::contains("dbt parse"));
}

#[test]
fn score_without_manifest_emits_helpful_error() {
    let tempdir = tempfile::tempdir().unwrap();
    Command::cargo_bin("dbt-fleet")
        .unwrap()
        .arg("score")
        .arg("--project")
        .arg(tempdir.path())
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("No manifest.json found"));
}

#[test]
fn trend_with_no_history_says_so() {
    let tempdir = tempfile::tempdir().unwrap();
    Command::cargo_bin("dbt-fleet")
        .unwrap()
        .arg("trend")
        .arg("--project")
        .arg(tempdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No history yet"));
}

#[test]
fn trend_demo_seeds_synthetic_history() {
    let tempdir = tempfile::tempdir().unwrap();
    Command::cargo_bin("dbt-fleet")
        .unwrap()
        .arg("trend")
        .arg("--project")
        .arg(tempdir.path())
        .arg("--demo")
        .assert()
        .success()
        .stdout(predicate::str::contains("Overall"))
        .stdout(predicate::str::contains("Ownership"))
        .stdout(predicate::str::contains("\u{2588}")); // bar character
}
