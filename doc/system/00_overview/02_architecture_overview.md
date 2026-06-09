## 02. Architecture Overview

A thin axum service over a pure assembly pipeline.

### Module map

- `gather.rs` — read-only repo discovery. For a target file, produces four
  governed source roles (target / adjacent / repo-truth / validation) plus
  deterministic `RepoFacts` (dirs, entry points, canonical docs, validation
  commands). Detects stack (rust / python / node / generic).
- `assemble.rs` — the pipeline. Maps each source role to a PCC governance
  `SourceClass`, builds a `ContextAssemblyRequest`, calls the real
  `assemble_context`, then builds a validated code-native payload behind each
  admitted ref. Returns an `AssembledBundle`.
- `payload.rs` — constructs and validates the code-native PCC contracts
  (`KeyFilePacketContract` / `RepoNavigationMapContract` /
  `ValidationCommandPacketContract`) on a fully-valid `ArtifactRecord` base.
- `store.rs` — in-process bundle store; the authority for which refs are
  admitted in a bundle. Fails closed on unknown bundle or scope escape.
- `http.rs` — axum 0.8 surface (see Contract Surface).
- `config.rs`, `error.rs`, `main.rs` — env config, fail-closed error→HTTP
  mapping, server entrypoint.

### Two-surface design (Option 3)

The **envelope** is PCC's `ContextBundleManifest` — the only thing in PCC that
emits a hashed, replay-eligible governed bundle. The **payload** behind each ref
is a PCC code-native contract. The code→authoring class mapping
(target→`ActiveScene`, adjacent→`AdjacentScene`, repo-truth→`AcceptedLoreRecord`,
validation→`AcceptedStyleRuleRecord`) lives entirely inside `assemble.rs`; the
wire output is refs + a hash + typed payloads.
