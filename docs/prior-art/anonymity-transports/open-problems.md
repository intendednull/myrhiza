**Date:** 2026-05-22
**Status:** active
**Subject:** What the anonymity-transport ecosystem structurally does not solve. The pitfalls Myrhiza will inherit from whichever transport plug-in apps select.

# Open problems — what no anonymity transport solves

Five active deployments, two academic ancestors, ~45 years of mixnet
literature. The set of problems they collectively *do not* solve is
the part Myrhiza most needs to know about, because picking any one
of them as a Myrhiza transport plug-in inherits these gaps.

## 1. Endpoint compromise is out of scope for every transport

**None of Tor, Veilid, I2P, Nym, or HOPR protects against an
adversary who has compromised the user's device.** Once the
malicious code is running on the user's machine, the transport-
layer encryption peels off at the application boundary; the
adversary sees plaintext, the user's identity, the user's contacts,
the user's keys.

**The implication for Myrhiza:** the transport plug-in is the
**second-to-last** line of defense, not the last. The
capability-mediated isolation between the kernel and the WASM
component is what makes endpoint compromise survivable — the
kernel can isolate one app's compromised component from the rest.
But the kernel *itself* compromised means all bets are off.

**Mitigation we can't outsource:** Myrhiza's component isolation
boundaries, key-handle capability model, and identity-recovery
flow (per [`signal/identity.md`](../signal/identity.md)). These
sit *above* the transport.

## 2. Global passive adversaries defeat low-latency transports

Tor's threat model **explicitly excludes** the global passive
adversary (GPA) — an attacker who can observe every link in the
network simultaneously. A GPA can correlate traffic timing at the
entry and exit of a Tor circuit and de-anonymize users
statistically, given enough traffic.

This is **not a bug** in Tor; it is a stated design choice. Onion
routing is fundamentally vulnerable to traffic correlation at the
transport layer; mixnets (Loopix, Nym, HOPR) are the response.

**Who counts as a GPA:** Five-Eyes-level state actors with
internet-backbone tap access. Large CDN providers (Cloudflare,
Akamai) who see a meaningful fraction of all internet traffic. AS
operators in concentrated markets.

**The implication for Myrhiza:** if a Myrhiza app's threat model
includes a GPA, the transport must be mixnet-class (Nym or HOPR),
not onion-class. **And the user must accept seconds-to-minutes
of latency.** No transport plug-in resolves the
latency-vs-GPA-resistance trade-off; spec authors choose where the
app sits.

## 3. Anonymity sets are not all equal

The strength of an anonymity guarantee depends on the size of the
*anonymity set* — the set of users that an adversary cannot
distinguish among.

| Transport | Realistic anonymity set |
|---|---|
| Tor | ~2-4M daily users |
| Veilid | low thousands |
| I2P | ~hundreds of thousands |
| Nym (5-hop) | ~100K |
| HOPR | ~10K |

**The cryptography is the same shape** across all these systems
(layered encryption, Sphinx-flavored packets, ephemeral keys). The
math is solid. What differs is the user base.

A "perfectly secure" anonymity protocol with 1,000 users gives
each user an anonymity-set-size of 1,000. That is bigger than the
trivial set (1 — the user themselves) but **substantially weaker
than Tor's ~2-4M**. A targeted adversary who can rule out 999 of
those 1,000 users by other means de-anonymizes the user trivially.

**For Myrhiza:** "we route over Veilid" is a real but limited
anonymity claim. It is **stronger** than "we use TLS." It is
**weaker** than "we route over Tor" by 3-4 orders of magnitude in
anonymity set size. Apps that frame Veilid as "Tor-equivalent" are
overselling.

## 4. Cover traffic is expensive; no transport does enough

Loopix's analysis shows that to defeat a GPA, each user needs to
generate **continuous low-rate cover traffic** that is
indistinguishable from real traffic. The cost is roughly tens of
kbps per active user.

For a smartphone on cellular, **tens of kbps continuous
background traffic is a real battery and data-plan cost.**
Estimated impact: ~10-50 MB/day at rest, plus a measurable battery
drain from radio wake-ups.

Of the live deployments:

- **Tor: no cover traffic.** Tor accepts the GPA tradeoff for
  latency.
- **I2P: very light** — tunnel-build messages and garlic-bundling
  amortize but do not approximate full cover.
- **Nym: real cover traffic in 5-hop mode** — battery cost is one
  reason NymVPN promotes 2-hop "fast mode" for everyday use.
- **HOPR: stake-weighted cover traffic** — node operators
  generate; clients mostly don't.

**The implication for Myrhiza:** if a Myrhiza app wants pattern
privacy beyond IP privacy, the transport must include cover
traffic *and* the user must accept the battery/data cost. There
is no free lunch here. Spec authors should be explicit about which
side of the trade an app sits on.

## 5. Sybil resistance is structural-not-cryptographic

None of these transports have meaningful **proof-of-personhood** or
**Sybil resistance**. Tor relays self-register; anyone can spin up
arbitrarily many. Veilid nodes likewise. I2P routers likewise.
Nym/HOPR mixnodes require a token stake — that is a *cost barrier*,
not a Sybil barrier (a wealthy attacker can run 10K nodes if they
buy 10K stakes).

**The classical attack:** an adversary who controls a large
fraction of relays/mixes can de-anonymize a non-trivial fraction of
traffic by correlation. Tor's 30%-of-bandwidth attacker becomes
viable around the 30%-of-relays threshold (roughly).

**For Myrhiza:** any anonymity-transport plug-in inherits the
underlying network's Sybil posture. **Myrhiza cannot make Tor
more Sybil-resistant** by sitting on top of it. The defense is
diversity-at-the-application-layer: if a Myrhiza app expects to be
attacked by a state-actor-sized adversary, route over **multiple
transports simultaneously** so that no single transport's Sybil
compromise breaks the app. (Run the same connection over Tor *and*
Veilid; require both to be compromised.)

## 6. Mobile UX is universally bad

Tor on mobile is **Tor Browser for Android** (Orbot is also
available). It works but battery-drains, slow-startups, and the
"why is this loading so slowly" UX is unsolved.

Veilid has a WASM browser path but no first-party iOS/Android app.
VeilidChat exists on Android via F-Droid; iOS is sparser.

I2P on mobile: Java I2P on Android requires a heavy install; i2pd
on iOS is essentially impossible (Apple restrictions on background
network daemons).

Nym/HOPR have mobile clients (NymVPN on iOS/Android) but the user
experience is "VPN app that is slower than other VPN apps for
unclear reasons."

**Implication for Myrhiza:** a Myrhiza app that aspires to a
phone-first user base inherits all of these mobile UX gaps. The
spec should treat "Myrhiza app over anonymity transport on mobile"
as a power-user opt-in, not a default. Tor-Browser-grade UX is
acceptable for the Tor-power-user; it is not acceptable as a
default for mass-market apps.

## 7. Bootstrap is slow, and there's no good answer

A fresh Tor client takes 5-30 seconds to come up. Fresh Veilid:
~seconds. Fresh I2P: 30s-2min to integrate into NetDB. Fresh Nym:
~seconds for client config + DH handshakes. Fresh HOPR: similar.

**No transport currently solves "I just opened the app, I want to
send a message in <1 second."** Apps that need fast cold-start
either pre-warm the transport (run a daemon, pay the persistent
cost) or accept the latency.

**For Myrhiza:** the kernel-side decision is whether to run the
transport plug-in as a persistent daemon (paying memory/battery
cost continuously) or as a per-app-launch thing (paying latency
cost per use). This is a kernel-architecture decision, not a
transport-API decision, but the choice cascades into the user
experience.

## 8. Topic-ID rotation through dumb relays — the Willow problem

The original [`willow/open-problems.md:207-218`](../willow/open-problems.md)
cited Tor's hidden-service descriptor rotation as the *closest
analogue* for Myrhiza's topic-ID-rotation problem. Tor's mechanism
gets ~80% of the way there but leaves gaps:

- **Tor's rotation is global (per-24h-period UTC).** Myrhiza topics
  might want different rotation cadences per topic.
- **Tor's rotation set (HSDirs) is selected from the consensus.**
  Myrhiza topics may run on relays whose membership is not
  consensus-driven. Where does the next-period relay set come from?
- **Tor's reconnection-across-boundary** is handled by descriptor
  pre-publish (overlap window). Myrhiza needs an analogous protocol
  but the details (who pre-publishes, where, with what auth) are
  spec-author choices.
- **Tor blinds the descriptor ID against a shared-random nonce.**
  Myrhiza needs an analogous source. Drand? Beacon chain?
  Kernel-generated authority set?

**This is the canonical open problem the corpus closes a gap on:**
spec authors writing Myrhiza's topic-rotation protocol can lift
the descriptor-blinding construction from [`tor.md`](tor.md) but
must design the surrounding policy themselves.

## 9. Latency budgets are app-specific; no transport publishes them

There is no published cross-transport benchmark suite. Numbers in
the [`comparisons.md`](comparisons.md) latency table are gathered
from individual project documentation and academic papers; nobody
has run *the same Myrhiza workload* across Tor, Veilid, I2P, Nym,
HOPR and published the resulting latency distributions.

**Implication for Myrhiza:** when Myrhiza picks transports per
app, the app spec author should publish a Myrhiza-benchmark suite
(connection setup time, sustained throughput, p99 latency) so
that operators choosing transports have empirical data, not
project marketing.

## 10. Verifiable claims of anonymity require formal models, which most don't have

Loopix has a formal model + epsilon-anonymity proofs. **Of the
deployed transports, only Nym inherits these proofs**, and the
inheritance is partial — Nym's 5-hop full-mixnet path matches
Loopix's model; the 2-hop "fast mode" does not.

Tor's anonymity claims are mostly **empirical** ("we resist the
attacks we know about, here are the published analyses"). Veilid
and I2P likewise have no formal model.

**For Myrhiza:** the discipline of stating Myrhiza's anonymity
claim explicitly (against what adversary, under what assumptions,
quantified how) is worth investing in regardless of which
transport plug-in carries the bytes. Without that discipline, "we
use Tor" is a vague claim. With it, "Myrhiza-over-Tor defeats a
local-ISP adversary but not a GPA, with stated anonymity set
~N users" is a real engineering specification.

## 11. Capability composition: anonymity transport vs sealed sender vs encrypted state

Three layers, three different problems. Signal's sealed-sender
hides the sender's identity from the *server*. Tor hides the
sender's IP from the *destination*. End-to-end encryption hides
the *content* from intermediate relays. **All three are needed for
meaningful anonymity**, and Myrhiza will need a composition
strategy.

The composition is not free:

- Sealed-sender requires server cooperation (the server's view of
  the protocol must permit "I'll forward this without inspecting
  origin").
- Tor IP privacy requires transport cooperation (you must actually
  route over Tor; sealing the sender doesn't hide your IP from
  Tor's exit relay).
- E2E encryption requires endpoint cooperation (both endpoints must
  hold the right keys; the broker / relay must be excluded from
  key material).

**For Myrhiza:** spec authors should not promise "anonymous" or
"private" without naming **which layer** of these three is being
provided. A "Tor-routed Myrhiza app with no sealed-sender" still
leaks the sender's pseudo-identity to the destination on every
message. A "sealed-sender Myrhiza app without Tor" leaks the
sender's IP. The combinations matter.

## Cross-references

- [`tor.md`](tor.md) — descriptor-rotation primitive (Problem 8)
- [`mixnets.md`](mixnets.md) — Loopix formal anonymity (Problem 10)
- [`comparisons.md`](comparisons.md) — anonymity set sizes (Problem 3)
- [`lessons.md`](lessons.md) — synthesis of what to borrow / avoid
- [`prior-art/signal/identity.md`](../signal/identity.md) — sealed-sender
  (Problem 11)
- [`prior-art/iroh/lessons.md`](../iroh/lessons.md) — relay metadata
  problem (Problem 2)
- [`prior-art/willow/open-problems.md`](../willow/open-problems.md) —
  the Myrhiza topic-rotation problem (Problem 8)

## Sources

- All per-system files in this folder cite their direct sources.
- Wikipedia *Anonymity network* article (cross-reference): <https://en.wikipedia.org/wiki/Anonymous_P2P>
- Tor metrics on anonymity set: <https://metrics.torproject.org/userstats-relay-country.html>
