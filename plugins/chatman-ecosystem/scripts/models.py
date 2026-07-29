#!/usr/bin/env python3
"""Typed result models for every value this plugin emits.

Machine-first: a command's return value is a model, not a string. JSON is the
canonical serialization, human text is a projection of it (see `emit.py`), and
the JSON Schema is generated from the model rather than maintained beside it.

Why this exists. The plugin emitted roughly thirty distinct payloads tagged with
schema URNs that were bare string literals scattered across the scripts, with no
registry, no schema documents, and no validation. Several emissions carried no
tag at all. A consumer had no way to know what a command would return, and a
producer had no way to find out it had drifted.

The rule enforced here: a payload's `schema` field is not decoration a caller
supplies, it is the model's identity. `ChatmanModel` stamps it on construction
and *rejects* a mismatched one on parse, so a schema URN cannot silently
disagree with the shape it labels.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any, ClassVar, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _standing import Standing, StandingReason  # noqa: E402

# --------------------------------------------------------------------------
# base
# --------------------------------------------------------------------------


class ChatmanModel(BaseModel):
    """A schema-tagged payload.

    `extra="forbid"` is deliberate: these payloads are evidence, and a field
    nobody declared is a field nobody verified.
    """

    model_config = ConfigDict(populate_by_name=True, extra="forbid")

    #: The URN identifying this payload shape. Set once per subclass.
    SCHEMA: ClassVar[str] = ""

    # Aliased because `schema` collides with pydantic's own BaseModel surface,
    # while the wire format has used the bare key `schema` since v1 and must
    # not change.
    schema_urn: str = Field(default="", alias="schema", serialization_alias="schema")

    @model_validator(mode="after")
    def _stamp_or_check_schema(self):
        declared = type(self).SCHEMA
        if not declared:
            raise TypeError(f"{type(self).__name__} does not declare a SCHEMA urn")
        if not self.schema_urn:
            object.__setattr__(self, "schema_urn", declared)
        elif self.schema_urn != declared:
            raise ValueError(
                f"schema mismatch: payload declares {self.schema_urn!r} "
                f"but {type(self).__name__} is {declared!r}"
            )
        return self

    def to_wire(self) -> dict[str, Any]:
        """The canonical JSON-ready dict, keyed as it appears on the wire."""
        return self.model_dump(by_alias=True, mode="json")

    @classmethod
    def json_schema(cls) -> dict[str, Any]:
        return cls.model_json_schema(by_alias=True)


# --------------------------------------------------------------------------
# errors -- machine-first, because a failure is a result too
# --------------------------------------------------------------------------


class ChatmanError(ChatmanModel):
    """A failure, as data.

    Every refusal and every resolution failure becomes one of these. The prose
    a human reads is rendered from `code` and `context`, not the other way
    round, so a caller can branch on `code` without parsing English.
    """

    SCHEMA: ClassVar[str] = "urn:chatman:error:v1"

    code: str = Field(description="Stable machine-readable identifier, SCREAMING_SNAKE.")
    message: str = Field(description="One-line human summary. Never parsed by consumers.")
    context: dict[str, Any] = Field(default_factory=dict)
    remedy: str | None = Field(
        default=None, description="A command or action that would clear this failure."
    )


# --------------------------------------------------------------------------
# resolution -- roots.py
# --------------------------------------------------------------------------


class ResolutionAttempt(BaseModel):
    model_config = ConfigDict(extra="forbid")

    candidate: str
    provenance: str | None = None
    outcome: str = Field(description="Why this candidate was rejected, or 'accepted'.")


class BinaryResolution(ChatmanModel):
    """Where an executable was found, and everything tried on the way.

    The attempt list is part of the success payload, not just the failure path:
    knowing a binary resolved via `cargo run` rather than `target/release` is
    the difference between a fast call and a rebuild.
    """

    SCHEMA: ClassVar[str] = "urn:chatman:binary-resolution:v1"

    binary: str
    resolved: bool
    argv: list[str] = Field(default_factory=list)
    how: str | None = None
    project_root: str | None = None
    attempts: list[ResolutionAttempt] = Field(default_factory=list)
    environment: dict[str, str | None] = Field(
        default_factory=dict,
        description="Steering variables as seen. null means unset, '' means set-but-empty.",
    )


class RootCandidate(BaseModel):
    model_config = ConfigDict(extra="forbid")

    provenance: str
    path: str
    usable: bool


class RootsReport(ChatmanModel):
    """How each root resolves, and by which rule."""

    SCHEMA: ClassVar[str] = "urn:chatman:roots-report:v1"

    plugin_root: str
    project_root: str | None
    project_candidates: list[RootCandidate] = Field(default_factory=list)
    target_dirs: list[str] = Field(default_factory=list)
    environment: dict[str, str | None] = Field(default_factory=dict)


# --------------------------------------------------------------------------
# phase
# --------------------------------------------------------------------------

PhaseDimension = Literal[
    "epistemic", "allocation", "planning", "actuation", "drift", "conformance"
]


class PhaseVector(BaseModel):
    model_config = ConfigDict(extra="forbid")

    epistemic: str
    allocation: str
    planning: str
    actuation: str
    drift: str
    conformance: str


class ActiveProjection(BaseModel):
    model_config = ConfigDict(extra="forbid")

    capabilities: list[str] = Field(default_factory=list)
    agents: list[str] = Field(default_factory=list)
    skills: list[str] = Field(default_factory=list)


class CombinationCensus(BaseModel):
    """Derived counts over the phase product space.

    Both numbers are computed, never read from a literal. `declared_raw` carries
    the profile's own claim so a drift between declaration and derivation is
    visible rather than believed.
    """

    model_config = ConfigDict(extra="forbid")

    raw: int
    lawful: int
    ratio: float
    declared_raw: int | None = None
    declared_raw_matches: bool
    lawful_per_state: dict[str, dict[str, int]] = Field(default_factory=dict)


# --------------------------------------------------------------------------
# ledger
# --------------------------------------------------------------------------


class LoopState(ChatmanModel):
    SCHEMA: ClassVar[str] = "urn:chatman:claude-code-loop-state:v1"

    project: str
    event_count: int = 0
    admitted_event_count: int = 0
    plan_receipt: str | None = None
    plan_digest: str | None = None
    session_id: str | None = None
    # Typed against the generated vocabulary rather than a local Literal, which
    # is how this field previously carried only four of the six values.
    standing: Standing = Standing.UNKNOWN
    updated_at_unix_ms: int = 0

    @property
    def pending_events(self) -> int:
        return max(0, self.event_count - self.admitted_event_count)


class MonitorTick(ChatmanModel):
    """One line of a monitor stream.

    Previously untagged on both monitors, which made the two streams
    indistinguishable to a consumer reading a merged log.
    """

    SCHEMA: ClassVar[str] = "urn:chatman:claude-code-monitor-tick:v1"

    stream: Literal["receipt-frontier", "phase-frontier"]
    project: str
    observed_at_unix_ms: int
    payload: dict[str, Any] = Field(default_factory=dict)


# --------------------------------------------------------------------------
# Gall checkpoints
# --------------------------------------------------------------------------


class Witness(BaseModel):
    """A named, executing piece of evidence.

    `why_this_counts` is required rather than optional: a test name alone does
    not say what it establishes, and a witness whose relevance cannot be stated
    in a sentence is usually not a witness.
    """

    model_config = ConfigDict(extra="forbid")

    test: str = Field(description="Test id or command that produces this evidence.")
    file: str = Field(description="Where it lives, as path:line.")
    result: str
    why_this_counts: str


class GallCheckpointReceipt(ChatmanModel):
    """Evidence for one Gall checkpoint.

    A Gall checkpoint is "the smallest closed, receipted transformation proving
    one complete category transition with explicit inputs, outputs, refusals,
    and verification" (mfw, 15-galls-law-evolutionary-construction.omdoc:37).

    Two rules from the sibling repositories are encoded as fields here rather
    than left to prose:

    * `negative_falsifier` is nullable, and null is a *claim* -- that no
      executing negative fixture exists yet. bcinr's rule is that a falsifier
      must be "a genuine Gall-checkpoint negative fixture, not a comment
      describing one", so a checkpoint with a null falsifier cannot be ALIVE.
    * `replayed_outside_session` exists because wasm4pm's promotion law
      requires replay in a separate session from the one that produced the
      evidence. A checkpoint cannot certify itself.
    """

    SCHEMA: ClassVar[str] = "urn:chatman:gall-checkpoint-receipt:v1"

    checkpoint: str = Field(description="Stable id, e.g. CE-GALL-23.")
    title: str
    git_revision: str = Field(description="The exact revision this evidence was taken at.")
    date: str
    standing: Standing
    reason: StandingReason | None = Field(
        default=None, description="Why the standing is capped below ALIVE."
    )
    blocked_by: list[str] = Field(
        default_factory=list, description="Checkpoint ids this one waits on."
    )

    command: str | None = None
    cwd: str | None = None
    exit_code: int | None = None

    known_anti_patterns: list[str] = Field(
        default_factory=list,
        description="Self-declared non-claims, so the receipt does not overclaim.",
    )
    positive_witness: Witness | None = None
    negative_falsifier: Witness | None = Field(
        default=None,
        description="Null means none exists yet -- which bars promotion to ALIVE.",
    )

    replayed_outside_session: bool = False
    replay_host: str | None = None
    sealed_at_commit: str | None = None

    def promotion_blockers(self) -> list[str]:
        """Why this receipt may not claim ALIVE. Empty means it may.

        The promotion law, as a computation rather than a convention.
        """
        blockers: list[str] = []
        if not self.replayed_outside_session:
            blockers.append("not replayed outside the originating session")
        if self.negative_falsifier is None:
            blockers.append("no executing negative falsifier")
        if not self.sealed_at_commit:
            blockers.append("no sealed commit")
        return blockers


#: Registry consumed by the schema exporter and by the conformance tests, so a
#: new model cannot be added without its schema being generated and checked.
REGISTRY: tuple[type[ChatmanModel], ...] = (
    ChatmanError,
    BinaryResolution,
    RootsReport,
    LoopState,
    MonitorTick,
    GallCheckpointReceipt,
)


def registry_by_urn() -> dict[str, type[ChatmanModel]]:
    return {model.SCHEMA: model for model in REGISTRY}
