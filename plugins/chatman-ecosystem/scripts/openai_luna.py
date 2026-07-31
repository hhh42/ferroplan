#!/usr/bin/env python3
"""CLI projection for the OpenAI-hosted OStar/Ferroplan/OntoStar pipeline."""

from __future__ import annotations

import argparse
import json
import os
import sys
from collections.abc import Sequence
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from mcp_client import McpClient  # noqa: E402
from openai_luna_protocol import (  # noqa: E402,F401
    A2AClient,
    LUNA_MODEL,
    McpSurface,
    OSTAR_STAR_SCHEMA,
    OpenAIResponsesClient,
    OstarStarClient,
    PROFILE_SCHEMA,
    RuntimeProfile,
    RuntimeRefusal,
    TRACE_SCHEMA,
    canonical_json,
    load_profile,
    validate_star_envelope,
)
from openai_luna_runtime import (  # noqa: E402,F401
    LunaHost,
    McpToolRegistry,
    seal_trace,
)


def _mcp_arg(value: str) -> tuple[str, Path]:
    label, separator, path = value.partition("=")
    if not separator or not label or not path:
        raise argparse.ArgumentTypeError("expected LABEL=/path/to/launcher")
    return label, Path(path)


def parser() -> argparse.ArgumentParser:
    scripts = Path(__file__).resolve().parent
    plugin = scripts.parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("prompt", nargs="?")
    parser.add_argument("--target", default="ferroplan")
    parser.add_argument("--project", type=Path, default=Path.cwd())
    parser.add_argument(
        "--profile",
        type=Path,
        default=plugin / "profiles/openai-luna.json",
    )
    parser.add_argument("--ostar-root", type=Path)
    parser.add_argument(
        "--star-launcher",
        type=Path,
        default=scripts / "run-ostar-star.sh",
    )
    parser.add_argument(
        "--ferroplan-launcher",
        type=Path,
        default=scripts / "run-ferroplan-mcp.sh",
    )
    parser.add_argument(
        "--ontostar-launcher",
        type=Path,
        default=scripts / "run-ontostar-mcp.sh",
    )
    parser.add_argument(
        "--mcp",
        action="append",
        default=[],
        type=_mcp_arg,
        metavar="LABEL=LAUNCHER",
    )
    parser.add_argument(
        "--ontostar-a2a-url",
        default=os.environ.get("ONTOSTAR_A2A_URL"),
    )
    parser.add_argument("--receipt", type=Path)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = parser().parse_args(argv)
    prompt = args.prompt if args.prompt is not None else sys.stdin.read()
    if not prompt.strip():
        print(
            canonical_json({"code": "TASK_EMPTY", "message": "prompt is empty"}),
            file=sys.stderr,
        )
        return 64
    try:
        profile = load_profile(args.profile)
        project = args.project.resolve()
        clients: dict[str, McpSurface] = {
            "ferroplan": McpClient(
                launcher=args.ferroplan_launcher,
                project_root=project,
                client_name="openai-luna-ferroplan",
            ),
            "ontostar": McpClient(
                launcher=args.ontostar_launcher,
                project_root=project,
                client_name="openai-luna-ontostar",
            ),
        }
        for label, launcher in args.mcp:
            if label in clients:
                raise RuntimeRefusal("MCP_LABEL_DUPLICATE", label)
            clients[label] = McpClient(
                launcher=launcher,
                project_root=project,
                client_name=f"openai-luna-{label}",
            )
        host = LunaHost(
            profile,
            OpenAIResponsesClient(),
            OstarStarClient(args.star_launcher, project, args.ostar_root),
            clients,
            A2AClient(args.ontostar_a2a_url) if args.ontostar_a2a_url else None,
        )
        trace = host.run(prompt, args.target)
    except RuntimeRefusal as error:
        trace = seal_trace(
            {
                "schema": TRACE_SCHEMA,
                "standing": "BLOCKED",
                "status": "refused",
                "error": {
                    "code": error.code,
                    "message": str(error),
                    "context": error.context,
                },
            }
        )
    output = json.dumps(trace, indent=2, sort_keys=True)
    print(output)
    if args.receipt:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        args.receipt.write_text(output + "\n", encoding="utf-8")
    return 0 if trace.get("standing") == "ALIVE" else 78


if __name__ == "__main__":
    raise SystemExit(main())
