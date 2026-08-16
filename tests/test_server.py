"""Tests for the REST API server's auth and CORS behavior.

server.py does its safety checks (refusing to start unauthenticated) and
builds its CORS middleware at *import time*, keyed off environment
variables read once at module load. To test multiple configurations
cleanly (with vs. without an API key, with vs. without the no-auth opt-out)
without fighting Python's module cache, each scenario here runs in its own
subprocess with a controlled environment, using FastAPI's TestClient
in-process within that subprocess (no real socket/port is bound).

Skipped automatically if fastapi/uvicorn (the `server` extra) aren't
installed, matching server.py's own ImportError message.
"""

import json
import os
import subprocess
import sys

import pytest

pytest.importorskip("fastapi")
pytest.importorskip("uvicorn")


def _run(env_overrides, script):
    """Run `script` in a fresh subprocess with a clean env + overrides.

    Returns (returncode, stdout, stderr).
    """
    env = os.environ.copy()
    # Start from a clean slate for the auth-relevant vars so leftovers from
    # the developer's shell (or a previous scenario) can't leak in.
    for key in (
        "PYBLASTRADIUS_API_KEY",
        "PYBLASTRADIUS_API_KEYS",
        "PYBLASTRADIUS_ALLOW_NO_AUTH",
        "PYBLASTRADIUS_CORS_ORIGINS",
    ):
        env.pop(key, None)
    env.update(env_overrides)

    proc = subprocess.run(
        [sys.executable, "-c", script],
        env=env,
        capture_output=True,
        text=True,
        timeout=60,
    )
    return proc.returncode, proc.stdout, proc.stderr


def test_server_refuses_to_start_without_api_key_or_opt_out():
    rc, _out, err = _run({}, "import pyblastradius.server")
    assert rc != 0
    assert "No API key configured" in err


def test_server_starts_with_explicit_no_auth_opt_out_for_dev():
    rc, out, err = _run(
        {"PYBLASTRADIUS_ALLOW_NO_AUTH": "1"},
        "import pyblastradius.server; print('OK')",
    )
    assert rc == 0, err
    assert "OK" in out


def test_health_endpoint_is_unauthenticated():
    script = """
import os
from fastapi.testclient import TestClient
import pyblastradius.server as server

client = TestClient(server.app)
r = client.get("/health")
print(r.status_code)
"""
    rc, out, err = _run({"PYBLASTRADIUS_API_KEY": "test-key-123"}, script)
    assert rc == 0, err
    assert out.strip() == "200"


def test_protected_endpoint_rejects_missing_and_wrong_key():
    script = """
from fastapi.testclient import TestClient
import pyblastradius.server as server

client = TestClient(server.app)
no_auth = client.get("/api/v1/status")
wrong_key = client.get("/api/v1/status", headers={"Authorization": "Bearer nope"})
print(no_auth.status_code, wrong_key.status_code)
"""
    rc, out, err = _run({"PYBLASTRADIUS_API_KEY": "test-key-123"}, script)
    assert rc == 0, err
    assert out.strip() == "401 401"


def test_protected_endpoint_accepts_correct_key_with_and_without_bearer_prefix():
    script = """
from fastapi.testclient import TestClient
import pyblastradius.server as server

client = TestClient(server.app)
with_bearer = client.get("/api/v1/status", headers={"Authorization": "Bearer test-key-123"})
raw_token = client.get("/api/v1/status", headers={"Authorization": "test-key-123"})
print(with_bearer.status_code, raw_token.status_code)
"""
    rc, out, err = _run({"PYBLASTRADIUS_API_KEY": "test-key-123"}, script)
    assert rc == 0, err
    assert out.strip() == "200 200"


def test_multiple_api_keys_all_accepted():
    script = """
from fastapi.testclient import TestClient
import pyblastradius.server as server

client = TestClient(server.app)
r1 = client.get("/api/v1/status", headers={"Authorization": "Bearer key-one"})
r2 = client.get("/api/v1/status", headers={"Authorization": "Bearer key-two"})
r3 = client.get("/api/v1/status", headers={"Authorization": "Bearer key-three"})
print(r1.status_code, r2.status_code, r3.status_code)
"""
    rc, out, err = _run({"PYBLASTRADIUS_API_KEYS": "key-one,key-two"}, script)
    assert rc == 0, err
    assert out.strip() == "200 200 401"


def test_cors_defaults_to_no_cross_origin_access_and_never_wildcards():
    script = """
import pyblastradius.server as server

for middleware in server.app.user_middleware:
    kwargs = middleware.kwargs
    if "allow_origins" in kwargs:
        print(kwargs["allow_origins"])
"""
    rc, out, err = _run({"PYBLASTRADIUS_API_KEY": "test-key-123"}, script)
    assert rc == 0, err
    assert out.strip() == "[]"


def test_cors_honors_explicit_allowlist_and_still_never_wildcards():
    script = """
import json
import pyblastradius.server as server

for middleware in server.app.user_middleware:
    kwargs = middleware.kwargs
    if "allow_origins" in kwargs:
        print(json.dumps(kwargs["allow_origins"]))
"""
    rc, out, err = _run(
        {
            "PYBLASTRADIUS_API_KEY": "test-key-123",
            "PYBLASTRADIUS_CORS_ORIGINS": "https://app.example.com,https://ops.example.com",
        },
        script,
    )
    assert rc == 0, err
    origins = json.loads(out.strip())
    assert origins == ["https://app.example.com", "https://ops.example.com"]
    assert "*" not in origins
