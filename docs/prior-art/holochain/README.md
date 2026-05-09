**Date:** 2026-05-08
**Status:** active
**Subject:** Holochain — peer-symmetric Rust runtime hosting WASM apps with deterministic-validation DHT

# Holochain

Holochain is a Rust runtime for peer-to-peer applications where each participant runs the same WebAssembly application logic locally, signs their own actions onto a personal append-only "source chain," and gossips public data into a sharded validating DHT. It is the closest architectural analog to Myrhiza in the wild: agent-symmetric, capability-mediated, WASM-hosted, no global consensus. It has been under development since 2016 and has accumulated approximately a decade of design corrections — most directly relevant to anyone building a peer-symmetric WASM runtime today.

The marketing language ("post-blockchain," "infinitely scalable") oversells. The technical reality is more interesting: Holochain is a serious, slow, careful attempt to build a deterministic-validation DHT with per-app rules, and the parts that are real are very real. The parts that are still aspirational (sharding completion, free-rider resistance, light clients, mobile) have been aspirational for years.

## Key facts

| Fact | Value |
|---|---|
| Founded | December 2016, as a [MetaCurrency Project](https://medium.com/holochain/perspectives-on-blockchains-and-cryptocurrencies-7ef391605bd1) initiative by Arthur Brock and Eric Harris-Braun |
| Predecessor | Go prototype `holochain-proto` (2016-2018), then `holochain-rust` (RSM redesign, 2020) |
| Primary language | Rust (~99.3% of [`holochain/holochain`](https://github.com/holochain/holochain)); guest WASM is also Rust via HDK |
| Repository scale | `holochain/holochain` ~13,400 commits; ~262 repos under the `holochain` GitHub org |
| License | [CAL-1.0 (Cryptographic Autonomy License)](https://github.com/holochain/holochain) — explicit user-rights clause for keys + data |
| Governance | [Holochain Foundation](https://blog.holochain.org/the-holochain-foundation-is-coming-of-age/) holds IP; Holo Ltd. is a separate hosting company. Eric Harris-Braun took Executive Director role late 2024 |
| Funding | 2018 ICO via Holo Ltd. (HOT token); foundation grants; HoloFuel ([XHF, audited Q2 2024](https://www.buyholo.net/en/learn/news)) intended as long-term native settlement layer |
| Current version | 0.6.0 released [Nov 19, 2025](https://www.holochain.org/roadmap/); 0.5.x branch maintained; 0.4.x is the prior "stable reduced feature" line |
| Core contributors | ACE/MMR team — maackle, neonphog, jost-s, ThetaSinner, freesig, steveej, lucksus; Arthur Brock and Eric Harris-Braun architecturally |

## Contents

Each file is independent and can be skimmed standalone.

**Technical subsystems**
- [**Architecture**](architecture.md) — conductor, cells, DNAs, zomes, ribosome+wasmer, lair keystore, DHT op types, admin/app websocket APIs.
- [**Capabilities**](capabilities.md) — `ZomeCallCapGrant`, grant lifecycle, secret-exchange dance, `call_remote` wire-level, granularity ceiling.
- [**Determinism**](determinism.md) — HDI/HDK host fn lists, `must_get_*` family, validation receipts, countersigning protocol mechanics, genesis sequence.
- [**Networking**](networking.md) — Kitsune2 deep dive (round structure, sectors+rings, bootstrap, rate limits), sim1h → kitsune2 history.
- [**Identity**](identity.md) — keys, lair internals, DPKI seven-year saga, warrants, encrypted entries, MLS gap.
- [**Browser viability**](browser.md) — why there's no native browser conductor.
- [**Distribution & versioning**](distribution.md) — manifest schemas, modifiers, bundle signing, version churn release-by-release.

**Tooling, testing, ecosystem**
- [**Tooling**](tooling.md) — `hc` CLI, scaffolder, Launcher, kangaroo/Shipyard, `@holochain/client`, holonix.
- [**Testing**](testing.md) — Tryorama, TryCP, Sweettest, Wind Tunnel, common dev failure modes.
- [**Apps shipping**](apps.md) — Relay, Acorn, Neighbourhoods, hREA, HoloFuel.
- [**Ecosystem**](ecosystem.md) — Foundation, Holo Inc., Volla partnership, GitHub activity, conferences.

**Project lens**
- [**History**](history.md) — chronological narrative 2016 → 2026.
- [**Governance & funding**](governance.md) — Foundation/Holo/Unyt structure, ICO, HOT, CAL-1.0, decision-making.
- [**Abandoned features**](abandoned.md) — DPKI, lib3h, sim2h, mobile, light client, HoloFuel saga, cross-DNA bridging.
- [**Open problems**](open-problems.md) — Sybil, free-rider, sharding, group identity — what Holochain hasn't solved.
- [**Critiques**](critiques.md) — third-party + internal honest assessments. Tandfonline trilemma, Basis walk-away, "Friendly Reality Check," HoloFuel post-mortem.
- [**Comparisons**](comparisons.md) — vs Ethereum, libp2p, SSB, Pears, Spritely OCapN, Croquet, Bluesky/Nostr, wasmCloud/Spin.

**Reference**
- [**Lessons for Myrhiza**](lessons.md) — validates / avoid / borrow — **the consult-this-when-designing file.**
- [**Glossary**](glossary.md) — DNA, zome, cell, conductor, warrant, etc.

## How to use this prior-art doc

Designing a Myrhiza feature with overlap to Holochain? Start with [**lessons**](lessons.md) for the action-oriented summary, then drop into the relevant subsystem file for depth. The architecture and glossary files are for orienting newcomers; the lessons and open-problems files are for shaping decisions.

Doc lives, not snapshot — bump the date in this file's header on every meaningful update. Add new findings to whichever subsystem file owns them, and surface the consequence in [`lessons.md`](lessons.md).
