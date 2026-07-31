//! A minimal MCP client for the integration tests: spawn the real binary,
//! complete the protocol handshake, then do one request / one response.
//!
//! Sequencing matters. rmcp serves requests CONCURRENTLY, so a test that
//! pipes every message at once gets its replies back in completion order —
//! and a `session_*` call can then overtake the `session_open` that was
//! supposed to create its handle. A real MCP client waits for each response,
//! so this one does too.

#![allow(dead_code)] // each test binary uses a different subset

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub const DOM: &str = "(define (domain d) (:requirements :strips) (:predicates (p) (q) (r)) \
    (:action a :precondition (p) :effect (and (not (p)) (q))) \
    (:action b :precondition (q) :effect (and (not (q)) (r))))";
pub const PROB: &str = "(define (problem pr) (:domain d) (:init (p)) (:goal (r)))";

pub struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl Client {
    /// Spawn the server and complete `initialize` + `notifications/initialized`.
    /// rmcp enforces the MCP lifecycle — a `tools/call` before this is a
    /// protocol error that ends the connection, by spec.
    pub fn start() -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ferroplan-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn ferroplan-mcp");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut c = Client {
            child,
            stdin: Some(stdin),
            stdout,
            next_id: 0,
        };
        let init = c.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1"}
            }),
        );
        assert_eq!(init["result"]["serverInfo"]["name"], "ferroplan");
        c.notify("notifications/initialized");
        c
    }

    /// Send a request and read exactly one response line.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let msg = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let stdin = self.stdin.as_mut().expect("stdin open");
        writeln!(stdin, "{msg}").expect("write request");
        stdin.flush().expect("flush");
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read response");
        assert!(!line.trim().is_empty(), "server closed without responding");
        let v: Value = serde_json::from_str(&line).expect("one JSON object per line");
        assert_eq!(v["id"], id, "response id must match the request");
        v
    }

    /// Send a notification (no reply expected, and none must arrive).
    pub fn notify(&mut self, method: &str) {
        let msg = json!({"jsonrpc": "2.0", "method": method});
        let stdin = self.stdin.as_mut().expect("stdin open");
        writeln!(stdin, "{msg}").expect("write notification");
        stdin.flush().expect("flush");
    }

    /// `tools/call`, returning the `result` object (which carries `isError`).
    pub fn call(&mut self, tool: &str, args: Value) -> Value {
        let r = self.request("tools/call", json!({"name": tool, "arguments": args}));
        r["result"].clone()
    }

    /// `tools/call` for a tool that must SUCCEED and returns a JSON document
    /// in its text block; the document is parsed and returned.
    pub fn call_json(&mut self, tool: &str, args: Value) -> Value {
        let res = self.call(tool, args);
        assert_ne!(
            res["isError"], true,
            "{tool} unexpectedly failed: {}",
            res["content"][0]["text"]
        );
        let text = res["content"][0]["text"]
            .as_str()
            .expect("a text content block");
        serde_json::from_str(text)
            .unwrap_or_else(|e| panic!("{tool} must return JSON: {e}\n{text}"))
    }

    /// The text of a tool result, whether it succeeded or failed.
    pub fn call_text(&mut self, tool: &str, args: Value) -> (String, bool) {
        let res = self.call(tool, args);
        let is_err = res["isError"] == true;
        (
            res["content"][0]["text"].as_str().unwrap_or("").to_string(),
            is_err,
        )
    }

    /// Close stdin and require a clean exit.
    pub fn finish(mut self) {
        self.stdin.take(); // EOF
        let status = self.child.wait().expect("wait");
        assert!(status.success(), "server exited with {status:?}");
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // A panicking test must not leave a server behind.
        let _ = self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
