//! `ferroplan-mcp` — a Model Context Protocol server exposing the ferroplan
//! planner to an LLM agent, over MCP stdio.
//!
//! This is the README's bet made operational — the agent *authors and
//! supervises* PDDL and the planner runs deterministically — in two shapes:
//!
//! - **one-shot**: `solve` / `parse` / `validate` / `decompose`, each taking
//!   the PDDL it needs and returning a structured result;
//! - **stateful**: `session_*`, which grounds a world ONCE and then takes
//!   incremental updates and rethinks against them. That is the shape a
//!   simulation actually has, and `session_fork` gives every actor its own
//!   mind over one shared grounded world.
//!
//! Built on [`rmcp`], the official MCP Rust SDK, so protocol details
//! (framing, capability negotiation, tool schemas, error conventions) come
//! from the SDK rather than from a hand-rolled loop here. Tool input schemas
//! are DERIVED from the parameter types, which is why the crate turns on
//! ferroplan's `schema` feature: `Options` advertises its real knobs instead
//! of an opaque object.

mod server;

use rmcp::transport::stdio;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // stdout is the transport — anything printed there corrupts the protocol,
    // so diagnostics belong on stderr.
    let service = server::Ferroplan::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
