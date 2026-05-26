**Date:** 2026-05-22
**Status:** active
**Subject:** What AT Protocol structurally doesn't solve — gaps Myrhiza will see when consulting it as prior art

# Open problems

What atproto doesn't address. Each entry: the problem, why it matters for Myrhiza, what (if any) deployed answer exists elsewhere.

This file is the consult-this-during-design counterpart to [lessons.md](lessons.md). Lessons are "atproto solved X; consider borrowing." This file is "atproto did **not** solve X; here are the canonical references for the gap."

## E2E messaging is third-party-bolt-on, not protocol-native

**The gap**: atproto provides identity and authenticity, not confidentiality. All atproto records are public by default. Direct messages on `bsky.app` are server-side-readable. **MLS adoption did not ship in atproto itself in 2024 or 2025.** The deployed answer (February 2026) is **Germ DM**, a third-party app that overlays MLS on top of atproto identity but stores messages in Germ's own infrastructure.

**Why it matters for Myrhiza**: Myrhiza's master spec PR #636 proposes E2E as a kernel-level concern via a `host.mls` capability. This is structurally different from atproto's "E2E is an app's problem" stance. Both approaches are defensible; the trade-offs are:

- atproto-style: keep the protocol simple; let apps choose their E2E story; allow multiple competing E2E approaches.
- Myrhiza-proposed: make E2E a first-class capability; one canonical MLS story; tighter integration with peer authority.

**Canonical references**: `prior-art/mls/` is the entry point. Germ DM's deployed integration is the closest deployed reference for "MLS layered on a non-MLS protocol's identity layer."

## DID registry is a single-operator service

**The gap**: `plc.directory` is operated by Bluesky PBC. ~99% of atproto users have `did:plc` identities resolvable only through this one service. If `plc.directory` goes offline, those identities become unresolvable. There is no fallback mechanism for the registry itself.

The atproto answer is **trust transparency**: the operation log is public, auditable, and exportable. Anyone could in principle stand up a mirror. In practice, no production mirror exists.

**Why it matters for Myrhiza**: Myrhiza is peer-symmetric and explicitly rejects single-operator infrastructure. The equivalent of "the rotation key recovery story" without a `plc.directory` requires either:

- **Replicated state-apply**: every peer maintains the identity operation log; recovery rules apply via Myrhiza's deterministic state-apply runtime.
- **Per-room operation logs**: identity recovery is room-scoped, validated by room participants.
- **MLS-group-state-as-identity**: the user's identity group is an MLS group of their devices; key rotation is an MLS commit; no global registry.

Each has design surface; none has been deployed.

**Canonical references**: `prior-art/holochain/` for the peer-symmetric topology question (Holochain has DHT-based agent identity but no multi-device story); `prior-art/mls/` for group-state-as-identity; PR #636 in the Willow repo for Myrhiza's current sketch.

## Sync at scale requires the Relay tier

**The gap**: atproto's sync model assumes a Relay aggregates many PDSes' firehoses and downstream consumers (AppViews) subscribe to the Relay. A Relay needs terabytes of storage and gigabit-class throughput. There is no design for **decentralized sync** — peer-to-peer content discovery without a central aggregator.

**Why it matters for Myrhiza**: Myrhiza needs every peer to be able to participate in the network without a centralized aggregator. Atproto's design tells you what *doesn't* work — pushing the entire firehose through one tier creates the bottleneck. The deployed alternatives that Myrhiza should consider:

- **DHT-based content discovery** (Holochain, IPFS): peers store content by hash and respond to requests routed via consistent hashing. Limitation: doesn't naturally support "stream me everyone's recent updates."
- **Gossip-based event propagation** (Willow, Hyperswarm-style): peers gossip events through topic-based channels. Limitation: every interested peer sees every event; bandwidth scales with network size.
- **Sync-protocol-style range queries** (Willow Sync Protocol, Iroh sync): peers exchange Merkle-summarized ranges of their event sets; bandwidth scales with diff, not total.
- **Selective subscription** (atproto's Relay tier, scaled down to individual peers): each peer subscribes to a curated set of upstream peers. Limitation: how do you pick the upstream peers?

Myrhiza's current design uses gossip-style propagation via iroh-gossip; the scaling characteristics are under active investigation.

**Canonical references**: `prior-art/iroh/` for sync-protocol design; `prior-art/holochain/` for DHT-based sharding; `prior-art/willow/networking.md` for the current Myrhiza approach.

## Lexicon evolution has no breaking-change story

**The gap**: Lexicon's `lexicon: 1` is a hard constant; the language doesn't version. Individual schemas evolve under strict additive rules (add optional fields, loosen constraints) and **cannot break**. When `app.bsky.feed.post` needs a fundamentally incompatible change, the answer is "publish a new NSID" (`app.bsky.feed.post2`) and forget the old data. There is no schema migration story.

**Why it matters for Myrhiza**: Myrhiza needs a snapshot-portability schema that survives across module versions, possibly across years. The atproto answer ("never break, just publish a new type") isn't workable if a snapshot is meant to outlive multiple module revisions. Alternative deployed approaches:

- **Protobuf-style field numbering**: every field has a stable number; renames are free; type changes are forbidden; deletions are tombstoned.
- **Avro-style schema registry**: a writer-schema is bundled with data; a reader-schema can be different; the registry tracks compatible pairs.
- **Migration functions as state-apply concern**: a snapshot includes a "migrate from version N to N+1" component that Myrhiza's state-apply runtime can invoke.

Myrhiza hasn't picked yet. The choice affects how `state-apply` can change over time without invalidating historical state.

**Canonical references**: Protobuf evolution rules; Avro schema-registry design; Lexicon's own evolution spec at <https://atproto.com/specs/lexicon>.

## Multi-writer / multi-device write coordination is sidestepped

**The gap**: atproto's PDS is the **single authoritative writer** for a user's repository. Multi-device writes are serialized through the PDS. There is no design for concurrent writes from two devices without server-side coordination.

**Why it matters for Myrhiza**: Myrhiza is peer-symmetric. Two devices owned by the same user might be offline and writing simultaneously; convergence requires CRDT-style or consensus-style merging. Atproto's answer is "don't do that; route everything through your PDS," which is structurally unavailable to Myrhiza.

Deployed alternatives:

- **Last-writer-wins with logical clocks**: simplest CRDT; data loss on conflict.
- **Operation-based CRDTs** (Automerge, Yjs, Loro): rich types with structural merge; bandwidth-heavy on conflict-prone data.
- **State-based CRDTs**: merge whole-state snapshots; simpler protocol, more bandwidth.
- **Per-event causal consistency** (Willow's approach): each event has a logical clock referencing predecessors; merge by causal order.

**Canonical references**: `prior-art/crdts/` for the full survey; `prior-art/willow/state-machine.md` for Myrhiza's current direction.

## Key-loss recovery without `plc.directory`

**The gap**: atproto's 72-hour recovery window depends entirely on `plc.directory` enforcing the priority-rotation rule. If you lose ALL your rotation keys (no offline backup, no recovery service), you have **no recovery path** — even on atproto, this is permanent identity loss.

**Why it matters for Myrhiza**: Myrhiza inherits this problem in stronger form because there's no central operator to provide a "recovery service" tier. Possible answers:

- **Social recovery**: trusted peers can attest to your identity-rotation request; quorum approval triggers recovery.
- **Hardware-token-only**: rotation key lives only in a hardware token; loss of the token is terminal but the token itself is reliable.
- **Identity-as-MLS-group**: your identity is a multi-device MLS group; losing one device doesn't lose identity; losing all devices is terminal.
- **Hybrid (social recovery + MLS)**: MLS group of devices for routine identity; social recovery as a deeper escape hatch.

None is deployed at scale. Atproto's punt ("don't lose your keys") is honest — it's *the* hard problem in identity, and nobody has a great answer.

**Canonical references**: Argent / Loopring social-recovery designs (Ethereum-ecosystem); SSSS (Shamir Secret Sharing Schemes) for backup-key splitting.

## Post-quantum migration story

**The gap**: atproto uses ECDSA-SHA256 throughout (secp256k1 and P-256). No post-quantum keys, no post-quantum signature scheme. The rotation-key curves in particular are baked into the PLC operation log — changing them is a forklift upgrade.

**Why it matters for Myrhiza**: Myrhiza is just as exposed and should plan ahead. Atproto's lesson is **keep the curve choice contained** — don't bake it into the on-disk operation log. The DID document's signing-key slot can accept any `did:key` curve, including future PQ ones; the rotation-key slot is restricted. Myrhiza should make its equivalent of "rotation key" curve-agnostic if possible.

**Canonical references**: MLS WG post-quantum work (`prior-art/mls/governance.md`); NIST PQC standards (ML-DSA, ML-KEM, SLH-DSA); the atproto cryptography spec at <https://atproto.com/specs/cryptography>.

## Resilience against Relay-tier centralization failure

**The gap**: April 2026's DDoS incidents (4 service interruption notices across 5 days) revealed that Bluesky's Relay tier is a single point of failure that can be DDoSed offline. The protocol has no design for **graceful degradation** when the Relay is unavailable — downstream AppViews including `bsky.app` lose live data; users see frozen timelines.

**Why it matters for Myrhiza**: this is the structural failure mode of any tiered federation. Atproto's design assumes the Relay is always up; reality is supplying counterexamples. The Myrhiza-equivalent question: what does Myrhiza do when whatever-aggregator-it-has fails? If the answer is "Myrhiza has no aggregator, every peer is the aggregator for its own subscriptions," the structural failure mode is avoided. But that's not free — see "Sync at scale" above.

**Canonical references**: Bluesky's April 2026 service interruption posts; general literature on tiered-federation failure modes.

## Sources

- atproto Lexicon evolution: <https://atproto.com/specs/lexicon>
- atproto cryptography: <https://atproto.com/specs/cryptography>
- April 2026 DDoS announcements: bsky.social/about/blog (April 15-20, 2026)
- "Glass Floor of Digital Sovereignty" (critical analysis): <https://blog.gelbphoenix.de/the-glass-floor-of-digital-sovereignty/>
- Plan B-2 design (Myrhiza): [`docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`](../../specs/2026-05-19-plan-b-2-persistent-identity-design.md)
- Willow open problems: [`prior-art/willow/open-problems.md`](../willow/open-problems.md)
- CRDT survey: [`prior-art/crdts/`](../crdts/)
- MLS PQ work: [`prior-art/mls/governance.md`](../mls/governance.md)
