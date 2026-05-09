**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — open problems Myrhiza will inherit by depending on it

# Open problems Iroh doesn't solve

Problems iroh structurally does not solve, that Myrhiza will inherit by depending on it. The team has been honest that iroh is *a transport library for direct connections*, not a P2P platform — these gaps are by design, not oversight. They are still our problems.

## 1. Discovery — how do strangers' NodeIDs find each other in the first place?

Iroh solves *address resolution*: given a NodeID, find the (possibly-NATted) IP/port to dial. The mechanism is [pkarr-on-Mainline-DHT + n0-operated DNS](https://www.iroh.computer/blog/iroh-dns).

Iroh **does not** solve *discovery* in the social sense: how do two strangers — who have never met — find each other's NodeIDs? Verbatim from an HN comment ([44706595](https://news.ycombinator.com/item?id=44706595)):

> *"In the iroh world, you dial another node by its NodeId, a 32-byte ed25519 public key…"*

NodeID exchange happens out-of-band: QR code, share link, copy-paste, a ticket pasted into chat, an email. This is the same shape as Magic Wormhole's PAKE codes, OCapN's sturdyrefs, libp2p's multiaddrs, Hypercore's discovery keys. **Every P2P system has this problem; iroh does not pretend to solve it.**

What this leaves to Myrhiza:

- **Public-service discovery** (a Mastodon-like search index of public Myrhiza apps).
- **Topical pub/sub discovery** ("anyone hosting a chess game right now?").
- **Friend-of-a-friend introductions** (cap-style three-party introductions).

Decision Myrhiza must make explicitly: do we ship a discovery primitive (DHT, gossip overlay, federated index, tag registry) or do we explicitly delegate? Don't ship "we'll figure it out." Even Holochain ships *bootstrap servers* — a discovery primitive — even though the eventual story is DHT-only. Plan for an analogous bootstrap layer.

## 2. Identity portability — NodeID rotation, multi-device, recovery

A NodeID is a single ed25519 public key. The corresponding private key lives on one device. **Lose the device, lose the identity.** This is the same constraint as a Mastodon account on a single server, or a self-hosted Matrix homeserver, or any keypair-shaped identity.

Iroh has explored solutions but not shipped them:

- [**Lose your device, but keep your keys (Oct 2024)**](https://www.iroh.computer/blog/frost-threshold-signatures) — FROST threshold signatures research. A NodeID can be split across N devices with a t-of-N signing threshold. **Status: research, not in 1.0.0-rc.0.**
- **Ephemeral NodeIDs** — a node can generate fresh NodeIDs per session, but every counterparty has to know the new key, defeating any reputation/cap-token system that relies on NodeID stability.

What this leaves to Myrhiza:

- A real story for *identity = stable across devices* that doesn't reduce to "NodeID = identity."
- A story for *identity recovery* when the user loses all their devices and has only a recovery phrase / cloud backup.
- A story for *device authorization* — adding a new device to an existing identity without rotating the identity.

Decision: separate NodeID (transport identity) from PrincipalID (application identity). The transport identity is a per-device key handled by iroh; the application identity is a Myrhiza-layer construct that survives device loss. **NodeID = identity is a category error** — the iroh team explicitly does not promise this.

## 3. Sybil resistance — none, by design

Anyone can spin up arbitrarily many NodeIDs on arbitrarily many machines. Generating a fresh ed25519 keypair is cheap; relays do not gate creation. There is no proof-of-personhood, no proof-of-stake, no resource expenditure, no membership gate at the iroh layer.

This is **the right tradeoff for a transport library** — Sybil is a policy concern, and a transport that bakes in a Sybil mechanism (e.g. Filecoin-style staking) becomes useless for everything that doesn't need that mechanism.

What this leaves to Myrhiza: every Myrhiza-layer construct that depends on "this NodeID is a unique person/principal" must enforce that itself. The same `Sybil Attack Vulnerability Trilemma` (Platt et al. 2024, cited in the Holochain critiques file) applies: Myrhiza picks a corner. Realistic options:

- **Permissioned-per-app membership proofs** (Holochain shape): each Myrhiza app declares its membership policy (invite-only, capability-token-gated, social-graph-attested, fee-paying, …); iroh has no opinion.
- **External proof-of-personhood** (BrightID, Worldcoin, Zupass, government ID): plug in at the kernel layer.
- **Web-of-trust petname graph** (OCapN / Spritely shape): authority follows from chains of introductions.

Don't pretend Myrhiza has a global Sybil floor; it does not, because iroh does not.

## 4. Relay-server economics — who pays for relay infrastructure long-term?

Iroh today operates **four public relay servers** funded by Number 0 ([FAQ](https://docs.iroh.computer/about/faq)). The commercial model exists ([Iroh Services, Oct 2025](https://www.iroh.computer/blog/iroh-0-93-iroh-online)), but as of May 2026 the per-app economics are:

- **Free public relays** are rate-limited and intended for development.
- **Production deployments** are expected to either pay for dedicated relays via Iroh Services, or self-host.

For Myrhiza, "every Myrhiza app uses n0's free relays" is **not a sustainable production posture**. The realistic options:

- **Myrhiza operates its own relay fleet.** Operational cost: Rust binary on a small VM in 3-4 regions, monitoring, DDoS protection, on-call. Estimate is "a few hundred dollars/month per region" at modest scale. Real money for a side project, irrelevant for a funded org.
- **Each Myrhiza app operator runs their own relays.** Pushed-to-edge model. Higher per-app cost, no central choke point.
- **Relay-as-a-service from a third party** (Iroh Services, Cloudflare, an eventual relay marketplace). Operationally cheapest, depends on a third party staying solvent.

The honest version: someone, somewhere, runs servers, *or* peers must always have public IPs (no NAT traversal). **There is no "purely peer-to-peer" iroh deployment in 2026 for the realistic NAT-laden internet.** The best Myrhiza can do is decentralize *which* relays an app uses (interoperable, swappable, per-app-configurable) so that no single operator becomes a chokepoint.

## 5. Censorship resistance — relay servers as a chokepoint

Even though iroh's relay traffic is end-to-end encrypted, the relay sees *which NodeID is talking to which NodeID, when, and how much*. Verbatim from the team ([HN comment](https://news.ycombinator.com/item?id=44706595)):

> *"relays do have a list of nodeIDs and list of connections they're facilitating, which is privileged information."*

Implications:

- **Subpoena risk.** A US-based relay operator served with a subpoena reveals the NodeID-to-NodeID social graph, even though they cannot reveal content.
- **State-level blocking.** A regime that blocks the n0 default relays' IPs/TLS endpoints by SNI breaks NAT-fronted connectivity for any iroh app in that region. Custom transports (Tor support landed in 0.97) are a real mitigation but require user opt-in.
- **Operator coercion.** A relay operator under legal or financial pressure can introduce timing-correlation attacks, log retention, or selective drop without breaking the e2ee guarantee. The "you can't see content" guarantee is much weaker than the "I can't see *anything* about who's talking" guarantee that onion routing provides.

What this leaves to Myrhiza:

- For threat models that include state-level adversaries, iroh-on-Tor (or iroh-on-Veilid) is the right deployment, not iroh-on-default-relays.
- For threat models that include hostile relay operators, *content* is safe but *metadata* is not. Cap-mediated authority should not leak through metadata patterns (e.g. "alice talks to bob every 10 seconds = alice is bob's secretary"). Padding/cover-traffic is a Myrhiza-layer concern iroh does not address.
- The default deployment is *not* censorship-resistant. Don't market it as such.

## 6. Durability — content addressing without persistence guarantees

`iroh-blobs` provides BLAKE3-verified streaming for content-addressed data ([blog](https://www.iroh.computer/blog/iroh-blobs-0-90-new-features)). What this provides: integrity (the bytes you got hash to the hash you asked for) and resumable transfer (BLAKE3's verified-streaming property).

What this **does not** provide:

- **Replication** — `iroh-blobs` does not replicate data across peers automatically; if the only node holding a blob goes offline, the blob is gone until that node returns.
- **Pinning quorum** — there is no notion of "this blob must be held by ≥3 peers." Apps must enforce it.
- **Erasure coding** — no Tahoe-LAFS-style redundant encoding; a blob is held in full or not at all.
- **Garbage collection of forgotten data** — bring-your-own-policy.

Iroh's stance: durability is application-policy, not transport-policy. Correct division of concerns. For Myrhiza this means: the Myrhiza kernel must specify *who pins what* and *how Myrhiza apps express durability requirements*. Holochain's per-app DHT-shard validators are one model; explicit pinning quorums (per-blob, per-app) are another. Don't conflate "iroh-blobs is content-addressed" with "iroh-blobs is durable storage."

## 7. Mutability and consensus — explicitly delegated

The [**Consensus is Impossible**](https://www.iroh.computer/blog/consensus-is-impossible) post is the team's clearest statement. Verbatim:

> *"All iroh protocols run up against these laws of distributed systems physics. Some examples: strictly speaking, iroh docs isn't a consensus protocol, it's a 'sync' protocol."*

Iroh-docs is **eventually consistent** — all nodes will eventually have the same data, but there is no protocol-level "agreed at time T" moment. For Myrhiza's `state-apply` profile, this is the load-bearing fact: cross-peer convergence is asymptotic, not synchronous. State-apply must be a deterministic function of `(prior state, event)` *because* there is no consensus to lean on.

What this leaves to Myrhiza: explicit semantics for "what does Myrhiza promise about cross-peer convergence?" Realistic options:

- **Per-app authority sets** (Holochain shape): writers of an event set are deterministically identifiable; convergence is per-app DHT-shard.
- **CRDT-only** (iroh-docs shape): no authority, eventual merge is mathematically guaranteed.
- **External consensus when needed** (Holochain's "agent activity" gossip, or a chain anchor for ordering): bolt-on, per-app.

Document the choice. Don't promise consensus iroh doesn't provide.

## 8. Performance benchmarks vs alternatives — absent

The iroh team has not published a head-to-head benchmark suite (vs libp2p, Hypercore, Tailscale, gRPC). The closest claim is "200k concurrent connections, millions of devices" ([LambdaClass interview](https://blog.lambdaclass.com/the-wisdom-of-iroh/), Apr 2025), which is throughput at the network level, not latency / TPS / memory-per-connection / messages-per-second under contention.

For Myrhiza this is a real unknown until *we* benchmark. Things to measure before locking the spec:

- Connection setup time, NAT-fronted, relay-fronted, hairpin (~1-RTT, ~3-RTT, ~5-RTT respectively, expected).
- Memory per long-lived `Connection` (with multiple ALPN streams).
- Throughput per `Connection` and aggregate per `Endpoint`.
- Behavior under packet loss (QUIC's strength) and under relay-saturation.

The [QAD blog](https://www.iroh.computer/blog/qad) and the [BLAKE3 hashing post](https://www.iroh.computer/blog/hashing-multiple-blobs-with-BLAKE3) have iroh-internal benchmarks but not comparisons. Generate Myrhiza-specific numbers from MVP onward.

## 9. Wire-format spec — not yet published

The [1.0 roadmap](https://www.iroh.computer/blog/road-to-1-0) commits to *"publishing open standards specifications"* before 1.0 finalizes. As of May 2026, 1.0.0-rc.0 has shipped without a published wire spec. The de facto spec is "whatever the iroh source code does."

For Myrhiza this matters because:

- **Single-implementation risk.** A protocol with one implementation has much weaker survivability than a protocol with a published spec and multiple implementations. Quinn-vs-Microsoft's-msquic is the gold-standard model; iroh is not yet there.
- **Audit difficulty.** Security and correctness audits of a protocol are easier when there's a spec to audit against.
- **Re-implementation cost.** If Myrhiza ever needs an iroh-compatible implementation in a different language (C++ embedded, etc), absent a spec the only option is "read the Rust and reimplement."

Track when the wire spec ships. Re-evaluate this open problem when it does.

## Implications for Myrhiza

Direct mappings from the open problems above to Myrhiza spec decisions:

| Open problem | Myrhiza decision required |
|---|---|
| 1. Discovery (NodeID exchange) | Ship a discovery primitive *or* explicitly delegate. Bootstrap layer at minimum. |
| 2. Identity portability | Separate **NodeID** (transport) from **PrincipalID** (application). FROST or split-key recovery as research, not 1.0. |
| 3. Sybil | Per-app membership proofs / capability tokens. Document explicitly that Myrhiza has no global Sybil floor. |
| 4. Relay economics | Decide: Myrhiza-operated, app-operated, or third-party relays. Document operating cost in the ops spec. |
| 5. Censorship resistance | Default deployment is not censorship-resistant. Document. Tor / Veilid as opt-in transports. |
| 6. Durability | Per-app pinning policy. State the durability semantics in each app's spec. |
| 7. Mutability / consensus | Pick per-app: deterministic state-apply, CRDT, or external anchor. Don't promise "eventual" without bound. |
| 8. Performance | Bench against libp2p / gRPC / Hypercore from MVP. Publish results. |
| 9. Wire spec | Track iroh's wire-spec publication. Re-evaluate single-impl risk when it lands. |

None of these problems is unique to iroh — they are the universal hard problems of P2P substrate. They are also problems iroh *correctly* refuses to solve at its layer. Myrhiza's value-add is solving them at the layer above. Be modest about which we actually solve at MVP and which we defer; honesty about the boundary is the only protection against shipping a worse Holochain.

## Sources

- [iroh — A new direction for iroh (Feb 17, 2023)](https://www.iroh.computer/blog/a-new-direction-for-iroh)
- [iroh — Comparing iroh & libp2p (Jan 5, 2024)](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [iroh — Consensus is Impossible (Feb 21, 2025)](https://www.iroh.computer/blog/consensus-is-impossible)
- [iroh — Dial by NodeID, no address required](https://www.iroh.computer/blog/iroh-dns)
- [iroh — Lose your device, but keep your keys (Oct 2024)](https://www.iroh.computer/blog/frost-threshold-signatures)
- [iroh — Roadmap to 1.0 (Oct 28, 2024)](https://www.iroh.computer/blog/road-to-1-0)
- [iroh — iroh services (Oct 2025)](https://www.iroh.computer/blog/iroh-0-93-iroh-online)
- [iroh — Tor custom transport](https://www.iroh.computer/blog/tor-custom-transport)
- [iroh — QAD: STUN to QUIC Address Discovery](https://www.iroh.computer/blog/qad)
- [iroh-blobs — BLAKE3 verified streaming](https://www.iroh.computer/blog/iroh-blobs-0-90-new-features)
- [iroh — Hashing multiple blobs with BLAKE3](https://www.iroh.computer/blog/hashing-multiple-blobs-with-BLAKE3)
- [iroh FAQ](https://docs.iroh.computer/about/faq)
- [LambdaClass — The Wisdom of Iroh (Apr 9, 2025)](https://blog.lambdaclass.com/the-wisdom-of-iroh/)
- [HN 44706595 — NodeID semantics, relay metadata leak](https://news.ycombinator.com/item?id=44706595)
- [Holochain prior-art — Sybil trilemma critique](../holochain/critiques.md)
- [Spritely prior-art — open problems (discovery, sybil, recovery)](../spritely-ocapn/open-problems.md)
