**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/Endo/SwingSet — comparisons to adjacent and competing systems

# Comparisons

This file places Agoric/Endo/SwingSet against six neighboring systems. The goal is to make Agoric's tradeoffs legible by contrast, so Myrhiza spec authors can see which axes have been explored and what was learned.

Cross-references: `../spritely-ocapn/comparisons.md` (the research-grade ocap sibling), `../iroh/comparisons.md` (the transport-substrate dependency).

## vs. Spritely Goblins / OCapN

Both Agoric and Spritely descend from Mark Miller's ocap research and the E language. Both implement variants of CapTP (the Capability Transport Protocol). They are genuine siblings, not competitors — and they actively cooperate on the OCapN standardization effort.

| Axis | Agoric | Spritely Goblins |
|---|---|---|
| Implementation language | JavaScript (XS engine on chain, Node.js off chain) | Racket (primary), Guile, separate Hoot WASM target |
| Runtime model | **Vat replay**: per-vat transcript + heap snapshot, deterministic replay | **Object-graph actors** with transactional update; persistent actor identity |
| Deployment context | Cosmos SDK chain (BFT consensus) + off-chain solo | General-purpose distributed app; no chain assumed |
| Concurrency primitive | Eventual send (`E()`/`HandledPromise`) within and across vats | Eventual send (`<-`); turns within a vat are transactional |
| Network protocol | CapTP-over-IBC (chain) and CapTP-over-WebSocket (Endo) | Goblins' netlayers — Tor onion, TCP, etc. — under the OCapN umbrella |
| Persistence | Vat heap snapshot + transcript replay; vat upgrade through new code bundle | Persistent actor sigils via Goblins persistence layer |
| Governance | Cosmos on-chain governance + Agoric Operating Co. | NLnet-grant-funded research-grade |
| Posture | Production-hardened ocap on chain | Reference / research; correctness-first |

The two implementations have known divergences (per [agoric-sdk #1827](https://github.com/Agoric/agoric-sdk/issues/1827)): Goblins models everything through lambda/procedure-style method handlers; Agoric uses multiple message kinds (call vs property-get vs procedure-call). Goblins assumes flaky low-latency connections (e.g., Tor) and surfaces disconnects; Agoric assumes store-and-forward IBC reliability and treats disconnects as exceptional. Serialization is similar but not bit-compatible.

OCapN is the cross-implementation interop standard ([ocapn.org](https://ocapn.org/), [github.com/ocapn/ocapn](https://github.com/ocapn/ocapn)). It is split into CapTP (semantics) and netlayers (transports). Both Agoric and Spritely participate. As of May 2026 the spec is draft; cross-implementation interop demos exist; production interop does not.

**For Myrhiza:** Goblins is the closer model for our P2P-first orientation. Agoric is the closer model for determinism and on-the-wire wire format engineering. Take ideas from both; bind to OCapN as the wire format if/when it stabilizes.

## vs. Cap'n Proto

Cap'n Proto is Kenton Varda's RPC framework. Varda explicitly modeled the RPC layer on **CapTP** ([Cap'n Proto on the friam mailing list](https://groups.google.com/g/friam/c/0HFPWRMPkkY)) — i.e., on the same E-language ancestor that Agoric inherits. Promise pipelining in both systems traces to Miller's dissertation.

| Axis | Agoric / Endo CapTP | Cap'n Proto |
|---|---|---|
| Wire format | Capdata (JSON-derivative with smallcaps for object refs) | Cap'n Proto schema (zero-copy, schema-defined) |
| Promise pipelining | Yes | Yes |
| Three-vat introductions | Yes (CapTP) | Yes (3-party handoff) |
| Implementation languages | JS first; Goblins (Racket), spritely-rs (Rust) emerging | C++ first; Go, Rust, Python, others |
| Primary use | Smart contracts (Agoric), wallet plugins (MetaMask Snaps) | RPC for Sandstorm, Cloudflare Workers, general systems |
| ocap purity | Strict in Agoric; Cap'n Proto extends but has system-call holes for performance | Strict on the wire; calling code can leak |

Cloudflare's **Cap'n Web** ([Cloudflare blog](https://blog.cloudflare.com/capnweb-javascript-rpc-library/)) is a 2024–2025 JavaScript RPC library that inherits Cap'n Proto's RPC semantics for the browser/edge. It is *not* Endo's CapTP, but it shares ancestry. This is the third active implementation lineage (alongside Endo CapTP and Goblins) of Miller's protocol idea.

**For Myrhiza:** Cap'n Proto's wire format is more mature and language-agnostic; CapTP's wire format is JS-shaped. If Myrhiza needs a polyglot ocap wire protocol (likely, given WASM Component Model implies multi-language guests), the OCapN spec — explicitly cross-implementation — is the right target. Cap'n Proto's choice of a schema-defined zero-copy format is worth borrowing if we want apps from different languages to share data without re-marshaling.

## vs. Solidity / EVM smart contracts

This is the explicit thesis of Agoric: **ocap contracts are safer than shared-memory contracts.** The Agoric pitch contrasts "object capabilities = explicit grants" with the EVM pattern of "everyone reads/writes a global storage trie, and authority is derived from msg.sender + custom check code."

Concrete differences:

- **Reentrancy** — In Solidity, reentrancy is a class of exploits (the DAO, many subsequent ones); fixes are conventions (checks-effects-interactions, ReentrancyGuard). In Agoric, vats are single-threaded with synchronous-within / async-across boundaries; reentrancy in the EVM sense is structurally impossible because cross-vat calls are messages.
- **Authorization** — Solidity uses `msg.sender` and access-control libraries. Agoric uses object references — to call `vault.close()` you need a reference to `vault`, and you got that reference from someone with the authority to give it to you. The audit cost is following the reference graph.
- **Composition** — Solidity contracts are reasoned about as state machines that interleave on a global ledger. Zoe (Agoric) defines "offer safety": users sign a quid-pro-quo offer (`{ give: X, want: Y }`) and the contract escrows. The contract cannot run away with the user's assets even if it is buggy, because Zoe holds the escrow.

Honest counterweights:

- The EVM ecosystem has 5+ years of battle-tested contracts and audited libraries (OpenZeppelin); Agoric's Zoe-based contracts are far fewer and less audited.
- Agoric vats are JS — and JS has its own footguns (prototype pollution, equality, the override mistake) which Hardened JS papers over but does not eliminate.
- The "offer safety" property is a runtime guarantee about Zoe specifically, not a property of all Agoric contracts; a custom contract that bypasses Zoe doesn't get it.

**For Myrhiza:** the ocap-vs-shared-memory thesis is the right one. We adopt it. We should also adopt offer-safety-style escrow primitives — if Myrhiza has financial-flavored apps, the kernel should provide an escrow capability rather than trusting the app.

## vs. CosmWasm

Both run as smart contract platforms on Cosmos SDK chains. Both support cross-chain via IBC. The technology choices diverge sharply.

| Axis | Agoric SwingSet | CosmWasm |
|---|---|---|
| Contract language | JavaScript (Hardened JS subset) | Rust → WASM (also AssemblyScript, Go via TinyGo) |
| Engine | XS (Moddable, no-JIT, deterministic-by-discipline) | wasmer / wasmvm (gas-metered WASM) |
| Memory model | Object graph in JS heap; vat snapshots | WASM linear memory + key-value store |
| ocap discipline | Strict; references are caps | Optional; contracts use addresses + access lists |
| Determinism story | XS-engine subsetting + SES + careful API exclusion | WASM is structurally deterministic; gas metering bounds runtime |
| Onboarding language | "Most JS devs can read it" | Rust learning curve is real |
| State migration | Vat upgrade with kernel-mediated identity preservation | Contract migration via `migrate` entrypoint, contract-author-defined |

CosmWasm is more popular by deployment count across Cosmos chains (Juno, Osmosis, Neutron, etc.). Agoric's SwingSet is a bigger conceptual leap (vats, eventual send, Zoe) but much narrower deployment (essentially the Agoric chain itself).

**For Myrhiza:** we are closer to CosmWasm in *engine choice* (WASM) and closer to Agoric in *security model* (ocap, deterministic state-apply). We get to combine the strengths if we are careful: WASM gives us cross-language guests and structural determinism; ocap discipline gives us composable authority. The cost: we have to do the SES-equivalent work of forbidding non-deterministic WASM imports and making capability-passing first-class — Component Model helps here, but it isn't free.

## vs. Iroh

Different layer entirely. **Agoric is a runtime + protocol; Iroh is a transport substrate.** They solve overlapping but distinct problems.

| Axis | Agoric | Iroh |
|---|---|---|
| Layer | Runtime / object protocol (CapTP) | Transport (QUIC + relay + discovery) |
| Identity | Vat names + capability references | NodeIDs (Ed25519 pubkey) |
| Networking | CapTP over IBC (chain) or WebSocket (off-chain) | QUIC with hole-punching, relays for fallback |
| State | Vat heap snapshots + transcripts | Ephemeral connection state (plus blobs/docs in higher Iroh layers) |

The interesting comparison: **Agoric's networking story is "CapTP-over-IBC" inside the chain context, and "CapTP-over-WebSocket" for off-chain Endo apps.** That's a per-deployment choice. OCapN proposes to make this pluggable via netlayers. Iroh is a candidate netlayer for any OCapN-style stack — and a strong candidate for Myrhiza, where we want P2P transport without a chain underlay.

**For Myrhiza:** if we adopt Iroh as the transport (`../iroh/`) and CapTP-over-OCapN as the wire-protocol family, we get a roughly orthogonal stack: Iroh handles "how do peers find each other and stream bytes," OCapN/CapTP handles "what those bytes mean as an ocap reference graph," and our state-apply components handle "what the resulting events do to local state." Each layer has a clear contract.

## vs. Hardened JS in browsers (Realms / iframe / Web Workers)

SES grew out of the **Realms** TC39 proposal, which would let JS create a fresh global environment within the same agent. Realms was the early-stage proposal; it has migrated into Compartments + ModuleSource. Browsers also offer iframe sandboxing (origin-isolated DOM context) and Web Workers (separate-thread JS) as ad-hoc isolation primitives.

| Axis | SES (Endo) | iframe sandbox | Web Worker | Realms (proposed) |
|---|---|---|---|---|
| Same agent? | Yes (same JS heap, same V8 isolate) | No (separate origin) | No (separate thread + heap) | Yes |
| Isolation primitive | Frozen primordials + Compartment | Same-origin policy + sandbox attribute | postMessage IPC | Realm-fresh globals |
| Cap-passing | Direct object refs (after lockdown) | postMessage of structured-cloneable | postMessage of structured-cloneable | Direct object refs |
| Performance | Lockdown is ~100ms one-time; runtime overhead small | Heavy (cross-origin) | Heavy (cross-thread) | Light (intended) |
| Production users | MetaMask Snaps, Agoric, LavaMoat | Universal | Universal | None — proposal stalled |

The honest picture: **SES is the only working same-agent confinement for JS in production.** iframes and workers are heavy. Realms has not landed at TC39 (it is now Compartments + ModuleSource, both Stage 1). MetaMask Snaps demonstrate that you can run untrusted plugin code in the same Node-or-Electron process as the wallet without giving it the wallet's authority — and they do that because Endo's SES makes it possible.

**For Myrhiza:** WASM Component Model gives us strong isolation by construction (linear memory, no shared globals). The SES design tension — confining a language that was never built for confinement — is one we don't inherit, because WASM was built for confinement from day one. We do still need to think about deterministic primitives (clock, randomness, file I/O) the way SES does, but the heap-isolation problem is solved for us at the engine level.

## Implications for Myrhiza

Across these comparisons:

- **CapTP / OCapN is the wire protocol family to track.** Both Agoric and Spritely target it; Cap'n Proto is adjacent. Bind Myrhiza's inter-peer messaging to OCapN if/when it stabilizes; don't invent our own.
- **The ocap-vs-shared-memory thesis is right.** Apply it from day one in our state-apply contracts; capabilities (object references in the runtime, exported world types in WASM) are the only authority surface.
- **WASM gets us isolation cheaply; we still owe the deterministic-imports work.** SES's hard part is freezing a language that wasn't designed for it. Our hard part is curating which WASI / kernel imports are deterministic. That work is real and should be specced before we ship anything load-bearing.
- **Single-engine determinism is a tax we should not pay if we don't have to.** Agoric had to choose XS to get determinism. WASM execution is deterministic by spec; we get this for free at the engine level and pay for it at the imports level.
- **Wire format matters more than runtime model for cross-language ecosystems.** The lesson from Cap'n Proto: pick a schema-defined cross-language wire format and stick with it. Component Model + a CapTP-style ocap binding could be that wire format for us.
- **Don't bake in a flagship application.** EVM / Solidity bake DeFi-shaped assumptions into the contract-author API. Agoric baked Zoe / IST into the chain. CosmWasm stays application-neutral. Myrhiza should stay closer to CosmWasm here: the runtime is application-neutral; specific patterns (escrow, voting, presence) are app-level.

## Sources

- https://github.com/ocapn/ocapn — OCapN spec repo
- https://ocapn.org/ — OCapN pre-standardization group
- https://github.com/Agoric/agoric-sdk/issues/1827 — Spritely / Agoric CapTP interop discussion
- https://capnproto.org/ — Cap'n Proto homepage
- https://capnproto.org/news/2013-12-13-promise-pipelining-capnproto-vs-ice.html — Cap'n Proto promise pipelining
- https://blog.cloudflare.com/capnweb-javascript-rpc-library/ — Cap'n Web (Cloudflare)
- https://groups.google.com/g/friam/c/0HFPWRMPkkY — Cap'n Proto on the FRIAM (ocap) list
- https://en.wikipedia.org/wiki/Cap%27n_Proto
- https://github.com/CosmWasm/cosmwasm
- https://cosmwasm.com/
- https://github.com/tc39/proposal-compartments
- https://github.com/tc39/proposal-ses
- https://hardenedjs.org/
- https://www.moddable.com/hardening-xs
- https://docs.agoric.com/guides/orchestration/ — Agoric Orchestration overview
- ../spritely-ocapn/ — sibling research-grade ocap project
- ../iroh/ — transport-substrate dependency
