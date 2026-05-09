# Architecture

Spritely Goblins is a [distributed object programming environment](https://codeberg.org/spritely/goblins) built around the *vat* — an event loop in a single thread that owns a set of *actors* and processes their messages transactionally. The shape is borrowed almost wholesale from Mark Miller's [E language](https://en.wikipedia.org/wiki/E_(programming_language)) (1997), then reified twice: a Racket implementation (`gitlab.com/spritely/goblins`, the original) and a Guile implementation that Spritely now treats as the lead codebase ([Goblins page](https://spritely.institute/goblins/)). At time of writing the Guile repo on Codeberg is at v0.18.0 (April 2026), with ~1,939 commits and an Apache-2.0 license. A Rust port exists at [gitlab.com/spritely/goblins-rust](https://gitlab.com/spritely/goblins-rust) but is community/research-shaped, sporadically updated, not the focus of Spritely's funded work tracks — see [`implementations.md`](implementations.md) for the full status.

```
   Machine A                                  Machine B
  +---------------------------+              +---------------------------+
  |  Vat-1     Vat-2   Vat-3  |              |  Vat-1                    |
  |  +-----+   +-----+ +-----+|   CapTP      |  +---------------+        |
  |  | a b |   | c d | |  e  |<------------->|  |   f   g   h   |        |
  |  +-----+   +-----+ +-----+|  netlayer    |  +---------------+        |
  |     ^near  ^near    ^near |  (Tor/TLS/   |       ^near               |
  |        ----- far ------>  |   libp2p/    |       <--- far ----+      |
  |           (cross-vat)     |   uds/ws)    |                           |
  +---------------------------+              +---------------------------+
```

## Actors and `become`

A Goblins actor is a procedure with private state, addressed only by reference. Mutation is not in-place; the actor calls `bcom` ("become") to substitute a new behavior for the next message turn. The canonical cell is three lines of Scheme ([Dustycloud preview](https://dustycloud.org/blog/goblins-time-travel-micropreview/)):

```scheme
(define (^cell bcom [val #f])
  (case-lambda
    [() val]
    [(new-val) (bcom (^cell bcom new-val))]))
```

This differs sharply from Erlang actors. Erlang processes are independently scheduled, mailboxes are per-process, and "become" is implicit in tail-recursive receive loops. Goblins actors share an event loop per vat, so two near actors invoking each other run in the *same* call stack — synchronously, transactionally — via the `$` operator. Cross-vat work uses `<-` and returns a promise. Promises and `become` together are why Goblins is described as "[quasi-functional](https://docs.racket-lang.org/goblins/intro.html)": each turn returns a delta that the vat applies atomically, so an unhandled exception rolls back the turn rather than corrupting state.

## Vats: near vs far

A vat is *the* unit of synchrony. Two refs are "near" iff they live in the same vat; everything else is "far" ([Vats — guile-goblins 0.16.1](https://files.spritely.institute/docs/guile-goblins/0.16.1/Vats.html)). All actor state lives inside an `actormap` data structure that the vat owns. Near refs allow `($ a 'method args ...)` for a synchronous call that resolves before the turn ends; far refs only support `(<- a 'method args ...)`, which queues a message and returns a promise/vow.

This near/far distinction is a hard invariant — there is no `force` to upgrade a far ref to a near ref. A guest can never accidentally block a vat by reaching into another vat. The cost is asymmetry: you write `$` for fast paths and `<-` for everything else, and refactoring a near ref to a far ref means converting call sites.

## Promise pipelining

Pipelining is what CapTP buys you over plain RPC. Without it, you `<-` to get a promise, `on` to subscribe, then `<-` again from the callback ([Promise pipelining docs](https://files.spritely.institute/docs/guile-goblins/0.10/Promise-pipelining.html)):

```scheme
;; B -> A -> B -> A -> B  (5 hops)
(on car-vow
  (lambda (car) (on (<- car 'drive) ...)))
```

With pipelining, you address the unresolved promise itself:

```scheme
;; B -> A -> B  (3 hops)
(on (<- car-vow 'drive) ...)
```

A's vat queues the second message against the unresolved answer slot; when the local promise resolves, A delivers the second message without involving B. This is "[remote car factory drives the car as soon as it's made, before B knows it exists](https://spritelyproject.org/news/what-is-captp.html)." See [`captp-and-ocapn.md`](captp-and-ocapn.md) for the wire-level mechanism (the `answer-pos` field of `op:deliver`).

## Time-travel debugging

Goblins' headline feature: the `actormap` is a transactional, persistent data structure, so the vat can keep a snapshot per event and the debugger walks history. From the [v0.11.0 announcement](https://spritely.institute/news/spritely-goblins-v0-11-0-released-time-travel-distributed-debugging-and-more.html) and the [distributed debugger writeup](https://spritely.institute/news/introducing-a-distributed-debugger-for-goblins-with-time-travel.html): "Goblins uses transactional heaps to manage actor state, enabling us to take snapshots of a vat before each event is processed." The Guile REPL meta-commands `,vat-peek`, `,vat-tree`, `,vat-graph` (Lamport causality), and `,vat-trace` (distributed backtrace) inspect past turns *without* re-executing them. Limitation as of writing: cross-process distributed debugging is documented but the implementation supports multiple vats only within a single Guile process.

The feature that earns Goblins its reputation. Exists because every turn produces a delta over an immutable map; you don't need a separate event-sourcing layer.

## Sealers and unsealers

Sealers are Spritely's primitive for typed value tagging without cryptography. `spawn-sealer-triplet` returns three procedures: `seal`, `unseal`, and a `check?` predicate ([Sealers docs](https://files.spritely.institute/docs/guile-goblins/0.13.0/Sealers.html)). Only the matching `unseal` recovers the inner value; `check?` answers "did *this* sealer make this brand?" The implementation today uses encapsulated cookie comparison (faster than the W7-style closure-only version used in early Goblins) ([v0.16.0 release notes](https://spritely.institute/news/spritely-goblins-v0-16-0-released.html)). Use cases: branded structs (analog of nominal types), rights amplification, persisted-secret containers.

## Distributed garbage collection

Each side of a CapTP session keeps four tables — imports, exports, questions, answers — mapping small integers to local refs ([erights.org Four Tables](http://erights.org/elib/distrib/captp/4tables.html); [Goblins CapTP docs](https://files.spritely.institute/docs/guile-goblins/0.10/CapTP-The-Capability-Transport-Protocol.html)). When a ref's local refcount drops, the side sends `op:gc-exports` (was `op:gc-export`, batched in 0.18 — [release notes](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)) and the peer drops the corresponding entry. **Acyclic only**: cycles spanning machines leak. The Goblins docs are honest about this — "full cycle-collecting distributed GC has been written, but requires special cooperation from the language GC that we don't have access to in Racket (or in most languages)."

## Hosts vs vats vs machines

Three nested layers, often conflated:

- **Vat** — single-threaded actor scheduler. One process can host many vats. The unit of synchrony.
- **Machine** — the network identity, owning a public/private keypair and a set of vats. Created by `spawn-mycapn`. The unit of trust ("a misbehaving machine is treated like one big misbehaving object").
- **Host** — the OS process or browser tab. One host runs at least one machine; multiple machines per host are possible for testing.

Across hosts, machines speak CapTP over a netlayer. Within a host, vats speak CapTP-in-process via `fake-intarwebs` (test) or directly through the actormap.

## Sources

- [Spritely Goblins (Codeberg)](https://codeberg.org/spritely/goblins)
- [What is Goblins? (Racket docs)](https://docs.racket-lang.org/goblins/intro.html)
- [Vats — guile-goblins 0.16.1](https://files.spritely.institute/docs/guile-goblins/0.16.1/Vats.html)
- [Promise pipelining](https://files.spritely.institute/docs/guile-goblins/0.10/Promise-pipelining.html)
- [Time-travel debugging announcement](https://spritely.institute/news/spritely-goblins-v0-11-0-released-time-travel-distributed-debugging-and-more.html)
- [Distributed debugger writeup](https://spritely.institute/news/introducing-a-distributed-debugger-for-goblins-with-time-travel.html)
- [Goblins time-travel preview (Dustycloud)](https://dustycloud.org/blog/goblins-time-travel-micropreview/)
- [Sealers](https://files.spritely.institute/docs/guile-goblins/0.13.0/Sealers.html)
- [v0.16.0 release notes](https://spritely.institute/news/spritely-goblins-v0-16-0-released.html)
- [v0.18.0 release notes (sleepy actors)](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)
- [Heart of Spritely whitepaper](https://files.spritely.institute/papers/spritely-core.html)
- [erights.org — CapTP Four Tables](http://erights.org/elib/distrib/captp/4tables.html)
- [E (programming language)](https://en.wikipedia.org/wiki/E_(programming_language))
