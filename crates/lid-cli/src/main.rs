//! `lidc` — coherence checker for the LID methodology.
//!
//! Subcommands:
//! * `lidc init`   — scaffold a new LID project (index.yaml + stub arrow doc).
//! * `lidc check`  — discover a LID repo, run the registered checks, report
//!   findings, exit 1 when severity crosses `--fail-on` threshold.
//! * `lidc status` — print a quick health dashboard (segment counts, spec
//!   coverage, drift/next counts); always exits 0.

mod report;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand};
use lid_core::model::SpecStatus;
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
    /// Scaffold a new LID project at the current (or --root) directory.
    ///
    /// Creates docs/arrows/index.yaml, a stub arrow document, and the
    /// docs/intent/ directory. Fails if a LID project already exists there.
    Init(InitArgs),
    /// Run the coherence checks against a LID repository.
    Check(CheckArgs),
    /// Print a health dashboard: segment counts, spec coverage, drift/next.
    Status,
}

#[derive(Args)]
struct InitArgs {
    /// Name of the first segment to create (default: "core").
    #[arg(long, default_value = "core")]
    segment: String,
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
    match &cli.cmd {
        Cmd::Init(args) => cmd_init(cli.root.as_deref(), args),
        Cmd::Check(args) => cmd_check(cli.root.as_deref(), cli.json, args),
        Cmd::Status => cmd_status(cli.root.as_deref(), cli.json),
    }
}

fn cmd_init(root: Option<&Path>, args: &InitArgs) -> Result<ExitCode> {
    let dir = match root {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().context("reading the current directory")?,
    };

    let index_path = dir.join("docs").join("arrows").join("index.yaml");
    if index_path.exists() {
        anyhow::bail!(
            "docs/arrows/index.yaml already exists; {} looks like an existing LID project",
            dir.display()
        );
    }

    let seg = &args.segment;
    let arrows_seg_dir = dir.join("docs").join("arrows").join(seg);
    let intent_dir = dir.join("docs").join("intent");

    fs::create_dir_all(&arrows_seg_dir).with_context(|| format!("creating docs/arrows/{seg}/"))?;
    fs::create_dir_all(&intent_dir).context("creating docs/intent/")?;

    fs::write(
        &index_path,
        format!("schema_version: 2\narrows:\n  {seg}:\n    status: UNMAPPED\n    detail: {seg}/overview.md\n"),
    )
    .context("writing docs/arrows/index.yaml")?;

    fs::write(
        arrows_seg_dir.join("overview.md"),
        format!(
            "# {seg}\n\n\
             ## Overview\n\n\
             <!-- Describe the {seg} segment here. -->\n\n\
             ## References\n\n\
             <!-- List related documents and spec files here. -->\n"
        ),
    )
    .with_context(|| format!("writing docs/arrows/{seg}/overview.md"))?;

    println!("Initialized LID project at {}", dir.display());
    println!();
    println!("Created:");
    println!("  docs/arrows/index.yaml            schema v2, segment '{seg}'");
    println!("  docs/arrows/{seg}/overview.md     stub arrow document");
    println!("  docs/intent/                      home for spec files (e.g. {seg}-specs.md)");
    println!();
    println!("Next:");
    println!("  lidc check                        verify coherence");
    println!("  lidc status                       segment and spec summary");

    Ok(ExitCode::SUCCESS)
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

fn cmd_status(root: Option<&Path>, as_json: bool) -> Result<ExitCode> {
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

    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    for seg in repo.index.arrows.values() {
        let key = format!("{:?}", seg.status).to_uppercase();
        *by_status.entry(key).or_default() += 1;
    }
    let total_segments = repo.index.arrows.len();

    let all_specs: Vec<_> = repo.specs.iter().flat_map(|f| f.specs.iter()).collect();
    let total_specs = all_specs.len();
    let implemented = all_specs
        .iter()
        .filter(|s| s.status == SpecStatus::Implemented)
        .count();
    let open = all_specs
        .iter()
        .filter(|s| s.status == SpecStatus::Open)
        .count();
    let deferred = all_specs
        .iter()
        .filter(|s| s.status == SpecStatus::Deferred)
        .count();

    let drift_count = repo
        .index
        .arrows
        .values()
        .filter(|s| s.drift.is_some())
        .count();
    let next_count = repo
        .index
        .arrows
        .values()
        .filter(|s| s.next.is_some())
        .count();

    if as_json {
        let json = serde_json::json!({
            "root": repo.root,
            "schema_version": repo.index.schema_version,
            "segments": {
                "total": total_segments,
                "by_status": by_status,
            },
            "specs": {
                "total": total_specs,
                "implemented": implemented,
                "open": open,
                "deferred": deferred,
            },
            "drift_count": drift_count,
            "next_count": next_count,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&json).context("serialising status JSON")?
        );
    } else {
        println!(
            "LID repo: {}   schema v{}",
            repo.root.display(),
            repo.index.schema_version
        );
        println!();
        println!("Segments   {total_segments} total");
        for (status, count) in &by_status {
            println!("  {status:<12} {count}");
        }
        println!();
        let pct = (implemented * 100).checked_div(total_specs).unwrap_or(0);
        println!("Specs      {total_specs} total");
        println!(
            "  [x] {implemented} implemented   [ ] {open} open   [D] {deferred} deferred   ({pct} % covered)"
        );
        println!();
        println!(
            "Drift       {drift_count} segment{} {} drift notes",
            if drift_count == 1 { "" } else { "s" },
            if drift_count == 1 { "has" } else { "have" }
        );
        println!(
            "Next        {next_count} segment{} {} active next-work entries",
            if next_count == 1 { "" } else { "s" },
            if next_count == 1 { "has" } else { "have" }
        );
        println!();
        println!("Run `lidc check` for full coherence findings.");
    }

    Ok(ExitCode::SUCCESS)
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
