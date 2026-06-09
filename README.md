# context-runtime (CTX)

The PCC-conforming **context runtime** for the self-healing loop. Turns a code
target (repo + file) into a governed context bundle the AI shaper can build a fix
from, safely.

> Canonical docs live in `doc/system/` and assemble to `doc/CTXSYSTEM.md`
> (`bash doc/system/BUILD.sh`). Read that first; this README is a pointer.

## What it does

For a target file it gathers candidate sources read-only, runs the **real**
`precomputed_context_core::assemble_context` to produce a governed
`ContextBundleManifest` (admissibility / freshness / authority / deterministic
`bundle_hash` / replay eligibility), builds a validated code-native PCC contract
behind each admitted ref (`KeyFilePacket` / `RepoNavigationMap` /
`ValidationCommandPacket`), and serves both over HTTP — fail-closed on scope
escape. It fills the "adapter boundary" forgeHQ deferred.

## API

- `POST /v1/context/assemble` → `{ context_bundle_id, bundle_hash, manifest,
  payload_refs, context_item_refs }`
- `GET /v1/context/{bundle_id}/payload?ref=<payload_ref>` → typed payload
  (`409` on scope escape, `404` on unknown bundle)
- `GET /healthz`

## Run

```bash
cargo build --offline --bin context-runtime
CONTEXT_RUNTIME_BIND=127.0.0.1:8011 ./target/debug/context-runtime
```

## Verify

```bash
cargo test --offline
# Rust<->Python crossing (run client from a project venv):
<venv>/bin/python scripts/smoke_crossing.py http://127.0.0.1:8011
```

## Boundaries

No LLM, no persistence, no verification, no proposals. It governs and serves
context. PCC owns contract shape; DataForge-Local owns durable truth; pact
verifies; forgeHQ proposes.
