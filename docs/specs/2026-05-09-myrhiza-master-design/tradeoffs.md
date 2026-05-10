**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Tradeoffs


## 18. Tradeoffs surfaced

| Decision | Runner-up | Why rejected |
|---|---|---|
| Event-log replay (paradigm) | Validating DHT (Holochain) | 6+ year unfinished sharding story; throws out Q3 progress |
| Full Component Model day-one | Extism v1 → CM v2 | Double-rewrite cost for app authors and Willow |
| Dual-stack v1 | Native-first + browser v1.5 | v1.5 slip risk; browser is project pitch |
| Module ecosystem (3-tier) | Kernel-baked features | Module-level evolution faster than kernel ABI; vendor lock-in avoided |
| Layered cap gating | Per-call only | Modules need containment; layered defense in depth |
| `IdentityScope` unified | Separate per-domain | Triple design + impl cost; PR #636 names structural similarity |
| MLS as module | Kernel-baked MLS | Vendor lock-in (one impl); kernel surface bloat; PQ migration kernel break |
| Ed25519 + iroh-blobs (P2P) | OCI + sigstore | Centralizes what we made P2P; Sigstore Public Good single point |
| Float ban in state-apply v1 | Spec-pinned floats | NaN canonicalization + SIMD divergence vectors; debugging painful |
| WASM on every backend | Native compilation for performance | Defeats sandbox model; capability discipline requires WASM execution |
| No worker class | Worker-as-peer-class | Architecturally honest; doesn't close paths; v1 ships zero |
| Counter + poll MVP | Single app or chat MVP | Coexistence (#4) requires two apps; chat recapitulates Willow |


