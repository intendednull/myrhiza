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

Read in roughly this order. Each file is independent and can be skimmed standalone.

1. [**Architecture**](architecture.md) — conductor, cells, DNAs, zomes, source chain, DHT.
2. [**Capabilities**](capabilities.md) — `ZomeCallCapGrant`, three access levels, comparison to WIT-typed handles.
3. [**Determinism**](determinism.md) — what the integrity/coordinator split enforces, what it doesn't.
4. [**Networking**](networking.md) — Kitsune2 deep dive, sim1h → sim2h → kitsune1 → kitsune2 history.
5. [**Identity**](identity.md) — keys, source chain signing, DPKI saga, warrants.
6. [**Browser viability**](browser.md) — why there's no native browser conductor and what they do instead.
7. [**Distribution & versioning**](distribution.md) — hApp bundle format, ABI churn release-by-release.
8. [**Apps shipping**](apps.md) — Relay, Acorn, Neighbourhoods, hREA, HoloFuel.
9. [**Open problems**](open-problems.md) — what Holochain hasn't solved and probably can't structurally.
10. [**Lessons for Myrhiza**](lessons.md) — validates / avoid / borrow — the consult-this-when-designing file.
11. [**Glossary**](glossary.md) — DNA, zome, cell, conductor, warrant, etc.

## How to use this prior-art doc

Designing a Myrhiza feature with overlap to Holochain? Start with [**lessons**](lessons.md) for the action-oriented summary, then drop into the relevant subsystem file for depth. The architecture and glossary files are for orienting newcomers; the lessons and open-problems files are for shaping decisions.

Doc lives, not snapshot — bump the date in this file's header on every meaningful update. Add new findings to whichever subsystem file owns them, and surface the consequence in [`lessons.md`](lessons.md).
