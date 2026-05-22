**Date:** 2026-05-22
**Status:** active
**Subject:** The decision file. What anonymity-transport prior art validates, what to avoid, what to borrow. Read this when designing Myrhiza's transport plug-in API or its topic-ID-rotation protocol.

# Lessons for Myrhiza — anonymity transports

The synthesis. **Validates / Avoid / Borrow.** Each entry cites the
evidence file. Read [`comparisons.md`](comparisons.md) for the
cross-cutting table and [`open-problems.md`](open-problems.md) for
what no transport solves.

## Validates

Choices Myrhiza is already making that this corpus supports.

1. **Netlayer pluggability is correct.** Tor, I2P, Veilid, Nym, and
   HOPR are each *transports*, not *applications*. They expose a
   "give me a stream to an opaque destination" interface that an
   application sits on top of. **Apps that want anonymity get it by
   routing over an anonymity transport; the app code is unchanged.**
   Myrhiza's plan for a custom-transport API (per
   [`prior-art/iroh/lessons.md:34`](../iroh/lessons.md)) is exactly
   this shape. *Source: every per-system file in this folder.*

2. **Capability mediation is the right abstraction even for
   anonymity transports.** Apps should not get raw access to the
   Tor SOCKS port, the Veilid `RoutingContext`, or the Nym client.
   The kernel should grant *capabilities* to use the transport — at
   minimum gating which destination addresses an app may dial, what
   bandwidth it may consume, and what cover-traffic posture it
   advertises. *Source: [`comparisons.md`](comparisons.md) license
   table — embedding requires process isolation anyway.*

3. **Multiple-transport-per-app is operationally sane.** No single
   anonymity transport solves all threat models. The trade-off
   space is fundamental (latency vs anonymity vs anonymity-set-size
   vs UX). Apps should be able to pick — and the kernel should be
   able to *fall back* — between transports. *Source:
   [`comparisons.md`](comparisons.md) latency-vs-anonymity diagram.*

4. **Topic-ID rotation via blinded keys is the right primitive.**
   Tor v3 hidden services rotate descriptors daily by blinding the
   service's ed25519 pubkey against the current 24-hour time
   period + a shared random nonce. The same construction —
   `blinded-id = H(root-key, period, srv)` with ed25519 scalar
   multiplication — is what [`willow/open-problems.md`](../willow/open-problems.md)
   needs. **Just hashing `(topic, period)` is insufficient**;
   blinded-key crypto is the smallest primitive that delivers
   unlinkability across rotations. *Source:
   [`tor.md`](tor.md) §"Hidden-service descriptor rotation".*

5. **Cover traffic at the kernel layer is a real engineering choice
   to make.** If Myrhiza ever wants pattern privacy beyond IP
   privacy, the kernel needs to generate low-rate background
   traffic. Loopix's analysis gives the cost (tens of kbps) and
   benefit (GPA resistance) quantitatively. *Source:
   [`mixnets.md`](mixnets.md) §"Loopix" + [`open-problems.md`](open-problems.md) §4.*

6. **Foundation-stewarded, donation-funded is a viable governance
   model.** Tor (501(c)(3)), Veilid Foundation, and the volunteer
   I2P community have all sustained anonymity infrastructure for
   3-23 years without a token or VC narrative. Myrhiza inheriting
   this posture is consistent with the dependency network.
   *Source: [`comparisons.md`](comparisons.md) stewardship column.*

## Avoid

Pitfalls these projects reveal, and how Myrhiza compensates.

| Pitfall | Source | Mitigation |
|---|---|---|
| **"Anonymous" claims without naming the layer.** Tor hides IP from destination; sealed-sender hides identity from server; E2E hides content from intermediaries. These are three different problems. Apps that say "anonymous" without specifying which layer mislead users. | [`open-problems.md`](open-problems.md) §11 | Myrhiza specs must explicitly name **which adversary** is defeated by each transport choice. A Myrhiza app saying "uses Tor" should also say "leaks sender pseudo-identity to destination" (the part Tor *doesn't* hide). |
| **Tying anonymity claims to a small anonymity set.** Veilid's anonymity set is ~thousands of users. Nym's is ~100K. Tor's is ~2-4M. Cryptographically the protocols are sound; in practice, a 1K-user anonymity set is trivially traversable for a targeted adversary. | [`open-problems.md`](open-problems.md) §3 | Myrhiza apps using small-network transports (Veilid, HOPR) should document the anonymity-set-size assumption explicitly. Treat Tor as the default first-choice and small-network options as supplements. |
| **Latency-cost-of-anonymity hidden from app developers.** Mixnet transports add **seconds** of end-to-end delay; that is unusable for interactive UI. Yet "decentralized VPN" marketing of Nym/HOPR can mislead app developers into thinking they can run real-time apps over a mixnet. | [`mixnets.md`](mixnets.md), [`comparisons.md`](comparisons.md) latency table | The Myrhiza transport-plug-in API must expose **expected latency / throughput** as machine-readable metadata. Apps query the transport's profile before choosing it; the kernel can refuse to bind apps whose latency budget exceeds the transport's. |
| **Token-funded anonymity infrastructure.** Nym ($30M CoinList, NYM token) and HOPR (HOPR token, DAO) introduce investor pressure that distorts privacy claims. Marketing emphasizes the token and the 2-hop "fast" mode; the actual Loopix-anonymity-providing 5-hop mode is downplayed. | [`mixnets.md`](mixnets.md) §"Nym" + §"HOPR" | Myrhiza should not adopt a token. If Myrhiza apps integrate Nym or HOPR transports, the spec should call out the token-narrative bias explicitly so users can evaluate the claim independently. |
| **Single-implementation protocols where the team can disappear.** Veilid is one team; Cult of the Dead Cow has been around forever but the active Veilid contributors are ~5 people. arti is the same team as C-tor — improvement, but still one organization. **Tor + arti are effectively one stewardship.** | [`veilid.md`](veilid.md), [`tor.md`](tor.md) | For a load-bearing Myrhiza transport, prefer transports with **specs distinct from implementations**. I2P's two-implementation reality (Java + i2pd, 13 years stable) is the counter-example. Track when arti reaches full feature parity with C-tor — that diversifies the Tor ecosystem. |
| **Mobile UX is bad everywhere.** Tor Browser on Android works but battery-drains. Veilid has no first-party iOS app. I2P is desktop-first. Nym/HOPR mobile apps are "slower than other VPNs." A Myrhiza app committed to mobile-default cannot inherit any transport's mobile UX as-is. | [`open-problems.md`](open-problems.md) §6 | Treat "Myrhiza app over anonymity transport on mobile" as a power-user opt-in, never a default. Spec authors should publish per-transport mobile-UX caveats. Battery cost, install path (F-Droid vs Play Store), background-network restrictions on iOS — name them. |
| **Bootstrap is slow; no transport solves <1-second cold-start.** Tor: 5-30s. Veilid: ~seconds. I2P: 30s-2min. Nym: ~seconds. If Myrhiza apps are launched on demand (not run as background daemons), the first action a user takes will hit transport bootstrap latency. | [`open-problems.md`](open-problems.md) §7 | The Myrhiza kernel must decide whether the transport plug-in runs as a long-lived daemon (memory + battery cost) or per-app-launch (latency cost). This is a kernel-architecture decision; document the trade explicitly. |
| **Global passive adversary is *only* defeated by mixnets, and only by their slow paths.** Tor, Veilid, and I2P do not resist a GPA. Nym 5-hop and HOPR do (with stated caveats). Apps wanting GPA resistance must accept seconds of latency, period. | [`open-problems.md`](open-problems.md) §2 | Spec authors picking a transport plug-in must name the adversary explicitly. "Defeats a local-ISP observer" (Tor) vs "defeats a GPA" (Nym 5-hop) are different products. |
| **Veilid's Private-Route-deanonymizes-Safety-Route issue (#395) is open.** A documented timing-correlation attack between Veilid's two anonymity primitives has been an open issue since the early releases. | [`veilid.md`](veilid.md) | Don't bet Myrhiza production on Veilid as the *sole* transport for high-threat-model apps until #395 closes. Use Veilid for the stylistic-fit P2P story; pair with Tor or Nym for high-stakes paths. |
| **arti is pre-2.x server-side; the C-tor stewardship is the dependency.** arti 2.0.0 supports client-side onion services but **cannot yet run as a relay or directory authority**. Until that ships, the entire Tor network is C-tor; arti is just an embedding client. | [`tor.md`](tor.md) | When Myrhiza integrates arti as the iroh-on-Tor path, pin arti 2.x and track relay/authority support as a milestone. Document that Myrhiza-on-Tor users contribute to Tor's anonymity set but the underlying network is still C-tor. |
| **No published cross-transport benchmark suite.** Latency / throughput / cold-start numbers for Tor vs Veilid vs Nym vs HOPR are gathered from project marketing and academic papers, not from a unified workload run by an independent third party. | [`open-problems.md`](open-problems.md) §9 | When Myrhiza picks transports per app, publish a Myrhiza-specific benchmark suite. Operators choosing transports need empirical data, not project narratives. |

## Borrow

Concrete subsystems we'll either depend on directly or mirror in Myrhiza's design.

1. **Tor v3 descriptor-rotation construction for topic-ID rotation.**
   The blinded-pubkey + time-period + shared-random-nonce
   construction is the smallest primitive that delivers
   unlinkability across rotations. Lift the cryptographic shape into
   Myrhiza's topic-rotation spec; choose period (24h? per-hour?) and
   nonce source (Drand? authority set? kernel?) deliberately. *See
   [`tor.md`](tor.md) §"Hidden-service descriptor rotation".*

2. **Veilid's Safety Route + Private Route split for sender/receiver
   privacy.** The decoupling of "sender hides IP" from "receiver
   hides identity" gives apps fine-grained control. The Myrhiza
   capability surface can map this directly: `cap-route-send-anon`
   (sender privacy), `cap-listen-anon` (receiver privacy), opt-in
   per app. *See [`veilid.md`](veilid.md) §"Safety Routes + Private
   Routes".*

3. **arti as the iroh-on-Tor embedding.** MIT/Apache-2.0 licensed,
   Rust-native, embeddable. Plan to ship `arti-client` as an
   optional Myrhiza dependency; apps that opt into the Tor transport
   get it without operators installing system-Tor. *See
   [`tor.md`](tor.md) §"arti — the Rust rewrite".*

4. **I2P's garlic-message bundling for kernel-layer packing.** Even
   outside a full anonymity routing, packing multiple in-flight
   messages from different apps into single transport-level
   envelopes complicates traffic analysis. Cheaper than cover
   traffic. *See [`i2p.md`](i2p.md) §"Garlic routing as a primitive".*

5. **Loopix's anonymity-evaluation methodology.** Stated adversary +
   stated assumptions + quantified anonymity bounds. Even if Myrhiza
   never adopts Loopix-style mixing, the *discipline of stating
   anonymity claims with epsilon bounds* is borrowable. Replaces
   vague "we use encryption" with rigorous "we defeat X under
   assumptions Y with anonymity-set-size N." *See
   [`mixnets.md`](mixnets.md) §"Loopix".*

6. **I2P's two-implementation discipline.** Java I2P + i2pd have
   coexisted for 13+ years against the same wire spec. The
   discipline (clear spec, semantic versioning of the wire
   protocol, multiple-implementation interop testing) is what made
   it possible. Worth mimicking for Myrhiza's wire formats —
   especially for capabilities that cross the runtime boundary.
   *See [`i2p.md`](i2p.md) §"Java I2P vs i2pd".*

7. **Foundation-stewardship as a counter-example to token funding.**
   Tor Project (501(c)(3)) and Veilid Foundation (501(c)(3))
   demonstrate that anonymity infrastructure can be donation-funded
   and outlast investor narratives. The Linux Foundation, OpenSSL
   Foundation, and Apache Software Foundation are the broader
   templates. Myrhiza's eventual governance entity should look like
   these, not like Nym Technologies AG. *See
   [`comparisons.md`](comparisons.md) governance table.*

8. **Per-app transport selection with kernel-mediated fallback.** A
   Myrhiza chat app picks Tor; a Myrhiza CRDT-sync app picks Nym;
   a Myrhiza video-call app stays on iroh-direct. The kernel
   advertises available transports + their cost profiles; apps pick
   declaratively; the kernel can fall back if (e.g.) Tor is blocked
   on the user's network. *See
   [`comparisons.md`](comparisons.md) §"What this means for Myrhiza's
   transport plug-in API".*

## The single most important lesson

Anonymity is a **layer of layers**:

1. **Transport-level anonymity** (Tor, Veilid, Nym): hides IP from
   destination and intermediate observers.
2. **Application-level identity unlinkability** (Signal's sealed
   sender, [`prior-art/signal/identity.md`](../signal/identity.md)):
   hides sender from broker.
3. **Content-level encryption** (E2E from MLS or X3DH): hides
   content from intermediaries.
4. **Pattern-level privacy** (cover traffic, garlic bundling): hides
   traffic-timing patterns from a strong observer.

**Myrhiza must compose all four to deliver meaningful anonymity.**
A "Tor-routed Myrhiza app" with no sealed-sender leaks the user's
pseudo-identity to the destination on every message. A "sealed-
sender Myrhiza app" without Tor leaks the user's IP. An E2E-encrypted
Myrhiza app without cover traffic leaks usage patterns to anyone
watching the encrypted traffic.

The transport plug-in solves layer (1) and partially (4). The
capability + identity model solves layer (2). MLS / X3DH solves
layer (3). **The composition is the product. The transport is just
one ingredient.**

## Cross-references

- [`README.md`](README.md) — folder overview + reading order
- [`tor.md`](tor.md), [`veilid.md`](veilid.md), [`i2p.md`](i2p.md), [`mixnets.md`](mixnets.md) — per-system evidence files
- [`comparisons.md`](comparisons.md) — cross-cutting table
- [`open-problems.md`](open-problems.md) — what's structurally unsolved
- [`prior-art/iroh/lessons.md`](../iroh/lessons.md) — netlayer-pluggable framing
- [`prior-art/signal/identity.md`](../signal/identity.md) — sealed-sender layer
- [`prior-art/willow/open-problems.md`](../willow/open-problems.md) —
  topic-ID rotation Tor-analogue problem
- [`prior-art/spritely-ocapn/lessons.md`](../spritely-ocapn/lessons.md) —
  netlayer abstraction parallel

## Sources

All sources live in the per-system files. This file is synthesis,
not primary evidence.
