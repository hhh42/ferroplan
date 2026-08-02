from __future__ import annotations

import json
import os
import subprocess
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from openai_luna_runtime import verify_trace
from openai_luna_testkit import fake_mcp_server, fake_star_launcher

PLUGIN = Path(__file__).resolve().parents[1]
CLI = PLUGIN / "scripts" / "openai_luna.py"
PROFILE = PLUGIN / "profiles" / "openai-luna.json"


class _ResponsesHandler(BaseHTTPRequestHandler):
    calls = 0
    payloads: list[dict] = []

    def log_message(self, *_args):
        return

    def do_POST(self):
        assert self.path == "/v1/responses"
        length = int(self.headers.get("Content-Length", "0"))
        payload = json.loads(self.rfile.read(length))
        type(self).payloads.append(payload)
        type(self).calls += 1
        if type(self).calls == 1:
            value = {
                "id": "resp-e2e-1",
                "output": [
                    {"type": "function_call", "call_id": "f1", "name": "ferroplan__solve", "arguments": "{}"},
                    {"type": "function_call", "call_id": "o1", "name": "ontostar__onto_admit_work_order", "arguments": "{}"},
                ],
            }
        else:
            value = {"id": "resp-e2e-2", "output": [{"type": "message", "content": [{"type": "output_text", "text": "e2e complete"}]}]}
        body = json.dumps(value).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def test_black_box_cli_end_to_end(tmp_path: Path) -> None:
    _ResponsesHandler.calls = 0
    _ResponsesHandler.payloads = []
    server = ThreadingHTTPServer(("127.0.0.1", 0), _ResponsesHandler)
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        ferro = fake_mcp_server(tmp_path / "ferro.py", "ferroplan")
        onto = fake_mcp_server(tmp_path / "onto.py", "ontostar")
        star = fake_star_launcher(tmp_path / "star.py")
        receipt = tmp_path / "receipt.json"
        env = dict(os.environ)
        env.update(
            OPENAI_API_KEY="test-key",
            OPENAI_BASE_URL=f"http://127.0.0.1:{server.server_port}/v1",
        )
        run = subprocess.run(
            [
                sys.executable,
                str(CLI),
                "--project",
                str(tmp_path),
                "--profile",
                str(PROFILE),
                "--star-launcher",
                str(star),
                "--ferroplan-launcher",
                str(ferro),
                "--ontostar-launcher",
                str(onto),
                "--receipt",
                str(receipt),
                "execute the bounded chain",
            ],
            cwd=tmp_path,
            env=env,
            capture_output=True,
            text=True,
            timeout=15,
        )
        assert run.returncode == 0, run.stderr or run.stdout
        trace = json.loads(run.stdout)
        assert trace["standing"] == "ALIVE"
        assert json.loads(receipt.read_text()) == trace
        assert verify_trace(trace)["valid"]
        assert _ResponsesHandler.calls == 2
        assert _ResponsesHandler.payloads[0]["model"] == "gpt-5.6-luna"
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


def test_cli_empty_prompt_is_usage_refusal(tmp_path: Path) -> None:
    run = subprocess.run(
        [sys.executable, str(CLI), "--profile", str(PROFILE)],
        cwd=tmp_path,
        input="   ",
        capture_output=True,
        text=True,
        timeout=5,
    )
    assert run.returncode == 64
    assert json.loads(run.stderr)["code"] == "TASK_EMPTY"
