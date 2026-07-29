"""Standing vocabulary. GENERATED from ontology/chatman-ecosystem.ttl.

Do not edit. Run `python3 scripts/generate.py build` instead.

Three unreconciled vocabularies existed before this file: loop.py accepted
four values, docs/gall-checkpoints.md listed seven, and the canonical set has
six. A standing that cannot be recorded in the ledger is not a standing.
"""

from __future__ import annotations

from enum import StrEnum


class Standing(StrEnum):
    """The six canonical standings, strongest first."""

    #: The declared consequence works and the required evidence is present.
    ALIVE = "ALIVE"
    #: A bounded working checkpoint exists but the larger crown does not.
    PARTIAL_ALIVE = "PARTIAL_ALIVE"
    #: A named external prerequisite prevents lawful progress.
    BLOCKED = "BLOCKED"
    #: The relevant build or projection fails.
    BUILD_BROKEN = "BUILD_BROKEN"
    #: Observation is insufficient to classify standing. Not UNSUPPORTED.
    UNKNOWN = "UNKNOWN"
    #: The required capability or semantic coordinate is absent.
    UNSUPPORTED = "UNSUPPORTED"


class StandingReason(StrEnum):
    """Why a standing is capped. Never a standing in its own right."""

    #: A known defect is recorded and unfixed.
    DEFECT_OPEN = "DEFECT_OPEN"
    #: A named upstream capability does not exist yet.
    DEPENDENCY_MISSING = "DEPENDENCY_MISSING"
    #: A stub or fabricated value stands in for a real check.
    MOCKED = "MOCKED"
    #: No executing negative fixture exists; the positive check may be vacuous.
    NO_FALSIFIER = "NO_FALSIFIER"
    #: Not replayed outside the originating session, so promotion is barred.
    NO_REPLAY = "NO_REPLAY"
    #: A policy fence refused the action; the refusal itself was lawful.
    REFUSED_BY_POLICY = "REFUSED_BY_POLICY"


#: Advisory ordering for the rule that a checkpoint preserves the standing of
#: its predecessors. Not a lattice: UNKNOWN and UNSUPPORTED are both 'no
#: positive claim' and differ in why, not in strength.
RANK: dict[Standing, int] = {
    Standing.ALIVE: 5,
    Standing.PARTIAL_ALIVE: 4,
    Standing.BLOCKED: 3,
    Standing.BUILD_BROKEN: 2,
    Standing.UNKNOWN: 1,
    Standing.UNSUPPORTED: 0,
}

#: Default for a surface that has done work but cannot be promoted.
DEFAULT = Standing.PARTIAL_ALIVE
