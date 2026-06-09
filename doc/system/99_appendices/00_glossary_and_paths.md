## 99. Appendices — Glossary and Paths

### Important paths

- Repo root: `~/Forge/ecosystem/local-systems/context-runtime`
- Documentation source root: `doc/system/`
- Build entry: `doc/system/BUILD.sh` → `doc/CTXSYSTEM.md`
- Pipeline: `src/assemble.rs`; HTTP: `src/http.rs`; gather: `src/gather.rs`;
  payloads: `src/payload.rs`
- Contract authority: `../../precomputed-context-core` (`PCC`)
- Consumer: `../forgeHQ` (`ContextBundleService`, candidate pipeline)

### Glossary

- **Envelope:** PCC's `ContextBundleManifest` — governed, hashed, replay-eligible
  bundle of source refs.
- **Payload:** the validated code-native PCC contract behind one admitted ref.
- **Admitted ref:** a `payload_ref` present in a bundle's inventory; the only
  refs the service will serve and the generator may modify.
- **Scope escape:** a request for a ref not admitted in the bundle — rejected
  fail-closed (`409`).
- **Fail-closed:** an invalid or rejected path returns an error and never a
  success envelope.
