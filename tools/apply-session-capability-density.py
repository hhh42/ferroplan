from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    p = ROOT / path
    text = p.read_text()
    found = text.count(old)
    if found < count:
        raise SystemExit(
            f"PATCH_ANCHOR_MISSING {path}: wanted {count}, found {found}: {old[:100]!r}"
        )
    p.write_text(text.replace(old, new, count))


replace(
    "crates/ferroplan/Cargo.toml",
    "serde = { workspace = true }\nthiserror = { workspace = true }",
    "serde = { workspace = true }\nthiserror = { workspace = true }\nblake3 = { workspace = true }",
)

state_fingerprint = r'''
    /// Canonical BLAKE3 identity of the complete mutable mind state.
    ///
    /// Immutable grounding columns are bound separately by the MCP layer's
    /// domain/problem digests. This digest commits to every mutable planning
    /// input: facts, fluents, definedness, goal, operator mask, scheduled
    /// world events, and in-flight action ends.
    pub fn state_fingerprint(&self) -> String {
        const DOMAIN: &[u8] = b"urn:ferroplan:session-state:v1\0";
        let mut hasher = blake3::Hasher::new();
        let mut frame = |bytes: &[u8]| {
            hasher.update(&(bytes.len() as u64).to_be_bytes());
            hasher.update(bytes);
        };
        for word in &self.task.init_bits {
            frame(&word.to_be_bytes());
        }
        for value in &self.task.fv0 {
            frame(&value.to_bits().to_be_bytes());
        }
        for defined in &self.task.fdef0 {
            frame(&[*defined as u8]);
        }
        for fact in &self.task.goal_pos {
            frame(&fact.to_be_bytes());
        }
        frame(format!("{:?}", self.task.goal_num).as_bytes());
        for forbidden in &self.forbidden {
            frame(&[*forbidden as u8]);
        }
        for (delay, fact, value) in &self.timed {
            frame(&delay.to_bits().to_be_bytes());
            frame(&fact.to_be_bytes());
            frame(&[*value as u8]);
        }
        for (remaining, end_op) in &self.running {
            frame(&remaining.to_bits().to_be_bytes());
            frame(&(*end_op as u64).to_be_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

'''
replace(
    "crates/ferroplan/src/session.rs",
    "    /// The temporal think (0.12 Phase 1): rebuild the duration table against\n",
    state_fingerprint
    + "    /// The temporal think (0.12 Phase 1): rebuild the duration table against\n",
)

managed_old = '''struct ManagedSession {
    session: Session,
    last_plan: Option<Plan>,
    cursor: usize,
    epoch: u64,
    domain_digest: String,
    problem_digest: String,
    receipt_head: Option<String>,
}'''
managed_new = '''pub(crate) struct ManagedSession {
    pub(crate) session: Session,
    pub(crate) last_plan: Option<Plan>,
    pub(crate) cursor: usize,
    pub(crate) epoch: u64,
    pub(crate) domain_digest: String,
    pub(crate) problem_digest: String,
    pub(crate) receipt_head: Option<String>,
    pub(crate) event_log: Vec<Value>,
    pub(crate) parent_session_id: Option<String>,
    pub(crate) generation: u64,
    pub(crate) allowed_ops: Vec<String>,
    pub(crate) denied_ops: Vec<String>,
}'''
replace("crates/ferroplan-mcp/src/session.rs", managed_old, managed_new)
replace(
    "crates/ferroplan-mcp/src/session.rs",
    "    sessions: Arc<AsyncMutex<BTreeMap<String, Arc<AsyncMutex<ManagedSession>>>>>,",
    "    pub(crate) sessions: Arc<AsyncMutex<BTreeMap<String, Arc<AsyncMutex<ManagedSession>>>>>,",
)
replace(
    "crates/ferroplan-mcp/src/session.rs",
    "    async fn get(&self, id: &str) -> Result<Arc<AsyncMutex<ManagedSession>>, String> {",
    "    pub(crate) async fn get(&self, id: &str) -> Result<Arc<AsyncMutex<ManagedSession>>, String> {",
)
replace(
    "crates/ferroplan-mcp/src/session.rs",
    '''            receipt_head: None,
        };''',
    '''            receipt_head: None,
            event_log: Vec::new(),
            parent_session_id: None,
            generation: 0,
            allowed_ops: Vec::new(),
            denied_ops: Vec::new(),
        };''',
)
for name in ["current_plan_valid", "chain_receipt", "validate_session_id", "digest_value"]:
    replace(
        "crates/ferroplan-mcp/src/session.rs",
        f"fn {name}(",
        f"pub(crate) fn {name}(",
    )

chain_old = '''pub(crate) fn chain_receipt(managed: &mut ManagedSession, event: &Value) -> Result<String, String> {
    let event_digest = digest_value(&canonicalize(event))?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(SESSION_RECEIPT_DOMAIN);
    update_framed(
        &mut hasher,
        managed.receipt_head.as_deref().unwrap_or("").as_bytes(),
    );
    update_framed(&mut hasher, event_digest.as_bytes());
    let receipt = hasher.finalize().to_hex().to_string();
    managed.receipt_head = Some(receipt.clone());
    Ok(receipt)
}'''
chain_new = '''pub(crate) fn chain_receipt(managed: &mut ManagedSession, event: &Value) -> Result<String, String> {
    let canonical_event = canonicalize(event);
    let event_digest = digest_value(&canonical_event)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(SESSION_RECEIPT_DOMAIN);
    update_framed(
        &mut hasher,
        managed.receipt_head.as_deref().unwrap_or("").as_bytes(),
    );
    update_framed(&mut hasher, event_digest.as_bytes());
    let receipt = hasher.finalize().to_hex().to_string();
    managed.receipt_head = Some(receipt.clone());
    managed.event_log.push(canonical_event);
    if managed.event_log.len() > 4096 {
        let overflow = managed.event_log.len() - 4096;
        managed.event_log.drain(..overflow);
    }
    Ok(receipt)
}'''
replace("crates/ferroplan-mcp/src/session.rs", chain_old, chain_new)

replace(
    "crates/ferroplan-mcp/src/main.rs",
    "mod session;\n",
    "mod session;\nmod session_control;\n",
)
replace(
    "crates/ferroplan-mcp/src/main.rs",
    '''struct Ferroplan {
    tool_router: ToolRouter<Self>,
    session_state: session::SessionState,
}''',
    '''struct Ferroplan {
    tool_router: ToolRouter<Self>,
    session_state: session::SessionState,
    session_control_state: session_control::ControlState,
}''',
)
replace(
    "crates/ferroplan-mcp/src/main.rs",
    "            tool_router: Self::tool_router() + Self::session_router() + Self::admission_router(),\n            session_state: session::SessionState::default(),",
    "            tool_router: Self::tool_router()\n                + Self::session_router()\n                + Self::session_control_router()\n                + Self::admission_router(),\n            session_state: session::SessionState::default(),\n            session_control_state: session_control::ControlState::default(),",
)
replace(
    "crates/ferroplan-mcp/src/main.rs",
    '''            .or_else(|| session::ontology_comment(name))
            .or_else(|| admission::ontology_comment(name))''',
    '''            .or_else(|| session::ontology_comment(name))
            .or_else(|| session_control::ontology_comment(name))
            .or_else(|| admission::ontology_comment(name))''',
)
replace(
    "crates/ferroplan-mcp/src/main.rs",
    '''        .chain(session::RESOURCE_TOOLS)
        .chain(admission::RESOURCE_TOOLS)''',
    '''        .chain(session::RESOURCE_TOOLS)
        .chain(session_control::RESOURCE_TOOLS)
        .chain(admission::RESOURCE_TOOLS)''',
)
main = ROOT / "crates/ferroplan-mcp/src/main.rs"
text = main.read_text()
text = text.replace("17 MCP tools", "31 MCP tools")
text = text.replace("(17\n//! total)", "(31\n//! total)")
text = text.replace("All 17 tool names", "All 31 tool names")
text = text.replace(
    "`session_status`/`session_close`; `cmca_allocate` runs",
    "`session_status`/`session_close`; branch, checkpoint, restore, compare, scope, and drive time through the `session_*` control tools; `cmca_allocate` runs",
)
main.write_text(text)

protocol = ROOT / "crates/ferroplan-mcp/tests/protocol.rs"
text = protocol.read_text()
anchor = '''        "session_advance",
        "session_close",'''
insert = '''        "session_advance",
        "session_apply_start",
        "session_checkpoint",
        "session_close",
        "session_compare",
        "session_elapse",
        "session_fork",
        "session_history",
        "session_list",
        "session_replan",
        "session_restore",
        "session_restrict_ops",
        "session_schedule_fact",
        "session_set",
        "session_state",
        "session_verify_checkpoint",'''
if anchor not in text:
    raise SystemExit("protocol tool anchor missing")
protocol.write_text(text.replace(anchor, insert, 1))

merged = ROOT / "crates/ferroplan-mcp/tests/merged_server.rs"
text = merged.read_text()
text = text.replace("ALL_17_TOOLS", "ALL_31_TOOLS")
text = text.replace("all_17_tools", "all_31_tools")
text = text.replace("exactly_17", "exactly_31")
text = text.replace("17-tool", "31-tool").replace("17-resource", "31-resource")
text = text.replace("17 tools", "31 tools").replace("17 resources", "31 resources")
text = text.replace("17,", "31,").replace("all 17", "all 31").replace("expected 17", "expected 31")
anchor = '''    "session_advance",
    "session_status",'''
insert = '''    "session_advance",
    "session_apply_start",
    "session_checkpoint",
    "session_compare",
    "session_elapse",
    "session_fork",
    "session_history",
    "session_list",
    "session_replan",
    "session_restore",
    "session_restrict_ops",
    "session_schedule_fact",
    "session_set",
    "session_state",
    "session_verify_checkpoint",
    "session_status",'''
if anchor not in text:
    raise SystemExit("merged tool anchor missing")
merged.write_text(text.replace(anchor, insert, 1))

workflow = ROOT / ".github/workflows/ferroplan-harvester.yml"
text = workflow.read_text()
text = text.replace(
    '      - "crates/ferroplan-mcp/src/session.rs"\n',
    '      - "crates/ferroplan-mcp/src/session.rs"\n      - "crates/ferroplan-mcp/src/session_control.rs"\n',
    1,
)
text = text.replace(
    '      - "crates/ferroplan-mcp/tests/session.rs"\n',
    '      - "crates/ferroplan-mcp/tests/session.rs"\n      - "crates/ferroplan-mcp/tests/session_control.rs"\n',
    1,
)
text = text.replace(
    "              'crates/ferroplan-mcp/src/session.rs',\n",
    "              'crates/ferroplan-mcp/src/session.rs',\n              'crates/ferroplan-mcp/src/session_control.rs',\n",
    1,
)
text = text.replace(
    "              'crates/ferroplan-mcp/tests/session.rs',\n",
    "              'crates/ferroplan-mcp/tests/session.rs',\n              'crates/ferroplan-mcp/tests/session_control.rs',\n",
    1,
)
text = text.replace(
    "          cargo test -p ferroplan-mcp --test session\n",
    "          cargo test -p ferroplan-mcp --test session\n          cargo test -p ferroplan-mcp --test session_control\n",
    1,
)
text = text.replace(
    "            crates/ferroplan-mcp/src/session.rs \\\n",
    "            crates/ferroplan-mcp/src/session.rs \\\n            crates/ferroplan-mcp/src/session_control.rs \\\n",
    1,
)
text = text.replace(
    "            crates/ferroplan-mcp/tests/session.rs \\\n",
    "            crates/ferroplan-mcp/tests/session.rs \\\n            crates/ferroplan-mcp/tests/session_control.rs \\\n",
    1,
)
workflow.write_text(text)

print("SESSION_CAPABILITY_DENSITY_PROJECTED")
