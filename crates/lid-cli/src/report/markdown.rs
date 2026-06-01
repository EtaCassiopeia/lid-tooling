//! Terminal-friendly grouped renderer.
//!
//! Findings are sorted (Error → Warning → Info), then by check ID,
//! then by location, and emitted under a per-severity header:
//!
//! ```text
//! ERRORS (2)
//!   src/auth.rs:42  [reverse-orphan]
//!     `@spec AUTH-999` references a spec ID that is not defined …
//!     → either add AUTH-999 to a spec file, delete the annotation, …
//!   docs/specs/auth-specs.md:12  [spec-id-format]
//!     duplicate spec ID `AUTH-001` (first defined at …)
//!
//! 2 finding(s)
//! ```
//!
//! Colour is applied via `owo-colors`'s `Style` so disabling it just
//! produces an empty style — no `if/else` smeared through the code.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use lid_core::{Finding, LidRepo, Severity};
use owo_colors::{OwoColorize, Style};

use super::RenderOptions;

#[derive(Default)]
struct Theme {
    error: Style,
    warning: Style,
    info: Style,
    check_id: Style,
    location: Style,
    remediation: Style,
}

impl Theme {
    fn from_options(opts: RenderOptions) -> Self {
        if opts.color {
            Self {
                error: Style::new().red().bold(),
                warning: Style::new().yellow().bold(),
                info: Style::new().blue().bold(),
                check_id: Style::new().dimmed(),
                location: Style::new().cyan(),
                remediation: Style::new().green(),
            }
        } else {
            Self::default()
        }
    }

    fn header(&self, severity: Severity) -> Style {
        match severity {
            Severity::Error => self.error,
            Severity::Warning => self.warning,
            Severity::Info => self.info,
        }
    }
}

/// Render `findings` produced against `repo` into a terminal-friendly
/// string. Empty input yields a one-line "no findings" message so the
/// CLI can always print *something*.
#[must_use]
pub fn render(repo: &LidRepo, findings: &[Finding], opts: RenderOptions) -> String {
    if findings.is_empty() {
        return "no findings\n".to_owned();
    }

    let theme = Theme::from_options(opts);

    let mut groups: BTreeMap<Severity, Vec<&Finding>> = BTreeMap::new();
    for f in findings {
        groups.entry(f.severity).or_default().push(f);
    }
    for items in groups.values_mut() {
        items.sort_by(|a, b| {
            a.check.cmp(&b.check).then_with(|| {
                let ap = a.location.as_ref().map(|l| (&l.path, l.line));
                let bp = b.location.as_ref().map(|l| (&l.path, l.line));
                ap.cmp(&bp)
            })
        });
    }

    let mut out = String::new();
    // Iterate Error → Warning → Info regardless of how the enum orders.
    for severity in [Severity::Error, Severity::Warning, Severity::Info] {
        let Some(items) = groups.get(&severity) else {
            continue;
        };
        let header = format!("{} ({})", severity_label(severity), items.len());
        let _ = writeln!(out, "{}", header.style(theme.header(severity)));
        for f in items {
            write_finding(&mut out, repo, f, &theme);
        }
        out.push('\n');
    }
    let _ = writeln!(out, "{} finding(s)", findings.len());
    out
}

fn severity_label(s: Severity) -> &'static str {
    match s {
        Severity::Error => "ERRORS",
        Severity::Warning => "WARNINGS",
        Severity::Info => "INFO",
    }
}

fn write_finding(out: &mut String, repo: &LidRepo, f: &Finding, theme: &Theme) {
    let loc = f.location.as_ref().map(|l| {
        let path = display_relative(repo, &l.path);
        match l.line {
            Some(n) => format!("{path}:{n}"),
            None => path,
        }
    });
    let check_id_str = format!("[{}]", check_id_label(f.check));
    match loc {
        Some(loc) => {
            let _ = writeln!(
                out,
                "  {}  {}",
                loc.style(theme.location),
                check_id_str.style(theme.check_id),
            );
        }
        None => {
            let _ = writeln!(out, "  {}", check_id_str.style(theme.check_id));
        }
    }
    let _ = writeln!(out, "    {}", f.message);
    if let Some(rem) = &f.remediation {
        let _ = writeln!(out, "    {}", format!("→ {rem}").style(theme.remediation));
    }
}

fn check_id_label(id: lid_core::CheckId) -> &'static str {
    // Routed through `CheckId::as_str` so this label stays in lockstep
    // with the JSON renderer and the eventual `--only` flag.
    id.as_str()
}

fn display_relative(repo: &LidRepo, path: &Path) -> String {
    path.strip_prefix(&repo.root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use lid_core::{ArrowIndex, Category, CheckId, Finding, Location, Severity, SpecId, Unmapped};

    use super::*;

    fn empty_repo() -> LidRepo {
        LidRepo {
            root: PathBuf::from("/repo"),
            index: ArrowIndex {
                schema_version: 1,
                last_updated: None,
                taxonomy: BTreeMap::new(),
                arrows: BTreeMap::new(),
                unmapped: Unmapped::default(),
            },
            specs: vec![],
            llds: vec![],
            arrow_docs: vec![],
            citations: vec![],
            decision_docs: vec![],
        }
    }

    fn finding(severity: Severity, check: CheckId, message: &str, line: Option<usize>) -> Finding {
        Finding {
            check,
            severity,
            category: Category::Schema,
            message: message.to_owned(),
            location: Some(Location {
                path: PathBuf::from("/repo/src/auth.rs"),
                line,
            }),
            spec: None,
            remediation: Some("do the thing".to_owned()),
        }
    }

    #[test]
    fn empty_findings_yields_no_findings_line() {
        let out = render(&empty_repo(), &[], RenderOptions { color: false });
        assert_eq!(out, "no findings\n");
    }

    #[test]
    fn renders_findings_grouped_by_severity() {
        let findings = vec![
            finding(Severity::Warning, CheckId::Orphan, "warn one", None),
            finding(Severity::Error, CheckId::ReverseOrphan, "err one", Some(42)),
            finding(Severity::Error, CheckId::Schema, "err two", None),
        ];
        let out = render(&empty_repo(), &findings, RenderOptions { color: false });
        // ERRORS header precedes WARNINGS header.
        let err_pos = out.find("ERRORS (2)").unwrap();
        let warn_pos = out.find("WARNINGS (1)").unwrap();
        assert!(err_pos < warn_pos, "got:\n{out}");
        assert!(out.contains("err one"));
        assert!(out.contains("err two"));
        assert!(out.contains("warn one"));
    }

    #[test]
    fn renders_repo_relative_paths() {
        let mut f = finding(Severity::Error, CheckId::Schema, "boom", Some(7));
        f.location = Some(Location {
            path: PathBuf::from("/repo/docs/arrows/auth.md"),
            line: Some(7),
        });
        let out = render(&empty_repo(), &[f], RenderOptions { color: false });
        assert!(out.contains("docs/arrows/auth.md:7"), "got:\n{out}");
        assert!(!out.contains("/repo/docs"), "expected repo prefix stripped");
    }

    #[test]
    fn no_color_output_has_no_ansi_escapes() {
        let findings = vec![finding(Severity::Error, CheckId::Schema, "boom", Some(1))];
        let out = render(&empty_repo(), &findings, RenderOptions { color: false });
        // ANSI escape sequences start with the escape byte 0x1B.
        assert!(!out.contains('\u{1b}'), "found ANSI escape: {out:?}");
    }

    #[test]
    fn color_output_contains_ansi_escapes() {
        let findings = vec![finding(Severity::Error, CheckId::Schema, "boom", Some(1))];
        let out = render(&empty_repo(), &findings, RenderOptions { color: true });
        assert!(out.contains('\u{1b}'), "expected ANSI escape, got:\n{out}");
    }

    #[test]
    fn includes_summary_line() {
        let findings = vec![
            finding(Severity::Error, CheckId::Schema, "a", None),
            finding(Severity::Warning, CheckId::Orphan, "b", None),
        ];
        let out = render(&empty_repo(), &findings, RenderOptions { color: false });
        assert!(out.contains("2 finding(s)"), "got:\n{out}");
    }

    #[test]
    fn check_id_is_rendered_in_kebab_case() {
        let findings = vec![finding(
            Severity::Error,
            CheckId::ReverseOrphan,
            "x",
            Some(1),
        )];
        let out = render(&empty_repo(), &findings, RenderOptions { color: false });
        assert!(out.contains("[reverse-orphan]"), "got:\n{out}");
    }

    #[test]
    fn remediation_arrow_appears_when_present() {
        let findings = vec![finding(Severity::Error, CheckId::Schema, "x", Some(1))];
        let out = render(&empty_repo(), &findings, RenderOptions { color: false });
        assert!(out.contains("→ do the thing"), "got:\n{out}");
    }

    #[test]
    fn finding_without_location_still_renders() {
        let mut f = finding(Severity::Warning, CheckId::Orphan, "no loc", None);
        f.location = None;
        let out = render(&empty_repo(), &[f], RenderOptions { color: false });
        assert!(out.contains("[orphan]"), "got:\n{out}");
        assert!(out.contains("no loc"));
    }

    #[test]
    fn finding_with_spec_id_does_not_break_rendering() {
        // Ensures the SpecId field, even when present, isn't required by
        // the renderer (today it's surfaced only via the message).
        let mut f = finding(Severity::Error, CheckId::ReverseOrphan, "x", Some(1));
        f.spec = Some(SpecId::parse("AUTH-001").unwrap());
        let out = render(&empty_repo(), &[f], RenderOptions { color: false });
        assert!(out.contains("[reverse-orphan]"));
    }
}
