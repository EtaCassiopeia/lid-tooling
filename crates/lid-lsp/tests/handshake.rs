//! Smoke test: spawn the `lid-lsp` binary, exchange a minimal LSP
//! handshake over stdio, and verify the server responds correctly.
//!
//! This is the lowest-level test of the wire protocol — every higher
//! handler builds on it. If this passes, the binary speaks LSP at all.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Wrap a JSON body in an LSP Content-Length header.
fn frame(body: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let _ = write!(out, "Content-Length: {}\r\n\r\n", body.len());
    out.extend_from_slice(body.as_bytes());
    out
}

/// Read framed LSP messages until one matching `id_marker` arrives;
/// notifications (window/logMessage, etc.) are silently consumed.
fn read_response_for_id<R: Read>(reader: &mut R, id_marker: &str, deadline: Instant) -> String {
    loop {
        let msg = read_framed_message(reader, deadline);
        if msg.contains(id_marker) {
            return msg;
        }
    }
}

/// Read the next framed LSP message from `reader`, with a deadline so
/// we don't hang the test suite if the server misbehaves.
fn read_framed_message<R: Read>(reader: &mut R, deadline: Instant) -> String {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];

    // Read headers until we see the blank line `\r\n\r\n`.
    loop {
        assert!(
            Instant::now() <= deadline,
            "timed out reading LSP headers; partial buffer:\n{}",
            String::from_utf8_lossy(&buf)
        );
        let n = reader.read(&mut byte).expect("read");
        assert!(n != 0, "EOF while reading headers; partial: {buf:?}");
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    // Parse Content-Length.
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

/// Spawn the server, run the test closure with stdio handles, then
/// send shutdown/exit and wait for clean exit.
fn with_server<F>(test_body: F)
where
    F: FnOnce(&mut std::process::ChildStdin, &mut std::process::ChildStdout),
{
    let binary = env!("CARGO_BIN_EXE_lid-lsp");
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn lid-lsp");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    let initialize =
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    stdin.write_all(&frame(initialize)).unwrap();
    let _ = read_response_for_id(
        &mut stdout,
        "\"id\":1",
        Instant::now() + Duration::from_secs(5),
    );
    stdin.write_all(&frame(initialized)).unwrap();

    test_body(&mut stdin, &mut stdout);

    let shutdown = r#"{"jsonrpc":"2.0","id":99,"method":"shutdown"}"#;
    let exit = r#"{"jsonrpc":"2.0","method":"exit"}"#;
    stdin.write_all(&frame(shutdown)).unwrap();
    let shutdown_response = read_response_for_id(
        &mut stdout,
        "\"id\":99",
        Instant::now() + Duration::from_secs(5),
    );
    assert!(
        shutdown_response.contains("\"result\""),
        "shutdown should succeed even after notifications, got: {shutdown_response}"
    );
    stdin.write_all(&frame(exit)).unwrap();
    drop(stdin);
    let status = child.wait().expect("wait for child");
    assert!(
        status.success() || status.code() == Some(0),
        "server should exit cleanly, got: {status:?}"
    );
}

#[test]
fn server_responds_to_initialize_and_shutdown() {
    let binary = env!("CARGO_BIN_EXE_lid-lsp");
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn lid-lsp");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    let initialize =
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    let shutdown = r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#;
    let exit = r#"{"jsonrpc":"2.0","method":"exit"}"#;

    stdin.write_all(&frame(initialize)).unwrap();
    stdin.write_all(&frame(initialized)).unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let init_response = read_response_for_id(&mut stdout, "\"id\":1", deadline);
    assert!(
        init_response.contains("\"result\""),
        "expected result, got: {init_response}"
    );
    assert!(
        init_response.contains("\"name\":\"lid-lsp\""),
        "expected server name in InitializeResult, got: {init_response}"
    );
    assert!(
        init_response.contains("\"textDocumentSync\""),
        "expected textDocumentSync capability, got: {init_response}"
    );

    stdin.write_all(&frame(shutdown)).unwrap();
    let shutdown_response = read_response_for_id(
        &mut stdout,
        "\"id\":2",
        Instant::now() + Duration::from_secs(5),
    );
    assert!(
        shutdown_response.contains("\"result\""),
        "shutdown should return a result, got: {shutdown_response}"
    );

    stdin.write_all(&frame(exit)).unwrap();
    drop(stdin);

    let status = child.wait().expect("wait for child");
    assert!(
        status.success() || status.code() == Some(0),
        "server should exit cleanly after exit, got: {status:?}"
    );
}

#[test]
fn did_open_change_close_cycle_does_not_break_server() {
    with_server(|stdin, _stdout| {
        let did_open = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///fake/auth.rs","languageId":"rust","version":1,"text":"// @spec AUTH-001\nfn login() {}\n"}}}"#;
        let did_change = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///fake/auth.rs","version":2},"contentChanges":[{"text":"// @spec AUTH-001, AUTH-002\nfn login() {}\n"}]}}"#;
        let did_close = r#"{"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"file:///fake/auth.rs"}}}"#;

        stdin.write_all(&frame(did_open)).unwrap();
        stdin.write_all(&frame(did_change)).unwrap();
        stdin.write_all(&frame(did_close)).unwrap();

        // None of these are requests, so we don't expect responses.
        // The harness's shutdown after this closure returns is the
        // liveness check: if the server crashed on any of the
        // notifications, that shutdown wouldn't succeed.
    });
}
