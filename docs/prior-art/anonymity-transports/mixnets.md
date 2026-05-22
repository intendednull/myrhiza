**Date:** 2026-05-22
**Status:** active
**Subject:** Mixnets — the academic lineage from Chaum 1981 through Loopix 2017, and the two production-attempting mixnets descended from Loopix: Nym and HOPR. Where Tor is "low-latency onion routing," mixnets are the high-latency-but-truly-traffic-analysis-resistant alternative.

# Mixnets — Chaum → Loopix → Nym / HOPR

A separate family from Tor's onion routing. **Onion routing** layer-
encrypts a packet through a chain of relays; the relays forward
immediately. **Mixnets** layer-encrypt *and* deliberately delay,
batch, or shuffle packets at each hop, making timing correlation
between input and output expensive for an observer.

The cost is latency. Mixnets are intrinsically slower than onion
routing — **seconds to minutes** of end-to-end delay vs Tor's
**100s of milliseconds**. The benefit is that mixnets resist
**global passive adversaries** (an attacker who can observe every
link in the network) where Tor structurally cannot.

## The original — Chaum 1981

**David Chaum**, *"Untraceable Electronic Mail, Return Addresses, and
Digital Pseudonyms"*, **Communications of the ACM, Vol. 24 No. 2,
February 1981, pp. 84-90** (DOI: [`10.1145/358549.358563`][chaum-doi]).

[chaum-doi]: https://doi.org/10.1145/358549.358563

The foundational paper. Chaum proposed:

1. A **mix** is a server that receives encrypted messages from
   multiple senders, decrypts one layer of each, **batches them**,
   **shuffles them**, then forwards the result to the next hop.
2. With layered encryption (the construction we now call **onion
   encryption**, but Chaum did it first), no mix in the chain knows
   both the original sender and the final recipient.
3. **Return addresses** can be encrypted such that the recipient can
   reply without learning the original sender's address.

**Chaum's mixnet has a critical property mixnet descendants kept and
onion routing dropped:** the batch-and-shuffle step. Each mix
collects N messages, waits, then forwards them in randomized order.
This destroys timing correlation between inputs and outputs at the
mix's boundary.

**Cost:** latency proportional to batch fill rate and mix depth.
Chaum's 1981 paper was written for email — a use case where minutes
of delay are fine.

## Loopix — the modern mixnet design (2017)

**Piotrowska, Hayes, Elahi, Meiser, Danezis**, *"The Loopix Anonymity
System"*, **USENIX Security Symposium 2017** (Vancouver, August).
[arXiv:1703.00536][loopix-arxiv].

[loopix-arxiv]: https://arxiv.org/abs/1703.00536

Loopix is the modern academic-grade mixnet design. Both **Nym** and
**HOPR** descend from it directly.

**What Loopix changed from Chaum:**

1. **Poisson mixing instead of threshold batching.** Each message
   arriving at a mix is delayed by an exponentially-distributed
   amount (independent per-message). No "batch fill" semantics — the
   mix forwards continuously, but with random per-message latency.
   This avoids the **threshold-flooding attack** where an adversary
   can fill a mix's batch with their own traffic and watch what
   remains.
2. **Cover traffic — loops + drops.** Each Loopix client emits
   continuous *cover traffic*: messages it sends to itself ("loops")
   to camouflage real traffic; "drop messages" to non-existent
   recipients to fill bandwidth at idle. An adversary watching the
   network cannot distinguish real traffic from cover traffic
   without breaking the encryption.
3. **Stratified topology.** Mixes are organized into layers (Loopix
   uses 3 layers); each message traverses exactly one mix per layer.
   This bounds the number of possible paths and concentrates cover
   traffic per layer.
4. **Sphinx packet format.** All Loopix packets are the **same
   fixed size** (Sphinx is a packet format that pads to fixed
   length and provides forward-secure layered encryption). An
   observer counting bytes per link learns nothing about individual
   messages.

**Performance from the paper's evaluation:**

- ~1.5ms processing overhead per mix.
- ~300 messages per second per mix.
- End-to-end message latency on the order of **seconds** (not
  milliseconds).
- Resists global passive adversaries with quantified anonymity bounds.

**The Loopix anonymity guarantee is mathematically grounded.** The
paper provides formal definitions ("epsilon-sender anonymity") and
proves the system achieves them under stated assumptions. This is
strictly stronger than Tor's "we resist the attacks we know about,
empirically."

## Nym — the largest deployed Loopix descendant

**Founded 2018** in Neuchâtel, Switzerland. Founder: **Harry
Halpin** (computer scientist, ex-INRIA). Whitepaper Feb 2021 with
**Claudia Diaz** and **Aggelos Kiayias**. Concept originated from
two EU-funded research projects (Panoramix, NEXTLEAP) following the
2013 Snowden disclosures.

### Key facts

| | |
|---|---|
| **Company** | Nym Technologies AG (Switzerland) |
| **Founder** | Harry Halpin |
| **Token** | NYM (1 billion max supply, fully unlocked) |
| **CoinList sale** | **2022-02-09**, raised **>$30M**, 51K new token holders, 1.19M unique registrants |
| **Mainnet launch** | **2022-04-14** (Station F, Paris; Snowden keynote) |
| **Topology** | 5-layer: entry gateway → 3 mix layers → exit gateway |
| **Topology update cadence** | Hourly, based on reputation |
| **Wire format** | Sphinx packets (Loopix-style) |
| **Implementation** | **Rust**, Apache-2.0 |
| **Repository** | <https://github.com/nymtech/nym> |
| **Flagship product** | **NymVPN** — public release 2025-03 |
| **Audits** | JP Aumasson, Oak Security, Cure53 (multiple) |

### Architecture specifics

Nym's stratified topology has **5 layers**, not Loopix's 3:

```
client → entry gateway → mix layer 1 → mix layer 2 → mix layer 3 → exit gateway → destination
```

Entry and exit gateways are functionally fixed (clients pick one of
each and stick with it for the session); the three mix layers
shuffle per-message.

**NymVPN modes:**

- **Fast mode (2-hop, WireGuard-style):** ~50ms latency, ~100 Mbps.
  *Not a true mixnet path* — this is more like a regular decentralized
  VPN. Marketed for everyday use.
- **Anonymous mode (5-hop, full mixnet):** seconds of latency,
  bandwidth limited by mix processing. The actual Loopix-flavored
  Nym path.

This **fast mode vs anonymous mode** split is honest: the
mathematically-grounded Loopix anonymity is in the 5-hop mode, not
the 2-hop. Users who want speed don't get the mixnet guarantees.

### Honest about Nym

**The good:**

- Functioning Rust implementation, multiple security audits, real
  open-source code.
- Direct lineage from Loopix's formal anonymity proofs; the
  cryptographic foundation is sound.
- Mainnet has been live since 2022; ~6,000 node operators after the
  CoinList allocation phase.
- NymVPN is a real product with a real user-facing app.

**The cautionary:**

- **Token-funded.** $30M CoinList sale; the NYM token is on
  cryptocurrency exchanges; price chart is volatile. Investor
  pressure to push narratives ("$NYM = decentralized VPN of the
  future") is a real bias source.
- **Fast mode dilutes the message.** Most marketing emphasizes
  NymVPN's "decentralized VPN" framing; the Loopix-anonymity
  guarantee is only in the 5-hop anonymous mode that most users
  never enable.
- **Topology trust:** the entry+exit gateways see client IP →
  next-hop. They cannot link to destination, but they are a known-
  IP-side trust point. Tor's entry-guard model is similar but
  better-studied.
- **Mainnet usage is small.** The Nym dashboard does not publish
  daily active connection counts; informally, NymVPN's userbase is
  thought to be in the low hundreds of thousands at most — far
  below Tor's ~2-4M.

## HOPR — the smaller Loopix descendant

**Founded** late-2010s in Zürich, Switzerland by **Sebastian Bürgel**
(later HOPR Association president, helped draft Switzerland's DLT
law). Token launch via **HOPR Genesis DAO** on **2021-02-24** —
preceded by a community presale on xDAI then public distribution on
Balancer + Uniswap.

### Key facts

| | |
|---|---|
| **Stewardship** | HOPR Association (Switzerland) |
| **Founder** | Sebastian Bürgel |
| **Token** | HOPR (ERC-20; runs on Gnosis Chain + Ethereum mainnet) |
| **Token launch** | 2021-02-24 |
| **First product** | RPCh (2023-Q1) — private RPC routing for Web3 |
| **Architecture** | Loopix-descended; incentivized mixnet via token-staked nodes |
| **Cover traffic** | Yes — "proportional to staked tokens" |
| **Repository** | <https://github.com/hoprnet/hoprnet> |
| **License** | GPL-3.0 (core) |

HOPR's pitch is the same Loopix-descended mixnet idea, with two
twists vs Nym:

1. **Crypto-economic incentives baked deeper.** Every relay hop is
   token-paid; cover traffic generation is stake-weighted. This is
   meant to make running a HOPR node economically self-sustaining,
   not donation-supported.
2. **Application focus on Web3 RPC privacy.** RPCh ("RPC through")
   was HOPR's first commercial product — routing Ethereum/Web3 wallet
   RPC calls through the HOPR mixnet, so that wallet providers don't
   see user IP. A narrower use case than Nym's general-purpose
   privacy.

### Honest about HOPR

**Scale is smaller than Nym.** HOPR's node count and user count are
both meaningfully below Nym's. The token-economic story is more
detailed but its actual user adoption (outside RPCh's wallet
integrations) is thin. Less independently audited.

**Token model means same investor-pressure failure mode as Nym.**

## Comparative table — Tor vs Nym vs HOPR vs Veilid vs I2P

| | **Tor** | **Nym** | **HOPR** | **Veilid** | **I2P** |
|---|---|---|---|---|---|
| Family | Onion routing | Loopix mixnet | Loopix mixnet | Onion-flavored DHT | Garlic routing |
| Latency overhead | 200–500ms | seconds (5-hop) | seconds | hundreds of ms | seconds |
| Cover traffic | None | Yes (Loopix-style) | Yes (stake-weighted) | None | Some (garlic bundling) |
| Anonymity set | ~2-4M users | ~100s of K | ~10s of K | hundreds to low K | ~55K routers |
| Audits | Many | Multiple (Aumasson/Oak/Cure53) | Some | None public | Volunteer-reviewed |
| Stewardship | 501(c)(3) | Company + token | Foundation + token | 501(c)(3) | Volunteer |
| Wire spec | Public, frozen | Public, evolving | Public | Single impl | Public, evolving |
| Production-ready | Yes (decades) | Yes (2022+) | Marginal | Research-grade | Yes (decades) |

## Implications for Myrhiza

- **Mixnets are the option when pattern privacy matters, not just
  IP privacy.** If a Myrhiza app's threat model is "global passive
  observer watches every link," Tor is not enough — circuits leak
  timing patterns. Loopix descendants are the academic answer; Nym
  is the most mature deployment.
- **The latency cost is *seconds*, not milliseconds.** A Myrhiza
  app routed over a true 5-hop Nym path will see end-to-end latency
  in the 1-10 second range. This is unusable for any interactive
  app. **Mixnet transports are for asynchronous workloads only**:
  pub/sub-style state propagation, email-flavored messaging,
  CRDT-sync where eventual consistency is the contract anyway.
- **Cover traffic at the kernel level is borrowable.** Even outside
  a full mixnet routing layer, generating low-rate background
  traffic at the Myrhiza-host level helps obscure when a user is
  active. The cost is small (a few KB/s); the gain against
  presence-detection adversaries is real. Watch Loopix's analysis
  of optimal cover-traffic rate.
- **Token-funded anonymity infrastructure is structurally
  conflicted.** Nym and HOPR both raise this concern: tokens
  introduce investor-narrative pressure that can distort the
  project's privacy claims. Tor's nonprofit model and Veilid's
  foundation model are cleaner. Be skeptical of mixnet projects
  whose marketing leads with the token.
- **Borrow Loopix's *evaluation methodology*, not necessarily its
  protocol.** Loopix provides formal anonymity definitions and proofs
  under stated assumptions. Any Myrhiza-level anonymity claim should
  follow that template — stated adversary, stated assumptions,
  quantified guarantees — rather than "trust us, it's encrypted."

## Cross-references

- [`tor.md`](tor.md) — onion routing as the low-latency alternative
- [`veilid.md`](veilid.md) — DHT + onion-flavored P2P (not a mixnet)
- [`i2p.md`](i2p.md) — garlic routing, conceptually nearest cousin
- [`comparisons.md`](comparisons.md) — side-by-side
- [`open-problems.md`](open-problems.md) — what no current mixnet solves
- [`prior-art/iroh/lessons.md`](../iroh/lessons.md) — netlayer-pluggable transport framing

## Sources

- David Chaum, *"Untraceable Electronic Mail, Return Addresses, and Digital Pseudonyms"*, CACM 24(2), Feb 1981, pp. 84-90. <https://cacm.acm.org/research/untraceable-electronic-mail-return-addresses-and-digital-pseudonyms/>
- Piotrowska, Hayes, Elahi, Meiser, Danezis, *"The Loopix Anonymity System"*, USENIX Security 2017. <https://www.usenix.org/conference/usenixsecurity17/technical-sessions/presentation/piotrowska>
- Loopix arXiv: <https://arxiv.org/abs/1703.00536>
- Nym — Wikipedia: <https://en.wikipedia.org/wiki/Nym_(mixnet)>
- Nym Technologies — Wikipedia: <https://en.wikipedia.org/wiki/Nym_Technologies>
- Nym CoinList sale announcement: <https://blog.coinlist.co/announcing-the-nym-token-sale-on-coinlist/>
- Nym mainnet launch coverage: <https://cryptobriefing.com/nym-technologies-invites-users-and-developers-to-its-privacy-enhancing-mixnet-following-record-breaking-coinlist-sale/>
- Nym docs (network concepts): <https://nym.com/docs/network/concepts/mixing>
- Nym repository: <https://github.com/nymtech/nym>
- HOPR token launch: <https://www.globenewswire.com/news-release/2021/02/24/2181048/0/en/HOPR-Hosts-Global-Token-Launch-Event-on-February-24th-after-First-of-Its-Kind-DAO-Experiment.html>
- HOPR repository: <https://github.com/hoprnet/hoprnet>
- HOPR token introduction (Bürgel, Medium): <https://medium.com/hoprnet/introducing-the-hopr-token-bd4a2a31fc7f>
- Sphinx packet format — Danezis & Goldberg, 2009: <https://www.ieee-security.org/TC/SP2009/oakland09.html>
