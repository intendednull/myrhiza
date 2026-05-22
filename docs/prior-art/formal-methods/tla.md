**Date:** 2026-05-22
**Status:** active
**Subject:** TLA+, TLC, PlusCal, Apalache, Quint, TLAPS

# TLA+ and its model-checkers

TLA+ is a specification language for concurrent and distributed systems, designed by Leslie Lamport in the late 1990s. A spec is a state machine: declared variables, an *Init* predicate on the initial state, a *Next* relation over (state, state') transitions, and a set of *invariants* (safety) and *temporal properties* (liveness) the system must satisfy. The language is mathematical (set theory + first-order logic + temporal logic of actions), deliberately *not* a programming language. You don't compile a TLA+ spec — you *check* it.

The toolchain has three load-bearing pieces: **TLC** (explicit-state model checker, ships in the TLA+ Tools repo), **Apalache** (symbolic model checker, separate project from Informal Systems), and **PlusCal** (an algorithmic frontend that translates to TLA+ for users who prefer pseudo-code shape over math). A fourth piece, **TLAPS** (TLA+ Proof System), exists for theorem-prover-style proofs but is far less used and not part of the recommended adoption path here. A fifth, newer surface — **Quint** — is an engineer-friendly DSL that compiles to TLA+ and uses Apalache as its primary backend; it is the most active surface improvement to TLA+ in the last 3 years.

## What each piece does

### TLA+ language

The mathematical core. Specs look like:

```tla
VARIABLES queue, processed
Init == queue = <<>> /\ processed = {}
Enqueue(x) == queue' = Append(queue, x) /\ UNCHANGED processed
Dequeue == queue # <<>> /\ queue' = Tail(queue) /\ processed' = processed \cup {Head(queue)}
Next == \E x \in Items : Enqueue(x) \/ Dequeue
Spec == Init /\ [][Next]_<<queue, processed>>
Invariant == \A i \in processed : i \in Items
```

`Init` defines initial states, `Next` defines allowed transitions, `Spec` is the closed temporal formula, and the model checker walks the reachable state space verifying every state satisfies `Invariant`.

### TLC — explicit-state model checker

TLC enumerates the reachable state space breadth-first, hashing each state to detect revisits, and reports the first state that violates an invariant along with a counterexample trace. It ships in the same repo as the TLA+ Tools ([`tlaplus/tlaplus`](https://github.com/tlaplus/tlaplus), MIT licensed, latest **v1.8.0 "Clarke release"** 2026-05-18, primary language Java). Strengths: fast on small bounded models, mature, handles real-world specs at scale; AWS reports TLC runs of tens of billions of states for some of their specs. Limitations: explicit-state means state-space blow-up is the dominant cost — you constantly tune bounds (set sizes, queue depths) to keep runs feasible.

### Apalache — symbolic model checker

Apalache ([`apalache-mc/apalache`](https://github.com/apalache-mc/apalache), Apache-2.0, latest **v0.57.0** 2026-04-24, primary language Scala) reformulates TLA+ verification as an SMT problem and dispatches to **Z3**. Instead of enumerating states it bounds the *depth* of the trace and asks the solver "is there a length-`k` execution that violates the invariant?". Strengths: handles unbounded data types (large or infinite sets, ranges) more gracefully than TLC; can find counterexamples that explicit-state would miss because the violating state lives far from any TLC-reachable region. Limitations: not all TLA+ constructs are supported — Apalache works on a *type-annotated* TLA+ subset and rejects specs that use too-dynamic constructs without type hints.

**TLC vs Apalache** is a real tradeoff, not "Apalache is the new TLC." TLC remains the default for most real specs. Apalache wins when state-space size kills TLC or when you need to prove an *inductive invariant* symbolically. Recent practitioners describe running both side-by-side as the realistic posture. See [Quint blog: "Why I use TLA+ and not(TLA+)"](https://protocols-made-fun.com/specification/modelchecking/tlaplus/quint/2024/10/05/tla-and-not-tla.html) for one team's framing.

### PlusCal — algorithmic frontend

PlusCal is a `--algorithm`-shaped pseudocode that translates to TLA+. Looks like:

```
--algorithm Queue {
  variables queue = <<>>, processed = {};
  process (Producer = "p") { while (TRUE) { with (x \in Items) { queue := Append(queue, x); } } }
  process (Consumer = "c") { while (TRUE) { await queue # <<>>; processed := processed \cup {Head(queue)}; queue := Tail(queue); } }
}
```

Two flavors ship — C-syntax (`pcal`) and Pascal-syntax (`pluscal`). The translator emits TLA+, which then runs through TLC or Apalache exactly as a hand-written spec would. PlusCal narrows TLA+'s expressiveness in exchange for a code-like surface; engineers reach for it when the algorithm-shape of the spec is more natural than the state-machine-shape.

### Quint — engineer-friendly DSL

Quint ([`informalsystems/quint`](https://github.com/informalsystems/quint), Apache-2.0) is a newer TLA+ frontend from Informal Systems that compiles to TLA+ and uses Apalache as its primary backend, with full TLA+ output for TLC as a secondary path. Quint trades some of TLA+'s mathematical generality for a regular syntax, a VS Code extension with type-checking, a REPL, and a CLI that installs via `npm install -g @informalsystems/quint`. The honest framing: Quint is the most active improvement on TLA+'s engineer-experience surface in 2024–2026, and worth knowing about, but it is not yet the dominant entry point — most adoption stories cited in [`adoption.md`](adoption.md) start from raw TLA+ or PlusCal.

### TLAPS — proof system

TLAPS lets you prove TLA+ theorems with explicit proof steps backed by Isabelle/HOL, Z3, and other backends. Far less used than TLC/Apalache. Worth knowing about; not in the recommended Myrhiza adoption path.

## Workflow

The standard loop is:

1. **Sketch the state machine in PlusCal or TLA+.** Pick the *coarsest* abstraction that still captures the property you care about. AWS's CACM 2015 paper is explicit: you are modeling the *design*, not the implementation.
2. **Define invariants** (safety: "no two leaders elected in the same term") and **temporal properties** (liveness: "every leader eventually commits").
3. **Run TLC** on a small bounded model. Counterexample traces appear in seconds-to-hours.
4. **Iterate the spec** until invariants hold under TLC's bounds. Bug found here is a *design bug*, not a code bug.
5. **Optionally run Apalache** if state-space size in TLC becomes infeasible or you want a symbolic proof of an inductive invariant.

What you *don't* get: any guarantee that the implementation matches the spec. The spec→code gap is bridged by code review, property-based tests, and (at AWS) increasingly by [Kani](kani.md) and [Loom](loom.md) at the implementation layer.

## Tooling status

- **TLA+ Toolbox** (Eclipse-based IDE) is *unmaintained* per the repository README; the recommended IDE is the VS Code extension [tlaplus/vscode-tlaplus](https://github.com/tlaplus/vscode-tlaplus). The Toolbox is still functional but not receiving new development.
- The **TLA+ Foundation** was announced 2023-04-21 under the Linux Foundation umbrella, with founding members AWS, Oracle, and Microsoft. Lamport is "distinguished scientist with Microsoft Research" and contributes but the Foundation now stewards day-to-day. This is governance-positive — TLA+ has a clear long-term home, which Coq/Rocq notably lacked until late 2024.
- **Learn TLA+** ([learntla.com](https://learntla.com/)) by Hillel Wayne is the canonical free tutorial, superseding his earlier *Practical TLA+* book (Apress 2018). Wayne is the most visible TLA+ educator outside the AWS team and effectively the project's marketing arm. Approximately 1,000 unique visitors a week per Wayne's own framing — small but disproportionately influential.

## Strengths

- **Sweet spot is concurrent/distributed-system design bugs.** AWS reports finding bugs in DynamoDB, S3 GC, EBS volume placement, and the IAM authorization stack — bugs that would have been caught only in extreme operational conditions, often after deployment.
- **Counterexample-driven debugging.** When TLC reports a violation it shows the exact sequence of state transitions, which is far more actionable than a stack trace from a production outage.
- **Forces the design conversation early.** AWS's CACM 2015 paper makes the social point explicitly: writing a TLA+ spec exposes assumptions that handwaving in design docs hides.
- **Cheap on a per-protocol basis.** A typical state machine is 50–300 lines of TLA+. The CACM 2015 paper says engineers became productive in 2–3 weeks.
- **Stable, mature toolchain.** TLA+ is old enough (Lamport's 1999 paper) that it doesn't churn under your spec.

## Limitations

- **You don't verify the implementation.** This is the load-bearing caveat. A TLA+ spec models what you *want* to be true. The Rust/C++/Java code that implements it is a separate artifact and can diverge silently. See [`open-problems.md`](open-problems.md).
- **State-space explosion is the constant enemy.** Any spec with non-trivial concurrency requires careful bounding (small sets, small queue depths, fewer concurrent actors) to keep TLC feasible. You learn to write specs *for* the model checker, not for elegance.
- **TLA+ syntax is mathematical.** Most engineers find it harder than learning a new programming language — there's no compile-and-run loop, no IDE-grade autocomplete (the VS Code extension helps but isn't language-server-grade), no print-debugging. PlusCal and Quint reduce this but don't eliminate it.
- **Liveness is harder than safety.** Liveness properties (`<>P` — "eventually P") require fairness assumptions and are vastly more expensive to check than safety. Most real specs check only safety in practice.
- **Apalache's TLA+ subset.** Apalache rejects specs that use unhinted dynamic constructs — you often have to retrofit type annotations to specs originally written for TLC.

## Implications for Myrhiza

A few specific load-bearing state machines are TLA+-shaped:

- **`state-apply` ordering and convergence.** Given an event log + a `state-apply` component, do two peers seeing different orderings of the same events converge? This is the canonical TLA+ question. TLC on a 3-peer, 5-event bounded model would find ordering bugs in hours, not weeks.
- **Capability-token check.** The lifecycle of a capability (mint → delegate → use → revoke) is a state machine with safety invariants ("no use after revoke", "no delegation past the original's expiry"). PlusCal-shaped spec, TLC-checked.
- **Component-link integrity.** Wire-up of `state-apply` / `state-propose` / `interaction` / `behavior` components — what compositions are well-formed, what invariants hold across boundaries.
- **The wire (gossip + replication).** When the runtime ships a gossip or replication protocol, that's the same shape as Raft / Paxos / MongoDB-Raft / etc., and the same TLA+ treatment that produced bug-finds at MongoDB applies. See [`adoption.md`](adoption.md).

What *not* to put in TLA+: anything below the protocol layer. The Rust async runtime, the WASM-host boundary, the actual serialization — those are [Loom](loom.md) and [Kani](kani.md) territory.

Concrete first-spec recommendation: write **one** TLA+ spec of the `state-apply` ordering before the runtime ships. 100–200 lines of PlusCal. Check it under TLC. If TLC's counterexamples are illuminating, write a second spec of the capability lifecycle. If they aren't, that's signal — either the design is simpler than feared or the spec is wrong.

## Sources

- TLA+ Tools (TLC, PlusCal, Toolbox): https://github.com/tlaplus/tlaplus — MIT, v1.8.0 "Clarke release" 2026-05-18, Java/TLA+
- Apalache: https://github.com/apalache-mc/apalache — Apache-2.0, v0.57.0 2026-04-24, Scala
- Apalache home: https://apalache-mc.org/
- Quint: https://github.com/informalsystems/quint — Apache-2.0, Quint→TLA+ compiler
- Quint blog "Why I use TLA+ and not(TLA+)": https://protocols-made-fun.com/specification/modelchecking/tlaplus/quint/2024/10/05/tla-and-not-tla.html
- Lamport's TLA+ home page: https://lamport.azurewebsites.net/tla/tla.html
- TLA+ Foundation launch (Linux Foundation, 2023-04-21): https://www.linuxfoundation.org/press/linux-foundation-launches-tlafoundation
- Learn TLA+ (Hillel Wayne): https://learntla.com/
- Wayne's Practical TLA+ (Apress 2018): https://www.hillelwayne.com/post/practical-tla/
- TLA+ VS Code extension: https://github.com/tlaplus/vscode-tlaplus
- "How Amazon Web Services Uses Formal Methods", Newcombe et al., CACM 58(4):66–73, 2015: https://cacm.acm.org/research/how-amazon-web-services-uses-formal-methods/
- "Systems Correctness Practices at AWS", ACM Queue/CACM 2024–2025: https://queue.acm.org/detail.cfm?id=3712057
- MongoDB Raft TLA+ spec: https://github.com/mongodb/mongo/blob/master/src/mongo/tla_plus/Replication/RaftMongo/RaftMongo.tla
- CockroachDB Parallel Commits blog (TLA+ verified): https://www.cockroachlabs.com/blog/parallel-commits/
