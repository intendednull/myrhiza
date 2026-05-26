**Date:** 2026-05-22
**Status:** active
**Subject:** Signal-specific terminology — ACI, PNI, prekey bundle, sealed sender, ratchet, zkgroup, etc.

# Signal — glossary

## A

**ACI (Account Identifier)** — A UUID minted by Signal at account
creation. The user's long-term identity. Survives phone-number changes.
Carries its own identity keypair (`IK_ACI`). See
[`identity.md`](identity.md).

**Axolotl Ratchet** — The Double Ratchet's original name (2013-2016).
Renamed in March 2016. See [`history.md`](history.md).

**AGPL-3.0** — GNU Affero General Public License v3.0. The license on
libsignal and Signal's mobile/desktop apps. Network-use trigger means a
service running AGPL software must release source. See
[`libsignal.md`](libsignal.md).

## C

**Chain key** — A per-direction key in the Double Ratchet's symmetric-
key ratchet. Advanced one step per message via HMAC; produces a fresh
message key each step. Deleted after the message key is derived. See
[`protocol.md`](protocol.md).

**CLA (Contributor License Agreement)** — Signal's CLA, which
contributors to libsignal must sign. Grants Signal Foundation broad
relicensing rights. See [`governance.md`](governance.md).

**Curve25519 / X25519 / Ed25519** — The Diffie-Hellman curve used in
Signal's classical (non-PQ) crypto. X25519 for key agreement, Ed25519 for
signatures (used in safety-number derivation and elsewhere).

## D

**DH ratchet** — The Diffie-Hellman ratchet in the Double Ratchet.
Provides per-round-trip post-compromise security. Each side advances
their DH keypair when they see a new public key from the counterparty.
See [`protocol.md`](protocol.md).

**Double Ratchet** — The session-cipher half of the Signal Protocol.
Two ratchets in parallel: symmetric (per-message FS) + DH (per-round-
trip PCS). Spec at <https://signal.org/docs/specifications/doubleratchet/>.

## H

**HKDF** — HMAC-based Key Derivation Function (RFC 5869). Used
throughout Signal Protocol for key derivation from secrets. Signal uses
HKDF-SHA256.

## I

**Identity key (IK)** — A long-term Curve25519 keypair that identifies
a user (or device, in multi-device). Never rotates within a single
ACI/PNI lifetime. Signs short-lived signed prekeys.

## K

**KDF chain** — A sequence of keys produced by repeated application of
a KDF. Each step deletes the previous chain key. Provides forward
secrecy and break-in recovery (when seeded with fresh entropy).

**KEM (Key Encapsulation Mechanism)** — A cryptographic primitive used
in PQXDH. The PQ analogue of DH key agreement. Signal uses CRYSTALS-
Kyber-1024 in production.

**Kyber / ML-KEM** — The CRYSTALS-Kyber lattice-based KEM. ML-KEM is
the NIST-standardized variant (FIPS 203, 2024-08-13). Signal's PQXDH
shipped on pre-standardization Kyber-1024; migration to ML-KEM is in
progress as of 2026-05-22. See [`post-quantum.md`](post-quantum.md).

**KVAC (Keyed-Verification Anonymous Credentials)** — The credential
scheme underlying zkgroup. Lets a verifier (Signal's server) check
credential validity without learning the credential holder's identity.
See [`groups.md`](groups.md).

## L

**libcrux-ml-kem** — The Cryspen-maintained verified Rust ML-KEM crate.
Used by both libsignal and OpenMLS. Dual Apache-2.0 / MIT. See
[`libsignal.md`](libsignal.md), [`post-quantum.md`](post-quantum.md).

**libsignal** — The Rust + FFI reference implementation of the Signal
Protocol. Shipped by Signal Foundation. AGPL-3.0. See
[`libsignal.md`](libsignal.md).

## M

**MAX_SKIP** — The maximum number of out-of-order message keys cached
per chain in the Double Ratchet. Typically 1000. See
[`protocol.md`](protocol.md).

**Megolm** — Matrix.org's symmetric-ratchet group encryption protocol.
Similar shape to Signal's Double Ratchet but without per-round-trip PCS.
See [`comparisons.md`](comparisons.md).

**Message key (MK)** — The per-message symmetric key in the Double
Ratchet. Derived from the chain key via HKDF. Used to encrypt one
message, then deleted.

**MLS (Messaging Layer Security)** — RFC 9420. The IETF-standardized
group key agreement protocol. Comparator to Signal's pairwise fan-out
for groups. See [`comparisons.md`](comparisons.md) and `prior-art/mls/`.

## O

**Olm** — Matrix.org's Double-Ratchet-derived 1:1 protocol. See
[`comparisons.md`](comparisons.md).

**OMEMO** — XEP-0384, the XMPP port of the Signal Protocol. Federated
deployment of X3DH + Double Ratchet. See
[`comparisons.md`](comparisons.md).

**One-time prekey (OPK)** — A Curve25519 keypair in Bob's prekey
bundle. Consumed by exactly one X3DH session, then deleted. Provides
forward secrecy for the first message. See [`protocol.md`](protocol.md).

**Open Whisper Systems** — The pre-2018 organization (Marlinspike's
for-profit Quiet Riddle Ventures LLC) that developed Signal. Replaced by
Signal Foundation in 2018-02-21. See [`history.md`](history.md).

**ORAM (Oblivious RAM)** — A cryptographic primitive that hides memory
access patterns. Signal uses Path ORAM in its contact-discovery SGX
enclave. See [`server.md`](server.md).

**OTR (Off-the-Record Messaging)** — The 2004 protocol (Borisov-
Goldberg-Brewer) whose DH ratchet inspired Signal's Double Ratchet.
See [`comparisons.md`](comparisons.md).

## P

**PCS (Post-Compromise Security)** — The property that future
communication is secure even after a current key compromise, *provided*
fresh entropy enters the protocol after the compromise. Provided by the
Double Ratchet's DH ratchet step.

**PNI (Phone Number Identifier)** — A separate UUID tied to a user's
current phone number, distinct from their ACI. Receives messages from
strangers and discovery probes. Rotates when the phone number changes.
See [`identity.md`](identity.md).

**Prekey bundle** — A user's published key material: identity key (IK),
signed prekey (SPK), optional one-time prekey (OPK). Stored on Signal's
server and handed out to senders running X3DH. See
[`protocol.md`](protocol.md).

**PQXDH (Post-Quantum Extended Diffie-Hellman)** — The PQ-augmented
replacement for X3DH. Hybrid X25519 + Kyber-1024. Shipped 2023-09. See
[`post-quantum.md`](post-quantum.md).

**Primary device** — In Signal's multi-device model, the user's phone
(must be Android or iOS). The primary signs new linked-device identity
keys. Lose the primary → cannot add/revoke linked devices.

**Profile key** — A per-user symmetric key, shared with contacts when
they accept the user's message request. Used to derive delivery tokens
for sealed sender. See [`identity.md`](identity.md).

## R

**Root key** — The Double Ratchet's master KDF input. Updated on every
DH ratchet step. Seeds both the sending and receiving chain keys.

## S

**Sealed sender** — Signal's delivery-anonymity mechanism. Wraps an
outgoing message so the server cannot see the sender's identifier.
Announced 2018-10-29. See [`identity.md`](identity.md).

**Sender certificate** — A short-lived (~24h) certificate issued by the
Signal CA to each registered device. Binds device identity key to
ACI/PNI. Used in sealed sender. See [`identity.md`](identity.md).

**SGX (Intel Software Guard Extensions)** — Intel's hardware-enforced
enclave technology. Used by Signal's contact discovery and SVR. Has
suffered multiple side-channel vulnerabilities. See
[`server.md`](server.md), [`critiques.md`](critiques.md).

**Signal CA** — Signal Foundation's long-lived signing key that issues
sender certificates. The central-trust root for sealed sender. See
[`identity.md`](identity.md), [`lessons.md`](lessons.md) §A2.

**Signal Foundation** — The 501(c)(3) nonprofit that operates Signal
since 2018-02-21. Founded by Marlinspike + Acton. See
[`governance.md`](governance.md).

**Signed prekey (SPK)** — A medium-lived Curve25519 keypair in Bob's
prekey bundle, signed by his identity key. Rotated periodically (Signal
rotates roughly weekly). See [`protocol.md`](protocol.md).

**SVR (Secure Value Recovery)** — Signal's PIN-based encrypted-state
recovery. SGX enclave + Argon2 + Raft consensus + 5-guess rate-limit.
See [`server.md`](server.md).

**Symmetric ratchet** — The per-message half of the Double Ratchet.
Chain key advanced by HMAC each message. Provides per-message forward
secrecy.

## T

**TextSecure** — The pre-Signal Android messaging app (2010-2015) where
the Signal Protocol was first deployed. Merged with RedPhone (voice) to
become unified Signal in 2015-11.

**TreeKEM** — The group key agreement primitive at the core of MLS.
Comparator to Signal's pairwise fan-out for groups. See
`prior-art/mls/protocol.md`.

## U

**Username (Signal feature)** — A user-chosen handle that lets contacts
initiate conversation without learning the user's phone number.
Resolved server-side to the user's ACI. Rolled out 2024-02-20. See
[`identity.md`](identity.md).

## V

**vodozemac** — Matrix.org's MIT-licensed Rust implementation of Olm +
Megolm. The licensing-clean reference implementation closest to Signal
in shape. See [`comparisons.md`](comparisons.md), [`libsignal.md`](libsignal.md).

## W

**WhatsApp** — Owned by Meta. Uses the Signal Protocol *spec* with a
closed-source implementation. Rolled out E2EE on 2016-04-05. ~2B users.
The largest deployment of the Signal Protocol by orders of magnitude.

## X

**X3DH (Extended Triple Diffie-Hellman)** — Signal's pre-PQXDH
initial key agreement protocol. Four DH exchanges. Marlinspike + Perrin,
rev 1 (2016-11-04). See [`protocol.md`](protocol.md).

## Z

**zkgroup** — Signal's keyed-verification anonymous credential scheme
for private group membership. Chase + Perrin + Zaverucha (CCS 2020).
See [`groups.md`](groups.md).

## Sources

- All cross-files in `prior-art/signal/`.
- Signal Protocol specifications: <https://signal.org/docs/>
