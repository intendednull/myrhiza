**Date:** 2026-06-08
**Status:** active
**Subject:** Component Model async ABI — `stream<T>`/`future<T>`, task/subtask, waitable-set, callback vs stackful, WASI 0.3 status

# Component Model async

The async support that Myrhiza would build `host.subscribe` on is specified in
the Component Model `Concurrency.md` explainer (renamed from `Async.md` in PR
#557, 2025-10-31; the old `design/mvp/Async.md` path now 404s, verified via the
GitHub contents API 2026-06-08). It is
gated in WIT/ABI by the 🔀 (async) emoji and lands as part of **WASI Preview 3 /
WASI 0.3**.

## The `async` effect, not a "color"

The Component Model adds an `async` *effect type* on function types:

```wit
interface processor {
  process: async func(in: inputs) -> outputs;  /* may block */
  ready: func() -> bool;                        /* may not block */
}
```

A non-`async` function **traps if the callee blocks before returning** — this is
load-bearing for browsers: it lets non-`async` exports be called in synchronous
contexts (event listeners, getters). An `async` export can still be *implemented*
with the sync ABI, so traditional blocking C code compiles to an `async` export
unchanged. The design explicitly aims *not* to give components a "color" in the
[Nystrom "What Color Is Your Function?"](https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/)
sense.

## Streams and futures

`stream<T>` and `future<T>` each have a **readable end** and a **writable end**.
Per Concurrency.md (verbatim): "When *consuming* a `stream` or `future` value as
a parameter … the receiver always gets *unique ownership* of the *readable end*."
The producer transfers ownership of a readable end it created via
`{stream,future}.new` (which also returns "a fresh paired writable end that is
permanently owned by the calling component instance").

Values move with the `stream.read` / `stream.write` canonical built-ins, which
take a linear-memory buffer pointer + length:

> "These built-ins can either return immediately if >0 elements were able to be
> written or read immediately (without blocking) or return a sentinel 'blocked'
> value indicating that the read or write will execute concurrently."

So I/O is **completion-based** (like `io_uring`/Overlapped I/O), not
readiness-based: notification signals that bytes are *already copied*. A
zero-length read/write is the escape hatch to query *readiness* without copying,
but the spec warns a subsequent non-zero read may still block.

`future` / `stream` may omit `<T>` — then they carry unit values whose *timing*
is the only signal.

## Waitables and waitable-sets

A blocking async call returns "the index of a newly-created **subtask**." Subtasks
are **waitables**; so are the readable/writable ends of streams and futures.
Multiple waitables join a **waitable set**, an `epoll`-style primitive:

- `waitable-set.new` — create empty set
- `waitable.join` — add/move/remove a waitable
- `waitable-set.wait` — block until a member has a pending event, return it
- `waitable-set.poll` — non-blocking variant returning a "none" sentinel

This means one waitable set "can uniformly wait on all the kinds of heterogeneous
I/O" — exactly the shape a multi-topic subscriber wants.

## Two async-export ABIs: stackful vs stackless

An `async` export returns its value by *calling* the imported `task.return`
(not via a core return). The two flavors:

- **Stackful** (🚟-gated): `(func (param …))` with no result; the guest may block
  on `waitable-set.wait` directly, keeping its own native stack.
- **Stackless (callback)**: `(func (param …) (result i32))` plus a companion
  exported **callback** `(func (param i32 i32 i32) (result i32))`. The `i32`
  result tells the runtime what to do next — low 4 bits: `0` = done, `1` = yield,
  `2` = wait on the waitable-set whose index is in the high 28 bits. "The runtime
  will repeatedly call the callback until a value of `0` is returned." This is an
  event-loop style: the guest never holds a suspended native stack between
  events.

The stackless/callback flavor is the one that maps cleanly onto a browser event
loop and onto a per-message push model (see [delivery-patterns.md](delivery-patterns.md)).

## Determinism: async introduces well-defined nondeterminism

Concurrency.md's **Nondeterminism** section is directly relevant to Myrhiza's
convergence model. Until shared-everything-threads lands, concurrency is
*cooperative*, so nondeterminism is only observable at well-defined points. The
spec enumerates internal nondeterministic choices, including (verbatim): "If
there are multiple waitables with a pending event in a waitable set that is being
waited on or polled, there is a nondeterministic choice of which waitable's event
is delivered first." Host-defined `stream`/`future` `read`/`write` ordering is
also host-dependent nondeterminism — though "it is possible for a host to define
a deterministic ordering."

Implication for Myrhiza: **the order in which subscription messages and topics
are delivered through a waitable set is nondeterministic and host-controlled.**
That is fine for an `interaction` component (non-deterministic, per-peer) but it
is exactly why subscription delivery order must never feed a canonical
state-digest. See [lessons.md](lessons.md).

## WASI 0.3 status (2026-06)

- WASI 0.3 / Preview 3 is the version that adds native async + `stream`/`future`;
  it refactors P2 interfaces onto native async (wasi.dev/roadmap).
- Status is **preview / release candidate**: snapshot `0.3.0-rc-2026-03-15`,
  matching what Wasmtime 43 ships. Completion targeted "around February 2026"
  per the roadmap but the RC train was still running in Q2 2026.
- Real-world (wasmCloud, 2026): "preview-quality, working end-to-end for Rust
  HTTP components, with some gaps elsewhere"; long-lived streams under load and
  backpressure are explicitly called out as not-yet-hardened.

## Sources

- https://github.com/WebAssembly/component-model/blob/main/design/mvp/Concurrency.md (fetched via GitHub API, main branch, 2026-06-08)
- https://github.com/WebAssembly/component-model/pull/557 (rename Async.md → Concurrency.md, 2025-10-31; Async.md confirmed absent via contents API 2026-06-08)
- https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md
- https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/ (Nystrom, "What Color Is Your Function?", 2015-02-01)
- https://wasi.dev/roadmap
- https://wasmcloud.com/blog/wasi-p3-on-wasmcloud/
- https://component-model.bytecodealliance.org/
