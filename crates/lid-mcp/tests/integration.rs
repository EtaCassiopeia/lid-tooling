#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;

use lid_mcp::{registry::RepoRegistry, tools};

#[tokio::test]
async fn init_creates_project_scaffold() {
    let dir = tempfile::tempdir().unwrap();

    let result = tools::init::lid_init(tools::init::InitInput {
        path: dir.path().to_string_lossy().into_owned(),
        segment: "core".to_owned(),
        spec_prefix: None,
    })
    .await
    .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(json["segment"], "core");
    assert!(dir.path().join("docs/arrows/index.yaml").exists());
    assert!(dir.path().join("docs/arrows/core/overview.md").exists());
    assert!(dir.path().join("docs/intent/core/core-specs.md").exists());
    assert!(dir.path().join("docs/intent/core/core-design.md").exists());

    let index = fs::read_to_string(dir.path().join("docs/arrows/index.yaml")).unwrap();
    assert!(index.contains("schema_version: 2"));
    assert!(index.contains("core:"));
}

#[tokio::test]
async fn init_then_discover_succeeds() {
    let dir = tempfile::tempdir().unwrap();

    tools::init::lid_init(tools::init::InitInput {
        path: dir.path().to_string_lossy().into_owned(),
        segment: "payments".to_owned(),
        spec_prefix: None,
    })
    .await
    .unwrap();

    let registry = RepoRegistry::new();
    let result = tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput {
            path: dir.path().to_string_lossy().into_owned(),
        },
    )
    .await
    .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(json["segment_count"], 1);
    assert_eq!(json["schema_version"], 2);
}

#[tokio::test]
async fn init_rejects_existing_project() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("docs/arrows")).unwrap();
    fs::write(
        dir.path().join("docs/arrows/index.yaml"),
        "schema_version: 2\narrows: {}\n",
    )
    .unwrap();

    let result = tools::init::lid_init(tools::init::InitInput {
        path: dir.path().to_string_lossy().into_owned(),
        segment: "core".to_owned(),
        spec_prefix: None,
    })
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn init_rejects_invalid_segment_name() {
    let dir = tempfile::tempdir().unwrap();

    let result = tools::init::lid_init(tools::init::InitInput {
        path: dir.path().to_string_lossy().into_owned(),
        segment: "My Segment".to_owned(),
        spec_prefix: None,
    })
    .await;

    assert!(result.is_err());
}

fn make_repo(root: &Path) {
    fs::create_dir_all(root.join("docs/arrows")).unwrap();
    fs::create_dir_all(root.join("docs/intent/auth")).unwrap();
    fs::write(
        root.join("docs/arrows/index.yaml"),
        "schema_version: 2\narrows:\n  auth:\n    status: MAPPED\n    detail: auth/core.md\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/intent/auth/auth-specs.md"),
        "- [ ] **AUTH-001**: the system shall authenticate users.\n- [x] **AUTH-002**: sessions shall expire.\n",
    )
    .unwrap();
}

#[tokio::test]
async fn discover_returns_root_and_segment_count() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());

    let registry = RepoRegistry::new();
    let result = tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput {
            path: dir.path().to_string_lossy().into_owned(),
        },
    )
    .await
    .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(json["schema_version"], 2);
    assert_eq!(json["segment_count"], 1);
}

#[tokio::test]
async fn status_counts_specs() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());

    let registry = RepoRegistry::new();
    // register first
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput {
            path: dir.path().to_string_lossy().into_owned(),
        },
    )
    .await
    .unwrap();

    let result = tools::status::lid_status(
        &registry,
        tools::status::StatusInput {
            project_root: dir.path().to_string_lossy().into_owned(),
        },
    )
    .await
    .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(json["open_specs"], 1);
    assert_eq!(json["implemented_specs"], 1);
    assert_eq!(json["total_specs"], 2);
}

#[tokio::test]
async fn update_spec_status_rewrites_file() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    let root = dir.path().to_string_lossy().into_owned();

    let registry = RepoRegistry::new();
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput { path: root.clone() },
    )
    .await
    .unwrap();

    tools::write_spec::lid_update_spec_status(
        &registry,
        tools::write_spec::UpdateSpecStatusInput {
            project_root: root.clone(),
            spec_id: "AUTH-001".to_owned(),
            status: "implemented".to_owned(),
        },
    )
    .await
    .unwrap();

    let content = fs::read_to_string(dir.path().join("docs/intent/auth/auth-specs.md")).unwrap();
    assert!(content.contains("- [x] **AUTH-001**:"));
}

#[tokio::test]
async fn add_spec_appends_and_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    let root = dir.path().to_string_lossy().into_owned();

    let registry = RepoRegistry::new();
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput { path: root.clone() },
    )
    .await
    .unwrap();

    tools::write_spec::lid_add_spec(
        &registry,
        tools::write_spec::AddSpecInput {
            project_root: root.clone(),
            segment_id: "auth".to_owned(),
            spec_id: "AUTH-099".to_owned(),
            text: "passwords shall be hashed.".to_owned(),
        },
    )
    .await
    .unwrap();

    let content = fs::read_to_string(dir.path().join("docs/intent/auth/auth-specs.md")).unwrap();
    assert!(content.contains("- [ ] **AUTH-099**: passwords shall be hashed."));
}

#[tokio::test]
async fn add_segment_and_update_status() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    let root = dir.path().to_string_lossy().into_owned();

    let registry = RepoRegistry::new();
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput { path: root.clone() },
    )
    .await
    .unwrap();

    tools::write_segment::lid_add_segment(
        &registry,
        tools::write_segment::AddSegmentInput {
            project_root: root.clone(),
            segment_id: "billing".to_owned(),
            status: "UNMAPPED".to_owned(),
            detail: "billing/core.md".to_owned(),
            blocks: vec![],
            children: vec![],
            spec_prefix: None,
            parent: None,
        },
    )
    .await
    .unwrap();

    tools::write_segment::lid_update_segment(
        &registry,
        tools::write_segment::UpdateSegmentInput {
            project_root: root.clone(),
            segment_id: "billing".to_owned(),
            status: Some("MAPPED".to_owned()),
            next: None,
            drift: None,
            blocks: None,
            children: None,
            parent: None,
        },
    )
    .await
    .unwrap();

    let content = fs::read_to_string(dir.path().join("docs/arrows/index.yaml")).unwrap();
    assert!(content.contains("billing:"));
    assert!(content.contains("MAPPED"));
}

// Realistic index.yaml fixture: dates, comments, inline arrays, taxonomy.
// Mirrors the structure of crates/lid-cli/tests/fixtures/urlshort/docs/arrows/index.yaml.
const REALISTIC_INDEX: &str = "\
schema_version: 2
last_updated: 2026-05-29

taxonomy:
  infrastructure: [storage, auth]
  domain: [shortener-core]

arrows:
  # ── Storage layer ────────────────────────────────────────────────────────────
  storage:
    status: MAPPED
    sampled: 2026-04-10
    audited: 2026-04-15
    detail: storage.md
    blocks: [shortener-core]
    children: [in-memory-store]
    next: \"provision Redis cluster; promote redis-store to MAPPED after infrastructure is ready\"

  in-memory-store:
    status: OK
    parent: storage
    audited: 2026-04-22
    detail: storage/in-memory-store.md

  # ── Domain layer ─────────────────────────────────────────────────────────────
  shortener-core:
    status: MAPPED
    sampled: 2026-04-20
    detail: shortener-core.md

  auth:
    status: UNMAPPED
    detail: auth.md
    blocks: [shortener-core]
    next: \"implement HMAC-SHA256 API-key middleware\"

unmapped:
  docs:
    intent: []
";

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn add_segment_does_not_reformat_realistic_yaml() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("docs/arrows")).unwrap();
    fs::create_dir_all(dir.path().join("docs/intent/auth")).unwrap();
    fs::write(dir.path().join("docs/arrows/index.yaml"), REALISTIC_INDEX).unwrap();
    fs::write(
        dir.path().join("docs/intent/auth/auth-specs.md"),
        "---\nprefix: USH-AUTH\n---\n- [ ] **USH-AUTH-001**: the system shall authenticate users.\n",
    )
    .unwrap();

    let root = dir.path().to_string_lossy().into_owned();
    let registry = RepoRegistry::new();
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput { path: root.clone() },
    )
    .await
    .unwrap();

    tools::write_segment::lid_add_segment(
        &registry,
        tools::write_segment::AddSegmentInput {
            project_root: root.clone(),
            segment_id: "billing".to_owned(),
            status: "UNMAPPED".to_owned(),
            detail: "billing/core.md".to_owned(),
            blocks: vec![],
            children: vec![],
            spec_prefix: Some("USH-BILL".to_owned()),
            parent: None,
        },
    )
    .await
    .unwrap();

    let after = fs::read_to_string(dir.path().join("docs/arrows/index.yaml")).unwrap();

    // New segment appears exactly once.
    assert_eq!(
        after.matches("billing:").count(),
        1,
        "segment header should appear once"
    );

    // No spurious fields injected by a full round-trip.
    assert!(
        !after.contains("blockedBy"),
        "blockedBy should not appear after add"
    );
    assert!(
        !after.contains("blocks: []"),
        "empty blocks should not be serialised"
    );
    assert!(
        !after.contains("children: []"),
        "empty children should not be serialised"
    );

    // Dates must remain as bare strings, not ISO timestamps.
    assert!(
        after.contains("sampled: 2026-04-10"),
        "sampled date must not be coerced"
    );
    assert!(
        after.contains("audited: 2026-04-15"),
        "audited date must not be coerced"
    );
    assert!(
        !after.contains("T00:00:00"),
        "dates must not become ISO timestamps"
    );

    // Comments must be preserved.
    assert!(
        after.contains("# ── Storage layer"),
        "section comments must be preserved"
    );
    assert!(
        after.contains("# ── Domain layer"),
        "section comments must be preserved"
    );

    // Inline arrays must stay inline.
    assert!(
        after.contains("blocks: [shortener-core]"),
        "inline arrays must stay inline after add"
    );
    assert!(
        after.contains("children: [in-memory-store]"),
        "inline arrays must stay inline after add"
    );
    assert!(
        after.contains("infrastructure: [storage, auth]"),
        "taxonomy inline arrays must stay inline"
    );

    // Every original line must still be present.
    for line in REALISTIC_INDEX.lines() {
        assert!(
            after.contains(line),
            "original line missing after add: {line:?}"
        );
    }

    // Scaffold files must be created.
    assert!(
        dir.path()
            .join("docs/intent/billing/billing-specs.md")
            .exists(),
        "specs.md must be created"
    );
    assert!(
        dir.path()
            .join("docs/intent/billing/billing-design.md")
            .exists(),
        "design.md must be created"
    );
    assert!(
        dir.path().join("docs/arrows/billing/core.md").exists(),
        "arrow doc must be created"
    );
}

#[tokio::test]
async fn read_file_returns_content() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    let root = dir.path().to_string_lossy().into_owned();
    let registry = RepoRegistry::new();

    let result = tools::read_file::lid_read_file(
        &registry,
        tools::read_file::ReadFileInput {
            project_root: root.clone(),
            path: "docs/intent/auth/auth-specs.md".to_owned(),
        },
    )
    .await
    .unwrap();

    assert!(result.contains("AUTH-001"));
}

#[tokio::test]
async fn read_file_rejects_path_traversal() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    let root = dir.path().to_string_lossy().into_owned();
    let registry = RepoRegistry::new();

    let result = tools::read_file::lid_read_file(
        &registry,
        tools::read_file::ReadFileInput {
            project_root: root.clone(),
            path: "../../etc/passwd".to_owned(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn append_to_design_doc_creates_and_appends() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    let root = dir.path().to_string_lossy().into_owned();
    let registry = RepoRegistry::new();
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput { path: root.clone() },
    )
    .await
    .unwrap();

    // First call creates the file.
    tools::read_file::lid_append_to_design_doc(
        &registry,
        tools::read_file::AppendToDesignDocInput {
            project_root: root.clone(),
            segment_id: "auth".to_owned(),
            content: "## Decision\n\nUse HMAC-SHA256 for API key signing.".to_owned(),
        },
    )
    .await
    .unwrap();

    let content = fs::read_to_string(dir.path().join("docs/intent/auth/auth-design.md")).unwrap();
    assert!(content.contains("HMAC-SHA256"));
    assert!(content.contains("## Decision"));

    // Second call appends.
    tools::read_file::lid_append_to_design_doc(
        &registry,
        tools::read_file::AppendToDesignDocInput {
            project_root: root.clone(),
            segment_id: "auth".to_owned(),
            content: "## Open Questions\n\nToken rotation interval TBD.".to_owned(),
        },
    )
    .await
    .unwrap();

    let content2 = fs::read_to_string(dir.path().join("docs/intent/auth/auth-design.md")).unwrap();
    assert!(content2.contains("HMAC-SHA256"));
    assert!(content2.contains("Token rotation interval"));
}

#[tokio::test]
async fn update_spec_text_rewrites_text_preserves_status() {
    let dir = tempfile::tempdir().unwrap();
    make_repo(dir.path());
    let root = dir.path().to_string_lossy().into_owned();
    let registry = RepoRegistry::new();
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput { path: root.clone() },
    )
    .await
    .unwrap();

    tools::write_spec::lid_update_spec_text(
        &registry,
        tools::write_spec::UpdateSpecTextInput {
            project_root: root.clone(),
            spec_id: "AUTH-001".to_owned(),
            new_text: "the system SHALL authenticate users via HMAC-signed tokens.".to_owned(),
        },
    )
    .await
    .unwrap();

    let content = fs::read_to_string(dir.path().join("docs/intent/auth/auth-specs.md")).unwrap();
    assert!(content.contains(
        "- [ ] **AUTH-001**: the system SHALL authenticate users via HMAC-signed tokens."
    ));
    // AUTH-002 (implemented) must be untouched.
    assert!(content.contains("- [x] **AUTH-002**: sessions shall expire."));
}

#[tokio::test]
async fn update_segment_patches_blocks_without_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    // Use a realistic fixture to prove inline arrays and comments are preserved.
    fs::create_dir_all(dir.path().join("docs/arrows")).unwrap();
    fs::create_dir_all(dir.path().join("docs/intent/auth")).unwrap();
    fs::write(dir.path().join("docs/arrows/index.yaml"), REALISTIC_INDEX).unwrap();
    fs::write(
        dir.path().join("docs/intent/auth/auth-specs.md"),
        "---\nprefix: USH-AUTH\n---\n",
    )
    .unwrap();
    let root = dir.path().to_string_lossy().into_owned();
    let registry = RepoRegistry::new();
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput { path: root.clone() },
    )
    .await
    .unwrap();

    tools::write_segment::lid_update_segment(
        &registry,
        tools::write_segment::UpdateSegmentInput {
            project_root: root.clone(),
            segment_id: "auth".to_owned(),
            status: Some("MAPPED".to_owned()),
            next: None,
            drift: None,
            blocks: Some(vec!["shortener-core".to_owned(), "storage".to_owned()]),
            children: None,
            parent: None,
        },
    )
    .await
    .unwrap();

    let after = fs::read_to_string(dir.path().join("docs/arrows/index.yaml")).unwrap();

    // Status updated.
    assert!(
        after.contains("    status: MAPPED"),
        "status must be updated"
    );
    // Blocks updated.
    assert!(
        after.contains("blocks: [shortener-core, storage]"),
        "blocks must be updated"
    );
    // Dates and comments still intact.
    assert!(
        after.contains("sampled: 2026-04-10"),
        "dates must be preserved"
    );
    assert!(
        after.contains("# ── Storage layer"),
        "comments must be preserved"
    );
    // Other inline arrays untouched.
    assert!(
        after.contains("children: [in-memory-store]"),
        "other segment's arrays must be intact"
    );
}

#[tokio::test]
async fn add_child_segment_creates_files_in_parent_dir() {
    let dir = tempfile::tempdir().unwrap();
    // Build a minimal project with a parent segment already in the index.
    fs::create_dir_all(dir.path().join("docs/arrows")).unwrap();
    fs::create_dir_all(dir.path().join("docs/intent/payments")).unwrap();
    fs::write(
        dir.path().join("docs/arrows/index.yaml"),
        "schema_version: 2\narrows:\n  payments:\n    status: MAPPED\n    detail: payments/overview.md\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("docs/intent/payments/payments-specs.md"),
        "---\nprefix: PAY\n---\n\n# payments specs\n",
    )
    .unwrap();

    let root = dir.path().to_string_lossy().into_owned();
    let registry = RepoRegistry::new();
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput { path: root.clone() },
    )
    .await
    .unwrap();

    tools::write_segment::lid_add_segment(
        &registry,
        tools::write_segment::AddSegmentInput {
            project_root: root.clone(),
            segment_id: "checkout".to_owned(),
            status: "UNMAPPED".to_owned(),
            detail: "payments/checkout.md".to_owned(),
            blocks: vec![],
            children: vec![],
            spec_prefix: Some("PAY-CHECKOUT".to_owned()),
            parent: Some("payments".to_owned()),
        },
    )
    .await
    .unwrap();

    // Intent files must be under docs/intent/payments/checkout/, not docs/intent/checkout/.
    assert!(
        dir.path()
            .join("docs/intent/payments/checkout/checkout-specs.md")
            .exists(),
        "nested specs file should be created under parent dir"
    );
    assert!(
        dir.path()
            .join("docs/intent/payments/checkout/checkout-design.md")
            .exists(),
        "nested design file should be created under parent dir"
    );
    assert!(
        !dir.path().join("docs/intent/checkout").exists(),
        "flat top-level dir should not be created for child segment"
    );

    // Arrow doc references must point to the nested intent paths.
    let arrow_content =
        fs::read_to_string(dir.path().join("docs/arrows/payments/checkout.md")).unwrap();
    assert!(
        arrow_content.contains("docs/intent/payments/checkout/checkout-design.md"),
        "arrow doc LLD reference must use nested path"
    );

    // index.yaml must include parent field.
    let index = fs::read_to_string(dir.path().join("docs/arrows/index.yaml")).unwrap();
    assert!(
        index.contains("parent: payments"),
        "parent field must be written to index.yaml"
    );
}

#[tokio::test]
async fn add_spec_to_child_segment_finds_nested_path() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("docs/arrows")).unwrap();
    // Create a nested project: payments (parent) + checkout (child).
    fs::write(
        dir.path().join("docs/arrows/index.yaml"),
        "schema_version: 2\narrows:\n  payments:\n    status: MAPPED\n    detail: payments/overview.md\n  checkout:\n    status: MAPPED\n    parent: payments\n    detail: payments/checkout.md\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("docs/intent/payments/checkout")).unwrap();
    fs::write(
        dir.path()
            .join("docs/intent/payments/checkout/checkout-specs.md"),
        "---\nprefix: PAY-CHECKOUT\n---\n\n# checkout specs\n",
    )
    .unwrap();

    let root = dir.path().to_string_lossy().into_owned();
    let registry = RepoRegistry::new();
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput { path: root.clone() },
    )
    .await
    .unwrap();

    tools::write_spec::lid_add_spec(
        &registry,
        tools::write_spec::AddSpecInput {
            project_root: root.clone(),
            segment_id: "checkout".to_owned(),
            spec_id: "PAY-CHECKOUT-001".to_owned(),
            text: "The system shall complete checkout within 3 seconds.".to_owned(),
        },
    )
    .await
    .unwrap();

    let content = fs::read_to_string(
        dir.path()
            .join("docs/intent/payments/checkout/checkout-specs.md"),
    )
    .unwrap();
    assert!(
        content.contains("PAY-CHECKOUT-001"),
        "spec must be appended to the nested spec file"
    );
}

#[tokio::test]
async fn append_to_design_doc_child_uses_parent_path() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("docs/arrows")).unwrap();
    fs::write(
        dir.path().join("docs/arrows/index.yaml"),
        "schema_version: 2\narrows:\n  payments:\n    status: MAPPED\n    detail: payments/overview.md\n  checkout:\n    status: MAPPED\n    parent: payments\n    detail: payments/checkout.md\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("docs/intent/payments/checkout")).unwrap();
    fs::write(
        dir.path()
            .join("docs/intent/payments/checkout/checkout-design.md"),
        "# checkout design\n\n## Overview\n\n<!-- stub -->\n",
    )
    .unwrap();

    let root = dir.path().to_string_lossy().into_owned();
    let registry = RepoRegistry::new();
    tools::discover::lid_discover(
        &registry,
        tools::discover::DiscoverInput { path: root.clone() },
    )
    .await
    .unwrap();

    tools::read_file::lid_append_to_design_doc(
        &registry,
        tools::read_file::AppendToDesignDocInput {
            project_root: root.clone(),
            segment_id: "checkout".to_owned(),
            content: "## Decision\n\nUse a single-page checkout flow.".to_owned(),
        },
    )
    .await
    .unwrap();

    let content = fs::read_to_string(
        dir.path()
            .join("docs/intent/payments/checkout/checkout-design.md"),
    )
    .unwrap();
    assert!(
        content.contains("## Decision"),
        "content must be appended to the nested design doc"
    );
    assert!(
        !dir.path()
            .join("docs/intent/checkout/checkout-design.md")
            .exists(),
        "must not create flat design doc for child segment"
    );
}
