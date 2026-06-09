**Date:** 2026-06-08
**Status:** active
**Subject:** Decisions for Myrhiza `host.subscribe` — how to stream subscription messages into a sandboxed interaction component, portably

# Lessons for host.subscribe

The Myrhiza questions, answered up front:

- **What are the concrete options to deliver a STREAM into a sandboxed
  interaction component?** Three: (1) submit-and-poll — *cannot model a stream*;
  (2) Component-Model `stream<T>` resource — native-ideal, *not portable*; (3)
  host-invokes-guest-callback export — *portable today*. See
  [delivery-patterns.md](delivery-patterns.md).
- **Which works BOTH natively (Wasmtime) and in browser (jco)?** Only the
  **guest-callback export** works in every browser including Safari. The
  `stream<T>` resource works natively but needs JSPI in-browser (Safari gap).
- **What's the Safari/JSPI constraint?** jco lowers WIT-boundary async *only* via
  JSPI; stable Safari does not ship JSPI yet (Phase 4, in Interop 2026; landed in
  Safari Technology Preview 238 on 2026-02-26 but not in a stable release as of
  2026-06). So async at the WIT boundary excludes stable Safari.
- **How does each interact with the single-use submit-and-poll token model?**
  Submit-and-poll's single-use token models request→response, not
  subscribe→stream. `stream<T>` *replaces* the token with a stream handle.
  Guest-callback *extends* it: a sync acquire returns a persistent subscription
  handle, delivery is the kernel calling an export.
- **Determinism?** Subscription is `interaction`-profile: per-peer delivery order
  is fine, but it must stay out of canonical state — `state-apply` must reject
  `host.subscribe`.

## Validates

- **Keeping the WIT boundary synchronous is the right portability call.** Myrhiza
  §8.5 already chose submit-and-poll specifically because "Browser jco preview2
  does not support async at the WIT boundary." The 2026 evidence vindicates this:
  the *only* browser async lowering (JSPI) still lacks Safari. A sync boundary
  with the host driving re-entry is the portable invariant — extend it to
  subscriptions rather than reach for `stream<T>`.
- **Make `subscription` a WIT `resource` — unforgeable handles are the Component
  Model grain.** Native `stream<T>` hands the guest a readable end as a *resource*:
  an unforgeable i32 index into a per-instance handle table the guest cannot
  fabricate or guess. The recommended sync `host.subscribe(topic) -> subscription`
  handle is *not* a `stream<T>` end — it is a value the kernel mints, so its
  representation is a deliberate choice: (a) a WIT `resource` (`own`/`borrow`),
  which gets that same table-index unforgeability *for free* and makes drop
  observable to the host; or (b) an HMAC-tagged opaque value like the §8.5
  `request-token`, which the kernel must validate on every use. Prefer **(a)**:
  unforgeability is structural not cryptographic, drop is a revocation/cleanup
  signal, and it reuses the Model's capability machinery instead of re-implementing
  token validation. The handle stays scoped per-topic, attenuable, revocable.
- **Backpressure belongs to the host.** Both Wasmtime's `StreamProducer` pull
  model and Myrhiza's §8.5 ("a slow operation does not stall the component's actor
  mailbox") put flow control on the kernel side. Keep it there: the kernel
  buffers/bounds gossip per subscription and decides when to push.
- **The Component Model's nondeterminism is *permitted* for interaction.**
  Concurrency.md confirms waitable-set event ordering and host-driven stream
  read/write ordering are nondeterministic. That is acceptable precisely because
  subscription delivery is `interaction`-profile and never hashed into state.
- **There is a clean native→portable migration story.** §8.5 already promises:
  "When jco preview3 stabilizes async at the WIT boundary, the kernel-side adapter
  migrates without API churn for app authors." A guest-callback `host.subscribe`
  can later be re-backed by `stream<T>` natively / once Safari ships JSPI, without
  changing the app-facing capability.

## Avoid

- **Do not model subscription as recycled submit-and-poll tokens.** Single-use is
  enforced for replay protection; re-arming a token is *rejected*. And there is no
  "submit" for an unsolicited inbound message. Forcing a stream through the token
  model means either token-churn against the 256 cap or violating replay
  protection. Submit-and-poll stays for request→response surfaces (broadcast,
  blob-fetch, http); subscription needs its own primitive.
- **Do not make in-browser delivery depend on `stream<T>` / JSPI as the baseline.**
  That ships a Safari-broken runtime. JSPI is Chrome-only-unflagged today
  (Firefox flagged). At most, treat native `stream<T>` as an *optimization* behind
  capability-detection, never the portable contract.
- **Do not co-suspend subscription delivery with sync-form host imports in one
  guest task under JSPI.** wit-bindgen#1609: suspending one `promising` root
  freezes the whole executor; a guest that reads a subscription stream and calls a
  sync-form host import in the same task can deadlock. Another reason to keep
  delivery as discrete sync callback invocations rather than an in-task awaited
  stream.
- **Do not let delivery order, topic set, or message identity reach a
  state-digest.** This is a *correctness* bug, not a style issue: it would break
  cross-peer convergence. `state-apply` must reject `host.subscribe` and refuse to
  hold subscription handles. Convergence stays per-topic in the kernel's replay
  engine; the sandbox only projects already-converged state.
- **Do not rely on WASIp3 host streams being production-ready in 2026.** wasmCloud
  reports long-lived streams under load + backpressure as the unhardened edge —
  which is *exactly* a subscription. Even natively, validate before betting on it.

## Borrow

- **The guest-callback / stackless-callback shape.** Model `host.subscribe(topic)
  -> subscription` as a synchronous acquire returning a persistent handle, then
  deliver via an exported `on-subscription-message(sub, topic, msg)` the kernel
  calls per message. This is the Component Model's own stackless-export idiom (a
  callback the runtime re-invokes until done) and works in every browser via the
  jco runtime shim with no JSPI. Multi-topic aggregation: one handler keyed by
  `subscription`/`topic-id`, the guest fans into per-channel in-sandbox state.
- **`waitable-set` semantics as the native fast-path, conceptually.** When/if the
  kernel uses native `stream<T>`, give each topic its own readable end and let the
  guest join them into one waitable set (epoll-style) — or hand one
  `stream<envelope>` tagged with `topic-id`. Borrow the *uniform multiplexing*
  idea even if the portable build uses callbacks.
- **Revocation mechanic — and it differs by path.** For the *native stream* path,
  borrow `StreamReader::close` discipline: "must be disposed of using `close`;
  otherwise … the writer end will hang indefinitely" — revocation drops the
  host's writable end and frees the readable resource. But the **recommended
  callback path has no `stream.close` resource to lean on**; revocation there is
  purely kernel-side: the kernel (1) stops calling `on-subscription-message` for
  that handle, (2) drops the `subscription` from its per-instance handle table,
  and (3) tears down the host-side gossip producer/buffer feeding it. After that,
  any subsequent guest use of the now-stale `subscription` handle must **trap (if
  a dropped WIT `resource`) or return an `error` (if a token-style value)** — it
  must never silently no-op, or a revoked app keeps believing it is subscribed.
  Either way revocation deterministically frees host resources with no leak that
  pins a gossip topic. Revocation + attenuation are first-class, not afterthoughts.
- **Completion-based, zero-copy buffers (native).** P3 `stream.read`/`write` are
  completion-based into caller buffers; the zero-length read/write signals
  readiness without copying. If Myrhiza ever exposes a native stream fast-path,
  inherit this rather than inventing a readiness protocol.
- **Capability detection from jco.** Probe JSPI availability
  (`WebAssembly.Suspending`) at runtime; the State-of-Wasm consensus is
  Chrome-shipped / Firefox-flagged / Safari-pending, so detection (not a build
  flag) decides whether the native-style stream path is even attemptable
  in-browser.

## Recommended direction (one-line)

Make `host.subscribe` a **sync-acquire + host-invoked guest-callback** capability
(persistent, per-topic, attenuable, revocable handle), portable across Wasmtime
and all browsers today; keep the Component-Model `stream<T>` resource as a
documented *native optimization and future portable target* once jco-preview3 /
Safari-JSPI land — migrating under the same app-facing handle per §8.5.

## Sources

- docs/specs/2026-05-09-myrhiza-master-design/abi.md §8.5
- https://github.com/WebAssembly/component-model/blob/main/design/mvp/Concurrency.md
- https://docs.wasmtime.dev/api/wasmtime/component/struct.StreamReader.html
- https://bytecodealliance.github.io/jco/transpiling.html
- https://github.com/bytecodealliance/wit-bindgen/issues/1609
- https://github.com/web-platform-tests/interop/issues/1093
- https://wasmcloud.com/blog/wasi-p3-on-wasmcloud/
