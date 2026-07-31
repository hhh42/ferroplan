from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

PLUGIN_ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = PLUGIN_ROOT / "scripts"
REQUIRED_VECTOR = {
    "epistemic": "admitted",
    "allocation": "allocated",
    "planning": "validated",
    "actuation": "publishable",
    "drift": "stable",
    "conformance": "conformant",
}
COLLAPSED_VECTOR = {
    "epistemic": "observed",
    "allocation": "unallocated",
    "planning": "unplanned",
    "actuation": "sealed",
    "drift": "drifted",
    "conformance": "unknown",
}


def project_key(project: Path) -> str:
    return hashlib.sha256(os.path.realpath(project).encode("utf-8")).hexdigest()[:24]


def run_script(
    name: str,
    *,
    args: list[str] | None = None,
    payload: dict | None = None,
    env: dict[str, str] | None = None,
    cwd: Path | None = None,
) -> subprocess.CompletedProcess[str]:
    command = [sys.executable, str(SCRIPTS / name), *(args or [])]
    return subprocess.run(
        command,
        cwd=cwd,
        input=json.dumps(payload) if payload is not None else None,
        capture_output=True,
        text=True,
        env={**os.environ, **(env or {})},
        check=False,
        timeout=30,
    )


class ClaudeProjectionTests(unittest.TestCase):
    def test_source_projection_validator(self) -> None:
        result = run_script(
            "validate-claude-projection.py",
            args=["--plugin-root", str(PLUGIN_ROOT)],
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        document = json.loads(result.stdout)
        self.assertTrue(document["valid"], document["errors"])
        self.assertEqual(document["standing"], "PARTIAL_ALIVE")

    def test_pending_observations_project_collapsed_effective_phase(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = root / "project"
            project.mkdir()
            data = root / "data"
            directory = data / "projects" / project_key(project)
            directory.mkdir(parents=True)
            (directory / "phase-state.json").write_text(
                json.dumps({"vector": REQUIRED_VECTOR, "receipt": "a" * 64}),
                encoding="utf-8",
            )
            (directory / "state.json").write_text(
                json.dumps(
                    {
                        "event_count": 4,
                        "admitted_event_count": 2,
                        "plan_receipt": "a" * 64,
                    }
                ),
                encoding="utf-8",
            )
            result = run_script(
                "effective-phase.py",
                args=["--project", str(project)],
                env={
                    "CLAUDE_PLUGIN_ROOT": str(PLUGIN_ROOT),
                    "CLAUDE_PLUGIN_DATA": str(data),
                },
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            document = json.loads(result.stdout)
            self.assertEqual(document["canonical_vector"], REQUIRED_VECTOR)
            self.assertEqual(document["effective_vector"], COLLAPSED_VECTOR)
            self.assertEqual(document["pending_event_count"], 2)
            self.assertTrue(document["requires_admission"])

    def test_protected_command_records_intent_and_denies_without_grant(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = root / "project"
            project.mkdir()
            data = root / "data"
            directory = data / "projects" / project_key(project)
            directory.mkdir(parents=True)
            (directory / "phase-state.json").write_text(
                json.dumps({"vector": REQUIRED_VECTOR, "receipt": "b" * 64}),
                encoding="utf-8",
            )
            (directory / "state.json").write_text(
                json.dumps(
                    {
                        "event_count": 0,
                        "admitted_event_count": 0,
                        "plan_receipt": "b" * 64,
                    }
                ),
                encoding="utf-8",
            )
            payload = {
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "tool_use_id": "tool-1",
                "session_id": "session-1",
                "cwd": str(project),
                "tool_input": {"command": "git push origin release"},
            }
            result = run_script(
                "actuation-intent.py",
                payload=payload,
                env={
                    "CLAUDE_PLUGIN_ROOT": str(PLUGIN_ROOT),
                    "CLAUDE_PLUGIN_DATA": str(data),
                },
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            refusal = json.loads(result.stdout)
            output = refusal["hookSpecificOutput"]
            self.assertEqual(output["permissionDecision"], "deny")
            self.assertIn("no valid derived execution grant", output["permissionDecisionReason"])
            intents = list((directory / "intents").glob("*.json"))
            self.assertEqual(len(intents), 1)
            intent = json.loads(intents[0].read_text(encoding="utf-8"))
            self.assertEqual(intent["operation"], "git-push")
            self.assertEqual(intent["effective_phase"], REQUIRED_VECTOR)

    @unittest.skipUnless(shutil.which("git"), "git is required")
    def test_generated_guard_requires_owner_first(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            registry = root / "plugins/chatman-ecosystem/profiles"
            registry.mkdir(parents=True)
            owner = root / "ontology/source.ttl"
            target = root / "generated/output.json"
            owner.parent.mkdir(parents=True)
            target.parent.mkdir(parents=True)
            owner.write_text("owner\n", encoding="utf-8")
            target.write_text("{}\n", encoding="utf-8")
            (registry / "artifact-ownership.json").write_text(
                json.dumps(
                    {
                        "artifacts": [
                            {
                                "path": "generated/output.json",
                                "owner": "ontology/source.ttl",
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "fixture"], cwd=root, check=True)
            payload = {
                "hook_event_name": "PreToolUse",
                "tool_name": "Edit",
                "cwd": str(root),
                "tool_input": {"file_path": str(target)},
            }
            denied = run_script("generated-guard.py", payload=payload, cwd=root)
            self.assertIn("HAND_CODED_GENERATED_OUTPUT", denied.stdout)
            owner.write_text("owner changed\n", encoding="utf-8")
            allowed = run_script("generated-guard.py", payload=payload, cwd=root)
            self.assertEqual(allowed.stdout.strip(), "")


if __name__ == "__main__":
    unittest.main()
