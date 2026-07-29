//! Shared tool-result plumbing for every tool group in this binary.
//!
//! The `Result<Value, String>` tool-body convention is used by the `session`,
//! `admission`, and stateless-planning tool groups alike; `to_result` is the
//! single place that maps it onto rmcp's `CallToolResult`. Success carries the
//! JSON both as pretty text (for models reading `content`) and as
//! `structuredContent` (for callers that consume the object). Failure carries
//! the message as text only — an error result must never set
//! `structured_content`.
//!
//! Note: this module's `pretty` is deliberately distinct from `crate::pretty`,
//! which is a generic fallible `Serialize` → `Result<String, String>` helper.
//! This one is infallible and `Value`-specific.

use rmcp::model::{CallToolResult, ContentBlock, ErrorData as McpError};
use serde_json::Value;

/// Map the `Result<Value, String>` tool-body convention onto rmcp's
/// `CallToolResult`, setting `structuredContent` on success only.
pub(crate) fn to_result(result: Result<Value, String>) -> Result<CallToolResult, McpError> {
    Ok(match result {
        Ok(value) => {
            let mut r = CallToolResult::success(vec![ContentBlock::text(pretty(&value))]);
            r.structured_content = Some(value);
            r
        }
        Err(message) => CallToolResult::error(vec![ContentBlock::text(message)]),
    })
}

/// Pretty-print a JSON value, falling back to its compact form.
pub(crate) fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}
