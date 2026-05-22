**Date:** 2026-05-22
**Status:** active
**Subject:** What Sybil-resistance and distributed-maintenance literature does not solve — open problems Myrhiza inherits when writing participation-enforcement specs.

# Open problems — Sybil resistance + distributed maintenance

What no algorithm in this folder cleanly solves for a peer-to-peer runtime. Each entry: short problem statement + why it matters for Myrhiza + canonical sources.

## 1. Permission-graph compromise recovery

SybilGuard / SybilLimit assume sparse attack edges between the honest social graph and the Sybil region. When the attacker *compromises* a high-trust honest peer (not creates new Sybils — compromises an existing one), the assumption is violated. The honest peer's cap-grants now leak into the Sybil region.

**What's needed:** a recovery mechanism. Options: time-limited cap-grants (forces re-evaluation), kernel-observed anomalous behavior (auto-throttle on rate spikes), explicit revocation propagating through the graph (MLS-style epoch updates).

**Canonical sources:** [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md), [`prior-art/mls/`](../mls/) (PCS — post-compromise security).

## 2. Metric for "participation" is gameable

Self-reported metrics (CPU hours, bandwidth served, snapshots persisted) are trivially gameable. Even kernel-observed metrics can be inflated by colluding peer-pairs trading useless work. BitTorrent's choking algorithm works because the metric is *block bytes received*, which is hard to fake — but the metric for "maintained a snapshot of someone else's app" is not so concrete.

**What's needed:** define the participation metric narrowly enough that it can't be inflated by collusion. Options: only count work that the asking peer can verify (challenge-response), or accept that participation is approximate and use it as a soft signal not a hard gate.

**Canonical sources:** [`bittorrent.md`](bittorrent.md), [`prior-art/willow/open-problems.md`](../willow/open-problems.md) §"Distributed maintenance".

## 3. Bootstrap with no graph

Myrhiza's permission graph is its advantage — *once it exists*. The first user joining a fresh Myrhiza network has no graph. Sybil-defense based on graph topology can't help. Standard P2P bootstrapping (well-known seed peers, certificate authorities, PKI) is the answer everyone falls back to.

**What's needed:** a v1 bootstrap story. Probably: out-of-band invite (user shares a cap-token URL); kernel-bundled "well-known operator" list; or both. Whatever it is, document the trust model honestly.

**Canonical sources:** [`prior-art/holochain/`](../holochain/) (DHT bootstrap), [`prior-art/iroh/`](../iroh/) (relay discovery).

## 4. Whitewashing — new identity after low reputation

Reputation systems are vulnerable to whitewashing: a peer accumulates low reputation, abandons the identity, creates a new one, starts fresh. Pseudo-anonymous P2P systems make whitewashing free; Myrhiza's peer-keypair model means a new keypair is one `keygen` call away.

**What's needed:** raise the cost of new identities. Options: invite-only (only existing peers can vouch for new ones — Myrhiza's permission graph does this); proof-of-something (PoW for new identities, raising the cost); or accept whitewashing and just gate on the permission graph.

**Canonical sources:** [`taxonomy.md`](taxonomy.md), [`eigentrust.md`](eigentrust.md), [`prior-art/willow/open-problems.md`](../willow/open-problems.md).

## 5. Collusion attacks on reciprocity

Two peers can collude to inflate their reciprocity scores (exchange useless data in a loop, report large transfers to neighbors). BitTorrent's choking is somewhat collusion-resistant because the transferred data is real (it's the file the user wants); for arbitrary maintenance work, the data exchanged may not have ground-truth value.

**What's needed:** make the maintenance work verifiable end-to-end (kernel can check the work was useful) or shift reciprocity to a domain where collusion is harder (only count work where a *third party* benefits).

**Canonical sources:** [`bittorrent.md`](bittorrent.md), [`bar-gossip.md`](bar-gossip.md).

## 6. Free-riding measurement at scale

Adar & Huberman (2000) measured free-riding on Gnutella by sniffing the network. Myrhiza's network is not amenable to the same measurement (encrypted, peer-to-peer, no central observer). Operators won't know whether their participation enforcement is working without telemetry — but telemetry undermines the privacy model.

**What's needed:** in-protocol metrics that respect privacy. Aggregate counters reported to operators (with differential-privacy noise); explicit opt-in telemetry from users; periodic surveys.

**Canonical sources:** [`taxonomy.md`](taxonomy.md), [`prior-art/anonymity-transports/`](../anonymity-transports/) (privacy-preserving observation).

## 7. The "altruistic majority" assumption

BAR Gossip and SybilGuard assume an altruistic majority — most peers are honest. This holds for many real P2P networks but not all. Myrhiza network sized at 100 peers with 30 honest, 30 rational, 40 Byzantine is a different problem.

**What's needed:** spec authors should state the altruistic-majority assumption explicitly and document failure modes when it's violated.

**Canonical sources:** [`bar-gossip.md`](bar-gossip.md), [`sybilguard-sybillimit.md`](sybilguard-sybillimit.md).

## 8. Per-app vs cross-app participation

A peer might be a heavy maintainer in app A and a free-rider in app B. Is the Sybil-resistance / reciprocity tracking per-app, cross-app, or both? The literature mostly assumes a single network; multi-app composition is unsolved.

**What's needed:** Myrhiza-specific design. Options: per-app ledgers (clean isolation); cross-app reputation (peer who maintains many apps is more trusted overall); hybrid.

**Canonical sources:** [`prior-art/willow/open-problems.md`](../willow/open-problems.md) §"Cross-app authority composition".

## 9. Storage durability ≠ availability

Filecoin proofs of replication / spacetime prove the data exists *somewhere* at proof-time. They don't prove the data is *served* when needed. Availability is a separate problem, traditionally answered with SLAs (centralized) or per-connection challenge-response (P2P).

**What's needed:** decide whether Myrhiza needs strong durability proofs (probably not v1) or just availability (probably yes — challenge-response).

**Canonical sources:** [`prior-art/willow/open-problems.md`](../willow/open-problems.md), Filecoin PoSt papers.

## 10. Sybil + reciprocity + cap-grants — three trust models, one runtime

Sybil-resistance answers "is this identity real?" Reciprocity answers "does this identity contribute?" Cap-grants answer "is this identity authorized?" All three are needed; their interactions are unspecified.

**What's needed:** a unified trust-model spec that names each layer explicitly and defines the interactions. A peer who is Sybil-suspect, contributes nothing, but holds a valid cap-grant — what does Myrhiza do?

**Canonical sources:** [`prior-art/capability-tokens/`](../capability-tokens/), [`prior-art/spritely-ocapn/`](../spritely-ocapn/), [`prior-art/willow/open-problems.md`](../willow/open-problems.md).

## Cross-references

- [`README.md`](README.md), [`lessons.md`](lessons.md), [`taxonomy.md`](taxonomy.md)
- Per-paper evidence files
- [`prior-art/willow/open-problems.md`](../willow/open-problems.md), [`prior-art/holochain/`](../holochain/), [`prior-art/mls/`](../mls/), [`prior-art/capability-tokens/`](../capability-tokens/), [`prior-art/anonymity-transports/`](../anonymity-transports/)

## Sources

All sources in per-paper evidence files.
