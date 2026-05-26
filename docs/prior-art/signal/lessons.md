**Date:** 2026-05-22
**Status:** active
**Subject:** Signal-into-Myrhiza synthesis — validates / avoid / borrow. The decision file.

# Signal — lessons for Myrhiza

This is the file to read when designing a Myrhiza spec that touches
multi-device identity, key rotation, sender anonymity, post-quantum
crypto, or 1:1 DM cryptography. Other files in this corpus are evidence;
this is synthesis.

## Validates

Things Signal's deployment-at-scale confirms Myrhiza is right (or
right-ish) to do:

### V1. Pairwise Double Ratchet for 1:1 DMs

Signal has shipped pairwise Double Ratchet at 70M-MAU scale (Signal) +
~2B (WhatsApp) for over a decade. Per-message forward secrecy + per-
round-trip post-compromise security is the *right* property set for 1:1
asynchronous messaging.

**Myrhiza translation:** Adopt the Double Ratchet shape for 1:1 DMs
even when MLS is in the picture for groups (see V2).

### V2. Pluralist: Double Ratchet for DMs + MLS for groups

Discord DAVE (MLS for group call keys + pairwise for DMs) is production
proof of the pluralist deployment. Signal itself doesn't use MLS, but
the largest MLS deployment in the world does not use MLS for 1:1
either.

**Myrhiza translation:** Don't force one protocol family to do
everything. Pick per use-case.

### V3. Identity ≠ active signing key

PNI/ACI split + multi-device per-device-identity-keys is the production
validation of "long-term identity decoupled from short-lived signing
keys." This is exactly the shape Willow's open-problems file gestures at
(`docs/prior-art/willow/open-problems.md:75-100`).

**Myrhiza translation:** Adopt a three-layer model: long-term identity
(ACI-equivalent) signs short-lived per-device keys (per-device IKs),
which in turn participate in the cryptographic session. Discovery goes
through a *separately rotatable* indirection (username / PNI / topic-
attached handle / whatever the discovery primitive ends up being).

### V4. Pre-key rotation under forward secrecy is feasible at scale

Signal's prekey-bundle pattern — identity key permanent, signed prekey
rotated periodically, one-time prekeys consumed-and-deleted — works in
production at scale. The Willow open-problems cite this directly
(`docs/prior-art/willow/open-problems.md:213-214`).

**Myrhiza translation:** The pattern is sound. Translate it to a
relay-with-storage capability (or DHT-backed bundle store) in the P2P
deployment.

### V5. Hybrid post-quantum is the right intermediate step

PQXDH's hybrid (X25519 + Kyber-1024) shipped in 2023 without protocol
redesign. It's a stop-gap (see avoid §A4) but the *hybrid* shape is the
right way to introduce PQ without committing to PQ-only crypto whose
parameters might be re-broken (Kyber received multiple security
updates pre-FIPS).

**Myrhiza translation:** If Myrhiza adopts post-quantum, hybrid with a
classical primitive. Use libcrux-ml-kem (same crate OpenMLS uses;
see `prior-art/mls/openmls.md`).

## Avoid

Things Signal does that Myrhiza should *not* copy:

### A1. Phone-number-rooted registration

Signal still *requires* a phone number at account creation, even though
the phone number can be hidden later. For a P2P runtime hosting third-
party apps, requiring a PSTN identifier at the protocol layer is
wrong.

**Myrhiza translation:** Identity registration is app-layer, not
runtime-layer. The runtime should know nothing about PSTNs or SMS.

### A2. Central-CA-issued sealed-sender certificates

Sealed sender depends on a single signing root (Signal CA). A compromise
breaks sealed sender for *all* Signal users. In a P2P translation, every
user is their own CA — strictly better (smaller blast radius), at the
cost of every user holding the CA key safely.

**Myrhiza translation:** When porting the sealed-sender pattern, do
**not** introduce a central signing root. The user's long-term identity
key is the CA for their own short-lived per-device certificates.

### A3. SGX-dependent contact discovery

Signal's contact-discovery and SVR features lean on Intel SGX. SGX has
had a *long* string of side-channel vulnerabilities and is not a load-
bearing crypto primitive. Signal is clear-eyed about this (treats SGX
as defense-in-depth, not as load-bearing), but the *functionality* is
SGX-dependent: a non-SGX deployment doesn't get contact-discovery
privacy.

**Myrhiza translation:** Do not design Myrhiza specs that assume
hardware-enforced confidentiality (SGX, TrustZone, SEV-SNP). Make
protocol-level guarantees that survive hardware compromise.

### A4. Ratchet-level classical-only crypto under "PQ" branding

PQXDH is PQ-for-initial-setup only. Marketing-wise this gets called
"post-quantum Signal." Apple's PQ3 has called this out as
insufficient ("Level 2"). The honest answer is "PQ-for-setup, not
PQ-for-ratchet."

**Myrhiza translation:** If Myrhiza claims post-quantum security in
any spec, be explicit about which protocol step is protected. Don't
inherit Signal's branding shortcut.

### A5. AGPL on the reference implementation

AGPL-3.0 on libsignal is a near-fatal licensing problem for a runtime
that hosts third-party apps. Signal's network-use trigger forces every
app using libsignal to be AGPL.

**Myrhiza translation:** Do not link libsignal. Re-implement from the
spec (CC-BY 4.0) or use vodozemac (MIT, Olm + Megolm).

### A6. Single-operator centralization

The "ecosystem is moving" defense of centralization is real for
iteration speed but trades off against single-point-of-failure risk.
Myrhiza is committed to P2P; don't reintroduce a single operator via
the back door.

**Myrhiza translation:** Don't ship a Myrhiza spec that has *any*
single operator on the critical path. Relays may be operated by
different parties; workers may be operated by different parties; no
one party holds keys that, if compromised, break the system for all
users.

### A7. Closed RFC process

Signal designs protocol changes inside Signal Foundation, publishes
them after implementation, and accepts no external standards-body
involvement. This produces fast iteration but no formal interop story.

**Myrhiza translation:** If Myrhiza wants apps from different teams to
interop, engage open standards (IETF, IRTF, W3C). Open-spec but
go-it-alone is the worst of both worlds — slow iteration *and* no
interop.

## Borrow

Things to actively borrow into Myrhiza:

### B1. Three-tier identity model (long-term / device / discovery)

ACI (long-term, signs everything else) + per-device IK (short-lived,
signs message keys) + username/PNI (discovery indirection, rotatable).

**Concretely:** A Myrhiza spec for multi-device identity should model
exactly this three-tier shape. The long-term identity is what survives
device replacement; the per-device key is what's compromised on phone
loss; the discovery indirection is what's rotated when users move/
change context.

### B2. Sealed-sender envelope pattern (without central CA)

The sealed-sender envelope shape (`encrypt(sender_cert, content)`
where the outer layer is encrypted to the recipient and the inner
sender cert proves identity) translates cleanly to a P2P deployment
*if* the central CA is replaced with user-self-signed certificates.

**Concretely:** Willow's seal-gift-wrap pattern already adopts this
shape (per the user's note about it deriving from sealed sender). The
Myrhiza implementation should keep the shape and replace the CA.

### B3. Prekey-bundle pattern + OPK consume-and-delete

Identity key + signed prekey + one-time prekey is the production-
proven shape for async key agreement. Translate to relay-with-storage
or DHT-bundle-store.

**Concretely:** A relay that stores prekey bundles needs an OPK-
handout-faithfully property (each OPK delivered to exactly one
initiator). Hard to guarantee without a trusted operator. Options:
(a) accept weaker forward secrecy if OPKs are replayed; (b) cryptographic
detection of double-spend (Sybil-resistant + signed-by-recipient); (c)
trust the user's own relay set. Worth a focused spec.

### B4. Safety-number out-of-band verification UX

The 60-digit safety number that users compare out-of-band is the
production-proven UX answer to "verify identity-key bindings."
Multiple parties have iterated on this (Signal, Briar, OMEMO clients).

**Concretely:** Myrhiza's UX layer (when it ships) should adopt the
safety-number pattern for any spec that involves user-controlled
identity-key trust. Variants (QR scan, audio fingerprint) are all
acceptable; the load-bearing property is *out-of-band* comparison.

### B5. libcrux-ml-kem as the Rust PQ KEM crate

Both libsignal and OpenMLS depend on it. Dual Apache-2.0 / MIT licensed,
F\*/hax-verified.
The closest thing to "the standard PQ KEM crate" in the Rust
ecosystem right now.

**Concretely:** If Myrhiza adopts PQ, the dependency picks itself.
Don't roll your own Kyber.

### B6. zkgroup credential pattern (research-grade for P2P)

zkgroup's "prove I'm a member without revealing which" pattern is the
production reference for anonymous group membership. Translation to
P2P is research-grade (no central verifier; need multi-show credentials
or a different verifier model), but the shape of the property is the
goal to aim for.

**Concretely:** If Myrhiza ever wants private-group-membership (the
server / relay cannot enumerate members), zkgroup's credential
structure is the place to start. Treat as a long-horizon spec, not a
v1 feature.

### B7. Honest "no key recovery without trade-offs" stance

Signal explicitly does not offer identity-key recovery and is clear
about why. That honesty has earned trust. Don't promise key recovery
in Myrhiza specs without being explicit about the trade-offs (PIN-
based: dictionary-attackable; secret sharing: usability disaster;
hardware-backed: custody problems).

**Concretely:** Any Myrhiza spec touching key recovery should
articulate the trade-off explicitly. Default to "primary device loss
means re-registration" with a clear path to add recovery later.

## Decision rules summary

- **For 1:1 DMs:** adopt Double-Ratchet shape, with hybrid PQ at
  setup if PQ is in scope.
- **For groups (>~5 members):** adopt MLS shape. Don't fan-out.
- **For identity:** three-tier (long-term / device / discovery).
- **For sender anonymity:** sealed-sender pattern, but per-user CAs.
- **For PQ:** hybrid via libcrux-ml-kem; be honest about ratchet-
  level gaps.
- **For library:** don't link libsignal (AGPL); re-implement from
  CC-BY spec or use vodozemac (MIT) as reference.
- **For governance:** open standards if interop matters; otherwise
  Signal's go-it-alone is OK *if* you accept the cost.

## Sources

- All cross-files in `prior-art/signal/`.
- `prior-art/willow/open-problems.md:75-100` (multi-device identity reference).
- `prior-art/willow/open-problems.md:213-214` (pre-key rotation reference).
- `prior-art/mls/lessons.md` (companion lessons file).
- `prior-art/iroh/lessons.md` (transport layer that would host similar P2P primitives).
