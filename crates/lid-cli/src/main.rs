//! `lidc` — coherence checker for the LID methodology.
//!
//! Subcommands and renderers will accrete over the next few commits.
//! Today the binary supports one operation: discover a LID repo and run
//! the default check registry against it, printing each finding in a
//! plain `severity check.id message` form. Markdown / JSON renderers
//! and the `--only` / `--fail-on` flags land in subsequent commits.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use lid_core::{LidRepo, checks};

#[derive(Parser)]
#[command(name = "lidc", version, about = "LID coherence checker")]
struct Cli {
    /// Directory to start discovery from. Defaults to the current
    /// directory; the search walks upward until `docs/arrows/index.yaml`
    /// is found.
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the coherence checks against a LID repository.
    Check(CheckArgs),
}

#[derive(Args)]
struct CheckArgs {}

fn main() -> ExitCode {
    match run() {
        Ok(exit) => exit,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let Cmd::Check(_) = cli.cmd;
    cmd_check(cli.root.as_deref())
}

fn cmd_check(root: Option<&Path>) -> Result<ExitCode> {
    let start = match root {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().context("reading the current directory")?,
    };

    let repo = LidRepo::discover(&start)
        .with_context(|| format!("discovering a LID repo at or above {}", start.display()))?;

    let mut findings = Vec::new();
    for check in checks::default_checks() {
        findings.extend(check.run(&repo));
    }

    if findings.is_empty() {
        println!("✓ no findings");
        return Ok(ExitCode::SUCCESS);
    }

    for f in &findings {
        let loc = f
            .location
            .as_ref()
            .map(|l| {
                let path = display_relative(&repo, &l.path);
                match l.line {
                    Some(line) => format!("{path}:{line}"),
                    None => path,
                }
            })
            .unwrap_or_default();
        let prefix = if loc.is_empty() {
            String::new()
        } else {
            format!("{loc}: ")
        };
        println!("{:?} [{:?}] {prefix}{}", f.severity, f.check, f.message);
    }
    println!();
    println!("{} finding(s)", findings.len());

    Ok(ExitCode::from(1))
}

fn display_relative(repo: &LidRepo, path: &Path) -> String {
    path.strip_prefix(&repo.root)
        .unwrap_or(path)
        .display()
        .to_string()
}
