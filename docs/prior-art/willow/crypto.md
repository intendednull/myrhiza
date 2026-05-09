**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — content sealing, channel keys, epoch rotation, key custody

What `willow-crypto` ships today, what was investigated and deferred,
and where PR #636 places the kernel boundary around encryption.
Companion: [networking.md](networking.md), [identity.md](identity.md),
[ui.md](ui.md), [README.md](README.md).

## Primitives shipped today (`crates/crypto/src/lib.rs`)

- **AEAD: ChaCha20-Poly1305** with random 12-byte nonces
  (`crypto/src/lib.rs:271-285`). Authenticated encryption with
  associated data — tampering with ciphertext or nonce surfaces
  as `CryptoError::DecryptionFailed`
  (`crypto/src/lib.rs:678-686`).
- **Symmetric channel key: `ChannelKey([u8; 32])`** — 256-bit key
  per channel. `ZeroizeOnDrop`, `Debug` redacted, type-level
  `assert_zeroize_on_drop` test
  (`crypto/src/lib.rs:96-114, 977-979`).
- **Key wrap: X25519 + HKDF-SHA256.** `encrypt_channel_key_for(
  channel_key, recipient_ed25519_public)` runs an ephemeral X25519
  Diffie-Hellman against the recipient's Ed25519-public-converted-to-X25519
  key, derives a wrapping key via HKDF with a versioned domain
  separator (`HKDF_KEYWRAP_DOMAIN = b"willow-crypto/v1/keywrap/channel-key"`),
  and ChaCha20-Poly1305-encrypts the channel-key bytes
  (`crypto/src/lib.rs:392-456`).
- **Ed25519 → X25519 conversion** follows RFC 7748: SHA-512 the
  Ed25519 seed, take the first 32 bytes
  (`crypto/src/lib.rs:363-388`).
- **Forward-secret ratchet: `KeyRatchet`** — HKDF-derived per-message
  keys, each with a fresh seed advance. Distinct domain separators
  for the message-key derivation (`HKDF_RATCHET_MSG_DOMAIN`) and
  the seed-advance derivation (`HKDF_RATCHET_ADVANCE_DOMAIN`) so
  the two derivations cannot collide
  (`crypto/src/lib.rs:57-65, 138-210`).
- **DoS-bounded decryption** — `MAX_RATCHET_LOOKAHEAD = 1024` and
  `open_content_bounded(sealed, key, current_counter)` reject
  any sealed packet whose claimed `ratchet_counter` exceeds
  `current_counter + MAX_RATCHET_LOOKAHEAD` *before* doing any
  HKDF work or AEAD verification, closing the unbounded-counter
  CPU DoS (issue #110 regression test pins the property:
  `u64::MAX` claim returns instantly, not in 584,000 years)
  (`crypto/src/lib.rs:289-355, 988-1014`).
- **`RatchetCache`** caches derived message keys per epoch with
  bounded eviction — repeating the same `(epoch, counter)` is
  O(1) instead of O(counter) HKDF replay
  (`crypto/src/lib.rs:493-622`). `clear()` wipes both the
  message-key cache and saved per-epoch ratchet states on
  identity-bound teardown (issue #178).
- **Wire form: `SealedContent { ciphertext, nonce, key_epoch,
  ratchet_counter }`** carried inside `Content::Encrypted`
  variants. `ratchet_counter == 0` is the "no ratchet" backwards-compat
  sentinel (`crypto/src/lib.rs:316-355`).

## Epoch-driven channel key rotation (`docs/specs/2026-04-24-epoch-key-rotation.md`)

A spec, not yet shipped. Establishes the authority pattern Myrhiza
should expect to inherit:

- An `Epoch` is `(channel_id, epoch_number: u32)`. `u32` width
  matches the existing `SealedContent.key_epoch` and
  `KeyRatchet::epoch` plumbing.
- A `RotateChannelKeyV2` event is its own DAG event, valid only
  when its `trigger` field references one of: kick (`Propose {
  KickMember }`), `RevokePermission { SendMessages }`,
  `RevokePermission { SyncProvider }`, membership-changing
  `AssignRole`, `GrantPermission { SendMessages }`, or `None`
  (explicit out-of-band rotation). Membership events do **not**
  mutate `state.channel_keys` as a side effect — only
  `apply_event` for `RotateChannelKeyV2` does (spec §Epoch
  definition).
- **Topic IDs intentionally rotate per epoch** so a passive
  gossip observer loses membership continuity across rotations.
  The spec calls this "partial metadata hiding" and is honest
  that it is not full forward secrecy: past ciphertext is safe
  only if every member actually deletes old epoch keys after use,
  which Willow cannot enforce — "it is a client-policy matter"
  (spec §Threat model).
- The rotation hierarchy provides **post-compromise security**
  but not full FS of in-flight messages, not post-quantum
  confidentiality, not IP/timing privacy, and explicitly does not
  protect pre-join history from a new member (default policy
  grants new members the current epoch key only) (spec §Threat
  model).

The spec acknowledges Willow's `seal_content` is not actually
called by production code today; the rotation work is landing
*before* the encryption producer is wired up so the PCS gap
doesn't ship as a latent vulnerability (spec §opening, lines 8-32).

## DMs deferred to MLS-over-Willow (`docs/specs/2026-04-24-seal-gift-wrap-dms.md`)

A round-2 review of a Nostr-NIP-17/44/59-inspired seal+gift-wrap
design concluded **NOT** to ship that design. Status: deferred.
No `EventKind` variants, no code. The deferral rationale is the
prior art:

- NIP-59 is a privacy envelope without a forward-secrecy layer
  underneath. Signal's Sealed Sender works because it sits on top
  of the Double Ratchet; NIP-17 has no equivalent. The Nostr
  ecosystem itself has moved on to NIP-EE / Marmot (both
  MLS-based).
- Matrix Megolm is the lived warning of group-chat-over-gossip
  without MLS — ~7 years of UTD ("Unable to Decrypt") production
  bugs live exactly in the seam this design was creating.
- RFC 9420 (MLS) gives O(log N) group rotation vs O(N) gift wraps
  in the deferred design, plus atomic admit-and-key-distribution
  via Welcome messages.

The deferral spec captured concrete lessons for the future MLS
spec: deniability claims were structurally false (the real Ed25519
signature non-repudiably binds the author once the rumor plaintext
is recovered); per-recipient inbox topics leak the active-DM-recipient
graph; per-author DAG pollution from one-shot ephemeral chains is
real. **MLS application messages should NOT enter the per-author
DAG** — they belong on a separate transport path (spec §"Crypto
lessons captured for the MLS spec", lines 56-100).

## PR #636 — kernel boundary around crypto

PR #636's "Crypto and key custody" section (lines 266-310) commits
to placement, not API signatures. The placement:

- **Secrets do not enter component memory in raw form.** Components
  hold opaque key handles; the kernel custodies bytes (line 300).
- **Private signing keys live only in the kernel.** No component
  sees them. Components describe events; the kernel signs (line 50).
- **Symmetric channel/group keys, ratchets, and MLS group state
  are kernel-custodied** on behalf of an app instance, by
  app-declared opaque handle (line 273).

Typed crypto host imports, with profile gating:

- `host.seal(handle, plaintext)` — state-`propose` and behavior
  profiles only. Produces ciphertext under the named key (line 275).
- `host.open(handle, ciphertext)` — interaction profile only.
  Decrypts for display (line 278).
- `host.verify-payload-mac(envelope, key-handle)` — state-`apply`
  deterministic helper. Proves "some holder of the key bound to
  this handle sealed this." Note: this proves *key possession*,
  not *author identity*; author identity comes from the outer
  Ed25519 sig on the event (lines 211-218).
- `host.install-key(handle, sealed-distribution-blob) -> ()` —
  state-`apply` deterministic helper. Returns `()` deliberately:
  the kernel records the (handle, blob) pair under the app's
  namespace on every peer regardless of whether *this* peer can
  actually unwrap the blob with its own X25519 key. State-`apply`
  is bit-identical across peers regardless of who can decrypt.
  Whether *this* peer can use the key is queried separately on
  the interaction side via `host.can-open(handle)` or by attempting
  `host.open` and getting an error (lines 220-232, 280-286).
- **MLS group state**, when adopted, lives kernel-side as a typed
  `host.mls` capability bound to an app's group handle. The app
  emits Welcome / Commit / Application events through ordinary
  state propose; the kernel-side MLS engine processes them under
  the requesting peer's identity (lines 294-298).

The `install-key` design is the critical determinism property.
The kernel cannot enforce "MUST NOT branch on return" if
`install-key` returned a bool, because state-`apply` would then
have a peer-local branch. Returning `()` removes the branch
entirely; per-peer decryptability is custodied privately and
queried separately on the interaction side.

The exact `host.seal` / `host.open` / `host.mls` interface,
key-derivation strategy, and persistence story are deferred to a
crypto-and-key-custody child spec (PR #636 line 307). What the
master spec commits to is **placement: encryption is a kernel
capability bound to opaque key handles, not an app concern.**

## Lift-into-Myrhiza notes

- **Primitives:** ChaCha20-Poly1305, X25519, Ed25519, BLAKE3, HKDF-SHA256,
  versioned HKDF domain separators. Direct lift; well-tested in
  `willow-crypto`.
- **DoS-bounded ratchet decryption:** `MAX_RATCHET_LOOKAHEAD` and
  pre-check before HKDF work — direct lift. Cheap, regression-tested,
  closes a real CPU DoS.
- **Epoch-rotation pattern:** rotation-as-its-own-DAG-event,
  triggered-by-membership-change, topic-ID rotates per epoch.
  Direct lift conceptually; under PR #636 the rotation event is
  app-defined, the kernel only records handle binding via
  `host.install-key`.
- **MLS adoption deferred but committed:** Myrhiza inherits Willow's
  decision to wait for MLS-over-Willow rather than ship NIP-17-shaped
  seal+gift-wrap. The lessons captured (deniability, inbox-topic
  leak, ephemeral-author DAG pollution) belong in the MLS-over-Willow
  child spec when it is written.
- **Kernel boundary commitment from PR #636:** components hold
  handles, kernel custodies bytes; `install-key` returns `()` so
  state-`apply` has no peer-local branch; `host.can-open` /
  `host.open` are interaction-only. This is a *placement*
  commitment, not an API. The exact WIT signatures, MLS engine
  integration, and persistence story are the deferred
  crypto-and-key-custody child spec — Myrhiza must own writing it.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- Willow repo: `/mnt/storage/projects/willow`
- `crates/crypto/src/lib.rs` — `ChannelKey`, `KeyRatchet`, `RatchetCache`, `seal_content`, `open_content`, `open_content_bounded`, `encrypt_channel_key_for`, `decrypt_channel_key`, `MAX_RATCHET_LOOKAHEAD`, all HKDF domain constants
- `crates/crypto/src/sas.rs`, `sas_wordlist.rs` — SAS verification (out of scope this file; flagged for future)
- `crates/messaging/src/` — `Content`, `SealedContent` (re-exported via `willow-crypto`)
- `docs/specs/2026-04-24-epoch-key-rotation.md` — `RotateChannelKeyV2`, trigger table, threat model
- `docs/specs/2026-04-24-seal-gift-wrap-dms.md` — DM deferral, MLS-over-Willow rationale, lessons captured
- PR #636 §"Crypto and key custody" (lines 266-310), §"Determinism, in detail" (lines 191-264), §"Constraints we accept" — `host.install-key` semantics, `host.can-open` placement
- `willow CLAUDE.md` § Message Flow, § Architecture Notes (Authority Model)
