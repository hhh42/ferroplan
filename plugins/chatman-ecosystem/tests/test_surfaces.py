"""The canonical work frontier must be reachable, correct, and hard to misuse.

The defect: a complete 8x10 candidate array sat in profiles/work-surfaces.json
while the loop was driven with hand-invented factors, and every integrity check
passed over the invention. Correct inputs have to be the easy path.
"""

from __future__ import annotations

import json

import pytest
import surfaces
from roots import plugin_root


@pytest.fixture(scope="module")
def profile():
    return surfaces.load_profile()


def test_profile_schema_is_checked(tmp_path):
    """A profile that is not the work frontier must not be silently accepted."""
    impostor = tmp_path / "other.json"
    impostor.write_text(json.dumps({"schema": "urn:chatman:something-else:v1"}))
    with pytest.raises(SystemExit, match="expected"):
        surfaces.load_profile(impostor)


def test_frontier_matches_the_allocator_arity(profile):
    frontier = surfaces.candidates(profile)
    assert len(frontier) == surfaces.REQUIRED_NODES == 8
    assert all(len(c["factors"]) == surfaces.REQUIRED_FACTORS == 10 for c in frontier)


def test_candidates_carry_only_keys_the_tool_accepts(profile):
    """cmca_allocate is deny_unknown_fields; a helpful extra key is a refusal."""
    for candidate in surfaces.candidates(profile):
        assert set(candidate) == set(surfaces.CANDIDATE_KEYS)


def test_surface_paths_are_not_sent_to_the_allocator(profile):
    """The paths exist and are useful, but must stay out of the payload."""
    assert surfaces.surface_paths(profile, "claude-plugin")
    assert all("paths" not in c for c in surfaces.candidates(profile))


def test_factor_order_is_the_documented_ten(profile):
    order = surfaces.factor_order(profile)
    assert len(order) == 10
    assert order[0] == "accessFrequency"
    assert order[-1] == "downstreamConsequence"


def test_canonical_factors_are_not_the_invented_scale(profile):
    """Regression against the actual incident.

    The fabricated candidates were all within 0.2-0.9. The canonical ones span
    a far wider scale, which is why the two allocate differently. If every
    factor ever falls inside the narrow band again, something has been
    hand-edited toward the invented shape.
    """
    values = [v for c in surfaces.candidates(profile) for v in c["factors"]]
    assert max(values) > 100, "canonical factors include large-magnitude terms"


def test_overrides_are_keyed_by_name_not_position(profile):
    order = surfaces.factor_order(profile)
    position = order.index("businessValue")
    frontier = surfaces.candidates(profile, {"claude-plugin": {"businessValue": 42.0}})
    changed = next(c for c in frontier if c["id"] == "claude-plugin")
    assert changed["factors"][position] == 42.0


def test_override_of_an_unknown_factor_names_the_known_ones(profile):
    with pytest.raises(SystemExit, match="known factors"):
        surfaces.candidates(profile, {"claude-plugin": {"notAFactor": 1.0}})


def test_override_of_an_unknown_surface_names_the_known_ones(profile):
    with pytest.raises(SystemExit, match="known surfaces"):
        surfaces.candidates(profile, {"not-a-surface": {"businessValue": 1.0}})


def test_overrides_do_not_mutate_the_profile(profile):
    before = json.dumps(profile, sort_keys=True)
    surfaces.candidates(profile, {"claude-plugin": {"businessValue": 99.0}})
    assert json.dumps(profile, sort_keys=True) == before


@pytest.mark.parametrize(
    ("assignment", "expected"),
    [
        ("claude-plugin.businessValue=12.5", ("claude-plugin", "businessValue", 12.5)),
        ("evidence.standing=1", ("evidence", "standing", 1.0)),
    ],
)
def test_override_parsing(assignment, expected):
    assert surfaces.parse_override(assignment) == expected


@pytest.mark.parametrize("bad", ["nodot=1", "a.b", "a.b=", "=1", "a.b=notanumber"])
def test_malformed_overrides_are_refused(bad):
    with pytest.raises(SystemExit):
        surfaces.parse_override(bad)


def test_validate_frontier_names_the_missing_factor():
    """Fail locally with a name, not remotely with an arity count."""
    order = ["a", "b", "c"]
    with pytest.raises(SystemExit, match="missing: c"):
        surfaces.validate_frontier(
            [{"id": f"n{i}", "factors": [0.0, 0.0], "cost": 1.0} for i in range(8)],
            order,
        )


def test_validate_frontier_rejects_the_wrong_node_count(profile):
    frontier = surfaces.candidates(profile)[:7]
    with pytest.raises(SystemExit, match="exactly 8"):
        surfaces.validate_frontier(frontier, surfaces.factor_order(profile))


def test_validate_frontier_rejects_duplicate_ids(profile):
    frontier = surfaces.candidates(profile)
    frontier[1] = dict(frontier[0])
    with pytest.raises(SystemExit, match="duplicated"):
        surfaces.validate_frontier(frontier, surfaces.factor_order(profile))


def test_validate_frontier_rejects_extra_keys(profile):
    frontier = surfaces.candidates(profile)
    frontier[0]["paths"] = ["somewhere"]
    with pytest.raises(SystemExit, match="deny_unknown_fields"):
        surfaces.validate_frontier(frontier, surfaces.factor_order(profile))


def test_every_surface_declares_the_paths_it_covers(profile):
    """A surface with no paths cannot be audited against the repository."""
    for candidate in surfaces.candidates(profile):
        assert surfaces.surface_paths(profile, candidate["id"]), candidate["id"]


def test_declared_surface_paths_exist_in_the_repository(profile):
    """Allocating capacity to a path that is gone is allocating to nothing."""
    repo = plugin_root().parent.parent
    for candidate in surfaces.candidates(profile):
        for relative in surfaces.surface_paths(profile, candidate["id"]):
            assert (repo / relative).exists(), f"{candidate['id']}: {relative} is missing"
