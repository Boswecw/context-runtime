## 00. Identity

**Repo:** `context-runtime`
**Proposed Designation:** `CTX`
**Repo Root:** `~/Forge/ecosystem/local-systems/context-runtime`
**Repo Class:** Service / Runtime (local-systems)
**Language:** Rust
**Operational Posture:** Internal business system, single-operator, local, fail-closed

### Identity note

`context-runtime` is the PCC-conforming context **runtime** for the self-healing
loop. `precomputed-context-core` (`PCC`) is a Library/Contract crate that, by
design, owns no service runtime; this repo is that runtime. It depends on PCC's
real exported contracts and never redefines them.
