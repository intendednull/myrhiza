**Date:** 2026-05-22
**Status:** active
**Subject:** Real-world adoption — who uses what, with concrete deployments and honest scale framing

# Adoption

Formal methods are widely *admired* and narrowly *used*. The gap between "we have heard of TLA+" and "we run TLA+ on a critical protocol" is large. This file catalogues the deployments that actually shipped, and tries to be honest about scale and pervasiveness — most teams who claim "we use TLA+" use it for one or two protocols, not pervasively.

## TLA+

### AWS — the canonical adoption story

AWS is the most documented production user of TLA+. The two load-bearing artifacts are:

- **["How Amazon Web Services Uses Formal Methods"](https://cacm.acm.org/research/how-amazon-web-services-uses-formal-methods/)** — Newcombe, Rath, Zhang, Munteanu, Brooker, Deardeuff, *Communications of the ACM* 58(4):66–73, April 2015. Reports use since 2011. Bug-finds discussed:
  - **DynamoDB** — replication protocol bug found post-deployment.
  - **EBS** — volume placement and replication.
  - **S3** — garbage collection, then later strong-consistency (post-2015; the 2020 strong-consistency rollout used the **P language**, not pure TLA+).
  - **Internal lock manager** — bug found that "would have remained latent in production indefinitely."
  - Engineers became productive in **2–3 weeks**, per the paper.
- **["Systems Correctness Practices at Amazon Web Services"](https://queue.acm.org/detail.cfm?id=3712057)** — ACM Queue / *CACM* 2024–2025 follow-up. Updates the picture: AWS's correctness toolkit has expanded to include **P** (formal modeling language developed by Microsoft Research, adopted heavily at AWS for the S3 strong-consistency work and other production protocols), **Kani** (Rust bounded model checker, see [`kani.md`](kani.md)), and lightweight property-based testing alongside the original TLA+ practice.

The honest framing from the 2025 paper: AWS uses formal methods *selectively*. Not every service. The investment is reserved for protocols where the bug-find economics justify it — primarily storage and consensus systems, less so for stateless web services.

### MongoDB — Raft replication spec

MongoDB maintains TLA+ specifications of their Raft-based replication protocol in-tree: [`mongodb/mongo` → `src/mongo/tla_plus/Replication/RaftMongo/`](https://github.com/mongodb/mongo/blob/master/src/mongo/tla_plus/Replication/RaftMongo/RaftMongo.tla). Independent academic work (Schultz et al., logless reconfiguration) produced TLAPS proofs of safety properties. The MongoDB engineering blog has multiple posts on prototyping in TLA+ before code: ["Rapid Prototyping a Safe, Logless Reconfiguration Protocol for MongoDB with TLA+"](https://www.mongodb.com/company/blog/technical/rapid-prototyping-safe-logless-reconfiguration-protocol-mongodb-tla-plus).

Scope: the *replication* protocol is specified. The query planner, the storage engine, the rest of the system — not.

### CockroachDB — Parallel Commits

CockroachDB used TLA+ to design and verify their **Parallel Commits** protocol — a 1-RTT distributed commit optimization. [Cockroach Labs blog post](https://www.cockroachlabs.com/blog/parallel-commits/) describes the spec; PRs in the codebase (e.g. [#73543](https://github.com/cockroachdb/cockroach/pull/73543)) add TLA+ specs of the transaction layer including pipeline writes and MVCC storage interactions.

Scope: one protocol (parallel commit), with adjacent specs of the transaction layer. The bulk of CockroachDB is not TLA+-specified.

### Microsoft Azure

Used TLA+ for Cosmos DB's consistency models per Lamport's site and adjacent talks. Less publicly documented than AWS, but Microsoft is a TLA+ Foundation inaugural member (2023). Leslie Lamport has been at Microsoft Research since 2001.

### Ethereum and beyond

The Ethereum ecosystem has multiple formal-verification efforts:

- **KEVM** — K Framework formal semantics of the EVM. Active. The K Framework is academic-grade.
- **Dafny-EVM** ([Consensys/evm-dafny](https://github.com/Consensys/evm-dafny)) — first formal & executable EVM semantics in Dafny. Cassez/Fuller/Ketabi/Pearce/Quiles, *Formal Methods 2023*. The Dafny version is *verified* (free of division-by-zero, overflow) and *executable* (an interpreter of EVM bytecode).
- **Apalache adoption in Cosmos / Tendermint** via Informal Systems — Informal Systems is both the steward of Apalache and a Tendermint/Cosmos contributor. Real Cosmos consensus components have TLA+/Apalache specs.

### Honest scale framing

TLA+ is used by perhaps a few hundred *engineering teams* worldwide. The Linux Foundation's TLA+ Foundation launch had AWS, Oracle, and Microsoft as founding members — three large companies, not three hundred. Hillel Wayne's Learn TLA+ tutorial reports approximately 1,000 unique visitors/week — a strong signal of educational interest, but small relative to the broader systems-engineering population. Most companies who *talk about* TLA+ have one engineer who built one spec and then left the company; the spec rotted.

The bug-find leverage on a single critical spec is high, but the *organizational* commitment needed for sustained TLA+ practice is the actual barrier, not the language.

## Loom

### Tokio itself

Loom's primary in-tree user is the Tokio runtime, which uses `cfg(loom)`-gated tests throughout. Search [tokio-rs/tokio](https://github.com/tokio-rs/tokio) for `cfg(loom)` to see the idiom. Tokio's synchronization primitives — `Mutex`, `RwLock`, `Notify`, `mpsc` channels, the runtime's task scheduler — all have Loom tests.

The Tokio team are the primary Loom developers, so Loom and Tokio coevolve. This is a positive feedback loop but also means Loom is *shaped by Tokio's needs*; runtimes with different concurrency idioms may find Loom's affordances less natural.

### Other Rust libraries

- **[`crossbeam`](https://github.com/crossbeam-rs/crossbeam)** — concurrent data structures (the Rust analogue to Java's `java.util.concurrent`). Uses Loom for testing.
- **[`parking_lot`](https://github.com/Amanieu/parking_lot)** — Rust's high-performance mutex/rwlock crate. Loom tests.
- **Smaller crates** — `dashmap`, `arc-swap`, etc., commonly adopt Loom for the synchronization-primitive subset of their tests.

Loom's footprint outside the Tokio sphere is real but not pervasive. A typical Rust crate that uses concurrency does *not* have Loom tests — most do `cargo test` against real OS threads and hope.

## Shuttle

AWS-internal projects predominantly. Grant Slatton's blog post ([`grantslatton.com/shuttle`](https://grantslatton.com/shuttle)) is the most public discussion of why AWS uses Shuttle over Loom — the answer being "Loom doesn't scale to the larger systems we test." Open-source adoption is smaller than Loom's; Shuttle is more "AWS-internal-library-with-public-source" than "industry-standard library."

## Kani

### AWS Firecracker

The flagship adoption story. The Firecracker team used Kani to prove properties of the security boundary between the host and untrusted guests — the **rate limiter** (where Kani found 5 bugs, including a rounding error that let guests exceed I/O bandwidth by up to 0.01%) and the **VirtIO stack** (where Kani found a bug allowing untrusted guests to set up a queue overlapping the MMIO region, crashing Firecracker on boot). Blog: [Using Kani to Validate Security Boundaries in AWS Firecracker](https://model-checking.github.io/kani-verifier-blog/2023/08/31/using-kani-to-validate-security-boundaries-in-aws-firecracker.html).

These are exactly the kind of bugs that survived "normal" testing — depth-2 logic errors in `unsafe` paths that property-based testing wouldn't construct.

### AWS S3

Per the AWS open-source blog ([How Open Source Projects are Using Kani to Write Better Software in Rust](https://aws.amazon.com/blogs/opensource/how-open-source-projects-are-using-kani-to-write-better-software-in-rust/)), parts of S3's storage layer use Kani for property verification. Less publicly documented than Firecracker.

### Other notable users

- **[s2n-quic](https://github.com/aws/s2n-quic)** — AWS's QUIC implementation. Kani proofs of select parser paths.
- **[Rust standard library](https://github.com/rust-lang/rust)** — the [Kani VeRust project](https://model-checking.github.io/verify-rust-std/) is an ongoing AWS-sponsored effort to verify pieces of `core` and `alloc`. Real properties have been verified; full-stdlib verification remains a long-running effort.

Outside AWS, Kani adoption is small. Academic groups use it, some open-source crates have experimental `cargo kani` integration, but it is not yet a mainstream Rust testing tool.

## What this implies for Myrhiza

The realistic priors:

1. **TLA+ is a budgetable, one-protocol-at-a-time investment.** Don't commit to "TLA+ everywhere." Commit to "TLA+ for `state-apply` ordering" and ship that spec. Then decide.
2. **Loom is essentially free.** Adopt it on day one for the kernel synchronization primitives.
3. **Kani is targeted, not pervasive.** Use it on parsers, decoders, capability-check functions. Don't try to verify whole subsystems.
4. **Shuttle is a fallback if Loom blows up.** Adopt it lazily.

The AWS pattern — TLA+ for protocol design, Kani for `unsafe` and parsers, Shuttle/Loom for concurrency — maps onto a Rust P2P runtime almost directly. The components are the same shape. The team-size constraint matters: AWS has hundreds of engineers across the formal-methods practice, of whom a handful are full-time. Myrhiza will have a much smaller team and should pick a *much* smaller scope.

## Sources

- AWS CACM 2015: https://cacm.acm.org/research/how-amazon-web-services-uses-formal-methods/
- AWS CACM/Queue 2024–2025: https://queue.acm.org/detail.cfm?id=3712057
- MongoDB Raft TLA+: https://github.com/mongodb/mongo/blob/master/src/mongo/tla_plus/Replication/RaftMongo/RaftMongo.tla
- MongoDB logless reconfig blog: https://www.mongodb.com/company/blog/technical/rapid-prototyping-safe-logless-reconfiguration-protocol-mongodb-tla-plus
- CockroachDB Parallel Commits: https://www.cockroachlabs.com/blog/parallel-commits/
- CockroachDB TLA+ PR: https://github.com/cockroachdb/cockroach/pull/73543
- TLA+ Foundation launch: https://www.linuxfoundation.org/press/linux-foundation-launches-tlafoundation
- KEVM (K Framework semantics of EVM): https://github.com/runtimeverification/evm-semantics
- Dafny-EVM (Cassez et al., FM 2023): https://arxiv.org/abs/2303.00152 ; repo https://github.com/Consensys/evm-dafny
- Kani Firecracker blog: https://model-checking.github.io/kani-verifier-blog/2023/08/31/using-kani-to-validate-security-boundaries-in-aws-firecracker.html
- AWS blog "How Open Source Projects are Using Kani": https://aws.amazon.com/blogs/opensource/how-open-source-projects-are-using-kani-to-write-better-software-in-rust/
- Verify Rust Std project: https://model-checking.github.io/verify-rust-std/
- Tokio Loom usage: https://github.com/tokio-rs/tokio
- Shuttle blog (Slatton): https://grantslatton.com/shuttle
- Learn TLA+: https://learntla.com/
