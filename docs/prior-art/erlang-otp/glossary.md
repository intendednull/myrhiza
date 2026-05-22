**Date:** 2026-05-22
**Status:** active
**Subject:** Glossary — Erlang/OTP/BEAM-specific terms used in this corpus

# Glossary

Cross-referenced from the rest of the corpus. Most entries also link to the file where the term is used most heavily.

**actor** — In Erlang/BEAM, a lightweight, mailbox-bound concurrent unit. Used informally as a synonym for "process." Per the actor lineage (Hewitt 1973). See [`architecture.md`](architecture.md).

**ABI (Application Binary Interface)** — The shape of how compiled modules call into each other. In OTP, the gen_server/gen_statem/supervisor callback signatures are an ABI in this sense — stable across decades. See [`behaviours.md`](behaviours.md).

**`appup`** — Per-application file declaring upgrade/downgrade instructions for cross-version migration. See [`hot-code-loading.md`](hot-code-loading.md).

**asmjit** — The C++ machine-code generation library used by BeamAsm. Supports x86-64 and AArch64.

**BeamAsm** — OTP 24's load-time JIT compiler. Converts BEAM bytecode to native code at module load. See [`runtime-internals.md`](runtime-internals.md).

**BEAM** — "Bogdan/Björn Erlang Abstract Machine" — the runtime VM since 1997. Sometimes ambiguously used for the whole runtime; we use it for the bytecode interpreter + JIT layer.

**behaviour** — OTP's name for what other languages call an interface or trait. A module implements a behaviour by exporting a fixed set of callback functions. The four canonical ones: `gen_server`, `gen_statem`, `supervisor`, `application`. See [`behaviours.md`](behaviours.md).

**CSLab** — Ericsson Computer Science Laboratory. Founded 1984 by Bjarne Däcker and Mike Williams. Birthplace of Erlang. See [`history.md`](history.md).

**`code_change/3`** — The OTP behaviour callback that performs in-memory state migration during a hot code upgrade. The single most-cited identifier in Myrhiza's hot-reload research direction. See [`hot-code-loading.md`](hot-code-loading.md).

**cookie** — The 20-character shared secret used by Distributed Erlang for cluster authentication. Cleartext on the wire as challenge-response; structurally trust-anyone-who-knows-it. See [`distribution.md`](distribution.md).

**`dets`** — Disk Erlang Term Storage. The disk-backed sibling of `ets`. Mostly unloved; mostly used by Mnesia internally. See [`storage.md`](storage.md).

**dirty scheduler** — A separate scheduler pool (CPU or I/O variants) for long-running NIFs. Production-default since OTP 20. See [`runtime-internals.md`](runtime-internals.md).

**EEF** — Erlang Ecosystem Foundation. CA 501(c)(3), incorporated 2019. Vendor-neutral home for community working groups; does not own the canonical implementation (Ericsson does). See [`history.md`](history.md).

**EEP** — Erlang Enhancement Proposal. The language-evolution process. EEP 76 (priority messages, OTP 28), EEP 75 (based float literals, OTP 28), EEP 79 (native records, OTP 29 experimental). Similar to PEP / TC39 / JEP for other languages.

**EPMD** — Erlang Port Mapper Daemon. Per-host daemon (TCP 4369) that maps node names to TCP ports for Distributed Erlang. See [`distribution.md`](distribution.md).

**ERTS** — Erlang Runtime System. The C-level runtime: BEAM, schedulers, GC, kernel modules. Bundled with OTP releases. Versioned independently of OTP (OTP 29 ships ERTS 16.x).

**`ets`** — Erlang Term Storage. In-memory shared-mutable hashtables. The only place real shared memory exists across BEAM processes. See [`storage.md`](storage.md).

**gen_event** — OTP behaviour for event managers / handlers. Largely fallen out of fashion; most modern code rolls its own pub/sub or uses Phoenix.PubSub / Registry.

**gen_fsm** — Old OTP state-machine behaviour. Deprecated in OTP 20 (2017); replaced by `gen_statem`. Still in tree as legacy code.

**gen_server** — The workhorse OTP behaviour. Generic synchronous + asynchronous server with `init/1`, `handle_call/3`, `handle_cast/2`, `handle_info/2`, `terminate/2`, `code_change/3` callbacks. See [`behaviours.md`](behaviours.md).

**gen_statem** — Modern OTP state-machine behaviour. Introduced OTP 19 (2016). `code_change/4` (note arity) for state migration. See [`behaviours.md`](behaviours.md).

**`global`** — OTP module providing strongly-consistent global name registration via distributed locking. Doesn't scale past small clusters. See [`distribution.md`](distribution.md).

**hex** — The BEAM-ecosystem package manager. Lives at `hex.pm`. Used by Elixir's `mix deps` and Erlang's `rebar3`.

**JAM** — "Joe's Abstract Machine" — the original 1991 Erlang interpreter, predecessor to BEAM. Long retired.

**mailbox** — Per-process FIFO message queue. Unbounded by default; backpressure is the application's problem. See [`architecture.md`](architecture.md).

**Mnesia** — OTP's in-tree distributed transactional database. RAM + disk + transactional + replicated. Famously gnarly under net-split. See [`storage.md`](storage.md).

**NIF** — Native Implemented Function. C code linked into BEAM, exposing function-shaped exports callable from Erlang. The escape hatch for performance; a node-down risk if buggy. Modern Rust binding: Rustler. See [`runtime-internals.md`](runtime-internals.md).

**OTP** — "Open Telecom Platform" — the library set + design principles that ships alongside Erlang the language. The two are typically referred to together as "Erlang/OTP." Not a VM (BEAM is).

**`pg`** — New OTP process-group registry (OTP 23+, by WhatsApp/Meta team). Eventually consistent, set-union merge, local-first reads. Replaced `pg2`. See [`distribution.md`](distribution.md).

**`pg2`** — Old OTP process-group module. Deprecated OTP 23, **removed in OTP 24**. Don't use; cite for historical context only.

**PID** — Process identifier. Local to a node + global by extension (a remote PID embeds node name). Forgeable across the wire (cookie-trusted). Not a capability. See [`architecture.md`](architecture.md).

**preemption** — In BEAM, preemption happens at function-call boundaries via reduction-counting. Cooperative-feeling, scheduler-preemptive. See [`architecture.md`](architecture.md).

**`ra`** — Third-party Raft implementation for BEAM, maintained by the RabbitMQ team. Used by RabbitMQ's quorum queues and Khepri. See [`storage.md`](storage.md), [`distribution.md`](distribution.md).

**rebar3** — Erlang project tool: deps, build, test. The Erlang-flavoured equivalent of `mix`. See [`elixir.md`](elixir.md) for the comparative ergonomics.

**reduction** — The unit of cooperative scheduling. ~1 function call. Process gets ~2000 reductions per slice before being descheduled. See [`runtime-internals.md`](runtime-internals.md).

**`relup`** — Release upgrade script. Generated from `.appup` files via `systools:make_relup/3`. Executable plan that the `release_handler` runs to upgrade a cluster. See [`hot-code-loading.md`](hot-code-loading.md).

**`release_handler`** — The OTP module that orchestrates a release upgrade. Lives in the `sasl` application. See [`hot-code-loading.md`](hot-code-loading.md).

**Rustler** — Rust binding generator for NIFs. Authored by Hansihe (Discord). De-facto standard for new NIF code in 2026. See [`runtime-internals.md`](runtime-internals.md).

**SASL** — System Architecture Support Libraries. OTP application that hosts `release_handler`, system logger, alarm handler. (Yes, the acronym clashes with "Simple Authentication and Security Layer." It's OTP-internal.)

**selective receive** — `receive Pattern -> Action end` skipping non-matching messages. Powerful but O(N) over the mailbox in worst case. See [`architecture.md`](architecture.md).

**supervision tree** — Hierarchy of supervisors and workers. Restart strategies (`one_for_one`, `one_for_all`, `rest_for_one`) declared per-supervisor. Restart intensity `(MaxR, MaxT)` as the failure-cascade backstop. See [`architecture.md`](architecture.md).

**`syn`** — Third-party process registry (Ostinelli). Adds metadata + unique-name support on top of `pg`-style mechanics. Less central than it was pre-OTP 23. See [`distribution.md`](distribution.md).

**term** — Generic name for any Erlang value: atom, integer, binary, tuple, list, map, pid, port, reference, function. `term_to_binary/1` is the external-format serialiser used by Distributed Erlang.

**term_to_binary** — Erlang's built-in serialisation format. Used on the wire by Distributed Erlang, and as the storage format by Mnesia/ETS/DETS. Versioned by tag-extension, not by handshake. See [`distribution.md`](distribution.md).

**TLS distribution** — `-proto_dist inet_tls` mode for Distributed Erlang, adding mTLS on top of the cookie. Required for any production deployment crossing trust boundaries. Adoption spotty. See [`distribution.md`](distribution.md).

**two-version invariant** — BEAM's guarantee that at any moment, a module has at most two loaded versions (current + old). Foundation of hot-code-loading correctness. See [`hot-code-loading.md`](hot-code-loading.md).

**vat** — Not an OTP term. Used in this corpus only when comparing to Agoric SwingSet / Spritely Goblins ([`comparisons.md`](comparisons.md)). A vat is a single-threaded isolation unit with persistent state in the ocap-runtime lineage. The BEAM analog is a process; the semantics differ (vats have transcript-driven replay; processes do not).
