**Date:** 2026-05-08
**Status:** active
**Subject:** Spritely Goblins / OCapN — distributed object-capability runtime + cross-implementation network protocol

# Spritely Goblins / OCapN

Spritely Goblins is a distributed object-capability programming environment built around the *vat* — a single-threaded actor event loop processing messages transactionally. **OCapN** (Object Capability Network) is the cross-implementation network protocol Spritely is co-designing with Agoric, MetaMask, and Cap'n Proto: CapTP semantics on top of pluggable netlayers, addressable via `ocapn://` sturdyrefs.

The Spritely lineage runs continuous from Hewitt 1973 → KeyKOS → Mark Miller's E (1997) → *Robust Composition* (2006) → Cap'n Proto / Sandstorm (2014) → Goblins (2018). It is the cleanest in-the-wild expression of object-capability discipline as a distributed systems primitive. Mature enough to host Brassica chat, Goblinville (multiplayer browser demo), and the Shepherd × Goblins fleet-orchestration work; not yet shipping production end-user apps at scale.

## Key facts

| Fact | Value |
|---|---|
| Founded | 2018 (Spritely Project, Christine Lemmer-Webber); Spritely Networked Communities Institute incorporated April 23, 2022 (501(c)(3)) |
| Co-founders | Christine Lemmer-Webber, Randy Farmer (Electric Communities Habitat veteran) |
| Current leadership | ED: Christine Lemmer-Webber. CTO: David Thompson. Founding Technologist: Jessica Tallon (ActivityPub co-author, Goblins/OCapN lead) |
| Primary language | Guile Scheme (canonical); Racket (maintained); Hoot-WASM (browser); Rust port exists as community/research |
| License | Apache-2.0 |
| Repo scale | Guile Goblins ~1,939 commits on Codeberg |
| Current versions | Goblins 0.18.0 ("sleepy actors"); Hoot 0.7.0; OCapN draft specs (CapTP / Netlayers / Locators) pre-1.0 |
| Funding | NLnet/NGI Assure grants, Sovereign Tech Fund, individual donors. >$3M cumulative; 2024–2025 supporter drive raised ~$90K from 500+ donors |
| Adjacent stack | Agoric Endo (`@endo/captp`), MetaMask Snaps, Cap'n Proto/Cloudflare. The most-deployed CapTP-shaped traffic in 2026 runs through these, not Spritely. |

## Contents

Each file is independent and can be skimmed standalone.

**Technical subsystems**
- [**Architecture**](architecture.md) — vats, actors, near vs far refs, time-travel debugging, sealers, distributed GC.
- [**CapTP and OCapN**](captp-and-ocapn.md) — wire-level message types, four-table abstraction, sturdyrefs + swiss-numbers, netlayers (Tor / TCP+TLS / libp2p / WebSocket / UDS / Prelay).
- [**Capabilities**](capabilities.md) — refs as first-class values, attenuation, no-forge invariants. Comparisons to Holochain caps and Component Model handles.
- [**Persistence**](persistence.md) — persistent vats, Bloblin store, sleepy actors, sturdyrefs as persistence boundary.

**Implementations + apps + ecosystem**
- [**Implementations**](implementations.md) — Guile / Racket / Hoot-WASM / Rust port / Endo / Cap'n Proto / DObjects.
- [**Apps**](apps.md) — Brassica, Goblin Chat, Cirkoban, Goblinville, Mandy, Shepherd × Goblins.
- [**Ecosystem**](ecosystem.md) — Spritely Institute, Mark Miller, OCapN governance, adjacent projects, community size.

**Project lens**
- [**History**](history.md) — chronological narrative 1973 (Hewitt actor model) → KeyKOS → E (1997) → Sandstorm (2014) → Spritely (2018) → OCapN (2022) → 2026.
- [**Critiques**](critiques.md) — third-party + internal honest assessments. E never crossed 100 users; OCapN pre-spec drift; Cloudflare Cap'n Web ships in production while Spritely doesn't.
- [**Comparisons**](comparisons.md) — vs Holochain, Erlang/BEAM, Akka/Pekko, Cap'n Proto, Endo/Agoric, Component Model + WIT, Croquet, Tahoe-LAFS.
- [**Open problems**](open-problems.md) — discovery, Sybil, durability, performance, adoption, real-time co-presence, recovery, formal verification, cyclic GC, mass revocation.

**Reference**
- [**Lessons for Myrhiza**](lessons.md) — validates / avoid / borrow — **the consult-this-when-designing file.**
- [**Glossary**](glossary.md) — vat, swissnum, sturdyref, netlayer, near/far ref, etc.

## How to use this prior-art doc

Designing a Myrhiza feature with overlap to Spritely or the broader ocap lineage? Start with [**lessons**](lessons.md) for the action-oriented summary, then drop into the relevant subsystem file for depth. Capabilities, CapTP, and persistence are the highest-leverage files for Myrhiza's design space.

Doc lives, not snapshot — bump the date in this file's header on every meaningful update.

**Framing disclosure.** These docs are written from a Component-Model-as-foundation stance — most "Implications for Myrhiza" sub-sections frame Spritely's choices through that lens. Future readers auditing whether Component Model itself is the right primitive should weigh the corpus accordingly: it's a learn-from-Spritely-into-CM artifact, not a neutral catalog. The Holochain prior-art folder carries the same disclosure for the same reason.
