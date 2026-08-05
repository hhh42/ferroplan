#!/usr/bin/env python3
"""Deterministic changed-file router for ferroplan's 80/20 ERRC CI."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

LANES = (
    "rust",
    "browser",
    "python",
    "plugin",
    "docs",
    "supply_chain",
    "admission",
)


def route(paths: list[str]) -> dict[str, list[str]]:
    normalized = sorted({p.strip().replace("\\", "/") for p in paths if p.strip()})
    result = {lane: [] for lane in LANES}
    result["fast_only"] = []
    for path in normalized:
        owned = set()
        if path in {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml"} or path.startswith("crates/"):
            owned.add("rust")
        if path.startswith(("crates/ferroplan-wasm/", "crates/ferroplan-bevy/", "web/")):
            owned.add("browser")
        if path.startswith("crates/ferroplan-py/"):
            owned.add("python")
        if path.startswith("plugins/chatman-ecosystem/"):
            owned.add("plugin")
        if path.startswith(("docs/", "book/")) or path.endswith(".md"):
            owned.add("docs")
        if path in {"Cargo.lock", "deny.toml"} or path.startswith(("licenses/", "release/")):
            owned.add("supply_chain")
        if path.startswith(("crates/ferroplan/src/production", "crates/ferroplan/src/readiness", "crates/ferroplan/tests/fortune5_", "docs/PRD-FORTUNE5", "docs/ARD-FORTUNE5")):
            owned.add("admission")
        if path.startswith(".github/workflows/") and path != ".github/workflows/errc-fast.yml":
            owned.add("admission")
        if not owned:
            result["fast_only"].append(path)
        for lane in owned:
            result[lane].append(path)
    return result


def discover(base: str, head: str) -> list[str]:
    proc = subprocess.run(
        ["git", "diff", "--name-only", f"{base}...{head}"],
        text=True,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"CHANGED_FILE_DISCOVERY_FAILED: {proc.stderr.strip()}")
    return proc.stdout.splitlines()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--paths", nargs="*")
    parser.add_argument("--report", default="target/ci/route.json")
    parser.add_argument("--github-output")
    args = parser.parse_args()
    if args.paths is not None:
        paths = args.paths
    elif args.base and args.head:
        paths = discover(args.base, args.head)
    else:
        parser.error("provide --paths or both --base and --head")
    routing = route(paths)
    report = {"schema": "ferroplan.ci.route.v1", "changed_files": sorted(set(paths)), "routing": routing}
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    output_path = args.github_output or os.getenv("GITHUB_OUTPUT")
    if output_path:
        with open(output_path, "a", encoding="utf-8") as handle:
            for lane in LANES:
                handle.write(f"{lane}={'true' if routing[lane] else 'false'}\n")
    json.dump(report, sys.stdout, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
