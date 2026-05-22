**Date:** 2026-05-22
**Status:** active
**Subject:** Erlang/OTP — design lessons for Myrhiza (validates / avoid / borrow)

# Lessons for Myrhiza

The consult-this-when-designing file. Synthesis from the rest of the corpus, framed as actionable design statements.

BEAM/OTP is the longest-running production validation of `(supervised actor tree + message-passing isolation + hot code reload + state migration callback)` in software. Four decades, multiple flagship deployments (WhatsApp, Discord, RabbitMQ, Riak), one diaspora-and-return for stewardship. The pattern survived. The pattern also has unflattering production reality that needs lifting carefully.

This file structures lessons as **Validates / Avoid / Borrow** tables. See the [framing disclosure in `README.md`](./README.md#how-to-use-this-prior-art-doc): "Validates" entries are claims about us dressed as observations about OTP — weight them with skepticism. "Avoid" and the load-bearing items in "Borrow" are the higher-leverage content.

The most important meta-lesson, surfaced repeatedly across the corpus and worth stating once at the top:

> **Hot code loading is BEAM's marquee feature and most production OTP shops have abandoned the heavy-weight version of it in favour of rolling restarts.** The mechanism (two-version invariant, `code_change/3`, `.appup`/`.relup`) is real and works; the operational cost (state-migration bugs cascading into restart loops) made it unattractive enough that even WhatsApp's current practice is reportedly mixed. **Myrhiza must internalise this before designing hot-reload v2** ([`hot-code-loading.md`](hot-code-loading.md)).

## Validates

Things OTP's four-decade production experience confirms about choices Myrhiza has already made or is leaning toward.

| Myrhiza choice | What OTP validates |
|---|---|
| **Supervised actor trees as the isolation primitive.** | BEAM supervision trees have shipped to consensus-critical telecom switches since 1998. The "supervisors do supervision, workers do work; isolation boundary is the process" pattern is the most-validated runtime pattern in production software. The shape works. |
| **Many small, isolated, message-passing components beat one big monolith.** | WhatsApp ran with single-digit engineers on hundreds of millions of users by leaning hard on isolated processes + supervised restart. The model genuinely scales human-operator-time, not just compute. |
| **Per-instance heap with no shared mutable state.** | BEAM per-process heaps + copying-message-passing have demonstrated that "no shared state across actors" is engineerable at planetary scale. Myrhiza's WASM Component Model linear-memory-per-component is the same property at the substrate level. |
| **"Let it crash" as the failure-recovery default.** | The supervisor-restart pattern lets workers be written naively (no defensive programming inside the worker; let the supervisor restart you cleanly). Reduces code complexity at the cost of accepting brief restart blips. Validates Myrhiza's "kernel restarts components that misbehave" model — though see "Avoid" for the limits. |
| **A pluggable, multi-language ecosystem on a single substrate is achievable.** | Erlang + Elixir + Gleam + LFE all target BEAM. The runtime serving multiple front-end languages is a real and durable pattern. Validates Myrhiza's WASM-Component-Model-as-substrate, multi-language-source approach. |
| **State-machine behaviours with explicit transitions are a sound design unit.** | `gen_statem`'s `(state_name, state_data)` model with state-entry callbacks is a coherent FSM contract. Validates the general shape of "state-apply is a state machine driven by events." |
| **A small, stable behaviour ABI outlives many internal rewrites.** | `code_change/3` has had a stable signature across OTP 12–29 (2008–2026). Validates the discipline of designing the kernel ABI as if it must outlive several Myrhiza generations. |
| **Built-in tracing is a load-bearing operability feature.** | `erlang:trace/3` is the single biggest reason BEAM ops culture exists. Validates Myrhiza giving the kernel a first-class "live tracing" hook from day one, even if the implementation lags. |

**Skepticism check on this section** (per the framing disclosure): every entry above is a Myrhiza decision we *want* validated. OTP's success at any of these is partial evidence, not proof. The WhatsApp/Discord production validation is the strongest item; the others are partial. Three specific items to treat with skepticism: (a) "let it crash" works when state is small or external; for stateful workloads it requires extra design we don't have an answer for yet ([`open-problems.md`](open-problems.md)). (b) The multi-language substrate validation is for BEAM-flavoured languages — Erlang-family pattern matching, message-passing semantics, supervision. WASM Component Model spans much further (C, Rust, Python, Go); we don't yet know whether *that* breadth survives the same way. (c) Stable behaviour ABIs sometimes survive because they were locked in too early to be improved — `gen_fsm` took 30 years to replace.

## Avoid

Things OTP did that did not work or should not be replicated in Myrhiza's design space.

| Anti-pattern | Why we avoid |
|---|---|
| **Cookie-based cluster authentication with full-trust-on-join.** | Default cookie auth + cleartext + RPC-anything model is for trusted private networks. Myrhiza's threat model is the open internet. **Do not borrow the wire protocol or the trust model from Distributed Erlang.** OCapN + cryptographic peer identity is the model; see [`prior-art/spritely-ocapn/captp-and-ocapn.md`](../spritely-ocapn/captp-and-ocapn.md). ([`distribution.md`](distribution.md)) |
| **Unbounded mailboxes / no built-in backpressure.** | The single most cited production footgun in the BEAM world (runaway mailbox = OOM). Myrhiza kernel queues must be bounded with explicit backpressure semantics from day one. ([`critiques.md`](critiques.md)) |
| **Hot code loading as a marquee feature.** | Most production OTP shops use rolling restart instead. Don't sell Myrhiza on hot-reload v2 as a flagship; sell it as an opt-in for the small subset of apps where its cost is justified. ([`hot-code-loading.md`](hot-code-loading.md)) |
| **`.appup` / `.relup` author-written instruction lists.** | Imperative, easy to author wrong, brittle. If Myrhiza needs cross-component upgrade coordination, ship a declarative manifest + kernel-driven planner, not a hand-authored instruction list. ([`hot-code-loading.md`](hot-code-loading.md)) |
| **NIFs / unsafe extension story.** | A buggy NIF crashes the entire BEAM node. WASM linear memory + Component Model resources is fundamentally safer; the equivalent is "use Rust + Rustler for safety," which is a community pattern, not a runtime guarantee. Myrhiza's substrate has stronger isolation built in; don't dilute it by ever shipping a NIF-equivalent escape hatch. ([`runtime-internals.md`](runtime-internals.md)) |
| **Mnesia-as-default-distributed-store.** | RabbitMQ — the most production-deployed Mnesia user — is migrating off. The "built-in distributed DB with no story for net-split" pattern is a known failure mode. Don't ship one in Myrhiza. Event log + CRDT-shaped state + per-peer storage is the cleaner shape. ([`storage.md`](storage.md)) |
| **Spawning a process per domain entity.** | The Erlang style "one process per user / per order / per object" was modish in 2014, now considered an anti-pattern. Processes are isolation boundaries, not domain modelling primitives. Myrhiza components should be the same: declare them for isolation needs, not for domain modelling. ([`critiques.md`](critiques.md)) |
| **`global` registry for production coordination.** | Strong consistency + global locks = doesn't scale past small clusters and is net-split-fragile. Myrhiza behaviour coordination should be eventually-consistent and CRDT-shaped, like `pg`, not lock-based like `global`. ([`distribution.md`](distribution.md)) |
| **Atom-creation-from-untrusted-input.** | Atoms aren't GC'd; creating them dynamically from user input is a DoS bug. Myrhiza component-handle naming should never accept untrusted strings as durable kernel-internal identifiers without normalising to a bounded space. ([`runtime-internals.md`](runtime-internals.md)) |
| **Single-corporate-steward language without foundation safety net.** | Akka's BSL relicense in 2022 was a wake-up call for actor-runtime users. OTP's Ericsson-stewardship + EEF-trademark dual structure has held up better. Myrhiza should plan its own equivalent: separate the implementation steward from the trademark / spec custody. ([`history.md`](history.md), [`comparisons.md`](comparisons.md)) |

## Borrow

Specific design choices from OTP that Myrhiza should adopt with attribution.

| Borrow | Where to apply |
|---|---|
| **Supervisor-tree shape with explicit restart strategies.** | Kernel supervises components per a declared strategy (`one_for_one` / `one_for_all` / `rest_for_one`). Restart-intensity backstop (max N restarts in M seconds → escalate). The shape is right; we just bind it to WASM components instead of BEAM processes. ([`architecture.md`](architecture.md), [`behaviours.md`](behaviours.md)) |
| **Two-version invariant for hot upgrade.** | If/when Myrhiza ships hot-reload v2, hold both `v1` and `v2` of a component simultaneously; new events go to `v2`, in-flight calls on `v1` drain, then `v1` is purged. This is the only correctness-preserving shape for live upgrade with in-flight work. ([`hot-code-loading.md`](hot-code-loading.md)) |
| **`code_change/3`-style migration callback as the upgrade contract.** | Component developer writes a pure `migrate(state_v1) -> Result<state_v2, MigrationError>` function. Kernel calls it to transform state at upgrade time. **Add what OTP didn't:** kernel pre-validates the migration in dry-run mode before activating, so a bad migration fails before going live (not after, in a restart loop). ([`hot-code-loading.md`](hot-code-loading.md), [`open-problems.md`](open-problems.md)) |
| **`gen_statem`-flavoured state-machine contract.** | For Myrhiza's `state-apply` profile: `(prior_state, event) -> (new_state, side_effects)` with explicit state-name transitions. OTP's `gen_statem` shape (state-enter callbacks, postponable events, named timers) is a good starting point; narrow it to pure functions for determinism. ([`behaviours.md`](behaviours.md)) |
| **`pg`-shape eventually-consistent group registry for behaviour coordination.** | Per-topic, per-peer-local registry of which behaviour instances are active. Set-union merge on partition heal; no global locks; local-first reads. The WhatsApp/Meta `pg` redesign is the canonical reference; lift the shape. ([`distribution.md`](distribution.md)) |
| **`application`-shape unit of release.** | The OTP `.app` file (name, version, modules, dependencies, environment) is the closest existing analog to a Myrhiza app bundle's manifest. Lift the shape (including the `applications:` dependency list as the dep graph). ([`behaviours.md`](behaviours.md)) |
| **Reduction-counting equivalent for fairness across components.** | BEAM gives each process a fixed slice (~2000 reductions) before yielding to the scheduler. WASM has `fuel` / epoch deadlines built in. Use them. The pattern "all components run for the same bounded slice before yielding" is right. ([`runtime-internals.md`](runtime-internals.md)) |
| **First-class tracing primitive.** | Kernel exposes a `trace` operation that any tool can attach to any component, getting function-call / event-send / state-change notifications, without restart, without recompile. This is BEAM's ops-killer-feature; build it from day one. ([`runtime-internals.md`](runtime-internals.md)) |
| **Application start-order driven by dependency declaration.** | OTP's boot script orders applications by their declared dependencies. Myrhiza app bundles should similarly declare component dependencies and the kernel should compute boot order. ([`behaviours.md`](behaviours.md)) |
| **Restart-intensity backstop.** | `(MaxRestarts, MaxSeconds)` — if a component restart-loops faster than the threshold, the kernel escalates (refuses to keep restarting, marks the component unhealthy, propagates upward). This is a load-bearing backstop and not optional. ([`architecture.md`](architecture.md)) |
| **`code_change/3`'s arity and signature stability.** | A small, simple migration callback with a stable signature outlives multiple internal redesigns. Pick the Myrhiza equivalent's signature carefully, then keep it stable. ([`behaviours.md`](behaviours.md), [`hot-code-loading.md`](hot-code-loading.md)) |
| **Dual-stewardship model (implementation owner + community foundation).** | Ericsson + EEF have held up better than Lightbend + Akka. Myrhiza's eventual community structure should separate "who owns the runtime implementation" from "who owns the trademark / spec / wire-format compatibility." ([`history.md`](history.md), [`comparisons.md`](comparisons.md)) |

## Open questions Myrhiza specs need to answer

OTP's experience surfaces questions, but doesn't answer them for our substrate:

1. **What is the Myrhiza-equivalent of `code_change/3`'s signature?** OTP's is `(OldVsn, OldState, Extra) -> {ok, NewState} | error`. We need to pick: do we pass `OldVsn` (semver string? content hash?), do we allow `Extra` (any benefit?), do we require purity (yes, per `state-apply` semantics)?
2. **What kernel-driven validation of a migration is feasible?** OTP did none; this is where Myrhiza could improve. The pre-check shape (run the new `state-apply` over a sample of old events; verify it produces consistent outputs) is doable in principle. Is it doable cheaply enough?
3. **What's the boot-order primitive for inter-component dependencies inside one app bundle?** Lift OTP's `applications:` list, or do we want something richer (per-version constraints, soft vs. hard dependencies)?
4. **Behaviour leader-election: `pg`-shape eventual-consistency or `ra`-shape Raft?** Both exist in the OTP world; the choice is application-dependent. Myrhiza's behaviour-coordination spec needs to make this call ([`open-problems.md`](open-problems.md), [`prior-art/willow/open-problems.md:303`](../willow/open-problems.md)).
5. **What does Myrhiza's `observer`-equivalent look like?** OTP has `observer` (wxWidgets, aging) + `recon` (production-grade lib) + OpenTelemetry. Myrhiza needs all three layers eventually; v1 priority is unclear.

These are the questions a future Myrhiza spec author should bring back to this corpus and use as the starting list. See [`open-problems.md`](open-problems.md) for more.

## Sources

All sources are cited per-file in the underlying corpus files. The synthesis above relies most heavily on:

- [`hot-code-loading.md`](hot-code-loading.md) for the migration callback shape.
- [`distribution.md`](distribution.md) for `pg` / `global` / coordination patterns.
- [`architecture.md`](architecture.md) and [`behaviours.md`](behaviours.md) for supervision-tree mechanics.
- [`critiques.md`](critiques.md) for the unflattering reality checks.
- [`history.md`](history.md) and [`comparisons.md`](comparisons.md) for governance / stewardship lessons.
