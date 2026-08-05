//! Dense persistent-session control plane.
//!
//! This module exposes the capabilities already protected by `ferroplan::Session`:
//! many independent minds over one grounding, atomic world edits, explicit forced
//! replanning, operator authority, temporal schedules, in-flight actions, immutable
//! checkpoints, exact restore, history, comparison, and optimistic concurrency.

use std::collections::BTreeMap;
use std::sync::Arc;

use ferroplan::Plan;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ErrorData as McpError};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tokio::sync::Mutex as AsyncMutex;

use crate::result::to_result;
use crate::session::{
    chain_receipt, current_plan_valid, digest_value, validate_session_id, ManagedSession,
};
use crate::Ferroplan;

const DEFAULT_HISTORY_LIMIT: usize = 256;
const MAX_HISTORY_LIMIT: usize = 4096;
const DEFAULT_REPLAN_BUDGET: usize = 50_000;
const MAX_REPLAN_BUDGET: usize = 10_000_000;
const MAX_MEMORY_MB: usize = 16_384;

pub(crate) const RESOURCE_TOOLS: &[&str] = &[
    "session_list",
    "session_state",
    "session_set",
    "session_fork",
    "session_replan",
    "session_checkpoint",
    "session_restore",
    "session_verify_checkpoint",
    "session_history",
    "session_compare",
    "session_restrict_ops",
    "session_schedule_fact",
    "session_apply_start",
    "session_elapse",
];

pub(crate) fn ontology_comment(name: &str) -> Option<&'static str> {
    Some(match name {
        "session_list" => "Enumerate live grounded minds deterministically with lineage, epoch, plan, authority, memory, and receipt summaries.",
        "session_state" => "Read selected grounded facts and fluents plus the exact semantic state fingerprint without mutating the mind.",
        "session_set" => "Atomically apply a validated batch of fact, fluent, and goal changes; no partial world edit is observable.",
        "session_fork" => "Create an independent mind over the same shared grounding, preserving state, plan, authority, temporal agenda, and receipt lineage.",
        "session_replan" => "Force a fresh deterministic bounded search, bypassing the retained-suffix fast path while preserving explicit budget law.",
        "session_checkpoint" => "Capture an immutable in-memory checkpoint of the complete grounded mind and bind it to a canonical digest.",
        "session_restore" => "Restore an exact checkpoint into a named live session with explicit replacement and predecessor-receipt lineage.",
        "session_verify_checkpoint" => "Compare a live mind with an immutable checkpoint across semantic state, plan, cursor, authority, and source identity.",
        "session_history" => "Read the bounded canonical event ledger that produced the current session receipt-chain head.",
        "session_compare" => "Compare two live minds under deterministic lock ordering and report state, goal, plan, authority, and lineage divergence.",
        "session_restrict_ops" => "Replace one mind's allowed/denied operator scope; forbidden actions cannot be planned or replayed.",
        "session_schedule_fact" => "Schedule a clock-relative temporal world fact while preserving dynamic-fact and finite-time fences.",
        "session_apply_start" => "Admit the start of a grounded durative action and carry its real pending end into future replans and replay.",
        "session_elapse" => "Advance a temporal mind's relative clock, firing due world events and action ends in deterministic order.",
        _ => return None,
    })
}

struct StoredCheckpoint {
    source_session_id: String,
    source_epoch: u64,
    digest: String,
    snapshot: ManagedSession,
}

#[derive(Clone, Default)]
pub(crate) struct ControlState {
    checkpoints: Arc<AsyncMutex<BTreeMap<String, StoredCheckpoint>>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ListInput {
    #[serde(default)]
    prefix: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct StateInput {
    session_id: String,
    #[serde(default)]
    facts: Vec<String>,
    #[serde(default)]
    fluents: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FactWrite {
    fact: String,
    value: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FluentWrite {
    fluent: String,
    value: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetInput {
    session_id: String,
    #[serde(default)]
    expected_epoch: Option<u64>,
    #[serde(default)]
    facts: Vec<FactWrite>,
    #[serde(default)]
    fluents: Vec<FluentWrite>,
    #[serde(default)]
    goal: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ForkInput {
    session_id: String,
    child_session_id: String,
    #[serde(default)]
    expected_epoch: Option<u64>,
    #[serde(default)]
    replace: bool,
}

fn default_replan_budget() -> usize {
    DEFAULT_REPLAN_BUDGET
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ReplanInput {
    session_id: String,
    #[serde(default)]
    expected_epoch: Option<u64>,
    #[serde(default = "default_replan_budget")]
    max_evaluated: usize,
    #[serde(default)]
    memory_mb: Option<usize>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CheckpointInput {
    session_id: String,
    checkpoint_id: String,
    #[serde(default)]
    expected_epoch: Option<u64>,
    #[serde(default)]
    replace: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RestoreInput {
    checkpoint_id: String,
    session_id: String,
    #[serde(default)]
    expected_epoch: Option<u64>,
    #[serde(default)]
    replace: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct VerifyCheckpointInput {
    checkpoint_id: String,
    session_id: String,
}

fn default_history_limit() -> usize {
    DEFAULT_HISTORY_LIMIT
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct HistoryInput {
    session_id: String,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_history_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CompareInput {
    left_session_id: String,
    right_session_id: String,
    #[serde(default)]
    expected_left_epoch: Option<u64>,
    #[serde(default)]
    expected_right_epoch: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RestrictOpsInput {
    session_id: String,
    #[serde(default)]
    expected_epoch: Option<u64>,
    #[serde(default)]
    allowed_prefixes: Vec<String>,
    #[serde(default)]
    denied_prefixes: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ScheduleFactInput {
    session_id: String,
    #[serde(default)]
    expected_epoch: Option<u64>,
    delay: f64,
    fact: String,
    value: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ApplyStartInput {
    session_id: String,
    #[serde(default)]
    expected_epoch: Option<u64>,
    action: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ElapseInput {
    session_id: String,
    #[serde(default)]
    expected_epoch: Option<u64>,
    delta: f64,
}

#[tool_router(router = session_control_router, vis = "pub")]
impl Ferroplan {
    #[tool(description = "List live persistent sessions with deterministic summaries and lineage.")]
    async fn session_list(
        &self,
        Parameters(input): Parameters<ListInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_list(input).await)
    }

    #[tool(
        description = "Read selected facts/fluents and the semantic fingerprint of a live session."
    )]
    async fn session_state(
        &self,
        Parameters(input): Parameters<StateInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_state(input).await)
    }

    #[tool(
        description = "Atomically apply fact, fluent, and goal changes with optimistic concurrency."
    )]
    async fn session_set(
        &self,
        Parameters(input): Parameters<SetInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_set(input).await)
    }

    #[tool(description = "Fork an independent mind over the parent's shared grounded world.")]
    async fn session_fork(
        &self,
        Parameters(input): Parameters<ForkInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_fork(input).await)
    }

    #[tool(description = "Force a fresh deterministic bounded search, bypassing suffix retention.")]
    async fn session_replan(
        &self,
        Parameters(input): Parameters<ReplanInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_replan(input).await)
    }

    #[tool(description = "Capture an immutable exact checkpoint of a grounded mind.")]
    async fn session_checkpoint(
        &self,
        Parameters(input): Parameters<CheckpointInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_checkpoint(input).await)
    }

    #[tool(description = "Restore an exact checkpoint into a named live session.")]
    async fn session_restore(
        &self,
        Parameters(input): Parameters<RestoreInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_restore(input).await)
    }

    #[tool(description = "Verify a live session against an immutable checkpoint.")]
    async fn session_verify_checkpoint(
        &self,
        Parameters(input): Parameters<VerifyCheckpointInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_verify_checkpoint(input).await)
    }

    #[tool(description = "Read the bounded canonical event ledger behind a session receipt chain.")]
    async fn session_history(
        &self,
        Parameters(input): Parameters<HistoryInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_history(input).await)
    }

    #[tool(description = "Compare two live minds under deterministic lock ordering.")]
    async fn session_compare(
        &self,
        Parameters(input): Parameters<CompareInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_compare(input).await)
    }

    #[tool(description = "Replace a mind's allowed and denied operator prefixes.")]
    async fn session_restrict_ops(
        &self,
        Parameters(input): Parameters<RestrictOpsInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_restrict_ops(input).await)
    }

    #[tool(description = "Schedule a clock-relative fact change in a temporal session.")]
    async fn session_schedule_fact(
        &self,
        Parameters(input): Parameters<ScheduleFactInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_schedule_fact(input).await)
    }

    #[tool(description = "Apply a durative action start and retain its real pending end.")]
    async fn session_apply_start(
        &self,
        Parameters(input): Parameters<ApplyStartInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_apply_start(input).await)
    }

    #[tool(description = "Advance temporal session time and fire due events/action ends.")]
    async fn session_elapse(
        &self,
        Parameters(input): Parameters<ElapseInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_elapse(input).await)
    }
}

impl Ferroplan {
    async fn do_session_list(&self, input: ListInput) -> Result<Value, String> {
        let prefix = input.prefix.unwrap_or_default();
        let entries: Vec<(String, Arc<AsyncMutex<ManagedSession>>)> = {
            let sessions = self.session_state.sessions.lock().await;
            sessions
                .iter()
                .filter(|(id, _)| id.starts_with(&prefix))
                .map(|(id, lock)| (id.clone(), Arc::clone(lock)))
                .collect()
        };
        let mut summaries = Vec::with_capacity(entries.len());
        for (id, lock) in entries {
            let managed = lock.lock().await;
            summaries.push(summary(&id, &managed)?);
        }
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-list:v2",
            "count": summaries.len(),
            "sessions": summaries
        }))
    }

    async fn do_session_state(&self, input: StateInput) -> Result<Value, String> {
        let lock = self.session_state.get(&input.session_id).await?;
        let managed = lock.lock().await;
        let mut facts = Map::new();
        let mut unknown_facts = Vec::new();
        for name in input.facts {
            match managed.session.fact(&name) {
                Some(value) => {
                    facts.insert(name.to_ascii_uppercase(), Value::Bool(value));
                }
                None => unknown_facts.push(name.to_ascii_uppercase()),
            }
        }
        let mut fluents = Map::new();
        let mut unknown_fluents = Vec::new();
        for name in input.fluents {
            match managed.session.fluent(&name) {
                Some(value) => {
                    fluents.insert(name.to_ascii_uppercase(), json!(value));
                }
                None => unknown_fluents.push(name.to_ascii_uppercase()),
            }
        }
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-state:v2",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "state_fingerprint": managed.session.state_fingerprint(),
            "goal_met": managed.session.goal_met(),
            "facts": facts,
            "fluents": fluents,
            "unknown_facts": unknown_facts,
            "unknown_fluents": unknown_fluents,
            "remaining_plan_valid": current_plan_valid(&managed),
            "receipt_chain_head": managed.receipt_head
        }))
    }

    async fn do_session_set(&self, input: SetInput) -> Result<Value, String> {
        if input.facts.is_empty() && input.fluents.is_empty() && input.goal.is_none() {
            return Err("session_set requires at least one fact, fluent, or goal change".into());
        }
        let lock = self.session_state.get(&input.session_id).await?;
        let mut managed = lock.lock().await;
        require_epoch(&managed, input.expected_epoch)?;

        let mut staged = managed.session.fork();
        for item in &input.facts {
            staged.set_fact(&item.fact, item.value)?;
        }
        for item in &input.fluents {
            if !item.value.is_finite() {
                return Err(format!("fluent `{}` must be finite", item.fluent));
            }
            staged.set_fluent(&item.fluent, item.value)?;
        }
        if let Some(goal) = &input.goal {
            staged.set_goal(goal)?;
        }

        managed.session = staged;
        managed.last_plan = None;
        managed.cursor = 0;
        managed.epoch = managed.epoch.saturating_add(1);
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "atomic-set",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "facts": input.facts.iter().map(|x| json!({"fact": x.fact, "value": x.value})).collect::<Vec<_>>(),
            "fluents": input.fluents.iter().map(|x| json!({"fluent": x.fluent, "value": x.value})).collect::<Vec<_>>(),
            "goal_changed": input.goal.is_some(),
            "state_fingerprint": managed.session.state_fingerprint()
        });
        let receipt = chain_receipt(&mut managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-set:v2",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "facts_applied": input.facts.len(),
            "fluents_applied": input.fluents.len(),
            "goal_changed": input.goal.is_some(),
            "goal_met": managed.session.goal_met(),
            "state_fingerprint": managed.session.state_fingerprint(),
            "receipt": receipt
        }))
    }

    async fn do_session_fork(&self, input: ForkInput) -> Result<Value, String> {
        validate_session_id(&input.child_session_id)?;
        if input.session_id == input.child_session_id {
            return Err("child_session_id must differ from session_id".into());
        }
        let mut sessions = self.session_state.sessions.lock().await;
        if sessions.contains_key(&input.child_session_id) && !input.replace {
            return Err(format!(
                "session `{}` already exists; set replace=true to discard it",
                input.child_session_id
            ));
        }
        let parent_lock = sessions
            .get(&input.session_id)
            .cloned()
            .ok_or_else(|| format!("unknown session `{}`", input.session_id))?;
        let mut parent = parent_lock.lock().await;
        require_epoch(&parent, input.expected_epoch)?;

        let parent_event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "forked-child",
            "session_id": input.session_id,
            "child_session_id": input.child_session_id,
            "epoch": parent.epoch
        });
        let parent_receipt = chain_receipt(&mut parent, &parent_event)?;
        let mut child = clone_managed(&parent);
        child.parent_session_id = Some(input.session_id.clone());
        child.generation = parent.generation.saturating_add(1);
        child.receipt_head = Some(parent_receipt.clone());
        let child_event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "fork-created",
            "session_id": input.child_session_id,
            "forked_from": input.session_id,
            "generation": child.generation,
            "epoch": child.epoch,
            "state_fingerprint": child.session.state_fingerprint()
        });
        let child_receipt = chain_receipt(&mut child, &child_event)?;
        let world_bytes = child.session.world_bytes();
        let mind_bytes = child.session.mind_bytes();
        sessions.insert(
            input.child_session_id.clone(),
            Arc::new(AsyncMutex::new(child)),
        );
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-fork:v2",
            "session_id": input.child_session_id,
            "forked_from": input.session_id,
            "generation": parent.generation.saturating_add(1),
            "shared_world_bytes": world_bytes,
            "private_mind_bytes": mind_bytes,
            "parent_receipt": parent_receipt,
            "receipt": child_receipt
        }))
    }

    async fn do_session_replan(&self, input: ReplanInput) -> Result<Value, String> {
        validate_budget(input.max_evaluated, input.memory_mb)?;
        let lock = self.session_state.get(&input.session_id).await?;
        let mut managed = lock.lock().await;
        require_epoch(&managed, input.expected_epoch)?;
        let solution = tokio::task::block_in_place(|| {
            managed
                .session
                .replan_budgeted(input.max_evaluated, input.memory_mb)
        });
        managed.cursor = 0;
        managed.last_plan = solution.plan.clone();
        let solution_value = serde_json::to_value(&solution).map_err(|e| e.to_string())?;
        let plan_digest = digest_plan(managed.last_plan.as_ref())?;
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "forced-replan",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "reason": input.reason,
            "max_evaluated": input.max_evaluated,
            "memory_mb": input.memory_mb,
            "solution_digest": digest_value(&solution_value)?,
            "plan_digest": plan_digest
        });
        let receipt = chain_receipt(&mut managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-replan:v2",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "forced": true,
            "solved": solution.solved,
            "plan_digest": plan_digest,
            "solution": solution_value,
            "receipt": receipt
        }))
    }

    async fn do_session_checkpoint(&self, input: CheckpointInput) -> Result<Value, String> {
        validate_session_id(&input.checkpoint_id)?;
        let lock = self.session_state.get(&input.session_id).await?;
        let mut managed = lock.lock().await;
        require_epoch(&managed, input.expected_epoch)?;
        let mut checkpoints = self.session_control_state.checkpoints.lock().await;
        if checkpoints.contains_key(&input.checkpoint_id) && !input.replace {
            return Err(format!(
                "checkpoint `{}` already exists; set replace=true to discard it",
                input.checkpoint_id
            ));
        }
        let digest = checkpoint_digest(&input.session_id, &managed)?;
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "checkpoint-created",
            "session_id": input.session_id,
            "checkpoint_id": input.checkpoint_id,
            "epoch": managed.epoch,
            "checkpoint_digest": digest
        });
        let receipt = chain_receipt(&mut managed, &event)?;
        checkpoints.insert(
            input.checkpoint_id.clone(),
            StoredCheckpoint {
                source_session_id: input.session_id.clone(),
                source_epoch: managed.epoch,
                digest: digest.clone(),
                snapshot: clone_managed(&managed),
            },
        );
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-checkpoint:v2",
            "checkpoint_id": input.checkpoint_id,
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "checkpoint_digest": digest,
            "receipt": receipt
        }))
    }

    async fn do_session_restore(&self, input: RestoreInput) -> Result<Value, String> {
        validate_session_id(&input.session_id)?;
        let checkpoint = {
            let checkpoints = self.session_control_state.checkpoints.lock().await;
            let stored = checkpoints
                .get(&input.checkpoint_id)
                .ok_or_else(|| format!("unknown checkpoint `{}`", input.checkpoint_id))?;
            StoredCheckpoint {
                source_session_id: stored.source_session_id.clone(),
                source_epoch: stored.source_epoch,
                digest: stored.digest.clone(),
                snapshot: clone_managed(&stored.snapshot),
            }
        };

        let mut sessions = self.session_state.sessions.lock().await;
        let predecessor = match sessions.get(&input.session_id) {
            Some(existing) => {
                if !input.replace {
                    return Err(format!(
                        "session `{}` already exists; set replace=true to restore over it",
                        input.session_id
                    ));
                }
                let current = existing.lock().await;
                require_epoch(&current, input.expected_epoch)?;
                current.receipt_head.clone()
            }
            None => {
                if input.expected_epoch.is_some() {
                    return Err(format!(
                        "cannot apply expected_epoch to absent session `{}`",
                        input.session_id
                    ));
                }
                None
            }
        };

        let mut restored = clone_managed(&checkpoint.snapshot);
        restored.parent_session_id = Some(checkpoint.source_session_id.clone());
        restored.generation = restored.generation.saturating_add(1);
        restored.epoch = restored.epoch.saturating_add(1);
        restored.receipt_head = predecessor.or_else(|| restored.receipt_head.clone());
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "checkpoint-restored",
            "session_id": input.session_id,
            "checkpoint_id": input.checkpoint_id,
            "checkpoint_digest": checkpoint.digest,
            "source_session_id": checkpoint.source_session_id,
            "source_epoch": checkpoint.source_epoch,
            "epoch": restored.epoch,
            "state_fingerprint": restored.session.state_fingerprint()
        });
        let receipt = chain_receipt(&mut restored, &event)?;
        let state_fingerprint = restored.session.state_fingerprint();
        sessions.insert(
            input.session_id.clone(),
            Arc::new(AsyncMutex::new(restored)),
        );
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-restore:v2",
            "session_id": input.session_id,
            "checkpoint_id": input.checkpoint_id,
            "state_fingerprint": state_fingerprint,
            "receipt": receipt
        }))
    }

    async fn do_session_verify_checkpoint(
        &self,
        input: VerifyCheckpointInput,
    ) -> Result<Value, String> {
        let checkpoint = {
            let checkpoints = self.session_control_state.checkpoints.lock().await;
            let stored = checkpoints
                .get(&input.checkpoint_id)
                .ok_or_else(|| format!("unknown checkpoint `{}`", input.checkpoint_id))?;
            checkpoint_view(stored)?
        };
        let lock = self.session_state.get(&input.session_id).await?;
        let managed = lock.lock().await;
        let live = managed_view(&managed)?;
        let differences = compare_views(&checkpoint, &live);
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-checkpoint-verification:v2",
            "checkpoint_id": input.checkpoint_id,
            "session_id": input.session_id,
            "matches": differences.is_empty(),
            "differences": differences,
            "checkpoint": checkpoint,
            "live": live
        }))
    }

    async fn do_session_history(&self, input: HistoryInput) -> Result<Value, String> {
        if input.limit == 0 || input.limit > MAX_HISTORY_LIMIT {
            return Err(format!("limit must be within 1..={MAX_HISTORY_LIMIT}"));
        }
        let lock = self.session_state.get(&input.session_id).await?;
        let managed = lock.lock().await;
        let total = managed.event_log.len();
        let end = input.offset.saturating_add(input.limit).min(total);
        let events = if input.offset < total {
            managed.event_log[input.offset..end].to_vec()
        } else {
            Vec::new()
        };
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-history:v2",
            "session_id": input.session_id,
            "offset": input.offset,
            "returned": events.len(),
            "total": total,
            "next_offset": (end < total).then_some(end),
            "events": events,
            "receipt_chain_head": managed.receipt_head
        }))
    }

    async fn do_session_compare(&self, input: CompareInput) -> Result<Value, String> {
        if input.left_session_id == input.right_session_id {
            let lock = self.session_state.get(&input.left_session_id).await?;
            let managed = lock.lock().await;
            require_epoch(&managed, input.expected_left_epoch)?;
            require_epoch(&managed, input.expected_right_epoch)?;
            let view = managed_view(&managed)?;
            return Ok(json!({
                "schema": "urn:chatman:ferroplan-session-compare:v2",
                "left_session_id": input.left_session_id,
                "right_session_id": input.right_session_id,
                "equivalent": true,
                "differences": [],
                "left": view,
                "right": view
            }));
        }

        let left_lock = self.session_state.get(&input.left_session_id).await?;
        let right_lock = self.session_state.get(&input.right_session_id).await?;
        let (left, right) = if input.left_session_id < input.right_session_id {
            let left = left_lock.lock().await;
            let right = right_lock.lock().await;
            require_epoch(&left, input.expected_left_epoch)?;
            require_epoch(&right, input.expected_right_epoch)?;
            (managed_view(&left)?, managed_view(&right)?)
        } else {
            let right = right_lock.lock().await;
            let left = left_lock.lock().await;
            require_epoch(&left, input.expected_left_epoch)?;
            require_epoch(&right, input.expected_right_epoch)?;
            (managed_view(&left)?, managed_view(&right)?)
        };
        let differences = compare_views(&left, &right);
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-compare:v2",
            "left_session_id": input.left_session_id,
            "right_session_id": input.right_session_id,
            "equivalent": differences.is_empty(),
            "differences": differences,
            "left": left,
            "right": right
        }))
    }

    async fn do_session_restrict_ops(&self, input: RestrictOpsInput) -> Result<Value, String> {
        let allowed = normalize_prefixes(input.allowed_prefixes)?;
        let denied = normalize_prefixes(input.denied_prefixes)?;
        let lock = self.session_state.get(&input.session_id).await?;
        let mut managed = lock.lock().await;
        require_epoch(&managed, input.expected_epoch)?;
        let mut staged = managed.session.fork();
        staged.restrict_ops(|display| {
            let upper = display.to_ascii_uppercase();
            let admitted = allowed.is_empty() || allowed.iter().any(|p| upper.starts_with(p));
            let refused = denied.iter().any(|p| upper.starts_with(p));
            admitted && !refused
        });
        managed.session = staged;
        managed.allowed_ops = allowed.clone();
        managed.denied_ops = denied.clone();
        managed.last_plan = None;
        managed.cursor = 0;
        managed.epoch = managed.epoch.saturating_add(1);
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "operator-scope-replaced",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "allowed_prefixes": allowed,
            "denied_prefixes": denied,
            "state_fingerprint": managed.session.state_fingerprint()
        });
        let receipt = chain_receipt(&mut managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-authority:v2",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "allowed_prefixes": managed.allowed_ops,
            "denied_prefixes": managed.denied_ops,
            "receipt": receipt
        }))
    }

    async fn do_session_schedule_fact(&self, input: ScheduleFactInput) -> Result<Value, String> {
        let lock = self.session_state.get(&input.session_id).await?;
        let mut managed = lock.lock().await;
        require_epoch(&managed, input.expected_epoch)?;
        let mut staged = managed.session.fork();
        staged.set_timed_fact(input.delay, &input.fact, input.value)?;
        managed.session = staged;
        managed.epoch = managed.epoch.saturating_add(1);
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "fact-scheduled",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "delay": input.delay,
            "fact": input.fact,
            "value": input.value,
            "state_fingerprint": managed.session.state_fingerprint()
        });
        let receipt = chain_receipt(&mut managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-schedule:v2",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "state_fingerprint": managed.session.state_fingerprint(),
            "receipt": receipt
        }))
    }

    async fn do_session_apply_start(&self, input: ApplyStartInput) -> Result<Value, String> {
        let lock = self.session_state.get(&input.session_id).await?;
        let mut managed = lock.lock().await;
        require_epoch(&managed, input.expected_epoch)?;
        let mut staged = managed.session.fork();
        staged.apply_start(&input.action)?;
        managed.session = staged;
        managed.last_plan = None;
        managed.cursor = 0;
        managed.epoch = managed.epoch.saturating_add(1);
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "durative-start-applied",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "action": input.action,
            "state_fingerprint": managed.session.state_fingerprint()
        });
        let receipt = chain_receipt(&mut managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-apply-start:v2",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "state_fingerprint": managed.session.state_fingerprint(),
            "receipt": receipt
        }))
    }

    async fn do_session_elapse(&self, input: ElapseInput) -> Result<Value, String> {
        let lock = self.session_state.get(&input.session_id).await?;
        let mut managed = lock.lock().await;
        require_epoch(&managed, input.expected_epoch)?;
        let mut staged = managed.session.fork();
        let broken_intervals = staged.elapse(input.delta)?;
        managed.session = staged;
        managed.last_plan = None;
        managed.cursor = 0;
        managed.epoch = managed.epoch.saturating_add(1);
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "time-elapsed",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "delta": input.delta,
            "broken_intervals": broken_intervals,
            "state_fingerprint": managed.session.state_fingerprint()
        });
        let receipt = chain_receipt(&mut managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-elapse:v2",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "broken_intervals": broken_intervals,
            "goal_met": managed.session.goal_met(),
            "state_fingerprint": managed.session.state_fingerprint(),
            "receipt": receipt
        }))
    }
}

fn validate_budget(max_evaluated: usize, memory_mb: Option<usize>) -> Result<(), String> {
    if max_evaluated == 0 || max_evaluated > MAX_REPLAN_BUDGET {
        return Err(format!(
            "max_evaluated must be within 1..={MAX_REPLAN_BUDGET}"
        ));
    }
    if memory_mb.is_some_and(|value| value > MAX_MEMORY_MB) {
        return Err(format!("memory_mb must be at most {MAX_MEMORY_MB}"));
    }
    Ok(())
}

fn require_epoch(managed: &ManagedSession, expected: Option<u64>) -> Result<(), String> {
    if let Some(expected) = expected {
        if managed.epoch != expected {
            return Err(format!(
                "stale session epoch: expected {expected}, observed {}",
                managed.epoch
            ));
        }
    }
    Ok(())
}

fn normalize_prefixes(values: Vec<String>) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for value in values {
        let value = value
            .trim()
            .trim_matches(|c| c == '(' || c == ')')
            .to_ascii_uppercase();
        if value.is_empty()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b' ' | b':'))
        {
            return Err(format!("operator prefix is not canonical: `{value}`"));
        }
        out.push(value);
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn clone_managed(managed: &ManagedSession) -> ManagedSession {
    ManagedSession {
        session: managed.session.fork(),
        last_plan: managed.last_plan.clone(),
        cursor: managed.cursor,
        epoch: managed.epoch,
        domain_digest: managed.domain_digest.clone(),
        problem_digest: managed.problem_digest.clone(),
        receipt_head: managed.receipt_head.clone(),
        event_log: managed.event_log.clone(),
        parent_session_id: managed.parent_session_id.clone(),
        generation: managed.generation,
        allowed_ops: managed.allowed_ops.clone(),
        denied_ops: managed.denied_ops.clone(),
    }
}

fn digest_plan(plan: Option<&Plan>) -> Result<Option<String>, String> {
    plan.map(|plan| {
        serde_json::to_value(plan)
            .map_err(|e| e.to_string())
            .and_then(|value| digest_value(&value))
    })
    .transpose()
}

fn managed_view(managed: &ManagedSession) -> Result<Value, String> {
    Ok(json!({
        "epoch": managed.epoch,
        "domain_digest": managed.domain_digest,
        "problem_digest": managed.problem_digest,
        "state_fingerprint": managed.session.state_fingerprint(),
        "goal_met": managed.session.goal_met(),
        "cursor": managed.cursor,
        "plan_digest": digest_plan(managed.last_plan.as_ref())?,
        "remaining_plan_valid": current_plan_valid(managed),
        "parent_session_id": managed.parent_session_id,
        "generation": managed.generation,
        "allowed_prefixes": managed.allowed_ops,
        "denied_prefixes": managed.denied_ops,
        "receipt_chain_head": managed.receipt_head
    }))
}

fn checkpoint_view(checkpoint: &StoredCheckpoint) -> Result<Value, String> {
    let mut view = managed_view(&checkpoint.snapshot)?;
    if let Value::Object(object) = &mut view {
        object.insert("checkpoint_digest".into(), json!(checkpoint.digest));
        object.insert(
            "source_session_id".into(),
            json!(checkpoint.source_session_id),
        );
        object.insert("source_epoch".into(), json!(checkpoint.source_epoch));
    }
    Ok(view)
}

fn compare_views(left: &Value, right: &Value) -> Vec<String> {
    const KEYS: &[&str] = &[
        "domain_digest",
        "problem_digest",
        "state_fingerprint",
        "goal_met",
        "cursor",
        "plan_digest",
        "remaining_plan_valid",
        "allowed_prefixes",
        "denied_prefixes",
    ];
    KEYS.iter()
        .filter(|key| left.get(**key) != right.get(**key))
        .map(|key| (*key).to_owned())
        .collect()
}

fn checkpoint_digest(session_id: &str, managed: &ManagedSession) -> Result<String, String> {
    digest_value(&json!({
        "schema": "urn:chatman:ferroplan-session-checkpoint:v2",
        "session_id": session_id,
        "epoch": managed.epoch,
        "domain_digest": managed.domain_digest,
        "problem_digest": managed.problem_digest,
        "state_fingerprint": managed.session.state_fingerprint(),
        "cursor": managed.cursor,
        "plan_digest": digest_plan(managed.last_plan.as_ref())?,
        "parent_session_id": managed.parent_session_id,
        "generation": managed.generation,
        "allowed_prefixes": managed.allowed_ops,
        "denied_prefixes": managed.denied_ops,
        "receipt_chain_head": managed.receipt_head
    }))
}

fn summary(id: &str, managed: &ManagedSession) -> Result<Value, String> {
    Ok(json!({
        "session_id": id,
        "epoch": managed.epoch,
        "parent_session_id": managed.parent_session_id,
        "generation": managed.generation,
        "domain_digest": managed.domain_digest,
        "problem_digest": managed.problem_digest,
        "state_fingerprint": managed.session.state_fingerprint(),
        "goal_met": managed.session.goal_met(),
        "cursor": managed.cursor,
        "plan_length": managed.last_plan.as_ref().map(|p| p.steps.len()),
        "plan_digest": digest_plan(managed.last_plan.as_ref())?,
        "remaining_plan_valid": current_plan_valid(managed),
        "world_bytes": managed.session.world_bytes(),
        "mind_bytes": managed.session.mind_bytes(),
        "allowed_prefixes": managed.allowed_ops,
        "denied_prefixes": managed.denied_ops,
        "event_count": managed.event_log.len(),
        "receipt_chain_head": managed.receipt_head
    }))
}
