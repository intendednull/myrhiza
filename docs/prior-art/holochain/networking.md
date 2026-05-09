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

Transport went tx5 (WebRTC, with libdatachannel C++ or pion-go backends, [tx5](https://github.com/holochain/tx5)) → iroh (default in 0.6.1, [upgrade-holochain-0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)). Myrhiza already builds on iroh; Holochain converged on the same answer four years later.

## Cost to keep in mind

The gossip layer alone is roughly half a decade of dedicated engineering (sim1h → sim2h → lib3h → kitsune1 → kitsune2). **Don't underestimate it.** This is the single biggest reason Myrhiza should build on iroh + (potentially) Willow's range-based set reconciliation rather than rolling its own DHT.

## Sharding remains unfinished

A node can declare a partial storage arc and only hold a slice of the DHT, but the arc-resizing logic and the guarantees about data availability under partial arcs are still being shaken out. The roadmap describes "full storage arc declared" as the safe default; partial arcs work but are not the load-tested path ([2025 at a Glance](https://blog.holochain.org/2025-at-a-glance-landing-reliability/)).

After 6+ years of "sharding is coming," Holochain is still effectively a "every node holds everything in their network" system. This is the canonical case study for why sharding-as-future-work is dangerous: you ship the easy thing, the easy thing becomes the load-tested path, and the hard thing never converges.

## Implications for Myrhiza

- **iroh is the right transport.** Validates the PR choice. Saves 4 years.
- **Don't build a custom signaling/switchboard as a "temporary" dev shortcut.** sim2h became culturally entrenched and slowed the real P2P work for a year+. iroh gives you NAT traversal + relays as a real solution from day 0.
- **Decide the sharding model up front.** Either commit to "every node holds everything" with explicit scale ceiling, or commit to a sharding model and load-test it from MVP. Don't ship "we'll figure it out later."

## Sources

- [Dev Pulse 148: Major Performance Improvements with 0.5](https://blog.holochain.org/dev-pulse-148-major-performance-improvements-with-0-5/)
- [2025 at a Glance: Landing Reliability](https://blog.holochain.org/2025-at-a-glance-landing-reliability/)
- [Kitsune2 issue 220 — initial sync time](https://github.com/holochain/kitsune2/issues/220)
- [Kitsune2 issue 222 — peer filtering](https://github.com/holochain/kitsune2/issues/222)
- [Concepts — DHT](https://developer.holochain.org/concepts/4_dht/)
- [Sim1h repo](https://github.com/holochain/sim1h)
- [Sim2h repo](https://github.com/holochain/sim2h)
- [tx5 repo](https://github.com/holochain/tx5)
- [Sim2h: Holochain's Simple Switch-board Networking](https://blog.holochain.org/sim2h-holochains-simple-switch-board-networking/)
- [Upgrade 0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
