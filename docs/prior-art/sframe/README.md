**Date:** 2026-05-22
**Status:** active
**Subject:** SFrame (RFC 9605) — per-frame authenticated encryption for real-time media, designed to layer above an MLS group key.

# SFrame — prior art

SFrame (Secure Frames) is the IETF answer to "MLS gave you a group key — now how do you encrypt the *video*?" It is a media-frame transform: an AEAD pass applied to encoded codec output, with per-sender keys derived (typically) from an MLS exporter. It is transport-independent — *not* a profile of RTP/SRTP — and was published as **RFC 9605** in August 2024 after four years of incubation through the SFrame WG.

This folder captures SFrame as Myrhiza's anticipated answer to E2EE A/V on top of MLS-shaped channels. The corpus is **speculative until Myrhiza ships voice/video capabilities** — it becomes load-bearing the moment a channel needs to carry media frames.

## Key facts

| | |
|---|---|
| Specification | [RFC 9605](https://www.rfc-editor.org/rfc/rfc9605.html) — *Secure Frame (SFrame): Lightweight Authenticated Encryption for Real-Time Media* |
| Status | Standards Track, published **August 2024** |
| IETF Working Group | SFrame WG (concluded). Chairs: Bobo Bose-Kolanu, Martin Thomson |
| Authors | E. Omara (Apple), J. Uberti (Fixie.ai), S. G. Murillo (CoSMo Software), R. Barnes Ed. (Cisco), Y. Fablet (Apple) |
| Predecessor | `draft-omara-sframe` (individual, 2020-05-19) → `draft-ietf-sframe-enc` 00–09 (2022-07-29 → 2024-04-04, ~24 months) |
| Ciphersuites | 5 total: `AES_128_CTR_HMAC_SHA256_{80,64,32}`, `AES_128_GCM_SHA256_128`, `AES_256_GCM_SHA512_128` |
| Key derivation | HKDF with labels `"SFrame 1.0 Secret key"` / `"SFrame 1.0 Secret salt"` from an external `base_key` |
| MLS coupling | Optional but canonical: `base_key = MLS-Exporter(...)` per RFC 9605 §5.2 |
| Relation to SRTP | **Alternative** to SRTP for media protection, not a layer inside SRTP. Transport-independent by design |
| Production deployments | Webex (since pre-2023 on draft-01); no other public production at-scale deployment that uses RFC 9605 verbatim |
| Production using the same pattern | Discord DAVE (since 2024-09-17) — implements the **MLS-exporter → per-sender frame transform** *pattern* without using RFC 9605 |

## Contents

1. **[sframe-spec.md](sframe-spec.md)** — RFC 9605 technical summary: payload format, header, ciphersuites, key schedule, MLS exporter integration.
2. **[deployments.md](deployments.md)** — Webex (the canonical RFC-aligned deployment) and Discord DAVE (the at-scale-but-divergent deployment). Honest about adoption.
3. **[open-problems.md](open-problems.md)** — what SFrame doesn't solve: metadata leakage, leaver-recovery latency, key-rotation cost, signalling out-of-scope, SFU side-channels.
4. **[myrhiza-relevance.md](myrhiza-relevance.md)** — when SFrame becomes load-bearing for Myrhiza, what we'd need to do at the kernel/capability boundary, runner-up paradigms.
5. **[lessons.md](lessons.md)** — the consult-this-when-designing decision file. validates / avoid / borrow.

## How to use

If you are designing or auditing anything that puts audio/video frames on a Myrhiza channel — read [`lessons.md`](lessons.md) and [`myrhiza-relevance.md`](myrhiza-relevance.md) first. Then [`sframe-spec.md`](sframe-spec.md) for the protocol shape. Then [`deployments.md`](deployments.md) to calibrate what "at scale" actually looks like.

If you are evaluating whether MLS is the right group-key primitive for capability streams that carry media — read [`open-problems.md`](open-problems.md) alongside [`prior-art/mls/open-problems.md`](../mls/open-problems.md). SFrame's open problems and MLS's open problems intersect heavily at the leaver-recovery / key-rotation latency boundary, and that intersection is where most of the engineering work lives.

**Framing disclosure.** These docs are written from a Myrhiza-as-MLS-channels stance — most "Implications for Myrhiza" sub-sections frame SFrame's choices through the lens of "we already use MLS, so SFrame is the obvious complement." That framing reflects the position of [`prior-art/mls/open-problems.md:45-47`](../mls/open-problems.md), which named SFrame as the IETF answer for media-on-MLS. Future readers auditing whether MLS-channels-for-everything is itself the right primitive should weigh this corpus accordingly: it's a learn-from-SFrame-into-MLS-channels artifact, not a neutral catalog of E2EE-media options.

**Speculative-until-needed disclosure.** Nothing in Myrhiza ships A/V today. Until a capability needs to carry media frames, this folder is *preparatory* — it exists so that a future spec author starting from "how do we E2EE voice on a channel?" doesn't have to redo the research. Treat the load-bearing claims as design hypotheses, not commitments.

## Glossary stub

- **AEAD** — Authenticated Encryption with Associated Data; the cryptographic primitive (AES-GCM, AES-CTR+HMAC) SFrame composes.
- **base_key** — the externally-supplied key SFrame derives per-sender keys from. In the MLS integration, this is an MLS exporter output.
- **CTR mode** — counter-mode AES; the underlying cipher for SFrame's three CTR ciphersuites (paired with HMAC for authentication).
- **DAVE** — Discord Audio & Video Encryption; Discord's production E2EE-media protocol since 2024-09-17. Uses MLS + a frame transform that is *not* RFC 9605 SFrame.
- **HKDF** — HMAC-based Key Derivation Function (RFC 5869); SFrame's key-schedule primitive.
- **KID** — Key ID; the SFrame header field identifying which key was used. Distinguishes senders within a group.
- **MLS exporter** — MLS group key material exported for use by non-MLS subsystems. RFC 9420 §8.5.
- **PERC** — Privacy-Enhanced RTP Conferencing; an earlier IETF effort whose "double encryption inside SRTP" approach the SFrame WG charter cites as the *failure* SFrame addresses.
- **SFU** — Selective Forwarding Unit; a server that forwards media streams between conference participants without decrypting them. SFrame's primary deployment shape.
- **SRTP** — Secure RTP (RFC 3711); the legacy media-encryption profile. SFrame is an *alternative*, not a layer inside.

## Cross-links

- Parent / source-of-citation: [`prior-art/mls/`](../mls/) — MLS exporter is SFrame's load-bearing input. [`prior-art/mls/open-problems.md:45-47`](../mls/open-problems.md) is the canonical citation that motivated this folder.
- Comparator on real-time-media E2EE: [`prior-art/signal/`](../signal/) — Signal's Calling stack uses different primitives.
- WebRTC-using-MLS-shape-without-SFrame data point: [`prior-art/pears/`](../pears/) — Keet uses WebRTC over Hyperswarm but does not adopt SFrame.

## Sources

- [RFC 9605 — Secure Frame (SFrame)](https://www.rfc-editor.org/rfc/rfc9605.html)
- [IETF SFrame WG charter](https://datatracker.ietf.org/wg/sframe/about/)
- [IETF Datatracker — draft-ietf-sframe-enc](https://datatracker.ietf.org/doc/draft-ietf-sframe-enc/)
- [IETF Datatracker — draft-omara-sframe](https://datatracker.ietf.org/doc/draft-omara-sframe/)
- [Discord DAVE protocol whitepaper](https://daveprotocol.com)
- [GitHub — discord/dave-protocol](https://github.com/discord/dave-protocol)
- [Webex blog — Scalable End-to-End Security in Webex (Richard Barnes, 2023-07-17, updated 2024-09-18)](https://blog.webex.com/collaboration/hybrid-work/scalable-end-to-end-security-in-webex/)
- [RFC 3711 — Secure RTP](https://www.rfc-editor.org/rfc/rfc3711.html)
- [RFC 9420 — MLS Protocol](https://www.rfc-editor.org/rfc/rfc9420.html)
- [`prior-art/mls/open-problems.md`](../mls/open-problems.md)
