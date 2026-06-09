## 30. Dependencies

### Primary technical dependencies

- Rust toolchain (edition 2024).
- `precomputed-context-core` (path dependency) — the governed contract surface.
- `axum` 0.8 + `tokio` 1 — HTTP runtime (matches the ecosystem convention in
  `Forge_Command/api`).
- `serde` / `serde_json` — request/response and contract serialization.
- `sha2` 0.10 — payload content integrity hashes.
- `chrono` 0.4 — artifact record timestamps.
- `thiserror` 2, `tracing` / `tracing-subscriber` — errors and logging.
- dev: `reqwest` 0.12 — HTTP test client.

### Dependency posture

Dependencies are accepted only when they support the runtime, the real PCC
contracts, deterministic hashing, or fail-closed serving. No persistence or LLM
dependency in C1.
