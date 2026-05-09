# Open problems — Croquet / Multisynq lockstep paradigm

**Date:** 2026-05-09
**Status:** active
**Subject:** Structural limits of the synchronized-VM / deterministic-lockstep architecture. What Croquet/Multisynq does *not* solve, and what Myrhiza must therefore solve differently or accept.

This file is the negative-space companion to `architecture.md` and `determinism.md`. The lockstep paradigm is elegant where it fits; it does not fit everywhere. Cross-link: `glossary.md`, `architecture.md`, `determinism.md`, `governance.md`, `comparisons.md`, `lessons.md`, `critiques.md`.

## 1. Reflector single point of failure

The reflector is the message-ordering authority. Without one, the simulation cannot advance — clients have no shared time and no event order. Multisynq operates a redundant DePIN reflector network ("Synchronizers"), and the reflector itself is stateless, but the *architectural dependency* on a coordinator is fundamental, not incidental. A pure-P2P session with no reflector is not a supported configuration; it is a research project (see Krestianstvo Luminary, AGERE 2019, which replaces the reflector with Gun DB gossip — described by its author as "transforms the only server related Croquet's part — Reflector ... into the pure peer-to-peer application"). For Myrhiza: any architecture that copies lockstep inherits the coordinator. Either pick a coordinator role explicitly, or pick a non-lockstep ordering primitive (CRDT merge, vector-clock causal order).

## 2. Reflector latency floor

Every message round-trips client → reflector → all clients before any view sees it. This sets a per-message latency floor of roughly the client-to-reflector RTT (tens of ms in practice; the Multisynq DePIN selects geographically near reflectors to minimize). CRDT merge has zero coordination latency by comparison — local writes apply locally, and propagation is asynchronous. For interactive single-user-feel local actions (typing, dragging) Croquet relies on view-side prediction; mismatches surface as snap-back. Myrhiza apps with strict local-feel needs cannot get there with reflector-mediated ordering alone.

## 3. Floating-point determinism gaps in JS

The original Smalltalk Croquet shipped its own software floating-point package precisely because the host's float math was untrustworthy. Modern Multisynq runs on V8 / SpiderMonkey / JavaScriptCore. IEEE-754 specifies basic arithmetic bit-exactly but explicitly leaves transcendentals (`Math.sin`, `cos`, `exp`, `log`, `pow`) implementation-defined to within last-bit precision. Three browsers, three answers. Multisynq's docs surface `Math.random` as deterministic (engine-replaced) but say less about `Math.sin` etc. — the operating posture in practice is "avoid in models." This is a constant footgun for physics-style state-apply code. Myrhiza's WASM Component Model substrate inherits the same problem at the WASM-host boundary — `f32`/`f64` arithmetic is deterministic, but any host-imported transcendental is suspect unless the runtime pins an implementation.

## 4. Long-running session state size

Models accumulate state. Snapshots accumulate with them. Multisynq snapshots periodically and treats the snapshot + tail-of-events as the canonical session, but there is no documented tombstone-GC or state-pruning mechanism — once a thing is in the model, removing it is the application author's job. For sessions that run for months (chat rooms, shared documents, long campaigns), snapshots only grow. Compare CRDTs, which have well-known but also-unsolved tombstone-bloat problems (see `crdts/open-problems.md`); the issue is shared, but Croquet has no equivalent of Yjs's `Doc.gc` flag.

## 5. Late-joiner cost

A new peer joining a long-running session downloads the latest snapshot and replays the tail of events since that snapshot. Snapshot download dominates for large sessions. The alternative — replaying the entire event log from scratch — would be verifiable against the original Models but is not the production path; Multisynq trusts the snapshot. Myrhiza's `state-apply` semantics, if they follow Croquet's approach, must decide: trust a snapshot (fast, requires snapshot-signing trust) or replay the full log (slow, self-verifying).

## 6. Schema migration

If a Model class gains a field, loses a field, or changes a field's type between SDK versions, an old snapshot will deserialize wrong or not at all. Multisynq exposes a per-class `types()` static for custom (de)serialization, but version-to-version migration is the application's responsibility — there is no framework-level migration story. In a peer-driven world where different peers may run different app versions simultaneously (see open problem 10), this becomes a coordination problem the framework does not solve.

## 7. Byzantine peers

A reflector orders messages but does not validate them semantically — the contract is "deliver this message to all replicas." A malicious peer can broadcast nonsense events; every replica's `state-apply` must therefore reject nonsense events identically, or one peer's accepted-nonsense will diverge from another's rejected-nonsense and the session forks. The reflector also does not check sender authentication beyond the API key / session join — peer identity within a session is by-convention. Compare event-sourced systems with signed events (Spritely's OCapN, Holochain's source-chain signatures): Croquet's event stream is unsigned at the application layer.

## 8. Group size scaling

Lockstep paradigms have a known scale ceiling. The reflector must broadcast every message to every client; bandwidth at the reflector is `O(N · message-rate)`. RTS games — the classical home of deterministic lockstep — cap out around 8–16 players for this reason. Multisynq markets "hundreds" of concurrent users (VentureBeat, 2023: "puts hundreds of players into web-based multiplayer action") and the architecture can do that, but "hundreds" is the ceiling, not the floor of the next-order-of-magnitude. Myrhiza apps that need 10k+ concurrent peers cannot use a single-session lockstep model.

## 9. Offline tolerance

A peer offline for a day returns to a session that has advanced thousands of events. The peer's local state is stale; the only path forward is download-fresh-snapshot. Local edits made offline have no path into the session — Multisynq is *online-first*, not local-first. Compare Automerge / Yjs, where offline edits merge cleanly on reconnect. For Myrhiza apps that need offline-edit-then-merge semantics, lockstep alone is insufficient; you would need a separate causal-merge layer for offline edits.

## 10. Cross-version peers in one session

If peer A runs `@multisynq/client` 1.1.0 and peer B runs 1.2.0, can they join the same session? The architecture demands deterministic Models — meaning the *same code* on every replica. The SDK's wire protocol may be backward-compatible, but the application's Model code must be byte-identical for `state-apply` to converge. In practice: rolling upgrades require the framework to detect mismatch and reject join, or the session forks silently. The Multisynq docs do not document this case in detail. Myrhiza must.

## 11. WASM Component Model integration

Multisynq is a JavaScript framework. There is no WASM Component Model artifact, no WIT interface, no component-side ABI for models. A would-be Myrhiza adopter cannot import Multisynq directly; the most that transfers is the *idea* (synchronized VM, reflector-as-orderer, model/view split). The actual host surface — what determinism guarantees the runtime provides, how snapshots are taken, how events are signed and ordered — must be designed natively for WASM. Treat Croquet as architectural inspiration, not as an importable dependency.

## 12. Sources

- Krestianstvo Luminary: Decentralized Virtual Time for Croquet architecture (AGERE 2019). https://2019.splashcon.org/details/agere/5/
- Multisynq client npm. https://www.npmjs.com/package/@multisynq/client
- Multisynq docs — Random in Models. https://docs.multisynq.io/tutorials/random
- Multisynq docs — Writing a Model / View. https://docs.multisynq.io/
- Smith, Kay, Raab, Reed, *Croquet — A Collaboration System Architecture*, C5 2003.
- VentureBeat, *Croquet makes it possible to put hundreds of players into web-based multiplayer action* (2023). https://venturebeat.com/business/croquet-makes-it-possible-to-put-hundreds-of-players-into-web-based-multiplayer-action
- Bruce Dawson, *Floating-Point Determinism* (2013). https://randomascii.wordpress.com/2013/07/16/floating-point-determinism/
- Glenn Fiedler, *Deterministic Lockstep* and *Floating Point Determinism*, gafferongames.com.
