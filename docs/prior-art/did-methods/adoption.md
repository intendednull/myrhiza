**Date:** 2026-05-22
**Status:** active
**Subject:** Honest scale audit — DIDs are mostly research-grade. Bluesky is the only at-scale production. Everywhere else: enterprise SSI pilots and abandoned experiments.

# DID adoption — the honest scale assessment

DID-Core marketing material implies an ecosystem with broad real-world deployment. The reality is more limited. This file audits actual production scale to keep Myrhiza's "should we adopt DIDs" decision honest.

## At-scale (millions+ of users)

**`did:plc`** is the **only** DID method at the millions-of-users scale. As of October 2024, `plc.directory` reported **~12M+ DIDs**, all created by Bluesky's onboarding flow. By 2025–2026 this is presumably larger (Bluesky's growth was rapid through 2024–2025). The exact current count isn't public.

This is the only ecosystem-relevant data point at scale. **And it's centralized in operation** — Bluesky PBC runs the registry, so the "decentralized identifier" framing is aspirational. See [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md) §"Federation status" for the unfulfilled federation roadmap.

## Mid-scale (thousands to hundreds-of-thousands)

**`did:web`** has the broadest enterprise adoption. Hard to count because each user is self-hosted. Known production deployments:

- **Microsoft Entra Verified ID** uses `did:web` as the sole trust system (migrated from `did:ion` December 2023). Used by enterprises that issue/verify Microsoft Verified ID credentials.
- **EU eIDAS 2.0 EUDI Wallet pilots** — multiple member-state pilots use `did:web` for issuer identity.
- **Various supply-chain credential issuers** (GS1 Digital Link, etc.) — `did:web:gs1.org`-style trust roots.

Rough estimate: hundreds-of-thousands to single-digit-millions of `did:web` documents in existence, dominated by organizational (not individual) identifiers. Not user-facing scale.

**`did:ethr`** has tens-of-thousands to low-hundreds-of-thousands of identifiers on Ethereum + L2s combined. ENS-adjacent DID activity, SpruceID Sign-In With Ethereum users (where session-specific `did:ethr`s are minted). The ethr-did-resolver v13.0.0 (2026-05-18) is healthy; the actual production scale of `did:ethr`-as-identity (rather than just Ethereum addresses) is modest.

**`did:cheqd`** is in commercial-pilot stage. Validator set is small (Cosmos-style). Partnerships with multiple SSI vendors but no published user-count figures. Probably low-thousands of operational DIDs as of 2026.

## Pilot / research-grade (hundreds or below)

**`did:webvh`** — BC Government's Verifiable Credential pilots, Trust over IP demonstrators, a few Hyperledger Indy-to-`did:webvh` migrations. Single-digit-thousands of identifiers at most.

**`did:peer`** — DIDComm v2 / Aries deployments at companies like Indicio, Trinsic, Animo, Avast (formerly Evernym). Pairwise identifiers per relationship, so the count is "relationships" not "people" — modest commercial use but no clear scale figure.

**`did:ion`** — public ION network nominally operates (anyone can run a Sidetree node), but **Microsoft removed `did:ion` as a trust system from Entra Verified ID in December 2023**. No published successor operator. The Sidetree spec hasn't shipped a new version since 1.0.0 (2021-03-09). Active identifier count probably in the low-thousands, dominated by historic identifiers. See [`abandoned.md`](abandoned.md).

## The "hundreds of methods" reality check

The W3C DID Extensions — Methods registry (republished 2026-04-10) lists hundreds of methods. The overwhelming majority are:

- **Per-vendor methods.** Every blockchain identity team registered their own (`did:hedera`, `did:sol`, `did:near`, `did:iden3`, `did:dock`, `did:kilt`, `did:btcr`, etc.). Most have <10 production identifiers.
- **Provisional / abandoned.** Registered during ToIP / Hyperledger Indy era, never reached production. Indy's own `did:indy` is the largest of these, kept alive by Sovrin Foundation but with declining commercial activity.
- **National / government experiments.** EBSI (`did:ebsi`) is the EU's blockchain identity infrastructure — modest pilot scale.

A reasonable inference: of the registry's hundreds of methods, **~10 have production deployments, ~3 have meaningful scale, and ~1 has at-scale user adoption** (`did:plc`).

This is the gap between DID marketing and DID reality. Anyone advocating "adopt DIDs because of the ecosystem" should be asked which methods they mean and how many users actually use them.

## Why has DID adoption been slow?

Several structural factors:

1. **Method fragmentation.** Mozilla's formal objection (see [`history.md`](history.md)) was exactly this: DID-Core doesn't require any specific method, so each ecosystem invented its own, defeating interop.
2. **No browser-native support.** WebCrypto handles ECDSA + Ed25519 but nothing about DID resolution. There's no `navigator.credentials.resolveDid()`. Universal-resolver requires a server.
3. **Verifiable Credential ecosystem is also slow.** DIDs are a building block for VCs, but VC adoption (outside government pilots) is modest. Without VC demand, DID demand is modest.
4. **Centralized alternatives are mature.** Sign In With Google / Apple solve "portable identity" with browser-native UX. The DID value proposition (user-controlled keys, no central operator) competes against a UX-superior incumbent.
5. **Bluesky succeeded by skipping the ecosystem.** `did:plc` works at scale because Bluesky controls the directory, the protocol, and the user-facing app. The DID format is internal plumbing, not an interop surface.

The Bluesky lesson is the load-bearing one: **DID-format-as-internal-plumbing is the only at-scale success story.** Treating DIDs as a general-purpose interop layer hasn't worked yet and isn't visibly working.

## What this means for Myrhiza

Three actionable lessons:

1. **Don't adopt DIDs to "get on the ecosystem."** The ecosystem is small. Adopting DIDs imposes spec-conformance overhead with no real interop payoff outside specific niches (BC Government VC issuance, Microsoft Entra VID).
2. **Consider DIDs as an export format.** If Myrhiza ever wants to expose a peer-author key to an external verifier, `did:key:z6Mki...` is ~30 lines of multicodec encoding. That export is cheap and useful even if Myrhiza never adopts DIDs internally.
3. **Take the `did:plc` design pattern, not the format.** [`rotation.md`](rotation.md) details how the rotation-key-vs-signing-key split applies to Myrhiza's per-author DAG. The pattern is the prior art; the JSON-LD wire format is incidental.

## Sources

- `plc.directory` operator status — <https://web.plc.directory/> (12M+ DIDs as of October 2024).
- Bluesky `did:plc` operator scaling — <https://github.com/did-method-plc/did-method-plc>.
- Microsoft Entra `did:ion` → `did:web` migration — <https://learn.microsoft.com/en-us/entra/verified-id/whats-new>.
- EU EUDI Wallet — <https://digital-strategy.ec.europa.eu/en/policies/eudi-wallet>.
- BC Government `did:webvh` pilot — <https://identity.foundation/didwebvh/>.
- W3C DID Extensions — Methods registry — <https://www.w3.org/TR/did-extensions-methods/>.
- AT Protocol identity in-tree — [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md).
