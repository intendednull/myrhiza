**Date:** 2026-05-22
**Status:** active
**Subject:** Honest critiques — what BEAM does poorly, what didn't survive contact with production

# Critiques

The lessons file ([`lessons.md`](lessons.md)) leans on these. Without honest critique, "borrow from OTP" becomes cargo-culting.

## RAM hunger

A BEAM process is cheap but not free. ~2 KB minimum per process, growing fast under any real load. A node with 1M processes needs ~2 GB just for process headers, before any application state. Production deployments routinely run 32–128 GB RAM nodes.

By comparison:

- Goroutines: ~2 KB minimum, similar order.
- Tokio tasks (Rust): ~64 bytes overhead minimum; orders of magnitude lighter.
- WASM components (Wasmtime): ~64 KB minimum per instance today (the engine + module store + linear memory page); higher than BEAM-process but with much stronger isolation.

The BEAM-process advantage isn't size, it's the per-process GC + supervisor-tree story. The size is acceptable when you genuinely need that story; it's overhead when you don't.

## Numerical performance

BEAM is **slow at floating-point math** — tagged pointers, boxed bignums, no SIMD without NIF. A Mandelbrot benchmark in pure Erlang runs ~50–100x slower than C. Even with BeamAsm, the speedup over the interpreter is only ~50% — large multiplier on a low base.

The community pattern: drop to NIFs for hot numerical paths. Rustler is well-trodden. Numerical packages like `nx` (Elixir's NumPy-equivalent, Valim-led) are NIF-backed via XLA/EXLA — Elixir glue, C++ math.

**Implication:** if Myrhiza's app workload includes numerical hot paths, WASM (which has SIMD, has predictable float performance, has Cranelift native codegen) is a better numerical substrate than BEAM by default.

## No shared-memory parallelism inside a process

Each BEAM process is single-threaded. To use 16 cores, you spawn 16 processes that communicate via messages. Fine for many workloads. Bad fit for workloads that *want* fork-join parallelism over a shared data structure — image processing, simulation, ML inference.

The OTP world's pattern: shard the data structure across processes (e.g. ETS with `write_concurrency`), or drop to NIFs (Rustler-style) and do fork-join in Rust/C. Neither is BEAM's strength.

**Implication:** WASM components today are also single-threaded by default; Component Model threading is on the roadmap but not landed in production. Both substrates share this limitation. Not a differentiator either way.

## Mailbox-as-unbounded queue

Process mailboxes have no built-in backpressure. A flooded producer + slow consumer = unbounded mailbox growth = OOM-kill. The community pattern is "the consumer should crash itself before the mailbox exhausts memory" via `process_flag(message_queue_data, off_heap)` + manual checks, or via libraries like `GenStage` / `Flow` that build backpressure on top.

This is a real production footgun. Most large BEAM deployments have had a "runaway mailbox" incident in their history. The Erlang in Anger book devotes a chapter to diagnosing this.

**Implication for Myrhiza:** event queues at the kernel must have bounded buffering with explicit backpressure semantics. "Use the BEAM mailbox shape" without "also include explicit backpressure" is borrowing the gun without the safety.

## The hot-code-loading reality gap

Covered in detail in [`hot-code-loading.md`](hot-code-loading.md). Short version: BEAM offers hot code loading; most production shops don't use the full `.appup`/`.relup` story; they use rolling restarts. **The marquee feature that distinguishes BEAM from every other runtime is the one most users have decided isn't worth the complexity.**

This is the most important "lift carefully" warning in the corpus. Don't sell Myrhiza on hot-reload v2 as a flagship feature; it didn't survive that role for OTP.

## Distribution protocol's trust model

Covered in [`distribution.md`](distribution.md). Default cookie auth + cleartext + full-trust-on-join is wrong for any open-network deployment. The official secure-coding guide says "TLS distribution required for untrusted networks" — but most production deployments lean on the network boundary instead of the protocol.

For Myrhiza this is a hard "don't borrow." Myrhiza's threat model is open internet; BEAM's distribution is for trusted networks.

## Mnesia's net-split story

Covered in [`storage.md`](storage.md). RabbitMQ — the most production-deployed Mnesia user — is migrating off Mnesia (to Khepri / Raft). When the flagship adopter walks away, the field has spoken.

## Toolchain second-class for Erlang (vs. Elixir)

Honest comparative observation: `rebar3` is functional but the discoverability, error messages, and tooling polish are notably weaker than `mix`. New BEAM developers picking up Erlang in 2026 find the ergonomic gap real.

The Erlang community's response: "use Elixir's tooling for Erlang projects" is a real pattern; `mix` can compile Erlang dependencies.

## Type system weakness

Erlang is dynamically typed. `dialyzer` (static analysis) helps but produces famously chatty output and is widely under-adopted. Elixir's situation is identical; `dialyzer` + `dialyxir` is the same conversation in Elixir tooling.

**Gleam** (Hindley-Milner-typed BEAM language) is the community's answer. Adoption is rising but small (1.0 in 2024); the typed-BEAM future is real but slow.

For comparison, WASM Component Model interfaces are statically typed via WIT. This is a real win for Myrhiza vs. BEAM at the substrate level.

## "Just spawn a process" is not a free architecture

Erlang elders sometimes promote a style where every domain concept gets its own process. In practice, this leads to:

- **Sequential bottlenecks** — one process per user means user-X-stuff is single-threaded.
- **Mailbox-shape problems** — see above.
- **Debugging surface** — `observer` showing 500K processes is not actually navigable.

Modern Elixir/Erlang style is moderate: processes for **isolation boundaries** (one per user session, one per long-lived connection), not for **every domain entity**. The shift took a decade of community learning.

**Implication:** Myrhiza's component-per-app + supervisor-tree mental model should be sparing. Components are isolation boundaries; not every domain concept needs its own component.

## Observability story has aged unevenly

`observer` (the in-tree GUI) is wxWidgets-based; works on Linux/Mac/Windows but has rough edges in 2026. `:recon` is excellent. OpenTelemetry integration (via `opentelemetry-erlang`) is good. But the surface is fragmented: there is no single "BEAM Datadog dashboard" experience, and the "use `observer`" advice in older tutorials no longer fits how most people deploy.

## Single-VM tracing scales poorly across a cluster

`erlang:trace/3` is per-node. Cross-node distributed tracing requires layering — OpenTelemetry or homegrown event routing. Not a fatal weakness but a real cost that the "tracing is free!" marketing elides.

## No native async-await

OTP processes ARE the async story. There is no `async`/`await` syntax. The community pattern is `spawn` + `monitor` + selective `receive`. This is fine for OTP-shaped code; it is friction when porting designs from `async`-aware languages (Rust, JS, Python) where the developer expects function-coloured concurrency.

## Footnote: third-party "BEAM is dying" critiques

A small but persistent online camp argues BEAM is being eclipsed by Go (similar concurrency model, much faster numerics, faster tooling, vastly bigger ecosystem). This is partially accurate as a *new-startup* trend (more new startups in 2026 pick Go than Elixir/Erlang) and inaccurate as a *production-decline* trend (no major BEAM shop is migrating off). The honest read is that BEAM has stabilised into a domain-specific tool — chat/messaging/realtime — while Go owns "general backend services."

## Sources

- *Erlang in Anger* (Fred Hébert): <https://www.erlang-in-anger.com/>
- Mailbox / backpressure patterns (Sasa Juric blog): <https://www.theerlangelist.com/article/spawn_or_not>
- RabbitMQ Khepri migration: <https://www.rabbitmq.com/blog/2022/05/17/rabbitmq-mnesia-migration>
- Dialyzer: <https://www.erlang.org/doc/apps/dialyzer/dialyzer.html>
- Gleam: <https://gleam.run/>
- Numerical performance comparison: <https://www.erlang-solutions.com/blog/why-numerical-erlang-is-slow/>
- "Hot code reloading with Erlang" — production-practice critique on HN: <https://news.ycombinator.com/item?id=10669131>
- EEF Security WG distribution hardening: <https://security.erlef.org/secure_coding_and_deployment_hardening/distribution.html>
