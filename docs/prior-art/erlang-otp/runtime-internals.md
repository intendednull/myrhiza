**Date:** 2026-05-22
**Status:** active
**Subject:** BeamAsm JIT, scheduler binding, NIFs, dirty schedulers, reduction counting, per-process GC

# Runtime internals

The implementation details that determine BEAM's real-world performance envelope. Skim if you just want patterns; read carefully if you are evaluating BEAM as a substrate.

## BeamAsm — the JIT

**Shipped in OTP 24** (released 2021-05-12) on x86-64. AArch64 support landed in OTP 24.2 (December 2021), making Apple Silicon and AWS Graviton first-class. Both architectures are production-default and have been since 2022.

**Design:** load-time conversion of BEAM bytecode to native machine code, using the **asmjit** library. Not a tracing JIT, not a tiered JIT — a single-pass ahead-of-execution compile that happens when each module is loaded. No warmup, no deoptimisation, predictable execution after load.

**Authors:** Lukas Larsson (Erlang Solutions) proposed the original PR (otp#2745, 2020-09); joined by the OTP core team at Ericsson (notably John Högberg / `jhogberg`) for the production landing. The AArch64 port (PR otp#4869) was Högberg-led.

**Performance impact:**

- ~50% improvement on the estone benchmark suite (OTP team's number).
- ~30–50% throughput gain on RabbitMQ (RabbitMQ team's number).
- Numerical workloads see less benefit (BEAM remains slow at floating-point and tight integer loops because of tagged-pointer overhead and boxed bignums).

**What BeamAsm did NOT change:**

- No NIF-style C extension story improvements. Calling C from BEAM remains the only way to do truly fast numerics.
- No shared-memory-parallelism inside a process. The per-process-heap model is preserved; BeamAsm compiles individual processes' execution but does not parallelise within one.
- No reduction-counting changes. Cooperative scheduling boundaries are identical.

**Implications for Myrhiza:** Wasmtime's Cranelift JIT is structurally similar — load-time compile, no warmup, predictable. The BEAM team's experience that "load-time JIT is enough; tracing JIT not worth the complexity" is encouraging precedent for the WASM-runtime side.

## Scheduler internals

One scheduler thread per online CPU (default; tunable via `+S` flag). Each scheduler owns:

- A **run queue** of runnable processes, multi-level (4 priorities: max/high/normal/low; normal is default).
- A **migration queue** for load-balancing work to other schedulers.
- A **timer wheel** for `receive after` and `erlang:send_after`.

Process is allocated 2000 **reductions** per slice (one reduction ≈ one function call). When budget hits zero, the scheduler descheduler the process and picks another. This is the load-balancing-mechanic Erlang gives you for free — no `await`, no `yield`, no manual cooperation.

**Scheduler busy-wait** is a real cost: BEAM schedulers historically spin briefly when out of work, to avoid OS-thread parking latency on the next message. Tunable via `+sbwt` and `+sbwtdcpu`. On idle nodes with low background work, this used to consume measurable CPU. OTP 23+ tuned the defaults down; OTP 27+ tuned them further.

**Bound vs. unbound schedulers:** by default schedulers are not CPU-pinned. The `+sbt` flag enables pinning (e.g. `+sbt db` for "default bind"). Production deployments with NUMA-aware tuning sometimes pin; most do not.

## Dirty schedulers

**Shipped production-default in OTP 20** (2017). Two extra scheduler pools:

- **Dirty CPU schedulers** — for long-running pure NIFs (e.g. cryptographic operations on large blobs).
- **Dirty I/O schedulers** — for blocking I/O NIFs.

Default count: equal to online CPUs for each pool. So a 16-core host has 16 normal + 16 dirty-CPU + 10 dirty-I/O schedulers (the dirty-I/O default is `min(10, schedulers)`).

**Why this exists:** before dirty schedulers, a single misbehaving NIF that ran for >1 ms could stall the entire BEAM. NIFs are supposed to yield voluntarily by chunking work, but in practice many didn't. The dirty pool gives the runtime a containment story without rewriting every NIF.

## NIFs — Native Implemented Functions

C code linked into the BEAM runtime, exposing function-shaped exports callable from Erlang. The escape hatch for performance-critical code or library bindings.

**Footguns:**

- **NIFs crash the entire BEAM.** A segfault in a NIF is a node-down event; there is no per-process isolation from C code.
- **NIFs must yield reductions** if they run longer than ~1 ms, or move to dirty schedulers. Failing to yield is a scheduler-collapse bug.
- **NIFs have no memory isolation.** A buggy NIF can corrupt any process's heap.

**The community pattern that emerged: Rustler.** A Rust binding generator for NIFs that uses Rust's type system to prevent the worst NIF crash modes. Originally by Hansihe (Discord engineer). Discord uses it heavily; the broader Elixir community has standardised on it. Rustler is roughly to NIFs what `rusty_v8` is to V8: a memory-safe API over an unsafe C ABI.

**Alternatives to NIFs:**

- **Ports** — child OS process talking to BEAM over stdin/stdout pipes. Crash-isolated; slow.
- **Port drivers** — C code linked into BEAM but with stricter API for safety. Mostly deprecated; superseded by NIFs.
- **`jinterface`** — Java nodes that look like Erlang nodes to a cluster. Very few production deployments.

## Garbage collection

**Per-process, generational, copying.** Each process has its own heap with two generations. GC runs only on the process being collected; other processes are unaffected.

**Properties:**

- **No stop-the-world.** No global GC pause. Predictable per-process latency.
- **Garbage is process-local.** A "big garbage" process doesn't affect siblings.
- **Reference counting for binaries >64 bytes.** Large binaries live off-heap in a refcounted store. This is the source of two classic BEAM gotchas:
  - **Binary leaks** — a long-lived process holding a subbinary reference into a large binary can pin megabytes after the rest of the consumers are gone. Tools like `recon_alloc` exist for diagnosis.
  - **`erlang:garbage_collect/0` is sometimes needed manually** to reclaim refcounted binaries when the holding process is otherwise idle.

**Heap-size tuning:** `min_heap_size` and `min_bin_vheap_size` per-process. Most apps leave defaults; high-throughput apps tune per-process-type.

## Process limits

- **Default max processes per node: 262,144** (2^18). Settable up to 2^27 via `+P` flag.
- **Default max atoms per node: 1,048,576**. Atoms are NOT garbage collected; creating atoms dynamically from user input is a classic DoS bug. `binary_to_atom` with untrusted input is the canonical mistake.

## Tracing and observability

**Built-in tracing primitive (`erlang:trace/3`)** lets a tracer process subscribe to any other process's function calls, sends, receives, GCs, etc. Without restart, without recompile. This is BEAM's killer ops feature.

The tooling ecosystem:

- **`observer`** (in-tree) — a wxWidgets GUI for live node inspection. Aging but functional.
- **`recon`** by Fred Hébert — production-safe diagnostic library. Universally used. Books named *Erlang in Anger* are written about its use cases.
- **`redbug`** — safer-by-default tracing wrapper.
- **`opentelemetry-erlang`** — modern OTel integration. Maintained by the EEF Observability WG.

**Implications for Myrhiza:** WASM has nothing equivalent. The closest is Wasmtime's debug-info hooks and component-model tracing-via-interfaces (the `wasi:observability/0.2.0` proposal). The "live attach a tracer to a running process, get function-call events out, no recompile" affordance is BEAM-unique among modern runtimes. If Myrhiza ever wants comparable ops ergonomics, that is a multi-year build, not a switch to flip.

## Sources

- BeamAsm docs: <https://www.erlang.org/doc/apps/erts/beamasm.html>
- BeamAsm blog post (Erlang team, 2020-09): <https://blog.erlang.org/a-first-look-at-the-jit/>
- BeamAsm initial PR: <https://github.com/erlang/otp/pull/2745>
- AArch64 BeamAsm PR: <https://github.com/erlang/otp/pull/4869>
- Performance testing the JIT (Erlang Solutions): <https://www.erlang-solutions.com/blog/performance-testing-the-jit-compiler-for-the-beam-vm/>
- NIF docs (OTP 29): <https://www.erlang.org/doc/apps/erts/erl_nif.html>
- Rustler: <https://github.com/rusterlium/rustler>
- *Erlang in Anger* (Fred Hébert): <https://www.erlang-in-anger.com/>
- recon: <https://github.com/ferd/recon>
