**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — ABI and composition


## 8. ABI and composition

### 8.1 Decision

**Full WebAssembly Component Model from day one**. WIT-bindgen for
the SDK (Rust). Wasmtime native runtime. jco-transpiled glue + core
wasm in browser.

### 8.2 Why full CM and not Extism

PR #636 leaned Extism v1 → CM v2 for ship-faster reasoning. Myrhiza
rejected this:

- Extism cannot express WCM resource handles, borrows, world
  composition, or futures/streams. Migration is a real refactor for
  app authors — not a regenerate-bindings event.
- Every Myrhiza app and module written before migration would be
  rewritten. Including Willow when it eventually refactors. Double-
  rewrite cost is unacceptable.
- We do not have a chat-product-keep-alive deadline that justified
  PR #636's ship-faster framing.
- Submit-and-poll (§8.5) gives us sync-ABI ergonomics regardless;
  full CM does not lose anything to Extism on that axis.

### 8.3 Cross-component composition

Components compose via typed WIT resource handles. A module exports
a WIT interface; an app (or another module) imports it. Resource
handles are non-forgeable refs ([capabilities.md](capabilities.md) §7.4). The kernel arbitrates every
cross-component call.

```
component A imports: my-app:counter
                          ↓
component B exports: my-app:counter  (the counter app)
```

`wac` (WCM composition tool) is supported for build-time composition.
Runtime composition is also supported through the kernel's component
instantiation pathway — apps may load module components dynamically
based on user choice.

### 8.4 No cross-component shared memory

No direct memory sharing between components. Every interaction is
typed, bounded, and refusable. The kernel is the call broker.

### 8.5 Submit-and-poll for inherently async surfaces

Browser jco preview2 does not support async at the WIT boundary.
state-apply is sync by definition. Kernel calls that wrap async
surfaces (gossip broadcast, blob fetch, HTTP, persistent KV, timers)
follow a submit-and-poll pattern:

```wit
// Async surfaces use a -submit / on-completion pair:
host.broadcast-submit(topic: topic-id, msg: list<u8>) -> request-token
host.blob-fetch-submit(hash: blob-hash) -> request-token
host.http-request-submit(req: http-request) -> request-token

// Each profile that uses async surfaces exports a corresponding handler:
on-broadcast-completion(token: request-token, result: result<unit, broadcast-error>) -> ()
on-blob-fetch-completion(token: request-token, result: result<list<u8>, fetch-error>) -> ()
on-http-completion(token: request-token, result: result<http-response, http-error>) -> ()
```

The component returns immediately; the kernel re-enters via the
exported handler when the operation finishes. Back-pressure is
preserved (a slow operation does not stall the component's actor
mailbox).

**Token lifecycle**: tokens are kernel-issued opaque HMAC-tagged
values. Components cannot forge tokens. Each token is single-use —
the kernel rejects repeated `on-completion` calls with the same
token (replay protection per [risks.md](risks.md) §19). Tokens issued to a component
expire when that component instance terminates.

**Outstanding-token bound**: the kernel caps per-component
outstanding tokens (default 256; configurable at master-spec
implementation time, not via app manifest). When the cap is hit,
new submit calls fail with `would-block-error` and the component
must wait for outstanding completions to drain.

When jco preview3 stabilizes async at the WIT boundary, the
kernel-side adapter migrates without API churn for app authors.

### 8.6 Coarse-grained interfaces

Interaction components return view models in per-surface units (one
channel timeline, one member list, one composer state). Returns are
version-tagged so the host can skip recomposition on no-op state
changes; large lists are paged. Behavior components observe and emit
in batches.

No tight inner-loop callbacks across component boundaries. Cross-
component calls have measurable cost (component instantiation, ABI
translation, capability gate check); coarse granularity amortizes
the cost.


