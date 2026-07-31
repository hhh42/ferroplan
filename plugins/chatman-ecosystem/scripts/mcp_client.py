#!/usr/bin/env python3
"""Bounded stdlib-only MCP stdio JSON-RPC client."""
from __future__ import annotations

import json
import os
import queue
import subprocess
import sys
import threading
import time
from collections.abc import Mapping
from pathlib import Path
from typing import Any


class McpToolError(RuntimeError):
    """Raised when an MCP call violates transport or result contracts."""


_EOF = object()


class McpClient:
    """Spawn one stdio MCP server and expose its bounded tool surface."""

    def __init__(
        self,
        *,
        launcher: Path | None = None,
        timeout: float = 30.0,
        project_root: Path | None = None,
        client_name: str = "chatman-ecosystem",
        environment: Mapping[str, str] | None = None,
        max_message_bytes: int = 4 * 1024 * 1024,
        max_tool_pages: int = 1024,
        shutdown_timeout: float = 1.0,
    ) -> None:
        self._launcher = launcher or Path(__file__).resolve().parent / "run-ferroplan-mcp.sh"
        self._timeout = timeout
        self._project_root = project_root.resolve() if project_root else None
        self._client_name = client_name
        self._environment = dict(environment or {})
        self._max_message_bytes = max_message_bytes
        self._max_tool_pages = max_tool_pages
        self._shutdown_timeout = shutdown_timeout
        self._next_id = 1
        self._process: subprocess.Popen[str] | None = None
        self._line_queue: queue.Queue[Any] = queue.Queue()
        self._reader_thread: threading.Thread | None = None

    def __enter__(self) -> "McpClient":
        try:
            self._start()
        except BaseException:
            self.close()
            raise
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.close()

    def _start(self) -> None:
        if not self._launcher.exists():
            raise McpToolError(f"MCP launcher not found: {self._launcher}")
        env = dict(os.environ)
        env.update(self._environment)
        configured_root = env.get("CHATMAN_PROJECT_DIR") or env.get("CLAUDE_PROJECT_DIR")
        fallback_root = self._launcher.resolve().parent.parent.parent.parent
        root = self._project_root or (Path(configured_root) if configured_root else fallback_root)
        env.setdefault("CHATMAN_PROJECT_DIR", str(root))
        env.setdefault("CLAUDE_PROJECT_DIR", str(root))
        argv = [str(self._launcher)]
        if not os.access(self._launcher, os.X_OK):
            if self._launcher.suffix == ".sh":
                argv = ["/bin/sh", str(self._launcher)]
            elif self._launcher.suffix == ".py":
                argv = [sys.executable, str(self._launcher)]
            else:
                raise McpToolError(f"MCP launcher is not executable: {self._launcher}")
        self._process = subprocess.Popen(
            argv,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=env,
        )
        self._reader_thread = threading.Thread(target=self._read_loop, daemon=True)
        self._reader_thread.start()
        self._request(
            "initialize",
            {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": self._client_name, "version": "0.1"},
            },
        )
        self._write({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def close(self) -> None:
        process = self._process
        self._process = None
        if process is None:
            return
        try:
            if process.stdin:
                process.stdin.close()
        except (BrokenPipeError, OSError):
            pass
        try:
            process.wait(timeout=self._shutdown_timeout)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=self._shutdown_timeout)
        if self._reader_thread is not None:
            self._reader_thread.join(timeout=1)
            self._reader_thread = None

    def _read_loop(self) -> None:
        process = self._process
        stdout = process.stdout if process else None
        try:
            if stdout is not None:
                for line in stdout:
                    self._line_queue.put(line)
        except (BrokenPipeError, OSError, ValueError):
            pass
        finally:
            self._line_queue.put(_EOF)

    def _write(self, message: dict[str, Any]) -> None:
        process = self._process
        if process is None or process.stdin is None:
            raise McpToolError("MCP subprocess is not running")
        try:
            process.stdin.write(json.dumps(message) + "\n")
            process.stdin.flush()
        except (BrokenPipeError, OSError) as error:
            raise McpToolError(f"MCP subprocess is not accepting input: {error}") from error

    def _read_response(self, expected_id: int, *, timeout: float | None = None) -> dict[str, Any]:
        process = self._process
        if process is None:
            raise McpToolError("MCP subprocess is not running")
        effective_timeout = self._timeout if timeout is None else timeout
        deadline = time.monotonic() + effective_timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise McpToolError(
                    f"MCP subprocess did not respond to id={expected_id} within {effective_timeout}s"
                )
            try:
                line = self._line_queue.get(timeout=remaining)
            except queue.Empty as error:
                raise McpToolError(
                    f"MCP subprocess did not respond to id={expected_id} within {effective_timeout}s"
                ) from error
            if line is _EOF:
                stderr = process.stderr.read() if process.stderr else ""
                raise McpToolError(
                    f"MCP subprocess closed stdout before responding to id={expected_id}: "
                    f"{stderr.strip()}"
                )
            if len(line.encode("utf-8", errors="replace")) > self._max_message_bytes:
                raise McpToolError(
                    f"MCP response exceeded {self._max_message_bytes} bytes for id={expected_id}"
                )
            line = line.strip()
            if not line:
                continue
            try:
                message = json.loads(line)
            except json.JSONDecodeError as error:
                raise McpToolError(f"MCP subprocess emitted non-JSON line: {line!r}") from error
            if not isinstance(message, dict):
                raise McpToolError(f"MCP subprocess emitted non-object JSON: {message!r}")
            if message.get("id") == expected_id:
                return message

    def _request(
        self,
        method: str,
        params: dict[str, Any] | None = None,
        *,
        timeout: float | None = None,
    ) -> Any:
        request_id = self._next_id
        self._next_id += 1
        request: dict[str, Any] = {"jsonrpc": "2.0", "id": request_id, "method": method}
        if params is not None:
            request["params"] = params
        self._write(request)
        response = self._read_response(request_id, timeout=timeout)
        if "error" in response:
            raise McpToolError(f"MCP `{method}` failed: {response['error']}")
        return response.get("result")

    def list_tools(self) -> list[dict[str, Any]]:
        """Return every advertised tool with pagination cycle and page bounds."""
        tools: list[dict[str, Any]] = []
        cursor: str | None = None
        seen_cursors: set[str] = set()
        for _page in range(self._max_tool_pages):
            params = {"cursor": cursor} if cursor else None
            result = self._request("tools/list", params)
            if not isinstance(result, dict) or not isinstance(result.get("tools"), list):
                raise McpToolError(f"MCP `tools/list` returned an unexpected result: {result!r}")
            for tool in result["tools"]:
                if not isinstance(tool, dict) or not isinstance(tool.get("name"), str):
                    raise McpToolError(f"MCP `tools/list` returned an invalid tool: {tool!r}")
                tools.append(tool)
            next_cursor = result.get("nextCursor")
            if not isinstance(next_cursor, str) or not next_cursor:
                return tools
            if next_cursor in seen_cursors:
                raise McpToolError(f"MCP `tools/list` cursor cycle detected: {next_cursor}")
            seen_cursors.add(next_cursor)
            cursor = next_cursor
        raise McpToolError(f"MCP `tools/list` exceeded {self._max_tool_pages} pages")

    def call_tool(
        self,
        name: str,
        arguments: dict[str, Any],
        *,
        timeout: float | None = None,
    ) -> dict[str, Any]:
        if not isinstance(arguments, dict):
            raise McpToolError(f"MCP tool `{name}` arguments must be an object")
        result = self._request(
            "tools/call",
            {"name": name, "arguments": arguments},
            timeout=timeout,
        )
        if not isinstance(result, dict):
            raise McpToolError(f"MCP tool `{name}` returned an unexpected result: {result!r}")
        if result.get("isError"):
            raise McpToolError(f"MCP tool `{name}` reported an error: {_extract_text(result)}")
        return result


def _extract_text(result: dict[str, Any]) -> str:
    parts = []
    for block in result.get("content", []) or []:
        if isinstance(block, dict) and block.get("type") == "text":
            parts.append(str(block.get("text", "")))
    return "\n".join(parts) if parts else json.dumps(result)


def tool_structured_result(result: dict[str, Any]) -> Any:
    """Return ``structuredContent`` or parse the first text projection as JSON."""
    if "structuredContent" in result and result["structuredContent"] is not None:
        return result["structuredContent"]
    text = _extract_text(result)
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return text


if __name__ == "__main__":  # pragma: no cover
    with McpClient() as client:
        print(json.dumps(client.list_tools(), indent=2))
