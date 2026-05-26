**Date:** 2026-05-22
**Status:** active
**Subject:** Side-by-side comparisons with Spritely/OCapN, Agoric/SwingSet, Akka/Pekko, Orleans, plus a "what about WASM?" panel

# Comparisons

OTP did not invent actor-runtimes; it survived as one. The comparisons below place BEAM in the broader actor / capability / live-runtime landscape, with explicit cross-links to other prior-art folders.

## Actor lineage vs. ocap lineage

Two parallel research traditions arrived at similar-looking runtime shapes from different roots:

- **Actor lineage:** Hewitt 1973 → Erlang (1986) → Akka (2009) → Orleans (2010) → Elixir (2011). Emphasis on isolated processes, message-passing, supervision, failure-isolation. Identity is opaque (PIDs, ActorRefs). Authority is implicit (you can send to any actor whose address you have, but the network layer decides what crosses).
- **Ocap lineage:** Hewitt → Actors (1973) → KeyKOS (1980) → E (1997) → Spritely Goblins (2018) → Agoric SwingSet (2018). Emphasis on capabilities as unforgeable references, where authority and identity are unified — *having* a reference *is* the right to invoke. Distribution protocol (CapTP, OCapN) preserves cap discipline across the wire.

BEAM is squarely on the actor side. PIDs are forgeable across the wire (the cookie auth model trusts whoever's in the cluster), there is no explicit capability primitive, supervision is the security boundary not the authority boundary. Production-validated; not capability-secure.

Spritely Goblins and Agoric SwingSet are squarely on the ocap side. References are unforgeable, authority is bundled with references, distributed GC is a first-class concern, cross-network calls preserve capability discipline.

**See [`prior-art/spritely-ocapn/`](../spritely-ocapn/) and [`prior-art/agoric-endo/`](../agoric-endo/) for the ocap side. Read [`prior-art/spritely-ocapn/captp-and-ocapn.md`](../spritely-ocapn/captp-and-ocapn.md) and [`prior-art/agoric-endo/vat-model.md`](../agoric-endo/vat-model.md) alongside this corpus's [`architecture.md`](architecture.md).**

The Myrhiza decision: **Myrhiza chooses the ocap side** (per `CLAUDE.md`: "Capabilities are the only host surface"). BEAM is *not* the model to copy here; Spritely + Agoric are. What BEAM offers is the operational lessons — supervision, message-passing, hot reload — that the ocap-runtime world has not yet had decades to learn.

## Vat-state vs. BEAM-process-state

A direct comparison the master spec cares about ([`prior-art/willow/open-problems.md:131-140`](../willow/open-problems.md)):

| Property | BEAM process | SwingSet vat | Spritely Goblins actor |
|---|---|---|---|
| State location | Process-private heap, in RAM | XS heap snapshot + transcript log | Actor-private; persistence by application |
| Identity | PID (forgeable on the wire) | KrefID (unforgeable, cap-secure) | Cap reference (unforgeable) |
| Persistence | Application's problem (Mnesia/ETS/files) | Kernel's problem (transcript-driven replay) | Application's problem |
| Hot upgrade | `code_change/3` callback | `vat:upgrade` with `baggage` survivor pattern | Module reload, application-driven state migration |
| Replay-after-crash | Mostly no (restart with init state) | **Byte-for-byte replay from transcript** | No (typically) |
| Determinism guarantee | None (concurrent, non-deterministic) | **Strict** (consensus-critical) | Per-turn deterministic; whole-vat replay not designed-for |

**This is the most directly Myrhiza-relevant comparison row in the whole corpus.** Myrhiza's `state-apply` is closer to SwingSet's vat-replay than to BEAM's "let it crash, restart with init state" shape. The supervisor-tree pattern from BEAM borrows; the persistence-via-application pattern does not.

See [`prior-art/agoric-endo/determinism.md`](../agoric-endo/determinism.md) and [`prior-art/agoric-endo/persistence.md`](../agoric-endo/persistence.md) for the SwingSet detail.

## Akka and Pekko (JVM actor world)

- **Akka** — JVM actor library, originally by Jonas Bonér, founded 2009. Lightbend (the commercial steward) changed Akka's license to **Business Source License 1.1 (BSL)** in **2022-09-07**, ending Apache-2.0 distribution of new versions. Community forked the Apache-2.0 fork as **Apache Pekko** (2022-11 incubation, 2024-06 graduated to Apache top-level).
- **Pekko** is the Apache-2.0 community fork, actively maintained, used by Play Framework and others that didn't want to take the BSL license.

**Differences from BEAM:**

- JVM-based; no per-actor GC; one JVM heap shared across all actors. Different failure-isolation story.
- Actor message passing implemented in user-space; no scheduler-level reduction-counting equivalent.
- Cluster sharding is the marquee distributed feature; structurally similar to BEAM's `pg` + manual coordination. Akka adds Akka-Cluster which provides leader election, cluster singletons, sharded actor mailboxes.
- **No hot code reloading.** JVM has class-loader-swapping but actor state survives is the application's problem.

**Lesson:** Akka's BSL relicense + Pekko fork **is the cautionary tale for "commercially-stewarded runtimes":** even an Apache-2.0 codebase under a single corporate steward can be relicensed without notice if there is no foundation safety net. The EEF + Ericsson dual-stewardship pattern (Ericsson owns the implementation, EEF owns the community trademark assets) is more robust than the Lightbend single-steward pattern was.

## Orleans (.NET virtual actors)

- Microsoft Research project, public ~2010, OSS Apache-2.0. Now maintained by .NET Foundation.
- **Virtual actors** — actor lifetime managed by the runtime, not the user. Actors are addressable by ID; the framework activates and deactivates instances as needed. This is the marquee architectural difference from BEAM (where you spawn explicitly).
- **Persistence is first-class** — actor state can be declared as durable storage-backed (Azure tables, SQL, custom).
- **Used in production at Microsoft for Halo, Skype, others.**

**Differences from BEAM:**

- Virtual-actor lifecycle hides the spawn/supervise complexity (also hides the failure-isolation surface). Whether this is good or bad depends on the application.
- Hot reload exists at the .NET assembly level; Orleans-specific hot upgrade is via grain (actor) versioning + serialised state — closer in shape to `code_change/3` than to JVM class-loader voodoo.

## Other "live runtime" comparisons

- **Smalltalk-80** — the ur-example of "the running system is the development environment." BEAM's hot-loading inherits this attitude from the Lisp / Smalltalk traditions. Smalltalk's live image is more radical than BEAM's; BEAM kept type-pattern-matched code paths discrete.
- **Common Lisp** — `(load "file.lisp")` in production has been a thing since 1984. Many of BEAM's hot-loading ergonomics are direct copies of Lisp's image-based development.
- **Java HotSwap / DCEVM** — JVM's "swap method bodies in a running class" capability. Much narrower than BEAM (no schema migration callback, no full module replacement). Used mainly for development hot-reload (JRebel commercialised it).

## What WASM Component Model gives that BEAM doesn't

This is the Myrhiza-pitch panel; bias-flagged in the README framing disclosure.

- **Polyglot.** CM components in Rust, Go, JS, Python, C, ... all interoperate via WIT. BEAM is mostly "Erlang or Elixir (or Gleam, LFE, ...)" — same VM, same language family, plus NIFs.
- **Static typing at the interface layer.** WIT is strongly typed; ABI mismatches caught at link time. BEAM is dynamically typed; ABI mismatches caught at runtime.
- **No GC required.** WASM components manage their own memory (linear memory + free / their language's GC if any). The substrate doesn't impose one.
- **Stronger isolation.** WASM linear memory is genuinely sandboxed; a buggy component cannot corrupt sibling components. BEAM NIFs can corrupt the entire node.
- **Cryptographic component identity.** Components are content-addressed (hash of the module). BEAM modules are name-addressed (atoms).

## What BEAM gives that WASM Component Model doesn't (today)

- **Hot code reloading.** Mature 30+ year story (warts and all; see [`hot-code-loading.md`](hot-code-loading.md) and [`critiques.md`](critiques.md)). CM has no equivalent.
- **Process model + supervision.** Built-in, not library-level. CM components are spun up by the host; there is no in-tree supervision contract.
- **Distributed runtime.** Distributed Erlang is in-tree (with its trust caveats; see [`distribution.md`](distribution.md)). CM has no transport story; each runtime (Wasmtime, etc.) brings its own or punts to the application.
- **Tracing primitive.** `erlang:trace/3` is BEAM-unique among modern runtimes.
- **Operational ecosystem.** `observer`, `recon`, `redbug`, `:telemetry`, the EEF Observability WG — a coherent ops story polished over decades. Wasmtime's observability story is much younger.

**Implications for Myrhiza:** the substrate trade-off is real and worth being explicit about. WASM CM gives Myrhiza substrate properties BEAM doesn't (polyglot, static types, stronger isolation, content-addressed identity); BEAM gives operational properties WASM CM doesn't yet have (supervision, hot reload, mature observability). The Myrhiza runtime is building those operational properties on top of the WASM CM substrate, and the prior-art folders for both sides (this folder + the WASM CM folder) are how we keep track of what we're borrowing from each tradition.

## Sources

- Akka relicense (Lightbend, 2022-09-07): <https://www.lightbend.com/blog/why-we-are-changing-the-license-for-akka>
- Apache Pekko: <https://pekko.apache.org/>
- Microsoft Orleans: <https://learn.microsoft.com/en-us/dotnet/orleans/>
- Hewitt actor model (1973): <https://www.cypherpunks.to/erights/history/actors.pdf>
- E language (Mark Miller): <http://erights.org/>
- Spritely Goblins prior-art folder: [`../spritely-ocapn/`](../spritely-ocapn/)
- Agoric SwingSet prior-art folder: [`../agoric-endo/`](../agoric-endo/)
- WASM Component Model prior-art folder: [`../wasm-component-model/`](../wasm-component-model/)
- Smalltalk-80 (Goldberg & Robson, 1983)
