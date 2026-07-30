"""Drive the real `ferroplan-mcp` binary over the canonical CMCA frontier.

CE-GALL-28's receipt records a positive witness -- specific `candidates_digest`
and `input_digest` values, and "correctness 0.1449 top with a 0.112-0.145
spread" -- for `cmca_allocate` over `profiles/work-surfaces.json`. `mcp_client.py`
existed with zero callers in this suite, so that witness was never actually
replayed against the binary it claims to describe.

Replayed here (CE-GALL-35): it does not reproduce. The real `input_digest` is a
full 64-hex BLAKE3 string, not the 8-character value the receipt records, and
`correctness` receives `share: 0.0` -- it is not the top candidate, let alone at
0.1449. This test pins the true, reproducible values so the refutation is an
executing fixture rather than a comment.
"""

from __future__ import annotations

import json

import pytest
from mcp_client import McpClient
from roots import plugin_root

pytestmark = pytest.mark.needs_cargo

FRONTIER_FILE = plugin_root() / "profiles" / "work-surfaces.json"


def _load_candidates() -> list[dict]:
    return json.loads(FRONTIER_FILE.read_text(encoding="utf-8"))["candidates"]


def test_canonical_frontier_allocation_matches_the_real_binary():
    """The true result of `cmca_allocate` over the canonical frontier.

    Pinned by direct observation of the real binary, not derived from
    CE-GALL-28's receipt -- which this test refutes.
    """
    with McpClient() as client:
        result = client.call_tool("cmca_allocate", {"candidates": _load_candidates()})

    assert result["isError"] is False
    payload = result["structuredContent"]["payload"]

    assert payload["input_digest"] == (
        "9e8f0839fd74fe089113679187a2523e95f712329869ed70e763fff907e3d8bf"
    )

    shares = {node["id"]: node["share"] for node in payload["allocations"]}
    assert shares["correctness"] == 0.0
    top_id = max(shares, key=shares.get)
    assert top_id == "planner-core"
    assert shares[top_id] == pytest.approx(0.26995849609375)


def test_ce_gall_28_receipt_digest_does_not_match_the_real_binary():
    """CE-GALL-28's recorded `input_digest` is refuted, not merely stale.

    The receipt's `f0a8d185` is 8 hex characters; `cmca_allocate` returns a
    64-character BLAKE3 digest. No revision of the tool could have produced an
    8-character `input_digest` -- the recorded value was never a real capture.
    """
    receipt = json.loads(
        (plugin_root() / "receipts" / "CE-GALL-28.json").read_text(encoding="utf-8")
    )
    recorded = receipt["positive_witness"]["result"]
    assert "f0a8d185" in recorded
    assert len("f0a8d185") != len(
        "9e8f0839fd74fe089113679187a2523e95f712329869ed70e763fff907e3d8bf"
    )
