//! tablint: an opinionated linter for Simple English prose, code comments, and decision records.

mod comments;
mod config;
mod constructs;
mod finding;
mod markdown;
mod repo;
mod rules;
mod simple;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a repository and print every finding.
    Check {
        /// The repository root.
        #[arg(default_value = ".")]
        root: PathBuf,
        /// A config file other than `<root>/tablint.toml`.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Print every rule ID and what it enforces.
    Rules,
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Rules => {
            for (id, what) in rules::RULES {
                println!("{id:<28} {what}");
            }
            ExitCode::SUCCESS
        }
        Command::Check { root, config } => match check(&root, config.as_deref()) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::from(1),
            Err(e) => {
                eprintln!("✗ {e:#}");
                ExitCode::from(2)
            }
        },
    }
}

/// Prints the findings, and returns whether the repository passed.
fn check(root: &Path, config: Option<&Path>) -> anyhow::Result<bool> {
    let cfg = config::Config::load(root, config)?;
    let repo = repo::Repo::open(root)?;
    let found = rules::run(&repo, &cfg)?;
    if found.is_empty() {
        println!("✓ {} files follow the rules", repo.files.len());
        return Ok(true);
    }
    for f in &found {
        println!("{f}");
    }
    let files = found
        .iter()
        .map(|f| f.path.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    eprintln!(
        "✗ {} findings in {files} files. `tablint rules` explains each rule",
        found.len()
    );
    Ok(false)
}
