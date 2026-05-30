//! Smoke tests: spawn the `lid-lsp` binary, exchange LSP messages over stdio,
//! and verify the server responds correctly without crashing.
//!
//! All blocking I/O runs in a background thread; the main test thread enforces
//! a hard 30-second timeout via an mpsc channel, matching the pattern used in
//! `tests/diagnostics.rs`. This prevents the test suite from hanging
//! indefinitely if the server misbehaves.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{Read, Write};
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
        assert!(n != 0, "EOF while reading LSP headers; partial: {buf:?}");
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

/// Read framed LSP messages until one whose body contains `id_marker`.
/// Notifications and other messages are silently consumed.
fn read_response_for_id<R: Read>(reader: &mut R, id_marker: &str) -> String {
    loop {
        let msg = read_framed_message(reader);
        if msg.contains(id_marker) {
            return msg;
        }
    }
}

/// Spawn the server, complete the LSP handshake, run `test_body` with the
/// stdio handles, then drive shutdown/exit and wait for clean process exit.
fn with_server<F>(test_body: F)
where
    F: FnOnce(&mut std::process::ChildStdin, &mut std::process::ChildStdout),
{
    let binary = env!("CARGO_BIN_EXE_lid-lsp");
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn lid-lsp");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    let initialize =
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    stdin.write_all(&frame(initialize)).unwrap();
    let _ = read_response_for_id(&mut stdout, "\"id\":1");
    stdin.write_all(&frame(initialized)).unwrap();

    test_body(&mut stdin, &mut stdout);

    let shutdown = r#"{"jsonrpc":"2.0","id":99,"method":"shutdown"}"#;
    let exit = r#"{"jsonrpc":"2.0","method":"exit"}"#;
    stdin.write_all(&frame(shutdown)).unwrap();
    let shutdown_response = read_response_for_id(&mut stdout, "\"id\":99");
    assert!(
        shutdown_response.contains("\"result\""),
        "shutdown should succeed, got: {shutdown_response}"
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
    let (tx, rx) = mpsc::channel::<String>();

    std::thread::spawn(move || {
        let binary = env!("CARGO_BIN_EXE_lid-lsp");
        let mut child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
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

        let init_response = read_response_for_id(&mut stdout, "\"id\":1");
        stdin.write_all(&frame(shutdown)).unwrap();
        let shutdown_response = read_response_for_id(&mut stdout, "\"id\":2");

        stdin.write_all(&frame(exit)).unwrap();
        drop(stdin);
        let _ = child.wait();

        // Send both responses concatenated so the main thread can assert on them.
        let _ = tx.send(format!("{init_response}\n{shutdown_response}"));
    });

    let responses = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("timeout: server did not complete handshake within 30 s");

    let (init_response, shutdown_response) =
        responses.split_once('\n').expect("expected two responses");

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
    assert!(
        shutdown_response.contains("\"result\""),
        "shutdown should return a result, got: {shutdown_response}"
    );
}

#[test]
fn did_open_change_close_cycle_does_not_break_server() {
    let (tx, rx) = mpsc::channel::<()>();

    std::thread::spawn(move || {
        with_server(|stdin, _stdout| {
            let did_open = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///fake/auth.rs","languageId":"rust","version":1,"text":"// @spec AUTH-001\nfn login() {}\n"}}}"#;
            let did_change = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///fake/auth.rs","version":2},"contentChanges":[{"text":"// @spec AUTH-001, AUTH-002\nfn login() {}\n"}]}}"#;
            let did_close = r#"{"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"file:///fake/auth.rs"}}}"#;

            stdin.write_all(&frame(did_open)).unwrap();
            stdin.write_all(&frame(did_change)).unwrap();
            stdin.write_all(&frame(did_close)).unwrap();

            // No responses expected for notifications. The liveness check is
            // that the subsequent shutdown (driven by `with_server`) succeeds.
        });
        let _ = tx.send(());
    });

    rx.recv_timeout(Duration::from_secs(30))
        .expect("timeout: server hung or crashed during open/change/close cycle");
}
