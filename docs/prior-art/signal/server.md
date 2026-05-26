**Date:** 2026-05-22
**Status:** active
**Subject:** Signal's server-side infrastructure — SGX enclaves, contact discovery, SVR, ORAM. Where the protocol meets production.

# Signal server — the operational substrate

The Signal Protocol is decentralized in theory. The Signal service is
**centralized in production** — one operator (Signal Foundation), one
ingress/egress (signal.org domains), one trust root (Signal CA for sealed-
sender certificates). This file documents the server-side components
relevant to understanding the protocol's actual deployment.

Source: <https://github.com/signalapp/Signal-Server> (Java/Kotlin, AGPL-3.0,
~13k stars, primarily Whisper Technologies developers).

## Why the server exists at all

E2EE protocols don't strictly require a centralized server — Matrix /
Element runs federated; OMEMO over XMPP runs decentralized; classic OTR
runs peer-to-peer. Signal chose centralization deliberately. Moxie
Marlinspike's defense is in "The ecosystem is moving" (2016-05-10): once
federated, protocols cannot iterate. Signal iterates — Sealed Sender, PNI,
PQXDH, key transparency, group calls, stories all shipped in years 5-15 of
the protocol's life. Marlinspike argues no federated protocol could have
shipped them at scale.

This is a defensible engineering position and a controversial political
one. See [`critiques.md`](critiques.md) for the rebuttals.

## What the server does

In rough order of "things the Signal-Server repo implements":

### 1. Account registration + phone-number verification

The server is the registration authority. A new client registers a phone
number, receives an SMS or voice-call code, verifies, and is issued an
ACI + PNI. The server runs the registration-fee budget line that costs
Signal $6M/year (largest single infrastructure line, per the 2023-11
"Signal is expensive" blog post).

### 2. Prekey storage and distribution

Clients upload their prekey bundles (`IK`, `SPK`, `OPK[]`). The server
hands them out one-at-a-time to senders running X3DH/PQXDH. The server
trusts itself to:
- Hand out each OPK exactly once.
- Track the user's OPK pool size and ping the client to refill when low.
- Serve the most-recent `SPK` and `IK`.

A malicious or compromised server could replay OPKs to multiple
initiators (weakening forward secrecy) or substitute an `IK` (full
MITM). The latter is mitigated by **safety numbers** (see below).

### 3. Message queuing

Signal does not store decrypted message content — it can't (E2EE). What
it does store: the encrypted envelope, awaiting recipient delivery.
Messages are deleted from the queue once delivered. Offline-recipient
queues can grow large; Signal places limits.

### 4. Contact discovery

When a new user wants to find which of their contacts are on Signal, the
client uploads phone-number hashes and the server returns which ones are
registered. **Doing this naively leaks the user's address book to the
server.** Signal's solution: contact discovery runs in an **SGX enclave**,
which the client remote-attests before submitting hashes. The enclave
processes the query against the user database in-enclave, returns
results, and the server operator cannot see the queries because the
enclave's memory is opaque even to the host OS.

Plus Path ORAM (2022-08-19 deployment) which obfuscates the *memory access
patterns* of the enclave so that timing/cache-side-channel analysis can't
reconstruct which numbers were queried. This is the "555,555× faster than
linear scan" optimization in the 2022 blog post — switching from O(N) to
O(log N) per query while keeping access-pattern privacy via ORAM.

### 5. Secure Value Recovery (SVR)

Announced 2019-12-19. The problem: users forget PINs and lose access to
their account-state (contacts, profile, settings). A naive "PIN-recovery"
scheme requires the server to know the PIN, which is unacceptable.
Signal's SVR:

- User picks a passphrase (PIN). Argon2 stretches it into an auth key + a
  master encryption key.
- The master key encrypts client state. Encrypted state is uploaded to
  the server.
- An SGX enclave on the server enforces a rate-limit: 5 PIN guesses
  before the encrypted state is permanently deleted. The enclave
  remembers attempt counts even if the server operator reboots — backed
  by a Raft-replicated consensus across enclaves in multiple data
  centers.
- Even with full server compromise, an attacker has at most 5 guesses
  per user's PIN before losing access to the encrypted state.

The Raft + SGX + Argon2 + rate-limiting combination is genuinely novel
infrastructure. Signal-Server's `enclave/` directory is the
implementation. The threat model includes "Signal Foundation itself is
adversarial" — operationally, Signal cannot decrypt SVR state even if
they wanted to.

### 6. Push notification routing

Mobile push notifications go through Apple APNS and Google FCM. Signal
uses these only to wake the client app; the actual message content is
fetched from Signal's queue over a separate authenticated channel. APNS
and FCM see "Signal wants to wake device X" with no message content.

## Safety numbers — out-of-band identity verification

A safety number is a 60-digit fingerprint derived from the two parties'
identity keys. Both clients display the same number; the users compare it
out-of-band (in person, on a phone call, etc.) to confirm they have the
right keys.

This is the user-facing answer to "what if the server MITM-substitutes
identity keys?" When safety numbers match, MITM is detectably impossible.
When they change (e.g., the contact reinstalled the app and generated a
new identity key), Signal warns the user and asks them to re-verify.

Safety numbers are the *only* thing protecting against a Signal-server-
substituting-keys attack. A user who never compares safety numbers is
trusting Signal's server.

## Key Transparency (in development)

Signal has announced plans for a key-transparency layer that would let
users verify the server is showing them consistent identity keys for a
recipient across queries. As of 2026-05-22 this is in development; no
production deployment is publicly confirmed. Once shipped, it weakens the
"trust Signal's server to not substitute keys" assumption to "trust
that *someone* watches the key-transparency log for substitutions."

Comparable to Google's Certificate Transparency. The underlying technique
is a publicly-verifiable Merkle-tree log of `(identifier → identity_key)`
records.

## Production scale

Per the 2023-11 "Signal is expensive" blog post:

- ~50 full-time Signal Foundation staff.
- $14M/year infrastructure cost.
- $19M/year personnel.
- $50M/year projected total by 2025.
- ~70M monthly active users (per Wikipedia, January 2025).

Compare to competitor headcount: LINE ~3,100 employees, KakaoTalk ~4,000.
Signal's small headcount is a feature of the architecture — the
centralized service is simple to operate compared to the legal/regulatory
overhead of social-media platforms.

## Implications for Myrhiza

- **Myrhiza will not have any of the server-side machinery.** No
  registration authority. No SGX enclave for contact discovery. No
  Raft-replicated SVR. No safety-number out-of-band re-verification UX
  built on top of a central key store.
- **Translate each server function to a P2P primitive:**
  - **Registration / phone-number verification** → out of scope for
    Myrhiza (it's a property of the app, not the runtime). An app that
    wants phone-number-rooted identity needs its own SMS gateway.
  - **Prekey storage** → relay-with-storage capability OR a DHT-based
    prekey-bundle store. Either way, the OPK-handout-faithfully property
    is harder to enforce without a central trusted operator.
  - **Message queuing** → already in Willow's spec via relay
    capability-doc; offline-recipient queuing is in scope.
  - **Contact discovery** → genuine open problem. PNI without a central
    server is the seal-gift-wrap pattern from Willow. Privacy-preserving
    P2P discovery is an active research problem (see Tor onion services
    for one shape; see Apple's Private Set Intersection variants for
    another).
  - **SVR-equivalent (key recovery)** → research-grade in P2P. The
    `(SGX enclave + Raft + rate-limiting)` triangle has no clean P2P
    analogue. A user who loses their primary device in Myrhiza loses
    their identity unless they distributed key shares to trusted
    contacts (Shamir secret sharing) or to multiple personally-owned
    devices.
- **Key transparency is more important in P2P than centralized.** In
  Signal, the key-transparency log is a backstop against one adversary
  (the server). In Myrhiza, every peer that participates in a topic
  could be substituting keys for anyone they relay messages to. A
  per-user key-transparency record is structurally necessary, not
  optional.
- **The "small headcount because centralized" argument is real and
  is the load-bearing operational argument for centralization.** A
  P2P system pushes operational cost to users. Be honest about this in
  any Myrhiza spec — users running their own relays/workers *are* the
  ops team. UX must absorb this.

## Sources

- Signal-Server repository: <https://github.com/signalapp/Signal-Server>
- "Signal is expensive" (2023-11-16): <https://signal.org/blog/signal-is-expensive/>
- "Technology Preview: Sealed Sender" (2018-10-29): <https://signal.org/blog/sealed-sender/>
- "Technology Preview: Secure Value Recovery" (2019-12-19): <https://signal.org/blog/secure-value-recovery/>
- "Building a Faster ORAM Layer for Enclaves" (2022-08-19): <https://signal.org/blog/building-faster-oram/>
- "The Ecosystem is Moving" (Marlinspike's federation argument, 2016-05-10): <https://signal.org/blog/the-ecosystem-is-moving/>
- Comparator: `prior-art/willow/networking.md` (relay capability model)
- Comparator: `prior-art/iroh/` (P2P transport that could host similar primitives)
