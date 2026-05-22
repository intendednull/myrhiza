**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — Gossipsub mesh-based pub/sub (v1.0 / v1.1 / v1.2)

# Gossipsub

Gossipsub is libp2p's baseline pub/sub protocol — a mesh-based epidemic broadcast with peer scoring. It is the production messaging layer of **Ethereum's consensus layer** (the largest libp2p deployment by node count and at-stake value), **Filecoin** (block + message propagation), and the IPFS / IPNS ecosystem. As of 2026-05-22 the production version everywhere is **v1.1**; v1.2's `IDONTWANT` extension is still a Working Draft.

This is **the most load-bearing file in this folder for Myrhiza** because (a) Myrhiza adopts epidemic gossip via `iroh-gossip` in plan B-4.1, (b) `iroh-gossip` implements HyParView + Plumtree rather than gossipsub, and (c) Myrhiza needs to know which algorithm is which, which attacks are mitigated where, and which design choices we inherited vs avoided. Gossipsub is the most-attacked, most-studied production gossip protocol in the P2P literature — its design choices are the load-bearing reference for any team building gossip-anything.

## What gossipsub is

A mesh-based pub/sub overlay with these properties:

- **Topic mesh.** For each topic a peer is subscribed to, it maintains a *mesh* of `D` other peers (default `D=6`, target between `D_lo=4` and `D_hi=12`). Mesh peers exchange full payloads (*eager push*).
- **Gossip set.** Peers outside the mesh receive **only message IDs**, not payloads (*lazy push* via IHAVE messages). Recipients can pull payloads they missed via IWANT.
- **Heartbeat.** Every 1 second (default), each peer rebalances its mesh: GRAFT new peers if below `D_lo`, PRUNE peers if above `D_hi`, emit IHAVE for recent messages to `D_lazy=6` random non-mesh peers.
- **Peer scoring (v1.1).** Each peer locally scores its neighbours. Negative scores trigger PRUNE from mesh + ignore in graft. The score combines per-topic and global parameters.
- **Peer exchange (v1.1).** When pruning, send a few candidate peers so the prunee can keep its mesh size up. Bootstraps a swarm without external discovery.

Compared to flood-based pubsub (`floodsub`), gossipsub has bounded fan-out per peer (`D`-ish, not "all subscribers"), which keeps amplification factor low at large topic sizes. Compared to plain epidemic broadcast trees (Plumtree), gossipsub adds **peer scoring as a first-class defence** against Sybil and eclipse attacks — Plumtree assumes a benign overlay; gossipsub assumes an adversarial one.

## Versions

| Version | Status | Latest revision | What it adds |
|---|---|---|---|
| v1.0 | 3A Recommendation, Active | r2, 2020-03-12 | Baseline mesh + heartbeat + IHAVE/IWANT + GRAFT/PRUNE. Replaces floodsub. |
| **v1.1** | **2A Candidate Recommendation, Active** | **r8, 2021-12-14** | **Peer scoring, peer exchange (PX), explicit peering, PRUNE backoff, opportunistic grafting.** Production version everywhere. |
| v1.2 | 1A Working Draft, Active | r1, 2023-07-14 | `IDONTWANT` control message: tell mesh peers "I already received message X, don't forward" — reduces duplicate-payload amplification, especially for larger messages. |

The v1.1 protocol id is `/meshsub/1.1.0`; v1.2 advertises `/meshsub/1.2.0`. Backward-compatible negotiation: a peer that supports v1.2 also accepts v1.1 connections.

**v1.1 is the production version.** Every shipping deployment runs v1.1 (Ethereum consensus, Filecoin, IPFS). v1.2's `IDONTWANT` is implemented in some clients (e.g. Lodestar) but not yet universal; v1.1 will remain the interop floor through at least 2026.

## Authorship + paper

**Spec author:** [Dimitris Vyzovitis (`@vyzo`)](https://github.com/vyzo). v1.0 + v1.1 are both attributed to him directly. v1.2 is [`@Nashatyrev`](https://github.com/Nashatyrev) (Lodestar / ConsenSys) + [`@Menduist`](https://github.com/Menduist) (Nimbus / Status) with vyzo in the Interest Group.

**Paper:** Vyzovitis, Napora, McCormick, Dias, Psaras. ["GossipSub: Attack-Resilient Message Propagation in the Filecoin and ETH2.0 Networks"](https://arxiv.org/abs/2007.02754) (arXiv:2007.02754, 2020-07-06). The paper documents v1.1 design + attack-resistance testbed: 5,000+ AWS VMs simulating Sybil / eclipse / Cold Boot / Cover / Censor / Flash attacks. Title is precise — gossipsub was *designed against* Filecoin + ETH2 threat models; the design's resilience claims are testbed-verified, not theoretical.

## Algorithm details (v1.1)

### Mesh maintenance

Per-topic state at each peer:

```
mesh[topic]          := set of peers we have eager-push relationships with (target |mesh| ≈ D)
fanout[topic]        := set of peers we eager-push to when publishing without subscribing (TTL'd)
gossip[topic]        := IHAVE buffer (recent message IDs to gossip out)
mcache               := short window of full messages (3 heartbeats by default) for IWANT replies
seen                 := bloom-style "have I seen this message ID?" cache (120s default)
```

Each heartbeat (1s):

1. **Prune** peers with score < 0 from `mesh[topic]`.
2. **Graft** if `|mesh[topic]| < D_lo` — pick from "known topic peers minus current mesh," skewed by score.
3. **Prune** if `|mesh[topic]| > D_hi` — random eviction.
4. **Emit IHAVE** for recent message IDs to `D_lazy=6` random non-mesh subscribers.
5. **Opportunistic grafting:** every 60s, if the median mesh score is below opportunistic threshold, GRAFT 2 high-scoring non-mesh peers.

Defaults (per the spec): `D=6`, `D_lo=4`, `D_hi=12`, `D_lazy=6`, `heartbeat_interval=1s`, `fanout_ttl=60s`, `mcache_len=5`, `mcache_gossip=3`, `seen_ttl=120s`.

### Control messages

The wire protocol carries five control message types alongside data payloads:

- **GRAFT(topic)** — "add me to your `mesh[topic]`." Sent on heartbeat when grafting.
- **PRUNE(topic, [backoff], [peer_exchange])** — "remove me from your `mesh[topic]`." v1.1 adds optional `backoff` (don't GRAFT me back for N seconds) and `peer_exchange` (here are some other peers you could mesh with).
- **IHAVE(topic, message_ids[])** — "I have these message IDs in topic." Gossiped lazily to non-mesh peers.
- **IWANT(message_ids[])** — "send me the payloads for these IDs." Reply to IHAVE.
- **(v1.2) IDONTWANT(message_ids[])** — "I already have these, don't send payloads." Sent eagerly when a peer receives a new message.

Data payloads are forwarded as `RPC.publish` messages with topic + sender + payload + (optional) signature + seqno.

### Peer scoring (v1.1 — the load-bearing addition)

Each peer computes a local score for every other peer. The score combines:

- **Per-topic parameters (`P1..P4`):**
  - **`P1` — Time in mesh.** Positive weight; grows linearly while the peer is in mesh, capped.
  - **`P2` — First message deliveries.** Positive weight; how often this peer is *the first* to deliver a valid message.
  - **`P3` — Mesh message delivery rate.** *Negative* weight; if the peer is in mesh but its delivery rate falls below expected (parametric), it's penalised. Catches lazy mesh peers.
  - **`P3b` — Mesh failure penalty.** Decay-resistant. Sticky across reconnects so a peer can't reset its penalty by churning.
  - **`P4` — Invalid messages.** Heavy negative weight. Catches peers shipping malformed payloads.
- **Global parameters (`P5..P7`):**
  - **`P5` — Application-specific score.** Pluggable; the app injects ed25519-signed reputation here.
  - **`P6` — IP colocation factor.** Negative if many peers share the same IP — defends against Sybil floods from one network.
  - **`P7` — Behavioural penalty.** Catches spam: too many graft/prune cycles, too many IHAVE per heartbeat, etc.

The score is a weighted sum (app-tunable). Thresholds:

- **`0`** — peers below this are pruned from mesh; ignored in graft.
- **`gossipThreshold`** (negative) — below this, peer is not sent IHAVE.
- **`publishThreshold`** (more negative) — below this, payloads are not forwarded to the peer.
- **`graylistThreshold`** (most negative) — below this, the peer's RPCs are dropped entirely.
- **`acceptPXThreshold`** — peer must score above this for us to trust its peer-exchange suggestions.
- **`opportunisticGraftThreshold`** — opportunistic grafting only considers peers above this.

Scores are **persisted across disconnects** with a decay (3600s default). A peer can't escape a negative score by reconnecting.

### Peer exchange (v1.1)

When pruning, a peer optionally attaches a list of candidate peers (their `PeerId` + signed peer record). The prunee can use these as new mesh candidates without external discovery. Defends against eclipse attacks: if I'm being pruned by all my mesh, the prunees themselves can give me alternatives.

Combined with peer scoring's `acceptPXThreshold`, PX is gated by the prunee's trust in the pruner — a low-score peer can't poison your peer set by sending you a list of Sybil identities.

### Explicit peering (v1.1)

Operators can declare *explicit peers* — connections forced regardless of score, mesh size, or churn. Used for "the four hard-coded bootstrap nodes I trust" or "the validator pair I always need to be reachable from." Messages flow through explicit peers like any mesh peer, but they're outside the heartbeat's mesh-management logic.

## Implementations

The libp2p ecosystem ships gossipsub in five places:

| Implementation | Crate / module | Version (2026-05) | Notes |
|---|---|---|---|
| **rust-libp2p** | [`libp2p-gossipsub`](https://crates.io/crates/libp2p-gossipsub) | `0.49.4` on crates.io (2026-03-26), master at `0.50.0` | v1.1 + v1.2 IDONTWANT support |
| **go-libp2p** | [`github.com/libp2p/go-libp2p-pubsub`](https://github.com/libp2p/go-libp2p-pubsub) | tracking go-libp2p 0.48 | Reference implementation; Ethereum + Filecoin run this. **The canonical implementation** for interop. |
| **js-libp2p** | [`@chainsafe/libp2p-gossipsub`](https://www.npmjs.com/package/@chainsafe/libp2p-gossipsub) | `14.1.2` | Maintained by ChainSafe, not by libp2p/js-libp2p directly. **Apache-2.0** (vs js-libp2p's dual Apache-2.0/MIT — the gossipsub TS port is single-licensed). |
| **nim-libp2p** | bundled in `libp2p` nimble | `1.15.3` | Used by Nimbus + Codex |
| **jvm-libp2p** | [`io.libp2p.pubsub.gossip`](https://github.com/libp2p/jvm-libp2p) | tracking 1.x | Used by Teku (Eth2 client) |

Cross-implementation interop is tested via [`libp2p/test-plans`](https://github.com/libp2p/test-plans). v1.1 interop is solid; v1.2 IDONTWANT support is uneven.

## Comparison: gossipsub vs Plumtree (load-bearing for Myrhiza)

Myrhiza's iroh-gossip dependency uses HyParView + Plumtree (Leitão et al., 2007), not gossipsub. The algorithms are siblings in the epidemic-broadcast family but differ in design priorities.

| Axis | gossipsub v1.1 | HyParView + Plumtree |
|---|---|---|
| **Membership** | Topic-scoped; subscribed peers + DHT/discovery seeded | HyParView active view (~5) + passive view (~30); transitive introduction from bootstrap |
| **Mesh / tree** | Random mesh, `D≈6` per topic, rebalanced via heartbeat | Spanning tree built from HyParView overlay |
| **Eager push** | All mesh peers receive payload | Tree edges receive payload |
| **Lazy push** | IHAVE → IWANT pull for non-mesh peers | IHAVE messages along non-tree HyParView edges; pull missing payloads if tree edge fails |
| **Adversary model** | Adversarial — peer scoring, PX gating, IP colocation, behavioural penalties | Benign — assumes churn but not Sybil |
| **Topic scope** | First-class topic with explicit subscribe/unsubscribe | First-class topic (in iroh-gossip's case); membership per topic |
| **Production scale** | Ethereum (~1M+ active validator keys, ~10k+ full beacon nodes) | iroh-gossip sized for "a few thousand peers per topic" per its docs |
| **Spec** | [libp2p/specs/pubsub/gossipsub](https://github.com/libp2p/specs/tree/master/pubsub/gossipsub) | Academic papers (Leitão 2007) — no IETF/libp2p-style spec |
| **Implementations** | 5 (Go, Rust, JS, Nim, JVM) | 1 (iroh-gossip in Rust) |
| **Sybil resistance** | Built in (P6 IP colocation, P5 app score, PX gating) | None — application's job |

**The structural difference that matters for Myrhiza:** gossipsub treats the overlay as adversarial; Plumtree treats it as benign-with-churn. Myrhiza's threat model is closer to gossipsub's (apps run on user devices, Sybil is real), but we inherit Plumtree because iroh picked it. The implication for `state-apply` profile: **gossip messages must be treated as adversarial regardless of which algorithm carries them**. The app-layer signing + capability check we'd do over gossipsub is the same one we need over Plumtree. The transport doesn't give us authenticity for free either way; iroh-gossip's `delivered_from: EndpointId` is *last-hop neighbor*, not original publisher (per [`../../specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md`](../../specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md) §1).

## Known attacks + mitigations

The v1.1 paper documents these attacks against the testbed:

- **Sybil flood** — many cheap identities. Mitigated by `P6` IP colocation + `P3` mesh delivery rate + `P5` app score.
- **Eclipse attack** — surround a victim with attacker peers. Mitigated by PX (the victim learns alternatives from honest prunees) + opportunistic grafting (the victim grafts to non-mesh high-score peers periodically) + `acceptPXThreshold` (the victim doesn't trust PX from low-score peers).
- **Cold-boot Sybil** — overwhelm fresh nodes that don't yet have scores. Mitigated by explicit peering (operator pre-configures trusted bootstrap) + early-phase score weighting.
- **Cover attack** — fill the mesh with valid-but-useless traffic to push out honest peers. Mitigated by `P3` (delivery rate per mesh peer) — useless traffic *from* a peer doesn't count.
- **Censor attack** — refuse to forward a victim's messages. Mitigated by random mesh selection + heartbeat-driven rebalancing — the censoring peers are not the only path.
- **Flash attack** — spike traffic to overwhelm validation. Mitigated by `P7` behavioural penalty + per-peer rate-limiting outside gossipsub.

Real-world attacks since v1.1 deployment:

- **Filecoin 2021 message-flood attack** (pre-v1.1 fully deployed) — mitigated by v1.1 rollout + ChainSafe's score-tuning blog series.
- **Ethereum Goerli 2023 GraffitiWall** — not an attack on gossipsub itself but used gossipsub to propagate the spam. No core protocol changes resulted; spam filtering was app-layer.
- **No CVE-grade gossipsub-specific exploit since v1.1's 2021 deployment**, as of 2026-05-22. The score function appears resilient at production scale.

## Implications for Myrhiza

1. **The gossipsub paper is required reading for any Myrhiza spec touching pub/sub authority.** Read it before designing the app-layer signing + score layer over iroh-gossip. The Plumtree primitive doesn't give us authority; we have to build it.
2. **Peer scoring is the load-bearing primitive missing from iroh-gossip.** If Myrhiza ships at scale on iroh-gossip's Plumtree, we will need to build a score-like layer above it — or accept Plumtree's "benign overlay" assumption, which is wrong at internet scale. See [`open-problems.md`](open-problems.md) §"Plumtree-without-scoring".
3. **Explicit peering is a useful pattern.** Myrhiza apps with operator-trusted bootstrap (a Cosmonic-style hub, a community moderator, a DAO multisig) should expose an explicit-peering surface — kernel-mediated "always forward to these peers regardless of mesh dynamics."
4. **The v1.2 IDONTWANT mechanism is worth porting.** Amplification factor matters at large topic sizes; IDONTWANT cuts duplicate payloads. Iroh-gossip's Plumtree already has a related mechanism (lazy-push IHAVE → on-demand pull), so the algorithmic gap is smaller than it looks — but if Myrhiza apps ever push large payloads through gossip (a bad idea, but inevitable), IDONTWANT-shape duplicate suppression is the right tool.
5. **Don't gossip large payloads.** Gossipsub paper recommends payload size under 1 MB; the production Eth2 deployment uses ~1.5 MB attestation blobs and that is already at the edge of viability. Myrhiza should treat gossip as "small notifications + IDs," with blob fetch via direct iroh streams or iroh-blobs — same pattern as iroh-docs ([`../iroh/docs.md`](../iroh/docs.md)) and iroh-willow.
6. **Topic IDs are an authority-laundering hazard.** Anyone who knows the topic ID can publish into it. Treat topic membership as untrusted, verify all payload content against your own authority model. Same lesson as iroh-gossip per [`../iroh/gossip.md`](../iroh/gossip.md) §"Topic IDs are a flat namespace with no auth."

## Sources

- [GossipSub paper (arXiv:2007.02754, Vyzovitis et al., 2020-07-06)](https://arxiv.org/abs/2007.02754)
- [gossipsub-v1.0 spec (r2, 2020-03-12)](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.0.md)
- [gossipsub-v1.1 spec (r8, 2021-12-14) — peer scoring](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.1.md)
- [gossipsub-v1.2 spec (r1, 2023-07-14) — IDONTWANT](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.2.md)
- [libp2p/specs/pubsub/gossipsub README](https://github.com/libp2p/specs/tree/master/pubsub/gossipsub)
- [libp2p-gossipsub crate (rust-libp2p)](https://crates.io/crates/libp2p-gossipsub)
- [go-libp2p-pubsub](https://github.com/libp2p/go-libp2p-pubsub)
- [`@chainsafe/libp2p-gossipsub` (TS port)](https://www.npmjs.com/package/@chainsafe/libp2p-gossipsub)
- [Scalable PubSub with GossipSub — Dimitris Vyzovitis talk](https://docs.libp2p.io/concepts/pubsub/overview/)
- [iroh-gossip — Plumtree comparison (sibling doc)](../iroh/gossip.md)
- [Plumtree paper (Leitão et al., 2007)](https://asc.di.fct.unl.pt/~jleitao/pdf/srds07-leitao.pdf)
- [HyParView paper (Leitão et al., 2007)](https://asc.di.fct.unl.pt/~jleitao/pdf/dsn07-leitao.pdf)
- [ChainSafe — Gossipsub score tuning blog](https://blog.chainsafe.io/)
- [Myrhiza Plan B-4.1 — Iroh-gossip subscribe + publish](../../specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md)
