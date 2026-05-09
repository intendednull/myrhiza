**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Bare: the small embeddable JS runtime under Pear and Keet

# Bare Runtime

Bare is the JavaScript runtime Pear apps execute on. It is also the runtime
embedded inside Holepunch's mobile apps (Keet iOS / Android). For Myrhiza spec
purposes, Bare is the most important non-WASM data point in this prior-art
set, because it is the load-bearing answer to "how do you actually ship a
JavaScript-based P2P app to a phone?"

## What Bare Is

Bare (`holepunchto/bare`, Apache-2.0, **1072 stars**, created 2022-12-18, last
updated 2026-05-06; npm package `bare`, current version **1.28.5** as of
2026-05-06) is, in its own README's words, "a small and modular JavaScript
runtime for desktop and mobile. Like Node.js, it provides an asynchronous,
event-driven architecture for writing applications in the lingua franca of
modern software. Unlike Node.js, it makes embedding and cross-device support
core use cases, aiming to run just as well on your phone as on your laptop."

The release cadence is high. The three most recent releases are v1.28.5
(2026-05-06), v1.28.4 (2026-04-07), v1.28.3 (2026-04-07). The GitHub Releases
API only exposes the most recent few; npm shows tags running back through v1.x.

Install path:

```sh
npm i -g bare
bare script.js
```

You can also embed it as a library — see Embedding below.

## Architecture

From the README:

> Bare is built on top of <https://github.com/holepunchto/libjs>, which provides
> low-level bindings to V8 in an engine independent manner, and
> <https://github.com/libuv/libuv>, which provides an asynchronous I/O event
> loop. Bare itself only adds a few missing pieces on top to support a wider
> ecosystem of modules:
>
> 1. A module system supporting both CJS and ESM with bidirectional
>    interoperability between the two.
> 2. A native addon system supporting both statically and dynamically linked
>    addons.
> 3. Light-weight threads with synchronous joins and `SharedArrayBuffer`
>    support.

So Bare = libjs (V8 ABI shim, `holepunchto/libjs`, 79 stars, "Simple and ABI
stable C bindings to V8 built on libuv") + libuv + a thin runtime. Note: the
README explicitly says "engine independent manner" — Bare is not theoretically
locked to V8, but in practice libjs is the only `BARE_ENGINE` shipped today
(default `github:holepunchto/libjs`).

## The Motivation

Bare exists because Node.js is too big for mobile and too unstable across
versions to embed reliably. The README does not commit to a specific binary
size, so the "Node is 50 MB, Bare is X MB" framing in the original task brief
is **not directly verifiable from the README** — flagging this. What the README
*does* commit to is:

- Tier 1 platform support for Android (arm/arm64/ia32/x64) and iOS (arm64,
  x64-simulator), alongside macOS, Linux, and Windows.
- An explicit C embedding API (`bare_setup`, `bare_load`, `bare_run`,
  `bare_teardown`, `bare_suspend`, `bare_resume`) defined in `include/bare.h`.
- A suspension/wakeup lifecycle ("needed for platforms with strict application
  lifecycle constraints, such as mobile platforms" — paraphrased) that Node
  does not expose.

The mobile-lifecycle story is where Bare diverges most sharply from Node.
Mobile OSes can suspend your app at any time; Bare exposes this as a
first-class state machine:

```
Active <-> Suspending <-> {Awake, Idle, Suspended}  ->  Resume
                              \-> Terminated
```

Each transition emits an event (`suspend`, `wakeup`, `idle`, `resume`) that
JavaScript code can hook to drain network connections, pause replication, etc.
Node does not have this; an embedder of Node into a mobile app has to fight
the lifecycle from outside.

## Module System

Bare's module system is its own, in `holepunchto/bare-module`. It supports
both CommonJS (the default for `.js`) and ESM (when `"type": "module"` is set,
or for `.mjs`), with bidirectional interop. Per `bare-module`'s README, it
implements the `package.json` fields you'd expect: `name`, `version`, `type`,
`exports` (including subpath and conditional exports), `imports`, etc.

This means a Bare app's `package.json` looks almost identical to a Node app's,
which is deliberate — you can author for both. The compatibility shim is
`bare-node-compat` (no longer at the original repo URL by that exact name as
of this writing — flagging that the original verification command returned
404 for `holepunchto/bare-node-compat`; the compat layer is referenced in the
Pear docs at <https://docs.pears.com/reference/node-compat.html> but the precise
repo location may have moved or been folded into other modules).

The standard library is *not* shipped with Bare. The README:

> Bare provides no standard library beyond the core JavaScript API available
> through the `Bare` namespace. Instead, we maintain a comprehensive collection
> of external modules built specifically for Bare.

Holepunch ships and maintains an extensive set of `bare-*` packages
(`bare-fs`, `bare-os`, `bare-tty`, `bare-buffer`, `bare-events`, `bare-path`,
`bare-subprocess`, `bare-module`, `bare-console`, `bare-bundle`, `bare-ipc`,
`bare-channel`, etc.) that together cover roughly the surface area of Node's
`fs`, `os`, `events`, `path`, `child_process`, `console`, IPC, and so on.
Pear's `package.json` depends on most of these.

This is **a large amount of duplicated work** vs. just shipping Node. The
payoff is control: every `bare-*` module is small, embeddable, and built to
not assume Node-isms. The cost is that the ecosystem is a small fraction of
npm's, and any third-party Node module needing native access must either
work through `bare-node-compat` or be ported.

## Native Addons

Bare's native-addon ABI is exposed via `Bare.Addon`. The core C interface is
ABI-stable (this is the point of `libjs` — it's "Simple and ABI stable C
bindings to V8"). For Node-API (`napi.h`) compatibility, Bare ships a
`include/node_api.h` header — a tiny 79-byte file, presumably just an
`#include` redirect rather than a full reimplementation. The intent is that
N-API addons compile against Bare with minimal source change, though
addon-by-addon validation is required.

Addons can be loaded statically (linked into the Bare binary) or dynamically
(loaded as shared libraries at runtime). For mobile, static linking is the
norm — Apple's App Store policy historically restricted dynamic loading.

## Embedding

The C API in `include/bare.h`:

```c
#include <bare.h>
#include <uv.h>

bare_t *bare;
bare_setup(uv_default_loop(), platform, &env, argc, argv, options, &bare);
bare_load(bare, filename, source, &module);
bare_run(bare, UV_RUN_DEFAULT);
int exit_code;
bare_teardown(bare, UV_RUN_DEFAULT, &exit_code);
```

For mobile embedding, see `holepunchto/bare-android` (Android example,
Apache-2.0, 21 stars, created 2023-03-01) and `holepunchto/bare-ios` (iOS
example, Apache-2.0, 16 stars, created 2023-02-06). Both use
`holepunchto/bare-kit` ("Bare for native application development",
Apache-2.0, 37 stars, created 2024-06-21) as the higher-level wrapper that
bundles the runtime, IPC, and platform glue into something an iOS/Android
project can drop in.

Prebuilt binaries for the supported platforms ship via
`holepunchto/bare-runtime` ("Prebuilt Bare binaries for macOS, iOS, Linux,
Android, and Windows", 10 stars).

## Mobile Integration: How Keet Ships JS to a Phone

The chain is:

1. The Keet mobile app is a native iOS / Android application.
2. The native shell uses `bare-kit` to embed `libbare.a` (iOS) or
   `libbare.so` (Android) into the app binary.
3. The native shell starts a Bare instance via `bare_setup` and loads the
   Keet JS bundle via `bare_load`.
4. Keet's JS calls into native UI primitives via IPC streams (`bare-ipc`,
   `bare-channel`); the React-Native-shaped code path is on the native side,
   the P2P logic is on the Bare side.
5. On suspend (app backgrounded), the native shell calls `bare_suspend`; on
   resume, `bare_resume`.

This is the only "JS-in-mobile-with-real-P2P" path Holepunch maintains. It is
the load-bearing piece of why Keet exists at all — and why Keet's stack
matters as a prior-art point even if the apps themselves are closed-source.

## Differences from Node

Things Bare does not have or has differently:

- **No bundled stdlib.** Everything is `bare-*` modules.
- **Different process model.** `Bare.Thread` is a lightweight thread
  primitive, not Node's `worker_threads`.
- **No npm-bundled tools.** No `npx`, no `npm` shipped with Bare itself
  (you install Bare via npm but Bare doesn't include it).
- **Mobile lifecycle.** First-class suspend/wakeup/idle/resume.
- **Embeddable C API.** Node has Node-API for *embedding native code into JS*,
  but Bare's `bare_setup`/`bare_run`/`bare_teardown` is for *embedding JS into
  a host application* — closer in spirit to V8's own embedder API.
- **Smaller surface area.** README posture is explicitly "bare." If Node has
  it and userland could implement it, Bare doesn't ship it.
- **Engine independence (in theory).** `BARE_ENGINE` is configurable, with
  libjs as the default. In practice libjs+V8 is what's tested.

## Determinism Characteristics

Bare is **not deterministic**, in the same way Node is not deterministic. V8
optimizes adaptively, libuv interleaves I/O nondeterministically, JS itself
has nondeterministic ordering for several operations (`Promise` resolution
across microtask boundaries, weakref finalization, etc.).

This is fine for the role Bare plays in the Holepunch stack: Hypercore is
already an append-only-log-with-cryptographic-merkle-proofs design where the
*log itself* is the source of truth, not the execution that produced it. Two
peers running different versions of Bare can append to the same Hypercore and
the result converges because the log is the convergence primitive, not the
execution.

For Myrhiza's component profiles (see `CLAUDE.md`):

- Bare-shaped runtimes fit `interaction` and `behavior` workloads — UI,
  bots, bridges. Nondeterminism is fine.
- Bare-shaped runtimes do **not** fit `state-apply`. The strict-determinism
  invariant ("pure function of `(prior state, event)` plus the deterministic
  helper set") cannot be satisfied by a Node-flavored JS runtime, and Myrhiza
  treats any non-determinism in `state-apply` as a correctness bug.

## Implications for Myrhiza

Direct lessons (things to copy or adapt):

- **Small, embeddable runtime as a goal.** Bare's pitch — embed into mobile,
  small binary, ABI-stable native API — is exactly what Myrhiza needs from
  wasmtime + the Component Model. Wasmtime is bigger than Bare today; that's
  a real cost we should be honest about. The Bare team chose to fork the
  ecosystem rather than carry Node's mass into mobile. Myrhiza will face a
  similar choice if wasmtime's mobile story doesn't tighten.
- **First-class mobile lifecycle.** Bare's suspend/wakeup/idle/resume state
  machine is something Myrhiza's runtime *will* need before it ships on
  iOS/Android. Don't wait until v2 to design this — wire it into the host
  ABI from the start.
- **ABI stability as a feature.** libjs is "ABI stable C bindings to V8."
  Wasmtime's component-model ABI is similarly the surface third parties
  will compile against. Treat any change to that ABI with the same
  conservatism Holepunch treats libjs changes.
- **The "no stdlib, all userland modules" posture.** The Component Model is
  inherently this way (every capability is an import); Bare arrived at the
  same place from the JS side. This is a confirmation of the design
  direction, not a coincidence.
- **Static-linking native addons for mobile.** Apple and Google both
  restrict dynamic loading. Myrhiza's component-instantiation story should
  be static-friendly. Don't design a host that *requires* runtime dynamic
  loading.

Anti-patterns Myrhiza explicitly skips:

- **JS as the guest language.** Already covered in `pear-runtime.md` — the
  whole reason Myrhiza chose WASM Component Model is that JS-in-a-runtime
  cannot offer per-call capability mediation, cannot offer determinism, and
  cannot offer language-agnostic guests. Bare confirms how much engineering
  investment it takes to make JS-in-mobile *workable*; that investment buys
  none of the properties Myrhiza needs.
- **Process-only isolation.** Pear apps are isolated by being in separate
  Bare processes. Myrhiza isolates at the WASM-component boundary — much
  finer-grained, no per-app process overhead, deterministic for `state-apply`.
- **Forking the ecosystem.** Bare maintains 100+ `bare-*` packages because
  it can't reuse Node's. Myrhiza must not end up forking the WASI / Component
  Model ecosystem; the win of Component Model is *not* having to do this.
  Stay aligned with upstream WASI Preview 2 / Preview 3 even where it costs
  short-term feature velocity.
- **Treating determinism as out-of-scope.** Bare can punt on this because
  Hypercore is the convergence primitive. Myrhiza's `state-apply` profile
  *is* the convergence primitive, so Myrhiza cannot punt on it.

UX reality checks:

- **Mobile suspend is brutal.** Bare exists in part because mobile OSes
  punish long-running JS processes. Myrhiza's runtime on mobile will hit the
  same wall — and WASM doesn't make it easier. Plan for the "runtime hosted
  inside a long-lived sidecar / native service" pattern from day one.
- **Closed-source flagships are real.** Keet is closed; Bare is open. The
  open layer is what we can study; the deployed UX is opaque to us. Be honest
  in spec writing about the line between "verified" and "inferred from public
  artifacts."

## Cross-References

- [`pear-runtime.md`](pear-runtime.md) — the deploy/discover layer that runs
  on Bare
- [`hypercore-stack.md`](hypercore-stack.md) — data layer atop Bare
- [`hyperswarm.md`](hyperswarm.md) — networking atop Bare
- [`keet-and-apps.md`](keet-and-apps.md) — what Bare actually ships in
- [`history.md`](history.md) — Bare's role in the Dat→Holepunch evolution
- [`comparisons.md`](comparisons.md) — Bare vs Node vs Deno vs Bun vs
  wasmtime
- [`critiques.md`](critiques.md) — Bare's small ecosystem, V8 lock-in
- [`open-problems.md`](open-problems.md) — engine independence, determinism,
  WASI integration
- [`lessons.md`](lessons.md) — what Myrhiza takes away
- Prior-art neighbors:
  [`../wasm-component-model/wasmtime.md`](../wasm-component-model/wasmtime.md)
  (substrate comparison),
  [`../iroh/mobile-and-wasm.md`](../iroh/mobile-and-wasm.md) (peer of
  "JS-in-mobile" question),
  [`../wasmcloud/`](../wasmcloud/),
  [`../holochain/`](../holochain/)

## Sources

- Bare repo and README: <https://github.com/holepunchto/bare>
- Bare `include/bare.h` (C embedding ABI):
  <https://github.com/holepunchto/bare/blob/main/include/bare.h>
- Bare on npm (current version 1.28.5, 2026-05-06):
  <https://www.npmjs.com/package/bare>
- libjs (engine ABI shim): <https://github.com/holepunchto/libjs>
- bare-module (module system): <https://github.com/holepunchto/bare-module>
- bare-kit (mobile embedding wrapper): <https://github.com/holepunchto/bare-kit>
- bare-android example: <https://github.com/holepunchto/bare-android>
- bare-ios example: <https://github.com/holepunchto/bare-ios>
- bare-runtime (prebuilt binaries): <https://github.com/holepunchto/bare-runtime>
- Pear Node.js compatibility reference:
  <https://docs.pears.com/reference/node-compat.html>
- Pear Bare overview reference:
  <https://docs.pears.com/reference/bare-overview.html>
- libuv: <https://github.com/libuv/libuv>
