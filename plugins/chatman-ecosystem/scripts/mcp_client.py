#!/usr/bin/env python3
"""Minimal stdlib-only MCP stdio JSON-RPC client for the Chatman ecosystem plugin.

This does not reimplement the MCP protocol in general -- it implements exactly
the subset `loop.py` and `phase.py` need to call `ferroplan-mcp` tools
(`initialize` handshake, `notifications/initialized`, and `tools/call`) without
adding a new pip dependency (no `mcp` SDK on the path these scripts run on).

Implemented as a context-manager class (`with McpClient() as client: ...`) so
the subprocess spawned for `run-ferroplan-mcp.sh` is always terminated on exit,
including on exception -- these scripts are invoked from Claude Code hooks and
must never leave orphaned `cargo run`/`ferroplan-mcp` processes behind. To that
end `__enter__` also guarantees cleanup if the handshake itself fails partway
through (a bad `initialize` response, or the subprocess dying right after
spawn) -- previously such a failure raised out of `_start()` before `with`
ever considered the block "entered", so `__exit__`/`close()` never ran and the
subprocess leaked.

Reads off the subprocess are bounded by `timeout` (constructor default, or a
per-call override) via a background reader thread feeding a queue -- a plain
blocking `readline()` has no way to time out, so a hung/misbehaving
`ferroplan-mcp` process (e.g. stuck mid-build) would otherwise wedge the
calling hook indefinitely.
"""

from __future__ import annotations

import json
import os
import queue
import subprocess
import sys
import threading
from pathlib import Path
from typing import Any


class McpToolError(RuntimeError):
    """Raised when an MCP JSON-RPC call errors, times out, or a tool result has isError=true."""


# Sentinel placed on the reader-thread queue when the subprocess's stdout closes.
_EOF = object()


class McpClient:
    """Spawns `run-ferroplan-mcp.sh`, performs the MCP stdio handshake, and
    exposes `call_tool` for JSON-RPC `tools/call` requests.

    Usage:
        with McpClient() as client:
            result = client.call_tool("verify_receipt", {"envelope": envelope})
    """

    def __init__(self, *, launcher: Path | None = None, timeout: float = 30.0) -> None:
        self._launcher = launcher or Path(__file__).resolve().parent / "run-ferroplan-mcp.sh"
        self._timeout = timeout
        self._next_id = 1
        self._process: subprocess.Popen[str] | None = None
        self._line_queue: queue.Queue[Any] = queue.Queue()
        self._reader_thread: threading.Thread | None = None

    def __enter__(self) -> McpClient:
        try:
            self._start()
        except BaseException:
            # _start() can raise partway through spawn/handshake; because that
            # happens before this method returns, `with` never considers the
            # block entered and __exit__ is never called. Clean up here so a
            # failed handshake can't leak the subprocess.
            self.close()
            raise
        return self

    def __exit__(self, *exc_info: object) -> None:
        self.close()

    def _start(self) -> None:
        if not self._launcher.exists():
            raise McpToolError(f"MCP launcher not found: {self._launcher}")
        env = dict(os.environ)
        env.setdefault("CLAUDE_PROJECT_DIR", str(self._launcher.resolve().parent.parent.parent.parent))
        self._process = subprocess.Popen(
            [str(self._launcher)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=env,
        )
        self._reader_thread = threading.Thread(target=self._read_loop, daemon=True)
        self._reader_thread.start()
        self._send_notification(
            "initialize",
            {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "chatman-ecosystem", "version": "0.1"},
            },
            expect_response=True,
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
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
        # The reader thread exits on its own once stdout closes (which killing
        # or a normal process exit both cause); it's a daemon thread either
        # way, so no explicit join is required for correctness here.

    def _read_loop(self) -> None:
        """Runs on a background thread: pushes every stdout line (and finally
        an EOF sentinel) onto `self._line_queue`, so reads can be bounded by a
        timeout even though `readline()` itself has no timeout parameter."""
        process = self._process
        stdout = process.stdout if process else None
        try:
            if stdout is not None:
                for line in stdout:
                    self._line_queue.put(line)
        except (BrokenPipeError, OSError, ValueError):
            # ValueError: reading from a stream after it's been closed by close().
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
        while True:
            try:
                line = self._line_queue.get(timeout=effective_timeout)
            except queue.Empty as error:
                raise McpToolError(
                    f"MCP subprocess did not respond to id={expected_id} within {effective_timeout}s"
                ) from error
            if line is _EOF:
                stderr = process.stderr.read() if process.stderr else ""
                raise McpToolError(
                    f"MCP subprocess closed stdout before responding to id={expected_id}: {stderr.strip()}"
                )
            line = line.strip()
            if not line:
                continue
            try:
                message = json.loads(line)
            except json.JSONDecodeError as error:
                raise McpToolError(f"MCP subprocess emitted non-JSON line: {line!r}") from error
            if message.get("id") == expected_id:
                return message
            # Not our response (e.g. a server-initiated notification) -- ignore and keep reading.

    def _send_notification(
        self, method: str, params: dict[str, Any], *, expect_response: bool
    ) -> dict[str, Any] | None:
        request_id = self._next_id
        self._next_id += 1
        self._write({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        if not expect_response:
            return None
        response = self._read_response(request_id)
        if "error" in response:
            raise McpToolError(f"MCP `{method}` failed: {response['error']}")
        return response.get("result")

    def call_tool(self, name: str, arguments: dict[str, Any], *, timeout: float | None = None) -> dict[str, Any]:
        """Send `tools/call` for `name` with `arguments` and return the tool result.

        Raises McpToolError with the actual error text if the JSON-RPC response
        is an error, if the tool result declares `isError: true`, or if no
        response arrives within `timeout` seconds (constructor default if
        unset here).
        """
        request_id = self._next_id
        self._next_id += 1
        self._write(
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "method": "tools/call",
                "params": {"name": name, "arguments": arguments},
            }
        )
        response = self._read_response(request_id, timeout=timeout)
        if "error" in response:
            raise McpToolError(f"MCP tool `{name}` call errored: {response['error']}")
        result = response.get("result")
        if not isinstance(result, dict):
            raise McpToolError(f"MCP tool `{name}` returned an unexpected response: {response!r}")
        if result.get("isError"):
            text = _extract_text(result)
            raise McpToolError(f"MCP tool `{name}` reported an error: {text}")
        return result


def _extract_text(result: dict[str, Any]) -> str:
    parts = []
    for block in result.get("content", []) or []:
        if isinstance(block, dict) and block.get("type") == "text":
            parts.append(str(block.get("text", "")))
    return "\n".join(parts) if parts else json.dumps(result)


def tool_structured_result(result: dict[str, Any]) -> Any:
    """Return `structuredContent` if present, otherwise parse the text content block."""
    if "structuredContent" in result and result["structuredContent"] is not None:
        return result["structuredContent"]
    text = _extract_text(result)
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return text


if __name__ == "__main__":  # pragma: no cover - manual smoke test entry point
    with McpClient() as client:
        digest = client.call_tool("canonical_digest", {"value": {"b": 1, "a": 2}})
        print(json.dumps(tool_structured_result(digest), indent=2))
    sys.exit(0)
