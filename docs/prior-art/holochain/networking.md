# Networking — Kitsune2 deep dive

Networking has been Holochain's longest-running engineering problem.

## History

| Era | Layer | What it was | Why it died |
|---|---|---|---|
| 2016-2018 | `holochain-proto` (Go) | libp2p-based DHT prototype | Whole stack rewritten in Rust as RSM ([Announcing the New Holochain](https://medium.com/holochain/unpacking-the-new-holochain-f54da3ca99b7)) |
| 2019 | [`sim1h`](https://github.com/holochain/sim1h) | DHT held in a centralized AWS DynamoDB; pretended to be P2P for testing | Centralized — for dev only |
| 2020 | [`sim2h`](https://github.com/holochain/sim2h) | Centralized switchboard for routing + sharded data | Single point of failure; dev-only crutch |
| 2020-2021 | `lib3h` | First real P2P attempt | Replaced; "obsolete and unmaintained" by RSM |
| 2021-2024 | `kitsune_p2p` | First real production gossip layer | Sync was unreliable; could take 30+ minutes or never complete ([Dev Pulse 148](https://blog.holochain.org/dev-pulse-148-major-performance-improvements-with-0-5/)) |
| 2025-present | `kitsune2` | Ground-up rewrite | Current. Wire-incompatible with kitsune1 — the 0.4→0.5 jump forks all networks |

## The headline number

**Time-to-full-DHT-sync went from 30+ minutes (often "never completed") to ~1 minute** with Kitsune2 ([Dev Pulse 148](https://blog.holochain.org/dev-pulse-148-major-performance-improvements-with-0-5/), [2025 at a Glance](https://blog.holochain.org/2025-at-a-glance-landing-reliability/)). That's a real 30× improvement, but it took roughly four years of work to land.

## Kitsune2 design

- Gossip rounds at 1-minute frequency for newcomers (until they have a full peer table and arc), then back off to 5 minutes ([kitsune2 issue 220](https://github.com/holochain/kitsune2/issues/220)).
- Peer selection filters out recently-failing peers ([issue 222](https://github.com/holochain/kitsune2/issues/222)).
- **Storage arc** is the per-peer claim of "DHT addresses I will store and validate everything about" — neighborhoods are the overlap of arcs ([concepts/4_dht](https://developer.holochain.org/concepts/4_dht/)).

## Transport

Transport went tx5 (WebRTC, with libdatachannel C++ or pion-go backends, [tx5](https://github.com/holochain/tx5)) → iroh (default in 0.6.1-rc, [upgrade-holochain-0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)). Myrhiza already builds on iroh; Holochain converged on the same answer after years of homegrown transport work.

## Cost to keep in mind

The gossip layer alone is roughly half a decade of dedicated engineering (sim1h → sim2h → lib3h → kitsune1 → kitsune2). **Don't underestimate it.** This is the single biggest reason Myrhiza should build on iroh + (potentially) Willow's range-based set reconciliation rather than rolling its own DHT.

## Sharding remains unfinished

A node can declare a partial storage arc and only hold a slice of the DHT, but the arc-resizing logic and the guarantees about data availability under partial arcs are still being shaken out. The roadmap describes "full storage arc declared" as the safe default; partial arcs work but are not the load-tested path ([2025 at a Glance](https://blog.holochain.org/2025-at-a-glance-landing-reliability/)).

After 6+ years of "sharding is coming," Holochain is still effectively a "every node holds everything in their network" system. This is the canonical case study for why sharding-as-future-work is dangerous: you ship the easy thing, the easy thing becomes the load-tested path, and the hard thing never converges.

## Kitsune2 internals — round structure and message types

A gossip round is a strict state machine between an **Initiator** and an **Acceptor**, implemented in [`kitsune2_gossip`](https://lib.rs/crates/kitsune2_gossip):

1. **Initiate** — Initiator sends agent ids, arc set, and a "bookmark" (sync cursor).
2. **Accept** — Acceptor replies with init fields, missing agent infos, new op ids, a new bookmark, and a DHT **snapshot**.
3. Initiator then chooses one of:
   - **NoDiff** — snapshot matches; nothing further to exchange (the steady-state "in sync" signal, gated to data older than 15 minutes per [issue 220](https://github.com/holochain/kitsune2/issues/220)).
   - **DiscSectorsDiff** — disagreement at the disc-sector level (recent ops); send missing agents, new ops, new bookmark.
   - **RingSectorDetailsDiff** — disagreement at the ring-sector level (historical ops); same payload shape.

Diffing is **op-id based**: peers compare hashes of op id sets per sector/ring rather than transferring full op contents speculatively. **Sectors** are the spatial dimension (a slice of the DHT address space); **rings** are the temporal dimension (a time bucket of ops). The two-axis decomposition is the core mechanism that lets a peer say "we agree on everything older than time T in arc range R" with one hash exchange instead of replaying op lists.

Three logically distinct gossip channels are multiplexed over the same rounds: **agent info gossip** (signed peer records announcing arc claims and transport URLs), **op gossip** (the DHT data itself), and behavior/metric data fed into the **behavior store** for peer scoring (`failed-to-connect` flags, last-gossip timestamps — see [issue 221](https://github.com/holochain/kitsune2/issues/221), [issue 222](https://github.com/holochain/kitsune2/issues/222)).

Gossip cadence is governed by three knobs: `initial_initiate_interval_ms` (aggressive while bootstrapping), `initiate_interval_ms` (steady-state — 1 min for newcomers, 5 min once synced), and `min_initiate_interval_ms` (floor on how often any given peer pair will re-gossip, regardless of role).

## Bootstrap, signal, and relay infrastructure

The 0.5 release [collapsed three previously separate services into one binary](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.5/), `kitsune2-bootstrap-srv` ([repo: `bootstrap2`](https://github.com/holochain/bootstrap2)). It provides peer discovery, WebSocket-based WebRTC signaling (the **SBD** — Simple Bootstrap Daemon — protocol), and message relay as fallback.

State is **ephemeral and per-instance**: the [Running Network Infrastructure docs](https://developer.holochain.org/resources/howtos/running-network-infrastructure/) state explicitly that "the state can't be shared among instances of the bootstrap server for load-sharing." Therefore **not federated** — operators scale by running independent bootstrap servers on different URLs, and peers re-announce themselves when they reconnect to a new instance. The Holochain Foundation runs a public test instance at `https://dev-test-bootstrap2.holochain.org/` (the older `bootstrap.holo.host` / `signal.holo.host` / `turn.holo.host` triple is pre-Kitsune2). Production deployments are expected to self-host. The server keeps lists of agent infos segregated by DNA hash; an agent fetches the list, then dials peers directly over the configured transport.

For **iroh** (added in 0.6.0, made default in the 0.6.1-rc line, see [upgrade-holochain-0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)), Holochain piggybacks on the four public **n0 relays** that ship with iroh by default (US x2, EU, Asia), configured per conductor via `network.relay_url`. Local-iroh-relay setups require enabling unencrypted relay connections in the conductor template. For the legacy **tx5** WebRTC stack, signaling and TURN run via the same `kitsune2-bootstrap-srv` binary speaking SBD.

## Storage arc resizing — the unfinished story

Per [Concepts/4_dht](https://developer.holochain.org/concepts/4_dht/), arc resizing is heuristic and uptime-based: peers observe their neighbors' uptime, **enlarge** arcs when neighbors disappear, and later **shrink** when redundancy is excessive. There is no published formal algorithm or convergence proof — empirically tuned, not derived from research. The blog acknowledges in [2025 at a Glance](https://blog.holochain.org/2025-at-a-glance-landing-reliability/) that partial-arc operation is not yet the load-tested path and that "full storage arc declared" is the safe default.

The failure mode for depopulated neighborhoods is implicit and ugly: there is no explicit invariant that says "if N peers leave, you lose data D." Surviving peers attempt to grow their arcs, but if the arc-resize signal arrives slower than the departure rate, ops in the abandoned slice are unrecoverable — there is no fallback to the source-chain author (who may also be offline). Holochain treats this as an operational concern of the application, not a guarantee of the platform.

## Bandwidth and rate limiting

Kitsune1's hard-coded gossip rate limit was **0.5 Mbps for recent data and 0.1 Mbps for historic data**; [Kitsune2 raised this to 10 Mbps for all data with a 1 GB burst over a 10-second window](https://blog.holochain.org/gossip-performance-improvements/). Per-peer-pair, applied symmetrically.

DOS resistance at the gossip layer comes from three places: (1) the per-peer rate cap above, (2) `min_initiate_interval_ms` which prevents a single peer from forcing repeated rounds, and (3) the behavior-store peer scoring that excludes recently-failing peers from gossip targets ([issue 222](https://github.com/holochain/kitsune2/issues/222)). No global gossip-layer rate limit and no proof-of-work admission control — a sufficiently large Sybil set could still saturate honest peers' inbound bandwidth budgets. Application-layer DOS resistance (invalid-op spam) is handled by the validation/warrant system: invalid ops produce a signed warrant against the author and the author is excluded.

## Wire-protocol versioning and migration

The kitsune1 → kitsune2 transition was **wire-incompatible by design** ([upgrade-holochain-0.5](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.5/)): "conductors running 0.5 won't be able to communicate with conductors running earlier releases." No compatibility shim, no dual-stack period, no in-place upgrade for existing networks. Apps had to redeploy onto fresh DHTs at 0.5, with `origin_time` and `quantum_time` removed from `dna.yaml` and the bootstrap/signal infrastructure swapped to `kitsune2-bootstrap-srv`. Because the DNA hash is part of the network identity and these fields fed into it, the wire change was effectively also a DNA-hash change for many apps. The first time in Holochain's history that the entire deployed ecosystem was forced to fork networks for a transport change. No well-known long-lived production DHTs from the kitsune1 era survived the transition; testnets simply restart.

## Implications for Myrhiza

- **iroh is the right transport.** Validates the PR choice. Saves 4 years.
- **Don't build a custom signaling/switchboard as a "temporary" dev shortcut.** sim2h became culturally entrenched and slowed the real P2P work for a year+. iroh gives you NAT traversal + relays as a real solution from day 0.
- **Decide the sharding model up front.** Either commit to "every node holds everything" with explicit scale ceiling, or commit to a sharding model and load-test it from MVP. Don't ship "we'll figure it out later."
- **Wire compatibility is a load-bearing product surface.** Plan dual-stack windows for transport changes, or accept that every change forks every network. Holochain has done both; the latter is painful.
- **DOS at the gossip layer needs explicit thought.** Bandwidth caps + behavior-store scoring is the floor; assume a sufficiently determined Sybil can still saturate.

## Sources

- [Dev Pulse 148: Major Performance Improvements with 0.5](https://blog.holochain.org/dev-pulse-148-major-performance-improvements-with-0-5/)
- [2025 at a Glance: Landing Reliability](https://blog.holochain.org/2025-at-a-glance-landing-reliability/)
- [Gossip Performance Improvements](https://blog.holochain.org/gossip-performance-improvements/)
- [kitsune2_gossip on lib.rs](https://lib.rs/crates/kitsune2_gossip)
- [Kitsune2 issue 220 — initial sync time](https://github.com/holochain/kitsune2/issues/220)
- [Kitsune2 issue 221 — behavior store](https://github.com/holochain/kitsune2/issues/221)
- [Kitsune2 issue 222 — peer filtering](https://github.com/holochain/kitsune2/issues/222)
- [bootstrap2 repository](https://github.com/holochain/bootstrap2)
- [Running Network Infrastructure](https://developer.holochain.org/resources/howtos/running-network-infrastructure/)
- [Concepts — DHT](https://developer.holochain.org/concepts/4_dht/)
- [Sim1h repo](https://github.com/holochain/sim1h)
- [Sim2h repo](https://github.com/holochain/sim2h)
- [tx5 repo](https://github.com/holochain/tx5)
- [Sim2h: Holochain's Simple Switch-board Networking](https://blog.holochain.org/sim2h-holochains-simple-switch-board-networking/)
- [Upgrade 0.4 → 0.5](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.5/)
- [Upgrade 0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
