//! `ferroplan-mcp` — a Model Context Protocol server exposing the ferroplan planner
//! to an LLM agent: `solve`, `parse`, `validate`, and `decompose` as MCP tools.
//!
//! This is the README's bet made operational — the agent *authors and supervises*
//! PDDL and the planner runs deterministically. The agent writes a domain + problem,
//! calls `solve` (or `decompose` for a too-big goal), reads the structured result,
//! and iterates; `validate` independently checks a plan under ferroplan's semantics.
//!
//! Transport: MCP stdio via the `rmcp` SDK (async, tokio). Tool schemas are derived
//! from `schemars::JsonSchema` on each request struct rather than hand-written JSON
//! Schema literals. `resources/*` exposes one resource per tool with the tool's
//! semantic description pulled from `plugins/chatman-ecosystem/ontology/
//! ferroplan-domain.ttl` (statically extracted at build time into
//! `TOOL_ONTOLOGY_SUMMARY`, embedded via `include_str!` — see `build.rs`/module doc
//! below for why static extraction was chosen over a live SPARQL engine).

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    ErrorData as McpError, ListResourcesResult, ReadResourceRequestParams, ReadResourceResult,
    Resource, ResourceContents, ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{tool, tool_handler, tool_router, RoleServer, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;

// Static per-tool semantic descriptions, sourced from
// `plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl`'s `rdfs:comment`
// annotations on the `fp:McpTool` instances for this server. Generated at
// compile time by `build.rs` (not a live TTL/SPARQL parse at startup)
// because the ontology is static per release and a build-time/embedded
// constant is simpler and cheaper than standing up a SPARQL engine for
// four fixed strings — see build.rs for the extraction logic.
include!(concat!(env!("OUT_DIR"), "/main_ontology.rs"));

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

#[derive(Debug, Clone)]
struct Ferroplan {
    tool_router: ToolRouter<Self>,
}

impl Ferroplan {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
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
    fn solve(
        &self,
        Parameters(req): Parameters<SolveRequest>,
    ) -> Result<String, String> {
        let opts = req.options.unwrap_or_default();
        let sol = ferroplan::solve(&req.domain, &req.problem, &opts).map_err(|e| e.to_string())?;
        pretty(&sol)
    }

    #[tool(
        description = "Syntax-check a PDDL source string and return a structure summary \
            WITHOUT grounding or solving — fast feedback while authoring. Auto-detects domain \
            vs problem; reports ok/error (with a line number) plus name, requirements, and \
            counts (types/predicates/actions, or objects/init/goal/metric). Use to catch PDDL \
            mistakes before `solve`."
    )]
    fn parse(&self, Parameters(req): Parameters<ParseRequest>) -> Result<String, String> {
        pretty(&ferroplan::parse(&req.pddl))
    }

    #[tool(
        description = "Independently validate a plan against a domain + problem under \
            ferroplan's own execution semantics (auto-detects classical vs temporal). Returns \
            whether the plan is executable and goal-reaching, with a reason if not. Use to \
            check a plan you wrote or one solve produced."
    )]
    fn validate(&self, Parameters(req): Parameters<ValidateRequest>) -> Result<String, String> {
        match ferroplan::plan::validate_plan(&req.domain, &req.problem, &req.plan)? {
            ferroplan::plan::Validity::Valid => Ok("Plan valid".to_string()),
            ferroplan::plan::Validity::Invalid(why) => Ok(format!("Plan invalid: {why}")),
        }
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
    ) -> Result<String, String> {
        let opts = req.options.unwrap_or_default();
        let dec =
            ferroplan::decompose(&req.domain, &req.problem, &opts).map_err(|e| e.to_string())?;
        pretty(&dec)
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
             checks a plan. Read `ferroplan://tools/<name>` resources for semantic \
             (ontology-sourced) descriptions of each tool.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = ["solve", "parse", "validate", "decompose"]
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
        let ontology_comment = match name {
            "solve" => SOLVE_ONTOLOGY,
            "parse" => PARSE_ONTOLOGY,
            "validate" => VALIDATE_ONTOLOGY,
            "decompose" => DECOMPOSE_ONTOLOGY,
            _ => return Err(McpError::resource_not_found(request.uri.clone(), None)),
        };
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

fn pretty<T: serde::Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string_pretty(v).map_err(|e| e.to_string())
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
