"""Properties of profiles/phase-space.json and the validator that reads it.

These tests are written against the profile as data, so they keep holding as
dimensions and invariants are added. Two of them currently fail and are marked
`xfail(strict=True)`: they document real defects executably, and the strictness
means the marker cannot be left behind once the defect is fixed.
"""

from __future__ import annotations

import itertools
import math

import phase
import pytest

#: The keys `validate_vector` actually reads (phase.py, `validate_vector`).
#: Anything else in an invariant is silently ignored, which is exactly how
#: `requires_any_prior` became a no-op nobody noticed.
UNDERSTOOD_INVARIANT_KEYS = {"id", "when", "requires", "requires_any", "forbids"}


def all_vectors(profile: dict) -> list[dict[str, str]]:
    """Every point in the raw product space, lawful or not."""
    names = list(profile["dimensions"])
    state_lists = [list(profile["dimensions"][n]["states"]) for n in names]
    return [dict(zip(names, combo, strict=True)) for combo in itertools.product(*state_lists)]


def lawful_vectors(profile: dict) -> list[dict[str, str]]:
    return [v for v in all_vectors(profile) if not phase.validate_vector(profile, v)]


def test_raw_count_matches_the_product_of_dimension_sizes(profile):
    expected = math.prod(len(d["states"]) for d in profile["dimensions"].values())
    assert len(all_vectors(profile)) == expected


def test_declared_raw_count_matches_the_derived_one(profile):
    """`raw_combination_count` is a literal in the profile; keep it honest.

    It is worth keeping as a declaration precisely because it is a cross-check:
    a dimension added without updating it should fail here rather than silently
    make the documented number wrong.
    """
    derived = math.prod(len(d["states"]) for d in profile["dimensions"].values())
    declared = profile.get("raw_combination_count")
    assert declared == derived, f"declared {declared}, derived {derived}"


def test_lawful_count_is_pinned(profile):
    """136 of 648. Pinned so that changing an invariant is a reviewed change.

    This number appears nowhere in the repository today, which is why nobody
    could tell that one of the eight invariants was doing nothing.
    """
    assert len(lawful_vectors(profile)) == 136


def test_every_vector_is_either_lawful_or_explains_itself(profile):
    for vector in all_vectors(profile):
        violations = phase.validate_vector(profile, vector)
        for violation in violations:
            assert isinstance(violation, str) and violation, vector


@pytest.mark.parametrize(
    "invariant_id",
    [
        "allocation-requires-admission",
        "candidate-plan-requires-allocation",
        "manufacturing-requires-candidate-plan",
        "receipt-requires-validation",
        "publication-requires-complete-standing",
        "refusal-seals-actuation",
        "nonconformance-blocks-publication",
    ],
)
def test_every_invariant_fires_at_least_once(profile, invariant_id):
    """An invariant that never fires is decoration, not a constraint.

    This is the class-level guard: it catches any future invariant that is
    misspelled, shadowed, or unsatisfiable, not just the one known instance.
    """
    declared = {inv["id"] for inv in profile["invariants"]}
    assert invariant_id in declared, f"{invariant_id} is not declared in the profile"

    fired = any(
        any(v.startswith(f"{invariant_id}:") for v in phase.validate_vector(profile, vector))
        for vector in all_vectors(profile)
    )
    assert fired, f"{invariant_id} never fires on any of the {len(all_vectors(profile))} vectors"


def test_every_invariant_key_is_understood(profile):
    """Structural guard against the next silently-ignored predicate key.

    `validate_vector` reads a fixed set of keys and ignores the rest, so a typo
    produces an invariant that loads cleanly and enforces nothing.
    """
    for invariant in profile["invariants"]:
        unknown = set(invariant) - UNDERSTOOD_INVARIANT_KEYS
        assert not unknown, f"{invariant.get('id')} has unread key(s): {sorted(unknown)}"


def test_validated_is_only_reachable_from_candidate(profile):
    """The property `validated-plan-requires-candidate` was trying to express.

    Asserted against the transitions table, which is what actually enforces it
    (`allowed_transition` is the only gate on a vector write).
    """
    in_edges = [t for t in profile["dimensions"]["planning"]["transitions"] if t[1] == "validated"]
    assert in_edges == [["candidate", "validated"]]


def test_census_agrees_with_independent_enumeration(profile):
    """`combination_census` must not drift from the tests' own enumeration."""
    census = phase.combination_census(profile)
    assert census["raw"] == len(all_vectors(profile))
    assert census["lawful"] == len(lawful_vectors(profile))
    assert census["declared_raw_matches"] is True


def test_census_lawful_per_state_sums_correctly(profile):
    """Each dimension's per-state counts must partition the lawful set."""
    census = phase.combination_census(profile)
    for dimension, counts in census["lawful_per_state"].items():
        assert sum(counts.values()) == census["lawful"], dimension


def test_exactly_one_lawful_vector_is_publishable(profile):
    """Publication is the narrowest state in the space, by design.

    `publication-requires-complete-standing` conjoins five conditions, leaving a
    single lawful publishable vector. Pinned because a weakening of that
    invariant would otherwise be invisible.
    """
    census = phase.combination_census(profile)
    assert census["lawful_per_state"]["actuation"]["publishable"] == 1


def test_transitions_only_name_declared_states(profile):
    for name, dimension in profile["dimensions"].items():
        states = set(dimension["states"])
        for source, target in dimension.get("transitions", []):
            assert source in states, f"{name}: unknown source state {source!r}"
            assert target in states, f"{name}: unknown target state {target!r}"


def test_initial_vector_is_lawful(profile):
    initial = {name: d["initial"] for name, d in profile["dimensions"].items()}
    assert phase.validate_vector(profile, initial) == []


def test_mutation_collapse_target_is_lawful(profile):
    """The floor that `invalidate_from_mutation` force-writes must itself be lawful.

    That write bypasses `allowed_transition` entirely, so nothing else checks it.
    """
    collapsed = {
        "epistemic": "observed",
        "allocation": "unallocated",
        "planning": "unplanned",
        "actuation": "sealed",
        "drift": "drifted",
        "conformance": "unknown",
    }
    assert phase.validate_vector(profile, collapsed) == []
