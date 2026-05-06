use clap::{Parser, Subcommand};

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
    /// Run governance policy checks against a dbt project. (Coming in v0.0.2.)
    Check {
        /// Path to the dbt project root. Defaults to current directory.
        #[arg(long, default_value = ".")]
        project: String,
    },
    /// Compute and persist a governance score for the project. (Coming in v0.0.4.)
    Score {
        #[arg(long, default_value = ".")]
        project: String,
    },
    /// Render the score history as a trend chart. (Coming in v0.0.4.)
    Trend {
        #[arg(long, default_value = ".")]
        project: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // No subcommand: print version + a one-line pitch and exit.
            println!("dbt-fleet {}", dbt_fleet::VERSION);
            println!("Governance scoring and trends for dbt projects.");
            println!();
            println!("Run `dbt-fleet --help` to see available commands.");
        }
        Some(Command::Check { project })
        | Some(Command::Score { project })
        | Some(Command::Trend { project }) => {
            eprintln!(
                "dbt-fleet {}: this command is not implemented yet.",
                dbt_fleet::VERSION
            );
            eprintln!("Project path was: {}", project);
            eprintln!();
            eprintln!("Track v0.1 progress at https://github.com/dreynow/dbt-fleet");
            std::process::exit(2);
        }
    }
}
