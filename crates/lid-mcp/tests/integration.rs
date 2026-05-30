use std::fs;
use std::path::Path;

use lid_mcp::{registry::RepoRegistry, tools};

#[tokio::test]
async fn init_creates_project_scaffold() {
    let dir = tempfile::tempdir().unwrap();

    let result = tools::init::lid_init(tools::init::InitInput {
        path: dir.path().to_string_lossy().into_owned(),
        segment: "core".to_owned(),
    })
    .await
    .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(json["segment"], "core");
    assert!(dir.path().join("docs/arrows/index.yaml").exists());
    assert!(dir.path().join("docs/arrows/core/overview.md").exists());
    assert!(dir.path().join("docs/intent").is_dir());

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
        },
    )
    .await
    .unwrap();

    let content = fs::read_to_string(dir.path().join("docs/arrows/index.yaml")).unwrap();
    assert!(content.contains("billing:"));
    assert!(content.contains("MAPPED"));
}
