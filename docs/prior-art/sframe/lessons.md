**Date:** 2026-05-22
**Status:** active
**Subject:** SFrame lessons for Myrhiza — validates / avoid / borrow. The decision file.

# Lessons from SFrame for Myrhiza

This is the synthesis. Other files are evidence; this is what to do with it. The lessons are **speculative until Myrhiza ships A/V** — they become operative the moment a Myrhiza capability needs to carry media frames.

## Validates

**The MLS-exporter-as-foundation pattern.** RFC 9605's choice to derive media keys from an MLS exporter (rather than re-inventing group key agreement, or sharing MLS message keys directly) cleanly separates the protocols' concerns: MLS handles "who is in the group + when does key roll," SFrame handles "encrypt this frame fast." This is the same separation Myrhiza should expect — MLS state in `state-apply` territory, media encryption in `interaction` / `behavior` territory. Don't try to make the media-encryption layer also re-establish group identity.

**Transport-independent transforms.** The PERC failure (RFC 9605 cites this in its motivation) was specifying the security layer too tightly to SRTP. SFrame walks away from SRTP and is transport-independent — it can wrap RTP, QUIC datagrams, P2P data channels, anything. Myrhiza's capability streams may live on libp2p, WebRTC data channels, raw QUIC, or some combination. The encryption transform must be transport-independent in the same way. **Lesson: any media-protection layer Myrhiza adds must take (key, counter, plaintext, AAD) → ciphertext as its only contract. No coupling to the carrier.**

**Codec-awareness is real engineering.** Discord DAVE's divergence from RFC 9605 is *not* irrationality — they hit a real engineering need (SFUs need codec metadata to forward) that the RFC didn't address. When Myrhiza picks a frame transform, build in codec-awareness from day one. The RFC's "opaque blob" abstraction sounds clean but doesn't survive contact with production SFUs.

**Per-sender ratchets bound exposure.** Both RFC 9605 and DAVE derive per-sender keys from a per-epoch base secret. This means a compromised sender's history is bounded by the epoch boundary, not the entire conversation. Myrhiza inherits this for free if we adopt either approach.

**Short tags are fine for media.** The 32-bit and 64-bit truncated-tag ciphersuites exist because for short-lived audio/video frames, 2^32 forgery resistance is acceptable and the bandwidth saving is meaningful. Don't reflexively pick maximum-security parameters when the threat model doesn't require them — measurement-of-attack-cost matters.

## Avoid

**Conflating SFrame with MLS.** Repeated throughout this corpus and worth restating in lessons. **SFrame is the media-key-derivation + frame-transform.** **MLS is the group-key protocol.** They are independent — SFrame can run on any source of `base_key` material; MLS can be used for things other than media. Conflating them in spec or code review is a category error.

**Conflating Discord DAVE with RFC 9605 SFrame.** DAVE uses the *pattern* (MLS → exporter → per-sender ratchet → frame transform) but **not the wire format**. The exporter label, the payload format, the auth tag length, and the codec-awareness mechanism are all Discord-specific. A reader who treats "Discord shipped SFrame at scale" as a claim about RFC 9605 adoption is wrong; what Discord shipped is the *idea behind* SFrame, with a wire format Discord owns.

**Treating SFrame as "the E2EE video story."** SFrame solves one layer (the per-frame transform). The full E2EE-video stack also needs group key agreement, signalling, leaver-recovery, codec-awareness, SFU behaviour, metadata mitigation, and verification ceremonies. Those other layers are *most* of the engineering. Don't size the A/V capability spec assuming SFrame solves the problem — assume it solves 1/8 of the problem cleanly.

**The PERC mistake — tying security to a single transport.** PERC tried to wedge E2EE into SRTP and it failed because SRTP-everywhere wasn't true. Myrhiza's transport diversity (libp2p, WebRTC, direct UDP for P2P, QUIC for non-P2P, possibly more) means any "tied to one transport" design will repeat PERC's failure. Build transform-shaped, not protocol-extension-shaped.

**Assuming reference-grade implementations exist.** As of 2026-05-22, RFC 9605 does **not** have OpenMLS-grade reference implementations across multiple languages. Cisco's `libsframe` is the most mature; everything else is fragmentary. Don't write a spec that depends on "the SFrame library" being a thing — it isn't, yet.

**Standardizing for interop we don't actually need.** RFC 9605 gives interop with Webex and (eventually) others. Discord skipped it because they don't need interop with Webex. If Myrhiza's interop story is "talks to other Myrhiza peers," the cost of adopting a custom transform is low and the codec-awareness benefit is high. Don't default to RFC adoption just because there's an RFC.

**Assuming the SFU can be trusted with metadata.** Even with SFrame, the SFU sees frame sizes, timing, sender identity, and IP-level metadata. If Myrhiza's threat model includes "the relay/SFU is hostile," SFrame is insufficient — you need cover traffic, padding, or constant-rate transmission. The crypto layer doesn't fix this.

## Borrow

**The DAVE divergence shape.** If Myrhiza adopts a DAVE-style custom transform (likely outcome per [`myrhiza-relevance.md` §2](myrhiza-relevance.md)), copy:
- The exporter label pattern (`"<Project> Secure Frames v0"` — Myrhiza-namespaced, with a v0 marker so versioning is visible from day one).
- The per-(epoch, sender) base secret derivation.
- The codec-aware unencrypted-range mechanism.
- The intra-epoch key ratchet (so we don't have to commit MLS at media cadence).
- The wire format magic marker for clean framing.

**The kernel-owns-keys principle.** Both Webex and DAVE keep keys in the trusted core, not in the application. Myrhiza's kernel-as-broker model maps onto this directly — make MLS state and SFrame key derivation a kernel capability, not an app-side library. Apps get `encrypt(frame)` and `decrypt(frame)` capabilities, not key material.

**Epoch-authenticator verification ceremony.** Both Webex and DAVE expose the MLS epoch authenticator as a short out-of-band verification string. Myrhiza should expose the same for any group with media — it's free safety value and matches Signal's "safety numbers" UX pattern that users have been trained on.

**Defensive ciphersuite pinning.** DAVE pins the MLS ciphersuite at group creation; downgrade attacks via mid-call ciphersuite renegotiation are prevented at the protocol level. Myrhiza should do the same.

**Conservative defaults.** RFC 9605 ships five ciphersuites but Webex and DAVE both deploy primarily `AES_128_GCM_SHA256_128`-equivalent. That's a reasonable Myrhiza default — pick one strong ciphersuite, ship it, don't ship a configurable matrix from day one.

**Honest scope communication.** RFC 9605 §4.1 explicitly says "Like SRTP, SFrame does not define how the keys used for SFrame are exchanged by the parties in the conference." and §5 reiterates that "[i]t is the responsibility of the application to provide the key management framework." This kind of in-spec scope-honesty is excellent — it prevents readers from over-claiming. Myrhiza's A/V spec, when it lands, should be similarly honest about what it does and doesn't solve.

## Decision flag

When a Myrhiza spec author hits "we need to encrypt media frames on this capability stream":

1. Read [`myrhiza-relevance.md`](myrhiza-relevance.md) — the candidate paradigms (RFC 9605 verbatim, DAVE-style custom, pure-peer mesh).
2. Read [`open-problems.md`](open-problems.md) — what the transform doesn't solve (leaver-window, metadata, signalling).
3. Read [`prior-art/mls/open-problems.md` §10](../mls/open-problems.md) — the upstream MLS-side framing.
4. Then write the A/V capability spec. Cite this folder, name the runner-up paradigms in the spec, flag the codec-awareness decision explicitly.

## Sources

- [RFC 9605](https://www.rfc-editor.org/rfc/rfc9605.html)
- [Discord DAVE whitepaper](https://daveprotocol.com)
- [Webex blog — Scalable E2E Security in Webex (Barnes, 2023)](https://blog.webex.com/collaboration/hybrid-work/scalable-end-to-end-security-in-webex/)
- [`prior-art/mls/open-problems.md`](../mls/open-problems.md)
- [`prior-art/mls/lessons.md`](../mls/lessons.md)
- [`sframe-spec.md`](sframe-spec.md), [`deployments.md`](deployments.md), [`open-problems.md`](open-problems.md), [`myrhiza-relevance.md`](myrhiza-relevance.md)
