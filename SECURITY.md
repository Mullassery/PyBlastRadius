# Security Policy

## Supported Versions

PyBlastRadius is pre-1.0 (`v0.3.0` at the time of writing). There is no
long-term support branch — only the latest release on PyPI and the `main`
branch are considered, and only `main` receives fixes.

| Version | Supported |
| ------- | --------- |
| latest `v0.x` on PyPI | yes |
| anything older | no |

## Reporting a Vulnerability

Please report security issues privately, not as a public GitHub issue:

- Email **mullassery@gmail.com** with a description of the issue, steps to
  reproduce, and (if known) the affected version/commit.
- This is a single-maintainer project run outside of paid work hours —
  there is no SLA. Best-effort acknowledgement is typically within a few
  days, but that is not a guarantee.

## Known Security-Relevant Facts (as of 2026-09-19)

Disclosed here rather than left implicit, per this project's honesty policy:

- **REST API auth**: `pyblastradius server` (`python/pyblastradius/server.py`)
  implements bearer-token auth via `PYBLASTRADIUS_API_KEY`/
  `PYBLASTRADIUS_API_KEYS` and an explicit CORS allowlist. This has pytest
  coverage (`tests/test_server.py`) that was verified passing during this
  audit (2026-09-19), but the server has not had a dedicated security
  review (e.g. rate limiting, timing-safe key comparison, key rotation are
  not verified either way — check the source directly before exposing this
  server to an untrusted network).
- **Dependency scanning**: `.github/workflows/ci.yml` now runs `cargo audit`
  (added 2026-09-19) against `Cargo.lock` on every push/PR. There is no
  equivalent automated scan for the Python dependencies yet (`click`,
  optionally `fastapi`/`uvicorn`/`rich`/`pyyaml`) — a manual `pip-audit` run
  during this same pass found no known vulnerabilities in the resolved
  Python dependency set, but that was a one-time check, not continuous.
- **`Cargo.lock`**: now committed (previously gitignored — see
  `ROADMAP_HONEST.md`), so `cargo audit` and builds are reproducible against
  a known dependency set rather than whatever happened to resolve locally.
- The chat/incident-management webhook code in `src/api/integrations.rs`
  does not send any HTTP requests today (calls are commented out) — there
  is no live outbound-webhook attack surface from that code path yet.

If you find something not covered here, please still report it privately
first.
