//! Generates `const *_ONTOLOGY: &str = "...";` resource-content constants for
//! each MCP tool group in this crate, extracted at compile time from the
//! repository's Ferroplan Turtle ontologies. The prose shown over
//! `resources/read` is therefore generated from admitted RDF sources rather
//! than hand-copied into the runtime and allowed to drift.
//!
//! Parser choice: a hand-rolled line-oriented text scanner, not a full RDF/
//! Turtle crate. The `fp:McpTool` blocks are constrained by convention to one
//! physical line per label/comment and the scanner is build-time only. If the
//! source grows multi-line literals or nested structures around these fields,
//! replace this scanner with a real Turtle parser rather than extending it.

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
    ("CMCA_RECURSIVE_ONTOLOGY", "cmca_allocate_recursive"),
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

const EXPERIENCE_TOOLS: &[(&str, &str)] = &[
    ("DX_MANIFEST_ONTOLOGY", "dx_manifest"),
    ("DX_COMPOSE_ONTOLOGY", "dx_compose"),
    ("DOCTOR_SCAN_ONTOLOGY", "doctor_scan"),
    ("DOCTOR_EXPLAIN_ONTOLOGY", "doctor_explain"),
    ("WIZARD_BOOTSTRAP_ONTOLOGY", "wizard_bootstrap"),
    ("WIZARD_RECIPE_ONTOLOGY", "wizard_recipe"),
    ("QOL_SNAPSHOT_ONTOLOGY", "qol_snapshot"),
    ("QOL_BATCH_ONTOLOGY", "qol_batch"),
    ("TELCO_ENVELOPE_ONTOLOGY", "telco_envelope"),
    ("TELCO_VERIFY_ONTOLOGY", "telco_verify"),
    ("VISION_LATTICE_ONTOLOGY", "vision_lattice"),
];

const FALLBACK: &str = "(ontology extraction fallback: no rdfs:comment could be located for \
    this tool's fp:McpTool individual at build time; see build.rs.)";

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo");
    let domain_relative = "../../plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl";
    let experience_relative =
        "../../plugins/chatman-ecosystem/ontology/ferroplan-experience.ttl";
    let domain_path = Path::new(&manifest_dir).join(domain_relative);
    let experience_path = Path::new(&manifest_dir).join(experience_relative);

    println!("cargo:rerun-if-changed={domain_relative}");
    println!("cargo:rerun-if-changed={experience_relative}");
    println!("cargo:rerun-if-changed=build.rs");

    let domain_ttl = read_ontology(&domain_path, &manifest_dir, domain_relative);
    let experience_ttl = read_ontology(&experience_path, &manifest_dir, experience_relative);
    let mut comments = extract_tool_comments(&domain_ttl);
    comments.extend(extract_tool_comments(&experience_ttl));
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
    write_group(
        &out_dir.join("experience_ontology.rs"),
        EXPERIENCE_TOOLS,
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

fn read_ontology(path: &Path, manifest_dir: &str, relative: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "ferroplan-mcp/build.rs: failed to read ontology at {} (resolved from \
             CARGO_MANIFEST_DIR={manifest_dir}, relative path {relative}): {error}",
            path.display()
        )
    })
}

/// Scan Turtle source for `fp:Tool*` individuals of type `fp:McpTool`, pulling
/// out `rdfs:label "..."` and the first `rdfs:comment "..."` belonging to
/// that tool. Fine-grained field comments are intentionally not captured.
fn extract_tool_comments(ttl: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let lines: Vec<&str> = ttl.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("fp:Tool") && line.contains("a fp:McpTool") {
            if let Some(label) = extract_quoted(line, "rdfs:label \"") {
                let mut j = i;
                let mut found = None;
                while j < lines.len() && j < i + 8 {
                    if let Some(comment) = extract_quoted(lines[j], "rdfs:comment \"") {
                        found = Some(comment);
                        break;
                    }
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

fn extract_quoted(line: &str, marker: &str) -> Option<String> {
    let start = line.find(marker)? + marker.len();
    let rest = &line[start..];
    let mut out = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\\' => {
                if let Some(&next) = chars.peek() {
                    out.push(next);
                    chars.next();
                }
            }
            '"' => return Some(out),
            _ => out.push(character),
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
    generated.push_str(
        "// @generated by ferroplan-mcp/build.rs from Ferroplan Turtle ontologies. Do not edit; do not commit (OUT_DIR only).\n",
    );
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

fn rust_string_literal(value: &str) -> String {
    format!("{value:?}")
}
