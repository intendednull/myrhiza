**Date:** 2026-05-22
**Status:** active
**Subject:** Signal vs other E2EE protocols — MLS, iMessage PQ3, OTR, Matrix Olm/Megolm, vodozemac.

# Signal — comparisons

This file places Signal in the landscape of E2EE messaging protocols. The
goal is honest comparison, not endorsement of any one.

## Signal Protocol vs MLS (RFC 9420)

The relationship Myrhiza needs to understand most carefully. See also
`prior-art/mls/comparisons.md`.

| Property | Signal Protocol (Double Ratchet) | MLS (TreeKEM) |
|---|---|---|
| Group model | Pairwise fan-out (N ciphertexts per send) | Tree-shared (1 ciphertext per send) |
| Forward secrecy | Per-message | Per-epoch |
| Post-compromise security | Per-round-trip | Per-commit |
| Member add cost | O(N) — new pairwise sessions | O(log N) — leaf insert |
| Member remove cost | O(N) — drop sessions | O(log N) — blank leaf, update tree |
| Standardization | None (CC-BY spec on signal.org) | RFC 9420 (IETF) |
| Production at scale | Yes (since 2014; ~70M MAU Signal + ~2B WhatsApp) | Yes (since 2022; Discord DAVE, Webex, RingCentral) |
| Reference impls | libsignal (AGPL-3.0), vodozemac (MIT for Olm) | OpenMLS (MIT+Apache-2.0), MLS++ |
| Authority for 1:1 DMs | Definitive (canonical) | Awkward (MLS optimizes for groups; 1:1 works but is overkill) |
| Authority for large groups | Costly at large N | Definitive — designed for it |

**Common misframing to avoid:** "MLS replaces the Signal Protocol." It
doesn't. MLS replaces the *group messaging* layer; for 1:1 DMs, the
Double Ratchet remains the right answer (cheaper per send, finer-grained
forward secrecy). Discord DAVE, the largest MLS deployment, uses MLS
for the *group call* key but keeps Double-Ratchet-shaped pairwise
sessions for DMs.

The Myrhiza decision is: which protocol family for which message type?
A pluralist answer ("Double Ratchet for DMs, MLS for groups") is
defensible and matches production reality at most large E2EE deployments.

## Signal Protocol vs iMessage PQ3

Apple shipped PQ3 on 2024-02-21, ~5 months after Signal's PQXDH
(2023-09-19). PQ3 is positioned explicitly as a Signal-rival design.

| Property | Signal PQXDH | iMessage PQ3 |
|---|---|---|
| Initial key agreement PQ | Yes — Kyber-1024 hybrid | Yes — Kyber-1024 |
| Ratchet-step PQ | No — DH ratchet steps are classical only | Yes — Kyber-768 used for periodic rekeying throughout session |
| Apple's framing | "Level 2" (Apple's categorization) | "Level 3" |
| Forward secrecy granularity | Per-message (Double Ratchet) | Per-message |
| Production deployment | Signal (~70M MAU) + WhatsApp PQXDH-aligned | iMessage (all Apple users on iOS 17.4+) |
| Spec / implementation | Open spec (CC-BY), AGPL reference impl | Closed spec, Apple-only implementation |

**Apple's "Level 3" claim is the live disagreement.** Apple frames PQ3
as strictly stronger because it mixes PQ KEM material into ongoing
session keys, not just the initial root. This is genuine — a quantum
adversary recording today and breaking X25519 later can still recover
*ratchet-step* message keys from Signal but cannot recover PQ-rekeyed
session keys from PQ3.

**But:** PQ3's PQ rekeying cadence is "periodic," not per-message. The
classical ratchet still operates between PQ rekey events. So the gap
is "PQ at every Nth message" (PQ3) vs "PQ only at session setup"
(Signal) — not "PQ everywhere" vs "PQ never." Both are intermediate
positions. PQ3 is closer to fully-PQ; Signal is closer to fully-
classical.

**Implementation cost:** PQ3's per-message overhead is materially
larger (the Kyber ciphertexts add ~1KB per rekey). For Signal's
fan-out group messaging this would be prohibitive at scale; for
iMessage (which has its own group cipher) it's tolerable. Signal's
choice to skip ratchet-level PQ is partly an engineering pragmatism
about per-recipient bandwidth.

## Signal Protocol vs OTR (predecessor)

OTR ("Off-the-Record Messaging," Borisov-Goldberg-Brewer 2004) is the
intellectual ancestor of the Double Ratchet's DH ratchet. The lineage:

- OTR provided forward secrecy via per-message DH key exchange.
- OTR required *both parties online* for the key exchange — no
  asynchronous messaging.
- OTR's symmetric ratchet was minimal; it relied on the per-message
  DH for forward secrecy, which made it heavy.

Signal's Double Ratchet generalized OTR's design:

- Asynchronous messaging via X3DH's prekey bundle (Bob can be offline).
- Two ratchets in parallel — symmetric for per-message FS, DH for PCS
  on round-trips. This is much cheaper than OTR's per-message DH.
- Header-encryption variant (rarely used in production) hides session
  metadata from passive observers.

**OTR's main fork has effectively merged with Signal.** Mainline OTR
clients (Pidgin, Adium) are deprecated; new XMPP encryption deployments
use OMEMO, which itself is a port of the Signal Protocol to XMPP.

## Signal Protocol vs Matrix Olm + Megolm + vodozemac

Matrix uses two protocols:

- **Olm** — a Double-Ratchet-derived protocol for 1:1 conversations.
  Very similar to Signal's Double Ratchet, with minor parameter
  differences (different KDF labels, different message format).
- **Megolm** — a *symmetric* ratchet for group messaging. Trades
  post-compromise security (Megolm doesn't have it) for simpler group
  semantics (no fan-out; one cipher per group).

**vodozemac** is Matrix.org's MIT-licensed Rust implementation of both
Olm and Megolm. The relevant contrast:

| Property | libsignal | vodozemac |
|---|---|---|
| License | AGPL-3.0 | MIT |
| Protocol target | Signal Protocol family | Olm + Megolm (Signal-derived but distinct) |
| FFI bindings | Java, Swift, Node | Python, JS (via wasm-bindgen), C |
| Production scale | Signal + WhatsApp scale | All Matrix homeservers (Element, Synapse) |
| Wire compatibility with libsignal | No — different message format | No |

For Myrhiza, vodozemac is the more practical reference impl to study:
MIT license means it can actually be linked or borrowed from. The
protocol differences from libsignal are small for 1:1 messaging but
significant for groups (Megolm's lack of PCS is a real
divergence).

Matrix is also moving to MLS for the next-generation group encryption
("MatrixRTC" group calls already use MLS; the message-encryption
migration is in progress). Megolm is, in a sense, the last large
deployment of pre-MLS symmetric-ratchet groups.

## Signal Protocol vs OMEMO

OMEMO is an XEP (XMPP extension protocol) for E2EE over XMPP. It is
mechanically a port of the Signal Protocol (X3DH + Double Ratchet) to
XMPP's bundle-discovery + stanza-routing semantics.

| Property | Signal | OMEMO |
|---|---|---|
| Centralized server? | Yes (Signal Foundation) | No — federated XMPP |
| Protocol | Signal Protocol | Signal Protocol (XMPP-bound) |
| Multi-device | Same ACI fanned out to devices | Same JID, per-device key per JID |
| Sealed sender | Yes | No (XMPP routing layer sees sender) |

OMEMO is the closest "Signal Protocol in a federated/decentralized
setting" deployment. It has been running for a decade in production
XMPP. Worth a look for Myrhiza specifically because it shows how
multi-device + prekey bundle deployment works without a central
operator — bundle discovery uses XMPP's `pubsub` mechanism.

## Signal Protocol vs WhatsApp's deployment

WhatsApp uses the Signal Protocol *spec* with its own closed-source
implementation. As of 2024-2025 the integration appears current —
WhatsApp added PQXDH alignment with Signal's 2023-09 release within
months.

Differences worth knowing:
- WhatsApp's implementation is *closed* — you cannot audit it.
- WhatsApp's *backups* are encrypted with a key managed by Meta. If
  enabled, backups defeat E2EE retroactively. Signal does not offer
  cloud backups for this reason; only on-device backups exist.
- WhatsApp uses *Signal's protocol spec* but does not use *libsignal*.
  Their implementation is independent. Bugs and behavior may differ.

The pattern is: protocol shared, implementation private, deployment
incentives diverge. WhatsApp's privacy posture is materially weaker
than Signal's despite using the same protocol.

## Implications for Myrhiza

- **The "pluralist Signal-Protocol + MLS" deployment posture is
  production-validated.** Don't try to make MLS do everything or
  Signal Protocol do everything. Pick per use-case.
- **vodozemac is the licensing-clean reference impl.** If Myrhiza
  ever wants Rust code that resembles Signal's Double Ratchet,
  vodozemac is the place to look, not libsignal.
- **The "ratchet-level PQ" gap is genuine.** PQ3's design is
  arguably stronger than PQXDH. A Myrhiza PQ posture should be
  honest about which level it's targeting.
- **OMEMO's federated deployment of the Signal Protocol matters
  more than Signal's centralized deployment, for Myrhiza purposes.**
  The bundle-discovery-via-pubsub pattern is the right shape for a
  P2P-runtime adaptation. Study OMEMO's bundle-rotation UX over the
  past decade.
- **WhatsApp's pattern (protocol-shared, implementation-private)
  is what large platforms tend to do.** A Myrhiza-app developer who
  picks the Signal Protocol gets the spec but is on the hook for
  the implementation — either re-implement, license from libsignal
  (AGPL-3.0), or use vodozemac (MIT).

## Sources

- "Quantum Resistance and the Signal Protocol" (2023-09-19): <https://signal.org/blog/pqxdh/>
- Apple PQ3 announcement: <https://security.apple.com/blog/imessage-pq3/> (2024-02-21)
- OMEMO XEP-0384: <https://xmpp.org/extensions/xep-0384.html>
- Matrix vodozemac: <https://github.com/matrix-org/vodozemac>
- Matrix Megolm spec: <https://gitlab.matrix.org/matrix-org/olm/-/blob/master/docs/megolm.md>
- Borisov, Goldberg, Brewer — "Off-the-Record Communication" (2004): <https://otr.cypherpunks.ca/otr-wpes.pdf>
- WhatsApp Security Whitepaper: <https://www.whatsapp.com/security/WhatsApp-Security-Whitepaper.pdf>
- Comparator: `prior-art/mls/comparisons.md`
- Comparator: `prior-art/mls/protocol.md` (TreeKEM mechanism)
