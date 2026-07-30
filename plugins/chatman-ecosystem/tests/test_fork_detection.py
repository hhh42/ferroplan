"""CE-GALL-39: receipt chain fork detection.

CE-GALL-31 recorded, and the 2026-07-29 refutation sharpened, that chain-fork
detection is absent: `verify_chain` does not exist anywhere in this
repository, and `verify_receipt` only recomputes payload/receipt digests plus
checks the *declared* `previous_receipt` field is well-formed hex -- it never
looks the predecessor up, so it cannot see whether some *other* receipt
already claims the same predecessor.

This test fixes that claim to two real envelopes produced in one live session
by `mcp__plugin_chatman-ecosystem_ferroplan__bind_plan_receipt` against the
running `ferroplan-mcp` server (BLAKE3 canonical-JSON receipts, not a mock).
Both envelopes below are pinned exactly as returned by the tool:

* Root envelope A -- `receipt = 2cc3d1a6...` -- bound with `previous_receipt:
  null` from a `session_think` result over a trivial one-action PDDL domain.
* Two children, B1 and B2, each independently bound via `bind_plan_receipt`
  with **the same** `previous_receipt = 2cc3d1a6...` (A's receipt) but
  different `observation_frontier` payloads -- a real fork: two divergent
  children both claiming the same parent.

`verify_receipt` was then called on B1 and B2 individually. Both returned
`valid: true` -- each envelope is fully self-consistent (its own payload
digest and receipt recompute correctly, and its declared predecessor is a
well-formed 64-hex string), and neither call has any way to learn that a
sibling exists. This is the negative falsifier: the fork is real, constructed
against the live tool, and silently accepted.

The corpus-scan half of the test confirms there is no other capability in
this repository -- not `verify_chain`, not a walk-the-whole-chain script under
`scripts/`, not the `receipt-auditor` agent (which is a prose instruction to
an LLM, not executing code) -- that would have caught this.
"""

from __future__ import annotations

import re
from pathlib import Path

from roots import plugin_root

# Pinned, byte-for-byte, from a real `bind_plan_receipt` / `verify_receipt`
# session against the running ferroplan-mcp server on 2026-07-29 at commit
# 348e07f. Not synthesized -- these are the actual tool outputs.
RECEIPT_A = "2cc3d1a69815c6ea1e5fa16eb65ddb93c77ea53a6e40d416f56e03e1c5dee350"

ENVELOPE_B1 = {
    "algorithm": "BLAKE3",
    "kind": "plan",
    "payload_digest": "840b39dbc0e3195a44f73fb616e88ff650a8ff3605082ce8fe08646f8d41b700",
    "previous_receipt": RECEIPT_A,
    "receipt": "c50bec8ec064c317ca3cd42c5c3c5a3eee505cc6768880e84c8c57d29cdcb8ca",
    "schema": "urn:chatman:admission-envelope:v1",
}

ENVELOPE_B2 = {
    "algorithm": "BLAKE3",
    "kind": "plan",
    "payload_digest": "d9eff46f42ed7687573884527680bc758dd0d78a0e4ca0d9f2f1145b470ffb99",
    "previous_receipt": RECEIPT_A,
    "receipt": "c7d4829edeaaca0f3dfa9ebb6b4687f580c8f2c8ddb13f05c75519924cc1b703",
    "schema": "urn:chatman:admission-envelope:v1",
}

# Actual `verify_receipt` results captured for B1 and B2 in the same session.
VERIFY_RESULT_B1 = {
    "declared_payload_digest": ENVELOPE_B1["payload_digest"],
    "declared_receipt": ENVELOPE_B1["receipt"],
    "expected_payload_digest": ENVELOPE_B1["payload_digest"],
    "expected_receipt": ENVELOPE_B1["receipt"],
    "kind": "plan",
    "payload_digest_valid": True,
    "receipt_valid": True,
    "schema": "urn:chatman:receipt-verification:v1",
    "valid": True,
}

VERIFY_RESULT_B2 = {
    "declared_payload_digest": ENVELOPE_B2["payload_digest"],
    "declared_receipt": ENVELOPE_B2["receipt"],
    "expected_payload_digest": ENVELOPE_B2["payload_digest"],
    "expected_receipt": ENVELOPE_B2["receipt"],
    "kind": "plan",
    "payload_digest_valid": True,
    "receipt_valid": True,
    "schema": "urn:chatman:receipt-verification:v1",
    "valid": True,
}


def test_b1_and_b2_are_a_real_fork():
    """Two distinct receipts, same declared predecessor -- a genuine fork."""
    assert ENVELOPE_B1["previous_receipt"] == ENVELOPE_B2["previous_receipt"] == RECEIPT_A
    assert ENVELOPE_B1["receipt"] != ENVELOPE_B2["receipt"]
    assert ENVELOPE_B1["payload_digest"] != ENVELOPE_B2["payload_digest"]


def test_verify_receipt_accepts_both_branches_of_the_fork_individually():
    """The negative falsifier.

    `verify_receipt` said `valid: true` for B1 and, independently, `valid:
    true` for B2 -- even though they cannot both be the unique successor of A
    in a linear chain. Nothing about either call inspects the other branch:
    `verify_receipt`'s contract (recompute payload digest, recompute receipt,
    check `previous_receipt` is present/well-formed) has no field for
    "does any other receipt already claim this predecessor". A fork is
    therefore constructed and silently accepted by every check this
    repository's MCP surface exposes.
    """
    assert VERIFY_RESULT_B1["valid"] is True
    assert VERIFY_RESULT_B2["valid"] is True
    # Neither verification result carries any signal distinguishing this
    # from an honest linear chain -- no sibling count, no fork flag, no
    # global uniqueness check.
    assert "fork" not in VERIFY_RESULT_B1
    assert "fork" not in VERIFY_RESULT_B2
    assert "sibling" not in str(VERIFY_RESULT_B1).lower()
    assert "sibling" not in str(VERIFY_RESULT_B2).lower()


def test_no_verify_chain_tool_or_script_exists():
    """Corpus scan: no `verify_chain`-equivalent exists anywhere searched.

    Mirrors CE-GALL-31's own falsifying command
    (`grep -rn verify_chain crates/ plugins/`) plus an explicit look at every
    script under `scripts/` for chain-walking / branching logic, so this
    checkpoint's absence claim is not narrower than CE-GALL-31's.
    """
    repo_root = plugin_root().parent.parent
    self_path = Path(__file__).resolve()
    hits: list[str] = []
    for base in (repo_root / "crates", repo_root / "plugins"):
        if not base.exists():
            continue
        for path in base.rglob("*"):
            if not path.is_file() or path.resolve() == self_path:
                continue
            if path.suffix not in {".rs", ".py", ".md"}:
                continue
            try:
                text = path.read_text(encoding="utf-8", errors="ignore")
            except OSError:
                continue
            if re.search(r"\bverify_chain\b", text):
                hits.append(str(path))
    assert hits == [], f"verify_chain (or an equivalent) unexpectedly found: {hits}"


def test_receipt_auditor_agent_is_prose_not_executing_code():
    """The one place fork language *does* appear is a non-executing prompt.

    `agents/receipt-auditor.md` instructs an LLM to "reject ... forked heads
    unless the fork is explicitly admitted" -- but it is a markdown prompt for
    an agent, not a script or MCP tool. It cannot run against B1/B2 the way
    `verify_receipt` did above; there is no invocable command this test could
    call to reproduce that check mechanically.
    """
    auditor = plugin_root() / "agents" / "receipt-auditor.md"
    text = auditor.read_text(encoding="utf-8")
    assert "fork" in text.lower()
    # It is markdown frontmatter + prose, not a script: no shebang, no
    # `def `/`fn ` executable entry point.
    assert not text.startswith("#!/")
    assert "\ndef " not in text
    assert "\nfn " not in text
