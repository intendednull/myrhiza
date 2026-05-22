**Date:** 2026-05-22
**Status:** active
**Subject:** BEAM scheduler, process abstraction, message passing, supervision tree mechanics

# Architecture

## Process abstraction

BEAM's "process" is **not** an OS process or thread. It is a lightweight cooperative-scheduled coroutine with its own heap, its own stack, its own garbage collector, and a single inbox mailbox. Spawn cost: ~2 KB heap at minimum, ~10 µs creation time, no kernel involvement.

A modern BEAM node routinely runs hundreds of thousands of processes. WhatsApp reportedly ran 2M+ concurrent processes per node in the mid-2010s. Discord's chat-presence service shards similarly.

**Key invariants:**

- **No shared state between processes.** All inter-process communication is by copying messages over mailboxes. There is no `Mutex<T>` because there is no `T` shared across processes.
- **Per-process heap and GC.** Each process collects independently. A long GC in process A does not stall process B. This is the load-bearing property that makes "millions of independent latency budgets" tractable.
- **Mailbox is unbounded by default.** This is a real footgun — see [`critiques.md`](critiques.md). Backpressure is the application's responsibility.
- **`Pid` is unforgeable inside a node** but trivially forgeable across the wire by anyone who knows the cookie. The ocap discipline that Spritely/Agoric have is *not* part of BEAM's threat model. See [`distribution.md`](distribution.md) for the consequences.

## Scheduler

BEAM ships with one **scheduler thread per online CPU core by default**. Each scheduler owns a run queue and runs processes cooperatively, switching after a **reduction count** budget (one reduction ≈ one function call or one BIF call; the budget is typically 2000 reductions per slice). This is preemptive at the scheduler level but cooperative-feeling at the language level because the BEAM bytecode interpreter / JIT inserts reduction checkpoints between operations.

Schedulers can **migrate processes** between run queues for load balancing. The migration logic is hand-tuned and has been the subject of multi-year evolution — Lukas Larsson and the OTP team have published several papers on it.

**Dirty schedulers** (OTP 17, 2014; production-default since OTP 20) are a parallel set of OS-thread schedulers for blocking I/O and CPU-bound NIFs. Splitting "normal" and "dirty" was the team's compromise after years of NIF-blocking-the-scheduler bugs.

**What BEAM does not do:** shared-memory parallelism *within* a single process. A single process is single-threaded by definition. Parallelism is achieved by spawning many processes; there is no fork-join inside one. (Recent work on shared heap experiments — "atomic heaps" — has not landed; the per-process-heap model is canonical.)

**Implications for Myrhiza:** WASM Component Model components are individually single-threaded (no Wasm threads in CM today, and even if they land they are opt-in). Component-per-actor maps cleanly onto the BEAM-process abstraction. **But** the Myrhiza host can multiplex many components onto OS-level worker threads similarly to BEAM scheduler threads; this is the borrowable pattern. See [`lessons.md`](lessons.md) for the explicit borrow.

## Message passing

The primitives are three operators:

- `Pid ! Msg` — send. Non-blocking, fire-and-forget. Returns `Msg`. No delivery guarantee on the wire layer; in-node send is reliable.
- `receive ... end` — selective receive. Pattern-matches messages out of the inbox in arrival order; non-matching messages remain.
- Process links and monitors (see below).

**Selective receive** is the most distinctive feature. A process can pull a specific message shape out of its mailbox without consuming earlier ones. This makes RPC-over-messages tractable: send a request tagged with a unique reference, then `receive {Ref, Reply} -> Reply end`. The mailbox's other contents are unaffected.

**Cost of selective receive:** O(N) scan over the mailbox per receive in the worst case. A flooded mailbox + a selective receive pattern that rarely matches is a classic O(N²) BEAM hang. OTP 19 added the `receive ... after 0` optimisation and selective-receive marker hints to mitigate, but the worst case persists. Production code learns to drain mailboxes or use bounded mailbox patterns.

## Links and monitors

Two primitives for process-level failure detection:

- **Link** (`link/1`, `spawn_link/1`) — bidirectional. If either linked process dies, the other receives an `EXIT` signal (and dies in turn unless trapping exits via `process_flag(trap_exit, true)`).
- **Monitor** (`monitor/2`) — unidirectional, one-shot. Monitoring process receives a `DOWN` message when the target dies. Cheaper than links for "I want to know when X dies but not vice versa."

These are the substrate of supervision. A supervisor uses links + `trap_exit` to be notified when children die and to restart them per its strategy.

## Supervision tree

A **supervisor** is a process whose only job is to start, monitor, and restart its children. Children can be worker processes or other supervisors, yielding a tree.

**Restart strategies** (set at the supervisor):

- `one_for_one` — if child dies, restart just that child. Default; most common.
- `one_for_all` — if any child dies, kill all siblings and restart the entire group together. For tightly-coupled groups where one dying invalidates the rest.
- `rest_for_one` — if child N dies, kill children N+1..end (those started after it), then restart N and the rest. For pipeline dependencies.
- `simple_one_for_one` — for "spawn N transient workers" — children all start from the same `child_spec`, added dynamically via `supervisor:start_child/2`. Still supported in current OTP, though Elixir's `DynamicSupervisor` is the ergonomic successor for new Elixir code; not deprecated in Erlang itself.

**Restart intensity** is a `(MaxRestarts, MaxSeconds)` tuple. If the supervisor restarts children more than `MaxRestarts` times within `MaxSeconds`, the supervisor itself terminates and propagates the failure upward. This is the "let it crash, but stop crashing if you crash too often" backstop.

**Child specs** declare restart policy per child:

- `permanent` — always restart.
- `transient` — restart only if termination was abnormal (i.e. not `normal`/`shutdown`).
- `temporary` — never restart; clean up on death.

**Tree shape lesson:** OTP design idiom is "supervisors do supervision, workers do work; never combine." A supervisor that also handles work creates a supervision loop where the failure isolation degrades. Joe Armstrong's *Programming Erlang* book makes this rule explicit and is widely cited.

**Implications for Myrhiza:** Myrhiza apps will be a small set of long-lived components (state-apply, state-propose, interaction, behavior per `CLAUDE.md`); the OTP supervision-tree shape — separating "the thing that watches" from "the thing that works" — is directly applicable as a kernel pattern. A kernel-level supervisor that restarts a `state-apply` component when its WASM trap-count exceeds a threshold is the obvious analog. See [`lessons.md`](lessons.md) "Borrow."

## Failure model — "let it crash"

The cultural slogan, but the precise meaning matters:

- **Defensive programming is discouraged at the worker level.** Workers assume their inputs are well-formed; if they aren't, the process crashes, the supervisor restarts it with a known-clean initial state, and life goes on.
- **Defensive programming is required at the system boundary.** Network parsers, JSON decoders, file parsers — these crash on malformed input but should crash *cleanly* and not leak file descriptors / sockets.
- **Crash log + restart > runtime patching.** "If you didn't think of this edge case in advance, your code shouldn't tiptoe around it — it should crash and you should fix it for next time."

The unflattering version of this slogan (which the corpus surfaces honestly in [`critiques.md`](critiques.md)): for systems where state survives crashes (e.g. anything with a database), "let it crash" is not a free pass — it's a contract that the persistent state has its own consistency story. Mnesia + ETS + per-process state interact in non-trivial ways under crash-restart. See [`storage.md`](storage.md).

## Sources

- BEAM scheduler internals: <https://www.erlang.org/doc/apps/erts/beamasm.html>
- Supervisor behaviour: <https://www.erlang.org/doc/system/sup_princ.html>
- *Programming Erlang*, 2nd ed., Joe Armstrong (Pragmatic Bookshelf, 2013)
- *Designing for Scalability with Erlang/OTP*, Cesarini & Vinoski (O'Reilly, 2016)
- Dirty schedulers paper: <https://www.erlang.org/blog/dirty-schedulers/>
- Selective receive optimisation: <https://www.erlang.org/blog/recv-mark/>
