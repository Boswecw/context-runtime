## 40. Validation and Proof

Validation is evidence-based and fail-closed.

### Minimum validation posture

- `cargo test --offline` must pass.
- Every rejected path (stale source, missing required source, unresolved
  authority, missing target, missing repo, scope escape, invalid payload
  contract) must fail closed — no success envelope on rejection.
- Assembly must be deterministic: identical inputs → identical `bundle_hash`.

### Current proof surface

`tests/assemble.rs` (hermetic temp repo + real forgeHQ if present):

- governed bundle with code-native payloads assembles and validates
- assembly is deterministic (same inputs → same hash)
- stale source fails closed
- missing target / missing repo fail closed
- store serves admitted refs and fails closed on scope escape / unknown bundle
- assembles cleanly against a real forgeHQ source file when present

`tests/http.rs`: boots the real axum server on an ephemeral port; assemble →
fetch payload → scope-escape 409 → unknown-bundle 404.

`scripts/smoke_crossing.py`: stdlib-only client (run from a project venv) that
drives the running service exactly as forgeHQ will — proves the Rust↔Python
crossing end to end.

### Running

```bash
cargo test --offline
# crossing smoke:
CONTEXT_RUNTIME_BIND=127.0.0.1:8011 ./target/debug/context-runtime &
<venv>/bin/python scripts/smoke_crossing.py http://127.0.0.1:8011
```
