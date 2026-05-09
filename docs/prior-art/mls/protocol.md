**Date:** 2026-05-09
**Status:** active
**Subject:** RFC 9420 — design-level walkthrough of the Messaging Layer Security protocol

> Companion files: [`group-lifecycle.md`](./group-lifecycle.md) — KeyPackage / Welcome / Commit mechanics; [`crypto.md`](./crypto.md) — ciphersuites and TreeKEM; [`openmls.md`](./openmls.md) — Rust implementation deep-dive; [`comparisons.md`](./comparisons.md), [`production-users.md`](./production-users.md), [`lessons.md`](./lessons.md), [`glossary.md`](./glossary.md).

## 1. What MLS is

MLS (Messaging Layer Security) is the IETF-standardized **group key agreement** protocol. It lets two-to-thousands of parties establish and continuously rotate a shared group key that protects end-to-end-encrypted application traffic, without all members needing to be online simultaneously.

| Field | Value |
|---|---|
| Document | **RFC 9420** — *The Messaging Layer Security (MLS) Protocol* |
| Track | Standards Track / Proposed Standard |
| Published | July 2023 |
| Authors | R. Barnes (Cisco), B. Beurdouche (Inria & Mozilla), R. Robert (Phoenix R&D), J. Millican (Meta), E. Omara, K. Cohn-Gordon (Oxford) |
| Companion | **RFC 9750** — *The MLS Architecture* (deployment-side concerns) |
| WG | `mls` (Security Area) — active; PQ work targeting Dec 2026 |

The protocol document fixes the cryptographic core. The architecture document (RFC 9750) describes the surrounding deployment shape — Authentication Service, Delivery Service, federation expectations.

## 2. Design goals

Quoting the abstract verbatim:

> *"Establishing keys to provide [end-to-end] protections is challenging for group chat settings, in which more than two clients need to agree on a key but may not be online at the same time. In this document, we specify a key establishment protocol that provides efficient asynchronous group key establishment with forward secrecy (FS) and post-compromise security (PCS) for groups in size ranging from two to thousands."*

The four load-bearing goals:

1. **Asynchronous group establishment** — a member can be added while offline; they catch up on next connect via a `Welcome` message.
2. **Forward secrecy (FS)** — compromise of current key material does not let an adversary decrypt past traffic.
3. **Post-compromise security (PCS)** — after a compromised member rotates their leaf key (an `Update`), future traffic is again confidential to the attacker. This is the property Signal popularized in two-party form; MLS lifts it to groups.
4. **Scalability to thousands** — handshake cost is `O(log N)` rather than `O(N)` per member, via the ratchet tree (TreeKEM). See [`crypto.md`](./crypto.md).

## 3. High-level architecture

A group is represented as a **left-balanced binary tree** of public keys (the *ratchet tree*). Each member occupies one leaf, holding the private key for that leaf and a path of derivable secrets from that leaf to the root. The root secret of the tree is the *commit secret*; it feeds the key schedule that produces per-epoch encryption keys.

State at every member is **derived deterministically** from the same input sequence (proposals + commits, ordered by the Delivery Service). This is the property that lets MLS map cleanly onto an event-sourced / state-replicated runtime — every member computes the same group state from the same handshake transcript.

**Distinguish two layers:**

- *MLS protocol* (RFC 9420) — group key agreement, handshake messages (Proposal, Commit, Welcome), key schedule.
- *MLS application messages* — the encrypted data plane. RFC 9420 defines the framing (`MLSMessage`, `PrivateMessage`, `PublicMessage`); what the plaintext payload means is up to the application.

## 4. Key concepts

| Concept | Meaning |
|---|---|
| **Group** | The set of members sharing a ratchet tree and key schedule. Identified by `group_id`. |
| **Epoch** | A monotonically-increasing counter; every Commit advances it. Each epoch has its own derived key set. |
| **Member** | An identity holding one leaf in the ratchet tree. Identified by their `LeafNode` and `Credential`. |
| **KeyPackage** | A pre-published bundle (init key, leaf node, lifetime, capabilities, signature) that lets others add this member to a group. |
| **Welcome** | The encrypted bundle that lets a freshly-added member catch up to current group state. |
| **Commit** | A handshake message that applies a batch of proposals atomically and advances the epoch. |
| **Proposal** | A pending change (Add / Remove / Update / GroupContextExtensions / PSK / ReInit / ExternalInit). |
| **Application message** | An end-user payload encrypted under epoch-derived keys. |

## 5. Threat model

RFC 9420 §16 (Security Considerations) names the model:

**MLS protects against:**
- A network adversary observing all handshake and ciphertext traffic.
- A malicious or compromised **Delivery Service** — confidentiality holds even if the DS sees every byte.
- Compromise of a member's long-term keys, *eventually* — once the affected member runs an Update + Commit, future traffic is again confidential (PCS).
- Re-decryption of past traffic from a current key compromise (FS).

**MLS does NOT protect against:**
- A malicious in-group member injecting valid messages while they remain in the group (insider attacks). Removing them requires consensus about who has authority — out of scope for the RFC.
- DoS by a malicious member (e.g. proposing nonsense, refusing to commit).
- Metadata leakage from the DS — group membership, message timing, sender identity in some framings, group sizes are all visible to the DS.
- A compromised **Authentication Service** issuing fake credentials. The RFC explicitly assumes a trusted AS but a largely untrusted DS.

The AS-trusted / DS-untrusted asymmetry is the protocol's foundational deployment assumption.

## 6. Authentication Service vs Delivery Service

RFC 9750 (architecture) names two infrastructure roles, both **out of scope for RFC 9420 itself**:

- **Authentication Service (AS)** — validates that a credential genuinely represents a claimed identity. Concretely: a CA-style PKI, a federation directory, an OIDC provider, etc.
- **Delivery Service (DS)** — orders and routes handshake + application messages. Concretely: a centralized server, a federated message bus, or in principle a P2P overlay.

Trust split: the AS is **trusted**, the DS is **largely untrusted** (but expected to deliver).

This split is the major design implication. MLS leaves both the identity layer and the transport layer pluggable, but every deployment must answer "who is the AS, who is the DS, and what's the threat model around them?". Most production deployments today centralize both; the IETF MIMI WG is the venue trying to standardize cross-AS / cross-DS interop.

## 7. Federation status

RFC 9420 is **per-group**. There is no native cross-AS / cross-deployment federation in the core spec. Two members can only join the same MLS group if both can fetch each other's KeyPackages and both can route handshake messages — implicitly assuming a shared (or interoperating) AS and DS.

The **MIMI WG** (More Instant Messaging Interoperability) was chartered to fill this gap. Drivers include the EU's Digital Markets Act, which classifies large messengers as "gatekeepers" and pushes them toward interop. MIMI uses MLS as its E2EE substrate but layers identity, room policy, content format, and discovery on top. Charter milestones target 2025–2026 deliverables (IESG submission of room-policy and protocol drafts in 2025; user discovery in late 2025).

For Myrhiza: cross-peer / cross-runtime interop is exactly the question MIMI is addressing for messaging, and worth tracking as an existence proof of "MLS-as-substrate, identity-and-routing-as-layer".

## 8. Wire format

MLS uses a **TLS-style binary presentation language** (RFC 8446 inspired). Two extensions over plain TLS encoding:

- **Optional values** — a single presence-signaling octet, followed by the value if present.
- **Variable-size vector lengths** — RFC 9000 (QUIC) variable-length integer encoding (1, 2, 4, or 8 bytes; prefix bits indicate length).

In Rust this maps cleanly to the [`tls_codec`](https://crates.io/crates/tls_codec) crate, which OpenMLS uses for all wire-format types. Every structured field has a stable encoding; transcript hashes commit to the canonical wire bytes.

## 9. MLS application messages

The top-level wire envelope is `MLSMessage`. It carries a `ProtocolVersion` (currently `mls10`) and a `WireFormat` selector that picks one of:

- **PublicMessage** — signed, **not encrypted**. Used for handshake messages where confidentiality is unnecessary (e.g. external proposals from non-members, GroupInfo).
- **PrivateMessage** — signed and **encrypted** under epoch-derived keys. Used for application traffic and (typically) handshake messages from members.
- **Welcome** — onboarding bundle for a newly added member.
- **GroupInfo** — public group state used for external commits / reinit handoff.
- **KeyPackage** — pre-published add-target.

`PrivateMessage` includes `group_id`, `epoch`, `content_type`, an `authenticated_data` field (AAD — visible to DS, integrity-protected), and the encrypted payload + sender data.

The `authenticated_data` field is the documented hook for application-level tagging (e.g. message-type, room metadata) that the DS may need to read but should not be able to forge.

## 10. Implications for Myrhiza

Speculative — none of this is committed runtime design. Recorded so future spec authors don't re-derive it.

- **MLS is the established primitive** for "group of N participants with a shared encrypted state". If Myrhiza grows multi-party room-shaped capabilities — channels, group state-apply with shared encrypted state, multi-party CRDT rooms — MLS is the IETF-blessed answer for the key schedule.
- **AS / DS split maps roughly onto Myrhiza's identity vs delivery split.** "Who is the AS" becomes "what identity authority does the runtime assume"; "who is the DS" becomes a question for the P2P overlay. Both are sandboxable as capabilities — the kernel mediates, the app sees only KeyPackage opaque handles.
- **Determinism friendly.** The state-apply layer is `(prior state, event) → new state`. MLS handshake processing has the same shape: `(prior group state, ordered handshake messages) → new group state`. State-apply could in principle wrap the MLS group as deterministic state, with handshake messages as events.
- **Wire format is TLS-codec, not Component-Model-native.** Adopting MLS means accepting the TLS presentation language at the wire boundary — the WIT side would need a thin codec capability or a host-mediated MLS surface.
- **PQ migration story is real but not yet standardized.** Hybrid ML-KEM ciphersuites are in `draft-ietf-mls-pq-ciphersuites` (target 2026). If Myrhiza adopts MLS now, designing for ciphersuite agility (via the reinit transition mechanism) is the safe move.

## Sources

- RFC 9420 — *The Messaging Layer Security (MLS) Protocol*. <https://www.rfc-editor.org/rfc/rfc9420.html>
- RFC 9750 — *The MLS Architecture*. <https://www.rfc-editor.org/rfc/rfc9750.html>
- IETF MLS WG. <https://datatracker.ietf.org/wg/mls/about/>
- IETF MIMI WG. <https://datatracker.ietf.org/wg/mimi/about/>
- Phoenix R&D — *RFC 9420 aka MLS — An Overview*. <https://blog.phnx.im/rfc-9420-mls/>
- IANA MLS Registry. <https://www.iana.org/assignments/mls/mls.xhtml>
- `tls_codec` crate. <https://crates.io/crates/tls_codec>
