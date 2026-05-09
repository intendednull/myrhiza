# Persistence

Goblins persistence is interesting because it's *manual but transactional* — Spritely chose explicit per-object self-portraits over orthogonal whole-image snapshots, then leaned hard on the actormap's transactional structure to make those self-portraits cheap to take. The story has shifted across releases (v0.13 → v0.17 → v0.18) and is now mature enough to back long-running applications.

## Machine snapshots: how Goblins does it

The unit of persistence is a **persistent vat** ([Persistent Vats docs](https://files.spritely.institute/docs/guile-goblins/0.15.1/Persistent-Vats.html)). `spawn-persistent-vat` takes:

- A `persistent-env` — the global registry of constructor names so the vat can rehydrate `^cell`, `^queue`, etc.
- A `spawn-roots-thunk` — only called on first run; produces the initial root object graph.
- A `store` — pluggable storage backend.
- `persist-on` — `'churn` (auto-save when the vat reaches quiescence between turns) or manual.
- `version` + `upgrade` — for migrating roots when the schema changes.

What gets serialized: each actor opts in via a "self-portrait" — a description of its persistable state. The [v0.13.0 release notes](https://spritely.institute/news/spritely-goblins-v0-13-0-object-persistence-and-easier-io.html) emphasize the security invariant: an actor "cannot describe itself with more power than it actually has access to," so a captured self-portrait can only contain refs the actor actually held. The `define-actor` macro generates the boilerplate. Versioning: every persisted object carries a version tag (default 0), and the `migrations` macro ([v0.14.0](https://spritely.institute/news/spritely-goblins-v0-14-0-libp2p-and-improved-persistence.html)) chains version bumps automatically.

Storage backends evolved sharply:

- **Syrup store** (v0.13, [Syrup Store docs](https://files.spritely.institute/docs/guile-goblins/0.14.0/Syrup-Store.html)) — on each churn, serialize the entire object graph to a Syrup-encoded log file. Simple, slow.
- **Bloblin store** (v0.17, October 2025, [release notes](https://spritely.institute/news/spritely-goblins-v0-17-0-persistence-is-better-than-ever.html)) — write the initial graph once, then stream compressed *deltas* (one per churn). "Many thousands of deltas per second to disk." A `persistence-store-copy!` migration tool moves Syrup stores to Bloblin.
- **IndexedDB store** (May 2026 commit on the `goblins` repo) — for browser persistence, used by the Hoot/WASM build.

The transactional actormap is the enabler: each turn produces a delta, so the persistence layer doesn't have to diff or scan, just append. The same property that makes time-travel debugging possible — both features piggyback on the actormap's structural sharing.

The **`^persistence-registry`** actor ([v0.14.0](https://spritely.institute/news/spritely-goblins-v0-14-0-libp2p-and-improved-persistence.html)) is the cross-vat coordinator. It is itself *not* persisted (spawned fresh on each program start), but it lets multiple persistent vats register themselves and resolve far refs across a program restart — the far-ref-as-promise that becomes a real far-ref once both ends register.

**Sleepy actors** (v0.18, April 2026, [release notes](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)) push this further: an idle actor can be flushed to the persistence store while keeping its reference live; an LRU policy maintains a hot working set. Effectively automatic memory paging for large actor populations.

## Sturdyrefs as the persistence boundary

A sturdyref is the persistent form of a *capability*, distinct from a persistent form of an *actor's local state*. The actor's state is in the vat's store; the sturdyref tells outside-world holders how to reach that actor again after a vat restart.

The pair is essential. After a restart:

1. The persistent vat replays its store, reconstructing actors. The same swiss-num resolves to the same actor instance because the swiss-num is part of the persisted bootstrap object's `fetch` table.
2. Outside holders re-resolve their sturdyref URL via a fresh CapTP session; `op:start-session` carries the session pubkey of the (still-same) machine; the bootstrap object's `fetch(swiss-num)` returns a fresh import.
3. Far refs the actor itself held are restored *as promises* (per persistence-registry semantics) and resolve once the peer comes back online.

So the persistence boundary is the **swiss-num inside the sturdyref**: it's the durable identity that survives restart. The actor's in-memory state is reconstructed from the store; the actor's network identity is reconstructed from the swiss-num + machine pubkey.

## Comparison to Holochain source chains

Both systems persist long-running state, but the *shape* differs structurally:

| Dimension | Holochain source chain | Goblins persistent vat |
|---|---|---|
| **Topology** | Append-only log per agent. Hash-linked, signed by author. | Living object graph in an actormap, with a per-turn delta stream. |
| **Identity** | The cell `(DNA hash, agent pubkey)`. Stable for the agent's lifetime. | The vat (process-local) + the machine pubkey (network identity). Sturdyref+swiss-num for cross-process. |
| **Replay semantics** | Validation must be deterministic. Source chains are gossiped to authority neighborhoods who replay validation. | No replay — the actormap is the canonical state. Time-travel debugger walks history but isn't load-bearing for correctness. |
| **Schema evolution** | Integrity-zome version bump forks the network (changes DNA hash). Coordinator zomes can hot-swap. | Per-object version tag + `migrations` macro. No notion of "forking the network." |
| **What lives there** | Actions: `Create`/`Update`/`Delete`/`CreateLink`/etc., each producing DHT ops. | Self-portraits of actors, plus deltas of cell/queue/etc. updates. |
| **Distribution model** | Public actions gossip into a sharded validating DHT. | Persistence is local; distribution is *separate* via CapTP. |
| **What "verifying" means** | Every authority neighbor re-runs validation callbacks deterministically. | No third-party verification — capability discipline replaces it. |
| **Failure mode** | Source-chain forks (rare, structurally prevented). | Vat process crash → restore from store; cycles between vats can leak refs. |

The deeper difference: Holochain is fundamentally a *log-replication* system that happens to host imperative logic. Goblins is fundamentally a *capability-graph* system that happens to need durability. Holochain validates by re-running deterministic functions over signed ops; Spritely validates by trusting the lexical scope of the host language and the unforgeability of the actor reference.

For Myrhiza this is the cleanest dichotomy in the prior-art set. Two coherent positions:

- *Holochain-shape*: state is an append-only signed log; correctness comes from deterministic re-validation by peers; sharing is gossip-into-DHT. Determinism is mandatory.
- *Spritely-shape*: state is a live actor graph; correctness comes from capability discipline + transactional turns; sharing is direct CapTP between consenting parties. Determinism is unnecessary outside of intra-vat transactional rollback.

A Component Model + WASM runtime can host either model. Resource handles map naturally to Goblins-style refs, which is the more direct fit; if Myrhiza wants Holochain-style validating DHT semantics for some app, those have to be *built on top of* the cap layer, not the other way around. (The reverse — building cap discipline on top of a validation-DHT primitive — is what Holochain has been retrofitting and is why their cap story is the weakest part of their stack.)

## Honest gaps

- The Goblins persistence layer assumes the host process is trusted. If the on-disk Bloblin file is tampered with, an attacker can synthesize alternate state. Spritely treats this as out-of-scope (filesystem trust is the OS's problem); Holochain solves it by signing every chain entry. Trade-off, not a bug.
- Cross-vat persistence works (via `^persistence-registry`), but cross-*machine* state coherence is not addressed by Goblins itself — left to applications that build on top of CapTP. No built-in CRDT or replication layer; the [community has discussed](https://community.spritely.institute/t/composing-capability-security-and-conflict-free-replicated-data-types/781) composing CRDTs but it's user-space.
- Sleepy-actors + IndexedDB + Hoot/WASM is the 2025-2026 frontier; the persistence story in the browser is younger than the desktop story and likely to evolve.

## Sources

- [Persistent Vats — guile-goblins 0.15.1](https://files.spritely.institute/docs/guile-goblins/0.15.1/Persistent-Vats.html)
- [v0.13.0 release notes (object persistence)](https://spritely.institute/news/spritely-goblins-v0-13-0-object-persistence-and-easier-io.html)
- [v0.14.0 release notes (persistence-registry, migrations)](https://spritely.institute/news/spritely-goblins-v0-14-0-libp2p-and-improved-persistence.html)
- [v0.17.0 release notes (Bloblin store)](https://spritely.institute/news/spritely-goblins-v0-17-0-persistence-is-better-than-ever.html)
- [v0.18.0 release notes (sleepy actors)](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)
- [Syrup Store — guile-goblins 0.14.0](https://files.spritely.institute/docs/guile-goblins/0.14.0/Syrup-Store.html)
- [Heart of Spritely whitepaper](https://files.spritely.institute/papers/spritely-core.html)
- [Composing capability security and CRDTs (community thread)](https://community.spritely.institute/t/composing-capability-security-and-conflict-free-replicated-data-types/781)
- [Holochain source chains (sibling prior-art doc)](../holochain/architecture.md)
