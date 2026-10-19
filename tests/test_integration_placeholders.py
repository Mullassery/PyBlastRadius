"""Placeholder integration clients must fail explicitly, not fabricate data."""

from __future__ import annotations

import pytest

from pyblastradius.integrations import OtelCollectorClient, StatGuardianClient


def test_otel_discovery_is_not_implemented():
    with pytest.raises(NotImplementedError, match="OTel trace discovery"):
        OtelCollectorClient().discover_dependencies()


def test_statguardian_clients_do_not_fabricate():
    client = StatGuardianClient("http://127.0.0.1:9", "key")
    for call in (
        client.get_quality_rules,
        client.get_quality_events,
        lambda: client.track_quality_incident({"x": 1}),
    ):
        with pytest.raises(NotImplementedError, match="StatGuardian"):
            call()
