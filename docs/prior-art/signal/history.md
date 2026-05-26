**Date:** 2026-05-22
**Status:** active
**Subject:** Lineage of the Signal Protocol — TextSecure (2010) → Axolotl Ratchet (2013) → Double Ratchet rename (2016) → X3DH spec (2016-11) → Sealed Sender (2018) → PNI (2022) → PQXDH (2023).

# Signal — history and lineage

## Pre-Signal: TextSecure (2010-2014)

The protocol's ancestor was TextSecure, an Android SMS-encryption app
released by Whisper Systems in 2010. Whisper Systems was bought by
Twitter in 2011; Marlinspike open-sourced TextSecure and RedPhone (a voice-
encryption companion) in 2011-12 around the time he left Twitter to found
Open Whisper Systems.

The first-generation TextSecure protocol used **OTR-style ratcheting** for
SMS-shaped messages. It was not the Double Ratchet.

## The Axolotl Ratchet — first public version (2013)

In 2013, Marlinspike and Trevor Perrin designed what was internally called
the **Axolotl Ratchet** — named after the self-healing salamander, because
the ratchet has post-compromise security (a "self-healing" property after
key compromise).

The design combined:
- The **DH ratchet** from OTR (Off-the-Record Messaging, Borisov-Goldberg-
  Brewer 2004).
- A **symmetric-key ratchet** modeled after the Silent Circle Instant
  Messaging Protocol.

The combination gave per-message forward secrecy (from the symmetric
ratchet) + per-round-trip post-compromise security (from the DH ratchet).

The Axolotl Ratchet was introduced to TextSecure in February 2014 (per
Wikipedia's Double Ratchet Algorithm article).

## The Signal app — unified TextSecure + RedPhone (2014-2015)

- 2013-01: Open Whisper Systems founded.
- 2014-07: Signal for iOS launches (initially RedPhone-only).
- 2015-03: Signal for iOS adds messaging.
- 2015-11: Android's TextSecure + RedPhone unified into Signal for Android.

The protocol design stayed the same; what changed was the brand and the
unification across the two communication modes.

## Specification publication — X3DH + Double Ratchet (2016)

In 2016, Marlinspike and Perrin published formal specifications:

- **X3DH spec, revision 1** (2016-11-04). Marlinspike + Perrin (editor).
- **Double Ratchet spec, revision 1** (November 2016). Perrin (editor) +
  Marlinspike. The "Axolotl Ratchet" was renamed to the **Double Ratchet
  Algorithm** in March 2016 — Perrin had argued the old name was
  confusing because it referred to both the algorithm and (in some
  documentation) the entire Signal Protocol.

The specs are CC-BY 4.0, hosted at <https://signal.org/docs/>. Both
revisions have continued: Double Ratchet is at revision 4 (2025-11-04),
authored by Perrin + Marlinspike + Schmidt (Rolfe Schmidt joined as
co-author in revision 3).

This is the first time the Signal Protocol existed *as a spec separate
from its implementation*. WhatsApp's adoption (announced 2014-11,
universal rollout 2016-04-05) drove much of the formalization pressure.

## WhatsApp adoption (2014-2016)

- 2014-11: WhatsApp announces partnership with Open Whisper Systems.
- 2016-04-05: Universal rollout — "end-to-end encryption to every form of
  communication."

WhatsApp's implementation is closed-source; only the spec is shared with
Open Whisper Systems. The trust model: WhatsApp publishes a technical
white paper describing how the protocol is integrated, and the protocol
spec is published, but you cannot audit WhatsApp's actual implementation
of either.

This is the largest deployment of the Signal Protocol by far —
~2B+ WhatsApp users vs ~70M Signal users.

## Signal Foundation founded (2018-02-21)

Open Whisper Systems was a for-profit C-corp (Quiet Riddle Ventures LLC).
On 2018-02-21, Brian Acton (co-founder of WhatsApp, departed Facebook
2017-09) co-founded **Signal Foundation** as a 501(c)(3) nonprofit with
Marlinspike, donating an initial **$50 million loan** (per Wikipedia).
The Foundation took over Signal's development; Open Whisper Systems
formally shut down soon after.

Acton's stated motivation: a desire to ensure Signal would not be
subject to commercial pressure to monetize via advertising. Acton joined
the foundation board as Executive Chairman.

## Sealed Sender (2018-10-29)

The 2018 release was a major addition: Signal could now deliver messages
without learning the sender's identity from the envelope. See
[`identity.md`](identity.md) for the mechanism.

This was the first major *protocol* change since the Double Ratchet spec
publication in 2016. It was non-backwards-compatible; clients without
sealed-sender support fell back to the older envelope format.

## zkgroup (2019-12-09 preview)

Signal's Private Group System was announced as a "Technology Preview" in
December 2019. The Chase-Perrin-Zaverucha paper followed in 2020 (CCS
2020). Production rollout for the new groups (called "Groups V2" or
"GroupsV2" in libsignal) was through 2020-2021. See [`groups.md`](groups.md).

## SVR — Secure Value Recovery (2019-12-19)

Announced one week after zkgroup. SVR shipped to production in
2019-2020. SGX + Raft + Argon2 is the substrate. See
[`server.md`](server.md).

## Marlinspike departs as CEO (2022-01-10)

Marlinspike announced his departure as CEO on 2022-01-10. He remained on
the Signal Foundation board. Brian Acton served as interim CEO during
the search.

## Meredith Whittaker appointed President (2022-09)

Whittaker (formerly Google AI ethics researcher, co-founder of the AI Now
Institute) was appointed as President in September 2022. Notably, the
position is **President**, not CEO — Signal has not formally replaced the
CEO role since Marlinspike's departure.

## PNI rollout (2022-11-15 announcement)

The Phone Number Identifier — separating user identity (ACI) from phone-
number-based discovery (PNI) — was announced 2022-11-15. The rollout was
gradual over multiple years. See [`identity.md`](identity.md).

## PQXDH (2023-09-19)

Post-quantum upgrade to X3DH. Hybrid X25519 + CRYSTALS-Kyber-1024. By
the announcement date, the protocol had been silently rolling out in
client apps. See [`post-quantum.md`](post-quantum.md).

This was Signal's first public commitment to post-quantum cryptography.
Apple's iMessage PQ3 followed 5 months later (2024-02-21) with a more
aggressive design (ratchet-level PQ). See [`comparisons.md`](comparisons.md).

## Usernames + phone-number privacy GA (2024-02-20)

The username feature shipped to general availability in February 2024.
Combined with PNI, this allowed users to communicate without exposing
phone numbers to non-contacts.

## Key Transparency (announced 2025, in development)

A planned key-transparency log to let users verify identity-key
consistency across queries. Comparable to Google's Certificate
Transparency. Not in production as of 2026-05-22.

## Lineage diagram

```
2004: OTR (Off-the-Record Messaging)
    │
    │  (DH ratchet pattern)
    ▼
2010: TextSecure (OTR-shaped SMS encryption)
    │
    │  Marlinspike, Whisper Systems → Twitter → OWS
    ▼
2013: Axolotl Ratchet
    │ (combines OTR's DH ratchet + Silent Circle's symmetric ratchet)
    ▼
2014: TextSecure v2 = Signal Protocol v1
    │
    │  WhatsApp adoption, formalization pressure
    ▼
2016-03: Axolotl Ratchet renamed "Double Ratchet Algorithm"
2016-04: WhatsApp universal rollout
2016-11: X3DH spec rev 1 published
2016-11: Double Ratchet spec rev 1 published
    │
    ▼
2018-02: Signal Foundation founded (Acton + Marlinspike)
2018-10: Sealed Sender released
    │
    ▼
2019-12: zkgroup preview / SVR preview
2020:    zkgroup + SVR shipped to production
    │
    ▼
2022-01: Marlinspike departs as CEO
2022-09: Whittaker appointed President
2022-11: PNI announcement
    │
    ▼
2023-09: PQXDH released
    │
    ▼
2024-02: Usernames + phone-number-hiding GA
2024-02: Apple ships iMessage PQ3 (rival post-quantum protocol)
    │
    ▼
2025:    Key Transparency announced (in development)
```

## Sources

- Wikipedia: Signal Messenger — <https://en.wikipedia.org/wiki/Signal_Messenger>
- Wikipedia: Signal Protocol — <https://en.wikipedia.org/wiki/Signal_Protocol>
- Wikipedia: Double Ratchet Algorithm — <https://en.wikipedia.org/wiki/Double_Ratchet_Algorithm>
- Signal blog: <https://signal.org/blog/>
- Acton announcement of Signal Foundation: <https://signal.org/blog/signal-foundation/> (2018-02-21)
- "The Ecosystem is Moving" (Marlinspike, 2016-05-10): <https://signal.org/blog/the-ecosystem-is-moving/>
