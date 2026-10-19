"""Real HTTP tests for the exposed Slack/PagerDuty Python integrations."""

from __future__ import annotations

import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest

from pyblastradius import PagerDutyIntegration, SlackIntegration


class _Handler(BaseHTTPRequestHandler):
    calls: list[tuple[str, str]] = []

    def do_POST(self):  # noqa: N802 - BaseHTTPRequestHandler API
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length).decode("utf-8")
        type(self).calls.append((self.path, body))

        if "fail" in self.path:
            payload, status = b'{"error":"boom"}', 500
        else:
            payload, status = b'{"dedup_key":"abc123"}', 200

        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *args):
        pass


@pytest.fixture
def server():
    _Handler.calls = []
    srv = HTTPServer(("127.0.0.1", 0), _Handler)
    thread = threading.Thread(target=srv.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        thread.join(timeout=5)


def test_slack_sends_real_post(server):
    SlackIntegration(f"{server}/slack").send_blast_radius_alert(
        "payment-service", 0.9, ["db", "cache"]
    )
    assert len(_Handler.calls) == 1
    path, body = _Handler.calls[0]
    assert path == "/slack"
    payload = json.loads(body)
    assert "payment-service" in payload["text"]
    assert any("Blast Radius Score" in f["title"] for f in payload["attachments"][0]["fields"])


def test_pagerduty_returns_dedup_key(server):
    client = PagerDutyIntegration("key123", api_url=f"{server}/pd")
    assert client.create_incident("svc", 0.95, "desc") == "abc123"
    assert _Handler.calls[0][0] == "/pd"


def test_non_2xx_raises(server):
    with pytest.raises(RuntimeError, match="HTTP 500"):
        SlackIntegration(f"{server}/fail").send_anomaly_alert(
            "postgres", "latency_spike", 0.85, 15
        )
