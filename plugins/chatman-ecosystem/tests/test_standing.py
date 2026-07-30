"""The standing vocabulary has exactly one source, and every consumer agrees.

Three vocabularies existed before this: `loop.py` accepted four values,
`docs/gall-checkpoints.md` listed seven, and the canonical set has six. So
BLOCKED, MOCKED and REFUSED could be claimed in the doc but never recorded in
the ledger, and BUILD_BROKEN could be recorded but not claimed.

A standing that cannot be recorded in the ledger is not a standing. These tests
make a fourth vocabulary impossible to introduce without failing CI.
"""

from __future__ import annotations

import subprocess
import sys

import generate
import pytest
import rdflib
from _standing import DEFAULT, RANK, Standing, StandingReason
from roots import plugin_root

#: The canonical six, from ~/mfw AGENTS.md:122-133.
CANONICAL = {
    "ALIVE",
    "PARTIAL_ALIVE",
    "BLOCKED",
    "BUILD_BROKEN",
    "UNKNOWN",
    "UNSUPPORTED",
}


def test_generated_enum_is_exactly_the_canonical_six():
    assert {s.value for s in Standing} == CANONICAL


def test_enum_matches_the_ontology_that_generated_it():
    standings, _reasons = generate.standing_vocabulary(plugin_root())
    assert {title for title, _rank, _c in standings} == {s.value for s in Standing}


def test_ranks_are_unique_and_total():
    """Ordering must be decidable, or 'preserves prior standing' is unenforceable."""
    assert len(set(RANK.values())) == len(RANK)
    assert set(RANK) == set(Standing)


def test_alive_outranks_every_other_standing():
    assert all(RANK[Standing.ALIVE] > RANK[s] for s in Standing if s is not Standing.ALIVE)


def test_partial_alive_outranks_the_non_claims():
    """PARTIAL_ALIVE is a real claim; UNKNOWN and UNSUPPORTED are not."""
    for weaker in (Standing.UNKNOWN, Standing.UNSUPPORTED, Standing.BUILD_BROKEN):
        assert RANK[Standing.PARTIAL_ALIVE] > RANK[weaker]


def test_default_is_not_a_promotion():
    """A surface that has done work but cannot be promoted defaults honestly."""
    assert DEFAULT is Standing.PARTIAL_ALIVE
    assert RANK[DEFAULT] < RANK[Standing.ALIVE]


# --------------------------------------------------------------------------
# MOCKED and REFUSED are reasons, not standings
# --------------------------------------------------------------------------


@pytest.mark.parametrize("dropped", ["MOCKED", "REFUSED"])
def test_dropped_values_are_not_standings(dropped):
    """Neither is a standing.

    MOCKED is why a standing is capped: a surface that executes and returns a
    fabricated value partly works, which PARTIAL_ALIVE records and MOCKED would
    lose. REFUSED is a run outcome -- a lawful refusal is the system working, so
    as a standing it would conflate evidence FOR promotion with brokenness.
    """
    assert dropped not in {s.value for s in Standing}


def test_mocked_survives_as_a_reason():
    """Dropping it as a standing must not lose the information."""
    assert StandingReason.MOCKED.value == "MOCKED"


def test_reasons_and_standings_do_not_overlap():
    assert not ({r.value for r in StandingReason} & {s.value for s in Standing})


# --------------------------------------------------------------------------
# every consumer is a projection of the one source
# --------------------------------------------------------------------------


def test_ledger_cli_accepts_every_standing():
    """The defect, directly: loop.py accepted four of the six."""
    proc = subprocess.run(
        [sys.executable, str(plugin_root() / "scripts" / "loop.py"), "admit", "--help"],
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 0, proc.stderr
    for standing in Standing:
        assert standing.value in proc.stdout, f"loop.py cannot record {standing.value}"


def test_loop_state_model_accepts_every_standing():
    from models import LoopState

    for standing in Standing:
        assert LoopState(project="/x", standing=standing).standing is standing


def test_loop_state_model_refuses_an_invented_standing():
    """The falsifier: a seventh value must not slip in through the model."""
    from models import LoopState

    with pytest.raises(ValueError):
        LoopState.model_validate({"project": "/x", "standing": "MOSTLY_FINE"})


def test_published_schema_enumerates_the_standings():
    """A consumer reading only the JSON Schema must see the same six."""
    from models import LoopState

    schema = LoopState.json_schema()
    blob = str(schema)
    for standing in Standing:
        assert standing.value in blob


def test_ontology_declares_a_rank_for_every_standing():
    """Guards the ce:maxTurns failure mode: declared but unusable data."""
    graph = rdflib.Graph().parse(
        plugin_root() / "ontology" / "chatman-ecosystem.ttl", format="turtle"
    )
    ranked = {
        str(graph.value(node, generate.DCTERMS.title))
        for node in graph.subjects(rdflib.RDF.type, generate.CE.Standing)
        if graph.value(node, generate.CE.standingRank) is not None
    }
    assert ranked == CANONICAL


def test_generated_file_is_not_hand_edited():
    """It carries the marker that tells a reader where to make changes."""
    source = (plugin_root() / "scripts" / "_standing.py").read_text(encoding="utf-8")
    assert "GENERATED" in source
    assert "ontology/chatman-ecosystem.ttl" in source
