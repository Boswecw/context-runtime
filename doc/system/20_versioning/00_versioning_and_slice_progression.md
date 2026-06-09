## 20. Versioning and Slice Progression

This repo advances through bounded slices, fail-closed.

### Current slice position — C1 (done)

C1 = "governed bundle over HTTP + Rust↔Python crossing", Option 3:

- Real `assemble_context` envelope from gathered live repo sources.
- Code-native, `.validate()`-passing PCC payloads behind every admitted ref.
- axum HTTP surface, fail-closed on scope escape / unknown bundle.
- Deterministic `bundle_hash` (replay handle).
- Rust↔Python crossing proven via `scripts/smoke_crossing.py`.

### Planned

- **C2** — richer gathering (import-aware adjacency, multiple key files,
  per-source authority levels) and DataForge-Local persistence + replay of
  bundles.
- **C3** — wire into forgeHQ: a context-client driver feeds
  `ContextBundleService` / `candidate_design` / `candidate_generation` with the
  governed refs + payloads.
- **C4** — pact verification stage consumes the bundle handle; then the forgeHQ
  AI shaper (request context packet → NeuroForge generate → pact-verify →
  propose).
