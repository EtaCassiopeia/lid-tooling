//! `lidc` — coherence checker for the LID methodology.
//!
//! Today the binary supports one operation: `lidc check` discovers a
//! LID repo, runs the registered checks (filtered by `--only`) against
//! it, prints findings via the markdown renderer (default) or the JSON
//! renderer (`--json`), and exits with a code that respects `--fail-on`.

mod report;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand};
use lid_core::{CheckId, LidError, LidRepo, Severity, checks};

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
struct CheckArgs {
    /// Only run the listed checks (comma-separated, kebab-case names).
    ///
    /// Example: `--only schema,reference-coherence`. The set of valid
    /// names matches `CheckId::as_str()` in `lid-core`.
    #[arg(long, value_delimiter = ',')]
    only: Vec<String>,

    /// Severity threshold for non-zero exit. The run "fails" (exit 1)
    /// when at least one finding has severity >= this value.
    ///
    /// One of `error` (default), `warning`, or `info`.
    #[arg(long, default_value = "error")]
    fail_on: String,
}

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
    let Cmd::Check(args) = &cli.cmd;
    cmd_check(cli.root.as_deref(), cli.json, args)
}

fn cmd_check(root: Option<&Path>, as_json: bool, args: &CheckArgs) -> Result<ExitCode> {
    let only = parse_only(&args.only)?;
    let fail_threshold: Severity = args
        .fail_on
        .parse()
        .map_err(|e: String| anyhow!("invalid --fail-on value: {e}"))?;

    let start = match root {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().context("reading the current directory")?,
    };

    let repo = LidRepo::discover(&start).map_err(|e| match e {
        e @ LidError::UnsupportedSchemaVersion { .. } => anyhow::Error::from(e),
        other => anyhow::Error::from(other).context(format!(
            "discovering a LID repo at or above {}",
            start.display()
        )),
    })?;

    let mut findings = Vec::new();
    for check in checks::default_checks() {
        if let Some(filter) = &only {
            if !filter.contains(&check.id()) {
                continue;
            }
        }
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

    let any_at_or_above = findings.iter().any(|f| f.severity >= fail_threshold);
    Ok(if any_at_or_above {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn parse_only(values: &[String]) -> Result<Option<BTreeSet<CheckId>>> {
    if values.is_empty() {
        return Ok(None);
    }
    let mut set = BTreeSet::new();
    for raw in values {
        let id: CheckId = raw
            .parse()
            .map_err(|e: String| anyhow!("invalid --only value: {e}"))?;
        set.insert(id);
    }
    Ok(Some(set))
}
