"""The machine-first output contract, asserted rather than described."""

from __future__ import annotations

import json

import pytest
from conftest import minimal_model
from emit import Format, serialize
from models import REGISTRY, ChatmanError, ChatmanModel, LoopState, registry_by_urn


@pytest.mark.parametrize("model_type", REGISTRY, ids=lambda m: m.SCHEMA)
def test_every_model_declares_a_urn(model_type: type[ChatmanModel]):
    assert model_type.SCHEMA.startswith("urn:chatman:")
    assert model_type.SCHEMA.count(":v") == 1, "a urn must carry exactly one version segment"


def test_urns_are_unique():
    urns = [m.SCHEMA for m in REGISTRY]
    assert len(urns) == len(set(urns))
    assert len(registry_by_urn()) == len(REGISTRY)


@pytest.mark.parametrize("model_type", REGISTRY, ids=lambda m: m.SCHEMA)
def test_schema_generation_round_trips(model_type: type[ChatmanModel]):
    schema = model_type.json_schema()
    assert schema["type"] == "object"
    # The wire key is `schema`, not the python attribute name. Getting this
    # wrong would publish a schema that no emitted payload satisfies.
    assert "schema" in schema["properties"]
    assert "schema_urn" not in schema["properties"]


def test_payload_is_stamped_without_being_asked():
    error = ChatmanError(code="X", message="y")
    assert error.to_wire()["schema"] == ChatmanError.SCHEMA


def test_mismatched_urn_is_rejected():
    """A urn that disagrees with its shape is the defect this guards against."""
    with pytest.raises(ValueError):
        LoopState.model_validate({"schema": "urn:chatman:something-else:v1", "project": "/x"})


def test_matching_urn_is_accepted():
    state = LoopState.model_validate({"schema": LoopState.SCHEMA, "project": "/x"})
    assert state.project == "/x"


def test_undeclared_fields_are_refused():
    """These payloads are evidence; an unreviewed field is unverified data."""
    with pytest.raises(ValueError):
        LoopState.model_validate({"project": "/x", "surprise": 1})


def test_pending_is_derived_not_stored():
    state = LoopState(project="/x", event_count=129, admitted_event_count=86)
    assert state.pending_events == 43
    assert "pending_events" not in state.to_wire()


def test_pending_never_goes_negative():
    state = LoopState(project="/x", event_count=1, admitted_event_count=5)
    assert state.pending_events == 0


@pytest.mark.parametrize("model_type", REGISTRY, ids=lambda m: m.SCHEMA)
def test_json_is_the_default_format(model_type: type[ChatmanModel]):
    """The default must not depend on tty, environment, or model type."""
    instance = minimal_model(model_type)
    default = serialize(instance)
    assert default == serialize(instance, Format.JSON)
    assert json.loads(default)["schema"] == model_type.SCHEMA


@pytest.mark.parametrize("model_type", REGISTRY, ids=lambda m: m.SCHEMA)
def test_human_format_never_raises(model_type: type[ChatmanModel]):
    """Unrendered models degrade to JSON rather than inventing prose."""
    assert serialize(minimal_model(model_type), Format.HUMAN).strip()


def test_error_human_rendering_carries_code_and_remedy():
    error = ChatmanError(
        code="MCP_UNRESOLVED",
        message="cannot resolve ferroplan-mcp",
        context={"FERROPLAN_ROOT": None},
        remedy="cargo build -p ferroplan-mcp",
    )
    rendered = serialize(error, Format.HUMAN)
    assert "MCP_UNRESOLVED" in rendered
    assert "cargo build -p ferroplan-mcp" in rendered
    # null must read as unset, not as the string "None".
    assert "<unset>" in rendered
