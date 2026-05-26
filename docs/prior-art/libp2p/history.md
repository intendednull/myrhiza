**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — history, IPFS split-out (2015–16), Parity rust-libp2p era, ETH2/Filecoin drive, iroh fork-out

# History

A 10-year history. The chronology matters because libp2p's design choices reflect specific moments: the IPFS-split decision (2015–16) is why libp2p has a DHT and content addressing in its DNA; the Parity-era is why rust-libp2p exists; the ETH2 + Filecoin drive (2019–22) is why gossipsub v1.1 has peer scoring; the iroh fork-out (2023) is why the "complexity vs effectiveness" framing dominates contemporary critiques.

## 2014–2015: pre-history (inside IPFS)

[Juan Benet](https://github.com/jbenet) starts the IPFS project. Networking is initially part of the IPFS Go codebase. The early IPFS uses a custom networking layer with Kademlia, mDNS, and a handshake protocol; it's monolithic and tightly coupled to IPFS-specific concerns.

The early networking primitives that later become libp2p:

- `multistream-select` (1A spec written by Juan Benet).
- `multiaddr` (the self-describing address format).
- Kademlia DHT (adapted from the original Maymounkov-Mazières paper).

These are designed inside IPFS but with cleanly separable interfaces.

## 2015–2016: the split-out

Mid-2015, Protocol Labs forms (Y Combinator-backed) to commercialize IPFS. The decision to split networking out:

- The `libp2p/libp2p` umbrella repo is created **2016-06-18**.
- The motivation: "build a modular networking stack that other projects can use, not just IPFS." Ethereum was an early consumer of the framing (Whisper experiments).
- Juan Benet, David Dias, Friedel Ziegelmayer, and others kick off the modularization.
- `go-libp2p` is the first implementation; js-libp2p is started in parallel for browser viability.

The architectural choice in this era: **modular composition** ("everything is a transport upgrade") was a deliberate response to monolithic networking stacks. The cost (multistream-select, three nested handshakes, etc.) wasn't yet apparent.

## 2016–2018: the Parity / Substrate era

[Parity Technologies](https://www.parity.io/) (Gavin Wood's London-based Ethereum-and-then-Polkadot company) writes **rust-libp2p** for Substrate, the framework that became Polkadot. The repo is created **2017-03-24** under [`libp2p/rust-libp2p`](https://github.com/libp2p/rust-libp2p).

- Parity's motivation: Substrate is in Rust; Rust needs libp2p.
- Initial author: Pierre Krieger (`@tomaka`) at Parity. Authors list in current Cargo.toml files still credits "Parity Technologies <admin@parity.io>" — historical.
- Parity hands off maintenance to Protocol Labs as Substrate's networking layer stabilises (~2019).

js-libp2p evolves rapidly through this era, with Brad Stewart, Vasco Santos, David Dias, and others at PL driving it.

## 2018–2020: Filecoin mainnet drive

Filecoin's mainnet launch (October 2020) is the first **production stress test** for the libp2p stack. Issues exposed:

- **Gossipsub v0/v1.0 amplification problems** — Filecoin's miner network gossiped large messages (block headers) at high rates. Without peer scoring, malicious peers could overwhelm the network.
- **DHT lookup performance** — the IPFS DHT was already showing 10–60s lookup latencies. Filecoin couldn't tolerate this for block propagation.
- **Connection churn** — miner peers churning connections caused mesh instability in gossipsub.

The Filecoin launch drives major libp2p investments:

- **Gossipsub v1.1** (peer scoring, peer exchange, explicit peering) — spec landed late 2020, deployment through 2021.
- **DHT optimizations** — Hydra Booster, accelerated DHT client.
- **Circuit Relay v2** — proper TURN-shape relays with reservation + voucher.

## 2019–2022: Ethereum 2.0 / consensus layer drive

Ethereum's transition to proof-of-stake (the Beacon Chain launched 2020-12-01; Merge 2022-09-15) commits to libp2p for the consensus layer's p2p. **Five client implementations** (Prysm, Lighthouse, Teku, Nimbus, Lodestar) all use libp2p — the single largest multi-impl libp2p deployment.

The Eth2 drive adds:

- **gossipsub v1.1 production validation** — the largest live gossipsub deployment by node count. Peer scoring tuned against Eth2's workload.
- **Discovery v5** — Eth team chose Discovery v5 (a libp2p-adjacent protocol) over libp2p Kademlia for consensus-layer peer discovery. The decision was driven by Eth1 continuity, not libp2p deficiency.
- **req/resp protocol pattern** — Eth2's req/resp over libp2p streams + snappy + SSZ became a reference pattern for "fast direct queries within a libp2p network."

## 2022: WebRTC + browser-native

js-libp2p ships **WebRTC** support (browser-to-browser direct) in mid-2022. This is the first time libp2p has a true browser-native peer story (not just "browser dials a relay over WebSocket"). The implementation drives:

- The WebRTC spec ([webrtc r1, 2023-04-12](https://github.com/libp2p/specs/blob/master/webrtc/README.md), Candidate Recommendation).
- The certhash multiaddr extension for `wss`-without-CA.
- WebRTC-Direct spec for browser-to-server without trusted TLS.

The legacy WebRTC-Star (centralized signaling) is deprecated in this era. The narrative shift: "browsers can be peers, not just relays."

## 2023: the iroh fork-out

[Brendan O'Brien (b5)](https://github.com/b5), an ex-Protocol-Labs / IPFS engineer, announces the iroh pivot **2023-02-17** ([A new direction for iroh](https://www.iroh.computer/blog/a-new-direction-for-iroh)). The decision: instead of competing with libp2p's full surface, iroh narrows to "just QUIC + relay + content-addressed blobs."

The iroh framing is explicit and critical (verbatim from the [comparison post](https://www.iroh.computer/blog/comparing-iroh-and-libp2p), Jan 2024):

> *"Most p2p projects end up defaulting into a boil-the-ocean stance where they try to ship one of everything: a DHT, transports, pubsub, RPC. Sometime last year we realized it just wouldn't be possible to ship all this stuff with the team we had, so we picked the transport layer, and are focused on integrating with other projects."*

> *"Libp2p is built to keep its reliance on central points of failure at an absolute minimum, which comes at the cost of effectiveness. Iroh is built to maximize effectiveness, which comes at the cost of a little centralization."*

The iroh-vs-libp2p debate becomes the defining critique of libp2p in 2024–25. The discourse:

- libp2p partisans: iroh is a regression on the centralization axis; "less pure p2p (iroh uses relays)."
- iroh partisans: libp2p's ~70% hole-punching success rate is unacceptable for consumer apps; the multi-transport stack is over-engineered.

Both critiques are real. The choice depends on the threat model and the workload.

## 2023–2024: gossipsub v1.2 IDONTWANT

The v1.2 spec ([Working Draft, r1 2023-07-14](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.2.md)) lands the **`IDONTWANT`** control message. Authors: `@Nashatyrev` (Lodestar / ConsenSys) + `@Menduist` (Nimbus / Status). Use case: reduce duplicate-payload amplification when many mesh peers receive the same message simultaneously.

Implementation status (as of 2026-05-22):

- ✅ go-libp2p-pubsub: shipped.
- ✅ rust-libp2p `libp2p-gossipsub 0.49.4`: shipped.
- ✅ `@chainsafe/libp2p-gossipsub 14.x`: shipped.
- ✅ nim-libp2p: shipped.
- 🟡 jvm-libp2p: partial.
- ❌ cpp-libp2p: not yet.

The Eth2 deployment is rolling out v1.2 unevenly. v1.1 remains the production interop floor.

## 2024: Protocol Labs restructuring

Mid-to-late 2024, **Protocol Labs undergoes significant restructuring**, including layoffs. Public signals:

- Multiple PL employees publicly announce departures (Twitter/Mastodon, Nov–Dec 2024).
- [Hydra Booster](https://github.com/libp2p/hydra-booster) is archived (the IPFS DHT-indexing project is no longer maintained).
- libp2p release cadence visibly slows.

The downstream impact on libp2p:

- go-libp2p continues at steady cadence (it's load-bearing for IPFS and Filecoin, both still funded).
- rust-libp2p slows: the 0.56 → 0.57 release gap has stretched to ~11 months as of 2026-05-22.
- js-libp2p continues with ChainSafe doing increasingly heavy lifting on extension packages.
- Specs continue to evolve but at slower pace; the libp2p/specs README explicitly notes incompleteness.

This is not a project death-spiral, but it is a maintenance-velocity signal. Tracking it is honest research.

## 2025–2026: present-day status

As of 2026-05-22:

- **go-libp2p v0.48.0** (2026-03-17) — healthy, regular release cadence.
- **rust-libp2p `libp2p 0.56.0`** (2025-06-27 on crates.io) — master at 0.57.0 unreleased ~11 months.
- **js-libp2p `libp2p@3.3.1`** (npm latest) — actively maintained.
- **nim-libp2p 1.15.3** — actively maintained alongside Status' Nimbus deployment.
- **gossipsub v1.1** universal production; v1.2 rolling out.
- **WebRTC + WebTransport** mature in js/go; alpha in rust.
- **Spec maturity:** ~10 specs at 3A Recommendation; ~5 at 2A; ~3 at 1A Working Draft.

The narrative for 2026: libp2p remains the **most-deployed P2P stack by validator count + dollar value at risk** (Ethereum consensus); iroh is the **faster-evolving alternative for new apps**. Both will persist; neither is dying. The Myrhiza-relevant signal is the maintenance-velocity differential — iroh ships, libp2p stabilizes.

## Timeline summary

| Year | Event |
|---|---|
| 2014–2015 | IPFS networking layer developed (pre-libp2p) inside go-ipfs |
| 2015 | Juan Benet founds Protocol Labs (YC W14) |
| 2016-06 | `libp2p/libp2p` umbrella repo created; go-libp2p split out |
| 2017-03 | `libp2p/rust-libp2p` repo created (Parity Technologies authorship) |
| 2018 | Substrate (Parity) ships on rust-libp2p; py-libp2p kicks off (EF / Trinity) |
| 2019 | Discovery v5 spec (Eth1 → Eth2 continuity); Parity hands rust-libp2p to PL |
| 2020-07 | GossipSub paper (Vyzovitis et al., arXiv:2007.02754) |
| 2020-10 | Filecoin mainnet launch |
| 2020-12 | Ethereum Beacon Chain genesis (Eth2 phase 0) |
| 2021-12 | gossipsub v1.1 spec r8 finalized (peer scoring production) |
| 2022-09 | Ethereum Merge (consensus layer fully live) |
| 2022-12 | QUIC + Noise specs reach 3A Recommendation |
| 2023-02 | iroh announces pivot (b5: "A new direction for iroh") |
| 2023-04 | WebRTC spec r1 (2A Candidate Recommendation) |
| 2023-07 | gossipsub v1.2 IDONTWANT spec r1 (1A Working Draft) |
| 2023–24 | js-libp2p ships WebRTC + WebTransport browser story |
| 2024 | iroh ↔ libp2p comparison framing dominates discourse |
| 2024-late | Protocol Labs restructuring; Hydra Booster archived |
| 2025–26 | rust-libp2p release cadence slows; gossipsub v1.2 deployment rolls out |
| 2026-03 | go-libp2p v0.48.0 |
| 2026-05 | rust-libp2p still on `libp2p 0.56.0` (crates.io); master 0.57.0 |

## Implications for Myrhiza

- **The "modular composition is over-engineered" framing is real.** libp2p's "everything is a transport upgrade" design (multistream-select + chained upgrades) reflects a 2015 worldview that didn't account for QUIC's emergence. Iroh's "one transport done well" stance is a deliberate 2023 response. Myrhiza inherits iroh's stance.
- **Eth2's choice of Discovery v5 over libp2p Kademlia is informative.** When a major user picks a libp2p-adjacent-but-not-libp2p protocol for a core function, the lesson is: libp2p's primitives are good defaults but not strictly required. Myrhiza can deviate from libp2p patterns when an alternative is genuinely better — the way Eth did.
- **Gossipsub v1.1's peer scoring was production-driven.** The peer-scoring design was driven by Filecoin's mainnet attacks. The v1.2 IDONTWANT was driven by Eth2's mesh-amplification observations. Production stress shapes specs; pre-production specs are aspirational. Myrhiza should not lock spec semantics until at least one production peer-network has stressed them.
- **Protocol Labs is one stakeholder among many.** The libp2p ecosystem can survive PL restructuring because Status, ConsenSys, ChainSafe, Soramitsu, EF all have their own implementations and their own funding. Iroh is single-stakeholder, n0; that is a different risk profile. Myrhiza is single-stakeholder today; we should know this means our risk profile is closer to iroh's than libp2p's.

## Sources

- [Juan Benet — IPFS founding](https://github.com/jbenet)
- [libp2p/libp2p umbrella repo — created 2016-06-18](https://github.com/libp2p/libp2p)
- [libp2p/rust-libp2p — created 2017-03-24](https://github.com/libp2p/rust-libp2p)
- [GossipSub paper (arXiv:2007.02754)](https://arxiv.org/abs/2007.02754)
- [Filecoin mainnet launch (2020-10)](https://filecoin.io/blog/posts/filecoin-mainnet-is-live/)
- [Ethereum Beacon Chain genesis](https://ethereum.org/en/history/#beacon-chain-genesis)
- [iroh — A new direction for iroh (2023-02-17)](https://www.iroh.computer/blog/a-new-direction-for-iroh)
- [iroh — Comparing iroh & libp2p (2024-01-05)](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [gossipsub v1.1 spec — r8, 2021-12-14](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.1.md)
- [gossipsub v1.2 spec — r1, 2023-07-14](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.2.md)
- [Hydra Booster archive](https://github.com/libp2p/hydra-booster)
