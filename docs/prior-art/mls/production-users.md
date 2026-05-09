**Date:** 2026-05-09
**Status:** active
**Subject:** Verified production deployments of MLS (RFC 9420) — who actually ships it, at what scale, on what implementation.

> Cross-refs: [protocol.md](./protocol.md), [openmls.md](./openmls.md), [glossary.md](./glossary.md), [comparisons.md](./comparisons.md), [lessons.md](./lessons.md), [governance.md](./governance.md).

The bar for "production" in this file is **shipping in a generally-available product to non-employee end users in 2026**. Pilots and betas are flagged as such. Every claim is anchored to a primary source (engineering blog, security whitepaper, IETF presentation, or the vendor's own protocol whitepaper). Press-only claims are marked.

## 1. Wire (Wire Swiss GmbH)

**Status:** GA, RFC 9420.
**Date shipped:** **2025-04-24** (general availability).
**Implementation:** [`wireapp/core-crypto`](https://github.com/wireapp/core-crypto) — Rust, wraps OpenMLS, exposes Kotlin/Swift/WASM bindings via UniFFI; persistent state in SQLCipher (mobile/desktop) or IndexedDB+AES-256-GCM (web).
**Scope:** 1:1 and group messaging, calling, conferencing, file sharing — all over MLS. Wire describes itself as "the first and only complete enterprise collaboration suite … completely secured by MLS."
**Scale at GA:** up to 2,000 group members × up to 8 devices each; calls up to 150. Migration from Proteus completed within 24–48 hours of the final wave; Proteus retired post-migration.
**Caveats:** Wire flagged "heavy loading on cloud back-end servers" during the rollout. They have publicly acknowledged client-side teething issues being addressed iteratively.
**Standards posture:** Wire helped initiate the IETF WG (2016); Raphael Robert is an RFC 9420 author and ex-Wire Head of Security.

## 2. Cisco Webex

**Status:** Production for E2EE meetings; RFC 9420 upgrade in progress (per Cisco's own blog, the deployment was originally on a draft and was to be upgraded post-RFC).
**Date shipped:** Draft-MLS Zero Trust Security for Webex Meetings shipped before 2023-07; "we look forward to upgrading Webex to the RFC version later this year" (Cisco blog, 2023-07-17, last-updated 2024-09-18).
**Scope:** End-to-end encrypted **meetings** (Webex Meetings) — voice, video, screenshare. Uses the MLS *epoch authenticator* as the security code shown to participants. Orchestration server holds a copy of the ratchet tree and applies the much smaller commit deltas; new joiners pull the full tree (a "few seconds" even at thousand-attendee scale).
**Implementation:** Cisco's own; not publicly-released as a standalone library.
**Standards posture:** Cisco employs MLS authors (Barnes, Mahy). The PQ-ciphersuites I-D is a Cisco draft.

## 3. Discord — DAVE protocol (audio + video only)

**Status:** GA for voice/video calls and Go Live streams.
**Date shipped:** **2024-09**, ramping to a preferred-by-default rollout, with enforcement planned for 2025. Whitepaper at [daveprotocol.com](https://daveprotocol.com/); audited by Trail of Bits.
**Scope:** **DM/GDM calls, server voice channels (excluding Stage), Go Live streams.** **Text messages on Discord are *not* end-to-end encrypted** and remain subject to content moderation.
**MLS specifics:** MLS 1.0 / RFC 9420; ciphersuite `DHKEMP256_AES128GCM_SHA256_P256`. MLS handles group key exchange; per-sender symmetric keys then encrypt media via WebRTC encoded transforms.
**Why this matters:** Discord is the largest consumer-scale MLS deployment by user count, even though only the realtime A/V plane is encrypted.

## 4. RCS Universal Profile 3.0 (GSMA / Google / Apple)

**Status:** Spec finalized **March 2025**; **limited rollout / beta as of 2026** on both Google Messages and iOS 26.4–26.5 betas. Initial rollout: en-locale only, US/UK/CA/AU/IE/IN/NZ.
**Driver:** GSMA mandated MLS as the cryptographic foundation of RCS Universal Profile 3.0 — the first cross-vendor consumer messaging spec built on MLS.
**Confirmation:**
  - **Google:** announced MLS intent for Messages in 2023; testing MLS in Google Messages reported July 2025; "limited rollout" as of early 2026.
  - **Apple:** committed publicly in March 2025; Senior Engineering Manager Emad Omara (an RFC 9420 author) confirmed E2EE RCS in iOS 26.5 beta.
**Honest caveat:** This is positioned as "the first large-scale messaging service with interoperable E2EE across client implementations from different providers" but is **not yet GA at scale** in 2026 — both implementations are in beta/limited rollout.

## 5. Apple iMessage (proper)

**Status:** **No public confirmation that the native iMessage protocol uses MLS.** Apple's Contact Key Verification (CKV) is a key-transparency layer over the existing iMessage encryption stack (PQ3 since 2024, which is Apple's own protocol — *not* MLS). Apple's MLS exposure is via RCS Universal Profile 3.0 (item 4), **not** the green-bubble-vs-blue-bubble iMessage path.
**Verification:** No Apple security guide as of 2026-05 documents MLS in iMessage proper. Treating any "iMessage uses MLS" claim as **(unverified)**.

## 6. Meta — WhatsApp / Messenger / Instagram

**Status:** **No public production deployment of MLS.**
**Verification:** Meta engineering blogs through 2026-01 cover Rust-at-scale and Private Processing but **do not announce MLS** in WhatsApp or Messenger. Meta is the EU DMA's sole designated messaging gatekeeper (WhatsApp), so MIMI-driven MLS adoption is plausible but not yet announced. Jon Millican (Meta) is an RFC 9420 co-author; Joël Alwen (formerly Meta, now AWS) is the principal academic on TreeKEM/CGKA. Authorship is not deployment.
**Status flag:** **(no primary source for Meta production MLS)**.

## 7. Matrix / Element

**Status:** **Not MLS in production.** Matrix's E2EE is Megolm/Olm. There has been research and prototyping interest in MLS for Matrix but no shipped Matrix-MLS product on matrix.org. **(unverified for production — no primary source located)**.

## 8. MIMI bridges

**Status:** **No production deployments.** All three core MIMI drafts (`content-08`, `protocol-06` "MIMI using HTTPS and MLS", `room-policy-03`) are still WG documents; some have IESG submission targets in 2025–2026 but none has been ratified. Production cross-app MLS interop is **not** a 2026 reality.

## 9. Honest scale assessment (2026-05)

| Vendor | Status | RFC version | Plane covered | User-scale tier |
|---|---|---|---|---|
| Wire | GA (Apr 2025) | RFC 9420 | All planes (msg/call/conf/files) | Enterprise-scale, low-millions |
| Cisco Webex | Production, upgrading to RFC | Draft → RFC 9420 | E2EE meetings | Enterprise-scale |
| Discord DAVE | GA (Sep 2024) | RFC 9420, MLS 1.0 | A/V + Go Live only | **Hundreds of millions** of users on the platform; A/V subset |
| RCS UP 3.0 (Google + Apple) | Beta / limited rollout | RFC 9420 via GSMA spec | RCS messages, 1:1 + group | Beta — not yet large-scale |
| Apple iMessage | Not MLS | n/a | n/a | n/a |
| WhatsApp / Messenger | Not announced | n/a | n/a | n/a |
| Matrix | Research only | n/a | n/a | n/a |
| MIMI bridges | WG drafts only | n/a | n/a | n/a |

**Summary for spec authors.** As of 2026-05, MLS clears the "production-grade" bar: it has a multi-year shipping deployment at Wire across all comms planes, a niche-but-massive deployment at Discord (A/V), and a vendor-cooperated consumer rollout via RCS UP 3.0 in beta on both Google and Apple. The "everyone uses MLS" framing common in press coverage is overstated — WhatsApp, iMessage proper, and Matrix do **not** ship MLS today. For Myrhiza's purposes, MLS is **proven for the group-key-agreement substrate** but **unproven as a cross-vendor interop substrate** until MIMI ratifies and at least one DMA-driven bridge ships.

## Sources

- RFC 9420, *The Messaging Layer Security (MLS) Protocol* — https://www.rfc-editor.org/rfc/rfc9420.html
- Wire blog, "MLS General Availability Announcement" — https://wire.com/en/blog/wire-mls-is-now-generally-available
- Wire blog, "From Vision to Reality: Redefining Secure Collaboration With MLS" (2025-04-30) — https://wire.com/en/blog/redefining-secure-collaboration-with-mls
- Wire blog, "Messaging Layer Security (MLS): The Future of Secure Collaboration" (2025-12-02) — https://wire.com/en/blog/messaging-layer-security-mls-explained
- Wire support, "Messaging Layer Security (MLS)" — https://support.wire.com/hc/en-us/articles/12434725011485-Messaging-Layer-Security-MLS
- `wireapp/core-crypto` — https://github.com/wireapp/core-crypto
- Cisco Webex blog, "How Messaging Layer Security Enables Scalable End-to-End Security in Webex" (2023-07-17, upd. 2024-09-18) — https://blog.webex.com/collaboration/hybrid-work/scalable-end-to-end-security-in-webex/
- Discord DAVE protocol whitepaper — https://daveprotocol.com/
- `discord/dave-protocol` — https://github.com/discord/dave-protocol
- Discord blog, "Meet DAVE: Discord's New End-to-End Encryption for Audio & Video" — https://discord.com/blog/meet-dave-e2ee-for-audio-video
- The Hacker News, "GSMA Confirms End-to-End Encryption for RCS" (2025-03) — https://thehackernews.com/2025/03/gsma-confirms-end-to-end-encryption-for.html
- 9to5Google, "RCS update adds end-to-end encryption" (2025-03-14) — https://9to5google.com/2025/03/14/rcs-end-to-end-encryption-update/
- 9to5Google, "Google Messages testing RCS' new MLS encryption" (2025-07-16) — https://9to5google.com/2025/07/16/google-messages-rcs-mls/
- 9to5Mac, "Apple confirms iOS 26.5 Messages app adds RCS end-to-end encryption" (2026-05-04) — https://9to5mac.com/2026/05/04/apple-confirms-ios-26-5-messages-app-adds-rcs-end-to-end-encryption/
- IETF MIMI WG (current drafts) — https://datatracker.ietf.org/wg/mimi/about/
