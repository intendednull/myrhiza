**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Implementation outline

# Implementation outline (handed off to writing-plans)

## 20. Implementation outline (handed off to writing-plans)

Implementation plan lives at
`docs/plans/2026-05-09-myrhiza-master-design.md`. Critical path
(reordered per implementation feasibility review — manifest schema
+ capability gating + backend trait abstraction must be designed in
from the start, not retrofitted):

1. **Workspace scaffold** + initial crate structure (`kernel/`,
   `sdk/`, `network/`, `storage/`, `crypto/`, `examples/counter/`,
   `examples/poll/`, `tests/`).
2. **Core types**: `IdentityScope`, `IdentityHandle`, `EventHash`,
   `Event`, `Topic`, `BundleHash`, manifest schema types.
3. **State-digest format pin** (decision step): commit `bincode 1.3.x`
   with default config; sorted-collection discipline doc.
4. **WIT package authoring**: state-apply, state-propose, interaction,
   behavior worlds; canonical kernel host import surface (per §3.5).
5. **Manifest schema implementation + capability vocabulary**:
   TOML parser; capability vocabulary registry; v1-mandatory
   high-value-op list; signature verification.
6. **Wasmtime backend with capability-gated linker** (designed in
   from start, not retrofitted): component instantiation; per-call
   gate dispatch; manifest intersection at instantiation.
7. **Backend trait abstraction**: stable internal trait both Wasmtime
   and jco backends will satisfy. Wasmtime impl satisfies it; jco
   impl deferred to step 17.
8. **State-apply ABI** + deterministic helper set + fuel budget +
   float-ban byte-level lint.
9. **Pre-check unification + drift detection scaffold**: dry-run
   path for state-apply; periodic state-digest gossip stub.
10. **Event/DAG primitives**: per-author Merkle DAG storage;
    topo-sort with EventHash lex tie-break; PendingBuffer (1h TTL,
    10K entries, per-author /50 sub-cap).
11. **iroh integration**: gossip wrapper, blob fetch wrapper, network
    trait abstraction (production iroh + MemNetwork test double).
12. **HeadsSummary sync** + drift-detection digest gossip integration.
13. **Crypto primitives**: host imports backed by Rust crypto crates
    (ed25519-dalek RFC 8032 strict, x25519-dalek, chacha20poly1305,
    hkdf-sha256, blake3).
14. **Bundle distribution + signing**: Ed25519 over canonical
    manifest+content+version+pubkey encoding; iroh-blobs publication
    and fetch; revocation topic auto-subscribe per §10.7.
15. **Counter app**: state-apply, propose, interaction, manifest.
16. **Poll app**: same shape.
17. **State-tier tests** for both apps.
18. **Kernel-tier tests** (kernel + MemNetwork): convergence,
    capability gating, coexistence.
19. **E2E test suite**: counter, poll, coexistence, multi-peer
    convergence, capability gating.
20. **SDK ergonomics**: macros and tooling for app authors;
    cargo-component integration; manifest helpers.
21. **jco backend implementation**: generate JS+wasm shim; iroh-relay
    bridge for browser transport. Implements the trait from step 7.
22. **Browser-tier tests**: headless Firefox, multi-tab convergence,
    nested-CM-in-browser-WASM viability under realistic memory
    pressure.
23. **v1.1 (or v1 stretch goal)**: counter app's auto-reset behavior
    component (acceptance criterion #6).
24. **Dependency-direction CI check**: enforce `examples/` →
    `crates/sdk` only; kernel crates never depend on examples.

**Order rationale**: steps 3-7 establish the runtime ABI and the
backend abstraction *before* deep kernel work. Step 5 (manifest)
comes before step 6 (Wasmtime) so the linker is built with capability
gating from the start, not retrofitted in step 9 of the original
ordering. Step 17 (jco backend) implements the trait designed in
step 7.


