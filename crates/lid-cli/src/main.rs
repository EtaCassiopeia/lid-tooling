//! `lidc` — coherence checker for the LID methodology.
//!
//! Today the binary supports one operation: discover a LID repo and run
//! the default check registry against it, printing findings via the
//! markdown renderer (default) or the JSON renderer (`--json`). The
//! `--only` / `--fail-on` flags land in subsequent commits.

mod report;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use lid_core::{LidRepo, checks};

use crate::report::{RenderOptions, json, markdown};

#[derive(Parser)]
#[command(name = "lidc", version, about = "LID coherence checker")]
struct Cli {
    /// Directory to start discovery from. Defaults to the current
    /// directory; the search walks upward until `docs/arrows/index.yaml`
    /// is found.
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// Emit findings as a stable JSON document instead of the
    /// terminal-friendly grouped format.
    #[arg(long, global = true)]
    json: bool,

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
    cmd_check(cli.root.as_deref(), cli.json)
}

fn cmd_check(root: Option<&Path>, as_json: bool) -> Result<ExitCode> {
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

    if as_json {
        let rendered = json::render(&findings).context("rendering JSON report")?;
        println!("{rendered}");
    } else {
        let opts = RenderOptions::from_stdout();
        let rendered = markdown::render(&repo, &findings, opts);
        print!("{rendered}");
    }

    if findings.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}
