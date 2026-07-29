#!/usr/bin/env python3
"""Serialize a result model to stdout.

Machine-first, stated precisely: **the default format is JSON and does not
depend on where the command runs.** A command's output contract must be the
same whether a human is at a terminal, a hook captured it, or CI piped it. A
tty-sensitive default would make the contract a function of invocation context,
which is the property this module exists to remove.

Human text is a projection registered per model type. It has no independent
existence: if a renderer is missing, the human format degrades to JSON rather
than inventing prose, because a payload with no reviewed rendering is better
shown verbatim than paraphrased.
"""

from __future__ import annotations

import json
import os
import sys
from collections.abc import Callable
from enum import StrEnum
from typing import Any, TextIO

from models import ChatmanError, ChatmanModel

Renderer = Callable[[Any], str]


class Format(StrEnum):
    JSON = "json"
    HUMAN = "human"
    SCHEMA = "schema"


_RENDERERS: dict[type[ChatmanModel], Renderer] = {}


def renders(model_type: type[ChatmanModel]) -> Callable[[Renderer], Renderer]:
    """Register the human projection for a model type."""

    def register(fn: Renderer) -> Renderer:
        _RENDERERS[model_type] = fn
        return fn

    return register


def use_color(stream: TextIO) -> bool:
    """Colour is opt-out via NO_COLOR and never applied to a non-tty.

    Note this governs *decoration only*. It can never change which characters
    carry meaning, so a piped or NO_COLOR rendering loses nothing.
    """
    if os.environ.get("NO_COLOR"):
        return False
    if os.environ.get("TERM") == "dumb":
        return False
    return hasattr(stream, "isatty") and stream.isatty()


def render_human(model: ChatmanModel) -> str:
    renderer = _RENDERERS.get(type(model))
    if renderer is None:
        # Deliberate: no reviewed rendering means show the real thing.
        return json.dumps(model.to_wire(), indent=2, sort_keys=True)
    return renderer(model)


def serialize(model: ChatmanModel, fmt: Format = Format.JSON) -> str:
    if fmt is Format.JSON:
        return json.dumps(model.to_wire(), indent=2, sort_keys=True)
    if fmt is Format.SCHEMA:
        return json.dumps(type(model).json_schema(), indent=2, sort_keys=True)
    return render_human(model)


def emit(
    model: ChatmanModel,
    fmt: Format = Format.JSON,
    *,
    stream: TextIO | None = None,
    exit_code: int = 0,
) -> int:
    """Write `model` and return `exit_code`, so callers can `return emit(...)`."""
    target = stream if stream is not None else sys.stdout
    print(serialize(model, fmt), file=target)
    return exit_code


def emit_error(
    error: ChatmanError,
    fmt: Format = Format.JSON,
    *,
    exit_code: int = 1,
) -> int:
    """Errors go to stderr so stdout stays a clean machine channel.

    A consumer piping stdout to a JSON parser must not have to filter
    diagnostics out of it.
    """
    return emit(error, fmt, stream=sys.stderr, exit_code=exit_code)


# --------------------------------------------------------------------------
# human projections
# --------------------------------------------------------------------------


@renders(ChatmanError)
def _render_error(error: ChatmanError) -> str:
    lines = [f"{error.code}: {error.message}"]
    if error.context:
        lines.append("")
        width = max(len(k) for k in error.context)
        for key in sorted(error.context):
            value = error.context[key]
            if isinstance(value, list):
                lines.append(f"  {key:<{width}} :")
                lines.extend(f"      {item}" for item in value)
            else:
                shown = "<unset>" if value is None else value
                lines.append(f"  {key:<{width}} = {shown}")
    if error.remedy:
        lines += ["", f"fix: {error.remedy}"]
    return "\n".join(lines)
