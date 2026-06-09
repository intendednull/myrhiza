**Date:** 2026-06-08
**Status:** active
**Subject:** Async/streaming delivery from host to a sandboxed WASM Component Model guest — native (Wasmtime) and browser (jco/JSPI)

# wasm-async-streaming

How does a trusted host push a *stream* of messages into a sandboxed WASM
Component Model guest, both natively (Wasmtime) and in the browser (jco
transpile)? This corpus exists to inform Myrhiza's **host.subscribe** capability:
a kernel-mediated subscription that delivers a per-topic stream of state/events
into a sandboxed `interaction` component, which aggregates channels in-sandbox.

The central tension: the Component Model now has a real streaming primitive
(`stream<T>`), but Myrhiza's existing async ABI (§8.5) is a sync-only
**submit-and-poll** pattern using single-use request-tokens — a one-token /
one-completion shape that does **not** model continuous per-message delivery.
And the browser path (jco) can only express async via JSPI, which stable Safari
does not yet ship (it is in Safari Technology Preview as of early 2026).

## Key facts

| Fact | Value | Verified source |
|---|---|---|
| Component Model async lives in | `Concurrency.md` (formerly `Async.md`) | github.com/WebAssembly/component-model |
| Streaming primitive | `stream<T>` / `future<T>`, with readable + writable ends | Concurrency.md |
| WASI version adding native async | WASI 0.3 (Preview 3) | wasi.dev/roadmap |
| WASI 0.3 status (2026-06) | Preview / release-candidate `0.3.0-rc-2026-03-15` | wasmCloud, wasi.dev |
| Wasmtime with WASIp3 + stream/future host API | 43.0.0 (2026-03-20); async maturing since v37 | github wasmtime releases |
| Native host→guest stream API | `StreamReader::new(store, StreamProducer)`, `pipe`, `StreamConsumer` | docs.wasmtime.dev |
| Two async-export flavors | stackful (`task.return`) and stackless (`callback`) | Concurrency.md |
| Browser async lowering in jco | JSPI **only** (`--async-mode jspi`, EXPERIMENTAL) | jco docs |
| JSPI spec phase | Phase 4, voted 2025-04-08 | wpt/interop#1093 |
| JSPI Chrome | unflagged since Chrome 137 (OT 123–136) | chromestatus / blink-dev |
| JSPI Firefox | behind a flag (≈139+) | platform.uno, interop#1093 |
| JSPI Safari | objection removed late 2025; in Interop 2026; landed in **Safari Tech Preview 238** (2026-02-26); **not in stable** | webkit.org, interop#1093 |

## Table of contents

- [lessons.md](lessons.md) — **the decision file**: options for streaming into a
  sandboxed guest, which works both natively and in-browser, the Safari/JSPI
  constraint, and how each interacts with Myrhiza's single-use submit-and-poll
  token model. Validates / Avoid / Borrow.
- [component-model-async.md](component-model-async.md) — `stream<T>`/`future<T>`,
  the async ABI, task/subtask, waitable-set, callback vs stackful, WASI 0.3
  status.
- [wasmtime-async.md](wasmtime-async.md) — native delivery: `Store::call_async`,
  epochs, async host functions, and the P3 host stream API
  (`StreamReader`/`StreamProducer`/`StreamConsumer`).
- [browser-jspi.md](browser-jspi.md) — jco + JSPI browser viability, the Safari
  gap, Asyncify vs JSPI, and the in-task deadlock hazard.
- [delivery-patterns.md](delivery-patterns.md) — submit-and-poll vs
  stream-resource vs guest-callback, with determinism notes.
- [open-problems.md](open-problems.md) — what these systems structurally don't
  solve → Myrhiza's risk list.

## Canonical reading order

1. `component-model-async.md` — the primitive and its vocabulary.
2. `delivery-patterns.md` — the three shapes for getting a stream into a guest.
3. `wasmtime-async.md` then `browser-jspi.md` — the two runtimes.
4. `lessons.md` — the Myrhiza decision.
5. `open-problems.md` — residual risk.

## Glossary stub

- **stream\<T\>** — Component Model value type carrying an ordered sequence of `T`;
  has a *readable end* and a *writable end*, each a waitable.
- **future\<T\>** — like `stream<T>` but delivers at most one value.
- **waitable / waitable-set** — an `epoll`-like set; `waitable-set.wait` blocks
  until one member (subtask, stream end, future end) has a pending event.
- **task / subtask** — a Component Model green-thread of execution; a blocking
  async call yields a *subtask* index to the caller.
- **stackful vs stackless export** — two async-export ABIs; stackless uses an
  exported **callback** the runtime re-invokes until it returns done.
- **JSPI** — JavaScript Promise Integration; lets wasm suspend on a JS Promise
  (`WebAssembly.Suspending` / `WebAssembly.promising`). The only async lowering
  jco supports.
- **submit-and-poll** — Myrhiza's current async ABI (§8.5): a `*-submit` returns a
  single-use `request-token`; the kernel re-enters via an exported
  `on-*-completion` handler. One token = one completion.
