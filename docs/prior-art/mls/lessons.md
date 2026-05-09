**Date:** 2026-05-09
**Status:** active
**Subject:** Lessons from MLS prior art for Myrhiza's group-shaped capabilities

# Lessons for Myrhiza

The decision-relevant synthesis. Other files are evidence; this file is what we take away.

Format: validates / avoid / borrow / open questions, then a recommendation matrix.

## Validates

MLS prior art **confirms** these Myrhiza design bets:

- **Asynchronous group key agreement is achievable.** RFC 9420 demonstrates that 2-to-thousands-of-member groups can establish and rotate shared keys without all members being simultaneously online. Myrhiza's "peers may be intermittently connected" model is compatible with group-shaped capabilities at scale.
- **CGKA is the right primitive for multi-party rotating-membership state.** O(log N) scaling on member updates (vs Signal Sender Keys' O(N)) means MLS-style group capabilities scale to Myrhiza-app room sizes that pairwise-key approaches don't.
- **Splitting the protocol from the deployment is correct.** RFC 9420 is silent on Authentication Service and Delivery Service. This separation lets MLS layer over different trust models (federated, centralized, P2P). Myrhiza inherits this property: kernel mediates I/O capabilities; MLS-on-Myrhiza picks a deployment shape per-app.
- **Forward secrecy + post-compromise security are now standard expectations.** RFC 9420 makes both load-bearing. If Myrhiza ships group caps without these properties, applications will have to roll their own — and almost certainly get them wrong.
- **TreeKEM scales gracefully across orders of magnitude.** The same protocol works for 2-member chats and 1000-member channels. Myrhiza's group-cap primitives shouldn't need separate small-group / large-group designs.
- **Open-source Rust implementations exist with healthy multi-org stewardship.** OpenMLS (Phoenix R&D + Cryspen + community), mls-rs (AWS + Wire) are both viable. Myrhiza isn't betting on a single-vendor primitive.

## Avoid

MLS prior art shows where the **easy mistakes** are:

- **Don't use ECDSA signatures for MLS in Myrhiza.** Cremers ETK 2025 proves MLS *fails FCGKA* with EUF-CMA-only signature schemes. ECDSA is EUF-CMA but not SUF-CMA. Use Ed25519 (which is SUF-CMA) or hybrid PQ schemes (ML-DSA). Cite [critiques.md](critiques.md) for the proof. This is a published-RFC-level finding worth reading before committing.
- **Don't expect MLS to handle federation across Authentication Services.** RFC 9420 explicitly excludes this. The IETF MIMI WG is designing it but the spec is still draft (May 2026). If Myrhiza apps need cross-Authentication-Service federation, expect to either wait for MIMI or design app-level coordination.
- **Don't expect MLS to handle identity binding.** MLS verifies that a leaf signed something with the claimed key, but does NOT verify that the key belongs to a specific human / device / org. Identity binding requires a separate Key Transparency layer (Apple Contact Key Verification, Keybase-style transparency log, or app-specific PKI). Myrhiza will need to design or adopt this.
- **Don't assume "production-shipping" means "battle-tested at billions of users."** Of the largest verified deployments: Webex shipped on draft MLS and is migrating to RFC 9420; Wire reached RFC 9420 GA only in April 2025; Discord DAVE shipped on RFC 9420 / MLS 1.0 (Sept 2024) but only for audio/video traffic, not chat. WhatsApp / Signal / iMessage do NOT use MLS. Treat MLS as "production-ready by IETF standards" but "still maturing in deployment."
- **Don't use OpenMLS's `rayon` parallelism in a `state-apply` component.** OpenMLS uses `rayon` internally for some operations (per [openmls.md](openmls.md)) — non-determinism risk. Myrhiza `state-apply` must be deterministic. Either disable rayon (verify there's a feature flag), use mls-rs (verify same), or route MLS operations *outside* `state-apply` (e.g. into a propose-time helper).
- **Don't expect a Component Model artifact off the shelf.** Neither OpenMLS nor mls-rs ships as a `.wasm` Component Model artifact. Myrhiza will need to author the WIT contract for `MlsGroup` operations and build the wrapping component itself.
- **Don't conflate "MLS encrypts messages" with "MLS protects against malicious group members."** A malicious group member can DoS the group (refuse to commit, propose flooding) and trivially read all encrypted traffic. MLS's threat model is network adversary + member compromise, not malicious admitted member.
- **Don't underestimate Welcome message size for large groups.** Adding a member to a 1000-member group means encrypting the group state + path secrets + ratchet tree to them. Bandwidth and storage matter.
- **Don't assume RCS UP 3.0 / Discord DAVE / Wire imply iMessage will adopt MLS.** Apple iMessage uses PQ3 (Apple's own protocol). Apple's MLS exposure is RCS only.

## Borrow

Specific patterns Myrhiza group-cap design should **steal**:

- **Trait-based crypto provider abstraction (OpenMLS pattern).** OpenMLS's `OpenMlsCrypto` + `StorageProvider` + `OpenMlsRand` traits decouple the protocol from the I/O. Myrhiza's WIT contract for MLS-on-Myrhiza should mirror this: MLS state-apply imports cap-mediated crypto + storage + RNG.
- **Sync-only API surface.** OpenMLS chose sync-only because `state-apply`-shaped components don't need async. Validates Myrhiza's `state-apply` purity. Don't pull in async runtimes for kernel-mediated cap operations.
- **Atomic Commit semantics.** A Commit applies a batch of Proposals atomically and advances the epoch. Either all Proposals apply or none. Myrhiza's `state-apply` should follow the same all-or-nothing pattern for batched events affecting the same epoch.
- **Application/handshake message split.** MLS distinguishes control-plane messages (Commits, Proposals, Welcomes) from data-plane (application messages). Myrhiza's group-cap design should similarly distinguish "membership operation" events from "state-mutation" events — they have different security and pre-check requirements.
- **Reinit for ciphersuite migration.** When MLS needs to migrate a group to a new ciphersuite, Reinit transitions to a fresh group with new group_id but preserves continuity. Myrhiza schema migration could borrow this shape: when a state-apply schema changes incompatibly, Reinit-style transition with continuity rather than catastrophic break.
- **External Commits for joining without Welcome.** Sometimes a peer needs to join a group without a Welcome from an existing member (e.g. P2P discovery). MLS's External Commit pattern is the design point.
- **MLS's KEM-based PathSecret encryption.** TreeKEM's structure (encrypt path-secrets up the tree to subsets of members) is the algorithmic primitive that gives O(log N) updates. If Myrhiza ever builds its own group key protocol (don't), this is the shape.

## Open questions

Myrhiza spec authors should address, with this corpus loaded:

- **Does Myrhiza's group-cap primitive use MLS, or is it custom?** Recommendation: use MLS. Custom group key protocols have a poor track record (Megolm has been broken; Sender Keys is proprietary). MLS is the IETF-vetted choice.
- **Which Rust implementation?** OpenMLS (MIT, Cryspen+Phoenix R&D stewardship, formal verification adjacent via libcrux) vs mls-rs (Apache/MIT, AWS+Wire stewardship, more crypto backends). See recommendation matrix below.
- **What's Myrhiza's Authentication Service?** RFC 9420 leaves this open. Options:
  - Pubkey-as-identity (peer-pubkey *is* the AS verdict — simplest, no out-of-band trust).
  - DID-based (W3C decentralized identifiers — adds spec complexity).
  - Application-defined (each app picks; kernel doesn't know).
- **What's Myrhiza's Delivery Service?** Same shape — kernel I/O caps deliver MLS messages; the routing semantics are app-defined.
- **How does MLS fit the `state-apply` purity requirement?** MLS state changes are deterministic functions of inputs; the protocol-level operations are pure. The non-determinism risks are in the implementation (`rayon` in OpenMLS, RNG calls). Need to design the WIT contract carefully so MLS-as-state-apply is actually pure.
- **WIT contract for MLS operations** — needs to be authored. None exists today. What's the right granularity? Per-Commit? Per-MLSMessage? Per-Proposal-batch?
- **Post-quantum migration story** — if Myrhiza adopts MLS today using non-PQ ciphersuites, the migration to PQ-hybrid (`pq-ciphersuites` draft, target Dec 2026) goes through Reinit. Plan for it.
- **Cross-Authentication-Service interop** — if Myrhiza apps need MIMI-style cross-app messaging, that spec is still draft. Treat it as a future concern.

## Recommendation matrix for Myrhiza

If Myrhiza decides to commit to MLS for group capabilities today:

| If you want… | Choose | Reason | Risk |
|---|---|---|---|
| **Most-used Rust impl with formal verification adjacency** | OpenMLS | Cryspen formal-verification work; Phoenix R&D RFC-author stewardship; MIT license; the IETF interop reference Rust impl | Sync-only API may not fit all use cases; `rayon` parallelism needs feature-flagging for `state-apply` determinism; no Component Model artifact |
| **Most-permissive license + most crypto backends + Wire production proven** | mls-rs (AWS) | Apache-2.0 OR MIT (more permissive than MIT-only); 5 crypto backends including CryptoKit; Wire's `core-crypto` is mls-rs-derived; production-shipped | Smaller ecosystem; less formal-verification coverage; AWS-driven stewardship |
| **Lowest-level integration / matching IETF reference behavior bit-for-bit** | mlspp (C++, Cisco) | Lead author's implementation; powers Webex production; the canonical interop benchmark | C++ in Rust-Myrhiza is awkward; BSD-2-Clause license; harder Component Model wrapping |
| **Maximum production-deployment evidence** | OpenMLS or mls-rs (both production) + Webex (mlspp) lessons | Both Rust libs have production deployments | None |
| **Best fit for Myrhiza `state-apply` Component Model** | OpenMLS with rayon disabled | Sync API + trait-based abstraction matches Myrhiza's I/O-via-cap model best | Component Model wrapping is still Myrhiza's job; verify rayon can be disabled cleanly |

## Recommended posture for the runtime spec

A defensible default:

1. **Don't design Myrhiza's own group key protocol.** Use MLS. Custom group crypto is hostile to security audit and almost always wrong.
2. **Adopt OpenMLS as the reference implementation.** MIT license fits Myrhiza's open-source posture; multi-org stewardship matches the "no single-vendor dependency" preference; Cryspen formal-verification adjacency is the strongest of the Rust options.
3. **Author a WIT contract for `MlsGroup` operations** as part of Myrhiza's Identity / crypto / trust capability set. This is a kernel-level capability; apps consume it.
4. **Use Ed25519 signatures by default** (SUF-CMA, not just EUF-CMA). Avoid ECDSA per Cremers ETK 2025.
5. **Plan the post-quantum migration path** via Reinit. Track `draft-ietf-mls-pq-ciphersuites` for the Dec 2026 milestone.
6. **Defer federation** to MIMI. Single-Authentication-Service per Myrhiza app is fine for v1; cross-app federation can come later.
7. **Identity binding is out-of-scope-for-MLS, in-scope-for-Myrhiza.** Either pubkey-as-identity (simple) or commit to a Key Transparency design (complex). Pick early.

## Sources

This file synthesizes from sibling files. Primary sources cited per sibling:

- [protocol.md](protocol.md), [crypto.md](crypto.md), [group-lifecycle.md](group-lifecycle.md) — RFC 9420 mechanics
- [openmls.md](openmls.md), [other-implementations.md](other-implementations.md) — implementation surface
- [production-users.md](production-users.md), [governance.md](governance.md) — adoption + stewardship
- [comparisons.md](comparisons.md), [open-problems.md](open-problems.md), [critiques.md](critiques.md) — alternative protocols + structural gaps + third-party voices
- Cremers ETK 2025 published-RFC-level FCGKA finding: cited in [critiques.md](critiques.md)
