**Date:** 2026-05-22
**Status:** active
**Subject:** Signal's multi-device identity model (ACI / PNI dichotomy) and sealed-sender delivery. The load-bearing file for Myrhiza's multi-device identity decision.

# Signal identity — ACI, PNI, Sealed Sender

Two identifiers per account plus a delivery-anonymity layer:

| | |
|---|---|
| **ACI** (Account Identifier) | The user's *real* account. Stable across phone-number changes. Receives messages from contacts and in groups. |
| **PNI** (Phone Number Identifier) | A *separate* identifier tied to the user's current phone number. Receives unsolicited messages and discovery probes. |
| **Sealed Sender** | A delivery mechanism that hides the sender's identifier from the Signal server. Announced 2018-10-29. |

Together: the user *has* a long-term identity (ACI) that is decoupled from
their phone number (PNI), and *delivery* of messages hides the sender's
identifier from the server.

Willow's open-problems file cites this design twice
(`docs/prior-art/willow/open-problems.md:86-88`) as the production reference
for "long-term identity, short-lived per-device signing key" and for
delivery-without-revealing-sender. This file is the entry point.

## The PNI/ACI split — why two identifiers?

Signal originally rooted identity in the phone number. That created several
problems:

1. **Account portability:** a user changing phone numbers should not lose
   their chat history or their contacts' ability to reach them. Phone-number-
   rooted identity makes this hard.
2. **Phone-number privacy:** users discovering each other by phone number
   leaks the phone number to anyone who initiates a discovery probe. A
   privacy-conscious user wants to be reachable without disclosing their
   number.
3. **PSTN-issued identifiers are not user-controlled:** carriers reassign
   numbers; SIM-swap attacks are real; some users use numbers obtained from
   VoIP providers that are not stable.

PNI was announced 2022-11-15 and rolled out over the following years. The
key idea: split *what identifies the user* (ACI, a UUID minted by Signal at
account creation) from *what identifies the phone number* (PNI, a different
UUID tied to the SIM). Each identifier has its own identity key in libsignal.

### Identifier semantics

- **ACI** is the canonical recipient address inside groups and in already-
  established 1:1 conversations. It does not change when the user's phone
  number changes.
- **PNI** is the address used for *initial contact* via phone-number
  discovery, *and* for messages from senders who don't yet have your ACI.
  When you change your phone number, your old PNI is detached and a new one
  is bound.
- Each carries its own identity key (per-identifier `IK`). A user has two
  long-term keypairs, not one.
- The server knows the mapping ACI ↔ PNI for each account; clients of
  the user's *contacts* learn the ACI when they first communicate, then
  prefer ACI for future messages.

### Username feature (2024-02-20)

Signal's username feature (rolled out 2024-02-20) layers on top of PNI: a
user can publish a username, and a sender can reach them via that username
without learning the phone number at all. The server resolves
`username → ACI` without revealing the underlying PNI to the sender.

This is the design pattern Willow's open-problems file gestures at: ACI
is the load-bearing user identity; PNI and username are *discovery
indirections* that the user can rotate without breaking ACI continuity.

## Multi-device — same ACI, different signing keys

A single Signal account has one **primary device** (always a mobile phone)
and zero or more **linked devices** (Desktop, iPad). The multi-device design:

- ACI is one keypair, registered on the primary device.
- Each linked device generates *its own* identity keypair at link time and
  registers it with the server, signed by the primary device.
- Each device has its own X3DH prekey bundle, its own Double Ratchet
  sessions with each contact, and its own one-time prekeys.
- A message sent to ACI fans out to *every active device's prekey bundle*.
  The sender runs X3DH separately against each device's bundle and emits a
  separate ciphertext per recipient device.

This is fan-out at the protocol layer, not a single multi-device session.
The trade-off: simple and proven, but every additional device adds a
multiplier to the sender's per-message cost and to the server's storage
cost.

Linked-device key registration is done via QR code at link time: the
primary device displays a QR with a session token, the linked device
scans, performs an authenticated key exchange, and uploads its new
identity key signed by the primary. The primary then publishes the new
device to the user's `linked-devices` list at the server.

### Implication: the "device" is the unit of cryptographic identity

A user's "identity" from the protocol's perspective is actually a set of
device identity keys, all signed by the primary. The user-visible ACI is
the *naming* of that set. This is the pattern Myrhiza's open-problems file
calls out: long-term identity (ACI) is distinct from the active signing
keys (per-device IKs).

The primary device is a single point of failure for adding/revoking
devices. Lose the primary, and you cannot remove the linked devices —
account re-registration is required.

## Sealed Sender — hiding the sender from the server

Announced 2018-10-29. The problem: end-to-end encryption hides message
*content* from the Signal server, but the server still sees who sent each
message to whom. That metadata graph is a serious privacy leak — for many
adversaries, knowing *that* Alice messaged Bob is enough.

### Mechanism

The client wraps its outgoing ciphertext in an envelope:

```
sealed_envelope = encrypt_to_recipient_IK(
    sender_certificate,        -- short-lived (~24h), signed by Signal CA
    ciphertext,                -- the X3DH/Double Ratchet payload
    sender_device_id
)
```

The server sees only:

- The outer envelope is encrypted to the recipient's identity key.
- No `From:` header on the envelope.
- The recipient's identifier on the outside (so the server knows where to
  deliver).

The recipient decrypts the envelope, recovers the sender certificate,
verifies the Signal CA's signature, recovers the sender identifier, and
runs Double Ratchet decryption as usual.

### Sender certificates

A short-lived certificate (~24-hour validity) issued by the Signal server
to each registered device. Contains: device's identity key, the device's
ACI/PNI, expiration. Signed by the Signal CA (a long-lived signing key
held by Signal Foundation).

The certificate exists because the recipient must still verify *which*
identity key signed the sealed envelope — without a certificate, the
recipient sees a key but cannot verify it's the one bound to the sender's
identifier. The certificate binds key → identifier and is short-lived
specifically so that compromise of a sender device limits damage to ~24h.

### Delivery tokens

A "delivery token" is a 96-bit value derived from the recipient's
**profile key** (a separate per-user symmetric key, shared with the
recipient's contacts when they accept your message request). The server
requires the sender to prove knowledge of the delivery token before
delivering a sealed-sender message — this restricts sealed sender to
contacts who have shared profile keys, preventing the feature from being
used for abuse / spam.

A recipient can disable the delivery-token check ("Sealed Sender:
Anyone") to accept sealed messages from non-contacts. Most users leave
this off by default.

### Threat model — what sealed sender protects and what it doesn't

Protects:
- The Signal server cannot link `(sender, recipient)` pairs from the
  envelope.

Does NOT protect:
- The Signal server *still* sees `(IP, recipient)` for every sealed-sender
  delivery. The sender's IP address is unredacted, and the recipient is in
  the clear. Network-level correlation across many deliveries can still
  reconstruct the social graph.
- A server-side adversary with traffic-pattern analysis can correlate
  *send time* + *recipient* with *deliver time* + *recipient* across two
  users to infer pairs. Signal mitigates this with delivery delays and
  jitter; the mitigation is not bulletproof.
- A compromised sender certificate (the Signal CA's long-lived signing
  key) breaks the entire sealed-sender system.

The point is: sealed sender pushes the social-graph leak from "trivial,
in the SQL log" to "requires active timing/IP correlation." That's still
a significant gain at low cost.

## Implications for Myrhiza

- **The PNI/ACI dichotomy is the right shape for "user-identity ≠
  device-identity ≠ discovery-handle."** Myrhiza should adopt the
  three-layer split as a design principle: a long-term identity (ACI-
  equivalent) signs short-lived device keys (per-device IKs), and discovery
  goes through a separately-rotatable indirection (username / PNI /
  whatever the discovery primitive is). This is the answer Myrhiza's
  open-problems file is reaching for.
- **Multi-device by fan-out is mechanically simple and proven.** A single
  multi-device session (MLS-style for 1:1) is conceptually cleaner but
  more complex. Signal's fan-out model has shipped at 70M-MAU scale; the
  cost per recipient is `O(devices)` not `O(devices²)`. For 1:1 DMs,
  fan-out is the right default; for groups, MLS is the right answer (see
  `prior-art/mls/`).
- **Sealed sender is borrowed-style for Myrhiza's relay model.** Willow's
  earlier seal-gift-wrap pattern derives directly from sealed sender. The
  pattern survives translation to P2P:
  - "Signal CA signs short-lived certificates" → "user's long-term key
    signs short-lived per-device certificates."
  - "Server delivers without seeing sender" → "relay delivers without
    seeing sender."
  - "Delivery token gates abuse" → "topic membership / channel-secret
    gates abuse." The replacement for delivery tokens in a P2P design is
    *whatever proves the sender is allowed to write to this channel*.
- **The primary-device-bootstraps-linked-devices pattern requires a
  secure side channel.** Signal uses QR-code-scanning at physical
  proximity. Myrhiza needs an equivalent: some side channel where two
  user-controlled devices establish trust without going through the
  network. QR scan + DH is the proven shape.
- **The "delete the primary, lose the ability to add/revoke linked
  devices" failure mode is a real cost.** A Myrhiza spec for multi-device
  identity should consider: do we accept that constraint (simple), or
  introduce a key-recovery mechanism (Signal SVR) that lets the user
  bootstrap a new primary from a recovery passphrase? Recovery has its
  own attack surface.
- **The Signal CA is a centralization wart.** Sealed sender depends on a
  single signing root; a P2P translation must replace this with
  per-user-signed certificates (the user is their own CA). The threat
  model shifts: instead of "Signal CA compromise breaks sealed sender for
  all users," it becomes "a single user's compromise breaks sealed sender
  for that user only." Strictly better, but it does require every user to
  hold their CA key safely.

## Sources

- "Technology Preview: Sealed Sender for Signal" (2018-10-29): <https://signal.org/blog/sealed-sender/>
- "Phone Number Privacy and Usernames" (2024-02-20): <https://signal.org/blog/phone-number-privacy-usernames/>
- "Faster ORAM Layer for Enclaves" (contact discovery + PNI prerequisites, 2022-08-19): <https://signal.org/blog/building-faster-oram/>
- libsignal repository (ACI/PNI implementation): <https://github.com/signalapp/libsignal>
- Wikipedia: Signal Messenger — <https://en.wikipedia.org/wiki/Signal_Messenger>
- Willow's open-problems reference: `docs/prior-art/willow/open-problems.md:86-88`
