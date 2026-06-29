//! `ferroplan-mcp` — a Model Context Protocol server exposing the ferroplan planner
//! to an LLM agent: `solve`, `validate`, and `decompose` as MCP tools.
//!
//! This is the README's bet made operational — the agent *authors and supervises*
//! PDDL and the planner runs deterministically. The agent writes a domain + problem,
//! calls `solve` (or `decompose` for a too-big goal), reads the structured result,
//! and iterates; `validate` independently checks a plan under ferroplan's semantics.
//!
//! Transport: MCP stdio — newline-delimited JSON-RPC 2.0, one message per line, no
//! embedded newlines. No async runtime; a blocking read loop over stdin keeps the
//! dependency surface to `serde`/`serde_json`, matching the rest of the workspace.
//! The server never panics on bad input: malformed JSON-RPC yields an error response,
//! a failing tool yields an `isError` tool result, and the loop continues until EOF.

use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

/// MCP protocol version we implement (echoed back from the client's `initialize` if
/// it sends one, so we interoperate with clients on a newer revision).
const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break }; // stdin closed / read error → exit
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                // JSON-RPC parse error: id unknown, so reply with null id.
                send(
                    &mut out,
                    &error_obj(Value::Null, -32700, &format!("parse error: {e}")),
                );
                continue;
            }
        };

        // A request has an `id`; a notification does not (and gets no reply).
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        match dispatch(method, params) {
            Outcome::Reply(result) => {
                if let Some(id) = id {
                    send(
                        &mut out,
                        &json!({"jsonrpc": "2.0", "id": id, "result": result}),
                    );
                }
            }
            Outcome::Err(code, message) => {
                if let Some(id) = id {
                    send(&mut out, &error_obj(id, code, &message));
                }
            }
            Outcome::Silent => {} // notification: no response
        }
    }
}

/// What to do with a dispatched message.
enum Outcome {
    /// Send this `result` (only if the message was a request).
    Reply(Value),
    /// Send a JSON-RPC error with this code/message (only if a request).
    Err(i64, String),
    /// A notification (or otherwise no reply).
    Silent,
}

fn dispatch(method: &str, params: Value) -> Outcome {
    match method {
        "initialize" => {
            let version = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(DEFAULT_PROTOCOL_VERSION)
                .to_string();
            Outcome::Reply(json!({
                "protocolVersion": version,
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "ferroplan",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "instructions": "Author a PDDL domain + problem, then call `solve` \
                    (or `decompose` for a goal too big for one-shot search) and read \
                    the structured result. `validate` independently checks a plan.",
            }))
        }
        // Notifications — no reply.
        "notifications/initialized" | "notifications/cancelled" => Outcome::Silent,
        "ping" => Outcome::Reply(json!({})),
        "tools/list" => Outcome::Reply(json!({ "tools": tool_specs() })),
        "tools/call" => call_tool(params),
        other => Outcome::Err(-32601, format!("method not found: {other}")),
    }
}

/// The tool catalogue advertised to the client.
fn tool_specs() -> Value {
    json!([
        {
            "name": "solve",
            "description": "Plan a PDDL domain + problem with ferroplan and return the \
                structured Solution (typed steps, makespan/metric, statistics). Handles \
                STRIPS, typing, ADL, numeric fluents, derived axioms, PDDL3 preferences, \
                and PDDL2.1 temporal (durative actions) — mode is auto-detected. A \
                solved:false result is a normal answer, not an error.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "PDDL domain source" },
                    "problem": { "type": "string", "description": "PDDL problem source" },
                    "options": {
                        "type": "object",
                        "description": "Optional solver Options: mode (auto|ff|partition|pddl3|temporal), search (auto|ehc|best-first|ehc-then-best-first), weight_g, weight_h, threads, max_evaluated, optimize. Omitted fields use defaults."
                    }
                },
                "required": ["domain", "problem"]
            }
        },
        {
            "name": "parse",
            "description": "Syntax-check a PDDL source string and return a structure \
                summary WITHOUT grounding or solving — fast feedback while authoring. \
                Auto-detects domain vs problem; reports ok/error (with a line number) \
                plus name, requirements, and counts (types/predicates/actions, or \
                objects/init/goal/metric). Use to catch PDDL mistakes before `solve`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pddl": {
                        "type": "string",
                        "description": "A PDDL domain OR problem source string."
                    }
                },
                "required": ["pddl"]
            }
        },
        {
            "name": "validate",
            "description": "Independently validate a plan against a domain + problem \
                under ferroplan's own execution semantics (auto-detects classical vs \
                temporal). Returns whether the plan is executable and goal-reaching, \
                with a reason if not. Use to check a plan you wrote or one solve produced.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "PDDL domain source" },
                    "problem": { "type": "string", "description": "PDDL problem source" },
                    "plan": {
                        "type": "string",
                        "description": "Plan to check: classical `step N: (action args)` lines, or a temporal `t: (action args) [dur]` plan."
                    }
                },
                "required": ["domain", "problem", "plan"]
            }
        },
        {
            "name": "decompose",
            "description": "Decompose a temporal goal too big for one-shot search into \
                ordered, individually-solved contracts, stitched into one validated plan. \
                Returns the inspectable Decomposition: each contract's named sub-goal, \
                sub-plan, and timeline offset, plus the stitched plan. A goal that can't \
                be split falls back to a single monolithic contract (reported honestly).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain": { "type": "string", "description": "PDDL domain source (durative actions)" },
                    "problem": { "type": "string", "description": "PDDL problem source" },
                    "options": { "type": "object", "description": "Optional solver Options (see `solve`)." }
                },
                "required": ["domain", "problem"]
            }
        }
    ])
}

/// Dispatch a `tools/call`. Tool-execution failures are returned as `isError` tool
/// results (not JSON-RPC errors), per the MCP convention, so the agent sees the
/// message and can correct its PDDL.
fn call_tool(params: Value) -> Outcome {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    let result = match name {
        "solve" => tool_solve(&args),
        "parse" => tool_parse(&args),
        "validate" => tool_validate(&args),
        "decompose" => tool_decompose(&args),
        other => return Outcome::Err(-32602, format!("unknown tool: {other}")),
    };
    Outcome::Reply(match result {
        Ok(text) => json!({ "content": [text_block(&text)] }),
        Err(text) => json!({ "content": [text_block(&text)], "isError": true }),
    })
}

fn tool_solve(args: &Value) -> Result<String, String> {
    let domain = require_str(args, "domain")?;
    let problem = require_str(args, "problem")?;
    let opts = parse_options(args)?;
    let sol = ferroplan::solve(domain, problem, &opts).map_err(|e| e.to_string())?;
    pretty(&sol)
}

fn tool_parse(args: &Value) -> Result<String, String> {
    let pddl = require_str(args, "pddl")?;
    pretty(&ferroplan::parse(pddl))
}

fn tool_validate(args: &Value) -> Result<String, String> {
    let domain = require_str(args, "domain")?;
    let problem = require_str(args, "problem")?;
    let plan = require_str(args, "plan")?;
    match ferroplan::plan::validate_plan(domain, problem, plan)? {
        ferroplan::plan::Validity::Valid => Ok("Plan valid".to_string()),
        ferroplan::plan::Validity::Invalid(why) => Ok(format!("Plan invalid: {why}")),
    }
}

fn tool_decompose(args: &Value) -> Result<String, String> {
    let domain = require_str(args, "domain")?;
    let problem = require_str(args, "problem")?;
    let opts = parse_options(args)?;
    let dec = ferroplan::decompose(domain, problem, &opts).map_err(|e| e.to_string())?;
    pretty(&dec)
}

// --- helpers ---------------------------------------------------------------

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing required string argument `{key}`"))
}

/// Deserialize the optional `options` object into [`ferroplan::Options`] (partial
/// objects are fine — omitted fields use defaults); absent ⇒ defaults.
fn parse_options(args: &Value) -> Result<ferroplan::Options, String> {
    match args.get("options") {
        None | Some(Value::Null) => Ok(ferroplan::Options::default()),
        Some(v) => serde_json::from_value(v.clone()).map_err(|e| format!("invalid options: {e}")),
    }
}

fn pretty<T: serde::Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string_pretty(v).map_err(|e| e.to_string())
}

fn text_block(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

fn error_obj(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Write one JSON-RPC message as a single line (newline-delimited stdio transport).
fn send(out: &mut impl Write, msg: &Value) {
    // A serialized JSON value never contains a raw newline, so one line per message.
    if writeln!(out, "{msg}").is_ok() {
        let _ = out.flush();
    }
}
