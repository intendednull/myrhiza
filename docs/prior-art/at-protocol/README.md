**Date:** 2026-05-22
**Status:** active
**Subject:** AT Protocol (atproto) + Bluesky — federated client-server protocol with DID-based identity, separated rotation/signing keys, and Lexicon-typed records

# AT Protocol (atproto) / Bluesky prior art

Folder of reference material on AT Protocol — the federated protocol that powers Bluesky — and Bluesky PBC, its primary steward and operator. The single most relevant deployed system for Myrhiza's **multi-device identity** open problem: AT Protocol is the only at-scale production system that has separated **long-term identity** from **active signing key** in a way that survives device compromise and host migration. That separation lives in `did:plc`, the DID method Bluesky built when it didn't like the existing options.

This is **not a P2P system**. It is a federated client-server protocol with strong centralization gravity: as of early 2026 roughly 99% of users are on Bluesky-operated infrastructure, the `plc.directory` registry is a single-operator service, and the "Relay" tier is resource-heavy enough that only a handful of independent operators run one. Treat the corpus accordingly — atproto's identity primitives are excellent prior art; its federation story is a cautionary tale about what "federation" looks like when only one operator can afford the central tier.

15 files, target ~1,800-2,200 lines.

## Key facts at a glance

| Field | Value |
|---|---|
| Stewards | **Bluesky Social PBC** (Delaware Public Benefit Corporation, incorporated October 2021, became benefit corp February 2022) |
| CEO (current) | **Toni Schneider** (interim, since 2026-03-09); **Jay Graber** is now Chief Innovation Officer |
| Founder | Jay Graber (joined August 2021 from Happening; project itself originated 2019 as Jack Dorsey-era Twitter initiative "bluesky") |
| Series B | **March 2026**: $100M round; total user base grew "from 13 million to over 43 million global users" since prior raise |
| User scale | **42.3M registered / ~27.5M MAU / ~3.68M DAU** as of February 2026 (third-party estimate — Bluesky does not publish official MAU) |
| Repo | [`bluesky-social/atproto`](https://github.com/bluesky-social/atproto) — TypeScript reference impl, 9.4k stars, ~2,940 commits, **dual MIT/Apache-2.0** |
| Indigo (Go relay/PDS) | [`bluesky-social/indigo`](https://github.com/bluesky-social/indigo) — Go implementation of the Relay (formerly named `bigsky`), MIT/Apache-2.0 |
| Specs | [atproto.com/specs](https://atproto.com/specs) — see [architecture.md](architecture.md), [identity.md](identity.md), [lexicon.md](lexicon.md) for snapshot dated 2026-05-22 |
| IETF status | **ATP Working Group** formed November 2025 at IETF 124 (Montreal BoF); first drafts target repository data structure + sync protocol, to be split from `draft-holmgren-at-repository` |
| Federation status | **Federation-shaped, not P2P**: ~99% of users on Bluesky PBC PDSes; third-party PDSes operate but are a long tail; only a handful of independent Relays exist (resource-heavy, terabyte-class storage) |
| Identity (key insight) | **`did:plc`** = self-authenticating DID with **1-5 rotation keys** (control identity reconfiguration) separate from a single **`#atproto` signing key** (controls repo signing) — the load-bearing prior art for Myrhiza |
| DID methods | `did:plc` (Bluesky-operated registry, ~12M+ DIDs as of October 2024) and `did:web` (self-hosted via HTTPS/DNS); `did:plc` is the default for new Bluesky accounts |
| Crypto | **secp256k1** and **NIST P-256** for rotation keys; ECDSA-SHA256; signing-key supports any `did:key` curve |
| Schema system | **Lexicon** — JSON-Schema-like with NSID (reverse-DNS) namespacing; `lexicon: 1` version constant; strict additive evolution rules |
| Repository format | **Merkle Search Tree** keyed by SHA-256-derived depth + signed commits + **CAR v1** export; PDS is authoritative writer |
| E2E messaging | **Bluesky has no native E2E DMs.** As of Feb 2026, **Germ DM** is a third-party app that overlays MLS-based encryption on atproto identities — first private messenger to launch natively in Bluesky app (Feb 2026). MLS adoption did **not** ship in atproto itself in 2024-2025. |

## Contents

Each file is independent and skimmable.

**Architecture & primitives**
- [**architecture.md**](architecture.md) — PDS / Relay / AppView trio, firehose, repos, MST, CAR; the federation topology and how data flows through it.
- [**identity.md**](identity.md) — `did:plc` rotation keys vs signing keys, 72-hour recovery window, plc.directory, did:web fallback, the load-bearing file for Myrhiza.
- [**lexicon.md**](lexicon.md) — schema system, NSID namespacing, strict additive evolution, the `lexicon: 1` version constant, and what Myrhiza can borrow for snapshot portability.
- [**networking.md**](networking.md) — Subscribe Repos firehose, repo sync (sync v1.1), XRPC, hosting requirements for self-operators.
- [**crypto.md**](crypto.md) — curves, signature scheme, commit-signing, why `secp256k1` + `p256` and not Ed25519, key encoding format (`did:key`).

**Ecosystem & history**
- [**governance.md**](governance.md) — Bluesky PBC structure, board, funding history (Twitter seed → 2024 $15M Series A → 2026 $100M Series B), IETF ATP Working Group.
- [**history.md**](history.md) — chronological timeline: ADX → atproto → BGS-renamed-to-Relay → Series B → leadership transition → IETF.
- [**federation.md**](federation.md) — the honest assessment: who else runs PDSes, who runs Relays, the ~99% concentration figure, hardware barriers, "credible exit" rhetoric vs reality.
- [**apps.md**](apps.md) — bsky.app, Statusphere, Whitewind, Smoke Signal, Germ DM, and the "Atmosphere" framing.
- [**comparisons.md**](comparisons.md) — atproto vs ActivityPub, vs Nostr, vs Matrix, vs Holochain, vs Willow.

**Synthesis**
- [**open-problems.md**](open-problems.md) — what atproto structurally doesn't solve (E2E messaging, true federation, sync at scale, key-loss recovery without `plc.directory`).
- [**critiques.md**](critiques.md) — third-party critiques: "federation-shaped, not federated," "plc.directory is a single point of trust," IETF standardization tensions.
- [**lessons.md**](lessons.md) — **the consult-this-when-designing decision file.** validates / avoid / borrow.
- [**glossary.md**](glossary.md) — system-specific terms.

## How to use

If you're designing Plan B-2 (persistent identity) or any future revision of Myrhiza's identity model, read in this order: [lessons.md](lessons.md) → [identity.md](identity.md) → [open-problems.md](open-problems.md) → [federation.md](federation.md). The first three give you the prior art; the fourth keeps you honest about what atproto delivers vs claims.

If you're sketching a Lexicon-equivalent schema system for Myrhiza snapshot portability: [lexicon.md](lexicon.md) → [open-problems.md](open-problems.md) §"Lexicon evolution" → [comparisons.md](comparisons.md).

If you're evaluating whether to lean on atproto as a transport substrate: don't, but [networking.md](networking.md) + [federation.md](federation.md) + [critiques.md](critiques.md) explain why honestly.

**Framing disclosure.** These docs are written from a P2P-with-deterministic-state-apply stance — most "Implications for Myrhiza" sub-sections frame atproto's choices through the lens of "what does a peer-symmetric WASM runtime want from this?" Future readers auditing whether the P2P stance itself is right should weigh the corpus accordingly: it's a learn-from-atproto-into-Myrhiza artifact, not a neutral catalog of federated protocols. In particular, atproto's federation story is summarized harshly because Myrhiza's design bet *depends* on not making the same trade-off; if you're evaluating that bet, read [critiques.md](critiques.md) and [federation.md](federation.md) side by side and form your own view.

## Sources

- AT Protocol specs: <https://atproto.com/specs>
- AT Protocol guides: <https://atproto.com/guides/overview>
- Bluesky FAQ: <https://bsky.social/about/faq>
- Bluesky Series B announcement (2026-03-19): <https://bsky.social/about/blog/03-19-2026-series-b>
- did:plc spec: <https://github.com/did-method-plc/did-method-plc>
- Bluesky PDS self-host repo: <https://github.com/bluesky-social/pds>
- Atproto reference repo: <https://github.com/bluesky-social/atproto>
- Indigo (Go relay/PDS): <https://github.com/bluesky-social/indigo>
- BGS-to-Relay rename discussion: <https://github.com/bluesky-social/atproto/discussions/1847>
- IETF ATP WG kickoff: <https://atproto.com/blog/kicking-off-the-atp-working-group>
- Backlinko Bluesky statistics: <https://backlinko.com/bluesky-statistics>
- "Is Bluesky Billionaire-Proof?" (Intercept, 2023): <https://theintercept.com/2023/06/01/bluesky-owner-twitter-elon-musk/>
- AT Protocol Wikipedia: <https://en.wikipedia.org/wiki/AT_Protocol>
