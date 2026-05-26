**Date:** 2026-05-22
**Status:** active
**Subject:** Signal Protocol — production-grade end-to-end-encrypted messaging (X3DH/PQXDH key agreement, Double Ratchet session, Sealed Sender delivery, PNI/ACI identity, libsignal Rust implementation).

# Signal — prior art

The canonical production reference for E2EE messaging at scale. This corpus
covers the **protocol family** (X3DH + Double Ratchet + Sealed Sender + PQXDH),
the **identity model** (ACI / PNI dichotomy, sealed sender certificates), and
the **Rust implementation** (libsignal) — not the Signal mobile/desktop apps
themselves except where their UX choices feed back into the protocol design.

Signal is the production reference Myrhiza will hit twice on its critical path
to multi-device DMs:

1. **Multi-device identity** — Signal's PNI/ACI split + sealed sender is the
   most-deployed working answer to "user identity ≠ active signing key, hide
   delivery sender from the relay." See [`identity.md`](identity.md).
2. **Pre-key rotation under forward secrecy** — Signal's prekey bundle +
   one-time prekey + signed prekey rotation is the production answer to
   "rotate keys at scale without ever doing an interactive handshake." See
   [`protocol.md`](protocol.md).

Both citations are already in
`docs/prior-art/willow/open-problems.md:86-88,212-214`. This folder turns
those one-line references into something a future spec author can actually
read.

## Four things that are not Signal

Distinguish throughout this corpus, because conflating them produces wrong
licensing, threat-model, and adoption claims:

| Name | What it is |
|---|---|
| **Signal Protocol** | The cryptographic spec family (X3DH, Double Ratchet, PQXDH, Sealed Sender). Free to implement. CC-BY 4.0 on the spec text. |
| **libsignal** | The Rust core implementation Signal ships, with Java/Swift/Node.js (TypeScript-typed) bindings. AGPL-3.0. The reference impl, not the protocol itself. |
| **Signal Foundation** | The 501(c)(3) nonprofit that operates the service. Founded 2018-02-21 by Moxie Marlinspike and Brian Acton with a $50M loan from Acton. |
| **Signal app / service** | The mobile/desktop app + the centralized server operated by Signal Foundation. ~70M MAU as of January 2025 (Signal Foundation, "Signal is expensive," 2023-11-16; user count is per Wikipedia's Signal Messenger entry). |

Other services run their own implementations of the Signal Protocol: WhatsApp
(rolled out 2016-04-05), Google Messages RCS (2020-11), Facebook Messenger
Secret Conversations (2016-10), Skype Private Conversations (2018-01).
Implementations are closed-source; only the spec is shared.

## Key facts

| | |
|---|---|
| First public release | TextSecure (2010), unified Signal app on Android 2015-11 |
| Founding org | Open Whisper Systems (Marlinspike, January 2013) → Signal Foundation (2018-02-21) |
| Current CEO | None — Meredith Whittaker, president (appointed September 2022) |
| Previous CEO | Moxie Marlinspike (stepped down 2022-01-10; remains on board) |
| Funding | Donations + $50M initial loan from Brian Acton; $50M projected annual operating cost by 2025 (per 2023-11 blog post) |
| MAU | ~70M (January 2025, per Wikipedia) |
| Source code (apps) | AGPL-3.0 |
| Source code (libsignal) | AGPL-3.0 |
| Spec text | CC-BY 4.0 (X3DH, Double Ratchet, PQXDH all on signal.org/docs) |
| Reference impl repo | [signalapp/libsignal](https://github.com/signalapp/libsignal) — 5.8k stars, primarily Rust |

## Reading order

1. **[`README.md`](README.md)** (this file) — orientation, distinguish the four "Signals."
2. **[`protocol.md`](protocol.md)** — X3DH + Double Ratchet, the core cryptographic loop.
3. **[`identity.md`](identity.md)** — ACI/PNI, sealed sender, sender certificates. *The load-bearing file for Myrhiza.*
4. **[`post-quantum.md`](post-quantum.md)** — PQXDH (2023-09 announcement), Kyber-1024 hybrid, ratchet-level PQ gap.
5. **[`libsignal.md`](libsignal.md)** — the Rust crate layout, FFI bindings, AGPL-3.0 implications.
6. **[`groups.md`](groups.md)** — zkgroup (KVAC anonymous credentials, 2019-12 preview), private group infrastructure. Why Signal stayed on Double Ratchet pairwise for groups instead of moving to MLS.
7. **[`server.md`](server.md)** — the centralized service: SGX enclaves, contact discovery, SVR, ORAM. Where the protocol meets production.
8. **[`history.md`](history.md)** — Axolotl (2013) → Double Ratchet rename (2016-03) → PQXDH (2023-09). Lineage and renames.
9. **[`governance.md`](governance.md)** — Signal Foundation, Whittaker presidency, funding model, Acton loan terms.
10. **[`comparisons.md`](comparisons.md)** — vs MLS (groups), vs iMessage PQ3 (Apple's PQ rival), vs OTR (predecessor).
11. **[`critiques.md`](critiques.md)** — third-party criticism: phone-number-rooted identity, centralization, federation refusal ("The ecosystem is moving"), metadata exposure.
12. **[`open-problems.md`](open-problems.md)** — what Signal does NOT solve (or what its solution has costs Myrhiza would not accept).
13. **[`lessons.md`](lessons.md)** — **the decision file.** What Signal's design validates for Myrhiza, what to avoid, what to borrow.
14. **[`glossary.md`](glossary.md)** — ACI, PNI, prekey bundle, sealed sender, etc.

## How to use

When designing a Myrhiza spec that touches multi-device identity, key rotation,
sender anonymity, or post-quantum cryptography: read [`lessons.md`](lessons.md)
first, then the relevant subsystem file. Cite Signal's choice as the production
reference and name your runner-up paradigm if you reject it.

**Framing disclosure.** These docs are written from a Myrhiza-as-P2P-runtime
stance — most "Implications for Myrhiza" sub-sections frame Signal's choices
through the lens of "we don't have a centralized server, so what survives?"
Signal's design is deeply shaped by *having* a centralized server it can lean
on for sender authentication, contact discovery, and key distribution; Myrhiza
will not have that lever and must reach the same security properties without
it. Future readers auditing whether the P2P stance is itself the right primitive
should weigh the corpus accordingly: it's a learn-from-Signal-into-P2P artifact,
not a neutral catalog.

## Sources

- libsignal repository: <https://github.com/signalapp/libsignal>
- Signal-Android repository: <https://github.com/signalapp/Signal-Android>
- Signal Protocol specifications: <https://signal.org/docs/>
- "Signal is expensive" (annual operating cost): <https://signal.org/blog/signal-is-expensive/> (2023-11-16)
- "The Ecosystem is Moving" (federation refusal): <https://signal.org/blog/the-ecosystem-is-moving/> (2016-05-10)
- Wikipedia: Signal Messenger — <https://en.wikipedia.org/wiki/Signal_Messenger>
- Wikipedia: Signal Protocol — <https://en.wikipedia.org/wiki/Signal_Protocol>
- Wikipedia: Double Ratchet Algorithm — <https://en.wikipedia.org/wiki/Double_Ratchet_Algorithm>
