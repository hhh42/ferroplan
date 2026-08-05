#!/usr/bin/env python3
"""Build a sealed OpenAI Luna verifier report from an observed JUnit receipt."""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
import xml.etree.ElementTree as ET
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

SCHEMA = "urn:chatman:openai-luna-verifier-report:v1"
CATEGORIES: tuple[tuple[str, str], ...] = (
    ("legacy_regression", "test_openai_luna"),
    ("unit", "test_openai_luna_unit"),
    ("contract", "test_openai_luna_contract"),
    ("ostar_contract", "test_openai_ostar_star"),
    ("property_fuzz", "test_openai_luna_property"),
    ("replay", "test_openai_luna_replay"),
    ("mutation_sentinels", "test_openai_luna_mutation"),
    ("integration", "test_openai_luna_integration"),
    ("e2e", "test_openai_luna_e2e"),
    ("security", "test_openai_luna_security"),
    ("chaos", "test_openai_luna_chaos"),
    ("stress", "test_openai_luna_stress"),
    ("benchmark", "test_openai_luna_benchmark"),
)


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_json(value).encode()).hexdigest()


def _classname_contains_module(classname: str, module: str) -> bool:
    """Match both pytest functions and unittest-style nested class names.

    Pytest emits `test_openai_luna_unit` for module-level functions but emits
    `test_openai_luna.StarRuntimeTests` for unittest classes. JUnit class
    nesting is not a different verification category, so classify by any
    exact dotted component rather than by the final component alone.
    """
    return module in classname.split(".")


def category_results(junit_path: Path) -> list[dict[str, Any]]:
    counts = {
        name: {"tests": 0, "failures": 0, "errors": 0, "skipped": 0}
        for name, _ in CATEGORIES
    }
    root = ET.parse(junit_path).getroot()
    for case in root.iter("testcase"):
        classname = case.attrib.get("classname", "")
        for name, module in CATEGORIES:
            if not _classname_contains_module(classname, module):
                continue
            bucket = counts[name]
            bucket["tests"] += 1
            bucket["failures"] += int(case.find("failure") is not None)
            bucket["errors"] += int(case.find("error") is not None)
            bucket["skipped"] += int(case.find("skipped") is not None)
            break
    return [
        {
            "name": name,
            "state": (
                "ALIVE"
                if values["tests"] and not values["failures"] and not values["errors"]
                else "BUILD_BROKEN"
            ),
            **values,
        }
        for name, values in counts.items()
    ]


def build_report(
    junit_path: Path, static_checks: list[str], python_version: str
) -> dict[str, Any]:
    categories = category_results(junit_path)
    standing = (
        "ALIVE"
        if all(item["state"] == "ALIVE" for item in categories)
        else "BUILD_BROKEN"
    )
    unsigned = {
        "schema": SCHEMA,
        "standing": standing,
        "generated_at": datetime.now(UTC).isoformat(),
        "python": python_version,
        "junit_sha256": "sha256:" + hashlib.sha256(junit_path.read_bytes()).hexdigest(),
        "static_checks": static_checks,
        "categories": categories,
        "summary": {
            "alive": sum(item["state"] == "ALIVE" for item in categories),
            "total": len(categories),
            "tests": sum(item["tests"] for item in categories),
            "verification_ladder": [item["name"] for item in categories],
        },
    }
    return {**unsigned, "report_sha256": digest(unsigned)}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--junit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--static-check", action="append", default=[])
    parser.add_argument("--python-version", default=sys.version.split()[0])
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.junit.is_file():
        raise SystemExit(f"JUnit receipt not found: {args.junit}")
    report = build_report(args.junit, args.static_check, args.python_version)
    output = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(output, encoding="utf-8")
    print(output, end="")
    return 0 if report["standing"] == "ALIVE" else 1


if __name__ == "__main__":
    raise SystemExit(main())
