**Date:** 2026-05-22
**Status:** active
**Subject:** Signal's core cryptographic protocols — X3DH key agreement + Double Ratchet session — for 1:1 messaging.

# Signal Protocol — X3DH + Double Ratchet

Two protocols compose into the Signal Protocol's 1:1 messaging session:

1. **X3DH** (Extended Triple Diffie-Hellman) — the *initial* key agreement.
   Establishes a shared root key when Alice messages Bob for the first time
   and Bob may be offline. Authors: Moxie Marlinspike + Trevor Perrin
   (editor), revision 1 (2016-11-04). PQXDH supersedes X3DH for new sessions
   since 2023-09 — see [`post-quantum.md`](post-quantum.md).
2. **Double Ratchet** — the *ongoing* per-message key derivation. Provides
   forward secrecy (past keys can't be recovered from a current key) and
   post-compromise security (future keys recover from a current key
   compromise). Authors: Trevor Perrin (editor) + Moxie Marlinspike (Rolfe
   Schmidt added on revision 3+), revision 4 (2025-11-04). Created originally
   as the "Axolotl Ratchet" in 2013; renamed in March 2016.

## X3DH — initial key agreement

### Goal

Establish a shared secret between Alice (initiator, online) and Bob (responder,
possibly offline). The asynchronous case is the design constraint: Alice cannot
wait for Bob to come online before sending the first message.

### Mechanism

Bob pre-publishes a **prekey bundle** to the Signal server:

- `IK_B` — Bob's long-term **identity key** (X25519 or X448).
- `SPK_B` — Bob's **signed prekey** (medium-lived, signed by `IK_B`). Rotated
  on a cadence (Signal rotates roughly weekly; the spec doesn't mandate).
- `OPK_B` (zero or many) — **one-time prekeys**. Each is consumed by exactly
  one X3DH run, then deleted.

Alice fetches a bundle from the server (one OPK if available), then computes
four Diffie-Hellman exchanges:

```
DH1 = DH(IK_A,  SPK_B)
DH2 = DH(EK_A,  IK_B)
DH3 = DH(EK_A,  SPK_B)
DH4 = DH(EK_A,  OPK_B)   -- omitted if no OPK available
```

`EK_A` is Alice's per-session **ephemeral key**. The shared secret is
`SK = HKDF(DH1 || DH2 || DH3 || DH4)`, used to seed the Double Ratchet's root
key.

Alice attaches her identity key, ephemeral key, and the prekey-bundle
identifiers in the first ciphertext she sends. Bob, when he comes online,
fetches the message, recomputes the same four DHs from his side, and derives
the same `SK`.

### Security properties

- **Forward secrecy** — yes, because `EK_A` is ephemeral and deleted after
  use, and `OPK_B` is one-shot.
- **Authentication** — mutual, via the long-term identity keys.
- **Deniability** — yes (cryptographic deniability): the transcript does not
  contain non-repudiable signatures over message content. Either party could
  plausibly have forged the transcript.
- **Replay** — if Alice's first message is replayed and Bob has not deleted
  the consumed OPK, Bob accepts it. The OPK exhaustion case (no OPKs left;
  X3DH runs without DH4) weakens forward secrecy modestly because future
  sessions with the same Alice can be linked through `SPK_B` reuse.

### Curve choice

The spec allows X25519 or X448 (not mixed within a protocol run). Signal's
production deployment uses X25519. Hash function is SHA-256 (or 512); session
key is 32 bytes via HKDF.

## Double Ratchet — ongoing session

### Goal

After X3DH establishes the root key, every subsequent message uses a *fresh*
key. Forward secrecy is per-message (not per-epoch as in MLS) — losing the
key for message 47 does not compromise message 46.

### Mechanism — two ratchets in one

**Symmetric-key ratchet** (per-message): a sending **chain key** `CK_send` is
advanced one step per message:

```
(CK_send', MK) = KDF(CK_send)
encrypt(MK, plaintext)
```

The chain key is deleted after deriving each message key. Past message keys
can't be recovered from `CK_send'`.

**Diffie-Hellman ratchet** (per round-trip): every time Alice receives a new
DH public key from Bob in a message header, she:

1. Performs a DH with her current `DHR` keypair to derive a new root key and
   a new receiving chain.
2. Generates a *fresh* DHR keypair, performs another DH with Bob's new
   public key, and derives a new sending chain.
3. Bob does the symmetric operation when he sees Alice's next message.

This is the "ping-pong": each side advances the DH ratchet on every
round-trip. It provides **post-compromise security** — if Mallory steals
Alice's keys at time T, the next ratchet step after T mixes in fresh DH
material that Mallory does not know, and future messages become safe again.

### Out-of-order messages

The DH ratchet introduces sequence problems: if Alice sends messages 1–5
under DH ratchet step 7, and Bob receives them out of order, he must keep
the chain key for step 7 alive long enough to derive message keys 1–5. The
spec defines a **message key cache** of size `MAX_SKIP` (typically 1000 per
chain) that holds out-of-order message keys for later decryption.

Skipped keys eventually expire — a peer that never receives the missing
message holds the cached key indefinitely until cache pressure forces
eviction.

### Header encryption variant

The optional **header encryption** variant encrypts the DH public key and
message sequence numbers in each header, preventing a passive eavesdropper
from determining whether two ciphertexts belong to the same session. Signal
production does NOT use header encryption in the wire protocol — sealed
sender accomplishes a related anonymity goal at the delivery layer instead.
See [`identity.md`](identity.md).

### Crypto primitives

Signal's deployed parameter choice (per the libsignal Rust crate):

- DH: X25519
- KDF: HKDF-SHA256 for the root key; HMAC-SHA256 for chain key derivation
- AEAD: AES-256-CBC + HMAC-SHA256 (encrypt-then-MAC). Note: not AES-GCM —
  the Signal Protocol predates widespread AES-GCM library availability on
  mobile and Marlinspike has cited AES-CBC + HMAC's better failure modes
  under nonce reuse.

## Pre-key rotation — the load-bearing operational property

The Willow open-problems file specifically cites Signal for pre-key rotation
(`docs/prior-art/willow/open-problems.md:213-214`). The mechanism:

- **Identity key** (`IK_B`): never rotates. Compromise = identity compromise;
  recovery requires re-registration.
- **Signed prekey** (`SPK_B`): rotated on a cadence (Signal rotates roughly
  weekly in production). Old `SPK_B` is retained briefly to accept in-flight
  X3DH initiations using the previous bundle, then deleted.
- **One-time prekeys** (`OPK_B`): each is consumed once. The server reports
  to the client when the OPK pool is running low; the client uploads more.
  Signal's client targets a pool of ~100 OPKs at the server.

**Production refill pattern.** The client refills OPKs in the background
when the server-reported count drops below a threshold (Signal's threshold
in libsignal-protocol is `MIN_PRE_KEY_COUNT = 10`). A client that goes
offline indefinitely eventually exhausts its OPK pool; subsequent X3DH
sessions to that client run *without* `DH4`, weakening forward secrecy
modestly but not breaking the protocol.

**What this requires the server to do:** hand out exactly one OPK per fetch,
delete it after handing out, and never serve the same OPK twice. Signal
trusts its own server to do this. A malicious server could replay OPKs to
multiple initiators and weaken forward secrecy uniformly across affected
sessions.

## Implications for Myrhiza

- **The prekey-bundle pattern survives in P2P**, but the OPK pool requires
  *some* always-available storage for the offline-party's bundle. In Signal
  this is the server; in Myrhiza this would need to be a relay-with-storage
  capability or a DHT bundle store. Either way, the storage layer needs to
  do single-shot OPK handout faithfully — see "trust the server to not
  replay OPKs" above. Replay-detection on the receiver side is non-trivial.
- **Per-message forward secrecy is cheap if you have the Double Ratchet
  already.** MLS's per-epoch model is coarser. For 1:1 DMs, Double Ratchet
  is still the right answer even when MLS is in the picture for groups.
- **The session-key cache is a real persistence requirement.** A
  Myrhiza-side Double Ratchet implementation must persist message keys
  across kernel restart, or every restart drops in-flight out-of-order
  decryption.
- **Curve choice is X25519 in production.** Myrhiza's existing
  ChaCha20-Poly1305 + X25519 (per `prior-art/willow/crypto.md`) is
  Signal-compatible by curve.

## Sources

- X3DH specification (rev 1, 2016-11-04): <https://signal.org/docs/specifications/x3dh/>
- Double Ratchet specification (rev 4, 2025-11-04): <https://signal.org/docs/specifications/doubleratchet/>
- Wikipedia: Double Ratchet Algorithm — <https://en.wikipedia.org/wiki/Double_Ratchet_Algorithm>
- Wikipedia: X3DH — <https://en.wikipedia.org/wiki/X3DH>
- libsignal repository (Rust impl): <https://github.com/signalapp/libsignal>
- "The Double Ratchet: Security Notions, Proofs, and Modularization for the Signal Protocol" — Alwen, Coretti, Dodis, EUROCRYPT 2019: <https://eprint.iacr.org/2018/1037>
