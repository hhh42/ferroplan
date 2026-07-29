#!/usr/bin/env python3
"""Validate the source-level law of the Chatman Claude Code projection."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

REQUIRED_NON_MANUFACTURING_AGENTS = {
    "cmca-allocator.md",
    "config-law-architect.md",
    "ecosystem-controller.md",
    "ferroplan-planner.md",
    "independent-validator.md",
    "rdf-observer.md",
    "receipt-auditor.md",
}
REQUIRED_HOOK_EVENTS = {
    "SessionStart",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "PostToolBatch",
    "ConfigChange",
    "WorktreeCreate",
    "WorktreeRemove",
    "Stop",
}


def load_json(path: Path, errors: list[str]) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        errors.append(f"missing or unreadable JSON: {path}: {error}")
    except json.JSONDecodeError as error:
        errors.append(f"invalid JSON: {path}: {error}")
    return None


def frontmatter(path: Path, errors: list[str]) -> dict[str, str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        errors.append(f"cannot read agent: {path}: {error}")
        return {}
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        errors.append(f"missing frontmatter: {path}")
        return {}
    result: dict[str, str] = {}
    for line in lines[1:]:
        if line.strip() == "---":
            return result
        if ":" not in line or line.startswith((" ", "\t")):
            continue
        key, value = line.split(":", 1)
        result[key.strip()] = value.strip()
    errors.append(f"unterminated frontmatter: {path}")
    return {}


def contains_tools(value: str, required: set[str]) -> bool:
    tokens = {token.strip() for token in value.split(",") if token.strip()}
    return required.issubset(tokens)


def validate(plugin_root: Path) -> dict[str, Any]:
    errors: list[str] = []
    warnings: list[str] = []
    repository_root = plugin_root.parent.parent

    manifest = load_json(plugin_root / ".claude-plugin" / "plugin.json", errors)
    if isinstance(manifest, dict):
        if manifest.get("version") != "26.7.29":
            errors.append("plugin version must be 26.7.29")
        if manifest.get("defaultEnabled") is not False:
            errors.append("plugin must be opt-in with defaultEnabled=false")
        if "lspServers" in manifest:
            errors.append("main plugin must not declare lspServers")
        if "skills" in manifest:
            errors.append("default skills/ scan must not be redundantly declared")
        user_config = manifest.get("userConfig")
        if not isinstance(user_config, dict) or "ferroplan_root" not in user_config:
            errors.append("plugin must declare ferroplan_root userConfig")

    if (plugin_root / ".lsp.json").exists():
        errors.append("main plugin .lsp.json must be absent")

    monitors = load_json(plugin_root / "monitors" / "monitors.json", errors)
    if isinstance(monitors, list):
        for index, monitor in enumerate(monitors):
            if not isinstance(monitor, dict):
                errors.append(f"monitor {index} must be an object")
                continue
            when = monitor.get("when")
            if not isinstance(when, str) or not when.startswith("on-skill-invoke:"):
                errors.append(f"monitor {monitor.get('name', index)} must be skill-triggered")

    hooks_document = load_json(plugin_root / "hooks" / "hooks.json", errors)
    if isinstance(hooks_document, dict):
        hooks = hooks_document.get("hooks")
        if not isinstance(hooks, dict):
            errors.append("hooks.json must contain a hooks object")
        else:
            missing = sorted(REQUIRED_HOOK_EVENTS - set(hooks))
            if missing:
                errors.append(f"missing hook events: {', '.join(missing)}")
            rendered = json.dumps(hooks, sort_keys=True)
            if 'phase.py\\\" hook' in rendered or 'phase.py\" hook' in rendered:
                errors.append("mutation hooks must not directly invoke phase.py hook")
            if "actuation-intent.py" not in rendered:
                errors.append("PreToolUse must manufacture structured actuation intents")
            if "event-summary.py" not in rendered:
                errors.append("batch/config/worktree events must use event-summary.py")

    agents_dir = plugin_root / "agents"
    for filename in sorted(REQUIRED_NON_MANUFACTURING_AGENTS | {"source-manufacturer.md"}):
        values = frontmatter(agents_dir / filename, errors)
        if not values:
            continue
        if "effort" not in values or "maxTurns" not in values:
            errors.append(f"{filename} must declare effort and maxTurns")
        if filename == "source-manufacturer.md":
            if values.get("isolation") != "worktree":
                errors.append("source-manufacturer must declare isolation: worktree")
            denied = values.get("disallowedTools", "")
            if any(tool in denied for tool in ("Write", "Edit", "NotebookEdit")):
                errors.append("source-manufacturer must retain source-edit tools")
        else:
            if not contains_tools(
                values.get("disallowedTools", ""), {"Write", "Edit", "NotebookEdit"}
            ):
                errors.append(f"{filename} must deny Write, Edit, and NotebookEdit")
        if filename == "ecosystem-controller.md" and "Agent(" not in values.get("tools", ""):
            errors.append("ecosystem-controller must bound the agents it can spawn")

    ownership = load_json(plugin_root / "profiles" / "artifact-ownership.json", errors)
    if isinstance(ownership, dict):
        artifacts = ownership.get("artifacts")
        if not isinstance(artifacts, list) or len(artifacts) < 20:
            errors.append("artifact ownership registry must cover the projection surface")

    projection = load_json(plugin_root / "profiles" / "claude-projection.json", errors)
    if isinstance(projection, dict):
        if projection.get("release") != "26.7.29":
            errors.append("claude projection profile release mismatch")
        lsp_policy = projection.get("lsp_policy")
        if not isinstance(lsp_policy, dict) or lsp_policy.get("main_plugin_registration") != "forbidden":
            errors.append("claude projection profile must forbid main-plugin LSP registration")

    try:
        readme = (plugin_root / "README.md").read_text(encoding="utf-8")
        if "two independent stdio authorities" in readme:
            errors.append("README still claims two stdio authorities")
        if "three Rust MCP servers" in readme:
            errors.append("README still claims three Rust MCP servers")
    except OSError as error:
        errors.append(f"cannot read README: {error}")

    required_repository_files = [
        repository_root / "docs" / "releases" / "v26.7.29.md",
        repository_root / "docs" / "architecture" / "claude-projection.md",
        repository_root / "docs" / "migration" / "v26.7.29.md",
        plugin_root / "ontology" / "authority-graph.ttl",
        plugin_root / "profiles" / "actuation-intent.schema.json",
    ]
    for path in required_repository_files:
        if not path.is_file():
            errors.append(f"required projection artifact missing: {path}")

    return {
        "schema": "urn:chatman:claude-projection-validation:v1",
        "plugin_root": str(plugin_root),
        "valid": not errors,
        "errors": errors,
        "warnings": warnings,
        "standing": "PARTIAL_ALIVE" if not errors else "BUILD_BROKEN",
        "limitations": [
            "This validator checks source-level projection law.",
            "It does not replace claude plugin validate or runtime MCP exercise."
        ]
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plugin-root", type=Path)
    args = parser.parse_args()
    root = args.plugin_root or Path(__file__).resolve().parent.parent
    result = validate(root.resolve())
    print(json.dumps(result, sort_keys=True, indent=2))
    return 0 if result["valid"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
