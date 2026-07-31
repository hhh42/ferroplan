#!/usr/bin/env python3
"""Protocols for the OpenAI-hosted OStar/Ferroplan/OntoStar execution surface."""

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

LUNA_MODEL = "gpt-5.6-luna"
PROFILE_SCHEMA = "urn:chatman:openai-luna-profile:v2"
OSTAR_STAR_SCHEMA = "urn:ostar:openai-star-result:v1"
TRACE_SCHEMA = "urn:chatman:a2a-openai-luna-trace:v2"
_EFFORTS = {"none", "low", "medium", "high", "xhigh", "max"}
_STAR_MODES = {"mustar", "sigma-star"}


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
    planning_tools: tuple[str, ...]
    admission_tools: tuple[str, ...]
    star_mode: str
    star_domain: str
    max_star_tasks: int

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
        planning = value.get("planning_tools", [])
        admission = value.get("admission_tools", [])
        for label, items in (
            ("required_servers", servers),
            ("planning_tools", planning),
            ("admission_tools", admission),
        ):
            if not isinstance(items, list) or not all(isinstance(x, str) and x for x in items):
                raise RuntimeRefusal(f"INVALID_{label.upper()}", f"{label} must be strings")
        if not planning:
            raise RuntimeRefusal("PLANNING_POLICY_EMPTY", "Ferroplan planning policy is empty")
        if not admission:
            raise RuntimeRefusal("ADMISSION_POLICY_EMPTY", "OntoStar admission policy is empty")
        star = value.get("star", {})
        if not isinstance(star, dict):
            raise RuntimeRefusal("STAR_PROFILE_INVALID", "star profile must be an object")
        mode = star.get("mode", "mustar")
        if mode not in _STAR_MODES:
            raise RuntimeRefusal("STAR_MODE_INVALID", f"unsupported Star mode: {mode}")
        domain = star.get("domain", "SYSTEM_DESIGN")
        if not isinstance(domain, str) or not domain.strip():
            raise RuntimeRefusal("STAR_DOMAIN_INVALID", "star domain must be non-empty")
        max_tasks = star.get("max_tasks", 8)
        if not isinstance(max_tasks, int) or not 1 <= max_tasks <= 32:
            raise RuntimeRefusal("STAR_TASK_BOUND_INVALID", "star max_tasks must be in [1, 32]")
        return cls(
            model,
            effort,
            rounds,
            tuple(servers),
            tuple(planning),
            tuple(admission),
            mode,
            domain.strip().upper(),
            max_tasks,
        )


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


def _validate_mustar_result(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise RuntimeRefusal("MUSTAR_RESULT_INVALID", "MuStar result must be an object")
    required_strings = (
        "title",
        "domain",
        "build_order",
        "artifact",
        "artifact_type",
        "operator_notation",
    )
    missing = [key for key in required_strings if not isinstance(value.get(key), str)]
    if missing:
        raise RuntimeRefusal(
            "MUSTAR_RESULT_INVALID",
            "MuStar result fields missing",
            {"missing": missing},
        )
    for key in ("build_order_adhered", "implementation_complete"):
        if not isinstance(value.get(key), bool):
            raise RuntimeRefusal("MUSTAR_RESULT_INVALID", f"{key} must be boolean")
    return value


def validate_star_envelope(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict) or value.get("schema") != OSTAR_STAR_SCHEMA:
        raise RuntimeRefusal(
            "OSTAR_STAR_ENVELOPE_INVALID",
            "OStar Star envelope schema mismatch",
        )
    mode = value.get("mode")
    if mode not in _STAR_MODES:
        raise RuntimeRefusal("OSTAR_STAR_MODE_INVALID", "OStar Star mode mismatch")
    if value.get("provisional") is not True or value.get("authority") != "proposer":
        raise RuntimeRefusal(
            "OSTAR_STAR_AUTHORITY_INVALID",
            "Star output must remain provisional",
        )
    if value.get("internal_actuation") is not False:
        raise RuntimeRefusal(
            "OSTAR_STAR_ACTUATION_UNBOUNDED",
            "Star adapter must suppress internal actuation",
        )
    results = value.get("results")
    if not isinstance(results, list) or not results:
        raise RuntimeRefusal(
            "OSTAR_STAR_RESULTS_EMPTY",
            "Star output contains no MuStar results",
        )
    for result in results:
        _validate_mustar_result(result)
    return value


class OstarStarClient:
    """Invoke the bounded OStar adapter: real Star planning contracts, no internal actuation."""

    def __init__(
        self,
        launcher: Path,
        project_root: Path,
        ostar_root: Path | None = None,
        timeout: float = 300.0,
    ) -> None:
        self.launcher = launcher
        self.project_root = project_root.resolve()
        self.ostar_root = ostar_root.resolve() if ostar_root else None
        self.timeout = timeout

    def solve(self, prompt: str, target: str, profile: RuntimeProfile) -> dict[str, Any]:
        env = dict(os.environ)
        env.setdefault("CHATMAN_PROJECT_DIR", str(self.project_root))
        if self.ostar_root:
            env["OSTAR_ROOT"] = str(self.ostar_root)
        request = {
            "schema": "urn:ostar:openai-star-request:v1",
            "mode": profile.star_mode,
            "domain": profile.star_domain,
            "max_tasks": profile.max_star_tasks,
            "model": profile.model,
            "reasoning_effort": profile.reasoning_effort,
            "target": target,
            "prompt": prompt,
        }
        argv = [str(self.launcher)]
        if not os.access(self.launcher, os.X_OK):
            if self.launcher.suffix == ".sh":
                argv = ["/bin/sh", str(self.launcher)]
            elif self.launcher.suffix == ".py":
                argv = [sys.executable, str(self.launcher)]
            else:
                raise RuntimeRefusal(
                    "OSTAR_STAR_UNAVAILABLE",
                    f"launcher is not executable: {self.launcher}",
                )
        try:
            run = subprocess.run(
                argv,
                cwd=self.project_root,
                env=env,
                input=canonical_json(request),
                capture_output=True,
                text=True,
                timeout=self.timeout,
            )
        except (OSError, subprocess.SubprocessError) as error:
            raise RuntimeRefusal("OSTAR_STAR_UNAVAILABLE", str(error)) from error
        if run.returncode:
            raise RuntimeRefusal(
                "OSTAR_STAR_REFUSED",
                "OStar Star adapter returned non-zero",
                {
                    "exit_code": run.returncode,
                    "stderr": run.stderr.strip()[-4000:],
                },
            )
        try:
            return validate_star_envelope(json.loads(run.stdout))
        except json.JSONDecodeError as error:
            raise RuntimeRefusal(
                "OSTAR_STAR_NON_JSON",
                "OStar Star adapter did not emit JSON",
            ) from error


class A2AClient:
    """OntoStar A2A is coordination-only; MCP remains the admission path."""

    def __init__(self, base_url: str, timeout: float = 10.0) -> None:
        self.base_url, self.timeout = base_url.rstrip("/"), timeout

    def _request(self, path: str, body: Any | None = None) -> Any:
        data = canonical_json(body).encode() if body is not None else None
        request = urllib.request.Request(
            self.base_url + path,
            data=data,
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
        self.base_url = (
            base_url
            or os.environ.get("OPENAI_BASE_URL")
            or "https://api.openai.com/v1"
        ).rstrip("/")
        self.timeout, self.transport = timeout, transport

    def create(self, payload: dict[str, Any]) -> dict[str, Any]:
        if self.transport:
            value = self.transport(payload)
        else:
            if not self.api_key:
                raise RuntimeRefusal(
                    "OPENAI_API_KEY_MISSING",
                    "OPENAI_API_KEY is required",
                )
            request = urllib.request.Request(
                self.base_url + "/responses",
                data=canonical_json(payload).encode(),
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json",
                },
                method="POST",
            )
            try:
                with urllib.request.urlopen(request, timeout=self.timeout) as response:
                    value = json.loads(response.read().decode())
            except urllib.error.HTTPError as error:
                body = error.read().decode(errors="replace")
                raise RuntimeRefusal(
                    "OPENAI_HTTP_ERROR",
                    f"HTTP {error.code}",
                    {"body": body},
                ) from error
            except (OSError, json.JSONDecodeError) as error:
                raise RuntimeRefusal("OPENAI_TRANSPORT_ERROR", str(error)) from error
        if not isinstance(value, dict):
            raise RuntimeRefusal(
                "OPENAI_RESPONSE_INVALID",
                "response root must be an object",
            )
        if value.get("error"):
            raise RuntimeRefusal(
                "OPENAI_RESPONSE_ERROR",
                "Responses API returned an error",
                value["error"],
            )
        if value.get("status") in {"failed", "cancelled", "incomplete"}:
            raise RuntimeRefusal(
                "OPENAI_RESPONSE_INCOMPLETE",
                "Responses API did not complete",
            )
        return value
