**Date:** 2026-05-22
**Status:** active
**Subject:** What SFrame structurally doesn't solve — metadata, leaver-recovery latency, key-rotation, SFU side-channels, signalling.

# SFrame — open problems

SFrame is a small, focused spec. That focus is its strength but also a list of things it deliberately doesn't solve. A reader who comes away thinking "SFrame solves E2EE for video calls" is wrong; SFrame solves *per-frame authenticated encryption keyed off an external secret*. Everything else in an E2EE-video-call stack — group key, signalling, leaver-recovery, metadata, SFU behaviour — is downstream of the spec and is where the actual hard engineering lives.

## 1. Metadata leakage to the SFU

The SFrame ciphertext is opaque, but the SFU still sees:

- **Frame sizes.** Codec output is highly variable per frame (I-frames vs P-frames; voice activity); a passive observer can identify keyframes, talker detection, and likely activity-level patterns.
- **Frame timing.** RTP timestamps and arrival cadence reveal who is speaking, when video is active, when someone unmutes.
- **Sender identity within the group.** SFU forwards by stream — the SFU knows which member sent which frame even if it can't decrypt.
- **Network metadata.** IP addresses, packet sizes, retransmission patterns.

SFrame's threat model (RFC 9605 §1 introduction, §9 Security Considerations) is explicit that the SFU does not need access to media content but does retain access to RTP metadata, with substantial metadata inference possible from that surface. (The RFC does not use the phrase "honest-but-curious" — that's the academic term-of-art for this threat model, not RFC 9605's wording.) Padding or constant-rate transmission could mitigate but are not part of the spec.

For Myrhiza: if Myrhiza's threat model includes "the SFU/relay is hostile and can identify talkers," SFrame is insufficient. The mitigations (cover traffic, fixed-rate audio, padding) cost real bandwidth and are not standardized.

## 2. Leaver-recovery latency

When a member is removed from an MLS group, the SFrame key derived from the MLS exporter does not change until the next MLS commit lands. The lag from "removal proposal sent" to "removed member can no longer decrypt frames" is bounded by:

1. Commit propagation latency to the Delivery Service.
2. The DS's commit-application cadence (Webex: roughly real-time; Discord DAVE: governed by their MLS deployment, see DAVE whitepaper §"Rolling Keys").
3. Other members' processing delay for the new epoch.

During the leaver window, the removed member continues decrypting frames sent under the old `base_key`. For a video call, the practical leaver window is on the order of **seconds**, not milliseconds. For most threat models this is acceptable; for "we kicked out someone hostile and want them blind *now*" it isn't.

SFrame's spec doesn't address this — it's an MLS-layer problem inherited by SFrame. See [`prior-art/mls/open-problems.md` §10](../mls/open-problems.md) for the upstream framing.

## 3. Key rotation within an epoch

SFrame derives `base_key` once per MLS epoch. Within an epoch, the per-frame nonces walk through the counter space until the counter is exhausted (2^N where N depends on the suite). For long calls or high-FPS streams this matters:

- AES-GCM nonce space is 96 bits; SFrame compresses this further (the encoded CTR is typically 16–32 bits before extension).
- At 50 fps audio + 30 fps video × multiple streams, 2^24 frames is consumed in ~30 minutes per sender.

The implementer is supposed to trigger an MLS commit (rotating the epoch) before counter exhaustion. Discord DAVE handles this via the "rolling keys" mechanism in §"Sender Key Ratchet" — a key ratchet inside the epoch, so that the 24-bit counter advances the ratchet generation rather than the GCM nonce. RFC 9605 does *not* specify this; it's left to applications.

For Myrhiza: any A/V capability needs a documented "when do we rotate" policy. Borrow DAVE's intra-epoch ratchet shape, or commit MLS-epoch-frequent enough that intra-epoch ratcheting is unnecessary.

## 4. SFU correctness without decryption

SFrame's bet is that the SFU does not need to decrypt frames to do its job. This is mostly true but not entirely:

- **Bandwidth estimation / congestion control.** Modern SFUs use TWCC (transport-wide congestion control) which works on RTP packet metadata — fine, no decryption needed.
- **Simulcast / SVC layer selection.** The SFU needs to know which layer is which to forward the right one. This requires some unencrypted metadata; SFrame leaves it to the codec / RTP layer.
- **Forward Error Correction.** FEC operates on RTP packets, not codec frames; works orthogonally to SFrame.
- **Mute / hold-frame substitution.** The SFU cannot generate plausible silence or freeze-frames without the key — this is a UX loss for some traditional features.
- **Recording.** Server-side recording requires either (a) a participant key, (b) a key escrow, or (c) recording the ciphertext and decrypting client-side later. None are great.

These constraints push the SFU toward a "thinner" role than legacy WebRTC SFUs play. For Myrhiza, if any relaying peer is to play SFU, expect these constraints.

## 5. Signalling out-of-scope

SFrame does not specify how senders advertise their KIDs, how receivers learn that a member's KID has rolled, or how the group agrees on the ciphersuite. RFC 9605 §4.1 explicitly defers this:

> "Like SRTP, SFrame does not define how the keys used for SFrame are exchanged by the parties in the conference."

And §5 opens with: "SFrame must be integrated with an E2E key management framework to exchange and rotate the keys used for SFrame encryption. ... It is the responsibility of the application to provide the key management framework."

For Myrhiza, this means the kernel (or whatever owns MLS state) has to expose KID-management capabilities to applications: rotate, advertise, learn-peer's-KID. There is no standard for this — every deployment invents its own.

## 6. Codec-awareness gap

RFC 9605 treats the codec frame as an opaque blob. Real SFUs need to read *some* per-frame metadata (Opus TOC, VP8/VP9 picture IDs, H.264 NAL headers) for forwarding decisions. Discord DAVE's "interleaved encrypted/unencrypted ranges" feature is exactly the workaround for this gap.

This is a real point of friction between standardization and pragmatism. If Myrhiza wants to forward through commodity SFU software, we either:

1. Adopt DAVE-style codec-aware ranges (and document our extension).
2. Use a tiny custom SFU that doesn't need per-frame codec metadata (and accept the network engineering cost).
3. Use peer-to-peer media only (mesh topology) and skip the SFU question entirely (acceptable for small calls, broken for ten+ participants).

## 7. Post-quantum

RFC 9605's ciphersuites are all AES-based. AES-256 is generally considered PQ-acceptable for confidentiality (Grover's algorithm gives a √N speedup, leaving AES-256 at 128 bits of post-quantum security). HMAC-SHA-256/SHA-512 are similarly considered PQ-fine.

The PQ exposure is at the **MLS layer** (the exporter input), not SFrame itself. Once MLS gets a PQ ciphersuite (`draft-ietf-mls-pq-ciphersuites`, see [`prior-art/mls/`](../mls/)), SFrame inherits it for free because the `base_key` is just opaque bytes from SFrame's perspective.

This is a clean separation-of-concerns win. SFrame doesn't need to be re-spec'd for PQ; it's *already* PQ-ready conditional on MLS being PQ-ready.

## 8. The "SFrame solves E2EE for video calls" misframing

Common shorthand to avoid: "SFrame is the E2EE story for media." It isn't. SFrame is *the per-frame transform* in the E2EE story for media. The full E2EE story includes:

- A group key agreement protocol (MLS, or hand-rolled).
- A leaver-recovery story bounded by the group protocol's commit cadence.
- A key-rotation policy within and across epochs.
- A signalling layer that delivers KIDs and ciphersuite agreement.
- An SFU that operates on ciphertext (or a peer-to-peer topology that skips SFUs).
- Codec-awareness shims so the SFU can route without decrypting.
- An out-of-band verification ceremony (epoch authenticator display, safety numbers).
- A defense-in-depth answer to metadata leakage to the relay.

SFrame solves one of those eight bullets cleanly. The other seven are still your problem. The Webex and DAVE deployments are each ~100KLOC of glue around the SFrame-or-SFrame-shaped transform.

## 9. Reference-grade implementation maturity

As of 2026-05-22 there is no widely-deployed reference RFC 9605 library in any language. Cisco maintains `libsframe` (C++); Google's WebRTC stack has an SFrame branch but its merged status into mainline WebRTC is not publicly documented. The OpenSFrame implementation by Mozilla / by various WG participants exists in fragmentary form. This contrasts sharply with MLS (OpenMLS, mlspp, mls-rs — three robust implementations).

For Myrhiza: implementing SFrame ourselves is plausible (it's a small spec, ~400 lines of crypto code) but means we own the security analysis. Wrapping `libsframe` via FFI is the obvious alternative; we should verify its maturity before committing.

## 10. Sources

- [RFC 9605 §9 — Security Considerations](https://www.rfc-editor.org/rfc/rfc9605.html#section-9)
- [RFC 9605 §4.1 — Key establishment out-of-scope statement](https://www.rfc-editor.org/rfc/rfc9605.html#section-4.1)
- [RFC 9605 §5 — Key Management framework requirement](https://www.rfc-editor.org/rfc/rfc9605.html#section-5)
- [RFC 9605 §5.2 — MLS integration](https://www.rfc-editor.org/rfc/rfc9605.html#section-5.2)
- [Discord DAVE whitepaper — Rolling Keys section](https://daveprotocol.com)
- [`prior-art/mls/open-problems.md`](../mls/open-problems.md)
- [draft-ietf-mls-pq-ciphersuites](https://datatracker.ietf.org/doc/draft-ietf-mls-pq-ciphersuites/)
