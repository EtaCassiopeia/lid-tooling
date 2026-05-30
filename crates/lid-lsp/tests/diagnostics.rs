//! End-to-end test: send `didOpen` for a source file containing an
//! `@spec` citation to an unknown spec, then read framed messages
//! until a `textDocument/publishDiagnostics` notification arrives.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

fn frame(body: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let _ = write!(out, "Content-Length: {}\r\n\r\n", body.len());
    out.extend_from_slice(body.as_bytes());
    out
}

fn read_framed_message<R: Read>(reader: &mut R) -> String {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = reader.read(&mut byte).expect("read");
        assert!(n != 0, "EOF while reading LSP headers");
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let headers = String::from_utf8_lossy(&buf).to_string();
    let length: usize = headers
        .lines()
        .find_map(|l| l.strip_prefix("Content-Length:").map(str::trim))
        .expect("Content-Length header missing")
        .parse()
        .expect("Content-Length must be a number");
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).expect("read body");
    String::from_utf8(body).expect("body must be UTF-8")
}

/// Wait for a notification whose method matches `method`.
fn read_notification<R: Read>(reader: &mut R, method: &str) -> String {
    loop {
        let msg = read_framed_message(reader);
        if msg.contains(&format!("\"method\":\"{method}\"")) {
            return msg;
        }
    }
}

/// Write a minimal LID layout under `root`. Mirrors the helper in
/// the CLI integration tests but lives here to keep the two tests
/// independent.
fn make_minimal_lid_repo(root: &Path) {
    fs::create_dir_all(root.join("docs/arrows")).unwrap();
    fs::create_dir_all(root.join("docs/intent/auth")).unwrap();
    fs::write(
        root.join("docs/arrows/index.yaml"),
        "\
schema_version: 2
arrows:
  auth:
    status: MAPPED
    detail: auth.md
    blocks: []
    blockedBy: []
",
    )
    .unwrap();
    fs::write(
        root.join("docs/arrows/auth.md"),
        "# Arrow: auth\n\n## References\n\n### EARS\n- docs/intent/auth/auth-specs.md\n\n### LLD\n- docs/intent/auth/auth-design.md\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/intent/auth/auth-specs.md"),
        "# auth\n\n- [x] **AUTH-001**: requirement text.\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/intent/auth/auth-design.md"),
        "\
# LLD: auth

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
| --- | --- | --- | --- |
| x | y | z | r |
",
    )
    .unwrap();
}

#[test]
fn did_open_publishes_diagnostic_for_unknown_spec_citation() {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    make_minimal_lid_repo(&root);
    let source_path = root.join("src/auth.rs");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "// placeholder\n").unwrap();

    let binary = env!("CARGO_BIN_EXE_lid-lsp");
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn lid-lsp");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    // Drive the LSP handshake and wait for publishDiagnostics in a
    // dedicated thread so the main thread can enforce a hard timeout via
    // a channel.  `read_framed_message` uses blocking I/O; without this
    // indirection a stalled server would hang the test indefinitely.
    let uri_clone = format!(
        "file://{}",
        source_path.to_string_lossy().replace('\\', "/")
    );
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let initialize =
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
        stdin.write_all(&frame(initialize)).unwrap();
        let _ = read_framed_message(&mut stdout); // initialize result
        stdin
            .write_all(&frame(
                r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
            ))
            .unwrap();

        let did_open = format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri_clone}","languageId":"rust","version":1,"text":"// @spec UNKNOWN-001\nfn x() {{}}\n"}}}}}}"#
        );
        stdin.write_all(&frame(&did_open)).unwrap();

        let notification = read_notification(&mut stdout, "textDocument/publishDiagnostics");
        let _ = tx.send(notification);
    });

    let notification = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("timeout: publishDiagnostics not received within 30 s");

    let parsed: serde_json::Value = serde_json::from_str(&notification).unwrap();
    let params = &parsed["params"];
    let uri = format!(
        "file://{}",
        source_path.to_string_lossy().replace('\\', "/")
    );
    assert_eq!(params["uri"], serde_json::Value::String(uri));
    let diagnostics = params["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1, "got: {diagnostics:?}");
    let diag = &diagnostics[0];
    assert!(
        diag["message"].as_str().unwrap().contains("UNKNOWN-001"),
        "got: {diag:?}"
    );
    assert_eq!(diag["code"], "reverse-orphan");
    assert_eq!(diag["severity"], 1); // DiagnosticSeverity::ERROR

    let _ = child.kill();
    let _ = child.wait();
}
