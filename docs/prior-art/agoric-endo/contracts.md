**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric — Zoe, ERTP, and the ocap smart-contract thesis

# Smart contracts on Agoric

Where Ethereum's contract model is "an account on a chain with code at it that other accounts call," Agoric's model is "a JavaScript object inside a vat that holds capabilities to other JavaScript objects." This is a much smaller departure from ordinary programming than Solidity is — and that's the whole pitch.

This file covers what a contract author actually writes. See [chain.md](./chain.md) for the chain layer it runs on, and [apps.md](./apps.md) for what's been built with it.

## The three layers

| Layer | What it is | Lives in |
|---|---|---|
| **ERTP** | A library — primitives for digital assets (issuer / brand / mint / purse / payment) | The contract's vat (it's just an npm package) |
| **Zoe** | A service vat — escrows assets, enforces offer safety, hosts contract instances | A dedicated vat, run by the chain |
| **Contract** | User-supplied JS code that implements business logic | A new vat per contract instance, child of Zoe |

You can use ERTP without Zoe (you'd just be issuing tokens with no exchange protocol on top). You cannot really use Zoe without ERTP — Zoe's escrow assumes ERTP-shaped assets. ([Zoe overview](https://docs.agoric.com/guides/zoe/), [ERTP overview](https://docs.agoric.com/guides/ertp/))

## ERTP — the asset model

ERTP ("Electronic Rights Transfer Protocol", `@agoric/ertp` — latest **0.17.0**, April 2026) is the asset primitive. It treats a digital asset not as a balance in a global ledger but as **a JavaScript object you hold a reference to**. Lose the reference, lose the asset. This is ocap discipline applied to value.

The five concepts:

- **Brand** — the *type* of an asset. "USDC" is a brand. Comparing brands is identity comparison on JS objects, not string comparison.
- **Issuer** — the authority that validates assets of a brand. The only object that can verify a payment is genuine. Holding the issuer is the ability to read and partition assets, but **not** to create them.
- **Mint** — the authority that *creates* assets of a brand. Strictly more powerful than the issuer; usually held by exactly one party (the contract that issued the token).
- **Purse** — a long-lived container. Lives in your wallet vat. You deposit payments into a purse, withdraw payments out.
- **Payment** — a transferable, single-use unit. Created by withdrawing from a purse, consumed by depositing into another purse. **Never split or merged in-place** — you call `issuer.combine([p1, p2])` and get back a new payment, leaving the originals invalid.

The use-once nature of payments is the key invariant. There is no double-spend at the language level: passing the *same payment* to two recipients fails for the second one because the issuer has already burned it. This is a much stronger guarantee than "the EVM checks balances," because it doesn't depend on the contract being bug-free.

`AmountMath` is the helper module for arithmetic on `{ brand, value }` pairs. Values can be `nat` (fungible, BigInt), `set` (NFT, array of identifiers), or `copyBag` (semi-fungible). Brand mismatches are runtime errors.

## Zoe — the contract host

Zoe (`@agoric/zoe` — latest **0.27.0**, April 2026) is a single, well-known vat that hosts every contract instance on Agoric. When you "deploy a contract," you are asking Zoe to spin up a new child vat from a code bundle.

Zoe enforces two invariants, *no matter what the contract code does*:

### Offer safety

A user submits an offer expressing what they're putting in (`give`) and what they want out (`want`). Zoe **escrows the `give`** before the contract sees anything. The contract returns an "allocation" — its proposal for who gets what. Zoe then checks: does each user either get their declared `want`, *or* a full refund of their `give`? If neither, the allocation is rejected and the user gets their refund.

> "When you make an offer, you get either what you said you wanted or a full refund of the assets you put in, even if the contract is buggy or malicious." ([Zoe docs](https://docs.agoric.com/guides/zoe/))

This is the headline property. A buggy contract cannot run away with your funds. The worst it can do is refund you (i.e., fail to trade). It can do other bad things — DoS, spam, leak information — but it cannot *steal*.

### Payout liveness

Less famous but equally important: Zoe guarantees that a user can always exit an offer and receive their payout, even if the contract has stalled. The exit policy is part of the offer (`onDemand`, `afterDeadline`, or `waived`). Zoe holds the escrowed assets and is itself a well-known service vat run by the system, so contract liveness bugs cannot trap funds indefinitely.

Practical caveat: "payout liveness" guarantees you get *your* assets back; it does not guarantee that the contract's *intended trade* completes. A hung exchange contract can be exited, but the trade you wanted is just gone.

## Hardened JS — what a contract author writes

A contract is one JavaScript file (a "bundle" — `@endo/bundle-source` glues the imports into one self-contained module). It exports a `start(zcf)` function, where `zcf` is the Zoe Contract Facet — the capability that lets the contract create invitations, reallocate escrowed assets, and surface a public API.

Skeleton:

```js
import { Far } from '@endo/far';
import { AmountMath } from '@agoric/ertp';

export const start = async (zcf) => {
  // ... define handlers, allocations, etc.
  const publicFacet = Far('PublicFacet', {
    makeSwapInvitation: () => zcf.makeInvitation(handler, 'swap'),
  });
  return harden({ publicFacet });
};
```

Things to notice:

- **No `class`, no `this`-binding hazards.** `Far(label, methods)` makes a remotable object whose methods are passed as bound functions.
- **`harden(...)` everywhere.** Frozen-deep. Every returned object is hardened before crossing a vat boundary. This is provided by SES.
- **No mutable globals.** SES `lockdown()` runs at vat startup; primordials (Object, Array, Promise) are frozen. You cannot monkey-patch the runtime.
- **No filesystem, no network, no crypto-by-default.** A contract gets *only* the capabilities Zoe (or a deployment script) hands it. If you don't pass it a `chainStorage` reference, it cannot publish anything. If you don't pass it a `timer`, it cannot read the clock.

This is the "ocap-style smart contract" thesis in concrete form: **the contract's authority is exactly what got passed in via `start`'s arguments. Nothing else exists for it.**

### Lifecycle and upgrade

Contracts are upgradable under Zoe. The mechanism (since approximately 2023, per the agoric-sdk changelog) is "kindful" durable objects: state lives in `zcf.makeDurableZone()` storage, and a contract upgrade replaces the *code* while reattaching the same state. This is one of the harder problems in long-running JS systems and is the reason `@endo/exo` (the durable-object framework, latest **1.7.0**) is its own package.

For contracts that don't need upgradability, the simpler API is the older "non-durable" facet. Contracts written before the durable-object work was done are stuck with their original code unless redeployed.

## The ocap thesis vs Solidity

The Agoric / Endo crowd makes a specific empirical claim: **most published smart-contract vulnerabilities are class-of-bug that ocap discipline rules out at the language level.** The argument has been made publicly by Mark S. Miller (Agoric's chief scientist, designer of E and SES) in the Epicenter interview ([Agoric blog](https://agoric.com/blog/technology/medium-epicenter-interview)) and in Agoric's papers list ([papers.agoric.com](https://papers.agoric.com/papers/)).

The specific failure modes ocap addresses:

| Solidity hazard | Why it doesn't exist in Hardened JS |
|---|---|
| Reentrancy | A contract calls another contract via eventual-send (`E(other).method(...)`); the response is a promise. There is no synchronous re-entry into your own state. |
| `tx.origin` confusion | There is no `msg.sender` / `tx.origin`. You only get capabilities you were handed. The "caller" concept is replaced by "whoever holds the capability you exposed." |
| Integer overflow | BigInt by default; `AmountMath` is purely additive on natural-number values. |
| Delegatecall / proxy hijack | No `delegatecall` primitive. Code substitution requires going through Zoe's controlled upgrade path. |
| Front-running on chain | Not solved by ocap *per se* — Agoric still has mempool ordering — but offer safety means a front-runner cannot steal your `give`, only refuse the trade. |

What ocap *doesn't* solve: economic exploits (oracle manipulation, MEV, governance attacks), business-logic bugs in `start()` itself, and any vulnerability that crosses the boundary into off-chain components. These are the residual class of attacks; they're real and they happen on Agoric the same as anywhere.

The honest version of the thesis: **ocap reduces the attack surface from "every line of contract code" to "the specific business-logic decisions about who gets what authority."** That's a real reduction but not a panacea, and Agoric's blog posts have been clear about that. ([Mark Miller, "Agoric and the Decades-Long Quest for Secure Smart Contracts"](https://medium.com/agoric/agoric-and-the-decades-long-quest-for-secure-smart-contracts-epicenter-interview-with-mark-s-76c9a0fab6e2))

For deeper reading: [`awesome-ocap`](https://github.com/dckc/awesome-ocap) curates the foundational papers (Hardy 1985 on KeyKOS, Miller 2006 thesis on robust composition).

## Contracts, Zoe, and ERTP — wiring

The relationship in one paragraph: **A contract is a child vat of Zoe. ERTP is a library it imports.** When the contract starts, Zoe passes it a `zcf` (Zoe Contract Facet) and the contract uses ERTP-shaped issuers / brands to describe what it can trade. Users send offers to Zoe (not directly to the contract); Zoe escrows the `give`, calls the contract's offer-handler with a stripped-down "ZcfSeat" representing the offer's allocation slot, and the contract decides how to reallocate. Zoe verifies the reallocation preserves offer safety, then commits and dispenses payouts.

ERTP is in `@agoric/ertp`. Zoe is in `@agoric/zoe` (it depends on ERTP). The contract imports both. There is no separate "ERTP service" or "ERTP vat" — every issuer is a JavaScript object created by some vat (often a contract vat) and held by capability.

## Implications for Myrhiza

1. **Offer safety is the load-bearing idea.** The pattern of "user submits intent, kernel escrows resources, app proposes allocation, kernel checks invariants, kernel commits or refunds" is portable to non-trade settings. For Myrhiza this is exactly the shape of `state-propose` → `state-apply` with the kernel as referee. The Zoe model gives us a precedent: the kernel doesn't need to understand what the app is trying to do; it just needs to enforce a small invariant ("you got what you asked for or you got refunded") that the app cannot break.
2. **ERTP-style asset objects are not the same as token balances.** "Holding a payment" is "holding a JS object reference"; this only survives across processes if the system has serialization that preserves capability semantics. That's exactly what `@endo/marshal` + CapTP solve. If we want anything like ERTP in Myrhiza, we need our own marshalling story — and our analog is closer to wasm-component resource handles than to JS objects.
3. **Hardened JS is unavailable to us.** WASM Components don't have JS primordials to freeze; the analog is the WASM Component Model's interface types, which are nominal and unforgeable by construction. The lesson is the *property* (no ambient authority, no global mutation) more than the implementation.
4. **Don't conflate the contract layer with the kernel layer.** Agoric makes a clean distinction: SwingSet is the kernel (delivery, persistence, metering); Zoe is a contract framework that runs in user-space *on top of* SwingSet. Zoe's offer-safety guarantees are not built into the kernel. We should preserve the same separation — Myrhiza kernel provides capabilities and determinism; "what kind of invariants apps want" is policy on top, not kernel.
5. **Upgrade is hard and they spent years on it.** `@endo/exo` and the durable-zone work in agoric-sdk represent significant engineering investment. We will face the analog problem (upgrading a long-running app while preserving its state). Read their solution before designing ours.

## Sources

- [Zoe overview docs](https://docs.agoric.com/guides/zoe/)
- [ERTP overview docs](https://docs.agoric.com/guides/ertp/)
- [Mark S. Miller, "Agoric and the Decades-Long Quest for Secure Smart Contracts"](https://medium.com/agoric/agoric-and-the-decades-long-quest-for-secure-smart-contracts-epicenter-interview-with-mark-s-76c9a0fab6e2)
- [Mark S. Miller — Foresight Institute biography](https://foresight.org/people/mark-s-miller/)
- [Agoric Open Systems Papers (curated)](https://papers.agoric.com/papers/)
- [`awesome-ocap` reading list (Dan Connolly)](https://github.com/dckc/awesome-ocap)
- [Core-eval / contract deployment docs](https://docs.agoric.com/guides/coreeval/)
- [`@agoric/zoe` on npm (v0.27.0, April 2026)](https://www.npmjs.com/package/@agoric/zoe)
- [`@agoric/ertp` on npm (v0.17.0, April 2026)](https://www.npmjs.com/package/@agoric/ertp)
- [`@endo/exo` on npm (v1.7.0, April 2026)](https://www.npmjs.com/package/@endo/exo)
