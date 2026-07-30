"""The canonical CMCA frontier, allocated by the real `ferroplan-mcp` binary.

CE-GALL-28's original positive witness ("correctness 0.1449 top with a
0.112-0.145 spread", `candidates_digest a473833974c74522`, `input_digest
f0a8d185`) does not reproduce: a genuine outside-session replay at the exact
commit it cites (`e2e683d`) gets a different, but internally consistent,
result -- interior (parented) nodes get `share: 0.0` and all mass cascades to
the four leaves, matching what CE-GALL-9 already documented about
`cmca_allocate`'s tree handling. See CE-GALL-35.

This is the first test in the suite to exercise the real MCP binary rather
than the Python control plane alone (`mcp_client.py` existed but had no
caller). `needs_cargo` keeps it out of the plugin-only CI job, which has no
Rust toolchain; it is meant for local/dev runs and the Rust-workspace CI job.
"""

from __future__ import annotations

import shutil

import pytest
import surfaces
from mcp_client import McpClient, McpToolError, tool_structured_result

pytestmark = pytest.mark.needs_cargo

#: Confirmed live against the real binary at commit e2e683d74ed26fcba3bdc671764a81d2862a090c
#: (this session, 2026-07-30, fresh clone, all steering vars cleared). See CE-GALL-35.
EXPECTED_INPUT_DIGEST = "9e8f0839fd74fe089113679187a2523e95f712329869ed70e763fff907e3d8bf"
EXPECTED_CANDIDATES_DIGEST = "0983969d34eb35a19d40621c462befbf5359e41c79694e91f338a91aca01ef0a"

#: Interior nodes -- referenced as another candidate's `parent` -- must get
#: share 0.0; only leaves receive allocation mass (CE-GALL-9).
INTERIOR_IDS = {"correctness", "session-runtime", "mcp-protocol", "claude-plugin"}
LEAF_IDS = {"planner-core", "semantic-projection", "receipt-security", "evidence"}


def _mcp_reachable() -> bool:
    return shutil.which("cargo") is not None


def _allocate(candidates):
    with McpClient(timeout=60) as client:
        alloc = tool_structured_result(client.call_tool("cmca_allocate", {"candidates": candidates}))
        digest = tool_structured_result(client.call_tool("canonical_digest", {"value": candidates}))
    return alloc, digest


@pytest.mark.skipif(not _mcp_reachable(), reason="no cargo toolchain to build/run ferroplan-mcp")
def test_canonical_frontier_allocation_is_pinned_and_deterministic():
    """Same frontier, called twice, must produce identical digests (Checkpoint 8)."""
    profile = surfaces.load_profile()
    candidates = surfaces.candidates(profile)

    try:
        first_alloc, first_digest = _allocate(candidates)
        second_alloc, second_digest = _allocate(candidates)
    except McpToolError as error:
        pytest.skip(f"ferroplan-mcp unreachable: {error}")

    assert first_alloc["payload"]["input_digest"] == second_alloc["payload"]["input_digest"]
    assert first_digest["digest"] == second_digest["digest"]

    assert first_alloc["payload"]["input_digest"] == EXPECTED_INPUT_DIGEST
    assert first_digest["digest"] == EXPECTED_CANDIDATES_DIGEST

    shares = {row["id"]: row["share"] for row in first_alloc["payload"]["allocations"]}
    assert set(shares) == INTERIOR_IDS | LEAF_IDS
    for interior_id in INTERIOR_IDS:
        assert shares[interior_id] == 0.0, f"{interior_id} is an interior node and must get share 0.0"
    for leaf_id in LEAF_IDS:
        assert shares[leaf_id] > 0.0, f"{leaf_id} is a leaf and must receive allocation mass"
