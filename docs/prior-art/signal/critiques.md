**Date:** 2026-05-22
**Status:** active
**Subject:** Third-party critiques of Signal — phone-number-rooted identity, centralization, "ecosystem is moving" controversy, metadata exposure, governance.

# Signal — critiques

Signal is widely considered the gold standard for E2EE messaging in the
security community. The critiques below are substantive and largely come
from cryptographers, security researchers, and decentralization advocates
who *use* Signal but disagree with specific design choices. They are not
contrarian-for-its-own-sake critiques.

## 1. Phone-number-rooted identity (mostly resolved)

**The critique:** For ~12 years, Signal required a phone number as the
primary identifier. Critics (notably the Tor community, EFF, and
decentralization advocates) argued this was incompatible with anonymity:
to use Signal, the user had to expose a phone number to *someone* (the
SMS gateway, the cellular carrier, the SIM issuer, possibly the home
country's lawful-intercept regime). For activists, journalists, and
others under surveillance, this was a hard barrier.

Marlinspike's published defense (multiple interviews, ~2015-2020) was
that phone-number identity was the only practical way to drive mass
adoption — comparable to email-as-identity but with the spam-resistance
property that phone numbers are scarce. Users would not adopt a contact-
discovery flow as friction-heavy as PGP key exchange.

**The resolution:** PNI (announced 2022-11-15) + usernames (rolled out
2024-02-20) significantly weakened this critique. A user can now operate
on Signal with a phone number that is *not visible* to non-contacts, and
can be reached via a username instead of a phone number.

**What remains:** registration *still requires* a phone number reachable
by SMS or voice-call. You cannot create a Signal account without one.
For the most extreme threat models (an adversary that has access to
*all* cellular records), this is still a leak — the registration phone
number is known to Signal Foundation and, via SMS routing, to the SMS
gateway. The PNI-rotation feature does not help with this.

## 2. Centralization

**The critique:** Signal is centralized. One organization (Signal
Foundation) operates one ingress (signal.org), runs the contact-discovery
SGX enclaves, holds the Signal CA key for sealed-sender certificates,
and could be served a court order or compromised.

**Marlinspike's defense:** "The ecosystem is moving" (2016-05-10) —
centralization is necessary for protocol iteration speed. A federated
protocol can't ship sealed sender, then PNI, then PQXDH in the span of
five years. Federation freezes protocols.

**Counter-counter from the Matrix community** (Matthew Hodgson, "Beware
the Matrix" blog post, 2020-08): "Federation is not a feature, it's a
property." Matrix federates and has shipped iterations (Olm, Megolm,
state res v2). Slower than Signal, but possible. Federation is "the
ecosystem is moving *together*" rather than the ecosystem moving via
one operator.

**What Marlinspike does not address:** centralization concentrates
*risk*. A single nation-state can attack Signal Foundation directly
(legal compulsion, infrastructure compromise, supply-chain attack on
the libsignal release pipeline). Federation distributes this risk; the
cost is iteration speed. Whether the trade is correct depends on the
threat model.

**Concrete operational manifestation:** Signal's Brazilian and UAE
blocks (2016 onward) show what happens when a nation-state attacks the
ingress. Signal can move ingress (Domain Fronting, Reflectors, etc.)
but the underlying architecture has one operator who can be pressured.

## 3. SGX enclaves are not load-bearing crypto

**The critique:** Signal's contact-discovery and SVR features rely on
Intel SGX enclaves for security against the server operator. SGX has
had a *long* run of side-channel vulnerabilities (Foreshadow 2018,
Plundervolt 2019, Load Value Injection 2020, ÆPIC Leak 2022, INCEPTION
2023, etc.). Each has been mitigated, but the underlying primitive
"the CPU keeps secrets from the OS" has proven brittle.

**Signal's response:** They've been clear-eyed about this. Their public
statements acknowledge SGX is *one layer* of defense, not the only
layer. The protocol-level guarantees (E2EE for messages, rate-limited
guesses for SVR) survive SGX compromise; SGX only hardens the
*metadata*-protection layer.

**The critique that remains:** Signal's contact discovery and SVR
features are *not* part of the protocol — they're operational properties
of Signal's specific deployment. A user of the Signal Protocol who
doesn't have SGX (anyone running OMEMO, or a Myrhiza port) doesn't get
contact-discovery privacy. So the SGX feature can't be borrowed by P2P
ports; only the protocol layer can.

## 4. Metadata exposure beyond sealed sender

**The critique** (Soltani-Greenwald, multiple academic security papers):
Sealed sender hides the sender identifier from Signal's *logs*, but
Signal's server still sees:

- IP addresses of all senders and recipients (TLS connection metadata).
- Timing of message arrivals and deliveries.
- Account-existence and online-status (whether a recipient is connected
  to the WebSocket).
- Group membership, indirectly, via zkgroup's server-side proof
  verification — Signal sees *a proof was submitted by some member*
  even though they don't see which member.

A nation-state observing Signal's ingress and a relay can correlate
`(IP, time, recipient)` tuples across many users and partially
reconstruct the social graph. Sealed sender is a real privacy win
*relative to no sealed sender*, but it's not the metadata-private answer
some users assume.

**Signal's position:** True. Sealed sender does what it does; it
doesn't solve traffic analysis. The Tor over Signal experiment
(briefly available in 2018 via Signal's Sealed Sender for new contacts)
was exploratory; nothing ever shipped to address traffic analysis at
production scale.

## 5. Governance opacity (CLA, no formal RFC process)

**The critique:** Signal's protocol changes happen *inside* Signal
Foundation. Specifications are published *after* implementation
decisions; there is no public RFC process, no external standards body
involvement, no community review window before deployment.

Compare to MLS, where the IETF MLS WG ran a 4+ year process to produce
RFC 9420, with external cryptographer review.

The CLA (Contributor License Agreement, see [`governance.md`](governance.md))
amplifies this — even contributors have signed away relicensing
rights, leaving Signal Foundation as the sole arbiter of future
versions of libsignal.

**Counter:** Signal's protocol designs *have* received academic
analysis after the fact (Alwen-Coretti-Dodis 2019 on Double Ratchet,
Cohn-Gordon et al. on PCS analysis, Bhargavan et al. on X3DH). The
absence of pre-deployment review hasn't yet produced a known-broken
protocol. But "hasn't yet" is the operative phrase — the pre-PQXDH
classical Signal Protocol is the most-analyzed E2EE protocol in the
world, but that analysis happened post-deployment.

## 6. The "Whittaker dragnet" advocacy

**The critique** (less common, more political): Whittaker's presidency
has shifted Signal Foundation's public voice toward strong anti-
surveillance-capitalism, anti-AI-deployment policy advocacy. Critics
argue this distracts from the technical mission and politicizes a
product that should be ideologically neutral.

**Counter-critique:** Whittaker's policy advocacy is part of *defending*
the mission. The legal/regulatory threats to E2EE (UK's Online Safety
Act, EU's Chat Control proposals, USA's EARN IT Act variants) are
existential. Engaging policy is mission-critical.

This critique is mostly tonal and depends on the reader's politics.
Worth noting that Signal under Marlinspike was also publicly anti-
surveillance; Whittaker has made it louder, not different.

## 7. The forking problem

**The critique:** Anyone can fork libsignal (AGPL-3.0 allows it) and
run a parallel Signal-Protocol-using service. But the *name* "Signal"
and the *network effect* are Signal Foundation's. A fork that wanted
to be more decentralized would have to:

- Run its own server (or be P2P).
- Re-implement contact discovery (no SGX in the wild for community
  forks).
- Have no interop with Signal's user base.

So in practice the AGPL openness is symbolic — no production fork
exists. (Molly, an alternative Android client, is the closest. Molly
uses Signal Foundation's server.)

**Signal's position:** They have not opposed forks. They have also not
provided any interop mechanism — Signal's server only talks to clients
authenticated through Signal's registration flow.

This is a critique of the *deployment*, not the *protocol*. Myrhiza
should care because it shows: AGPL-licensed reference code does not
automatically produce a healthy ecosystem of implementations.

## Implications for Myrhiza

- **Phone-number identity is the wrong default for a P2P runtime.** PNI
  + usernames is the right shape; require neither phone number nor
  network operator at the protocol layer.
- **Federation vs centralization is a governance choice with real
  iteration-speed costs.** Myrhiza's P2P stance commits to the
  decentralized end. Be honest about the cost: protocol changes will be
  harder to roll out than Signal's have been.
- **Don't rely on SGX or any other "trust the CPU" primitive for
  load-bearing privacy.** Make protocol-level guarantees that survive
  hardware compromise.
- **Metadata privacy is the unsolved problem.** Sealed sender helps a
  little; traffic analysis still wins for a sufficiently capable
  adversary. Myrhiza inherits this problem and has fewer tools to
  address it (no central operator to inject jitter / aggregate
  deliveries).
- **Open standards (IETF/W3C) vs go-it-alone is a real choice.**
  Signal's choice (go-it-alone) maximized iteration speed. MLS's choice
  (IETF) maximized interop. A spec author in Myrhiza picks per use-
  case. For *interop with apps outside Myrhiza*, standardization wins.
- **AGPL+CLA produces a healthy reference implementation but does not
  by itself produce a healthy ecosystem of forks.** Licensing alone is
  not a strategy.

## Sources

- "The Ecosystem is Moving" (Marlinspike, 2016-05-10): <https://signal.org/blog/the-ecosystem-is-moving/>
- "Beware the Matrix" (Hodgson, 2020-08): blog post on Matrix.org responding to Marlinspike
- Alwen, Coretti, Dodis — "The Double Ratchet: Security Notions, Proofs, and Modularization" (EUROCRYPT 2019): <https://eprint.iacr.org/2018/1037>
- Cohn-Gordon, Cremers, Garratt — "On Post-compromise Security" (CSF 2016): <https://eprint.iacr.org/2016/221>
- SGX vulnerability history: <https://en.wikipedia.org/wiki/Software_Guard_Extensions#List_of_SGX_vulnerabilities>
- Molly (Signal-Android fork): <https://molly.im/>
- Comparator: `prior-art/mls/governance.md` §IETF-process vs Signal's closed design
