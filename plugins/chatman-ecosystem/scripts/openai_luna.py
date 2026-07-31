#!/usr/bin/env python3
"""CLI projection for the GPT-5.6 Luna Chatman execution host."""

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
    AUTHORIZED_EXECUTOR,
    LUNA_MODEL,
    MUSTAR_ENVELOPE_SCHEMA,
    MUSTAR_OBLIGATION_SCHEMA,
    McpSurface,
    MustarClient,
    OpenAIResponsesClient,
    PROFILE_SCHEMA,
    RuntimeProfile,
    RuntimeRefusal,
    TRACE_SCHEMA,
    canonical_json,
    load_profile,
    project_executor,
    validate_mustar_envelope,
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
    scripts, plugin = Path(__file__).resolve().parent, Path(__file__).resolve().parent.parent
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("prompt", nargs="?")
    p.add_argument("--target", default="ferroplan")
    p.add_argument("--project", type=Path, default=Path.cwd())
    p.add_argument("--mustar-project", type=Path)
    p.add_argument("--profile", type=Path, default=plugin / "profiles/openai-luna.json")
    p.add_argument("--mustar-launcher", type=Path, default=scripts / "run-mustar-next.sh")
    p.add_argument("--ferroplan-launcher", type=Path, default=scripts / "run-ferroplan-mcp.sh")
    p.add_argument("--ontostar-launcher", type=Path, default=scripts / "run-ontostar-mcp.sh")
    p.add_argument("--mcp", action="append", default=[], type=_mcp_arg, metavar="LABEL=LAUNCHER")
    p.add_argument("--ontostar-a2a-url", default=os.environ.get("ONTOSTAR_A2A_URL"))
    p.add_argument("--receipt", type=Path)
    return p


def main(argv: Sequence[str] | None = None) -> int:
    args = parser().parse_args(argv)
    prompt = args.prompt if args.prompt is not None else sys.stdin.read()
    if not prompt.strip():
        print(canonical_json({"code": "TASK_EMPTY", "message": "prompt is empty"}), file=sys.stderr)
        return 64
    try:
        profile, project = load_profile(args.profile), args.project.resolve()
        clients: dict[str, McpSurface] = {
            "ferroplan": McpClient(launcher=args.ferroplan_launcher, project_root=project,
                                   client_name="openai-luna-ferroplan"),
            "ontostar": McpClient(launcher=args.ontostar_launcher, project_root=project,
                                  client_name="openai-luna-ontostar"),
        }
        for label, launcher in args.mcp:
            if label in clients:
                raise RuntimeRefusal("MCP_LABEL_DUPLICATE", label)
            clients[label] = McpClient(launcher=launcher, project_root=project,
                                       client_name=f"openai-luna-{label}")
        host = LunaHost(
            profile, OpenAIResponsesClient(),
            MustarClient(args.mustar_launcher, (args.mustar_project or project).resolve()), clients,
            A2AClient(args.ontostar_a2a_url) if args.ontostar_a2a_url else None,
        )
        trace = host.run(prompt, args.target)
    except RuntimeRefusal as error:
        trace = seal_trace({
            "schema": TRACE_SCHEMA, "standing": "BLOCKED", "status": "refused",
            "error": {"code": error.code, "message": str(error), "context": error.context},
        })
    output = json.dumps(trace, indent=2, sort_keys=True)
    print(output)
    if args.receipt:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        args.receipt.write_text(output + "\n", encoding="utf-8")
    return 0 if trace.get("standing") == "ALIVE" else 78


if __name__ == "__main__":
    raise SystemExit(main())
