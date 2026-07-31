#!/usr/bin/env python3
"""GPT-5.6 Luna host for MuStar, Ferroplan MCP, OntoStar MCP, and A2A."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import urllib.error
import urllib.request
from collections.abc import Callable, Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Protocol

sys.path.insert(0, str(Path(__file__).resolve().parent))

LUNA_MODEL = "gpt-5.6-luna"
PROFILE_SCHEMA = "urn:chatman:openai-luna-profile:v1"
MUSTAR_ENVELOPE_SCHEMA = "chatmangpt.sr.result.v1"
MUSTAR_OBLIGATION_SCHEMA = "chatmangpt.mustar.obligation.v1"
TRACE_SCHEMA = "urn:chatman:a2a-openai-luna-trace:v1"
AUTHORIZED_OBLIGATION = "ImplementAcceptedDelta"
AUTHORIZED_EXECUTOR = "agent:openai-luna"
_EFFORTS = {"none", "low", "medium", "high", "xhigh", "max"}


class RuntimeRefusal(RuntimeError):
    def __init__(self, code: str, message: str, context: Mapping[str, Any] | None = None) -> None:
        self.code = code
        self.context = dict(context or {})
        super().__init__(message)


class McpSurface(Protocol):
    def __enter__(self) -> "McpSurface": ...
    def __exit__(self, *exc_info: object) -> None: ...
    def list_tools(self) -> list[dict[str, Any]]: ...
    def call_tool(self, name: str, arguments: dict[str, Any]) -> dict[str, Any]: ...


@dataclass(frozen=True)
class RuntimeProfile:
    model: str
    reasoning_effort: str
    max_rounds: int
    required_servers: tuple[str, ...]
    admission_tools: tuple[str, ...]

    @classmethod
    def from_dict(cls, value: Mapping[str, Any]) -> "RuntimeProfile":
        if value.get("schema") != PROFILE_SCHEMA:
            raise RuntimeRefusal("PROFILE_SCHEMA_MISMATCH", "profile schema mismatch")
        model = value.get("model")
        if model != LUNA_MODEL:
            raise RuntimeRefusal(
                "MODEL_NOT_LUNA",
                "executor is pinned to the explicit GPT-5.6 Luna model id",
                {"required": LUNA_MODEL, "observed": model},
            )
        effort = value.get("reasoning_effort", "medium")
        if effort not in _EFFORTS:
            raise RuntimeRefusal("INVALID_REASONING_EFFORT", "unsupported reasoning effort")
        rounds = value.get("max_rounds", 12)
        if not isinstance(rounds, int) or not 1 <= rounds <= 64:
            raise RuntimeRefusal("INVALID_MAX_ROUNDS", "max_rounds must be in [1, 64]")
        servers = value.get("required_servers", ["ferroplan", "ontostar"])
        tools = value.get("admission_tools", [])
        if not isinstance(servers, list) or not all(isinstance(x, str) and x for x in servers):
            raise RuntimeRefusal("INVALID_REQUIRED_SERVERS", "required_servers must be strings")
        if not isinstance(tools, list) or not all(isinstance(x, str) and x for x in tools):
            raise RuntimeRefusal("INVALID_ADMISSION_TOOLS", "admission_tools must be strings")
        if not tools:
            raise RuntimeRefusal("ADMISSION_POLICY_EMPTY", "OntoStar admission policy is empty")
        return cls(model, effort, rounds, tuple(servers), tuple(tools))


def load_profile(path: Path) -> RuntimeProfile:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeRefusal("PROFILE_UNREADABLE", f"cannot read {path}: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeRefusal("PROFILE_INVALID", "profile root must be an object")
    return RuntimeProfile.from_dict(value)


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def sha256_digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_json(value).encode()).hexdigest()


def validate_mustar_envelope(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or value.get("schema") != MUSTAR_ENVELOPE_SCHEMA:
        raise RuntimeRefusal("MUSTAR_ENVELOPE_INVALID", "MuStar envelope schema mismatch")
    if value.get("status") not in {"pass", "dispatched"}:
        raise RuntimeRefusal("MUSTAR_NOT_ADMITTED", "MuStar judgment did not pass")
    data = value.get("data")
    obligation = data.get("obligation") if isinstance(data, dict) else None
    if not isinstance(obligation, dict) or obligation.get("schema") != MUSTAR_OBLIGATION_SCHEMA:
        raise RuntimeRefusal("MUSTAR_OBLIGATION_INVALID", "MotionObligation schema mismatch")
    if not isinstance(data.get("obligation_hash"), str) or not data["obligation_hash"]:
        raise RuntimeRefusal("MUSTAR_HASH_MISSING", "MuStar obligation hash is missing")
    return value


class MustarClient:
    def __init__(self, launcher: Path, project_root: Path, timeout: float = 30.0) -> None:
        self.launcher = launcher
        self.project_root = project_root.resolve()
        self.timeout = timeout

    def next(self, target: str) -> dict[str, Any]:
        env = dict(os.environ)
        env.setdefault("CHATMAN_PROJECT_DIR", str(self.project_root))
        env.setdefault("CLAUDE_PROJECT_DIR", str(self.project_root))
        argv = [str(self.launcher), target]
        if not os.access(self.launcher, os.X_OK):
            if self.launcher.suffix == ".sh":
                argv = ["/bin/sh", str(self.launcher), target]
            elif self.launcher.suffix == ".py":
                argv = [sys.executable, str(self.launcher), target]
            else:
                raise RuntimeRefusal("MUSTAR_UNAVAILABLE", f"launcher is not executable: {self.launcher}")
        try:
            run = subprocess.run(
                argv, cwd=self.project_root, env=env,
                capture_output=True, text=True, timeout=self.timeout,
            )
        except (OSError, subprocess.SubprocessError) as error:
            raise RuntimeRefusal("MUSTAR_UNAVAILABLE", str(error)) from error
        if run.returncode:
            raise RuntimeRefusal(
                "MUSTAR_REFUSED", "MuStar returned non-zero",
                {"exit_code": run.returncode, "stderr": run.stderr.strip()},
            )
        try:
            return validate_mustar_envelope(json.loads(run.stdout))
        except json.JSONDecodeError as error:
            raise RuntimeRefusal("MUSTAR_NON_JSON", "MuStar did not emit JSON") from error


def project_executor(envelope: Mapping[str, Any]) -> dict[str, Any]:
    data = envelope["data"]
    original = data["obligation"]
    dispatch = original.get("dispatch")
    if not isinstance(dispatch, dict):
        raise RuntimeRefusal("MUSTAR_DISPATCH_MISSING", "MotionObligation has no dispatch")
    kind = original.get("kind")
    old_executor = dispatch.get("executor")
    old_command = dispatch.get("command")
    executor, command, projection = old_executor, old_command, "identity"
    if kind == AUTHORIZED_OBLIGATION and old_executor == "agent:claude-code":
        executor, projection = AUTHORIZED_EXECUTOR, "claude-code-to-openai-luna"
        if isinstance(command, str) and command.startswith("claude-code:"):
            command = "openai-luna:" + command[len("claude-code:") :]
    return {
        "schema": "urn:chatman:executor-projection:v1",
        "projection": projection,
        "obligation_id": original.get("id"),
        "obligation_hash": data["obligation_hash"],
        "kind": kind,
        "original_executor": old_executor,
        "effective_executor": executor,
        "original_command": old_command,
        "effective_command": command,
        "authorized": kind == AUTHORIZED_OBLIGATION and executor == AUTHORIZED_EXECUTOR,
    }


class A2AClient:
    """OntoStar A2A is coordination-only; MCP remains the admission path."""

    def __init__(self, base_url: str, timeout: float = 10.0) -> None:
        self.base_url, self.timeout = base_url.rstrip("/"), timeout

    def _request(self, path: str, body: Any | None = None) -> Any:
        data = canonical_json(body).encode() if body is not None else None
        request = urllib.request.Request(
            self.base_url + path, data=data,
            headers={"Content-Type": "application/json"} if data else {},
            method="POST" if data else "GET",
        )
        with urllib.request.urlopen(request, timeout=self.timeout) as response:
            return json.loads(response.read().decode())

    def probe(self) -> dict[str, Any]:
        return {
            "agent_card": self._request("/agent-card"),
            "heartbeat": self._request("/", {"tool": "onto_status", "params": {}}),
            "authority": "coordination-only",
        }


class OpenAIResponsesClient:
    def __init__(
        self,
        api_key: str | None = None,
        *,
        base_url: str | None = None,
        timeout: float = 120.0,
        transport: Callable[[dict[str, Any]], dict[str, Any]] | None = None,
    ) -> None:
        self.api_key = api_key or os.environ.get("OPENAI_API_KEY")
        self.base_url = (base_url or os.environ.get("OPENAI_BASE_URL") or
                         "https://api.openai.com/v1").rstrip("/")
        self.timeout, self.transport = timeout, transport

    def create(self, payload: dict[str, Any]) -> dict[str, Any]:
        if self.transport:
            value = self.transport(payload)
        else:
            if not self.api_key:
                raise RuntimeRefusal("OPENAI_API_KEY_MISSING", "OPENAI_API_KEY is required")
            request = urllib.request.Request(
                self.base_url + "/responses", data=canonical_json(payload).encode(),
                headers={"Authorization": f"Bearer {self.api_key}",
                         "Content-Type": "application/json"}, method="POST",
            )
            try:
                with urllib.request.urlopen(request, timeout=self.timeout) as response:
                    value = json.loads(response.read().decode())
            except urllib.error.HTTPError as error:
                body = error.read().decode(errors="replace")
                raise RuntimeRefusal("OPENAI_HTTP_ERROR", f"HTTP {error.code}", {"body": body}) from error
            except (OSError, json.JSONDecodeError) as error:
                raise RuntimeRefusal("OPENAI_TRANSPORT_ERROR", str(error)) from error
        if not isinstance(value, dict):
            raise RuntimeRefusal("OPENAI_RESPONSE_INVALID", "response root must be an object")
        if value.get("error"):
            raise RuntimeRefusal("OPENAI_RESPONSE_ERROR", "Responses API returned an error", value["error"])
        if value.get("status") in {"failed", "cancelled", "incomplete"}:
            raise RuntimeRefusal("OPENAI_RESPONSE_INCOMPLETE", "Responses API did not complete")
        return value
