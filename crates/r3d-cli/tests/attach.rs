#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
//! `r3d mcp --attach` relays an agent's stdio to the app's endpoint.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use r3d_commands::Workspace;
use r3d_mcp::Server;
use r3d_mcp::attach::{ENDPOINT_FILE, Host};
use serde_json::{Value, json};

#[test]
fn the_relay_carries_mcp_to_the_app() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = Arc::new(Mutex::new(
        Workspace::create(&tmp.path().join("P.r3d"), "P").unwrap(),
    ));
    // Stands in for the app.
    let host = Host::start(Server::shared(ws.clone(), None), tmp.path(), "P").unwrap();

    let mut relay = Command::new(env!("CARGO_BIN_EXE_r3d"))
        .args(["mcp", "--attach", "--endpoint"])
        .arg(tmp.path().join(ENDPOINT_FILE))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = relay.stdin.take().unwrap();
    let mut stdout = BufReader::new(relay.stdout.take().unwrap());
    let mut send = |v: Value| writeln!(stdin, "{v}").unwrap();
    let mut receive = || {
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        serde_json::from_str::<Value>(&line).unwrap()
    };
    send(
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": {"name": "test", "version": "0"}}}),
    );
    assert_eq!(receive()["result"]["serverInfo"]["name"], "rustle3d");
    send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    send(
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
        "name": "panel_create_rect", "arguments": {"width": 100, "height": 100}}}),
    );
    assert_eq!(receive()["result"]["isError"], false);
    assert_eq!(ws.lock().unwrap().session().project().panels().len(), 1);

    // The app stops accepting agents: the relay ends.
    drop(host);
    assert!(relay.wait().unwrap().success());
}
