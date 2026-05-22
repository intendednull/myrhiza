**Date:** 2026-05-22
**Status:** active
**Subject:** Side-by-side comparison of the five anonymity transports and the two mixnet ancestors. The cross-cutting table for Myrhiza spec authors picking a transport plug-in.

# Comparisons — anonymity transports, side-by-side

The five live transports + two ancestors at a glance. **Use this file
to pick a starting transport candidate; then read the per-system file
for detail.**

## Family tree

```
Chaum 1981 (CACM)
    │ "untraceable email" — layered encryption + batch-and-shuffle
    │
    ├─→ Onion routing (Syverson/Reed/Goldschlag, US NRL, 1995-98)
    │     │ drops the batch-and-shuffle, keeps layered encryption
    │     │
    │     ├─→ Tor (2002, Dingledine + Mathewson)
    │     │     └─→ arti (2020-, Rust rewrite, 2.0.0 in 2026)
    │     │
    │     └─→ Veilid (2023, cDc) — onion-flavored + DHT
    │
    ├─→ I2P (2003, Hyphanet fork) — garlic routing (batch with bundling)
    │
    └─→ Loopix (Piotrowska et al., USENIX 2017)
          │ Poisson mixing + cover traffic + Sphinx packets
          │
          ├─→ Nym (2018-, Halpin) — 5-layer stratified mixnet
          │
          └─→ HOPR (~2018-, Bürgel) — incentivized mixnet, Web3 focus
```

## At-a-glance comparison

|  | **Tor** | **arti** | **Veilid** | **I2P** | **Nym** | **HOPR** |
|---|---|---|---|---|---|---|
| **Family** | Onion routing | Onion routing | Onion + DHT | Garlic routing | Loopix mixnet | Loopix mixnet |
| **Year founded** | 2002 | 2020 | 2023 | 2003 | 2018 | ~2018 |
| **Current version** | 0.4.9.8 | 2.0.0 | 0.5.3 | 2.12.0 (Java) / 2.60.0 (i2pd) | mainnet 2022+ | mainnet 2021+ |
| **Language** | C | **Rust** | **Rust** | Java + C++ | **Rust** | TypeScript + Rust |
| **License** | BSD-3 | MIT/Apache-2.0 | **MPL-2.0** | Mixed (PD/BSD/GPL/MIT) | Apache-2.0 | GPL-3.0 |
| **Stewardship** | 501(c)(3) | 501(c)(3) (same) | **501(c)(3)** | Volunteer | Company + token | Foundation + token |
| **Funding** | US govt + donors | (same as Tor) | Donations | Volunteer | $30M CoinList + token | Token + DAO |
| **Network size** | ~7K relays | (uses Tor net) | hundreds-to-low-K | ~55K routers | ~6K nodes | ~K nodes |
| **Daily users** | ~2-4M | ~2-4M | low K | hundreds of K | low hundreds of K | low tens of K |
| **Latency overhead** | 200-500ms | (same) | hundreds of ms | ~1s | **seconds (5-hop)** | seconds |
| **Cover traffic** | No | No | No | Light (bundling) | **Yes** | Yes (stake-weighted) |
| **Browser-native** | Tor Browser | No (yet) | **WASM build** | Browser proxy | Via Nym client | Via HOPR client |
| **Hidden services** | **v3 (Ed25519)** | v3 client + experimental server | **Private Routes** | **eepsites (.i2p)** | Not core focus | Not core focus |
| **Resists global passive adversary** | No | No | No | Partial | **Yes** (Loopix proofs) | Yes (claimed) |
| **Audits** | Decades of analysis | Same team as Tor | None public | Volunteer-reviewed | JP Aumasson, Oak, Cure53 | Some |
| **For Myrhiza** | First-choice IP privacy plug-in | (becomes first choice once 2.x server-side stable) | **Stylistic kin** — second-choice plug-in | Niche, but garlic-bundling primitive borrowable | Asynchronous-workload plug-in only | Niche — Web3 RPC paths |

## Latency vs anonymity trade-off

```
HIGH ANONYMITY ↑
    │            ┌─ Nym (5-hop mode)
    │            └─ HOPR
    │
    │       ┌─ Nym (5-hop, but expensive)
    │       └─ I2P
    │
    │   ┌─ Tor hidden services
    │   ├─ Veilid Private Routes
    │   └─ I2P eepsites
    │
    │ ┌─ Tor circuits (clearnet)
    │ ├─ Veilid Safety Routes (sender priv)
    │ └─ Nym 2-hop ("fast mode")
    │
LOW ANONYMITY ↓
        LOW LATENCY ←——————————————→ HIGH LATENCY
        (10ms)                       (seconds-minutes)
```

The Y-axis is anonymity guarantee against a strong adversary. The
X-axis is latency added by the transport. **You cannot move
diagonally in this diagram for free** — the engineering trade-off is
fundamental, not implementation-dependent.

For Myrhiza, the implications are:

- **Interactive UI apps (chat with typing indicators, shared
  cursors)** can tolerate **Tor or Veilid Safety Routes** — sub-
  second added latency. They cannot tolerate mixnets.
- **Asynchronous apps (forum posts, CRDT background sync, email-
  like messaging)** can tolerate **Nym 5-hop or HOPR** — seconds of
  latency is fine because the user isn't waiting on a UI frame.
- **Hidden-service-style "host me anonymously" apps** want **Tor v3
  hidden services or Veilid Private Routes** specifically — these
  hide both endpoints, not just the client.

## Threat-model matrix

|  | **Local network observer** | **Hostile ISP / state** | **Global passive observer** | **Compromised endpoint** |
|---|---|---|---|---|
| Tor | Defeats | Mostly defeats | Vulnerable to traffic correlation | Vulnerable |
| Veilid | Defeats | Mostly defeats | Vulnerable (issue #395) | Vulnerable |
| I2P | Defeats | Defeats for in-network only | Vulnerable to long-term analysis | Vulnerable |
| Nym (5-hop) | Defeats | Defeats | **Defeats** (with stated bounds) | Vulnerable |
| HOPR | Defeats | Defeats | Defeats (claimed) | Vulnerable |

**No transport defeats endpoint compromise.** That is an
application-layer (capability + key-isolation) concern, not a
transport concern. **Myrhiza's capability model is the right place
to harden against compromised endpoints; the transport-layer
anonymity stack is orthogonal.**

## Operational maturity

| | Production-ready? | Years deployed | Bus-factor risk |
|---|---|---|---|
| Tor (C) | Yes | 23 | Low (team of dozens, 501c3) |
| arti | 2.0+ for client; server WIP | 0 stable | Low (same team) |
| Veilid | Research-grade | 3 | High (~5-person team) |
| I2P (Java) | Yes | 23 | Medium (volunteer) |
| i2pd | Yes | 13 | Medium (volunteer) |
| Nym | Yes | 4 | Medium (company + token) |
| HOPR | Marginal | 5 | Medium (foundation + token) |

## License compatibility for embedding in Myrhiza

Myrhiza is Apache-2.0 + MIT dual. Which transports can be statically
linked into a Myrhiza app component?

- **arti (MIT/Apache-2.0)**: clean.
- **Veilid (MPL-2.0)**: file-level copyleft. MPL files must remain
  MPL when modified. Embedding `veilid-core` as an unmodified
  dependency is fine; modifications need to ship as MPL.
- **i2pd (BSD-3-Clause)**: clean.
- **Java I2P (mixed)**: complicated — some components GPL. Avoid
  static linking; spawn as a subprocess.
- **Nym (Apache-2.0)**: clean.
- **HOPR (GPL-3.0)**: GPL viral if statically linked. Must run as
  separate process and communicate via IPC.

**Practical conclusion:** arti and Nym are the cleanest embedding
candidates. Veilid is fine as an unmodified dependency. I2P and HOPR
need to run out-of-process.

## Performance characteristics

Throughput (single-stream, typical):

- Tor circuit: **1-10 MB/s** depending on slowest hop
- Tor hidden service: **100-500 KB/s** (six hops)
- Veilid Private Route: **~100 KB/s** estimated (no published benchmarks)
- I2P tunnel: **~100-500 KB/s**
- Nym 5-hop: **~10-100 KB/s** (limited by mix processing)
- HOPR: **~10-100 KB/s** (similar)

Connection setup:

- Tor circuit: **2-10 seconds** to bootstrap directory + build circuit
- Tor hidden service: **5-30 seconds** (descriptor lookup + rendezvous)
- Veilid: **~seconds** for route negotiation
- Nym mainnet: **~seconds** to bootstrap topology
- I2P: **30 seconds - 2 minutes** to integrate into NetDB at startup

The **bootstrap cost** is often the biggest pain in interactive UX —
"why does the app take 30 seconds to start" is the user complaint
that ends up dominating, not steady-state latency.

## What this means for Myrhiza's transport plug-in API

The custom-transport API from [`iroh/lessons.md:34`](../iroh/lessons.md)
contemplated by Myrhiza needs to accommodate:

1. **Asynchronous bootstrap.** Transports take 1-30 seconds to come
   up; the API should be "open me a stream, here's a future to await."
2. **Per-app transport selection.** A chat app picks Tor; a CRDT
   sync app picks Nym (async-tolerant); a high-bandwidth video app
   stays on iroh-direct.
3. **Graceful capability degradation.** If the user is on a network
   that blocks Tor (corporate firewall, country with DPI), the
   transport should signal that to the kernel for fallback handling.
4. **Cost transparency.** Apps should be able to query "what is the
   expected latency / throughput / availability of this transport?"
   without measuring it themselves.

## Cross-references

- [`tor.md`](tor.md), [`veilid.md`](veilid.md), [`i2p.md`](i2p.md), [`mixnets.md`](mixnets.md) — per-system depth
- [`open-problems.md`](open-problems.md) — gaps no transport closes
- [`lessons.md`](lessons.md) — the consult-this-when-designing file
- [`prior-art/iroh/lessons.md`](../iroh/lessons.md) — netlayer-pluggable framing
- [`prior-art/signal/identity.md`](../signal/identity.md) — sealed sender as
  endpoint-layer comparator

## Sources

- Tor metrics: <https://metrics.torproject.org/>
- arti repository: <https://gitlab.torproject.org/tpo/core/arti>
- Veilid Foundation: <https://veilid.org/>
- I2P: <https://i2p.net/en/>
- Nym: <https://nym.com/>
- HOPR: <https://hoprnet.org/>
- Loopix paper: <https://arxiv.org/abs/1703.00536>
- Chaum 1981: <https://cacm.acm.org/research/untraceable-electronic-mail-return-addresses-and-digital-pseudonyms/>
- Sphinx packet format (Danezis & Goldberg, 2009): IEEE S&P
