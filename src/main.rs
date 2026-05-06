use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dbt_fleet::check::CheckReport;

#[derive(Parser)]
#[command(
    name = "dbt-fleet",
    version,
    about = "Governance scoring and trends for dbt projects.",
    long_about = "dbt-fleet reads your dbt project's manifest.json and answers three questions:\n\
                  \n\
                  1. Are the models that matter documented and owned?\n\
                  2. Does this PR break anything downstream?\n\
                  3. Are we getting better or worse over time?\n\
                  \n\
                  Output: a single self-contained HTML report. No database. No SaaS account."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run governance policy checks against a dbt project.
    Check {
        /// Path to the dbt project root. Defaults to current directory.
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
        /// Write the report to a file instead of stdout. Required for --format html.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Compute a governance score and append it to .dbt-fleet/history.json.
    Score {
        /// Path to the dbt project root. Defaults to current directory.
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Render the score history as a trend chart in the terminal.
    Trend {
        /// Path to the dbt project root. Defaults to current directory.
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Replace history with 90 days of synthesized snapshots. Useful for
        /// README screenshots and launch posts before real history exists.
        #[arg(long)]
        demo: bool,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
    Html,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // No subcommand: print version + a one-line pitch and exit.
            println!("dbt-fleet {}", dbt_fleet::VERSION);
            println!("Governance scoring and trends for dbt projects.");
            println!();
            println!("Run `dbt-fleet --help` to see available commands.");
            ExitCode::SUCCESS
        }
        Some(Command::Check {
            project,
            format,
            output,
        }) => match dbt_fleet::check::run(&project) {
            Ok(report) => {
                if let Err(e) = emit(&report, format, output.as_deref()) {
                    eprintln!("Failed to write output: {:#}", e);
                    return ExitCode::from(2);
                }
                if report.passed() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(e) => {
                eprintln!("dbt-fleet check failed: {:#}", e);
                ExitCode::from(2)
            }
        },
        Some(Command::Score { project }) => match run_score(&project) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("dbt-fleet score failed: {:#}", e);
                ExitCode::from(2)
            }
        },
        Some(Command::Trend { project, demo }) => match run_trend(&project, demo) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("dbt-fleet trend failed: {:#}", e);
                ExitCode::from(2)
            }
        },
    }
}

fn run_score(project: &Path) -> Result<()> {
    let report = dbt_fleet::check::run(project)?;
    let manifest = dbt_fleet::manifest::Manifest::find(project)?;
    let tiers = dbt_fleet::tier::TierConfig::load(project)?;
    let snapshot = dbt_fleet::score::ScoreSnapshot::compute(&report, &manifest, &tiers);
    let history = dbt_fleet::history::History::append(project, snapshot.clone())?;

    println!("dbt-fleet {}", dbt_fleet::VERSION);
    println!();
    println!(
        "Snapshot recorded ({} total in history):",
        history.snapshots.len()
    );
    println!("  Timestamp:   {}", snapshot.timestamp);
    println!("  Overall:     {:.1}%", snapshot.overall_pct);
    println!(
        "  Ownership:   {:.1}% ({} tier-1 models)",
        snapshot.ownership_pct, snapshot.tier1_models
    );
    println!(
        "  Descriptions: {:.1}% ({} columns)",
        snapshot.description_pct, snapshot.total_columns
    );
    Ok(())
}

fn run_trend(project: &Path, demo: bool) -> Result<()> {
    if demo {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let synthetic = dbt_fleet::demo::synthesize(now, 90);
        dbt_fleet::history::History::replace(project, synthetic)?;
        eprintln!("Replaced history with 90 days of synthesized snapshots.");
        eprintln!();
    }
    let history = dbt_fleet::history::History::load(project)?;
    print!("{}", dbt_fleet::render::trend::render(&history));
    Ok(())
}

fn emit(report: &CheckReport, format: OutputFormat, output: Option<&Path>) -> Result<()> {
    let rendered: String = match format {
        OutputFormat::Human => {
            // Human format prints to stdout directly with its own formatter.
            // If --output was set, capture into a string instead.
            if output.is_some() {
                let mut buf = String::new();
                dbt_fleet::check::write_human(report, &mut buf)?;
                buf
            } else {
                dbt_fleet::check::print_human(report);
                return Ok(());
            }
        }
        OutputFormat::Json => serde_json::to_string_pretty(report)?,
        OutputFormat::Html => dbt_fleet::render::html::render(report),
    };

    match output {
        Some(path) => {
            std::fs::write(path, rendered.as_bytes())?;
            eprintln!("Wrote report to {}", path.display());
        }
        None => print!("{}", rendered),
    }
    Ok(())
}
