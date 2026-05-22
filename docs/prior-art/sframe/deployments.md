**Date:** 2026-05-22
**Status:** active
**Subject:** SFrame in production — Webex (RFC-aligned) and Discord DAVE (pattern-aligned but not RFC).

# SFrame deployments

Two production deployments matter: **Webex** (Cisco's MLS + SFrame meeting stack) and **Discord DAVE** (the only at-scale consumer voice/video E2EE deployment). Their relationships to RFC 9605 are different and the difference is load-bearing.

## 1. Webex — the RFC-aligned canon

| Field | Value |
|---|---|
| Vendor | Cisco |
| Product | Webex Meetings (Zero Trust Security for Webex) |
| First production ship | Before 2023-07 (per Richard Barnes' blog post 2023-07-17, "*we look forward to upgrading Webex to the RFC version later this year*") |
| Initial protocol version | `draft-ietf-sframe-enc-01` (cited verbatim in Webex blog) |
| Current protocol version | Migrating to RFC 9605 + RFC 9420 (final state unverified in public sources as of 2026-05) |
| MLS Delivery Service | Centralized; Webex orchestration server holds a copy of the ratchet tree |
| Author of architecture | Richard Barnes (Cisco Chief Security Architect for Collaboration; also SFrame WG editor and RFC 9605 co-author) |

Webex is the deployment SFrame was *designed for*. Barnes is both the editor of the RFC and the architect of the production system. The vertical integration shows: the Webex blog post's diagrams are essentially the SFrame WG slide deck with product chrome added.

Architecture, per Barnes' blog:

1. MLS handles per-meeting group key agreement. Every meeting is its own MLS group.
2. SFrame applies to each codec frame using a `base_key` derived via MLS exporter.
3. The Webex orchestration server holds the ratchet tree and applies commit deltas; new joiners pull the full tree ("*a few seconds even at thousand-person meeting*" scale).
4. The MLS *epoch authenticator* — a short digest of the current group state — is displayed as the meeting security code that participants can verify out-of-band.

Webex is the only public deployment that uses RFC 9605 (or its draft-01 predecessor) verbatim, with no proprietary frame format on top.

## 2. Discord DAVE — the pattern-aligned divergence

| Field | Value |
|---|---|
| Vendor | Discord |
| Product | All voice/video calls (DM/GDM calls, server voice channels excluding stage channels, Go Live streams) |
| Announced | 2024-09-17 (DAVE protocol v1.0 initial commit in `discord/dave-protocol`) |
| Current spec version | v1.1.4 (2025-08-14, per repo) |
| MLS version | RFC 9420 (MLS 1.0) |
| MLS ciphersuite | `DHKEMP256_AES128GCM_SHA256_P256` (MLS ciphersuite 2) |
| Frame transform | **Proprietary** — not RFC 9605 |
| Exporter label | `"Discord Secure Frames v0"` (not `"SFrame 1.0"`) |
| Audit | Trail of Bits security review (referenced in Discord communications; specific report PDF not located in public Trail of Bits publications repo as of this writing) |
| Reach | Discord's full voice/video user base. Discord-published MAU figures (~200M+) — but only those on supporting clients use DAVE |

### What DAVE shares with SFrame

The *design pattern* is identical: MLS group → MLS exporter → per-sender ratcheted symmetric keys → AEAD-encrypted media frames. Discord engineers clearly read RFC 9605 (or its drafts) when designing this — the architectural shape mirrors §5.2 of the RFC.

### What DAVE *doesn't* share with SFrame

The frame transform format is entirely Discord's own. Verbatim from the DAVE whitepaper:

- Exporter label: `"Discord Secure Frames v0"` instead of `"SFrame 1.0"`.
- Per-sender base secret: `sender_base_secret = MLS-Exporter("Discord Secure Frames v0", littleEndianSenderID, 16)` — context is the **sender's user ID** as little-endian 64 bits, not the SFrame-style KID-derived input.
- Wire format: 0xFAFA magic marker + ULEB128-encoded nonce + ULEB128 unencrypted-range pairs + 1-byte supplemental data size + truncated 64-bit AES-GCM tag. This is **not** RFC 9605's header format.
- Codec-awareness: DAVE explicitly leaves codec-required header ranges (e.g. Opus TOC byte, VP8/H.264 frame metadata) *unencrypted* to preserve compatibility with SFU forwarding logic.
- Cipher: only `AES128-GCM`, with the auth tag truncated to 64 bits per NIST SP 800-38D.

The DAVE whitepaper makes **no reference** to RFC 9605, draft-ietf-sframe-enc, or draft-omara-sframe — verified via verbatim grep against `protocol.md` at `discord/dave-protocol@main`. The repo says it cites "RFC 9420 (MLS)" and "RFC 8446 (TLS presentation language)" but contains zero SFrame citations.

### Why this matters

It is tempting (and the user-facing brief explicitly warns against this) to describe Discord DAVE as "Discord's SFrame deployment." It is more accurately: **Discord's MLS-derived frame transform that uses the SFrame pattern but ships a Discord-specific codec-aware wire format.** The distinction matters because:

1. **Interop:** DAVE clients cannot decrypt SFrame frames and vice versa. The two are not protocol-compatible.
2. **Security audit transfer:** RFC 9605's security analysis applies to RFC 9605. The Trail of Bits review of DAVE is the relevant artifact for DAVE's security claims; an SFrame proof does not automatically cover DAVE.
3. **Specification drift:** If Discord wanted to migrate DAVE-clients to RFC 9605 SFrame frames, that would be a wire-format break and a forced client upgrade.

Why did Discord diverge? The DAVE whitepaper does not say. Plausible reasons (informed speculation, flagged as such):

- DAVE shipped 2024-09-17; RFC 9605 published 2024-08 — DAVE's design clearly predates the final RFC by months and likely tracked an earlier draft.
- DAVE's codec-aware unencrypted-range mechanism is a real engineering need (SFUs need to read at least *some* per-frame metadata for forwarding decisions, jitter buffer hints, and FEC). RFC 9605 does not specify codec-aware ranges; it treats the frame as opaque. Discord's format is a reasonable extension at the cost of standardization.
- Discord owns its client + SFU stack end-to-end; the interop premium that standards offer is lower for a vertically-integrated single-vendor deployment.

## 3. The honest adoption picture

As of 2026-05-22:

- **Webex** is the only at-scale deployment that uses RFC 9605 (or its drafts) **as the wire format**.
- **Discord DAVE** is the only at-scale consumer-grade deployment of E2EE voice/video on MLS, but it uses a Discord-specific frame transform.
- **Google Meet** has expressed interest in MLS but has not publicly committed to RFC 9605 SFrame as of this date; verify before citing.
- **WhatsApp** ships group-call E2EE on Signal-protocol-derived primitives, not MLS/SFrame.
- **iMessage / FaceTime** use Apple's own PQ3 protocol; no public MLS or SFrame integration.

So when Myrhiza considers "the SFrame ecosystem," the load-bearing data point is one production deployment using RFC 9605 (Webex), one production deployment using the pattern with a custom transform (Discord DAVE), and a lot of WG and academic interest. This is a small-N, learn-from-the-shipped-examples situation, not a battle-tested-by-billions-of-users situation.

The DAVE-vs-SFrame divergence is also a cautionary signal: standardization happened, but the largest consumer deployment found enough friction in the standard to ship something custom. That tension is worth carrying into any Myrhiza decision about whether to wrap RFC 9605 verbatim or design our own frame transform on top of MLS.

## 4. Sources

- [Discord DAVE protocol whitepaper](https://daveprotocol.com)
- [GitHub — discord/dave-protocol](https://github.com/discord/dave-protocol)
- [discord/dave-protocol — commit history showing v1.0 on 2024-09-17 and v1.1.4 on 2025-08-14](https://github.com/discord/dave-protocol/commits/main)
- [Webex blog — How MLS Enables Scalable End-to-End Security (Richard Barnes, 2023-07-17, last updated 2024-09-18)](https://blog.webex.com/collaboration/hybrid-work/scalable-end-to-end-security-in-webex/)
- [Cisco — Zero-Trust Security for Webex white paper](https://www.cisco.com/c/en/us/solutions/collateral/collaboration/white-paper-c11-744553.html)
- [RFC 9605 — Secure Frame (SFrame)](https://www.rfc-editor.org/rfc/rfc9605.html)
- [`prior-art/mls/other-implementations.md` §1 — mlspp and Webex](../mls/other-implementations.md)
- [`prior-art/mls/production-users.md` §2 — Cisco Webex](../mls/production-users.md)
