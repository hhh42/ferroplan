//! `ferroplan-mcp` — a single Model Context Protocol server exposing the
//! ferroplan planner, persistent self-hosting sessions, and Chatman
//! admission receipts to an LLM agent, as 17 MCP tools:
//!
//! - Stateless planning: `solve`, `parse`, `validate`, `decompose` — the
//!   README's bet made operational: the agent *authors and supervises* PDDL
//!   and the planner runs deterministically.
//! - Persistent repository minds: `session_open`, `session_observe`,
//!   `session_set_goal`, `session_think`, `session_advance`,
//!   `session_status`, `session_close`, `cmca_allocate`,
//!   `cmca_allocate_recursive` — ground once, observe admitted drift,
//!   replay the remaining plan, and search only when the suffix no longer
//!   stands. `cmca_allocate_recursive` chains a sequence of admitted CMCA
//!   allocations, each depth binding the previous depth's real receipt
//!   digest.
//! - Canonical evidence admission: `canonical_digest`, `bind_allocation_receipt`,
//!   `bind_plan_receipt`, `verify_receipt` — bind the exact outputs of the
//!   above authorities into replayable BLAKE3 envelopes with explicit
//!   predecessor commitments. This part does not plan, allocate, validate,
//!   or actuate.
//!
//! Transport: MCP stdio via the `rmcp` SDK (async, tokio multi-thread
//! runtime — required by the session tools' `session_think`, which runs its
//! CPU-bound search via `tokio::task::block_in_place` while holding a
//! per-session lock; see `session.rs`). Tool schemas are derived from
//! `schemars::JsonSchema` on each request struct rather than hand-written
//! JSON Schema literals. `resources/*` exposes one resource per tool (17
//! total), under a single unified `ferroplan://tools/<name>` URI scheme,
//! with the tool's semantic description pulled from
//! `plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl` (statically
//! extracted at build time into per-module `*_ONTOLOGY` constants, embedded
//! via `include_str!` — see `build.rs` for why static extraction was chosen
//! over a live SPARQL engine; it still generates three separate per-module
//! files, one per tool group, which this binary's `session`/`admission`
//! modules `include!` directly).
//!
//! This binary previously shipped as three separate binaries
//! (`ferroplan-mcp`, `ferroplan-session-mcp`, `chatman-admission-mcp`),
//! merged into one per the `rmcp`-supported multi-router pattern: each tool
//! group lives in its own module with its own `#[tool_router(router =
//! <name>, vis = "pub")]` `impl Ferroplan` block (`main_router` here,
//! `session::session_router`, `admission::admission_router`), and the
//! merged constructor sums the three `ToolRouter`s (`ToolRouter` implements
//! `Add`/`AddAssign` as a plain map-union merge by tool name — safe here
//! since no tool name collides across the three original servers).

mod admission;
mod result;
mod session;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ErrorData as McpError, ListResourcesResult, ReadResourceRequestParams,
    ReadResourceResult, Resource, ResourceContents, ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{tool, tool_handler, tool_router, RoleServer, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::result::to_result;

// Static per-tool semantic descriptions for this module's own four
// (stateless planning) tools, sourced from
// `plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl`'s `rdfs:comment`
// annotations on the `fp:McpTool` instances for this server. Generated at
// compile time by `build.rs` (not a live TTL/SPARQL parse at startup)
// because the ontology is static per release and a build-time/embedded
// constant is simpler and cheaper than standing up a SPARQL engine for
// four fixed strings — see build.rs for the extraction logic.
include!(concat!(env!("OUT_DIR"), "/main_ontology.rs"));

const MAIN_RESOURCE_TOOLS: &[&str] = &["solve", "parse", "validate", "decompose"];

fn main_ontology_comment(name: &str) -> Option<&'static str> {
    Some(match name {
        "solve" => SOLVE_ONTOLOGY,
        "parse" => PARSE_ONTOLOGY,
        "validate" => VALIDATE_ONTOLOGY,
        "decompose" => DECOMPOSE_ONTOLOGY,
        _ => return None,
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SolveRequest {
    /// PDDL domain source
    domain: String,
    /// PDDL problem source
    problem: String,
    /// Optional solver Options: mode (auto|ff|partition|pddl3|temporal), search
    /// (auto|ehc|best-first|ehc-then-best-first), weight_g, weight_h, threads,
    /// max_evaluated, optimize. Omitted fields use defaults.
    #[serde(default)]
    options: Option<ferroplan::Options>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ParseRequest {
    /// A PDDL domain OR problem source string.
    pddl: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ValidateRequest {
    /// PDDL domain source
    domain: String,
    /// PDDL problem source
    problem: String,
    /// Plan to check: classical `step N: (action args)` lines, or a temporal
    /// `t: (action args) [dur]` plan.
    plan: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DecomposeRequest {
    /// PDDL domain source (durative actions)
    domain: String,
    /// PDDL problem source
    problem: String,
    /// Optional solver Options (see `solve`).
    #[serde(default)]
    options: Option<ferroplan::Options>,
}

#[derive(Clone)]
struct Ferroplan {
    tool_router: ToolRouter<Self>,
    session_state: session::SessionState,
}

impl Ferroplan {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router() + Self::session_router() + Self::admission_router(),
            session_state: session::SessionState::default(),
        }
    }
}

#[tool_router]
impl Ferroplan {
    #[tool(
        description = "Plan a PDDL domain + problem with ferroplan and return the structured \
            Solution (typed steps, makespan/metric, statistics). Handles STRIPS, typing, ADL, \
            numeric fluents, derived axioms, PDDL3 preferences, and PDDL2.1 temporal (durative \
            actions) — mode is auto-detected. A solved:false result is a normal answer, not an \
            error."
    )]
    fn solve(&self, Parameters(req): Parameters<SolveRequest>) -> Result<CallToolResult, McpError> {
        to_result(self.do_solve(req))
    }

    #[tool(
        description = "Syntax-check a PDDL source string and return a structure summary \
            WITHOUT grounding or solving — fast feedback while authoring. Auto-detects domain \
            vs problem; reports ok/error (with a line number) plus name, requirements, and \
            counts (types/predicates/actions, or objects/init/goal/metric). Use to catch PDDL \
            mistakes before `solve`."
    )]
    fn parse(&self, Parameters(req): Parameters<ParseRequest>) -> Result<CallToolResult, McpError> {
        to_result(as_value(&ferroplan::parse(&req.pddl)))
    }

    #[tool(
        description = "Independently validate a plan against a domain + problem under \
            ferroplan's own execution semantics (auto-detects classical vs temporal). Returns \
            whether the plan is executable and goal-reaching, with a reason if not. Use to \
            check a plan you wrote or one solve produced."
    )]
    fn validate(
        &self,
        Parameters(req): Parameters<ValidateRequest>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_validate(req))
    }

    #[tool(
        description = "Decompose a temporal goal too big for one-shot search into ordered, \
            individually-solved contracts, stitched into one validated plan. Returns the \
            inspectable Decomposition: each contract's named sub-goal, sub-plan, and timeline \
            offset, plus the stitched plan. A goal that can't be split falls back to a single \
            monolithic contract (reported honestly)."
    )]
    fn decompose(
        &self,
        Parameters(req): Parameters<DecomposeRequest>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_decompose(req))
    }
}

impl Ferroplan {
    fn do_solve(&self, req: SolveRequest) -> Result<serde_json::Value, String> {
        let opts = req.options.unwrap_or_default();
        let sol = ferroplan::solve(&req.domain, &req.problem, &opts).map_err(|e| e.to_string())?;
        as_value(&sol)
    }

    /// A plan that does not stand is a normal answer (`valid: false` with a
    /// reason), NOT an error: only a domain/problem/plan that cannot be parsed
    /// or grounded — the `?` on `validate_plan` below — is an error result.
    /// `reason` carries `Validity::Invalid`'s inner string verbatim; the
    /// former "Plan invalid: " prefix was presentation, not data.
    fn do_validate(&self, req: ValidateRequest) -> Result<serde_json::Value, String> {
        let (valid, reason) =
            match ferroplan::plan::validate_plan(&req.domain, &req.problem, &req.plan)? {
                ferroplan::plan::Validity::Valid => (true, None),
                ferroplan::plan::Validity::Invalid(why) => (false, Some(why)),
            };
        Ok(json!({
            "schema": "urn:ferroplan:plan-validation:v1",
            "valid": valid,
            "reason": reason,
        }))
    }

    fn do_decompose(&self, req: DecomposeRequest) -> Result<serde_json::Value, String> {
        let opts = req.options.unwrap_or_default();
        let dec =
            ferroplan::decompose(&req.domain, &req.problem, &opts).map_err(|e| e.to_string())?;
        as_value(&dec)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Ferroplan {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            "ferroplan",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "Author a PDDL domain + problem, then call `solve` (or `decompose` for a goal too \
             big for one-shot search) and read the structured result. `validate` independently \
             checks a plan. Open a persistent repository mind with `session_open` and drive it \
             via `session_observe`/`session_set_goal`/`session_think`/`session_advance`/\
             `session_status`/`session_close`; `cmca_allocate` runs the pinned Chatman \
             Multifractal Cascade Allocator. Bind evidence from any of the above into \
             replayable BLAKE3 envelopes with `canonical_digest`/`bind_allocation_receipt`/\
             `bind_plan_receipt`/`verify_receipt`. Read `ferroplan://tools/<name>` resources \
             for semantic (ontology-sourced) descriptions of each tool.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = all_tool_names()
            .into_iter()
            .map(|name| {
                Resource::new(
                    format!("ferroplan://tools/{name}"),
                    format!("{name} (semantic summary)"),
                )
                .with_description(format!(
                    "Ontology-sourced semantics for the `{name}` tool, from \
                     ferroplan-domain.ttl."
                ))
                .with_mime_type("application/json")
            })
            .collect();
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let name = request
            .uri
            .strip_prefix("ferroplan://tools/")
            .ok_or_else(|| McpError::resource_not_found(request.uri.clone(), None))?;
        let ontology_comment = main_ontology_comment(name)
            .or_else(|| session::ontology_comment(name))
            .or_else(|| admission::ontology_comment(name))
            .ok_or_else(|| McpError::resource_not_found(request.uri.clone(), None))?;
        let body = serde_json::json!({
            "tool": name,
            "source": "plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl",
            "rdfs_comment": ontology_comment,
        });
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            serde_json::to_string_pretty(&body).unwrap_or_default(),
            request.uri,
        )]))
    }
}

/// All 17 tool names across the three merged tool groups, in a stable order
/// (stateless planning, then session, then admission).
fn all_tool_names() -> Vec<&'static str> {
    MAIN_RESOURCE_TOOLS
        .iter()
        .chain(session::RESOURCE_TOOLS)
        .chain(admission::RESOURCE_TOOLS)
        .copied()
        .collect()
}

/// Retained generic pretty-printer, distinct from `crate::result::pretty`
/// (which is infallible and `Value`-specific). Kept as the stateless-planning
/// group's text-rendering helper.
#[allow(dead_code)]
fn pretty<T: serde::Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string_pretty(v).map_err(|e| e.to_string())
}

/// Reflect a serializable planner output into the `Result<Value, String>`
/// tool-body convention `crate::result::to_result` consumes.
fn as_value<T: serde::Serialize>(v: &T) -> Result<serde_json::Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = Ferroplan::new()
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| {
            eprintln!("serving error: {e}");
            e
        })?;
    service.waiting().await?;
    Ok(())
}
