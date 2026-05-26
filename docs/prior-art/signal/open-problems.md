**Date:** 2026-05-22
**Status:** active
**Subject:** What Signal does NOT solve — open problems and structural limitations of the protocol family.

# Signal — open problems

Signal is the gold standard for E2EE messaging, but it has explicit
unsolved problems and structural limitations. This file catalogs them so
a Myrhiza spec author knows what *not* to assume Signal has answered.

Signal's design has shipped at 70M-MAU scale for over a decade. The
problems below are real *despite* that — they would be larger problems
in a P2P deployment that lacks Signal's central operator as a leverage
point.

## 1. Identity-key recovery after primary-device loss

**The problem:** If a user loses their primary device without first
adding a backup recovery, their identity key is gone. They re-register,
producing a new ACI/PNI pair. Their contacts see a "safety number
changed" warning; chat history is gone (unless backed up to a different
device or extracted before loss).

**What Signal does:** SVR (Secure Value Recovery) stores *settings and
contacts* (recoverable via PIN), but does NOT store identity keys. A
user who recovers via PIN gets back their contact list and profile, but
all conversations restart with new keys.

**Why it's hard:** Identity-key recovery from a passphrase is
fundamentally a trade-off between user-rememberable entropy and
adversary cost. Signal chose to *not* take this trade-off — they treat
identity-key compromise as catastrophic and avoid offering recovery.

**For Myrhiza:** This is harder in P2P. There is no SGX-rate-limited
SVR-equivalent. Options: (a) accept the loss (UX harsh), (b) ship
secret-sharing across trusted devices/contacts (research-grade UX),
(c) hardware-backed keys (custodial UX problems). All have real
trade-offs.

## 2. Ratchet-level post-quantum security

See [`post-quantum.md`](post-quantum.md). PQXDH covers initial key
agreement only. Subsequent DH ratchet steps are classical, so a
quantum adversary recording today can recover post-setup message keys
once they have a sufficient quantum computer.

Apple's iMessage PQ3 closes this gap (or claims to). Signal has not
publicly committed to ratchet-level PQ.

**For Myrhiza:** Inherits the gap if it adopts PQXDH unchanged.
Closing it requires either (a) Apple's PQ-rekeying approach (large
per-message overhead, especially for fan-out), or (b) waiting for
ratchet-level PQ research to converge.

## 3. Metadata privacy beyond sealed sender

See [`critiques.md`](critiques.md) §4. Sealed sender hides sender
identifier from the server *envelope*, but not from:

- TLS connection metadata (IP, timing).
- Account-existence + online-presence probes.
- Group-membership-proof submission (the server sees that a proof was
  submitted by some group member, even if it can't identify which).
- Cross-correlation between sender's IP and recipient's notification
  arrival.

Signal's mitigation is the centralized server itself — Signal can pool
many users' traffic into the same servers, reducing some correlation
attacks via volume. A P2P deployment does not have this pooling.

**Standing open problem in the literature.** Mixing networks (Tor,
Loopix, Nym) provide stronger metadata privacy but at significant
latency cost (seconds-to-minutes per message). Signal has not adopted
mixing for the production protocol.

## 4. Group-call cryptography is separate from messaging

Signal's group voice/video calls use a Selective Forwarding Unit (SFU)
with SRTP and an SFU-distributed group key. This is not the Double
Ratchet. Group-call keys do not have the same forward-secrecy /
post-compromise security properties as messaging keys.

**Why it's hard:** Real-time media has latency budgets (~150ms total)
that don't tolerate per-message KEM operations. The crypto for group
calls is structurally different from the crypto for messaging.

MLS faces the same problem; MatrixRTC uses MLS for group call key
establishment but a separate SRTP-class cipher for media.

**For Myrhiza:** Group calls are out of scope for Willow today, but
inherit Signal's open-problem if added.

## 5. Federation interop

Signal does not federate. A user on a hypothetical alternative
Signal-Protocol-compatible service cannot reach a Signal user. Even
though the underlying protocols (Signal Protocol + libsignal) are
public/AGPL, there's no interop mechanism.

The EU's Digital Markets Act (DMA, 2024) has begun forcing some
interop on "gatekeeper" messaging services. Signal is below the DMA
gatekeeper threshold and has stated it will not interop with WhatsApp
(WhatsApp is a gatekeeper required to interop). The technical work
for federated Signal Protocol is not happening at Signal.

**For Myrhiza:** P2P is the federation answer at the *protocol* layer.
But the question of "how do two Myrhiza instances interop with
different identity systems" remains a real spec problem.

## 6. Anti-spam without central operator

Signal's spam defenses lean heavily on:
- Phone-number-rooted identity (spam costs ~$0.01 per SMS per fake
  account).
- Server-side rate limiting on prekey-bundle fetch.
- Server-side abuse detection for sealed-sender messages from
  non-contacts.

A P2P deployment has none of these. **Anti-spam in P2P E2EE is an
open problem with no clean answer.** Cryptocurrency-attached anti-spam
proposals exist (Nostr uses payment for relay use) but have not
deployed at scale.

**For Myrhiza:** Real spec problem. Worth its own document.

## 7. Cryptographic agility vs locked-in primitives

Signal Protocol parameters (X25519, SHA-256, AES-CBC + HMAC) were
chosen in 2013-2014 and largely unchanged since. Migrating to new
primitives (e.g., X448 for higher security margin, AES-GCM-SIV for
nonce-misuse resistance) requires protocol-version negotiation, which
Signal does not currently support gracefully — message format does not
include a version field.

PQXDH was bolt-on rather than versioned migration; the design works
because the *initial key agreement* is upgradeable but the *ratchet
state* is not.

**For Myrhiza:** Worth designing crypto-agility into the wire format
from day 1. Postcard + a leading version byte is the cheapest answer.

## 8. State persistence and reliability

Double Ratchet state is large and fragile. Each session has:

- Root key
- Sending chain key + counter
- Receiving chain keys for every counterparty DH key seen
- Skipped-message-key cache (up to MAX_SKIP entries per chain)
- One-time prekey state (which OPKs are deleted vs still live)

Total per-session state: hundreds of bytes to several KB depending on
out-of-order traffic. State must persist across app restart; loss of
state means re-running X3DH and losing in-flight skipped messages.

Signal's clients handle this with platform-specific encrypted databases
(SQLCipher on Android/iOS, encrypted SQLite on Desktop). The encryption
key is held by the platform's secure storage (Android Keystore,
iOS Keychain).

**For Myrhiza:** State persistence + encryption + cross-restart
reliability is a real engineering problem. Inherits naturally from
Willow's existing persistence model but requires explicit design for
the Double Ratchet's session-state shape.

## 9. Cross-device state synchronization (multi-device)

Each linked device has its *own* Double Ratchet sessions with each
contact. There is no shared session state across a user's devices.
Implication: if a user reads a message on their phone, the desktop
client decrypts the same message *independently* (because the sender
fanned-out a separate ciphertext to the desktop). State across the
user's devices (read receipts, delivery acks, draft messages,
last-read-position) is synchronized through a separate Signal-Sync
protocol layered on top of the per-device sessions.

**The problem:** This works but is bespoke. There is no general "share
session state across my devices" primitive in the Signal Protocol.

MLS does not natively solve this either — MLS sessions are per-leaf,
and a user's multiple devices would be multiple leaves. Discord's DAVE
deployment of MLS handles this with one leaf per user (single device)
and treats multi-device as a future problem.

**For Myrhiza:** Real spec problem and the load-bearing one for the
multi-device-identity spec already gestured at in Willow's open-
problems file. The Signal answer (per-device sessions + separate sync
protocol) is one shape; the MLS-with-one-leaf-per-device answer is
another. Neither is obviously right.

## 10. Server-trust on identity-key first-publication

When Alice fetches Bob's prekey bundle for the first time, she trusts
the server to return Bob's *actual* identity key. The server could
substitute its own and MITM. Safety numbers (out-of-band comparison)
catch this after the fact, but most users never compare safety
numbers.

Signal's planned **Key Transparency** (in development as of 2026-05-22)
would close this gap: a Merkle-tree log of `(identifier → identity_key)`
records that a third party can audit for substitutions.

**For Myrhiza:** No central server, but the equivalent problem
exists: trust-on-first-use for any peer's identity key. Key
transparency is *more* necessary in P2P than centralized, because
every peer is potentially a relay that could be substituting keys
for downstream parties.

## 11. Account portability without server cooperation

If Signal Foundation goes away (legal action, funding collapse, etc.),
users lose access to their accounts and chat history. The protocol has
no notion of "migrate my account to a different server" because there
*is* only one server.

A migration plan does not publicly exist. The single-operator risk is
unhedged at the architecture level.

**For Myrhiza:** P2P sidesteps this — there is no single operator to
lose. But it has its own analogous problem: if the user's primary
device fails, the account is functionally gone (see §1).

## Sources

- Signal blog: <https://signal.org/blog/>
- Discord DAVE deployment (one-leaf-per-user MLS): <https://discord.com/safety-and-policies/dave-protocol-whitepaper>
- Loopix (mix network with low latency): <https://arxiv.org/abs/1703.00536>
- EU DMA messaging-interop article: <https://digital-strategy.ec.europa.eu/en/policies/dma-explained>
- Comparator: `prior-art/willow/open-problems.md` (cross-link target)
- Comparator: `prior-art/mls/open-problems.md`
