from __future__ import annotations

import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from openai_luna_testkit import fake_mcp_server, fake_star_launcher, profile
from mcp_client import McpClient
from openai_luna_protocol import A2AClient, OstarStarClient, OpenAIResponsesClient
from openai_luna_runtime import LunaHost, verify_trace


class _A2AHandler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        return

    def _send(self, value):
        body = json.dumps(value).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        assert self.path == "/agent-card"
        self._send({"name": "OntoStar", "skills": ["admit"]})

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        json.loads(self.rfile.read(length) or b"{}")
        self._send({"status": "ok"})


def test_real_stdio_clients_and_star_launcher(tmp_path: Path) -> None:
    ferro = fake_mcp_server(tmp_path / "ferro.py", "ferroplan")
    onto = fake_mcp_server(tmp_path / "onto.py", "ontostar")
    star = fake_star_launcher(tmp_path / "star.py")
    scripted = iter([
        {
            "id": "r1",
            "output": [
                {"type": "function_call", "call_id": "f1", "name": "ferroplan__solve", "arguments": "{}"},
                {"type": "function_call", "call_id": "o1", "name": "ontostar__onto_admit_work_order", "arguments": "{}"},
            ],
        },
        {"id": "r2", "output": [{"type": "message", "content": [{"type": "output_text", "text": "done"}]}]},
    ])
    host = LunaHost(
        profile(),
        OpenAIResponsesClient(transport=lambda _payload: next(scripted)),
        OstarStarClient(star, tmp_path),
        {
            "ferroplan": McpClient(launcher=ferro, project_root=tmp_path, timeout=5),
            "ontostar": McpClient(launcher=onto, project_root=tmp_path, timeout=5),
        },
    )
    trace = host.run("execute", "ferroplan")
    assert trace["standing"] == "ALIVE"
    assert verify_trace(trace)["valid"]


def test_real_a2a_probe_is_coordination_only() -> None:
    server = ThreadingHTTPServer(("127.0.0.1", 0), _A2AHandler)
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        probe = A2AClient(f"http://127.0.0.1:{server.server_port}").probe()
        assert probe["agent_card"]["name"] == "OntoStar"
        assert probe["heartbeat"]["status"] == "ok"
        assert probe["authority"] == "coordination-only"
    finally:
        server.shutdown()
        thread.join(timeout=5)
