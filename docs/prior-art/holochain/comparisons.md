# Comparisons

Where Holochain sits relative to neighboring systems. For each comparison: what Holochain claims, where the claim is sound, where it overstates, and what the apples-to-apples picture looks like. Competitive prior art for someone designing a successor — not a marketing matrix.

## vs Ethereum / smart-contract chains

Holochain markets itself as "post-blockchain." The relevant axes:

| Axis | Ethereum | Holochain | Honest read |
|---|---|---|---|
| Global ordering | Yes (chain) | No | Holochain wins on throughput, loses on auctions, scarce-resource allocation, double-spend detection at the protocol level. |
| Sybil resistance | Global PoS | Per-DNA membrane | Different problem solved. Holochain has no global Sybil story. |
| Compute cost model | Gas | Free (host pays) | Holochain has no built-in DoS economics. App authors must encode them. |
| State availability | Globally replicated | Per-shard, AP within app | Holochain is more scalable; less censorship-resistant in practice. |
| Programmability | EVM / Solidity | Rust HDK / WASM | Holochain wins on language ergonomics; Ethereum wins on tooling/auditing maturity. |

**Right:** Global consensus is the wrong tool for most app-level state. A messaging app, a social graph, a per-org ledger does not need PoS finality.

**Wrong / partial:** "post-blockchain" implies superseded. For applications that *do* need globally agreed scarcity (currency, registries, NFTs that must be unique across humanity), Holochain has no answer at the protocol layer; it punts to app code.

**When each works:** Ethereum/L2s for adversarial multi-party scarcity. Holochain (or Holochain-shaped systems) for cooperative, group-scoped state where a membrane defines who counts.

## vs IPFS / libp2p

Holochain rebuilt its networking instead of using libp2p. Earlier prototypes used libp2p; the current stack is Kitsune2 over TX5 (WebRTC + SBD signaling).

**Why they rebuilt:**

- libp2p NAT traversal was unreliable for browser-anchored peers; Holochain wants every laptop and HoloPort to be a peer, not just well-connected servers.
- libp2p's pub/sub + Kademlia don't cleanly express *neighborhood-shard authority + validation gossip*. Holochain needed a DHT that gossips not just key→value but key→(value, validation receipts, link metadata).
- libp2p in Rust was, at the time, less mature for the WebRTC + browser story Holo hosting needed.

**What Kitsune2 / TX5 gives that libp2p doesn't:**

- WebRTC-first transport with SBD as a lightweight signaling broker (so any peer can be reached from a browser-anchored peer without running a full relay).
- Domain-aware gossip — gossip rounds carry validation status, not just opaque blocks.
- Sharded DHT with per-arc storage authority semantics built into the protocol, not the app layer.

**What it loses vs libp2p:**

- No protocol pluralism — you can't run a Holochain DHT alongside a Bitswap session or a GossipSub topic on the same connection.
- Smaller community of audit/eyes than libp2p.
- IPFS's content-addressed-everywhere story (CIDs as a universal lingua franca) is gone — Holochain hashes are app-DNA-scoped.

**For a Component-Model P2P runtime**, the lesson is: libp2p modularity at the transport layer is a strength to keep; Holochain's mistake was conflating "we need richer DHT semantics" with "rip out libp2p."

## vs Secure Scuttlebutt (SSB)

Both are gossip + identity-anchored append-only logs.

| Axis | SSB | Holochain |
|---|---|---|
| Identity | One feed per pubkey, lifetime | One source chain per (agent, DNA); a person has many |
| Replication scope | Friend-of-friend follow graph | Per-app DHT shard |
| Conflict model | Single-writer feeds; no merging | Single-writer source chains + multi-writer DHT entries with validation |
| Apps | One global namespace ("messages and other stuff") | Many sandboxed apps per agent |
| Offline-first | Strong (sneakernet works) | Weak (validation often needs dependency fetches) |

**SSB's bet:** human social graphs are the right replication scope. Cheap, robust, but capped at "what your friends say."

**Holochain's bet:** apps define the replication scope via DNAs. More flexible, but each app pays the cost of standing up its own DHT.

**Tradeoff:** SSB is the better choice if your app *is* a social feed and you want true offline-first. Holochain is better if you need cross-agent shared state with custom validation. SSB has the better aesthetic-of-locality; Holochain has the better aesthetic-of-correctness.

## vs Hypercore / Holepunch / Pears

Both run "apps locally, P2P, no servers." Both target the desktop+mobile JS-or-similar developer.

| Axis | Pears (Holepunch) | Holochain |
|---|---|---|
| Runtime | Bare (small JS runtime) + Node-compatible | Conductor (Rust) + WASM zomes |
| Data primitives | Hypercore (single-writer log), Hyperbee, Hyperdrive, Autobase (multi-writer merge) | Source chain (single-writer log) + DHT (multi-writer with validation) |
| Discovery | HyperDHT + Hyperswarm | Kitsune2 DHT + bootstrap |
| Validation | App-defined; no host-enforced determinism | Host-enforced deterministic validation callbacks |
| Capabilities | None as a primitive | Capability grants as source-chain entries |
| Language | JavaScript-first | Rust-first (any WASM) |

**Pears' bet:** a working JS runtime + a battle-tested log primitive (hypercore) gets developers shipping today. Multi-writer is solved at the app layer with Autobase.

**Holochain's bet:** validation is a first-class protocol concern; apps shouldn't have to reinvent it.

**Tradeoff:** Pears wins on shipping-velocity and developer pool. Holochain wins on adversarial-tolerance (because validation is enforced) and on capability-style integrity. Pears has no equivalent of "the network rejects this entry"; in Pears, bad data just exists in some peer's hypercore and downstream code has to handle it.

## vs Spritely / OCapN

Both are object-capability systems for distributed apps, but at different layers.

| Axis | OCapN / Spritely Goblins | Holochain |
|---|---|---|
| Capability model | CapTP — typed object refs, promise pipelining, distributed GC | Capability grants as DHT entries naming a zome function + caller filter |
| Granularity | Per-method, per-object | Per-zome-function |
| Composition | First-class (refs flow as args/returns) | Out-of-band (you pass a cap secret, you don't pass an object) |
| Transport | Netlayer-pluggable (Tor, libp2p, TCP+TLS) | Kitsune2 |
| Persistence model | Live actors | Append-only source chain |

**Right (about Holochain):** caps are real, signed, revocable.

**Wrong / partial:** Holochain's caps are coarse — "you can call zome function `foo`" — and don't compose. You can't hand a peer a capability that itself contains references to further capabilities the way CapTP can. Holochain caps are closer to bearer tokens with a function selector than to ocap object references.

**Lesson for a successor:** OCapN's CapTP is the more honest ocap model. Combine CapTP-style typed references with Holochain's gossip+validation for state, and you get something neither system has alone.

## vs Croquet / Multisynq

Both are deterministic-replication systems but at opposite ends of the latency / scope spectrum.

| Axis | Croquet (Multisynq) | Holochain |
|---|---|---|
| Replication unit | Computation (events advance a shared model) | Data (source chain entries, DHT entries) |
| Sync model | Reflector-mediated, lockstep, ms-latency | Gossip, eventually consistent, sec-to-min latency |
| Determinism | Required end-to-end (every replica replays events identically) | Required only inside validation callbacks |
| Scope | Session (small group, real-time) | App-wide (whole DNA membership) |
| Failure model | Reflector outage = session pauses | Authority unavailability = retry, indeterminate validation |

**Right (about Holochain):** for non-real-time, large-membership apps, gossip is correct.

**Partial:** Holochain has no story for *real-time co-presence*. You cannot build a multiplayer game or a shared CRDT-style document on raw Holochain primitives without bolting on something like Y.js. Croquet *is* that primitive.

**Lesson:** these are complementary. A general P2P runtime should host both: deterministic-replicated session VMs (Croquet-shaped) for real-time, and validated-DHT (Holochain-shaped) for durable state.

## vs Bluesky / AT Protocol / Nostr

These are federated, not P2P. Important not to lump.

| Axis | AT Protocol | Nostr | Holochain |
|---|---|---|---|
| Architecture | PDS-per-user (server) + relays + AppViews | Relays (server-ish) + key-anchored events | DHT-per-app + agent-per-app |
| Identity | DID + handle | Pubkey | Pubkey-per-DNA |
| Migration | Account portability across PDSes | Trivial (key-based) | Per-app, no global identity |
| Moderation | App-level labelers | Client-side filtering | Per-app validation rules |
| Throughput | Server-bounded | Relay-bounded | Sharded DHT-bounded |
| "Truly P2P"? | No (always servers) | No (relays are servers) | Yes (in principle) |

**Overlap:** All four reject the "single global chain decides truth" model. All four have key-anchored identity. All four push policy to app/client code.

**Where they diverge:** AT and Nostr are pragmatic — they accept servers and design for portability. Holochain rejects servers and pays for it in NAT, sync, and discovery complexity. AT has shipped (millions of users); Nostr has shipped (hundreds of thousands); Holochain has not.

**Lesson:** "P2P-pure" is an aesthetic choice that costs years of latency to product-market fit. AT-style "servers are fine if data and identity are portable" is the pragmatic Pareto point.

## vs Component Model / wasmCloud / Spin

The comparison most relevant to Myrhiza.

| Axis | wasmCloud | Spin | Holochain |
|---|---|---|---|
| Module format | WASM Component Model | WASM Component Model | Pre-CM WASM (custom HDK ABI) |
| Composition | WIT interfaces, wRPC over NATS lattice | WIT interfaces, in-process | None — zomes call each other via host hooks |
| Transport | NATS (cloud, cluster) | HTTP triggers | Kitsune2 (P2P, WebRTC) |
| Identity | NATS decentralized auth (ed25519, JWT) | None at runtime | Per-agent, per-DNA pubkeys |
| State | App-supplied (KV, SQL, NATS JetStream) | App-supplied | Built-in (source chain + DHT) |
| Distribution model | Lattice across clouds/edge | Single host or platform | True P2P |
| Caps as type | Yes (WIT imports = capability set) | Yes (WIT imports = capability set) | No (caps are runtime DHT entries) |

**Where Holochain is right:** typed-state + identity + validation as a runtime concern, not a library concern, is correct. Apps shouldn't have to reinvent multi-writer integrity for the millionth time.

**Where Holochain is behind:** the WASM Component Model + WIT is *exactly* the typed-effect, capability-as-import primitive Holochain's HDK is missing. Holochain still defines its ABI with custom Rust macros, not WIT. wasmCloud and Spin solved that.

**Where wasmCloud is wrong for this niche:** the lattice is a NATS cluster — distributed but not peer-to-peer. No story for two laptops behind NAT replicating state to each other without a NATS server in the middle. wasmCloud treats P2P as out of scope.

**Where Spin is wrong for this niche:** Spin is a serverless trigger model — pure request/response, no durable per-user state, no peer transport.

**The unfilled niche:** no shipping system today combines Component-Model + WIT-typed capability imports with peer-to-peer trust topology, validated source chains, and a browser-viable runtime. Holochain has state+identity but pre-CM ABI; wasmCloud has CM+WIT but cluster-only topology.

## Sources

- Holo Host comparative axes — https://www.buyholo.net/en/learn/comparative
- Holochain Comparisons wiki — https://github.com/holochain/holochain-proto/wiki/Comparisons
- TX5 repo — https://github.com/holochain/tx5
- Kitsune2 repo — https://github.com/holochain/kitsune2
- Scuttlebutt protocol guide — https://ssbc.github.io/scuttlebutt-protocol-guide/
- *SSB: An Identity-Centric Protocol*, ACM ICN 2019 — https://conferences.sigcomm.org/acm-icn/2019/proceedings/icn19-19.pdf
- Pears docs — https://docs.pears.com/
- Holepunch repos — https://github.com/holepunchto
- Spritely OCapN intro — https://spritely.institute/news/introducing-ocapn-interoperable-capabilities-over-the-network.html
- OCapN spec repo — https://github.com/ocapn/ocapn
- Croquet GitHub — https://github.com/croquet/croquet
- Multisynq / Croquet — https://croquet.io/
- *Bluesky and the AT Protocol* (Kleppmann) — https://bsky.social/about/bluesky-and-the-at-protocol-usable-decentralized-social-media-martin-kleppmann.pdf
- Nostr vs Fediverse vs Bluesky (Soapbox) — https://soapbox.pub/blog/comparing-protocols
- wasmCloud project — https://github.com/wasmcloud/wasmcloud
- wasmCloud security/auth — https://wasmcloud.com/docs/hosts/security/
- Spin framework — https://spinframework.dev/
