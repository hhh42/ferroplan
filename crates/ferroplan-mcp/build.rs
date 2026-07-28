//! Generates `const *_ONTOLOGY: &str = "...";` resource-content constants for
//! each of the three MCP binaries in this crate, extracted at compile time
//! from `plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl`'s
//! `rdfs:comment` annotations on the `fp:McpTool` individuals — so the prose
//! shown over `resources/read` is generated from the ontology instead of
//! hand-copied and liable to drift.
//!
//! Parser choice: a hand-rolled line-oriented text scanner, not a full RDF/
//! Turtle crate (e.g. oxigraph). The file's `fp:McpTool` blocks are
//! constrained by convention to one physical line per triple/comment (no
//! multi-line string literals, no line-wrapped comments — confirmed by
//! inspecting the file before writing this parser), so a real Turtle parser
//! would add a runtime-irrelevant build dependency and a real RDF graph to
//! extract four fields per tool that a handful of `&str` scans already get
//! correctly. This is a build-time-only judgment call, specific to this
//! file's current shape; if the ontology ever grows multi-line comments or
//! nested blank-node structures, prefer switching to a real parser rather
//! than growing this one.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// (const name, tool label as it appears in `rdfs:label "..."`).
const SESSION_TOOLS: &[(&str, &str)] = &[
    ("OPEN_ONTOLOGY", "session_open"),
    ("OBSERVE_ONTOLOGY", "session_observe"),
    ("SET_GOAL_ONTOLOGY", "session_set_goal"),
    ("THINK_ONTOLOGY", "session_think"),
    ("ADVANCE_ONTOLOGY", "session_advance"),
    ("STATUS_ONTOLOGY", "session_status"),
    ("CLOSE_ONTOLOGY", "session_close"),
    ("CMCA_ONTOLOGY", "cmca_allocate"),
];

const ADMISSION_TOOLS: &[(&str, &str)] = &[
    ("DIGEST_ONTOLOGY", "canonical_digest"),
    ("BIND_ALLOC_ONTOLOGY", "bind_allocation_receipt"),
    ("BIND_PLAN_ONTOLOGY", "bind_plan_receipt"),
    ("VERIFY_ONTOLOGY", "verify_receipt"),
];

const MAIN_TOOLS: &[(&str, &str)] = &[
    ("SOLVE_ONTOLOGY", "solve"),
    ("PARSE_ONTOLOGY", "parse"),
    ("VALIDATE_ONTOLOGY", "validate"),
    ("DECOMPOSE_ONTOLOGY", "decompose"),
];

const FALLBACK: &str = "(ontology extraction fallback: no rdfs:comment could be located for \
    this tool's fp:McpTool individual at build time; see build.rs.)";

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo");
    let ttl_relative = "../../plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl";
    let ttl_path = Path::new(&manifest_dir).join(ttl_relative);

    // Track the ontology source so `cargo build` re-runs this script (and
    // hence regenerates the constants) whenever the TTL changes.
    println!("cargo:rerun-if-changed={ttl_relative}");
    println!("cargo:rerun-if-changed=build.rs");

    let ttl = fs::read_to_string(&ttl_path).unwrap_or_else(|error| {
        panic!(
            "ferroplan-mcp/build.rs: failed to read ontology at {} (resolved from \
             CARGO_MANIFEST_DIR={manifest_dir}, relative path {ttl_relative}): {error}",
            ttl_path.display()
        )
    });

    let comments = extract_tool_comments(&ttl);
    let mut fallbacks_used = Vec::new();

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    write_group(
        &out_dir.join("session_ontology.rs"),
        SESSION_TOOLS,
        &comments,
        &mut fallbacks_used,
    );
    write_group(
        &out_dir.join("admission_ontology.rs"),
        ADMISSION_TOOLS,
        &comments,
        &mut fallbacks_used,
    );
    write_group(
        &out_dir.join("main_ontology.rs"),
        MAIN_TOOLS,
        &comments,
        &mut fallbacks_used,
    );

    if !fallbacks_used.is_empty() {
        println!(
            "cargo:warning=ferroplan-mcp/build.rs: fell back to a placeholder ontology comment \
             for tool(s) with no extractable rdfs:comment: {}",
            fallbacks_used.join(", ")
        );
    }
}

/// Scan the Turtle source for `fp:Tool*` individuals of type `fp:McpTool`,
/// pulling out `rdfs:label "..."` (the tool name used by the binaries) and
/// the first `rdfs:comment "..."` line that follows it (the tool's
/// top-level semantic description; per-field comments inside `[ ... ]`
/// blocks come after it and are intentionally not captured here).
fn extract_tool_comments(ttl: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let lines: Vec<&str> = ttl.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("fp:Tool") && line.contains("a fp:McpTool") {
            if let Some(label) = extract_quoted(line, "rdfs:label \"") {
                // Look ahead for the first rdfs:comment line belonging to
                // this tool (may be on the same line in principle, but in
                // this file it is always the next physical line).
                let mut j = i;
                let mut found = None;
                while j < lines.len() && j < i + 8 {
                    if let Some(comment) = extract_quoted(lines[j], "rdfs:comment \"") {
                        found = Some(comment);
                        break;
                    }
                    // Stop scanning once we've moved past this tool's block
                    // into the next individual.
                    if j > i && lines[j].starts_with("fp:") {
                        break;
                    }
                    j += 1;
                }
                if let Some(comment) = found {
                    out.insert(label, comment);
                }
            }
        }
        i += 1;
    }
    out
}

/// Extract the quoted string following `marker` on `line`, handling `\"`
/// escapes the way this ontology file actually uses them (it doesn't use
/// any inside comments, but this is defensive: it stops at the first
/// unescaped `"`).
fn extract_quoted(line: &str, marker: &str) -> Option<String> {
    let start = line.find(marker)? + marker.len();
    let rest = &line[start..];
    let mut out = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(&next) = chars.peek() {
                    out.push(next);
                    chars.next();
                }
            }
            '"' => return Some(out),
            _ => out.push(c),
        }
    }
    None
}

fn write_group(
    dest: &Path,
    tools: &[(&str, &str)],
    comments: &BTreeMap<String, String>,
    fallbacks_used: &mut Vec<String>,
) {
    let mut generated = String::new();
    generated.push_str("// @generated by ferroplan-mcp/build.rs from ferroplan-domain.ttl. Do not edit; do not commit (OUT_DIR only).\n");
    for (const_name, tool_label) in tools {
        let text = match comments.get(*tool_label) {
            Some(comment) => comment.clone(),
            None => {
                fallbacks_used.push((*tool_label).to_string());
                FALLBACK.to_string()
            }
        };
        generated.push_str(&format!(
            "const {const_name}: &str = {};\n",
            rust_string_literal(&text)
        ));
    }
    fs::write(dest, generated).unwrap_or_else(|error| {
        panic!(
            "ferroplan-mcp/build.rs: failed to write generated ontology file {}: {error}",
            dest.display()
        )
    });
}

/// Render `s` as a Rust string literal via `Debug`, which escapes quotes,
/// backslashes, and control characters correctly for embedding in generated
/// source.
fn rust_string_literal(s: &str) -> String {
    format!("{s:?}")
}
