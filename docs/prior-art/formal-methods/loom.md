**Date:** 2026-05-22
**Status:** active
**Subject:** Loom and AWS Shuttle — Rust concurrency-bug detection via interleaving exploration

# Loom (and Shuttle)

[Loom](https://github.com/tokio-rs/loom) is a Rust concurrency permutation tester. You write a test that uses Loom's drop-in replacements for `std::sync::{Mutex, Arc, atomic::*}` and `std::thread`; Loom runs that test many times, permuting the possible interleavings of memory operations *under the C11 memory model*, and reports the first interleaving that triggers a failure. The C11 model is the same one Rust inherits, so behaviors Loom permits are behaviors a real Rust execution could exhibit.

[Shuttle](https://github.com/awslabs/shuttle) is an AWS Labs library in the same shape but a different strategy: instead of exhaustive interleaving exploration, it does *randomized* scheduling with bug-finding heuristics (random schedules + the **PCT — Probabilistic Concurrency Testing** algorithm). Shuttle's authors describe it as "heavily inspired by Loom" but trading soundness for scalability — Loom proves no bug exists in the bounded model, Shuttle just makes the bug very likely to surface within a budget.

Together they cover the realistic Rust-concurrency-bug-detection spectrum: Loom for small primitives where exhaustive exploration is feasible, Shuttle for larger systems where it isn't.

## What Loom does

Loom is a state-space explorer specialized for the C11 memory model. Given a test like:

```rust
#[test]
fn concurrent_counter() {
    loom::model(|| {
        let counter = loom::sync::Arc::new(loom::sync::atomic::AtomicUsize::new(0));
        let c1 = counter.clone();
        let h1 = loom::thread::spawn(move || c1.fetch_add(1, Ordering::SeqCst));
        let c2 = counter.clone();
        let h2 = loom::thread::spawn(move || c2.fetch_add(1, Ordering::SeqCst));
        h1.join().unwrap();
        h2.join().unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    });
}
```

Loom enumerates the legal interleavings of the two threads' atomic operations under C11 and runs the test body once per interleaving. If any interleaving fails the assertion, Loom reports the schedule. The model is *bounded* — Loom is not a model checker over arbitrary programs, it's a model checker over the operations its instrumented primitives expose.

Loom is **an implementation of techniques from the [CDSChecker paper](http://plrg.ics.uci.edu/cdschecker/) — Norris & Demsky, "CDSChecker: Checking Concurrent Data Structures Written with C/C++ Atomics" (OOPSLA 2013)**, adapted to Rust. Recent versions use **DPOR (Dynamic Partial Order Reduction)** to avoid exploring redundant interleavings — without DPOR the state space blows up combinatorially even for small tests.

## Loom's tradeoffs

**Strengths.**

- **Cheap to adopt.** `loom = "0.7"` in `[dev-dependencies]`, wrap your test in `loom::model(|| { ... })`, replace `std::sync::Foo` with `loom::sync::Foo` behind a `cfg(loom)` flag. The Tokio runtime uses this pattern throughout — see [tokio-rs/tokio](https://github.com/tokio-rs/tokio) for the canonical idiom.
- **Catches real bugs.** Loom has found bugs in Tokio's synchronization primitives that survived production for months. The bug shape is "a particular interleaving violates a safety invariant that holds for *most* interleavings" — exactly the bug shape that escapes randomized stress-testing.
- **Reproducible counterexamples.** Loom records the schedule that triggered the failure and replays it deterministically, so debugging is sane.
- **No annotation overhead.** Unlike Kani, you don't write invariant contracts. You write assertions, and Loom finds the schedule that violates them.

**Limitations.**

- **C11 model is incomplete.** Loom's own README acknowledges it doesn't fully implement C11 — specifically, known limitations around SeqCst and load-buffering behavior. Loom can find *some* memory-ordering bugs, not all of them.
- **State space is the bottleneck.** A `loom::model` test with three threads and a few atomics is fine; ten threads with cell mutations is intractable. Test design matters — you write tests that exercise *one* concurrency hazard at a time.
- **Loom is not async-aware out of the box.** Loom permutes thread schedules but doesn't model `tokio::spawn` natively. Tokio's own Loom tests construct a custom executor under Loom; this pattern is reusable but not trivial.
- **Slow release cadence.** Loom 0.7.2 (2024-04-23) is the current stable on crates.io; the GitHub repo has post-0.7.2 work but no published 0.8 series. For a "use it now" library this matters less than for a "build hard against the API" one.

## Shuttle

Shuttle ([`awslabs/shuttle`](https://github.com/awslabs/shuttle), Apache-2.0, **0.9.1** on crates.io 2026-04-21; v0.9.0 GitHub tag 2026-04-20) is the alternative when Loom's exhaustive exploration doesn't scale.

Shuttle's API is shaped almost identically to Loom's — drop-in `shuttle::sync` / `shuttle::thread` modules, wrap the test body in `shuttle::check_random` or `shuttle::check_pct`. The key differences:

- **Schedules are randomized, not exhaustive.** A passing Shuttle test does *not* prove the code is correct — it proves the code survived N randomized schedules. Standard practice: `check_pct(test_body, num_iterations=10_000, max_depth=...)`.
- **PCT (Probabilistic Concurrency Testing)** is Shuttle's hero algorithm. Burckhardt et al.'s 2010 ASPLOS paper "[A Randomized Scheduler with Probabilistic Guarantees of Finding Bugs](https://dl.acm.org/doi/10.1145/1736020.1736040)" — it gives a *lower-bound probability* of finding a bug of a given depth in a given number of iterations. Empirically, most concurrency bugs require very few preemptions; PCT finds those quickly.
- **Scales to larger test cases.** Shuttle has wrappers for `tokio`-style tests and is happy with hundreds of tasks; Loom would explode.
- **AWS uses it in production.** Shuttle was developed at AWS for testing AWS-internal Rust libraries — see Grant Slatton's blog [`grantslatton.com/shuttle`](https://grantslatton.com/shuttle) for the story.

**When to use which:**

| Situation | Use |
|---|---|
| Single concurrency primitive (lock, channel, atomic counter); want a *proof* under the model | Loom |
| Larger system (multi-actor async, many tasks); want bug-finding | Shuttle |
| Bug suspected, want a reproducible counterexample fast | Both — Shuttle finds it, Loom proves no further bug |
| New Rust codebase, "I want concurrency tests on day one" | Loom first; Shuttle when Loom blows up |

The Tokio team uses Loom. AWS internal teams predominantly use Shuttle. The two are not adversarial — many real codebases adopt both.

## What Loom/Shuttle don't do

Both tools test *Rust code* against *the memory model*. They don't:

- Verify the code matches a higher-level specification (TLA+ territory).
- Find logic bugs that don't depend on interleaving — a wrong constant, an off-by-one, a missing case. Standard tests find those.
- Check WASM-guest code. Loom/Shuttle run on the host runtime. A WASM component's internal concurrency, if any, is outside their scope.
- Model OS-level scheduling pathologies (priority inversion, real-time deadline misses). They model the abstract memory model, not the scheduler.
- Catch bugs that require more interleaving permutations than the search budget. Loom's bounds, Shuttle's randomized budget — both can miss bugs that only appear after astronomical numbers of preemptions.

## Implications for Myrhiza

Any non-trivial concurrent Rust in the kernel — broker queues, capability-table mutation, the event log's write path, the gossip task's interaction with the persistence layer — is Loom-shaped. The cost of adoption is essentially zero per test (one dev-dependency, a `cfg(loom)` wrap), and the bug-finding leverage is high.

A reasonable per-component policy: **every kernel primitive that exposes a `pub` mutable surface gets a Loom test of its critical invariants.** Capability-table reads/writes, broker queue enqueue/dequeue under contention, persistence-log append ordering, gossip-deliver-then-state-apply sequencing. These are the spots production bugs hide in P2P runtimes.

For larger end-to-end async scenarios (multi-peer integration tests), Shuttle is the right shape. Adopt it lazily — only when a Loom test you wanted to write turned out to be too large.

**Don't** try to Loom-test the entire runtime. Loom is a per-primitive tool. The TLA+ spec covers the cross-component story.

## Sources

- Loom: https://github.com/tokio-rs/loom — MIT, 0.7.2 (2024-04-23 crates.io stable), docs https://docs.rs/loom
- Loom crates.io: https://crates.io/crates/loom
- CDSChecker paper (Norris & Demsky, OOPSLA 2013): http://plrg.ics.uci.edu/cdschecker/
- Loom DPOR reference (Aronis et al., TOPLAS 2016): http://plrg.eecs.uci.edu/publications/toplas16.pdf
- Shuttle: https://github.com/awslabs/shuttle — Apache-2.0, 0.9.1 (2026-04-21 crates.io)
- Shuttle crates.io: https://crates.io/crates/shuttle
- Shuttle docs: https://docs.rs/shuttle
- PCT paper (Burckhardt et al., ASPLOS 2010): https://dl.acm.org/doi/10.1145/1736020.1736040
- Grant Slatton on Shuttle: https://grantslatton.com/shuttle
- Tokio Loom usage idiom: https://github.com/tokio-rs/tokio (search for `cfg(loom)`)
- RustMC (extending GenMC to Rust, 2025): https://arxiv.org/abs/2502.06293 — adjacent research-grade work
