**Date:** 2026-06-08
**Status:** active
**Subject:** Browser delivery — jco + JSPI viability, the Safari gap, Asyncify vs JSPI, and the in-task deadlock hazard

# Browser: jco + JSPI

In the browser, Myrhiza runs the same component transpiled by **jco** to ES
modules. This is the *hard* side: the browser has no native Component Model
runtime, so async at the WIT boundary must be lowered onto a JavaScript async
mechanism. jco supports exactly one such mechanism.

## jco async is JSPI-only (and experimental)

jco's transpiler exposes async via `--async-mode`, and per the jco docs (verbatim):
"EXPERIMENTAL: For the component imports and exports, functions and methods on
resources can be specified as `async`. **The only option is `jspi`**." Companion
flags `--async-imports` / `--async-exports` select which boundaries are async.

There is **no Asyncify path for component async in jco**. (Asyncify is the older
Binaryen/Emscripten binary-rewriting technique; jco does not use it to lower
component-model async.) So in the browser, async-at-the-WIT-boundary == JSPI ==
the union of browsers that ship JSPI.

## JSPI mechanism

JSPI (the Component Model explicitly lists "Allow polyfilling in browsers via
JavaScript Promise Integration" as a goal) provides:

- **`WebAssembly.Suspending`** — wraps an *import* so the wasm call suspends
  "until the `Promise` returned by the import is resolved," then resumes with the
  resolved value.
- **`WebAssembly.promising`** — wraps an *export* so calling it returns a
  `Promise`.

This is how a transpiled component awaits host (JS) async work, and how jco
surfaces an async export to JS callers.

## Browser shipping status (verified, 2026-06)

| Browser | Status | Detail |
|---|---|---|
| Spec | **Phase 4** | WebAssembly CG voted 2025-04-08 (wpt/interop#1093) |
| Chrome | **shipped, unflagged** | since **Chrome 137**; origin trial ran Chrome 123–136; "will ship enabled for all users" (blink-dev Intent to Ship) |
| Firefox | **behind a flag** | available ≈Firefox 139+ but flagged; expected to unflag "this year" (2026) per platform.uno |
| Safari/WebKit | **in preview, not in stable** | objection removed late 2025; JSPI is in **Interop 2026**; landed in **Safari Technology Preview 238** (released 2026-02-26) but not in any stable Safari release as of 2026-06 |

**The Safari gap is the load-bearing browser constraint for Myrhiza.** Any
design that *requires* async at the WIT boundary in the browser excludes Safari
users until WebKit ships JSPI in *stable* Safari. JSPI has already landed in
Safari Technology Preview 238 (2026-02-26), so the gap is narrowing — but a
preview channel is not a deployment target; the constraint holds until a stable
release ships it (timeline unknown; Interop 2026 is the signal, not a ship date).

## Asyncify vs JSPI (why jco picked JSPI)

For context on the abandoned alternative: Asyncify instruments the whole binary
to unwind/rewind the call stack into linear memory. Costs (Binaryen/Emscripten
docs): ".wasm file can grow by 50% or more"; every function call gets slower from
mode checks. JSPI has "zero overhead during normal execution" — cost is paid only
at suspension via the browser's stack-switching. JSPI wins on size and steady-state
speed; its cost is the browser-availability gap (Safari). This is why jco lowers
component async exclusively through JSPI.

## In-task deadlock hazard (wit-bindgen#1609)

A structural JSPI limitation directly relevant to *streaming into a guest*:
**JSPI suspends a single execution context rooted at one `WebAssembly.promising`
export.** If a whole Rust async runtime (executor + all its tasks) sits on that
one context, suspending it freezes *every* task. The reported smoking gun: a
`futures::join!()` where arm A awaits a **sync-form** host import (returning a
Promise via `WebAssembly.Suspending`) while arm B must write to the stream arm A
is reading — arm A's suspension parks the entire executor, arm B never runs, the
read hangs.

Mitigations noted upstream: declare the import as `async func` (async-form
lowering rather than sync-form), spawn subtasks as independent `promising`
entries, or defer suspension until the executor queue is empty. For Myrhiza this
means: a browser `interaction` component that both *receives* a subscription
stream and *calls* sync-form host imports inside the same guest task can deadlock
under JSPI. The subscription delivery and the guest's other host calls must be
structured so they don't co-suspend one `promising` root.

## Net for Myrhiza (browser)

- WIT-boundary async streaming works in **Chrome today**, **Firefox behind a
  flag**, **not in stable Safari** (preview only, STP 238).
- Therefore the *portable* design cannot depend on async at the WIT boundary in
  the browser. The submit-and-poll / guest-callback shape, which keeps the WIT
  boundary **synchronous** and lets the host (the jco runtime shim) re-enter the
  guest, sidesteps JSPI entirely and runs in every browser. See
  [delivery-patterns.md](delivery-patterns.md) and [lessons.md](lessons.md).

## Sources

- https://bytecodealliance.github.io/jco/transpiling.html
- https://github.com/bytecodealliance/jco/blob/main/docs/src/transpiling.md
- https://github.com/WebAssembly/js-promise-integration/blob/main/proposals/js-promise-integration/Overview.md
- https://github.com/web-platform-tests/interop/issues/1093
- https://groups.google.com/a/chromium.org/g/blink-dev/c/w_jCD4gf7Bc (Intent to Ship, Chrome 137)
- https://developer.chrome.com/blog/webassembly-jspi-origin-trial
- https://webkit.org/blog/17818/announcing-interop-2026/
- https://webkit.org/blog/17848/release-notes-for-safari-technology-preview-238/ (STP 238, JSPI landed, 2026-02-26)
- https://platform.uno/blog/the-state-of-webassembly-2025-2026/
- https://github.com/bytecodealliance/wit-bindgen/issues/1609
- https://loke.dev/blog/wasm-jspi-async-integration
- https://emscripten.org/docs/porting/asyncify.html
