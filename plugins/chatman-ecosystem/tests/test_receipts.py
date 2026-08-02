"""Gall checkpoint receipts, and the promotion law as a computation.

A Gall checkpoint is "the smallest closed, receipted transformation proving one
complete category transition with explicit inputs, outputs, refusals, and
verification" (mfw `15-galls-law-evolutionary-construction.omdoc:37`).

Two rules from the sibling repositories are enforced here rather than trusted:

* wasm4pm's **promotion law** -- ALIVE requires exact revision, clean checkout,
  positive witness, negative falsifier, deterministic result, and replay
  *outside the originating session*. A checkpoint may not certify itself.
* bcinr's **falsifier rule** -- the falsifier must be "a genuine
  Gall-checkpoint negative fixture, not a comment describing one". A string
  naming a test that does not exist must not pass.

Note what these tests deliberately do NOT do: they never run a witness command
and write its result back into a receipt. That would let a checkpoint certify
itself inside the session that produced it, which is exactly what the promotion
law forbids. CI checks shape, agreement, and the law; `replayed_outside_session`
is set by a distinct run.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

import generate
import pytest
from _standing import Standing
from jsonschema import Draft202012Validator
from models import GallCheckpointReceipt
from roots import plugin_root

RECEIPTS = plugin_root() / "receipts"
DOC = plugin_root().parent.parent / "docs" / "gall-checkpoints.md"


def receipt_paths() -> list[Path]:
    return sorted(RECEIPTS.glob("CE-GALL-*.json"))


def load(path: Path) -> GallCheckpointReceipt:
    return GallCheckpointReceipt.model_validate(json.loads(path.read_text(encoding="utf-8")))


def test_receipts_exist():
    assert receipt_paths(), "no checkpoint receipts found"


@pytest.mark.parametrize("path", receipt_paths(), ids=lambda p: p.stem)
def test_receipt_validates_against_the_committed_schema(path):
    schema_file = plugin_root() / "schemas" / generate.urn_to_filename(
        GallCheckpointReceipt.SCHEMA
    )
    validator = Draft202012Validator(json.loads(schema_file.read_text(encoding="utf-8")))
    errors = sorted(validator.iter_errors(json.loads(path.read_text(encoding="utf-8"))), key=str)
    assert not errors, [e.message for e in errors]


@pytest.mark.parametrize("path", receipt_paths(), ids=lambda p: p.stem)
def test_receipt_id_matches_its_filename(path):
    assert load(path).checkpoint == path.stem


@pytest.mark.parametrize("path", receipt_paths(), ids=lambda p: p.stem)
def test_standing_is_from_the_canonical_vocabulary(path):
    assert load(path).standing in set(Standing)


# --------------------------------------------------------------------------
# the promotion law
# --------------------------------------------------------------------------


@pytest.mark.parametrize("path", receipt_paths(), ids=lambda p: p.stem)
def test_promotion_law_is_satisfied_by_anything_claiming_alive(path):
    """ALIVE requires replay outside the session, a falsifier, and a seal.

    Run today this passes only because nothing claims ALIVE. Set any receipt's
    standing to ALIVE without replaying it and this fails -- which is what
    makes it a law rather than a formality.
    """
    receipt = load(path)
    if receipt.standing is not Standing.ALIVE:
        return
    assert not receipt.promotion_blockers(), receipt.promotion_blockers()


def test_promotion_law_actually_refuses():
    """The falsifier for the law itself: it must reject a premature ALIVE."""
    premature = GallCheckpointReceipt(
        checkpoint="CE-GALL-99",
        title="claims alive without evidence",
        git_revision="0" * 40,
        date="2026-07-29",
        standing=Standing.ALIVE,
    )
    blockers = premature.promotion_blockers()
    assert "not replayed outside the originating session" in blockers
    assert "no executing negative falsifier" in blockers


def test_nothing_from_this_cycle_claims_alive():
    """Honest state: no clean-worktree replay has been done for any of it."""
    alive = [p.stem for p in receipt_paths() if load(p).standing is Standing.ALIVE]
    assert not alive, f"{alive} claim ALIVE without an out-of-session replay"


@pytest.mark.parametrize("path", receipt_paths(), ids=lambda p: p.stem)
def test_a_capped_standing_says_why(path):
    """A standing below ALIVE without a reason is an unexplained cap."""
    receipt = load(path)
    if receipt.standing is Standing.ALIVE:
        return
    assert receipt.reason is not None, f"{receipt.checkpoint} is capped but gives no reason"


@pytest.mark.parametrize("path", receipt_paths(), ids=lambda p: p.stem)
def test_a_missing_falsifier_is_declared_not_hidden(path):
    """bcinr's rule: absence of a falsifier is a claim, and must be stated."""
    receipt = load(path)
    if receipt.negative_falsifier is not None:
        return
    # Either the receipt says NO_FALSIFIER, or it is a recorded negative whose
    # whole content is that the capability does not exist.
    acceptable = {"NO_FALSIFIER", "MOCKED", "DEPENDENCY_MISSING", "DEFECT_OPEN"}
    assert receipt.reason and receipt.reason.value in acceptable, (
        f"{receipt.checkpoint} has no falsifier and does not say so"
    )


# --------------------------------------------------------------------------
# bcinr's rule: a falsifier that is a string describing a test cannot pass
# --------------------------------------------------------------------------


def collected_test_names() -> set[str]:
    """Every test function defined in the suite.

    Read from source rather than from `pytest --collect-only`, whose quiet
    output is a per-file count rather than node ids. A falsifier names a test
    function, so a function of that name being defined is exactly the property
    to check -- and reading source avoids a subprocess whose rootdir and
    isolation fixture would have to be reasoned about.
    """
    names: set[str] = set()
    for source in (plugin_root() / "tests").glob("test_*.py"):
        for line in source.read_text(encoding="utf-8").splitlines():
            match = re.match(r"def (test_[A-Za-z0-9_]*)", line)
            if match:
                names.add(match.group(1))
    return names


@pytest.mark.parametrize("path", receipt_paths(), ids=lambda p: p.stem)
def test_a_named_falsifier_test_actually_exists(path):
    """A falsifier naming a test that is not in the suite is a comment."""
    receipt = load(path)
    witness = receipt.negative_falsifier
    if witness is None or not witness.test.startswith("test_"):
        return  # prose witnesses (a live observation) are handled above
    assert witness.test in collected_test_names(), (
        f"{receipt.checkpoint} names falsifier {witness.test!r}, which no test provides"
    )


@pytest.mark.parametrize("path", receipt_paths(), ids=lambda p: p.stem)
def test_a_named_positive_witness_test_actually_exists(path):
    receipt = load(path)
    witness = receipt.positive_witness
    if witness is None or not witness.test.startswith("test_"):
        return
    assert witness.test in collected_test_names()


# --------------------------------------------------------------------------
# receipts and the document agree
# --------------------------------------------------------------------------


def test_every_receipt_is_mentioned_in_the_checkpoint_document():
    """A receipt nobody can find from the doc is evidence nobody will read."""
    text = DOC.read_text(encoding="utf-8")
    missing = [p.stem for p in receipt_paths() if p.stem not in text]
    assert not missing, f"receipts absent from {DOC.name}: {missing}"


@pytest.mark.parametrize("path", receipt_paths(), ids=lambda p: p.stem)
def test_receipt_standing_agrees_with_the_document(path):
    """The structural cure for the split vocabulary: one standing per checkpoint.

    The doc and the receipt are two projections of one fact. Where they
    disagree, the checkpoint's standing is undefined.
    """
    receipt = load(path)
    text = DOC.read_text(encoding="utf-8")
    heading = re.search(
        rf"^#+ .*{re.escape(receipt.checkpoint)}\b.*$", text, re.MULTILINE
    )
    assert heading, f"{receipt.checkpoint} has no heading in {DOC.name}"

    section = text[heading.end() :]
    next_heading = re.search(r"^#+ ", section, re.MULTILINE)
    if next_heading:
        section = section[: next_heading.start()]

    declared = re.search(r"\*\*Current standing:\*\*\s*`([A-Z_]+)`", section)
    assert declared, f"{receipt.checkpoint} states no standing in {DOC.name}"
    assert declared.group(1) == receipt.standing.value, (
        f"{receipt.checkpoint}: doc says {declared.group(1)}, "
        f"receipt says {receipt.standing.value}"
    )
