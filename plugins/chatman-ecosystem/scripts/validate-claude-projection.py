#!/usr/bin/env python3
"""Validate the source-level law of the Chatman Claude Code projection."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

NON_MANUFACTURING_AGENTS = {
    "cmca-allocator.md",
    "config-law-architect.md",
    "ecosystem-controller.md",
    "ferroplan-planner.md",
    "independent-validator.md",
    "rdf-observer.md",
    "receipt-auditor.md",
}
ALL_SKILLS = {
    "admit", "allocate", "audit", "compose", "configure", "doctor",
    "manufacture", "observe", "phase-change", "plan", "publish",
    "self-host", "validate",
}
REQUIRED_HOOK_EVENTS = {
    "SessionStart", "PreToolUse", "PostToolUse", "PostToolUseFailure",
    "PostToolBatch", "ConfigChange", "WorktreeCreate", "WorktreeRemove", "Stop",
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
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        errors.append(f"cannot read Markdown artifact: {path}: {error}")
        return {}
    if not lines or lines[0].strip() != "---":
        errors.append(f"missing frontmatter: {path}")
        return {}
    result: dict[str, str] = {}
    for line in lines[1:]:
        if line.strip() == "---":
            return result
        if ":" in line and not line.startswith((" ", "\t")):
            key, value = line.split(":", 1)
            result[key.strip()] = value.strip()
    errors.append(f"unterminated frontmatter: {path}")
    return {}


def contains_tools(value: str, required: set[str]) -> bool:
    tokens = {token.strip() for token in value.split(",") if token.strip()}
    return required.issubset(tokens)


def hook_commands(groups: Any) -> list[str]:
    commands: list[str] = []
    if not isinstance(groups, list):
        return commands
    for group in groups:
        if not isinstance(group, dict):
            continue
        for hook in group.get("hooks", []):
            if isinstance(hook, dict) and isinstance(hook.get("command"), str):
                commands.append(hook["command"])
    return commands


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
            mutation_commands = hook_commands(hooks.get("PostToolUse")) + hook_commands(
                hooks.get("PostToolUseFailure")
            )
            if any("phase.py" in command and " hook" in command for command in mutation_commands):
                errors.append("mutation hooks must not directly invoke phase.py hook")
            pre_commands = "\n".join(hook_commands(hooks.get("PreToolUse")))
            lifecycle_commands = "\n".join(
                hook_commands(hooks.get(event))
                for event in ("PostToolBatch", "ConfigChange", "WorktreeCreate", "WorktreeRemove")
            )
            if "actuation-intent.py" not in pre_commands:
                errors.append("PreToolUse must manufacture structured actuation intents")
            if "event-summary.py" not in lifecycle_commands:
                errors.append("batch/config/worktree events must use event-summary.py")

    agents_dir = plugin_root / "agents"
    for filename in sorted(NON_MANUFACTURING_AGENTS | {"source-manufacturer.md"}):
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
        elif not contains_tools(
            values.get("disallowedTools", ""), {"Write", "Edit", "NotebookEdit"}
        ):
            errors.append(f"{filename} must deny Write, Edit, and NotebookEdit")
        if filename == "ecosystem-controller.md" and "Agent(" not in values.get("tools", ""):
            errors.append("ecosystem-controller must bound the agents it can spawn")

    skills_dir = plugin_root / "skills"
    present_skills = {path.parent.name for path in skills_dir.glob("*/SKILL.md")}
    missing_skills = sorted(ALL_SKILLS - present_skills)
    if missing_skills:
        errors.append(f"missing skills: {', '.join(missing_skills)}")
    for name in sorted(ALL_SKILLS & present_skills):
        values = frontmatter(skills_dir / name / "SKILL.md", errors)
        if values.get("name") != name:
            errors.append(f"skill frontmatter name mismatch: {name}")
    publish = frontmatter(skills_dir / "publish" / "SKILL.md", errors)
    if publish.get("disable-model-invocation") != "true":
        errors.append("publish must declare disable-model-invocation: true")

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
            errors.append("projection profile must forbid main-plugin LSP registration")

    try:
        readme = (plugin_root / "README.md").read_text(encoding="utf-8")
        for stale in ("two independent stdio authorities", "three Rust MCP servers"):
            if stale in readme:
                errors.append(f"README contains stale claim: {stale}")
    except OSError as error:
        errors.append(f"cannot read README: {error}")

    required_files = [
        repository_root / "docs" / "releases" / "v26.7.29.md",
        repository_root / "docs" / "architecture" / "claude-projection.md",
        repository_root / "docs" / "migration" / "v26.7.29.md",
        plugin_root / "ontology" / "authority-graph.ttl",
        plugin_root / "profiles" / "actuation-intent.schema.json",
        plugin_root / "scripts" / "effective-phase.py",
        plugin_root / "scripts" / "actuation-intent.py",
        plugin_root / "scripts" / "grant-actuation.py",
    ]
    for path in required_files:
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
            "It does not replace claude plugin validate or runtime MCP exercise.",
        ],
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
