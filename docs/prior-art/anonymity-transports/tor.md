**Date:** 2026-05-22
**Status:** active
**Subject:** Tor — onion routing, v3 hidden services, descriptor rotation, and the arti Rust client. The closest production analogue to Myrhiza's "topic-ID rotation through dumb relays" problem.

# Tor — onion routing + hidden services + arti

The Tor network — about **7,000 volunteer-operated relays** worldwide
overlaying the public internet, run by **The Tor Project**, a
Massachusetts 501(c)(3) nonprofit founded 2006. The thing apps care
about: a SOCKS proxy that routes TCP through three randomly selected
relays so that **no single relay knows both source and destination**.

The thing *Myrhiza* cares about: **hidden services** (officially "onion
services" since around 2015), where a server publishes a `.onion`
address — a cryptographic identifier, **no DNS, no IP** — and clients
connect to it through rendezvous-point negotiation that hides both
endpoints from each other and from the network.

## Architecture in 6 lines

1. **Client builds a 3-hop circuit.** Picks a *guard* (entry), *middle*,
   and *exit* relay from the consensus directory. Sends a hop-by-hop
   key exchange (Tor's `ntor` v3 handshake — Curve25519-based).
2. **Layer-encrypts payload.** Adds three layers of AES-CTR; each
   relay peels one layer.
3. **Exit talks to clearnet.** The exit relay opens the TCP connection
   to the user's destination. Returns bytes back through the circuit.
4. **For hidden services:** instead of exit-to-clearnet, the client
   connects through a *rendezvous point* that the service has agreed
   to meet at. Client and service each build their own 3-hop circuit
   to the rendezvous; total circuit = 6 hops.
5. **Service publishes a descriptor** (encrypted blob containing
   introduction points the client should use to negotiate the
   rendezvous) to a small set of *hidden service directory* (HSDir)
   relays selected by a hash of the service's public key + current
   time period.
6. **Service identity = `ed25519` public key**. The `.onion` address
   is `base32(pubkey || checksum || version)`. v3 onions are **56
   characters**.

## v3 vs v2: what was deprecated

Onion services have two protocol versions in deployment history:

| Version | Key | Address | Status |
|---|---|---|---|
| **v2** | RSA-1024 | 16 chars | **Deprecated and disabled.** |
| **v3** | Ed25519 | 56 chars | Current default; introduced 2018 (Tor 0.3.2). |

The v2 deprecation timeline ([Tor blog 2020-07-02](https://blog.torproject.org/v2-deprecation-timeline/)):

- **2020-09-15** — Tor 0.4.4.x begins logging deprecation warnings.
- **2021-07-15** — Tor 0.4.6.x removes v2 from the codebase. New v2
  services can no longer be created.
- **2021-10-15** — All supported client stable versions disable v2
  entirely. v2 `.onion` addresses become unreachable.

Reasons cited: RSA-1024 too short; the 16-character address allowed
brute-force prefix mining (`facebookcorewwwi.onion`); v2's directory
protocol leaked the service's existence to any client probing the right
HSDir; v2 had no defense against guard-discovery attacks. v3 fixes
all four.

**Lesson for Myrhiza:** Tor deprecated a working protocol over
**~16 months**, with overlapping deployment of v2-supporting and
v2-disabled clients. The replacement (v3) had been available for
~3 years before final v2 disable. Plan analogous timelines if Myrhiza
ever rotates its own identifier or transport format.

## Hidden-service descriptor rotation — the bit Willow cares about

`prior-art/willow/open-problems.md:207-218` cites Tor's v3 descriptor
rotation as the **closest analogue** for Myrhiza's
"rotate-topic-ID-through-dumb-relays" problem. Here is the mechanism:

**The descriptor identifier rotates every 24 hours.**

For each onion service, the kernel-side identifier the HSDirs index by
is not the onion's long-term public key directly — it is:

```
descriptor-id = blake2b(
  blinded-pubkey,      // ed25519 pubkey * (current-time-period)
  current-time-period, // increments every 24 hours
  current-srv          // shared random value from authorities
)
```

The **blinded public key** is the service's identity pubkey
multiplied by a hash-derived scalar that depends on the current
24-hour time period. Anyone with the original onion pubkey can compute
the blinded pubkey for any given day (this is what clients do). But
the HSDir that *stores* the descriptor cannot derive the original
pubkey from the blinded one — so HSDirs see only "this opaque
descriptor for this opaque ID expires at midnight UTC."

**Consequence for unlinkability:**

- HSDirs cannot correlate a service's presence across days. Each day,
  a *different* set of HSDirs (chosen by hashing the blinded ID
  against the consensus directory) holds the descriptor.
- A passive observer watching HSDirs cannot enumerate the set of
  services on the network.
- Clients who *don't know the onion address* cannot brute-force
  HSDir-stored descriptors back into onion addresses.

**Shared random value (`SRV`):** A per-day-period random nonce
generated jointly by the directory authorities (Tor's ~10 trusted
operators) using a verifiable distributed protocol. The SRV
guarantees that **no single authority can predict tomorrow's
descriptor-ID layout** in advance — which would otherwise let an
attacker pre-position adversarial HSDirs.

### Why this is "the closest analogue" for Willow / Myrhiza

The Myrhiza problem in `willow/open-problems.md`:

> Members of a topic should be able to **rotate the topic-ID through
> dumb relays** so that the relay does not learn the long-term topic
> identity. Members should be able to find each other across rotation
> boundaries without the relay learning.

Tor's solution maps to:

| Tor v3 piece | Myrhiza analogue |
|---|---|
| Onion long-term ed25519 key | Topic root key |
| Blinded pubkey (rotates per 24h) | Rotated topic-ID |
| HSDir set selected by `H(blinded \|\| period \|\| SRV)` | Set of relays the topic uses this period |
| Service publishes descriptor to HSDirs | Topic members announce next ID to current relay before rotation |
| Client computes blinded-pubkey from onion-address + time-period | Joining peer computes next topic-ID from member-shared root + period |

**What Tor's design teaches Myrhiza explicitly:**

1. **The rotation interval is global, not per-service.** All onion
   services rotate at the same UTC midnight. This means relays don't
   need per-service rotation state — they just expire descriptors at
   the boundary.
2. **The shared-random value is mandatory.** Without an external
   nonce, the period itself is predictable; attackers can mine HSDir
   positions weeks in advance to colocate near victim services.
   Myrhiza will need an analogous nonce source (kernel-generated?
   beacon? Drand?).
3. **Descriptor key blinding is the unlinkability primitive.** Without
   it, a "rotated ID" is just a hash — HSDirs/relays can still
   correlate across periods by content. Myrhiza will need an analogous
   blinding scheme; ed25519's nice scalar-multiply structure is one
   reason Tor picked Curve25519.
4. **Reconnection across the boundary is a hard problem Tor doesn't
   fully solve.** A client connecting at 23:59 UTC may find its
   service via the period-N descriptor; ten minutes later, the
   service has rotated, the client's circuits are dead, and it must
   re-fetch the period-(N+1) descriptor. Tor handles this by
   pre-publishing the next period's descriptor early (overlapping
   window). Myrhiza should plan this.

Sources for the descriptor-rotation mechanism: [Tor rend-spec-v3.txt][rendspec],
sections 1.4 + 2.2.

[rendspec]: https://spec.torproject.org/rend-spec-v3

## arti — the Rust rewrite, now at 2.0.0

The Tor Project has been writing a Rust client called **arti**
([gitlab.torproject.org/tpo/core/arti][arti-repo]) since around 2020.
The motivation: C-tor is ~20 years old, full of historically grown
cruft, hard to add features safely, hard to embed in mobile apps.
arti targets the same wire protocol with a clean Rust API.

[arti-repo]: https://gitlab.torproject.org/tpo/core/arti

**License:** MIT OR Apache-2.0 (standard Rust dual-license).
**Latest version: 2.0.0**, released **2026-02-02**
([blog post][arti-2-0-0]). Prior milestone was 1.0.0 in 2022-09 (the
"ready for production use" SOCKS-proxy MVP). 1.8.0 (2025-12-02) was
the last 1.x.

[arti-2-0-0]: https://blog.torproject.org/arti_2_0_0_released/

**What arti 2.0.0 can do:**

- SOCKS proxy for client connections — production-ready.
- Connect to v3 onion services as a client — stabilized, but with
  a documented gap: **arti's client-side onion implementation is
  missing the `vanguards-lite` feature** that C-tor uses to prevent
  guard-discovery attacks against frequent onion users. The Tor
  Project explicitly notes this is "**not yet as secure as the
  equivalent feature in C-tor**" ([arti 1.1.6 blog][arti-116]).
- Host v3 onion services — experimental, present since arti 1.2.x,
  still labeled "not for production use, not for any purpose that
  requires privacy" as of the 1.x line.
- IP-over-Onion (DNS resolution over onion) — stabilized in 2.0.0.
- RPC interface — present since 1.4.0, lets applications drive arti
  programmatically.

[arti-116]: https://blog.torproject.org/arti_116_released/

**What arti 2.0.0 cannot do:**

- **Cannot run as a relay.** Relay support is the post-2.0 roadmap.
  Until it ships, every arti node is a pure client; the Tor network
  itself is still ~7,000 C-tor relays.
- **Cannot run as a directory authority.** Same — roadmap.
- **No vanguards-lite.** See above.

### Embedding arti — what Myrhiza would do

For Myrhiza to route iroh-on-Tor (per [`prior-art/iroh/lessons.md:34`][iroh-lessons]),
the integration shape is:

[iroh-lessons]: ../iroh/lessons.md

1. Embed `arti-client` as a Rust dependency.
2. Bootstrap an arti `TorClient` per Myrhiza host.
3. Open a SOCKS-style connection: `client.connect(...)` returns a
   `DataStream` wrapping a `tokio::io::AsyncRead + AsyncWrite`.
4. Feed that stream to iroh's custom-transport API instead of UDP.

The cost: **+200–500ms RTT per hop** (Tor adds 2-3× round-trip vs
direct TCP for client-only circuits, **6× for hidden-service
rendezvous**). For chat-style apps where 1-2 second message latency
is fine, this is acceptable. For real-time anything (video, voice,
shared cursor), it is not.

**For state-level threat models** where the user actively wants to
hide IP, this is the option. For metadata patterns (alice-talks-to-bob
every 10s), Tor alone does not help — patterns persist through
circuits. Cover traffic / padding is needed; arti does not provide it
by default.

## Three structural Tor properties Myrhiza should understand

1. **Directory authorities are a 9–10 person trust root.** The Tor
   consensus is signed by a fixed-membership set of operators (Roger
   Dingledine, Nick Mathewson, and ~7 others). They publish a new
   consensus hourly. **If they collude, Tor's anonymity is broken.**
   In practice the diversity (geographic, jurisdictional, social) of
   the authorities has held; this is the model Tor bets on.
2. **Exit relays are a moral/legal hot-zone.** Exit operators receive
   abuse complaints, subpoenas, occasional raids — because traffic
   appears to originate from them. The number of exits is much
   smaller than the number of total relays (~1,500 of ~7,000) and
   this asymmetry is structural. **Hidden services don't have exits.**
   This is why hidden services are the relevant Myrhiza primitive,
   not exit-relay routing.
3. **Funding is dominantly US government.** As of 2012 (the most
   recent published figure), ~80% of the Tor Project's $2M annual
   budget came from US State Department, Broadcasting Board of
   Governors, NSF. Recent budgets are larger but the funding mix is
   not fully public. Critics from both sides — "the US government
   funds Tor to spy on dissidents who use it" / "the US government
   would never fund a tool that worked against US interests" — make
   conspiratorial claims that are difficult to evidence. The
   load-bearing fact: **Tor's design is open and auditable, and the
   audits have held**. The funding source is a sociopolitical
   consideration for users in adversarial-to-US jurisdictions, not a
   technical compromise.

## Latency, throughput, scale

| Metric | Typical | Notes |
|---|---|---|
| Client RTT vs direct | 2-3x | Three hops, ntor key exchange |
| Hidden service RTT | 5-10x | Six hops (3 client + 3 service) |
| Throughput per circuit | 1-10 MB/s | Bottlenecked by slowest hop |
| Network capacity | ~600 Gbps aggregate | 2025 metrics from `metrics.torproject.org` |
| Daily users | ~2-4 million | Per Tor metrics |
| Active relays | ~7,000 | Per consensus |
| Active onion services | ~700,000 (v3) | Per metrics; mostly automated services |

## Repo and source links

- C-tor: <https://gitlab.torproject.org/tpo/core/tor> (BSD-3-Clause)
- arti: <https://gitlab.torproject.org/tpo/core/arti> (MIT OR Apache-2.0)
- Spec: <https://spec.torproject.org/>
- Metrics: <https://metrics.torproject.org/>

## Implications for Myrhiza

- **Topic-ID rotation will need a Tor-v3-style time-blinded identifier
  scheme.** Just hashing `(topic, period)` is not enough — that lets
  relays correlate by content. The blinded-pubkey construction is the
  smallest primitive that actually delivers unlinkability across
  rotations.
- **The rotation interval is a global tuning knob.** Tor picked 24h.
  Myrhiza picking shorter (1h?) means more rotation churn but
  shorter correlation window; longer means less churn but more
  exposure. Pick deliberately and document.
- **Plan a shared-random-value source.** Without it, rotation is
  predictable and the entire mechanism is brittle. Drand, beacon
  chains, or a Myrhiza-internal authority set are the choices.
- **arti is the right embedding for iroh-on-Tor.** MIT/Apache-2.0
  licensing, Rust-native, embeddable. Plan to pin to 2.0.x while the
  hidden-service-server features stabilize.
- **Document the latency cost loudly.** "Myrhiza app run over Tor
  transport: expect +500ms-1s for client connections, +1-3s for
  hidden-service rendezvous." Apps that need sub-100ms must not run
  over Tor.

## Sources

- *Tor (anonymity network)* — Wikipedia: <https://en.wikipedia.org/wiki/Tor_(network)>
- *Onion routing* — Wikipedia: <https://en.wikipedia.org/wiki/Onion_routing>
- Tor v2 deprecation timeline (Tor blog, 2020-07-02): <https://blog.torproject.org/v2-deprecation-timeline/>
- Arti 2.0.0 release (Tor blog, 2026-02-02): <https://blog.torproject.org/arti_2_0_0_released/>
- Arti 1.8.0 release (Tor blog, 2025-12-02): <https://blog.torproject.org/arti_1_8_0_released/>
- Arti 1.1.6 release (Tor blog) — first client onion support: <https://blog.torproject.org/arti_116_released/>
- Tor rendezvous spec v3: <https://spec.torproject.org/rend-spec-v3>
- arti repository: <https://gitlab.torproject.org/tpo/core/arti>
- C-tor repository: <https://gitlab.torproject.org/tpo/core/tor>
- Tor metrics: <https://metrics.torproject.org/>
