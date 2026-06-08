**Date:** 2026-06-08
**Status:** active
**Subject:** What WASM async/streaming does NOT solve for host.subscribe — Myrhiza's risk list

# Open problems

Structural gaps in the surveyed systems (Component Model async, Wasmtime P3, jco,
JSPI). Each becomes a Myrhiza risk to track for `host.subscribe`.

## 1. No portable WIT-boundary async (the Safari/JSPI gap)

There is no async-at-the-WIT-boundary mechanism that works in *all* target
browsers today. jco lowers async only via JSPI; JSPI is unflagged in Chrome 137+,
flagged in Firefox, and **not in stable Safari** — it landed in Safari Technology
Preview 238 (2026-02-26) but no stable Safari release ships it (Phase 4, Interop
2026, no stable-release date). So the Component Model's flagship streaming
primitive `stream<T>` is, in 2026, *not portable*. Myrhiza cannot make it the
baseline transport.

- **Risk:** any design that assumes `stream<T>` in the browser breaks Safari.
- **Mitigation:** sync-boundary guest-callback delivery (see lessons.md); treat
  `stream<T>` as a capability-detected native optimization only.
- **Trigger to revisit:** *stable* Safari ships JSPI (already in Safari Technology
  Preview 238; watch Interop 2026 / WebKit for the stable release), and jco's
  `--async-mode jspi` exits EXPERIMENTAL.

## 2. The single-use-token model does not express subscriptions

Myrhiza's submit-and-poll (§8.5) is request→response: a `*-submit` initiates one
operation and yields one single-use `request-token`, consumed by one
`on-*-completion`. A subscription is unsolicited, open-ended, many-message. There
is no "submit" for an inbound gossip message, and single-use is *enforced* (replay
protection) so a token cannot be re-armed. The token model structurally cannot
carry a stream.

- **Risk:** shoehorning subscriptions into tokens → 256-cap churn or replay-rule
  violation.
- **Mitigation:** a distinct `host.subscribe` primitive with a *persistent*
  handle, not a token (lessons.md → Borrow).

## 3. WASIp3 host streams are preview-quality exactly where Myrhiza needs them

The native `stream<T>` host API exists (Wasmtime 43, `0.3.0-rc-2026-03-15`) but
the hardened cases are short-lived request/response streams. wasmCloud explicitly
flags **long-lived streams under load and backpressure** as unfinished Q2-2026
work — which *is* the subscription use case.

- **Risk:** betting the native path on an unproven edge; backpressure/lifecycle
  bugs surface only under sustained load.
- **Mitigation:** load-test long-lived subscriptions before relying on host
  `stream<T>`; bound kernel-side buffering independently of the stream's own
  backpressure.

## 4. JSPI in-task deadlock with sync-form host imports (wit-bindgen#1609)

JSPI suspends one execution context per `WebAssembly.promising` root. A guest task
that both reads a stream and awaits a sync-form host import can park its entire
executor and deadlock (a sibling arm that must write the stream never runs).

- **Risk:** a browser `interaction` component reading a subscription and calling
  other host imports in the same task can hang — silently, only under specific
  interleavings.
- **Mitigation:** discrete sync callback delivery (no in-task awaited stream);
  if async-form is ever used, follow the upstream guidance (async-form imports,
  independent subtask `promising` roots, defer suspension until queue drains).

## 5. Spec/ABI churn risk during the P3 RC train

Component-model-async is still moving: v41 *removed* the `POLL` callback code from
the canonical ABI and changed `waitable-set.poll`; v41 forbade sync functions
blocking before return; v43 tightened blocking checks. The WASIp3 snapshot is an
RC, not a final 0.3.0.

- **Risk:** building directly on bleeding-edge ABI details that may shift before
  0.3.0 final; guest toolchains (wit-bindgen) and jco may lag/change.
- **Mitigation:** isolate the async/stream mechanism behind the kernel adapter so
  ABI churn doesn't reach app authors (§8.5 migration promise); pin runtime
  versions; gate on a stable 0.3.0 before exposing native streams as contract.

## 6. Resource/handle exhaustion and leak hazards

Every waitable, waitable-set, stream/future end, and resource consumes host
memory (Wasmtime added a configurable handle-count limit, issue #11552).
`StreamReader` not `close`d leaks and "the writer end will hang indefinitely."

- **Risk:** a misbehaving or malicious app opening many subscriptions exhausts
  host handle tables; failure to tear down on revoke pins gossip topics/memory.
- **Mitigation:** per-component caps on live subscriptions (mirror the 256
  outstanding-token bound); revocation must deterministically close the host
  producer and free the handle; configure Wasmtime's handle limit.

## 7. Determinism is the host's responsibility, not the spec's

The Component Model *permits* deterministic stream ordering but does not provide it
by default — waitable-set event order and host stream read/write order are
nondeterministic. Nothing in the ABI stops a careless host from letting delivery
order influence state.

- **Risk:** if subscription delivery order, topic set, or message identity ever
  reaches a canonical state-digest, cross-peer convergence breaks — a correctness
  bug, not a quirk.
- **Mitigation:** `state-apply` must reject `host.subscribe` and refuse
  subscription handles; subscription data is `interaction`-only; convergence stays
  in the kernel's per-topic replay engine. Enforce mechanically (the profile
  boundary), not by convention.

## 8. Content-addressed topic IDs are not human-meaningful

Orthogonal to the async mechanism but load-bearing for the capability surface:
topic IDs are BLAKE3 content hashes. None of the surveyed async/stream machinery
helps a user understand *which* topic a stream carries.

- **Risk:** UX/authorization confusion; an attenuated capability scoped to the
  wrong hash.
- **Mitigation:** manifest-declared topic scoping + a kernel-side
  name/label mapping outside the hash; out of scope for the transport but must be
  designed alongside `host.subscribe`.

## Sources

- https://github.com/web-platform-tests/interop/issues/1093 (JSPI browser status)
- https://webkit.org/blog/17818/announcing-interop-2026/ (Safari/Interop 2026)
- https://webkit.org/blog/17848/release-notes-for-safari-technology-preview-238/ (JSPI in Safari Tech Preview 238, 2026-02-26)
- https://wasmcloud.com/blog/wasi-p3-on-wasmcloud/ (P3 preview gaps)
- https://github.com/bytecodealliance/wit-bindgen/issues/1609 (JSPI deadlock)
- https://github.com/bytecodealliance/wasmtime/releases (v41, v43 ABI changes)
- https://github.com/bytecodealliance/wasmtime/issues/11552 (handle-count limit)
- https://docs.wasmtime.dev/api/wasmtime/component/struct.StreamReader.html (close/leak)
- docs/specs/2026-05-09-myrhiza-master-design/abi.md §8.5
- https://github.com/WebAssembly/component-model/blob/main/design/mvp/Concurrency.md (nondeterminism)
