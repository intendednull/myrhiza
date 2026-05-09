# Glossary

Spritely / OCapN / E-lineage terms used throughout this prior-art doc. Generic distributed-systems terms (gossip, consensus, etc.) are deferred to other glossaries.

## Object capability theory

- **Capability (in Mark Miller's sense)** — an unforgeable reference that simultaneously *designates* an object and *grants the authority* to invoke it. Possession is permission; there is no separate ACL check. A reference is a capability.
- **Object capability (ocap)** — a security model in which the only way to obtain a capability is to (a) be created with it, (b) receive it in a message from another holder, or (c) create the target object yourself. No global mutable state, no ambient authority.
- **Attenuation** — wrapping a capability in a more-restricted facet (e.g. `read-only`, `time-limited`) before passing it on, so the recipient gets strictly less authority than the sender held. Standard idiom in ocap design.
- **Principle of Least Authority (POLA)** — every component receives only the capabilities it strictly needs to do its job. The design discipline that capability systems make easy and ACL systems make hard.
- **Confused deputy** — Norm Hardy's 1988 attack pattern: a privileged program (the deputy) is tricked into using its own authority on behalf of a less-privileged caller. Capabilities prevent it because the deputy invokes the caller's own capability rather than its ambient authority.

## E lineage

- **Vat** — a single-threaded event loop containing some number of objects (actors). All objects in one vat share one thread of execution, one stack, and one event queue. The unit of concurrency and the boundary across which message delivery becomes asynchronous.
- **Actor / object** — a unit of state and behavior living inside a vat. In Goblins, "actor" and "object" are used interchangeably; the Goblins actor model unifies them.
- **Near ref** — a reference to an object in the *same vat* as the holder. Calls on near refs are synchronous and immediate.
- **Far ref** — a reference to an object in a *different vat* (which may be a different process or a different machine). Calls on far refs are asynchronous and return a promise.
- **Promise pipelining** — sending a message to the *promised result of another message* before the first message has resolved. Lets a chain of dependent calls cross the network in one round trip instead of N. The headline performance feature of CapTP and a defining E innovation.
- **Eventual send** — the asynchronous message-send used to talk to far refs (and optionally to near ones); produces a promise instead of a return value, by construction can never deadlock.
- **Sealer / unsealer** — a paired primitive (also called a "brand"): the sealer wraps a value into an opaque box; only the matching unsealer can open it. Used to build data-abstraction, rights amplification, and identity primitives without static typing.
- **Swiss number** — a high-entropy unguessable identifier embedded in a network reference. Possession of the swiss number is what makes the reference a capability — bearer-token semantics over the network.
- **Sturdyref** — a serializable, persistent capability that can be saved to disk, written into a file, mailed, etc., and later "redeemed" back into a live reference to the original object. The ocap analogue of a URL or a password-with-permission.

## Spritely / OCapN specific

- **CapTP** — Capability Transport Protocol. The wire-level message protocol for communication between vats: handles object-reference passing, eventual sends, promise resolution, promise pipelining, third-party handoffs, and acyclic distributed garbage collection. Originated in E in the late 1990s; reimplemented in Cap'n Proto, Agoric/SwingSet, and Spritely Goblins.
- **OCapN** — Object Capability Network. The Spritely-led, multi-organization (Spritely, Agoric, MetaMask, Cap'n Proto) standardization effort begun under an NLnet grant in October 2022. Defines three layered specs: **CapTP** (semantics), **Netlayers** (transport abstraction), and **Locators** (URI/addressing format).
- **Netlayer** — OCapN's pluggable transport abstraction. The generalization of E's old "VatTP". An OCapN netlayer must provide secure pairwise channels between machines; the protocol itself is transport-agnostic. Goblins ships netlayers for Tor onion services, TCP+TLS, libp2p, Unix domain sockets, Prelay (a centralized fallback relay), and WebSocket.
- **Goblins** — Spritely's actor-model + capability-security library, with interoperable Guile and Racket implementations. Provides local synchronous transactional execution, asynchronous remote messaging, OCapN over the network, persistence, and a time-traveling debugger.
- **Hoot** — Spritely's Scheme-to-WebAssembly compiler backend for GNU Guile, plus a self-contained WASM toolchain written in Scheme. Targets WasmGC. Enables Goblins (and OCapN) to run in the browser.
- **Brassica / Brassica-chat** — Spritely's experimental P2P chat application built on Goblins; a testbed for OCapN over arbitrary netlayers (Tor, E2EE relay, etc.).
- **Three Vats / Three Networks demo** — informal label for the class of OCapN interop demos in which three Goblins vats (e.g. "Server A", "Server B", "Carol") communicate seamlessly across three different network substrates, illustrating that OCapN's security and semantics are preserved across heterogeneous netlayers. Public materials don't use the literal phrase as a fixed release name; the September 2025 Shepherd × Goblins update is the prominent example.
- **Machine** — a host process containing one or more vats. CapTP secures *pairwise machine-to-machine* channels (the "MachineTP" sub-layer) and addresses objects via (machine, vat, object) tuples; sturdyrefs and locators encode this hierarchy.
- **Distributed GC (in CapTP)** — acyclic distributed garbage collection: when a remote machine no longer holds a reference, the holding machine is informed and may reclaim. Reference cycles spanning machines are *not* automatically collected — a known limitation Goblins inherits from CapTP.
- **Persistent vat** — a vat backed by a persistence store (e.g. the Bloblin store) that serializes its entire live object graph and can be cleanly stopped and resumed. Goblins guarantees that restored objects regain only the capabilities they held at save time — no privilege escalation on restore.
- **Sleepy actor** — an actor swapped out of memory under a configurable caching policy (introduced in Goblins 0.18) but whose reference remains live; restored from the persistence store on next message receipt.

## Sources

- [Mark Miller — *Robust Composition*](http://erights.org/talks/thesis/markm-thesis.pdf) — canonical reference for the conceptual vocabulary.
- [erights.org](http://erights.org/) — E-language and CapTP documentation, including [CapTP overview](http://erights.org/elib/distrib/captp/index.html).
- [Spritely Goblins documentation](https://files.spritely.institute/docs/guile-goblins/latest/) — authoritative for current Goblins / OCapN definitions.
- [OCapN draft specifications](https://github.com/ocapn/ocapn/tree/main/draft-specifications) — CapTP, Netlayers, Locators draft specs.
- [Hardy — "The Confused Deputy"](http://web.cs.wpi.edu/~cs557/f14/papers/confused_deputy-hardy.pdf).
