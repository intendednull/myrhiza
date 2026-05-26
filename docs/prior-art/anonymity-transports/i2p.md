**Date:** 2026-05-22
**Status:** active
**Subject:** I2P — the Invisible Internet Project. Long-running peer-to-peer anonymity network; an evidence point that "small but real" anonymity stacks can persist for two decades on volunteer labor.

# I2P — the Invisible Internet Project

A peer-to-peer anonymity network started in **2003** (as a Hyphanet
fork) and still actively developed. Different in fundamental
posture from Tor: where Tor optimizes for **anonymous access to the
clearnet** via exit relays, I2P optimizes for **anonymous services
hosted inside the network** (`.i2p` "eepsites" reachable only from
inside I2P). The clearnet-outproxy story exists but is not the focus.

## Key facts

| | |
|---|---|
| **Founded** | 2003 (fork from Hyphanet / Freenet) |
| **Network size** | ~55,000 active routers (Wikipedia, 2025-26 data) |
| **Reference implementation** | Java — current **v2.12.0** (released **2026-04-20**) |
| **Alternative implementation** | **i2pd** — C++, current **v2.60.0** (released **2026-04-25**) |
| **License (Java)** | Mix: public domain, BSD, GPL, MIT (per-component) |
| **License (i2pd)** | **BSD-3-Clause** |
| **Stewardship** | Community-driven; no formal foundation entity |
| **Anonymity primitive** | **Garlic routing** (Chaum-mix-flavored bundling) + tunnels |

## Architecture in 5 lines

1. **Every I2P node is both a client and a router.** Unlike Tor's
   clear client/relay split, every I2P participant relays traffic for
   others (with throttling for low-bandwidth nodes). The network
   topology is fully peer-to-peer.
2. **Each node builds inbound and outbound tunnels.** A tunnel is a
   chain of 2-3 peers that messages traverse. Unlike Tor's
   bidirectional circuit, I2P uses **unidirectional tunnels** —
   separate inbound and outbound paths.
3. **Garlic routing wraps multiple messages.** "Garlic" because each
   layer-encrypted message can contain several payload "cloves"
   (independent messages going to potentially different
   destinations). This complicates traffic analysis: an observer
   seeing N bytes leave a peer cannot tell whether it is one big
   message or ten small ones.
4. **Services use destination keys, not IPs or DNS.** A destination
   is identified by a public key (currently EdDSA Ed25519 for
   modern destinations). Lookup is via the **NetDB**, I2P's
   distributed routing-info database (a custom Kademlia variant).
5. **`.i2p` eepsites are services on the inside.** Conceptually
   parallel to Tor's `.onion` services. The address is the hash of
   the destination's public key (32-byte hash, `.b32.i2p` suffix
   for the compact form).

## Java I2P vs i2pd — the two-implementation reality

Unlike Tor (one canonical C implementation + arti rewrite in
progress), I2P has had **two co-existing implementations for over a
decade**:

| | Java I2P | i2pd |
|---|---|---|
| **Started** | 2003 | 2013 |
| **Language** | Java | C++ |
| **Code size** | Larger; bundles email, BitTorrent, IRC | Smaller; protocol core only |
| **Bundled apps** | I2P-Bote (email), I2PSnark (torrent), Susimail, IRC | None — "stripped of the bloat" |
| **Memory footprint** | Higher | Lower |
| **License** | Mix (public domain, BSD, GPL, MIT) | BSD-3-Clause |
| **Main repo** | <https://geti2p.net/> | <https://github.com/PurpleI2P/i2pd> |
| **Audience** | Desktop users wanting the full I2P suite | Servers, embedded, headless deployments |

**Compatibility:** Both implementations speak the same wire protocol
and participate in the same NetDB. A user running Java I2P and another
running i2pd can route through each other.

**Implications for Myrhiza:** The fact that I2P has sustained two
implementations of a complex anonymity protocol for **13+ years**
without forking the network is a counter-example to the "every
non-trivial protocol becomes single-implementation" narrative. It
took explicit spec discipline and a sufficient-but-not-huge
community. **A two-implementation network is achievable but
expensive.**

## What I2P optimizes for vs Tor

| | Tor | I2P |
|---|---|---|
| **Primary use case** | Anonymous clearnet access | Anonymous in-network services |
| **Exit infrastructure** | ~1,500 exit relays | Few outproxies; not a focus |
| **Hidden / in-network services** | `.onion` v3 | `.i2p` eepsites |
| **Routing direction** | Bidirectional 3-hop circuit | Unidirectional 2-3-hop tunnels |
| **Network size** | ~7,000 relays, ~2-4M users | ~55,000 routers, smaller user count |
| **Default service latency** | Fast for clearnet, slower for hidden | Slower than Tor for clearnet, comparable for in-network |
| **Cover traffic** | None | Light — tunnel-build chatter, garlic-message padding |

**Why I2P is smaller than Tor:** Tor solved the "anonymous web
browsing" UX problem (Tor Browser is one download); I2P has always
been a more technical install (configure your browser to use the I2P
proxy, learn what eepsites are). Tor's focus on clearnet access
matched the dominant threat model (a user in country X wanting to
read clearnet sites their government blocks); I2P's focus on
in-network services matched a narrower threat model (a user wanting
to host or visit sites that should not appear on the clearnet at all).

## Honest about scale

**~55,000 active routers is a real network, not a research artifact.**
But the user count is small relative to Tor:

- Tor: ~2-4M daily users
- I2P: low hundreds of thousands, estimated (no first-party metrics)

I2P does not publish daily-active-user counts. The Wikipedia
"40K active routers" → 55K is a router-count, not a user-count.
**Per-user anonymity is bounded by anonymity set size**; I2P's set is
real but ~10× smaller than Tor's.

**For Myrhiza's purposes:** I2P is an *evidence point*, not a
*candidate transport*. It proves that volunteer-run, foundation-free
anonymity networks can persist for two decades. But its small user
base means an I2P-only Myrhiza transport would give a smaller
anonymity set than Tor or Veilid.

## Garlic routing as a primitive

The most interesting I2P-specific design choice is **garlic-message
bundling**. Where onion routing layers encryption such that each
hop unwraps one layer revealing a single next-hop + payload, garlic
routing lets a *single* outer envelope contain *multiple* inner
"cloves," each potentially destined for a different recipient and
each individually encrypted.

**Why this matters:**

- A single outbound packet from peer A can carry payloads for peers
  B, C, and D. An observer counting A's outbound bytes cannot
  attribute them per-destination.
- Cloves can carry delivery instructions, ACKs, and tunnel-build
  messages alongside user data, amortizing protocol overhead.
- This is the same architectural insight as **Loopix's cover
  traffic** (mix in indistinguishable padding) but achieved through
  bundling rather than synthetic dummy traffic.

**For Myrhiza:** if pattern-privacy ever becomes a Myrhiza
requirement (alice-talks-to-bob-every-10s pattern is detectable
through Tor circuits), garlic-style bundling at the kernel layer
— packing multiple app-level messages into single transport-level
envelopes — is a borrowable primitive. **Cheaper than full cover
traffic, but still complicates traffic analysis.**

## Governance / funding

I2P has **no formal foundation entity**, no central nonprofit, no
budget visibility. Development is volunteer. The Java I2P website
(`i2p.net` since the 2025 migration from `geti2p.net`) is community-
maintained.

This is a different model from Tor's 501(c)(3) and Veilid's
Foundation, and from Nym/HOPR's company-and-token model. **I2P is
the volunteer end of the spectrum.**

**Implication for Myrhiza:** the volunteer-only governance model
*does work* for niche-but-persistent infrastructure. If Myrhiza apps
want to route over I2P, the rules of engagement are: contribute
patches and resources, expect no SLA, expect slow review. Same as
running OpenBSD or NetBSD.

## Repo and source links

- Main project (Java): <https://i2p.net/en/>
- i2pd (C++): <https://github.com/PurpleI2P/i2pd>
- Java I2P repo: <https://github.com/i2p/i2p.i2p>
- Tech docs: <https://i2p.net/en/docs>

## Implications for Myrhiza

- **I2P as a transport plug-in: theoretically possible, practically
  niche.** The wire protocol is stable and documented; either Java
  I2P or i2pd could be embedded. But the smaller anonymity set makes
  it a secondary option behind Tor + Veilid.
- **Garlic-routing as a kernel-level packing primitive: borrowable.**
  Bundle multiple in-flight Myrhiza messages into single transport-
  level envelopes when destinations share an outbound path. Cheaper
  than cover traffic, still useful against traffic analysis.
- **The two-implementation evidence point: take seriously.** Tor is
  effectively single-implementation (C-tor; arti not yet at parity).
  I2P proves that anonymity protocols *can* be specified well enough
  to support multiple implementations. If Myrhiza wants its
  transport-anonymity layer to outlive any single team, follow I2P's
  spec discipline.
- **Volunteer-governed-only is a viable steady state, but not a
  growth state.** I2P has not grown meaningfully since the early
  2010s. If Myrhiza aspires to a Tor-scale (~M users) anonymity set,
  volunteer-only governance is probably insufficient.

## Sources

- I2P — Wikipedia: <https://en.wikipedia.org/wiki/I2P>
- I2P official site: <https://i2p.net/en/>
- i2pd repository: <https://github.com/PurpleI2P/i2pd>
- Garlic routing — Wikipedia: <https://en.wikipedia.org/wiki/Garlic_routing>
- I2P documentation: <https://i2p.net/en/docs>
