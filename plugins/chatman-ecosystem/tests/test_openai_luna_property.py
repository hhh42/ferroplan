from __future__ import annotations

import random
import string

import pytest

from openai_luna_testkit import FakeMcp
from openai_luna_protocol import canonical_json, sha256_digest
from openai_luna_runtime import McpToolRegistry, _negative


def test_tool_name_projection_property() -> None:
    rng = random.Random(5607)
    alphabet = string.ascii_letters + string.digits + " -./:@#$%^&*()[]{}"
    for _ in range(500):
        name = "".join(rng.choice(alphabet) for _ in range(rng.randint(1, 180)))
        client = FakeMcp([{"name": name}], {name: {"ok": True}})
        tool = McpToolRegistry({"srv": client}).discover()[0]
        assert 1 <= len(tool["name"]) <= 64
        assert all(ch.isalnum() or ch in "_-" for ch in tool["name"])


def test_canonical_digest_property_under_key_permutation() -> None:
    rng = random.Random(5613)
    for _ in range(100):
        pairs = [(f"k{i}", rng.randint(-1000, 1000)) for i in range(20)]
        left = dict(pairs)
        rng.shuffle(pairs)
        right = dict(pairs)
        assert canonical_json(left) == canonical_json(right)
        assert sha256_digest(left) == sha256_digest(right)


@pytest.mark.parametrize(
    "value",
    [
        {"valid": False},
        {"nested": [{"admission": "denied"}]},
        {"standing": "BLOCKED"},
        {"ok": False},
        {"verdict": "invalid"},
    ],
)
def test_negative_detector_mutation_sentinels(value: object) -> None:
    assert _negative(value)


@pytest.mark.parametrize(
    "value",
    [
        {"valid": True},
        {"status": "solved"},
        {"admission": "admitted"},
        {"ok": True, "items": [1, 2, 3]},
    ],
)
def test_positive_detector_property(value: object) -> None:
    assert not _negative(value)
