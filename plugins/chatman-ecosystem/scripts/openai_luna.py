#!/usr/bin/env python3
"""CLI projection for the OpenAI-hosted OStar/Ferroplan/OntoStar pipeline."""
from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
from collections.abc import Sequence
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from mcp_client import McpClient  # noqa: E402
from openai_luna_protocol import (  # noqa: E402
    TRACE_SCHEMA,
    A2AClient,
    McpSurface,
    OpenAIResponsesClient,
    OstarStarClient,
    RuntimeRefusal,
    canonical_json,
    load_profile,
)
from openai_luna_runtime import LunaHost, seal_trace  # noqa: E402


def _mcp_arg(value: str) -> tuple[str, Path]:
    label, separator, path = value.partition("=")
    if not separator or not label or not path:
        raise argparse.ArgumentTypeError("expected LABEL=/path/to/launcher")
    return label, Path(path)


def parser() -> argparse.ArgumentParser:
    scripts = Path(__file__).resolve().parent
    plugin = scripts.parent
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("prompt", nargs="?")
    result.add_argument("--target", default="ferroplan")
    result.add_argument("--project", type=Path, default=Path.cwd())
    result.add_argument("--profile", type=Path, default=plugin / "profiles/openai-luna.json")
    result.add_argument("--ostar-root", type=Path)
    result.add_argument("--star-launcher", type=Path, default=scripts / "run-ostar-star.sh")
    result.add_argument("--ferroplan-launcher", type=Path, default=scripts / "run-ferroplan-mcp.sh")
    result.add_argument("--ontostar-launcher", type=Path, default=scripts / "run-ontostar-mcp.sh")
    result.add_argument("--mcp", action="append", default=[], type=_mcp_arg, metavar="LABEL=LAUNCHER")
    result.add_argument("--ontostar-a2a-url", default=os.environ.get("ONTOSTAR_A2A_URL"))
    result.add_argument("--receipt", type=Path)
    return result


def _blocked(error: RuntimeRefusal) -> dict:
    return seal_trace({"schema": TRACE_SCHEMA, "standing": "BLOCKED", "status": "refused", "error": {"code": error.code, "message": str(error), "context": error.context}})


def _write_atomic(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent, text=True)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            handle.write(text)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except BaseException:
        try:
            os.unlink(temporary)
        except OSError:
            pass
        raise


def main(argv: Sequence[str] | None = None) -> int:
    args = parser().parse_args(argv)
    prompt = args.prompt if args.prompt is not None else sys.stdin.read()
    if not prompt.strip():
        print(canonical_json({"code": "TASK_EMPTY", "message": "prompt is empty"}), file=sys.stderr)
        return 64
    try:
        runtime_profile = load_profile(args.profile)
        project = args.project.resolve()
        clients: dict[str, McpSurface] = {
            "ferroplan": McpClient(launcher=args.ferroplan_launcher, project_root=project, client_name="openai-luna-ferroplan"),
            "ontostar": McpClient(launcher=args.ontostar_launcher, project_root=project, client_name="openai-luna-ontostar"),
        }
        for label, launcher in args.mcp:
            if label in clients:
                raise RuntimeRefusal("MCP_LABEL_DUPLICATE", label)
            clients[label] = McpClient(launcher=launcher, project_root=project, client_name=f"openai-luna-{label}")
        host = LunaHost(runtime_profile, OpenAIResponsesClient(), OstarStarClient(args.star_launcher, project, args.ostar_root), clients, A2AClient(args.ontostar_a2a_url) if args.ontostar_a2a_url else None)
        trace = host.run(prompt, args.target)
    except RuntimeRefusal as error:
        trace = _blocked(error)
    except Exception as error:  # Last-resort typed receipt: never emit an unreceipted traceback.
        trace = _blocked(RuntimeRefusal("RUNTIME_UNEXPECTED", str(error), {"type": type(error).__name__}))
    output = json.dumps(trace, indent=2, sort_keys=True)
    print(output)
    if args.receipt:
        _write_atomic(args.receipt, output + "\n")
    return 0 if trace.get("standing") == "ALIVE" else 78


if __name__ == "__main__":
    raise SystemExit(main())
