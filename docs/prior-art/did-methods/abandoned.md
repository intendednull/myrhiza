**Date:** 2026-05-22
**Status:** active
**Subject:** Abandoned and stagnant DID infrastructure — `did:ion`'s collapse, `didkit`'s archival, `did:tdw`'s rename, dead methods in the registry. Where the ecosystem has retreated.

# Abandoned DID infrastructure — the retreat audit

DID-Land has had several high-profile retreats. Documenting them keeps Myrhiza's adoption-decision honest.

## `did:ion` — architectural elegance, operational collapse

ION was *the* DIF flagship project from 2018–2022: Microsoft-led, Bitcoin-anchored via Sidetree, IPFS-hosted, with a multi-key recovery model nicer than any other method. The DIF's [identity.foundation/ion/](https://identity.foundation/ion/) page still describes it in the present tense as "an open, public, permissionless decentralized identifier network."

**What happened:**

- **2020-03:** ION network goes live on Bitcoin mainnet ([Microsoft Community Hub announcement](https://techcommunity.microsoft.com/blog/microsoft-security-blog/ion-%E2%80%93-we-have-liftoff/1441555)).
- **2021-03:** Sidetree v1.0.0 spec finalized.
- **2022-06:** `decentralized-identity/ion` repository tags release v1.0.4 — **its last release as of 2026-05**.
- **2022:** ION transitioned from "Microsoft Verified ID preview" to general availability with `did:ion` as the default trust system.
- **2023-12:** **Microsoft Entra Verified ID dropped `did:ion` as a trust system option, replacing it with `did:web`.** This is documented in [Microsoft's "what's new" feed](https://learn.microsoft.com/en-us/entra/verified-id/whats-new): "Microsoft Verified ID supported the DID:ION method in preview until December 2023. The option of selecting did:ion as a trust system is removed, and the only trust system available is did:web."
- **2024 onward:** No public deployment activity at scale. The Sidetree spec hasn't shipped a new version. The ION repo accumulates issues but ships no releases.

**Why did Microsoft retreat?** No official statement explains the decision. Plausible factors:

1. **Bitcoin transaction cost volatility.** Each ION operation batches into a Bitcoin transaction, costing ~$5–50 depending on fee market. For an enterprise issuing thousands of credentials, that's real money for a feature most users didn't notice.
2. **IPFS dependency.** ION nodes need IPFS to retrieve operation payloads (only the commit hash is on Bitcoin). IPFS reliability has been chronically weak. A resolver that fails because an IPFS pin expired is a bad user experience.
3. **Operational complexity.** Running an ION node requires a Bitcoin full node + IPFS + the Sidetree batching service. Microsoft retreated to `did:web`, which is "host a JSON file on HTTPS."
4. **Internal politics.** Microsoft's identity team (Entra) consolidated around `did:web` after the decentralized-identity team's organizational position weakened. Speculative but consistent with the timing.

**The lesson is harsh:** the *most architecturally sophisticated* DID method couldn't sustain a single major operator. The recovery-key-vs-update-key model is right ([`rotation.md`](rotation.md) borrows from it), but the implementation context (Bitcoin + IPFS + Sidetree) was wrong.

**Status today (2026-05):** The public ION network nominally runs — anyone with the operational chops can start a Sidetree node — but the dominant operator left, no replacement emerged, and identifier-creation has slowed to a trickle. ION should be classified as **architecturally instructive but operationally moribund**.

## Spruce `didkit` — archived 2025-07-10

DIDKit was Spruce's flagship product 2020–2023: a CLI + JS bindings + WASM build + Python/Ruby/Java bindings wrapping the `ssi` Rust crate. It was *the* "any DID method, any language" tool.

**What happened:**

- **2020-10:** First `didkit` release on crates.io.
- **2023-06:** Last `didkit` crate version v0.6.0 published.
- **2024:** Reduced activity; Spruce focused on `ssi` directly + mobile.
- **2025-07-10:** **`spruceid/didkit` repository archived by Spruce.** Quote from the archive notice: "As we do not use the DIDKit bindings internally anymore, we have decided to archive their respective repositories."

Spruce now directs users to:

1. The `ssi` crate directly (Rust users).
2. `sprucekit-mobile` (iOS/Android — Spruce's mobile-focused commercial product).

**Implications:**

- Anything citing `didkit` as the recommended Spruce entry point is **out of date**.
- The underlying `ssi` crate is healthy (v0.16.0, 2026-04-16) — `didkit` was a packaging layer that Spruce no longer needed because they moved to direct `ssi` use + mobile native libraries.
- For Myrhiza, this is "depend on `ssi`, ignore `didkit`."

## `did:tdw` → `did:webvh` rename (mid-2025)

The "Trust DID Web" method was originally `did:tdw`. During the v0.4 → v0.5 transition in mid-2025, it was renamed to `did:webvh` ("web + verifiable history"). The rename was driven by:

- Spec branding clarity: "Trust DID Web" was ambiguous (trust *of* DID Web? trust *via* DID Web? Trust DID *Web* the method?).
- Alignment with what the method does: append "vh" (verifiable history) to "web."
- Avoidance of confusion with ToIP's "Trust Registry" work.

The rename has caused chronic naming inconsistency: older docs cite `did:tdw`, newer ones `did:webvh`, and the repo and spec URL are both `didwebvh` but the legacy `trustdidweb` URL still redirects.

Implication for Myrhiza: when referencing this method, use `did:webvh`. The `did:tdw` name is historical.

## Dead and stagnant methods in the registry

The W3C DID Extensions — Methods registry contains hundreds of entries. A sampling of methods that registered but never reached production:

- **`did:btcr`** — Bitcoin reference. Registered 2018; minimal commercial use.
- **`did:v1`** — "Veres One" ledger from Digital Bazaar. Registered early; small ecosystem.
- **`did:sov`** — Sovrin's flagship method on Indy. Sovrin Foundation has shrunk; activity declining since 2022.
- **`did:nuts`** — Dutch healthcare. Active in NL but narrow.
- **`did:elem`** — Element DID, Sidetree-based on Ethereum. Forerunner to ION; mostly historical.
- **`did:com`** — Commercio Network. Italian SSI commercial vendor; modest scale.
- **`did:moncon`**, **`did:erc725`**, **`did:abt`**, **`did:bba`**, **`did:meta`**, **`did:io`**, **`did:work`**, **`did:lit`**, **`did:vivid`**, **`did:morpheus`**, etc. — long tail. Most have <100 identifiers in production, many have zero recent activity.

The pattern: a blockchain or identity startup wanted a DID method as a marketing badge, registered one, and either pivoted or shut down without retracting the registration. The W3C registry is **not curated for liveness** — registered methods stay listed regardless of operational status.

**Implication for Myrhiza:** when reading "DID Method X exists," check the actual project status (recent commits, recent deployments, operator status) before treating it as a candidate. Most aren't.

## Indy / Hyperledger trajectory

Hyperledger Indy was the dominant SSI stack from ~2017–2022, supporting `did:sov` and a Hyperledger-flavored DID method ecosystem. Indy adoption has shrunk since 2022:

- **Sovrin Foundation downsizing** — staffing reductions, smaller validator set.
- **Migration pressure** — Indy users moving to `did:webvh` (BC Government's path) or non-Indy stacks.
- **AnonCreds (Indy's credential format)** is being repackaged for non-Indy substrates, but the underlying ledger is in decline.

This isn't "abandoned" exactly — Indy still runs — but the trajectory is downward. `did:webvh` benefits from being the migration target.

## Implications for Myrhiza

Three lessons:

1. **DID method longevity is unpredictable.** ION had Microsoft's full backing and was state-of-the-art; Microsoft left in 18 months of GA. The "stable, well-funded" DID method category has fewer entries than the marketing implies.
2. **The retreats often happen quietly.** Microsoft's `did:ion` retreat is documented only in a "what's new" feed, not a press release. Spruce's `didkit` archival is one line in a GitHub repo. **Anyone evaluating DIDs should check operator status, not project marketing.**
3. **`did:webvh` is the rising method.** As Indy declines, BC Government's pilot success, and Trust over IP backing, `did:webvh` is becoming the "if you must adopt a DID method, pick this one" answer. Whether it sustains beyond pilots is the next question.

## Sources

- ION repository (last release v1.0.4, 2022-06-09) — <https://github.com/decentralized-identity/ion>.
- Microsoft Entra "what's new" — `did:ion` removal — <https://learn.microsoft.com/en-us/entra/verified-id/whats-new>.
- Microsoft Community Hub — "ION – We Have Liftoff!" (2020-03) — <https://techcommunity.microsoft.com/blog/microsoft-security-blog/ion-%E2%80%93-we-have-liftoff/1441555>.
- Spruce `didkit` archival (2025-07-10) — <https://github.com/spruceid/didkit>.
- `did:webvh` rename — <https://identity.foundation/didwebvh/>.
- W3C DID Extensions — Methods (long tail) — <https://www.w3.org/TR/did-extensions-methods/>.
- Sovrin Foundation governance — <https://sovrin.org/>.
