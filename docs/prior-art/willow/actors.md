**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — `willow-actor` framework, state-management discipline, dual-target runtime

`willow-actor` is the in-tree concurrency primitive: per-message `Handler<M>`,
typed `Addr<A>`, supervision, dual-target native+WASM. The companion
state-management discipline bans `Arc<Mutex<T>>` for business state and routes
all shared mutable state through actors. Both are the direct architectural
ancestors of Myrhiza's kernel-internal concurrency model.

See also: [workers.md](workers.md), [apps.md](apps.md),
[state-machine.md](state-machine.md), [README.md](README.md).

## Core primitives — shipped today

`crates/actor/src/lib.rs:1-60` (1420 lines) declares the public surface:

- **`Actor`** trait with `started()` / `stopped()` / `idle()` lifecycle hooks.
  Uses RPITIT (Rust 1.75+) instead of `async_trait` — no proc macro, no
  Box-per-handler. `Send + 'static + Sized`; actors are single-owner so
  `Sync` is never required.
- **`Handler<M: Message>`** — implemented per message type. New message types
  are additive (just add another impl), unlike ractor's single `Msg` enum.
  `Message` carries `type Result: Send + 'static`, enabling typed request-reply.
- **`Addr<A>`** — cheap-clone typed handle. Mailbox bounded at
  `DEFAULT_MAILBOX_CAPACITY = 10_000` (`runtime.rs:38`); a full mailbox
  drops with a `tracing::warn` rather than blocking. `do_send()` for
  fire-and-forget, `ask()` for typed reply via internal oneshot.
- **`Recipient<M>`** — type-erased single-message handle for pub/sub.
- **`Context<A>`** — `address()`, child `spawn()`, `stop()`, intervals.
- **Mailbox semantics** — bounded MPSC. Processes one message via
  `recv().await`, drains remaining via `try_recv()`, then calls `idle()`.
  `idle()` is the batching hook: set a dirty flag during mutations, fire
  subscriber notifications once in `idle()` after the burst drains.
- **`StreamHandler<S>`** — consumes external streams as actor messages
  (used to wrap iroh `TopicEvents` without manual `while let` loops).
- **Supervision** — `RestartPolicy::{Never, OnFailure{max}, Backoff{initial,
  max_delay, max_retries}}` (`supervisor.rs:18-29`). Restart preserves the
  `Addr` because the channel survives; only the actor instance is replaced.

### Dual-target runtime (`crates/actor/src/runtime.rs`)

`cfg`-switched per target. Native: `tokio::task::spawn` / `tokio::time::sleep`
/ `tokio::sync::mpsc` + `oneshot`. WASM: `wasm_bindgen_futures::spawn_local`
/ `gloo_timers::future::sleep` / `futures_channel::mpsc` + `oneshot`. Single
API surface; `Send` required unconditionally because on WASM (single-threaded)
everything is trivially `Send`. The actor-system spec
(`docs/specs/2026-03-29-actor-system-design.md` §"Iroh integration")
deliberately rejects ractor's `tokio_with_wasm` shim — Willow already uses
`wasm-bindgen-futures` directly via iroh; another tokio-shaped layer is
indirection without value.

## Extended types — shipped today

- **`StateActor<S>`** (`state.rs:77-100`, 550 lines). State as `Arc<S>` for
  cheap reads; mutations via `Arc::make_mut()` for copy-on-write — in-place
  when refcount is 1, clone-then-mutate otherwise. Messages: `Get<S>`,
  `Set<S>`, `Mutate` (closure), `Select` (closure read), `Subscribe`. `Notify`
  fires once per `idle()` round, not per mutation. `S: Send + Sync + 'static`;
  `S: Clone` only required for `Mutate`.
- **`StateRef<S>`** — type-erased cloneable handle composing both
  `StateActor` and `DerivedActor`.
- **`DerivedActor<Src, T>`** (`derived.rs`, 482 lines). Subscribes to one
  or more `StateRef` sources via the `DeriveSource` trait (single refs and
  tuples up to arity 6); recomputes when sources change, notifies subscribers
  only when value actually changes (`PartialEq`).
- **`Broker<T>`** (`broker.rs`, 276 lines). Pub/sub. Dead recipients
  auto-pruned on next publish. Used for cross-actor events
  (e.g. `ClientEvent`).
- **`FsmActor<M>`** (`fsm.rs`, 460 lines), **`Pool<A>`** (round-robin work
  distribution, 233 lines), **`Debounce<M>` / `Throttle<M>`** (330 lines),
  **`StreamOutput<T>`** (actor-produced async streams).

## State-management discipline — the rule

`docs/specs/2026-04-26-state-management-model-design.md` (lines 32-79):

> Shared mutable state in library crates lives inside an actor. No
> `Arc<Mutex<T>>` / `Arc<RwLock<T>>` / `parking_lot::*` for business state.
> The actor owns the data; consumers send messages.

Decision tree (spec lines 38-79):

```
Need shared mutable state?
├─ Default                                 → StateActor<S> or bespoke actor
├─ External-callback boundary (iroh)?      → Lock OK (// state: lock-ok)
├─ Sync trait abstraction (legacy)?        → Single Mutex<Inner>
├─ One-shot init of static data?           → OnceLock<T> / LazyLock<T>
├─ Cross-task control flag (stop/cancel)?  → AtomicBool / AtomicU32
├─ Single-threaded WASM interior mut?      → Rc<RefCell<T>>
├─ Reactive UI state in web?               → Leptos signal (StateActor only
│                                              when mutated outside reactive scope)
└─ Coordination signal between actors?     → tokio::sync::watch / oneshot /
                                              broadcast / Notify
```

The rule explicitly forbids `tokio::sync::Mutex` for business state too.
No hard CI gate — agent-context awareness instead: rule lives in `CLAUDE.md`,
every PR review reasons against it.

### Why locks were banned

Spec §"Problem" (lines 8-19) audits six hotspots where multi-`parking_lot::Mutex`
patterns hand-rolled atomicity that `StateActor` provides for free.
`SearchIndexHandle` carried four independent `Mutex`es (`search/handle.rs:29-32`)
with no cross-field atomicity; `MemNicknameStore` used `RwLock<HashMap>` +
`RwLock<u64>` version where `StateActor`'s `Notify` is version-bumped. Without
a written rule, contributors reverse-engineered patterns from neighbours that
included both the right answer (`StateActor` in workers) and the wrong answer
(`Arc<Mutex<_>>` in `ClientHandle`). Why `StateActor` by default (lines 81-92):
atomicity across fields, batched change notification, cheap `Arc` reads,
copy-on-write, single-task ownership, dual-target — all from one primitive.

### Documented exemptions (`// state: lock-ok — <reason>`)

- `crates/network/src/iroh.rs:107,132,196,197,215` — iroh callback boundary.
- `crates/network/src/mem.rs` — `MemNetwork`, `#[cfg(feature = "test-utils")]`.
- `crates/client/src/lib.rs:275,318,337` — `topics`, `join_links`,
  `pending_joins`. Single guards; actor migration deferred (F4 in spec
  §"Follow-up work") because it ripples through `MutationContext` /
  `ListenerContext` generic over `N::Topic`.
- `crates/client/src/mentions.rs:79` — `OnceLock<Regex>`.
- `Rc<RefCell<_>>` / `SendWrapper<RefCell<_>>` in `crates/web/src/*.rs` —
  single-threaded WASM, `RefCell` is correct.
- `tokio::sync::watch` / `oneshot` in worker runtime — coordination
  signals, not shared mutable state.

## For Myrhiza

PR #636 §"Runtime and actors" (diff lines 387-419) commits to: the runtime
sits *underneath* the actor model, not in place of it. The mapping:

- **Component instances are owned 1:1 by an actor.** The actor's mailbox
  serializes calls into the component's WASM instance. Component instances
  are the unit of typed sandboxing; actors remain the unit of concurrency.
- **The kernel itself is composed of actors** — loader actor, per-topic
  state-materialization actor (which owns one state component instance and
  calls `apply` on each event), interaction actors per active interaction
  component, behavior actors per behavior instance.
- **Lock-vs-actor decisions in kernel code still follow the existing
  decision tree.** Components never see locks; they see only the actor's
  mailbox semantics, surfaced as synchronous WIT calls.
- **Persistence is owned by host actors, not by components.** A state
  component returns updated state in linear memory; the kernel-side
  materialization actor decides snapshotting + storage coordination.

Dual-target runtime (`tokio` native, `wasm-bindgen-futures` web) is
*intended* to survive at the kernel layer per PR #636 §"What stays the
same about Willow" (diff lines 354-364). Concrete subsystems historically
native-only (MLS engine, persistent key storage, full-fat blob store) need
platform backends behind a stable kernel-internal trait — confirming each
survives jco transpilation is a child-spec concern.

What changes for Myrhiza:

- **Per-message `Handler<M>` survives unchanged.**
- **`StateActor<S>` becomes a kernel implementation detail.** Apps don't
  see it; an app's state component owns its own linear memory.
- **Supervision policy generalizes** to per-instance restart for WASM
  components that trap. Out-of-fuel and trap conditions become
  supervisor-visible failures.
- **`Broker<T>` is a candidate kernel primitive** for routing inter-component
  events without per-pair wiring.

The `idle()` batching hook is load-bearing for state-component performance
under bursty event loads — it lets a stream of incoming events apply through
`state-apply`, mark state dirty, then fire one `Notify` to subscribed
interaction components per drain cycle rather than once per event. Worth
preserving.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- `crates/actor/src/lib.rs:1-60` — public surface
- `crates/actor/src/{actor,addr,context,mailbox,envelope}.rs` — core types
- `crates/actor/src/runtime.rs:1-145` — dual-target runtime
- `crates/actor/src/state.rs:1-100,77-100` — `StateActor<S>` + messages
- `crates/actor/src/derived.rs:1-80` — `DerivedActor<Src, T>`, `DeriveSource`
- `crates/actor/src/broker.rs:55-60` — `Broker<T>` pub/sub
- `crates/actor/src/supervisor.rs:16-44` — `RestartPolicy`, `spawn_supervised`
- `crates/actor/src/{fsm,pool,debounce,stream}.rs` — extended types
- `docs/specs/2026-03-29-actor-system-design.md` — original framework design,
  why-not-ractor/xtra/kameo
- `docs/specs/2026-03-31-actor-system-library-design.md` — `StateActor` /
  `DerivedActor` / streams extension
- `docs/specs/2026-04-26-state-management-model-design.md:8-200` — lock ban,
  decision tree, audit hotspots, exemption catalogue
- PR #636 (`/tmp/willow-pr-636.diff`) lines 387-419 — "Runtime and actors":
  kernel composes from actors with components owned 1:1
- PR #636 lines 354-364 — dual-target intent at kernel layer
