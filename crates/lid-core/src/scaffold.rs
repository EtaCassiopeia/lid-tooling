use std::path::{Path, PathBuf};

/// Suggest a spec-ID prefix for a new segment.
///
/// Strategy:
/// 1. Collect the first hyphen-delimited component of every existing prefix
///    (e.g. `USH` from `USH-AUTH`).
/// 2. If one namespace accounts for a strict majority of the existing
///    prefixes, return `{NAMESPACE}-{SEGMENT_UPPER}`.
/// 3. Otherwise return `{SEGMENT_UPPER}` (uppercase segment name).
///
/// This lets a project with a consistent namespace like `USH` auto-suggest
/// `USH-BILLING` for a new `billing` segment, while a mixed or empty project
/// simply uppercases the name.
#[must_use]
pub fn suggest_prefix(segment_id: &str, existing_prefixes: &[&str]) -> String {
    let segment_upper = segment_id.to_uppercase();

    if existing_prefixes.is_empty() {
        return segment_upper;
    }

    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for p in existing_prefixes {
        if let Some(ns) = p.split('-').next() {
            if !ns.is_empty() {
                *counts.entry(ns).or_insert(0) += 1;
            }
        }
    }

    if let Some((ns, &cnt)) = counts.iter().max_by_key(|(_, c)| *c) {
        if cnt * 2 > existing_prefixes.len() {
            return format!("{ns}-{segment_upper}");
        }
    }

    segment_upper
}

/// Create the intent-directory scaffold for a new segment.
///
/// Creates:
/// - `docs/intent/{seg}/`
/// - `docs/intent/{seg}/{seg}-specs.md`  — empty spec list with `prefix:` frontmatter
/// - `docs/intent/{seg}/{seg}-design.md` — minimal design stub
///
/// Returns the paths of the two files written.
///
/// # Errors
///
/// Returns an I/O error if any directory or file operation fails.
pub fn scaffold_intent_dir(
    root: &Path,
    segment_id: &str,
    spec_prefix: &str,
) -> Result<[PathBuf; 2], std::io::Error> {
    let intent_dir = root.join("docs").join("intent").join(segment_id);
    std::fs::create_dir_all(&intent_dir)?;

    let specs_path = intent_dir.join(format!("{segment_id}-specs.md"));
    std::fs::write(
        &specs_path,
        format!("---\nprefix: {spec_prefix}\n---\n\n# {segment_id} specs\n"),
    )?;

    let design_path = intent_dir.join(format!("{segment_id}-design.md"));
    std::fs::write(
        &design_path,
        format!(
            "# {segment_id} design\n\n\
             ## Overview\n\n\
             <!-- Describe the design for {segment_id} here. -->\n\n\
             ## Decisions\n"
        ),
    )?;

    Ok([specs_path, design_path])
}

/// Create a stub arrow doc at `docs/arrows/{detail}` if it does not already exist.
///
/// The stub includes `## References` bullets pointing at the intent files so
/// that `lidc check` does not flag the design doc as an orphan.
///
/// Returns `Some(path)` when the file was created, `None` when it already existed.
///
/// # Errors
///
/// Returns an I/O error if any directory or file operation fails.
pub fn scaffold_arrow_doc(
    root: &Path,
    detail: &str,
    segment_id: &str,
) -> Result<Option<PathBuf>, std::io::Error> {
    let arrow_path = root.join("docs").join("arrows").join(detail);
    if arrow_path.exists() {
        return Ok(None);
    }
    if let Some(parent) = arrow_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let title = segment_id.replace('-', " ");
    std::fs::write(
        &arrow_path,
        format!(
            "# {title}\n\n\
             ## Overview\n\n\
             <!-- Describe the {segment_id} segment here. -->\n\n\
             ## References\n\n\
             ### LLD\n\
             - `docs/intent/{segment_id}/{segment_id}-design.md`\n\n\
             ### EARS\n\
             - `docs/intent/{segment_id}/{segment_id}-specs.md`\n"
        ),
    )?;
    Ok(Some(arrow_path))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn suggest_prefix_no_existing_returns_segment_upper() {
        assert_eq!(suggest_prefix("auth", &[]), "AUTH");
        assert_eq!(suggest_prefix("store-interface", &[]), "STORE-INTERFACE");
    }

    #[test]
    fn suggest_prefix_majority_namespace_prepended() {
        let existing = ["USH-AUTH", "USH-API", "USH-ANLY", "USH-CORE"];
        assert_eq!(suggest_prefix("billing", &existing), "USH-BILLING");
        assert_eq!(
            suggest_prefix("rate-limiter", &existing),
            "USH-RATE-LIMITER"
        );
    }

    #[test]
    fn suggest_prefix_no_majority_falls_back_to_upper() {
        let existing = ["USH-AUTH", "FOO-BAR", "BAZ-QUX"];
        assert_eq!(suggest_prefix("billing", &existing), "BILLING");
    }

    #[test]
    fn suggest_prefix_single_existing_uses_its_namespace() {
        assert_eq!(suggest_prefix("payments", &["USH-AUTH"]), "USH-PAYMENTS");
    }

    #[test]
    fn scaffold_intent_dir_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let [specs, design] = scaffold_intent_dir(dir.path(), "auth", "USH-AUTH").unwrap();
        assert!(specs.exists());
        assert!(design.exists());
        let specs_content = std::fs::read_to_string(&specs).unwrap();
        assert!(specs_content.contains("prefix: USH-AUTH"));
        let design_content = std::fs::read_to_string(&design).unwrap();
        assert!(design_content.contains("## Overview"));
    }

    #[test]
    fn scaffold_arrow_doc_creates_file_with_references() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("docs/arrows")).unwrap();
        let created = scaffold_arrow_doc(dir.path(), "auth/overview.md", "auth").unwrap();
        assert!(created.is_some());
        let content = std::fs::read_to_string(created.unwrap()).unwrap();
        assert!(content.contains("docs/intent/auth/auth-design.md"));
        assert!(content.contains("docs/intent/auth/auth-specs.md"));
    }

    #[test]
    fn scaffold_arrow_doc_skips_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("docs/arrows")).unwrap();
        let path = dir.path().join("docs/arrows/existing.md");
        std::fs::write(&path, "existing content").unwrap();
        let result = scaffold_arrow_doc(dir.path(), "existing.md", "seg").unwrap();
        assert!(result.is_none());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "existing content");
    }
}
