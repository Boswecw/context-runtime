#!/usr/bin/env python3
"""
Rust<->Python crossing smoke for context-runtime.

Drives the running Rust service exactly as forgeHQ will: assemble a governed
context bundle for a real source file, fetch a code-native payload, and confirm
the service fails closed on a scope-escape ref. Stdlib-only (matches forgeHQ's
stdlib-first driver posture).

Usage:
    python3 smoke_crossing.py [BASE_URL] [REPO_ROOT] [TARGET_FILE]
Defaults:
    BASE_URL    = http://127.0.0.1:8011
    REPO_ROOT   = <ecosystem>/local-systems/forgeHQ
    TARGET_FILE = app/services/context_bundle_service.py
"""
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

BASE = sys.argv[1] if len(sys.argv) > 1 else os.environ.get(
    "CONTEXT_RUNTIME_URL", "http://127.0.0.1:8011"
)
_here = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = sys.argv[2] if len(sys.argv) > 2 else os.path.abspath(
    os.path.join(_here, "..", "..", "forgeHQ")
)
TARGET = sys.argv[3] if len(sys.argv) > 3 else "app/services/context_bundle_service.py"


def _req(method, url, body=None):
    data = json.dumps(body).encode() if body is not None else None
    headers = {"content-type": "application/json"} if data else {}
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            return resp.status, json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read().decode() or "{}")


def main():
    print(f"[smoke] base={BASE} repo_root={REPO_ROOT} target={TARGET}")

    status, health = _req("GET", f"{BASE}/healthz")
    assert status == 200 and health.get("ok") is True, f"healthz failed: {status} {health}"
    print(f"[smoke] healthz ok — contract={health['contract']!r}")

    status, bundle = _req(
        "POST",
        f"{BASE}/v1/context/assemble",
        {
            "repo_id": "forgehq",
            "repo_root": REPO_ROOT,
            "target_file": TARGET,
            # Generous ceiling for the happy-path demo; the default 7-day policy
            # would (correctly) fail closed on an older repo. Stale rejection is
            # covered by the Rust test suite.
            "max_source_age_minutes": 5_256_000,
        },
    )
    assert status == 200, f"assemble failed: {status} {bundle}"
    bundle_id = bundle["context_bundle_id"]
    refs = bundle["context_item_refs"]
    assert bundle_id.startswith("ctxb_"), bundle_id
    assert bundle["bundle_hash"], "missing bundle_hash"
    assert refs, "no admitted refs"
    print(
        f"[smoke] assembled {bundle_id} hash={bundle['bundle_hash']} "
        f"refs={len(refs)} freshness={bundle['manifest']['freshness_band']}"
    )

    target_ref = f"file://forgehq/{TARGET}"
    assert target_ref in refs, f"target ref not admitted: {target_ref}"
    status, payload = _req(
        "GET", f"{BASE}/v1/context/{bundle_id}/payload?ref={urllib.parse.quote(target_ref, safe='')}"
    )
    assert status == 200, f"payload fetch failed: {status} {payload}"
    assert payload["artifact_class"] == "key_file_packet", payload["artifact_class"]
    assert payload["contract"]["file_path"] == TARGET, payload["contract"]["file_path"]
    print(
        f"[smoke] payload ok — class={payload['artifact_class']} "
        f"file_path={payload['contract']['file_path']} hash={payload['content_hash'][:12]}..."
    )

    # Scope escape must fail closed (409).
    escape_ref = "file://forgehq/not/admitted.py"
    status, _ = _req(
        "GET",
        f"{BASE}/v1/context/{bundle_id}/payload?ref={urllib.parse.quote(escape_ref, safe='')}",
    )
    assert status == 409, f"scope escape should be 409, got {status}"
    print("[smoke] scope-escape correctly rejected (409)")

    print("[smoke] PASS — Rust<->Python crossing proven end to end")


if __name__ == "__main__":
    main()
