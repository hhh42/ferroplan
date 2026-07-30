#!/usr/bin/env python3
"""The eight admitted work surfaces, as CMCA candidates.

`profiles/work-surfaces.json` has always held a complete, correct 8x10
candidate array -- the exact shape `cmca_allocate` requires, with the ten
factor names, the repository paths each surface covers, and a note explaining
that the paths are kept *out* of the candidate objects because the MCP input is
`deny_unknown_fields`.

Nothing surfaced it. `skills/allocate/SKILL.md` cited the file for its
`factor_order` and then said "call cmca_allocate with the exact candidates",
which reads as "consult this for ordering, then supply your own values". So the
loop was driven with hand-invented factors in the 0.2-0.9 range while the real
ones span a different scale entirely (businessValue 10.0, downstreamConsequence
1000.0), and the resulting allocation receipt was cryptographically perfect over
fabricated inputs.

This module exists so the canonical candidates are the path of least
resistance. Overrides are keyed by factor *name*, resolved through
`factor_order`, so a caller never indexes into a positional float array -- the
representation that lost the names in the first place.
"""

from __future__ import annotations

import json
import sys
from collections.abc import Iterable, Mapping, Sequence
from pathlib import Path
from typing import Annotated, Any

import typer

sys.path.insert(0, str(Path(__file__).resolve().parent))
from roots import plugin_root  # noqa: E402

PROFILE_SCHEMA = "urn:chatman:cmca-work-frontier:v1"

#: The allocator is pinned to exactly these arities by the bcinr-cmca crate.
#: Checked here so a malformed frontier fails locally with a named factor
#: rather than as an arity refusal from across an MCP boundary.
REQUIRED_NODES = 8
REQUIRED_FACTORS = 10

#: The only keys `cmca_allocate` accepts. Its input is `deny_unknown_fields`,
#: so an extra key -- a helpful `paths`, say -- is rejected outright.
CANDIDATE_KEYS = ("id", "parent", "factors", "cost")


def load_profile(path: Path | None = None) -> dict[str, Any]:
    source = path or (plugin_root() / "profiles" / "work-surfaces.json")
    profile = json.loads(source.read_text(encoding="utf-8"))
    declared = profile.get("schema")
    if declared != PROFILE_SCHEMA:
        raise SystemExit(
            f"{source} declares schema {declared!r}, expected {PROFILE_SCHEMA!r}"
        )
    return profile


def factor_order(profile: Mapping[str, Any]) -> list[str]:
    return list(profile["factor_order"])


def surface_paths(profile: Mapping[str, Any], candidate_id: str) -> list[str]:
    """Repository paths a surface covers. Never sent to the allocator."""
    return list(profile.get("surface_paths", {}).get(candidate_id, []))


def candidates(
    profile: Mapping[str, Any],
    overrides: Mapping[str, Mapping[str, float]] | None = None,
) -> list[dict[str, Any]]:
    """The canonical frontier, optionally adjusted by named factor.

    The returned dicts carry exactly `CANDIDATE_KEYS` and are safe to pass
    verbatim to `cmca_allocate`.
    """
    order = factor_order(profile)
    index = {name: position for position, name in enumerate(order)}

    built: list[dict[str, Any]] = []
    for candidate in profile["candidates"]:
        factors = list(candidate["factors"])
        for name, value in (overrides or {}).get(candidate["id"], {}).items():
            if name not in index:
                raise SystemExit(
                    f"unknown factor {name!r} for {candidate['id']!r}; "
                    f"known factors: {', '.join(order)}"
                )
            factors[index[name]] = float(value)
        built.append(
            {
                "id": candidate["id"],
                "parent": candidate.get("parent"),
                "factors": factors,
                "cost": float(candidate.get("cost", 1.0)),
            }
        )

    unknown = set(overrides or {}) - {c["id"] for c in built}
    if unknown:
        raise SystemExit(
            f"unknown surface(s) {sorted(unknown)}; "
            f"known surfaces: {', '.join(c['id'] for c in built)}"
        )

    validate_frontier(built, order)
    return built


def validate_frontier(frontier: Sequence[Mapping[str, Any]], order: Iterable[str]) -> None:
    """Fail locally, naming what is wrong, before any MCP round trip."""
    names = list(order)
    if len(frontier) != REQUIRED_NODES:
        raise SystemExit(
            f"CMCA requires exactly {REQUIRED_NODES} candidates, got {len(frontier)} "
            f"(ids: {', '.join(str(c.get('id')) for c in frontier)})"
        )

    seen: set[str] = set()
    for candidate in frontier:
        identifier = str(candidate.get("id", ""))
        if not identifier or identifier in seen:
            raise SystemExit(f"candidate id {identifier!r} is empty or duplicated")
        seen.add(identifier)

        extra = set(candidate) - set(CANDIDATE_KEYS)
        if extra:
            raise SystemExit(
                f"candidate {identifier!r} carries {sorted(extra)}, which "
                "cmca_allocate rejects (its input is deny_unknown_fields)"
            )

        factors = candidate.get("factors") or []
        if len(factors) != REQUIRED_FACTORS:
            missing = names[len(factors) :] if len(factors) < REQUIRED_FACTORS else []
            detail = f" (missing: {', '.join(missing)})" if missing else ""
            raise SystemExit(
                f"candidate {identifier!r} has {len(factors)} factors, "
                f"expected {REQUIRED_FACTORS}{detail}"
            )
        for position, value in enumerate(factors):
            if not isinstance(value, (int, float)) or value != value:
                raise SystemExit(
                    f"candidate {identifier!r} factor {names[position]!r} is not finite"
                )


def parse_override(assignment: str) -> tuple[str, str, float]:
    """`claude-plugin.businessValue=12.5` -> (surface, factor, value)."""
    target, _, raw = assignment.partition("=")
    surface, _, factor = target.partition(".")
    if not surface or not factor or not raw:
        raise SystemExit(
            f"malformed --set {assignment!r}; expected surface.factorName=value"
        )
    try:
        return surface, factor, float(raw)
    except ValueError:
        raise SystemExit(f"--set {assignment!r} value is not a number") from None


app = typer.Typer(
    add_completion=False,
    no_args_is_help=True,
    help="The canonical eight CMCA work surfaces.",
)


@app.callback()
def _root() -> None:
    """Present so subcommands keep their names."""


@app.command(name="candidates")
def candidates_command(
    set_: Annotated[
        list[str] | None,
        typer.Option("--set", help="Override one factor: surface.factorName=value"),
    ] = None,
    paths: Annotated[
        bool, typer.Option("--paths", help="Include the repository paths per surface.")
    ] = False,
) -> None:
    """Emit the eight candidates, ready to pass to cmca_allocate verbatim."""
    profile = load_profile()

    overrides: dict[str, dict[str, float]] = {}
    for assignment in set_ or []:
        surface, factor, value = parse_override(assignment)
        overrides.setdefault(surface, {})[factor] = value

    frontier = candidates(profile, overrides)
    payload: dict[str, Any] = {
        "factor_order": factor_order(profile),
        "candidates": frontier,
    }
    if paths:
        payload["surface_paths"] = {
            c["id"]: surface_paths(profile, c["id"]) for c in frontier
        }
    # Emitted as plain JSON rather than a ChatmanModel: this payload's shape is
    # dictated by the MCP tool's input contract, not by ours, and wrapping it
    # would invite someone to pass the wrapper.
    typer.echo(json.dumps(payload, indent=2, sort_keys=True))


if __name__ == "__main__":
    app()
