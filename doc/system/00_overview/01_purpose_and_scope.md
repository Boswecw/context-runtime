## 01. Purpose and Scope

`context-runtime` turns a code target (repo + file) into a **governed context
bundle** the self-healing AI shaper can build a fix from, safely.

### Role in the self-healing loop

```
[context-runtime: gather -> assemble(PCC) -> serve]
   -> forgeHQ AI shaper (context packet -> generate -> pact-verify -> propose)
   -> DataForge-Local healing-proposals -> FC /self-healing -> operator -> apply
```

It fills the gap forgeHQ explicitly deferred: forgeHQ's pipeline governs *scope*
(`ContextBundle.context_item_refs`; the generator fail-closes on any ref outside
the admitted set) but does not produce the admitted refs in a governed way, nor
hold the content behind them ("the adapter boundary is intentionally not wired").
`context-runtime` is that producer and that adapter.

### In-scope posture

- Read-only repo discovery (gather candidate sources for a target file).
- Run the real `precomputed_context_core::assemble_context` to produce a
  governed `ContextBundleManifest` (admissibility / freshness / authority /
  deterministic `bundle_hash` / replay eligibility).
- Build a validated code-native PCC contract behind each admitted ref
  (`KeyFilePacket` / `RepoNavigationMap` / `ValidationCommandPacket`).
- Serve the manifest + payloads over HTTP, fail-closed on scope escape.

### Out-of-scope posture

This repo does not run an LLM, does not persist truth (DataForge-Local owns
durable operational truth), does not verify fixes (that is `pact`), and does not
mint upstream truth. It governs and serves context; it proposes nothing.
