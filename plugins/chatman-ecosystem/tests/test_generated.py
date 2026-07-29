"""Generated projections must be current, and must actually describe reality.

Two distinct properties, both required. A committed schema that is stale
describes an older payload; a committed schema that no emitted payload
satisfies describes nothing at all. Publishing either is worse than publishing
none, because a consumer would trust it.
"""

from __future__ import annotations

import json

import generate
import pytest
from conftest import minimal_model
from jsonschema import Draft202012Validator
from models import REGISTRY, ChatmanModel
from roots import plugin_root


def test_committed_projections_are_current():
    """The same check CI runs. Fails loudly rather than regenerating silently."""
    root = plugin_root()
    stale = [
        str(p.path.relative_to(root))
        for p in generate.all_projections(root)
        if not p.path.is_file() or p.path.read_text(encoding="utf-8") != p.content
    ]
    assert not stale, (
        f"stale or missing generated files: {stale}. Run: python3 scripts/generate.py build"
    )


@pytest.mark.parametrize("model_type", REGISTRY, ids=lambda m: m.SCHEMA)
def test_every_model_has_a_committed_schema(model_type: type[ChatmanModel]):
    path = plugin_root() / "schemas" / generate.urn_to_filename(model_type.SCHEMA)
    assert path.is_file(), f"no committed schema for {model_type.SCHEMA}"
    schema = json.loads(path.read_text(encoding="utf-8"))
    assert schema["$id"] == model_type.SCHEMA
    assert schema["$schema"] == generate.SCHEMA_DIALECT


@pytest.mark.parametrize("model_type", REGISTRY, ids=lambda m: m.SCHEMA)
def test_committed_schema_is_itself_valid(model_type: type[ChatmanModel]):
    path = plugin_root() / "schemas" / generate.urn_to_filename(model_type.SCHEMA)
    Draft202012Validator.check_schema(json.loads(path.read_text(encoding="utf-8")))


@pytest.mark.parametrize("model_type", REGISTRY, ids=lambda m: m.SCHEMA)
def test_emitted_payload_validates_against_its_committed_schema(model_type):
    """The seam that matters: what we emit must satisfy what we publish."""
    path = plugin_root() / "schemas" / generate.urn_to_filename(model_type.SCHEMA)
    validator = Draft202012Validator(json.loads(path.read_text(encoding="utf-8")))
    errors = sorted(validator.iter_errors(minimal_model(model_type).to_wire()), key=str)
    assert not errors, [e.message for e in errors]


def test_urn_to_filename_keeps_the_version():
    """Two versions of one payload must be publishable side by side."""
    assert (
        generate.urn_to_filename("urn:chatman:claude-code-loop-state:v1")
        == "claude-code-loop-state.v1.json"
    )
    assert generate.urn_to_filename("urn:chatman:error:v2") == "error.v2.json"


def test_check_detects_a_tampered_projection(tmp_path, monkeypatch):
    """A drift check that cannot fail is worse than no drift check."""
    root = plugin_root()
    # Pick a JSON projection explicitly rather than by position: not every
    # projection is JSON (the standing vocabulary is generated Python), and a
    # positional pick silently broke when a new generator was added first.
    target = next(p for p in generate.all_projections(root) if p.path.suffix == ".json")

    original = target.path.read_text(encoding="utf-8")
    tampered = json.loads(original)
    tampered["title"] = "TAMPERED"
    monkeypatch.setattr(
        generate,
        "all_projections",
        lambda _root: [generate.Projection(target.path, json.dumps(tampered) + "\n")],
    )

    stale = [
        p.path
        for p in generate.all_projections(root)
        if p.path.read_text(encoding="utf-8") != p.content
    ]
    assert stale == [target.path]
    assert target.path.read_text(encoding="utf-8") == original, "the check must not mutate"
