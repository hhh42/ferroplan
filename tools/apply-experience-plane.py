from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    target = ROOT / path
    text = target.read_text()
    found = text.count(old)
    if found < count:
        raise SystemExit(
            f"EXPERIENCE_PROJECTION_ANCHOR_MISSING {path}: wanted {count}, found {found}: {old[:120]!r}"
        )
    target.write_text(text.replace(old, new, count))


experience = ROOT / "crates/ferroplan-mcp/src/experience.rs"
text = experience.read_text()
old_ontology = '''pub(crate) fn ontology_comment(name: &str) -> Option<&'static str> {
    Some(match name {
        "dx_manifest" => "Return the complete self-describing Ferroplan capability manifest, including authority categories, contracts, effects, reversibility, receipt behavior, and composition examples.",
        "dx_compose" => "Search the bounded capability graph for a minimal deterministic tool sequence from admitted starting atoms to requested outcome atoms.",
        "doctor_scan" => "Diagnose global or per-session health, assign typed findings, calculate standing, and emit executable remediation hints without mutating state.",
        "doctor_explain" => "Classify a tool or protocol failure into a typed cause with bounded confidence, corrective actions, and refusal-preserving recovery guidance.",
        "wizard_bootstrap" => "Atomically manufacture a ready persistent planning mind from domain, problem, goal, authority scope, and bounded search settings.",
        "wizard_recipe" => "Compile a high-level operator intent into an ordered, inspectable Ferroplan tool recipe with preflight, rollback, and receipt checkpoints.",
        "qol_snapshot" => "Read session state, selected facts and fluents, plan standing, diagnostics, memory, lineage, and recent history in one round trip.",
        "qol_batch" => "Apply a bounded heterogeneous session transaction on a staged fork and commit exactly once, or refuse without partial mutation.",
        "telco_envelope" => "Manufacture a deterministic transport-neutral BLAKE3 integrity envelope with correlation, causation, idempotency, predecessor, and expiry fields; it performs no network operation.",
        "telco_verify" => "Verify a transport envelope's schema, payload identity, envelope identity, routing expectations, predecessor, and expiry without treating integrity as authentication.",
        "vision_lattice" => "Enumerate a bounded combinatorial capability lattice, minimal reachability depths, dependency edges, blocked frontiers, and theoretical composition capacity.",
        _ => return None,
    })
}
'''
new_ontology = '''include!(concat!(env!("OUT_DIR"), "/experience_ontology.rs"));

pub(crate) const ONTOLOGY_SOURCE: &str =
    "plugins/chatman-ecosystem/ontology/ferroplan-experience.ttl";

pub(crate) fn ontology_comment(name: &str) -> Option<&'static str> {
    Some(match name {
        "dx_manifest" => DX_MANIFEST_ONTOLOGY,
        "dx_compose" => DX_COMPOSE_ONTOLOGY,
        "doctor_scan" => DOCTOR_SCAN_ONTOLOGY,
        "doctor_explain" => DOCTOR_EXPLAIN_ONTOLOGY,
        "wizard_bootstrap" => WIZARD_BOOTSTRAP_ONTOLOGY,
        "wizard_recipe" => WIZARD_RECIPE_ONTOLOGY,
        "qol_snapshot" => QOL_SNAPSHOT_ONTOLOGY,
        "qol_batch" => QOL_BATCH_ONTOLOGY,
        "telco_envelope" => TELCO_ENVELOPE_ONTOLOGY,
        "telco_verify" => TELCO_VERIFY_ONTOLOGY,
        "vision_lattice" => VISION_LATTICE_ONTOLOGY,
        _ => return None,
    })
}

pub(crate) fn ontology_source(name: &str) -> Option<&'static str> {
    RESOURCE_TOOLS.contains(&name).then_some(ONTOLOGY_SOURCE)
}
'''
if old_ontology in text:
    text = text.replace(old_ontology, new_ontology, 1)
elif "experience_ontology.rs" not in text:
    raise SystemExit("EXPERIENCE_ONTOLOGY_PROJECTION_REFUSED")
experience.write_text(text)

main = ROOT / "crates/ferroplan-mcp/src/main.rs"
text = main.read_text()
text = text.replace(
    '''                .with_description(format!(
                    "Ontology-sourced semantics for the `{name}` tool, from \\
                     ferroplan-domain.ttl."
                ))''',
    '''                .with_description(format!(
                    "Ontology-sourced semantics for the `{name}` tool, from its owning \\
                     Ferroplan Turtle graph."
                ))''',
)
old_body = '''        let body = serde_json::json!({
            "tool": name,
            "source": "plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl",
            "rdfs_comment": ontology_comment,
        });'''
new_body = '''        let ontology_source = experience::ontology_source(name).unwrap_or(
            "plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl",
        );
        let body = serde_json::json!({
            "tool": name,
            "source": ontology_source,
            "rdfs_comment": ontology_comment,
        });'''
if old_body in text:
    text = text.replace(old_body, new_body, 1)
elif "let ontology_source = experience::ontology_source(name)" not in text:
    raise SystemExit("MAIN_ONTOLOGY_PROVENANCE_PROJECTION_REFUSED")
text = text.replace(
    "/// All 42 tool names across the three merged tool groups, in a stable order",
    "/// All 42 tool names across the five merged tool groups, in a stable order",
)
main.write_text(text)

experience_test = ROOT / "crates/ferroplan-mcp/tests/experience_plane.rs"
text = experience_test.read_text()
anchor = '''    assert_eq!(manifest["advertised_tool_count"], 42);
    assert_eq!(manifest["modeled_tool_count"], 42);
'''
insert = '''    assert_eq!(manifest["advertised_tool_count"], 42);
    assert_eq!(manifest["modeled_tool_count"], 42);

    let resource = client.request(
        "resources/read",
        json!({"uri": "ferroplan://tools/dx_manifest"}),
    );
    let resource_text = resource["result"]["contents"][0]["text"]
        .as_str()
        .expect("experience resource text");
    let resource_body: Value = serde_json::from_str(resource_text).expect("experience resource JSON");
    assert_eq!(
        resource_body["source"],
        "plugins/chatman-ecosystem/ontology/ferroplan-experience.ttl"
    );
    assert!(resource_body["rdfs_comment"]
        .as_str()
        .is_some_and(|comment| comment.contains("self-describing")));
'''
if anchor in text and "ferroplan-experience.ttl" not in text:
    text = text.replace(anchor, insert, 1)
experience_test.write_text(text)

print("EXPERIENCE_RDF_PROJECTION_COMPLETE")
