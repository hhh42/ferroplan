"""CE-GALL-38 evidence: MCP `validate` already returns a structured verdict.

CE-GALL-30 (2026-07-29) recorded that MCP `validate` returned the prose string
`"Plan valid"`, incompatible with `bind_plan_receipt`'s boolean `valid`
requirement, forcing `skills/admit/SKILL.md:15`'s hand-fabricated
`{"valid": true}` construction.

This file pins the *actual* raw responses observed by directly invoking
`mcp__plugin_chatman-ecosystem_ferroplan__validate` in this session (not a
synthetic mock, not a re-derivation from old docs) against a trivial
2-predicate/1-action STRIPS domain:

    (define (domain gall38)
      (:requirements :strips)
      (:predicates (at-a) (at-b))
      (:action move :parameters ()
        :precondition (at-a)
        :effect (and (at-b) (not (at-a)))))

    (define (problem gall38-p)
      (:domain gall38) (:objects)
      (:init (at-a)) (:goal (at-b)))

Valid-plan call (`step 1: (move)`)      -> {"reason":null,"schema":"urn:ferroplan:plan-validation:v1","valid":true}
Invalid-plan call (`step 1: (nonexistent-action)`)
    -> {"reason":"plan action `NONEXISTENT-ACTION ` not a grounded op","schema":"urn:ferroplan:plan-validation:v1","valid":false}

Both are structured JSON with a native boolean `valid` field and a
`urn:ferroplan:plan-validation:v1` schema tag -- exactly what
`bind_plan_receipt`'s `validator_result` needs, with zero string coercion.
"""

from __future__ import annotations

import json

# Raw responses as literally returned by the live MCP `validate` tool call
# during this session, captured verbatim (whitespace/ordering as emitted).
RAW_VALID_RESPONSE = '{"reason":null,"schema":"urn:ferroplan:plan-validation:v1","valid":true}'
RAW_INVALID_RESPONSE = (
    '{"reason":"plan action `NONEXISTENT-ACTION ` not a grounded op",'
    '"schema":"urn:ferroplan:plan-validation:v1","valid":false}'
)


def _validator_result_from_tool_response(raw: str) -> dict:
    """What a caller must do today to build `bind_plan_receipt`'s
    `validator_result` from a live `validate` response: parse JSON and read
    the `valid` field directly. No prose scanning, no substring matching on
    "valid"/"invalid", no hand-fabrication.
    """
    payload = json.loads(raw)
    assert isinstance(payload["valid"], bool), "valid field must already be a bool"
    return payload


def test_validate_response_is_structured_json_not_prose():
    """CE-GALL-30's refuted claim ('validate returns prose \"Plan valid\"')
    does not hold at this commit: the raw response is a JSON object, not a
    bare string."""
    payload = json.loads(RAW_VALID_RESPONSE)
    assert payload["schema"] == "urn:ferroplan:plan-validation:v1"
    assert set(payload.keys()) == {"reason", "schema", "valid"}


def test_valid_plan_yields_boolean_true_directly_usable_by_bind_plan_receipt():
    result = _validator_result_from_tool_response(RAW_VALID_RESPONSE)
    assert result["valid"] is True
    assert result["reason"] is None


def test_invalid_plan_yields_boolean_false_with_reason_no_hand_fabrication():
    """Negative falsifier: an invalid plan (wrong grounded action name)
    against the same domain/problem must not be coerced by string
    inspection -- the tool itself reports valid: false."""
    result = _validator_result_from_tool_response(RAW_INVALID_RESPONSE)
    assert result["valid"] is False
    assert "not a grounded op" in result["reason"]


def test_no_hand_fabrication_required_unlike_ce_gall_30_era():
    """Demonstrates the CE-GALL-30-era workaround (skills/admit/SKILL.md:15's
    hand-fabricated `{"valid": true}`) is no longer the only path: the
    boolean can be read straight off the structured response for both the
    valid and invalid cases, with no prose parsing step in between."""
    for raw, expected in ((RAW_VALID_RESPONSE, True), (RAW_INVALID_RESPONSE, False)):
        payload = json.loads(raw)
        # Directly usable as (part of) bind_plan_receipt's validator_result --
        # no regex, no "in prose" substring check, no manual construction.
        validator_result = {"valid": payload["valid"], "reason": payload["reason"]}
        assert validator_result["valid"] is expected
