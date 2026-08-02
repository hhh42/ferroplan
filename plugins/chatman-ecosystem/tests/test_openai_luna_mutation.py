from __future__ import annotations

import pytest
from openai_luna_protocol import OpenAIResponsesClient, RuntimeRefusal
from openai_luna_runtime import LunaHost
from openai_luna_testkit import (
    FakeStar,
    ScriptedResponses,
    final_response,
    function_call,
    profile,
    standard_surfaces,
    star_envelope,
)


def run(script, *, star=None, surfaces=None):
    responses = ScriptedResponses(script)
    return LunaHost(profile(), OpenAIResponsesClient(transport=responses.create), star or FakeStar(), surfaces or standard_surfaces()).run("execute", "ferroplan")


def test_mutation_remove_planning_witness_blocks() -> None:
    trace = run([function_call("r1", ("o1", "ontostar__onto_admit_work_order", {})), final_response()])
    assert trace["standing"] == "BLOCKED"


def test_mutation_remove_admission_witness_blocks() -> None:
    trace = run([function_call("r1", ("f1", "ferroplan__solve", {})), final_response()])
    assert trace["standing"] == "BLOCKED"


def test_mutation_negative_planning_result_blocks() -> None:
    surfaces = standard_surfaces(planning={"valid": False})
    trace = run([function_call("r1", ("f1", "ferroplan__solve", {}), ("o1", "ontostar__onto_admit_work_order", {})), final_response()], surfaces=surfaces)
    assert trace["standing"] == "BLOCKED"


def test_mutation_negative_admission_result_blocks() -> None:
    surfaces = standard_surfaces(admission={"admission": "denied"})
    trace = run([function_call("r1", ("f1", "ferroplan__solve", {}), ("o1", "ontostar__onto_admit_work_order", {})), final_response()], surfaces=surfaces)
    assert trace["standing"] == "BLOCKED"


def test_mutation_star_actuation_refused_before_mcp() -> None:
    with pytest.raises(RuntimeRefusal) as caught:
        run([final_response()], star=FakeStar(star_envelope(internal_actuation=True)))
    assert caught.value.code == "OSTAR_STAR_ACTUATION_UNBOUNDED"
