# FP-PRD-F5-001 Errata

**Applies to:** `PRD-FORTUNE5-CAPABILITY-READINESS.md`  
**Authority:** This correction supersedes the contradictory sentence in Section 5.  
**Date:** 2026-08-05

## E-001 — MCP+ baseline

The original PRD draft incorrectly stated that `fp.mcpplus` was blocked because no `ferroplan-mcp` crate existed.

The exact repository baseline contains `crates/ferroplan-mcp`, including stateless planning, persistent sessions, experience tools, admission tools, ontology-generated tool resources, and protocol tests.

The corrected readiness statement is:

> At the start of the Fortune 5 readiness program, `fp.mcpplus` is `PARTIAL`, not absent. The implementation exists, but production admission requires bounded frames and inputs, bounded concurrency and sessions, typed terminal failures, candidate-only authority, exact protocol claims, redacted protocol-safe telemetry, deadline/resource enforcement, malformed-input and saturation tests, and exact-source evidence. No existing tool count, receipt-shaped output, or passing subset may promote the capability to `ADMITTED`.

This correction changes no authority boundary:

- MCP+ exposes candidate capabilities.
- Connectivity grants no execution authority.
- Planning is not actuation.
- Existing admission-envelope tools are evidence utilities; they do not replace BRCE, observed consequence, POWL conformance, OCEL evidence, Truex receipt/refusal, or replay.
